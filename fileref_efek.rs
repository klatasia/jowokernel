// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-13R — **PENYIMPAN BYTE FileRef** (jalur ABI v2 `FileRef::READ/WRITE`).


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering, AtomicU64};

use crate::memori_efek;

/// `EBADF` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 9).
pub const EBADF: i32 = 9;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `EMFILE` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 24).
pub const EMFILE: i32 = 24;
/// `EOVERFLOW` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 75).
pub const EOVERFLOW: i32 = 75;

/// Kapasitas byte satu objek berkas.
pub const KAPASITAS: usize = 4096;
/// Jumlah slot penyimpan.
pub const N_SLOT: usize = 32;

/// Sentinel "handle tidak sah".
pub const TAK_SAH: u64 = u64::MAX;

/// Penanda laporan; hanya ada di Rust.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4652_4546; // "KLATFREF"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Objek berkas dibuat.
    Buat = 0,
    /// Salinan baca berhasil.
    Baca = 1,
    /// Objek dilepas.
    Lepas = 2,
    /// Ditolak: handle tak sah / basi.
    TolakHandle = 3,
    /// Ditolak: rentang di luar batas / luber.
    TolakRentang = 4,
    /// Ditolak: sisi memori (hak/batas/handle).
    TolakMemori = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Arena: N_SLOT objek berkas, tiap objek KAPASITAS byte, dijaga spinlock
// ---------------------------------------------------------------------------

struct Slot {
    byte: [AtomicU8; KAPASITAS],
    panjang: AtomicU32,
    gen: AtomicU32,
    hidup: AtomicBool,
}

impl Slot {
    const fn baru() -> Self {
        Self {
            byte: [const { AtomicU8::new(0) }; KAPASITAS],
            panjang: AtomicU32::new(0),
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

const fn bungkus(index: usize, gen: u32) -> u64 {
    ((gen as u64) << 16) | (index as u64)
}

const fn indeks(h: u64) -> usize {
    (h & 0xFFFF) as usize
}

const fn generasi(h: u64) -> u32 {
    ((h >> 16) & 0xFFFF_FFFF) as u32
}

/// Validasi handle berkas. HARUS dipanggil dengan KUNCI dipegang.
fn slot_sah(h: u64) -> Result<usize, i32> {
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
    Ok(i)
}

// ---------------------------------------------------------------------------
// Operasi keadaan — API aman
// ---------------------------------------------------------------------------

/// Buat objek berkas berisi `isi`. Mengembalikan handle atau `-errno`.
pub fn buat(isi: &[u8]) -> Result<u64, i32> {
    if isi.len() > KAPASITAS {
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
            return Err(-EMFILE);
        }
    };
    let gen = ARENA[i].gen.load(Ordering::Relaxed).wrapping_add(1);
    let mut k = 0;
    while k < isi.len() {
        ARENA[i].byte[k].store(isi[k], Ordering::Relaxed);
        k += 1;
    }
    ARENA[i].panjang.store(isi.len() as u32, Ordering::Relaxed);
    ARENA[i].gen.store(gen, Ordering::Relaxed);
    ARENA[i].hidup.store(true, Ordering::Release);
    buka();
    bump(Counter::Buat);
    Ok(bungkus(i, gen))
}

/// Baca byte berkas ke `dst` (semantik `read(2)`): `off > panjang` → EINVAL,
/// `off == panjang` → EOF (0), selainnya `min(dst.len(), panjang - off)`.
/// Mengembalikan jumlah byte tersalin (>=0) atau `-errno`.
pub fn baca_ke_slice(h: u64, off: usize, dst: &mut [u8]) -> i32 {
    kunci();
    let i = match slot_sah(h) {
        Ok(i) => i,
        Err(e) => {
            buka();
            bump(Counter::TolakHandle);
            return e;
        }
    };
    let panjang = ARENA[i].panjang.load(Ordering::Relaxed) as usize;
    if dst.is_empty() {
        buka();
        bump(Counter::Baca);
        return 0;
    }
    if off > panjang {
        buka();
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    let tersedia = panjang - off;
    let n = if dst.len() < tersedia { dst.len() } else { tersedia };
    let mut k = 0;
    while k < n {
        dst[k] = ARENA[i].byte[off + k].load(Ordering::Relaxed);
        k += 1;
    }
    buka();
    bump(Counter::Baca);
    n as i32
}

/// Tulis `src` ke berkas pada offset `off`. Mengembalikan jumlah byte atau `-errno`.
pub fn tulis_dari_slice(h: u64, off: usize, src: &[u8]) -> i32 {
    kunci();
    let i = match slot_sah(h) {
        Ok(i) => i,
        Err(e) => {
            buka();
            bump(Counter::TolakHandle);
            return e;
        }
    };
    if src.is_empty() {
        buka();
        return 0;
    }
    let akhir = match off.checked_add(src.len()) {
        Some(a) => a,
        None => {
            buka();
            bump(Counter::TolakRentang);
            return -EOVERFLOW;
        }
    };
    if akhir > KAPASITAS {
        buka();
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    let mut k = 0;
    while k < src.len() {
        ARENA[i].byte[off + k].store(src[k], Ordering::Relaxed);
        k += 1;
    }
    if akhir > ARENA[i].panjang.load(Ordering::Relaxed) as usize {
        ARENA[i].panjang.store(akhir as u32, Ordering::Relaxed);
    }
    buka();
    src.len() as i32
}

/// Potongan vertikal NYATA: baca `len` byte dari berkas `fileref` pada `off`
/// dan tulis ke objek memori `mem` (mulai offset 0). Mengembalikan jumlah byte
/// tersalin (>=0) atau `-errno`.
///
/// Tidak ada nilai sintetis: byte berasal dari penyimpan berkas Rust dan
/// benar-benar disalin ke penyimpan memori Rust.
pub fn baca_ke_memori(fileref: u64, off: usize, mem: u64, len: usize) -> i32 {
    if len > KAPASITAS {
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    let mut buf = [0u8; KAPASITAS];
    let n = baca_ke_slice(fileref, off, &mut buf[..len]);
    if n < 0 {
        return n;
    }
    let rc = memori_efek::tulis(mem, 0, &buf[..n as usize]);
    if rc < 0 {
        bump(Counter::TolakMemori);
        return rc;
    }
    n
}

/// Kebalikan [`baca_ke_memori`]: baca `len` byte dari objek memori `mem`
/// (mulai offset 0) dan tulis ke berkas `fileref` pada `off`.
pub fn tulis_dari_memori(fileref: u64, off: usize, mem: u64, len: usize) -> i32 {
    if len > KAPASITAS {
        bump(Counter::TolakRentang);
        return -EINVAL;
    }
    let mut buf = [0u8; KAPASITAS];
    let n = memori_efek::baca(mem, 0, &mut buf[..len]);
    if n < 0 {
        bump(Counter::TolakMemori);
        return n;
    }
    tulis_dari_slice(fileref, off, &buf[..n as usize])
}

/// Panjang logis berkas, atau `-errno`.
pub fn panjang(h: u64) -> i32 {
    kunci();
    let i = match slot_sah(h) {
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

/// Lepas objek berkas. Mengembalikan `0` atau `-errno`.
pub fn lepas(h: u64) -> i32 {
    kunci();
    let i = match slot_sah(h) {
        Ok(i) => i,
        Err(e) => {
            buka();
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

/// Jumlah objek berkas hidup.
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

/// Buat objek berkas berisi `panjang` byte dari `isi`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_fileref_efek_buat(isi: *const u8, panjang: u32) -> u64 {
    if panjang as usize > KAPASITAS {
        return TAK_SAH;
    }
    if panjang == 0 {
        return buat(&[]).unwrap_or(TAK_SAH);
    }
    if isi.is_null() {
        return TAK_SAH;
    }
    // SAFETY: `isi` non-null dan `panjang` byte terbaca (kontrak C ABI).
    let s = unsafe { core::slice::from_raw_parts(isi, panjang as usize) };
    buat(s).unwrap_or(TAK_SAH)
}

/// Potongan vertikal ABI v2: baca berkas → objek memori.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_fileref_efek_baca_ke_memori(
    fileref: u64,
    off: u64,
    mem: u64,
    mem_off: u64,
    len: u32,
) -> i32 {
    // `mem_off` diterima demi kesetaraan tanda tangan seam; implementasi
    // potongan ini menulis mulai offset 0 (lihat dokumentasi modul). Nilai
    // bukan-nol ditolak jujur daripada diabaikan diam-diam.
    if mem_off != 0 {
        return -EINVAL;
    }
    baca_ke_memori(fileref, off as usize, mem, len as usize)
}

/// Panjang logis berkas.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_fileref_efek_panjang(fileref: u64) -> i32 {
    panjang(fileref)
}

/// Lepas objek berkas.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_fileref_efek_lepas(fileref: u64) -> i32 {
    lepas(fileref)
}

/// Jumlah objek berkas hidup.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_fileref_efek_hidup() -> u32 {
    hidup()
}

/// Nilai satu penghitung audit.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_fileref_efek_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset arena + penghitung. TIDAK dipanggil produksi.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_fileref_efek_reset() -> u32 {
    reset();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks bukti
// ---------------------------------------------------------------------------

/// Cermin `KlatosFileRefEfekReport`.
#[repr(C)]
pub struct KlatosFileRefEfekReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub dif_ukur: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_baca: u32,
    pub prod_lepas: u32,
    pub prod_tolak: u32,
    pub status: i32,
    pub reserved: u32,
}

/// Oracle pembanding deterministik — formulasi BERBEDA dari implementasi.
struct Naif {
    byte: [u8; KAPASITAS],
    panjang: usize,
    hidup: bool,
}

impl Naif {
    fn baru(isi: &[u8]) -> Self {
        let mut byte = [0u8; KAPASITAS];
        byte[..isi.len()].copy_from_slice(isi);
        Self {
            byte,
            panjang: isi.len(),
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
        if off > self.panjang {
            return -EINVAL;
        }
        let tersedia = self.panjang - off;
        (if len < tersedia { len } else { tersedia }) as i32
    }

    /// Salin byte ke `dst` (untuk perbandingan byte, bukan hanya kode).
    fn baca_ke(&self, off: usize, dst: &mut [u8]) -> i32 {
        let n = self.baca(off, dst.len());
        if n <= 0 {
            return n;
        }
        let mut k = 0;
        while k < n as usize {
            dst[k] = self.byte[off + k];
            k += 1;
        }
        n
    }
}

/// Matriks bukti.
pub fn run_matrix() -> KlatosFileRefEfekReport {
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
    catat!(REPORT_TAG == 0x4B4C_4154_4652_4546);
    catat!(EBADF == 9 && EINVAL == 22 && EMFILE == 24 && EOVERFLOW == 75);
    catat!(KAPASITAS == 4096 && N_SLOT == 32);

    // --- potongan vertikal NYATA: berkas → memori -------------------------
    {
        let isi: [u8; 200] = {
            let mut a = [0u8; 200];
            let mut k = 0;
            while k < 200 {
                a[k] = (k as u8).wrapping_mul(13).wrapping_add(7);
                k += 1;
            }
            a
        };
        let fr = buat(&isi).expect("buat fileref gagal");
        catat!(panjang(fr) == 200);
        let mem = memori_efek::buat(memori_efek::HAK_BACA | memori_efek::HAK_TULIS, 256)
            .expect("buat memori gagal");

        // Baca 200 byte dari awal.
        catat!(baca_ke_memori(fr, 0, mem, 200) == 200);
        let mut keluar = [0u8; 200];
        catat!(memori_efek::baca(mem, 0, &mut keluar) == 200);
        catat!(keluar == isi);

        // Baca sebagian pada offset.
        let mem2 = memori_efek::buat(memori_efek::HAK_BACA | memori_efek::HAK_TULIS, 64)
            .expect("buat memori2 gagal");
        catat!(baca_ke_memori(fr, 150, mem2, 64) == 50); // hanya 50 tersisa (EOF)
        let mut sep = [0u8; 50];
        catat!(memori_efek::baca(mem2, 0, &mut sep) == 50);
        catat!(sep[..] == isi[150..200]);

        // offset tepat di ujung → 0 (EOF).
        catat!(baca_ke_memori(fr, 200, mem2, 16) == 0);
        // offset melewati ujung → EINVAL.
        catat!(baca_ke_memori(fr, 201, mem2, 16) == -EINVAL);
        // memori terlalu kecil untuk hasil → EINVAL (fail-closed).
        let kecil = memori_efek::buat(memori_efek::HAK_BACA | memori_efek::HAK_TULIS, 8).unwrap();
        catat!(baca_ke_memori(fr, 0, kecil, 64) == -EINVAL);

        catat!(lepas(fr) == 0);
        catat!(panjang(fr) == -EBADF); // basi
        catat!(memori_efek::lepas(mem) == 0);
        catat!(memori_efek::lepas(mem2) == 0);
        catat!(memori_efek::lepas(kecil) == 0);
    }

    // --- handle tak sah / generasi basi -----------------------------------
    {
        catat!(baca_ke_slice(TAK_SAH, 0, &mut [0u8; 1]) == -EBADF);
        catat!(panjang(TAK_SAH) == -EBADF);
        catat!(lepas(TAK_SAH) == -EBADF);
        let luar = bungkus(N_SLOT + 3, 1);
        catat!(baca_ke_slice(luar, 0, &mut [0u8; 1]) == -EBADF);

        let h1 = buat(&[1, 2, 3, 4]).unwrap();
        catat!(lepas(h1) == 0);
        let h2 = buat(&[5, 6]).unwrap();
        catat!(h1 != h2);
        catat!(baca_ke_slice(h1, 0, &mut [0u8; 1]) == -EBADF);
        catat!(lepas(h2) == 0);
    }

    // --- panjang nol ------------------------------------------------------
    {
        let h = buat(&[]).unwrap();
        catat!(panjang(h) == 0);
        catat!(baca_ke_slice(h, 0, &mut []) == 0);
        catat!(baca_ke_slice(h, 0, &mut [0u8; 4]) == 0); // EOF langsung
        catat!(lepas(h) == 0);
    }

    // --- kebalikan: memori → berkas ---------------------------------------
    {
        let fr = buat(&[0u8; 32]).unwrap();
        let mem = memori_efek::buat(memori_efek::HAK_BACA | memori_efek::HAK_TULIS, 16).unwrap();
        let mut isi = [0u8; 16];
        let mut k = 0;
        while k < 16 {
            isi[k] = (k as u8).wrapping_add(0x40);
            k += 1;
        }
        catat!(memori_efek::tulis(mem, 0, &isi) == 16);
        catat!(tulis_dari_memori(fr, 8, mem, 16) == 16);
        let mut cek = [0u8; 16];
        catat!(baca_ke_slice(fr, 8, &mut cek) == 16);
        catat!(cek == isi);
        catat!(lepas(fr) == 0);
        catat!(memori_efek::lepas(mem) == 0);
    }

    // --- diferensial vs oracle naif --------------------------------------
    let mut diukur: u32 = 0;
    let mut beda: u32 = 0;
    {
        let mut isi = [0u8; 300];
        let mut k = 0;
        while k < 300 {
            isi[k] = (k as u8).wrapping_mul(7).wrapping_add(3);
            k += 1;
        }
        let fr = buat(&isi).unwrap();
        let naif = Naif::baru(&isi);
        let mut rng: u64 = 0x0FED_CBA9_8765_4321;
        let mut i = 0u32;
        while i < 2000 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let off = ((rng >> 8) as usize) % 400;
            let len = ((rng >> 32) as usize) % 65;
            let len = if i % 17 == 0 { 0 } else { len };
            let mut a = [0u8; 64];
            let mut b = [0u8; 64];
            let rc_a = baca_ke_slice(fr, off, &mut a[..len]);
            let rc_n = naif.baca_ke(off, &mut b[..len]);
            if rc_a != rc_n {
                beda += 1;
            } else if rc_a > 0 && a[..rc_a as usize] != b[..rc_a as usize] {
                beda += 1; // byte NYATA harus identik, bukan hanya kode hasil
            }
            diukur += 1;
            i += 1;
        }
        catat!(diukur == 2000);
        catat!(beda == 0);
        catat!(lepas(fr) == 0);
    }

    // --- penghitung bergerak ---------------------------------------------
    catat!(counter(Counter::Buat) > 0);
    catat!(counter(Counter::Baca) > 0);
    catat!(counter(Counter::Lepas) > 0);
    catat!(counter(Counter::TolakHandle) > 0);
    catat!(counter(Counter::TolakRentang) > 0);
    catat!(counter(Counter::TolakMemori) > 0);

    let a = counter(Counter::Buat);
    let sisa = buat(&[9]);
    if let Ok(h) = sisa {
        let _ = lepas(h);
    }
    catat!(counter(Counter::Buat) == a + 1);

    let status = if checks == passed { 0 } else { -1 };
    KlatosFileRefEfekReport {
        tag: REPORT_TAG,
        checks,
        passed,
        dif_ukur: diukur,
        dif_beda: beda,
        prod_buat: counter(Counter::Buat),
        prod_baca: counter(Counter::Baca),
        prod_lepas: counter(Counter::Lepas),
        prod_tolak: counter(Counter::TolakHandle)
            + counter(Counter::TolakRentang)
            + counter(Counter::TolakMemori),
        status,
        reserved: 0,
    }
}

/// Titik masuk C ABI matriks bukti penyimpan byte FileRef.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_fileref_efek_selftest() -> KlatosFileRefEfekReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4652_4546);
        assert!(EBADF == 9 && EINVAL == 22 && EMFILE == 24 && EOVERFLOW == 75);
        assert!(KAPASITAS == 4096);
        assert!(N_SLOT == 32);
        assert!(Counter::Buat as usize == 0);
        assert!(Counter::TolakMemori as usize == 5);
        assert!(N_COUNTER == 6);
        // Laporan: 8 (tag) + 10*4 = 40 -> align 8 -> 48.
        assert!(core::mem::size_of::<KlatosFileRefEfekReport>() == 48);
        assert!(core::mem::align_of::<KlatosFileRefEfekReport>() == 8);
    };
}
