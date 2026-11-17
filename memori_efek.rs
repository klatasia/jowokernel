// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-13R — **KONTEKS EFEK KAPABILITAS MEMORI** (jalur ABI v2).


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU8, Ordering, AtomicU64};

/// `EBADF` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 9).
pub const EBADF: i32 = 9;
/// `EACCES` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 13).
pub const EACCES: i32 = 13;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `EMFILE` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 24).
pub const EMFILE: i32 = 24;
/// `EOVERFLOW` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 75).
pub const EOVERFLOW: i32 = 75;

/// Kapasitas byte satu slot.
pub const KAPASITAS: usize = 4096;
/// Jumlah slot arena.
pub const N_SLOT: usize = 32;

/// Sentinel "handle tidak sah".
pub const TAK_SAH: u64 = u64::MAX;

/// Hak per-slot.
pub const HAK_BACA: u16 = 0x001;
/// Hak tulis.
pub const HAK_TULIS: u16 = 0x002;

/// Penanda laporan; hanya ada di Rust.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4D45_4D4F; // "KLATMEMO"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Slot dibuat.
    Buat = 0,
    /// Salinan keluar berhasil.
    Baca = 1,
    /// Salinan masuk berhasil.
    Tulis = 2,
    /// Slot dilepas.
    Lepas = 3,
    /// Ditolak: handle tak sah / basi.
    TolakHandle = 4,
    /// Ditolak: hak salah.
    TolakHak = 5,
    /// Ditolak: rentang di luar batas / luber.
    TolakRentang = 6,
    /// Ditolak: kapasitas (slot habis / panjang > kapasitas).
    TolakKapasitas = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

fn bump_tolak(rc: i32) {
    match -rc {
        EBADF => bump(Counter::TolakHandle),
        EACCES => bump(Counter::TolakHak),
        EOVERFLOW | EINVAL => bump(Counter::TolakRentang),
        EMFILE => bump(Counter::TolakKapasitas),
        _ => bump(Counter::TolakRentang),
    }
}

// ---------------------------------------------------------------------------
// Arena: N_SLOT slot, tiap slot KAPASITAS byte, dijaga spinlock
// ---------------------------------------------------------------------------
//
// Tidak ada `static mut` dan tidak ada penafsiran penunjuk untuk keadaan: tiap
// byte adalah `AtomicU8`, tiap bidang slot atomik. Pola ini sama dengan
// `input_keymap.rs` (FASE-6): cukup, karena tidak ada proses konkuren di dalam
// satu jalur syscall. LOCK hanya menjamin konsistensi multi-byte sebuah salinan.

struct Slot {
    byte: [AtomicU8; KAPASITAS],
    panjang: AtomicU32,
    hak: AtomicU16,
    gen: AtomicU32,
    hidup: AtomicBool,
}

impl Slot {
    const fn baru() -> Self {
        Self {
            byte: [const { AtomicU8::new(0) }; KAPASITAS],
            panjang: AtomicU32::new(0),
            hak: AtomicU16::new(0),
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

/// Bungkus `(index, generation)` menjadi handle 64-bit.
const fn bungkus(index: usize, gen: u32) -> u64 {
    ((gen as u64) << 16) | (index as u64)
}

const fn indeks(h: u64) -> usize {
    (h & 0xFFFF) as usize
}

const fn generasi(h: u64) -> u32 {
    ((h >> 16) & 0xFFFF_FFFF) as u32
}

/// Validasi handle + hak. HARUS dipanggil dengan KUNCI dipegang.
fn slot_sah(h: u64, hak_diminta: u16) -> Result<usize, i32> {
    if h == TAK_SAH {
        return Err(-EBADF);
    }
    let i = indeks(h);
    if i >= N_SLOT {
        return Err(-EBADF);
    }
    if !ARENA[i].hidup.load(Ordering::Relaxed) {
        return Err(-EBADF);
    }
    if ARENA[i].gen.load(Ordering::Relaxed) != generasi(h) {
        return Err(-EBADF);
    }
    if ARENA[i].hak.load(Ordering::Relaxed) & hak_diminta != hak_diminta {
        return Err(-EACCES);
    }
    Ok(i)
}

// ---------------------------------------------------------------------------
// Operasi keadaan — API aman
// ---------------------------------------------------------------------------

/// Buat slot berkapasitas `panjang` byte dengan hak `hak`.
///
/// Mengembalikan handle (>0) atau `-errno`.
pub fn buat(hak: u16, panjang: usize) -> Result<u64, i32> {
    if panjang > KAPASITAS {
        bump(Counter::TolakKapasitas);
        return Err(-EINVAL);
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
            return Err(-EMFILE);
        }
    };
    let gen = ARENA[i].gen.load(Ordering::Relaxed).wrapping_add(1);
    let mut k = 0;
    while k < panjang {
        ARENA[i].byte[k].store(0, Ordering::Relaxed);
        k += 1;
    }
    ARENA[i].panjang.store(panjang as u32, Ordering::Relaxed);
    ARENA[i].hak.store(hak, Ordering::Relaxed);
    ARENA[i].gen.store(gen, Ordering::Relaxed);
    ARENA[i].hidup.store(true, Ordering::Release);
    buka();
    bump(Counter::Buat);
    Ok(bungkus(i, gen))
}

/// Salin byte keluar dari slot ke `dst`. Mengembalikan jumlah byte atau `-errno`.
pub fn baca(h: u64, off: usize, dst: &mut [u8]) -> i32 {
    kunci();
    let i = match slot_sah(h, HAK_BACA) {
        Ok(i) => i,
        Err(e) => {
            buka();
            bump_tolak(e);
            return e;
        }
    };
    if dst.is_empty() {
        buka();
        bump(Counter::Baca);
        return 0;
    }
    let panjang = ARENA[i].panjang.load(Ordering::Relaxed) as usize;
    let akhir = match off.checked_add(dst.len()) {
        Some(a) => a,
        None => {
            buka();
            bump(Counter::TolakRentang);
            return -EOVERFLOW;
        }
    };
    if off > panjang || akhir > panjang {
        buka();
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    let mut k = 0;
    while k < dst.len() {
        dst[k] = ARENA[i].byte[off + k].load(Ordering::Relaxed);
        k += 1;
    }
    buka();
    bump(Counter::Baca);
    dst.len() as i32
}

/// Salin byte masuk dari `src` ke slot. Mengembalikan jumlah byte atau `-errno`.
pub fn tulis(h: u64, off: usize, src: &[u8]) -> i32 {
    kunci();
    let i = match slot_sah(h, HAK_TULIS) {
        Ok(i) => i,
        Err(e) => {
            buka();
            bump_tolak(e);
            return e;
        }
    };
    if src.is_empty() {
        buka();
        bump(Counter::Tulis);
        return 0;
    }
    let panjang = ARENA[i].panjang.load(Ordering::Relaxed) as usize;
    let akhir = match off.checked_add(src.len()) {
        Some(a) => a,
        None => {
            buka();
            bump(Counter::TolakRentang);
            return -EOVERFLOW;
        }
    };
    if off > panjang || akhir > panjang {
        buka();
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    let mut k = 0;
    while k < src.len() {
        ARENA[i].byte[off + k].store(src[k], Ordering::Relaxed);
        k += 1;
    }
    buka();
    bump(Counter::Tulis);
    src.len() as i32
}

/// Lepas slot. Handle lama menjadi basi. Mengembalikan `0` atau `-errno`.
pub fn lepas(h: u64) -> i32 {
    kunci();
    let i = match slot_sah(h, 0) {
        Ok(i) => i,
        Err(e) => {
            buka();
            bump_tolak(e);
            return e;
        }
    };
    ARENA[i].hidup.store(false, Ordering::Release);
    let gen = ARENA[i].gen.load(Ordering::Relaxed).wrapping_add(1);
    ARENA[i].gen.store(gen, Ordering::Relaxed);
    buka();
    bump(Counter::Lepas);
    0
}

/// Panjang logis slot, atau `-errno`.
pub fn panjang(h: u64) -> i32 {
    kunci();
    let i = match slot_sah(h, 0) {
        Ok(i) => i,
        Err(e) => {
            buka();
            return e;
        }
    };
    let p = ARENA[i].panjang.load(Ordering::Relaxed) as i32;
    buka();
    p
}

/// Jumlah slot hidup.
pub fn hidup() -> u32 {
    let mut n = 0;
    kunci();
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

/// Nilai satu penghitung audit.
pub fn counter(which: Counter) -> u32 {
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Kosongkan arena + penghitung. TIDAK dipanggil produksi.
pub fn reset() {
    kunci();
    let mut i = 0;
    while i < N_SLOT {
        ARENA[i].hidup.store(false, Ordering::Relaxed);
        ARENA[i].panjang.store(0, Ordering::Relaxed);
        ARENA[i].hak.store(0, Ordering::Relaxed);
        ARENA[i].gen.store(0, Ordering::Relaxed);
        i += 1;
    }
    buka();
    let mut c = 0;
    while c < N_COUNTER {
        COUNTERS[c].store(0, Ordering::Release);
        c += 1;
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Buat objek memori. `hak` = bitmask [`HAK_BACA`]/[`HAK_TULIS`].
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_buat(hak: u32, panjang: u32) -> u64 {
    match buat(hak as u16, panjang as usize) {
        Ok(h) => h,
        Err(_) => TAK_SAH,
    }
}

/// Salin `len` byte keluar dari slot `h` pada offset `off` ke `dst`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_baca(h: u64, off: u64, dst: *mut u8, len: u32) -> i32 {
    if len == 0 {
        return baca(h, off as usize, &mut []);
    }
    if dst.is_null() {
        return -EINVAL;
    }
    // SAFETY: `dst` non-null dan `len` byte berada di dalam objek pemanggil
    // (kontrak C ABI modul ini; pemanggil menjamin kapasitas >= len).
    let s = unsafe { core::slice::from_raw_parts_mut(dst, len as usize) };
    baca(h, off as usize, s)
}

/// Salin `len` byte masuk dari `src` ke slot `h` pada offset `off`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_tulis(h: u64, off: u64, src: *const u8, len: u32) -> i32 {
    if len == 0 {
        return tulis(h, off as usize, &[]);
    }
    if src.is_null() {
        return -EINVAL;
    }
    // SAFETY: `src` non-null dan `len` byte terbaca (kontrak C ABI modul ini).
    let s = unsafe { core::slice::from_raw_parts(src, len as usize) };
    tulis(h, off as usize, s)
}

/// Lepas slot.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_lepas(h: u64) -> i32 {
    lepas(h)
}

/// Panjang logis slot.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_panjang(h: u64) -> i32 {
    panjang(h)
}

/// Jumlah slot hidup.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_hidup() -> u32 {
    hidup()
}

/// Nilai satu penghitung audit.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset arena + penghitung. TIDAK dipanggil produksi.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_reset() -> u32 {
    reset();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks bukti
// ---------------------------------------------------------------------------

/// Cermin `KlatosMemoriEfekReport`.
#[repr(C)]
pub struct KlatosMemoriEfekReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub dif_ukur: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_baca: u32,
    pub prod_tulis: u32,
    pub prod_lepas: u32,
    pub prod_tolak: u32,
    pub status: i32,
    pub reserved: u32,
}

/// Oracle pembanding deterministik — formulasi BERBEDA dari implementasi.
///
/// Model naif memakai `[u8; KAPASITAS]` biasa + `panjang`, bukan arena atomik,
/// sehingga perbandingan menguji KONTRAK (batas, panjang nol, luber), bukan
/// menyalin kode.
struct Naif {
    byte: [u8; KAPASITAS],
    panjang: usize,
    hidup: bool,
}

impl Naif {
    fn baru(panjang: usize) -> Self {
        Self {
            byte: [0u8; KAPASITAS],
            panjang,
            hidup: true,
        }
    }

    fn baca(&self, off: usize, len: usize) -> i32 {
        if !self.hidup {
            return -EBADF;
        }
        if len == 0 {
            return 0;
        }
        match off.checked_add(len) {
            None => -EOVERFLOW,
            Some(a) if off > self.panjang || a > self.panjang => -EINVAL,
            Some(_) => len as i32,
        }
    }

    fn tulis(&mut self, off: usize, src: &[u8]) -> i32 {
        if !self.hidup {
            return -EBADF;
        }
        if src.is_empty() {
            return 0;
        }
        match off.checked_add(src.len()) {
            None => -EOVERFLOW,
            Some(a) if off > self.panjang || a > self.panjang => -EINVAL,
            Some(_) => {
                let mut k = 0;
                while k < src.len() {
                    self.byte[off + k] = src[k];
                    k += 1;
                }
                src.len() as i32
            }
        }
    }
}

/// Matriks bukti. Mengembalikan laporan dengan `status == 0` bila seluruh
/// pemeriksaan lulus.
pub fn run_matrix() -> KlatosMemoriEfekReport {
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
    catat!(REPORT_TAG == 0x4B4C_4154_4D45_4D4F);
    catat!(EBADF == 9 && EACCES == 13 && EINVAL == 22 && EMFILE == 24 && EOVERFLOW == 75);
    catat!(KAPASITAS == 4096 && N_SLOT == 32);
    catat!(HAK_BACA == 1 && HAK_TULIS == 2);

    // --- siklus NYATA round-trip -----------------------------------------
    {
        let h = buat(HAK_BACA | HAK_TULIS, 256).expect("buat gagal");
        catat!(h != TAK_SAH);
        catat!(panjang(h) == 256);

        let mut masuk = [0u8; 16];
        let mut k = 0;
        while k < 16 {
            masuk[k] = (k as u8).wrapping_mul(7).wrapping_add(3);
            k += 1;
        }
        catat!(tulis(h, 0, &masuk) == 16);
        let mut keluar = [0u8; 16];
        catat!(baca(h, 0, &mut keluar) == 16);
        catat!(keluar == masuk);

        // offset sebagian
        let mut sebagian = [0u8; 4];
        catat!(baca(h, 4, &mut sebagian) == 4);
        catat!(sebagian == [masuk[4], masuk[5], masuk[6], masuk[7]]);

        // tulis pada offset
        let patch = [0xAAu8; 4];
        catat!(tulis(h, 8, &patch) == 4);
        let mut cek = [0u8; 4];
        catat!(baca(h, 8, &mut cek) == 4);
        catat!(cek == patch);

        catat!(lepas(h) == 0);
        catat!(panjang(h) == -EBADF); // basi
    }

    // --- panjang nol ------------------------------------------------------
    {
        let h = buat(HAK_BACA | HAK_TULIS, 16).unwrap();
        catat!(baca(h, 0, &mut []) == 0);
        catat!(tulis(h, 0, &[]) == 0);
        catat!(baca(h, 999, &mut []) == 0); // nol selalu 0, tak memandang offset
        catat!(lepas(h) == 0);
    }

    // --- hak salah --------------------------------------------------------
    {
        let ro = buat(HAK_BACA, 16).unwrap();
        catat!(tulis(ro, 0, &[1, 2, 3]) == -EACCES);
        catat!(baca(ro, 0, &mut [0u8; 3]) == 3);
        catat!(lepas(ro) == 0);

        let wo = buat(HAK_TULIS, 16).unwrap();
        catat!(baca(wo, 0, &mut [0u8; 3]) == -EACCES);
        catat!(tulis(wo, 0, &[1, 2, 3]) == 3);
        catat!(lepas(wo) == 0);
    }

    // --- handle tak sah ---------------------------------------------------
    {
        catat!(baca(TAK_SAH, 0, &mut [0u8; 4]) == -EBADF);
        catat!(baca(0xDEAD_BEEF_0000_0001, 0, &mut [0u8; 4]) == -EBADF);
        catat!(tulis(TAK_SAH, 0, &[1]) == -EBADF);
        catat!(lepas(TAK_SAH) == -EBADF);
        // indeks di luar rentang
        let luar = bungkus(N_SLOT + 5, 1);
        catat!(baca(luar, 0, &mut [0u8; 1]) == -EBADF);
    }

    // --- generasi basi ----------------------------------------------------
    {
        let h1 = buat(HAK_BACA | HAK_TULIS, 8).unwrap();
        catat!(lepas(h1) == 0);
        let h2 = buat(HAK_BACA | HAK_TULIS, 8).unwrap();
        // Slot mungkin dipakai ulang; handle lama harus basi.
        catat!(h1 != h2);
        catat!(baca(h1, 0, &mut [0u8; 1]) == -EBADF);
        catat!(lepas(h2) == 0);
    }

    // --- rentang di luar batas / luber -----------------------------------
    {
        let h = buat(HAK_BACA | HAK_TULIS, 32).unwrap();
        catat!(baca(h, 33, &mut [0u8; 1]) == -EINVAL); // off > panjang
        catat!(baca(h, 32, &mut [0u8; 1]) == -EINVAL); // off == panjang, len > 0
        catat!(baca(h, 30, &mut [0u8; 3]) == -EINVAL); // akhir > panjang
        catat!(baca(h, 32, &mut []) == 0); // tepat di ujung, nol
        catat!(tulis(h, 30, &[0u8; 3]) == -EINVAL);
        catat!(baca(h, usize::MAX - 2, &mut [0u8; 10]) == -EOVERFLOW);
        catat!(tulis(h, usize::MAX - 2, &[0u8; 10]) == -EOVERFLOW);
        catat!(lepas(h) == 0);
    }

    // --- panjang maksimum sah --------------------------------------------
    {
        let h = buat(HAK_BACA | HAK_TULIS, KAPASITAS).unwrap();
        let mut besar = [0u8; KAPASITAS];
        let mut k = 0;
        while k < KAPASITAS {
            besar[k] = (k & 0xFF) as u8;
            k += 1;
        }
        catat!(tulis(h, 0, &besar) == KAPASITAS as i32);
        let mut keluar = [0u8; KAPASITAS];
        catat!(baca(h, 0, &mut keluar) == KAPASITAS as i32);
        catat!(keluar == besar);
        catat!(lepas(h) == 0);
    }

    // --- kapasitas --------------------------------------------------------
    {
        catat!(buat(HAK_BACA, KAPASITAS + 1).is_err());
    }

    // --- diferensial vs oracle naif --------------------------------------
    let mut diukur: u32 = 0;
    let mut beda: u32 = 0;
    {
        let h = buat(HAK_BACA | HAK_TULIS, KAPASITAS).unwrap();
        let mut naif = Naif::baru(KAPASITAS);
        // LCG deterministik.
        let mut rng: u64 = 0x1234_5678_9ABC_DEF0;
        let mut i = 0u32;
        while i < 2000 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let op = (rng >> 60) & 1;
            let off = ((rng >> 8) as usize) % (KAPASITAS + 64);
            let len = ((rng >> 32) as usize) % 65; // 0..=64, muat penyangga
            let len = if i % 17 == 0 { 0 } else { len };
            if op == 0 {
                let mut a = [0u8; 64];
                let rc_a = baca(h, off, &mut a[..len]);
                let rc_n = naif.baca(off, len);
                if rc_a != rc_n {
                    beda += 1;
                }
            } else {
                let mut s = [0u8; 64];
                let mut k = 0;
                while k < len {
                    s[k] = (k as u8) ^ (i as u8);
                    k += 1;
                }
                let rc_a = tulis(h, off, &s[..len]);
                let rc_n = naif.tulis(off, &s[..len]);
                if rc_a != rc_n {
                    beda += 1;
                }
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
    catat!(counter(Counter::Baca) > 0);
    catat!(counter(Counter::Tulis) > 0);
    catat!(counter(Counter::Lepas) > 0);
    catat!(counter(Counter::TolakHandle) > 0);
    catat!(counter(Counter::TolakHak) > 0);
    catat!(counter(Counter::TolakRentang) > 0);

    // Determinisme: matriks ulang memberi jawaban identik.
    let a = counter(Counter::Buat);
    let sisa = buat(HAK_BACA, 8);
    if let Ok(h) = sisa {
        let _ = lepas(h);
    }
    catat!(counter(Counter::Buat) == a + 1);

    let status = if checks == passed { 0 } else { -1 };
    KlatosMemoriEfekReport {
        tag: REPORT_TAG,
        checks,
        passed,
        dif_ukur: diukur,
        dif_beda: beda,
        prod_buat: counter(Counter::Buat),
        prod_baca: counter(Counter::Baca),
        prod_tulis: counter(Counter::Tulis),
        prod_lepas: counter(Counter::Lepas),
        prod_tolak: counter(Counter::TolakHandle)
            + counter(Counter::TolakHak)
            + counter(Counter::TolakRentang)
            + counter(Counter::TolakKapasitas),
        status,
        reserved: 0,
    }
}

/// Titik masuk C ABI matriks bukti konteks efek memori.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_memori_efek_selftest() -> KlatosMemoriEfekReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4D45_4D4F);
        assert!(EBADF == 9 && EACCES == 13 && EINVAL == 22 && EMFILE == 24 && EOVERFLOW == 75);
        assert!(KAPASITAS == 4096);
        assert!(N_SLOT == 32);
        assert!(HAK_BACA == 1 && HAK_TULIS == 2);
        assert!(Counter::Buat as usize == 0);
        assert!(Counter::TolakKapasitas as usize == 7);
        assert!(N_COUNTER == 8);
        // Laporan: 8 (tag) + 11*4 = 44 -> align 8 -> 56.
        assert!(core::mem::size_of::<KlatosMemoriEfekReport>() == 56);
        assert!(core::mem::align_of::<KlatosMemoriEfekReport>() == 8);
    };
}
