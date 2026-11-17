// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-14R (Deliverable C1) — **MESIN KEADAAN CINCIN PENYANGGA**.
//!
//! Modul ini hanya bergantung pada `core` supaya dapat diuji host dengan
//! `#[path]` tanpa menyeret kontrak ABI v2.

#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;

/// Jumlah slot arena. `RingBuffer` dibuat per-adapter/per-konsol, bukan
/// per-syscall; 1024 jauh melampaui kebutuhan nyata dan tetap tanpa alokator.
pub const N_SLOT: usize = 1024;

/// Sentinel "handle tidak sah" (32-bit, karena C++ menyimpan handle sebagai `u32`).
pub const TAK_SAH: u32 = u32::MAX;

/// Penanda laporan; hanya ada di Rust. "KLATCINC".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4349_4E43;

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Slot cincin dibuat.
    Buat = 0,
    /// Ruang dipesan (`reserve_space`).
    Pesan = 1,
    /// Ruang dikembalikan (`reclaim_space`).
    Kembali = 2,
    /// Salinan masuk/keluar dihitung (`copy_data_in`/`copy_data_out`).
    Salin = 3,
    /// Slot dilepas.
    Lepas = 4,
    /// Ditolak: handle tak sah / basi.
    TolakHandle = 5,
    /// Ditolak: transisi melanggar invarian keadaan.
    TolakRentang = 6,
    /// Ditolak: kapasitas nol.
    TolakKapasitas = 7,
    /// Ruang dimundurkan (`rewind_space`) — invers `maju_masuk`.
    Mundur = 8,
}

const N_COUNTER: usize = 9;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Baca penghitung (untuk matriks bukti).
pub fn counter(c: Counter) -> u32 {
    COUNTERS[c as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
pub fn reset() -> u32 {
    let mut i = 0;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// ---------------------------------------------------------------------------
// Arena: N_SLOT slot, tiap slot tiga u64 keadaan + generasi + hidup
// ---------------------------------------------------------------------------

struct Slot {
    mulai: AtomicU64,
    terpakai: AtomicU64,
    kapasitas: AtomicU64,
    gen: AtomicU32,
    hidup: AtomicBool,
}

impl Slot {
    const fn baru() -> Self {
        Self {
            mulai: AtomicU64::new(0),
            terpakai: AtomicU64::new(0),
            kapasitas: AtomicU64::new(0),
            gen: AtomicU32::new(0),
            hidup: AtomicBool::new(false),
        }
    }
}

static ARENA: [Slot; N_SLOT] = [const { Slot::baru() }; N_SLOT];
static KUNCI: AtomicBool = AtomicBool::new(false);
/// Status interupsi pemegang KUNCI. Hanya pemegang yang menulis dan
/// membacanya sebelum melepas, jadi tidak perlu atomik kuat.
static KUNCI_FLAGS: AtomicU64 = AtomicU64::new(0);

fn kunci() {
    // Matikan interupsi SEBELUM mengambil kunci. Pemegang kunci tidak boleh
    // di-preempt dan handler interupsi tidak boleh masuk lagi ke bagian kritis;
    // keduanya membuat penunggu berputar selamanya (P0-1).
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while KUNCI
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    KUNCI_FLAGS.store(flags, Ordering::Relaxed);
}

fn buka() {
    // Baca status milik sendiri SEBELUM melepas kunci, supaya pengambil
    // berikutnya tidak bisa menimpa nilainya.
    let flags = KUNCI_FLAGS.load(Ordering::Relaxed);
    KUNCI.store(false, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Bungkus `(index, generation)` menjadi handle 32-bit.
const fn bungkus(index: usize, gen: u32) -> u32 {
    ((gen & 0xFFFF) << 16) | (index as u32 & 0xFFFF)
}

const fn indeks(h: u32) -> usize {
    (h & 0xFFFF) as usize
}

const fn generasi(h: u32) -> u32 {
    (h >> 16) & 0xFFFF
}

/// Validasi handle. HARUS dipanggil dengan KUNCI dipegang.
fn slot_sah(h: u32) -> Result<usize, i32> {
    if h == TAK_SAH {
        return Err(-EINVAL);
    }
    let i = indeks(h);
    if i >= N_SLOT {
        return Err(-EINVAL);
    }
    if !ARENA[i].hidup.load(Ordering::Relaxed) {
        return Err(-EINVAL);
    }
    if ARENA[i].gen.load(Ordering::Relaxed) & 0xFFFF != generasi(h) {
        return Err(-EINVAL);
    }
    Ok(i)
}

// ---------------------------------------------------------------------------
// Operasi keadaan — API aman (tanpa kunci; pemanggil memegang KUNCI)
// ---------------------------------------------------------------------------

/// Aritmetika murni; tak menyentuh arena. `c` tidak boleh nol.
#[inline]
fn mulai_bebas_dari(s: u64, u: u64, c: u64) -> u64 {
    (s.wrapping_add(u)) % c
}

/// Sisa sampai ujung cincin.
#[inline]
fn sisa_ujung_dari(s: u64, u: u64, c: u64) -> u64 {
    (c - mulai_bebas_dari(s, u, c)) % c
}

/// Batas salin masuk: `min(c - u, min(c - mulai_bebas, panjang))`.
#[inline]
fn batas_masuk_dari(s: u64, u: u64, c: u64, panjang: u64) -> u64 {
    let a = c - u;
    let b = c - mulai_bebas_dari(s, u, c);
    a.min(b).min(panjang)
}

/// Batas salin keluar: `min(min(u, ukuran), c - (s % c))`.
#[inline]
fn batas_keluar_dari(s: u64, u: u64, c: u64, ukuran: u64) -> u64 {
    let a = u.min(ukuran);
    let b = c - (s % c);
    a.min(b)
}

/// Muat keadaan slot (s, u, c) atau `Err`. HARUS dengan KUNCI dipegang.
fn muat(h: u32) -> Result<(usize, u64, u64, u64), i32> {
    let i = slot_sah(h)?;
    let s = ARENA[i].mulai.load(Ordering::Relaxed);
    let u = ARENA[i].terpakai.load(Ordering::Relaxed);
    let c = ARENA[i].kapasitas.load(Ordering::Relaxed);
    Ok((i, s, u, c))
}

// ---------------------------------------------------------------------------
// API publik (dipakai matriks host)
// ---------------------------------------------------------------------------

/// Buat cincin berkapasitas `kapasitas` byte. Mengembalikan handle atau `TAK_SAH`.
pub fn buat(kapasitas: u64) -> u32 {
    if kapasitas == 0 {
        bump(Counter::TolakKapasitas);
        return TAK_SAH;
    }
    kunci();
    let mut dipilih: Option<usize> = None;
    let mut i = 0;
    while i < N_SLOT {
        if !ARENA[i].hidup.load(Ordering::Relaxed) {
            dipilih = Some(i);
            break;
        }
        i += 1;
    }
    let i = match dipilih {
        Some(i) => i,
        None => {
            buka();
            bump(Counter::TolakKapasitas);
            return TAK_SAH;
        }
    };
    let gen = ARENA[i].gen.load(Ordering::Relaxed).wrapping_add(1) & 0xFFFF;
    ARENA[i].mulai.store(0, Ordering::Relaxed);
    ARENA[i].terpakai.store(0, Ordering::Relaxed);
    ARENA[i].kapasitas.store(kapasitas, Ordering::Relaxed);
    ARENA[i].gen.store(gen, Ordering::Relaxed);
    ARENA[i].hidup.store(true, Ordering::Release);
    buka();
    bump(Counter::Buat);
    bungkus(i, gen)
}

/// Lepas slot. Handle lama menjadi basi. Mengembalikan `0` atau `-errno`.
pub fn lepas(h: u32) -> i32 {
    kunci();
    let i = match slot_sah(h) {
        Ok(i) => i,
        Err(e) => {
            buka();
            bump(Counter::TolakHandle);
            return e;
        }
    };
    ARENA[i].hidup.store(false, Ordering::Release);
    let gen = ARENA[i].gen.load(Ordering::Relaxed).wrapping_add(1) & 0xFFFF;
    ARENA[i].gen.store(gen, Ordering::Relaxed);
    buka();
    bump(Counter::Lepas);
    0
}

/// Jumlah slot hidup.
pub fn hidup() -> u32 {
    kunci();
    let mut n = 0;
    let mut i = 0;
    while i < N_SLOT {
        if ARENA[i].hidup.load(Ordering::Relaxed) {
            n += 1;
        }
        i += 1;
    }
    buka();
    n
}

macro_rules! kueri {
    ($nama:ident, $ret:ty, $gagal:expr, $body:expr) => {
        pub fn $nama(h: u32) -> $ret {
            kunci();
            let hasil = match muat(h) {
                Ok((i, s, u, c)) => {
                    let _ = i;
                    let f: fn(u64, u64, u64) -> u64 = $body;
                    f(s, u, c)
                }
                Err(_) => {
                    buka();
                    bump(Counter::TolakHandle);
                    return $gagal;
                }
            };
            buka();
            hasil
        }
    };
}

kueri!(mulai_bebas, u64, u64::MAX, |s, u, c| mulai_bebas_dari(s, u, c));
kueri!(mulai_terpakai, u64, u64::MAX, |s, _u, c| s % c);
kueri!(terpakai, u64, u64::MAX, |_s, u, _c| u);
kueri!(kapasitas, u64, u64::MAX, |_s, _u, c| c);
kueri!(tersedia, u64, u64::MAX, |_s, u, c| c - u);
kueri!(sisa_ujung, u64, u64::MAX, |s, u, c| sisa_ujung_dari(s, u, c));

/// `u < c` (ruang tersisa).
pub fn ada_ruang(h: u32) -> u32 {
    kunci();
    let hasil = match muat(h) {
        Ok((_, _s, u, c)) => {
            if u < c {
                1
            } else {
                0
            }
        }
        Err(_) => {
            buka();
            bump(Counter::TolakHandle);
            return 0;
        }
    };
    buka();
    hasil
}

/// Batas salin masuk. Mengembalikan `u64::MAX` bila handle tak sah.
pub fn batas_masuk(h: u32, panjang: u64) -> u64 {
    kunci();
    let hasil = match muat(h) {
        Ok((_, s, u, c)) => batas_masuk_dari(s, u, c, panjang),
        Err(_) => {
            buka();
            bump(Counter::TolakHandle);
            return u64::MAX;
        }
    };
    buka();
    bump(Counter::Salin);
    hasil
}

/// Batas salin keluar. Mengembalikan `u64::MAX` bila handle tak sah.
pub fn batas_keluar(h: u32, ukuran: u64) -> u64 {
    kunci();
    let hasil = match muat(h) {
        Ok((_, s, u, c)) => batas_keluar_dari(s, u, c, ukuran),
        Err(_) => {
            buka();
            bump(Counter::TolakHandle);
            return u64::MAX;
        }
    };
    buka();
    bump(Counter::Salin);
    hasil
}

/// Majukan penanda terpakai sebesar `n` (dipakai `reserve_space`/`copy_data_in`).
///
/// Menjaga invarian aritmetika: `terpakai + n` tidak boleh luber `u64` dan tidak
/// boleh melebihi kapasitas. **Keputusan admission** kepenuhan tetap milik
/// `admisi_mem3`; di sini hanya invarian keadaan yang dijaga.
pub fn maju_masuk(h: u32, n: u64) -> i32 {
    kunci();
    let (i, _s, u, c) = match muat(h) {
        Ok(v) => v,
        Err(e) => {
            buka();
            bump(Counter::TolakHandle);
            return e;
        }
    };
    let baru = match u.checked_add(n) {
        Some(v) => v,
        None => {
            buka();
            bump(Counter::TolakRentang);
            return -EINVAL;
        }
    };
    if baru > c {
        buka();
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    ARENA[i].terpakai.store(baru, Ordering::Relaxed);
    buka();
    bump(Counter::Pesan);
    0
}

/// Kembalikan ruang sebesar `n` (dipakai `reclaim_space`).
///
/// `mulai += n`, `terpakai -= n`. Gagal bila `n > terpakai`.
pub fn kembali(h: u32, n: u64) -> i32 {
    kunci();
    let (i, s, u, _c) = match muat(h) {
        Ok(v) => v,
        Err(e) => {
            buka();
            bump(Counter::TolakHandle);
            return e;
        }
    };
    if n > u {
        buka();
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    ARENA[i].mulai.store(s.wrapping_add(n), Ordering::Relaxed);
    ARENA[i].terpakai.store(u - n, Ordering::Relaxed);
    buka();
    bump(Counter::Kembali);
    0
}

/// Mundurkan penanda terpakai sebesar `n` — **invers persis** `maju_masuk`.
///
/// `terpakai -= n`, `mulai` **tidak** berubah. Dipakai untuk membatalkan
/// penulisan sebagian (`copy_data_in`) ketika pembangunan rantai VirtIO gagal
/// di tengah jalan, sehingga cincin TX tidak pernah tertinggal dalam keadaan
/// terpakai tanpa descriptor. Gagal bila `n > terpakai`.
///
/// Berbeda dengan `kembali` (yang juga memajukan `mulai`, karena ia memulihkan
/// dari DEPAN), `mundur` membatalkan penulisan dari BELAKANG.
pub fn mundur(h: u32, n: u64) -> i32 {
    kunci();
    let (i, _s, u, _c) = match muat(h) {
        Ok(v) => v,
        Err(e) => {
            buka();
            bump(Counter::TolakHandle);
            return e;
        }
    };
    if n > u {
        buka();
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    ARENA[i].terpakai.store(u - n, Ordering::Relaxed);
    buka();
    bump(Counter::Mundur);
    0
}

// ---------------------------------------------------------------------------
// Laporan + matriks bukti
// ---------------------------------------------------------------------------

/// Laporan matriks bukti cincin penyangga (56 byte, cermin C++).
#[repr(C)]
pub struct KlatosCincinReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub dif_ukur: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_pesan: u32,
    pub prod_kembali: u32,
    pub prod_salin: u32,
    pub prod_lepas: u32,
    pub prod_tolak: u32,
    pub status: i32,
    pub reserved: u32,
}

/// Oracle naif untuk diferensial: keadaan cincin yang sama, ditulis ulang
/// dengan rumus paling lugas. Bila hasil Rust berbeda dari oracle ini pada satu
/// kasus pun, modul gagal.
struct Naif {
    s: u64,
    u: u64,
    c: u64,
}

impl Naif {
    fn baru(c: u64) -> Self {
        Self { s: 0, u: 0, c }
    }
    fn mulai_bebas(&self) -> u64 {
        (self.s + self.u) % self.c
    }
    fn mulai_terpakai(&self) -> u64 {
        self.s % self.c
    }
    fn tersedia(&self) -> u64 {
        self.c - self.u
    }
    fn sisa_ujung(&self) -> u64 {
        let mb = (self.s + self.u) % self.c;
        (self.c - mb) % self.c
    }
    fn batas_masuk(&self, panjang: u64) -> u64 {
        let a = self.c - self.u;
        let b = self.c - ((self.s + self.u) % self.c);
        a.min(b).min(panjang)
    }
    fn batas_keluar(&self, ukuran: u64) -> u64 {
        let a = self.u.min(ukuran);
        let b = self.c - (self.s % self.c);
        a.min(b)
    }
    fn maju(&mut self, n: u64) -> i32 {
        if n > self.c - self.u {
            return -1;
        }
        self.u += n;
        0
    }
    fn kembali(&mut self, n: u64) -> i32 {
        if n > self.u {
            return -1;
        }
        self.s += n;
        self.u -= n;
        0
    }
    fn mundur(&mut self, n: u64) -> i32 {
        if n > self.u {
            return -1;
        }
        self.u -= n;
        0
    }
}

/// Jalankan matriks bukti lengkap.
pub fn run_matrix() -> KlatosCincinReport {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;

    macro_rules! catat {
        ($cond:expr) => {{
            checks += 1;
            if $cond {
                passed += 1;
            }
        }};
    }

        // D-9: reset() dihapus - jangan hapus state produksi.
        // Uji-sendiri kini memakai slot bebas dan melepas miliknya sendiri.

    // --- konstanta --------------------------------------------------------
    catat!(REPORT_TAG == 0x4B4C_4154_4349_4E43);
    catat!(EINVAL == 22);
    catat!(N_SLOT == 1024);
    catat!(TAK_SAH == u32::MAX);

    // --- siklus NYATA: isi lalu kosongkan ---------------------------------
    {
        let c = 4096u64;
        let h = buat(c);
        catat!(h != TAK_SAH);
        catat!(kapasitas(h) == c);
        catat!(terpakai(h) == 0);
        catat!(tersedia(h) == c);
        catat!(mulai_bebas(h) == 0);
        catat!(mulai_terpakai(h) == 0);
        catat!(sisa_ujung(h) == 0); // (c - 0) % c == 0
        catat!(ada_ruang(h) == 1);

        // pesan 1000 byte di awal
        catat!(maju_masuk(h, 1000) == 0);
        catat!(terpakai(h) == 1000);
        catat!(tersedia(h) == c - 1000);
        catat!(mulai_bebas(h) == 1000);
        catat!(mulai_terpakai(h) == 0);
        catat!(sisa_ujung(h) == c - 1000);

        // kembalikan 400 byte dari depan
        catat!(kembali(h, 400) == 0);
        catat!(terpakai(h) == 600);
        catat!(mulai_terpakai(h) == 400);
        catat!(mulai_bebas(h) == 1000);
        catat!(sisa_ujung(h) == c - 1000);

        catat!(lepas(h) == 0);
        catat!(terpakai(h) == u64::MAX); // basi
    }

    // --- pembungkusan (wrap) ---------------------------------------------
    {
        let c = 100u64;
        let h = buat(c);
        // isi 80, kembalikan 70 -> mulai_terpakai=70, mulai_bebas=80
        catat!(maju_masuk(h, 80) == 0);
        catat!(kembali(h, 70) == 0);
        catat!(mulai_terpakai(h) == 70);
        catat!(mulai_bebas(h) == 80);
        // batas masuk dibatasi oleh c - mulai_bebas = 20
        catat!(batas_masuk(h, 1000) == 20);
        // batas keluar dibatasi oleh c - mulai_terpakai = 30
        catat!(batas_keluar(h, 1000) == 10); // terpakai=10
        // maju 20 -> mulai_bebas=0 (wrap), terpakai=30
        catat!(maju_masuk(h, 20) == 0);
        catat!(mulai_bebas(h) == 0);
        catat!(terpakai(h) == 30);
        catat!(sisa_ujung(h) == 0);
        catat!(batas_masuk(h, 1000) == 70);
        catat!(lepas(h) == 0);
    }

    // --- mundur: invers maju_masuk (pemulihan tulis-sebagian) -------------
    {
        let c = 4096u64;
        let h = buat(c);
        // maju 1000 lalu mundur 1000 harus kembali ke keadaan awal.
        catat!(maju_masuk(h, 1000) == 0);
        catat!(terpakai(h) == 1000);
        catat!(mulai_terpakai(h) == 0);
        catat!(mulai_bebas(h) == 1000);
        catat!(mundur(h, 1000) == 0);
        catat!(terpakai(h) == 0);
        catat!(mulai_terpakai(h) == 0); // mulai TIDAK berubah (beda dgn kembali)
        catat!(mulai_bebas(h) == 0);
        catat!(tersedia(h) == c);
        // maju 300, maju 200, mundur 200 -> terpakai 300, mulai_bebas 300.
        catat!(maju_masuk(h, 300) == 0);
        catat!(maju_masuk(h, 200) == 0);
        catat!(mundur(h, 200) == 0);
        catat!(terpakai(h) == 300);
        catat!(mulai_bebas(h) == 300);
        // mundur melebihi terpakai ditolak.
        catat!(mundur(h, 301) == -EINVAL);
        catat!(terpakai(h) == 300);
        catat!(lepas(h) == 0);
    }

    // --- invarian: melebihi kapasitas ditolak ----------------------------
    {
        let c = 64u64;
        let h = buat(c);
        catat!(maju_masuk(h, c) == 0);
        catat!(ada_ruang(h) == 0);
        catat!(tersedia(h) == 0);
        catat!(batas_masuk(h, 100) == 0);
        catat!(maju_masuk(h, 1) == -EINVAL); // terpakai+n > c
        catat!(maju_masuk(h, u64::MAX) == -EINVAL); // luber
        catat!(kembali(h, c + 1) == -EINVAL); // n > terpakai
        catat!(kembali(h, c) == 0);
        catat!(terpakai(h) == 0);
        catat!(ada_ruang(h) == 1);
        catat!(lepas(h) == 0);
    }

    // --- kapasitas nol ditolak -------------------------------------------
    catat!(buat(0) == TAK_SAH);

    // --- handle tak sah ---------------------------------------------------
    {
        catat!(terpakai(TAK_SAH) == u64::MAX);
        catat!(kapasitas(0xDEAD_0001) == u64::MAX);
        catat!(mulai_bebas(TAK_SAH) == u64::MAX);
        catat!(batas_masuk(TAK_SAH, 1) == u64::MAX);
        catat!(batas_keluar(TAK_SAH, 1) == u64::MAX);
        catat!(ada_ruang(TAK_SAH) == 0);
        catat!(lepas(TAK_SAH) == -EINVAL);
        // indeks di luar rentang
        let luar = bungkus(N_SLOT + 7, 1);
        catat!(terpakai(luar) == u64::MAX);
    }

    // --- generasi basi ----------------------------------------------------
    {
        let h1 = buat(16);
        catat!(lepas(h1) == 0);
        let h2 = buat(16);
        catat!(h1 != h2);
        catat!(terpakai(h1) == u64::MAX); // basi
        catat!(lepas(h2) == 0);
    }

    // --- slot berulang-kali (recycle) ------------------------------------
    {
        let mut k = 0;
        while k < 3 {
            let h = buat(128);
            catat!(h != TAK_SAH);
            catat!(maju_masuk(h, 64) == 0);
            catat!(kembali(h, 32) == 0);
            catat!(terpakai(h) == 32);
            catat!(lepas(h) == 0);
            k += 1;
        }
    }

    // --- diferensial vs oracle naif --------------------------------------
    let mut diukur: u32 = 0;
    let mut beda: u32 = 0;
    {
        let c = 4096u64;
        let h = buat(c);
        let mut naif = Naif::baru(c);
        let mut rng: u64 = 0x1234_5678_9ABC_DEF0;
        let mut i = 0u32;
        while i < 2000 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let op = (rng >> 61) % 3;
            if op == 0 {
                let n = (rng >> 8) % (c + 1);
                let ra = maju_masuk(h, n);
                let rn = naif.maju(n);
                if (ra == 0) != (rn == 0) {
                    beda += 1;
                }
            } else if op == 1 {
                let n = (rng >> 8) % (c + 1);
                let ra = kembali(h, n);
                let rn = naif.kembali(n);
                if (ra == 0) != (rn == 0) {
                    beda += 1;
                }
            } else {
                let n = (rng >> 8) % (c + 1);
                let ra = mundur(h, n);
                let rn = naif.mundur(n);
                if (ra == 0) != (rn == 0) {
                    beda += 1;
                }
            }
            // bandingkan setiap kueri pada keadaan yang sama
            if mulai_bebas(h) != naif.mulai_bebas() {
                beda += 1;
            }
            if mulai_terpakai(h) != naif.mulai_terpakai() {
                beda += 1;
            }
            if tersedia(h) != naif.tersedia() {
                beda += 1;
            }
            if sisa_ujung(h) != naif.sisa_ujung() {
                beda += 1;
            }
            let p = (rng >> 16) % (c + 1);
            if batas_masuk(h, p) != naif.batas_masuk(p) {
                beda += 1;
            }
            if batas_keluar(h, p) != naif.batas_keluar(p) {
                beda += 1;
            }
            diukur += 1;
            i += 1;
        }
        catat!(diukur == 2000);
        catat!(beda == 0);
        catat!(lepas(h) == 0);
    }

    // --- penghitung bergerak ---------------------------------------------
    catat!(counter(Counter::Buat) > 0);
    catat!(counter(Counter::Pesan) > 0);
    catat!(counter(Counter::Kembali) > 0);
    catat!(counter(Counter::Salin) > 0);
    catat!(counter(Counter::Lepas) > 0);
    catat!(counter(Counter::Mundur) > 0);
    catat!(counter(Counter::TolakHandle) > 0);
    catat!(counter(Counter::TolakRentang) > 0);
    catat!(counter(Counter::TolakKapasitas) > 0);

    // Determinisme: satu buat lagi menaikkan penghitung tepat satu.
    let a = counter(Counter::Buat);
    let sisa = buat(8);
    let _ = lepas(sisa);
    catat!(counter(Counter::Buat) == a + 1);

    let status = if checks == passed { 0 } else { -1 };
    KlatosCincinReport {
        tag: REPORT_TAG,
        checks,
        passed,
        dif_ukur: diukur,
        dif_beda: beda,
        prod_buat: counter(Counter::Buat),
        prod_pesan: counter(Counter::Pesan),
        prod_kembali: counter(Counter::Kembali),
        prod_salin: counter(Counter::Salin),
        prod_lepas: counter(Counter::Lepas),
        prod_tolak: counter(Counter::TolakHandle)
            + counter(Counter::TolakRentang)
            + counter(Counter::TolakKapasitas),
        status,
        reserved: 0,
    }
}

// ---------------------------------------------------------------------------
// Titik masuk C ABI
// ---------------------------------------------------------------------------

/// Titik masuk C ABI matriks bukti mesin keadaan cincin.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_selftest() -> KlatosCincinReport {
    run_matrix()
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_buat(kapasitas: u64) -> u32 {
    buat(kapasitas)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_lepas(h: u32) -> i32 {
    lepas(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_hidup() -> u32 {
    hidup()
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_mulai_bebas(h: u32) -> u64 {
    mulai_bebas(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_mulai_terpakai(h: u32) -> u64 {
    mulai_terpakai(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_terpakai(h: u32) -> u64 {
    terpakai(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_kapasitas(h: u32) -> u64 {
    kapasitas(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_tersedia(h: u32) -> u64 {
    tersedia(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_ada_ruang(h: u32) -> u32 {
    ada_ruang(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_sisa_ujung(h: u32) -> u64 {
    sisa_ujung(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_batas_masuk(h: u32, panjang: u64) -> u64 {
    batas_masuk(h, panjang)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_batas_keluar(h: u32, ukuran: u64) -> u64 {
    batas_keluar(h, ukuran)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_maju_masuk(h: u32, n: u64) -> i32 {
    maju_masuk(h, n)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_kembali(h: u32, n: u64) -> i32 {
    kembali(h, n)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_mundur(h: u32, n: u64) -> i32 {
    mundur(h, n)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTERS[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_cincin_reset() -> u32 {
    reset()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4349_4E43);
        assert!(EINVAL == 22);
        assert!(N_SLOT == 1024);
        assert!(TAK_SAH == u32::MAX);
        assert!(Counter::Buat as usize == 0);
        assert!(Counter::TolakKapasitas as usize == 7);
        assert!(Counter::Mundur as usize == 8);
        assert!(N_COUNTER == 9);
        // Laporan: 8 (tag) + 12*4 = 56.
        assert!(core::mem::size_of::<KlatosCincinReport>() == 56);
        assert!(core::mem::align_of::<KlatosCincinReport>() == 8);
    };
}
