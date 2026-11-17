// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT



#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Tag laporan; cermin `KlatosBerkasMayaReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_424D_4159; // "KLATBMAY"

/// Ukuran halaman; cermin `PAGE_SIZE` C++. Hanya untuk gerbang admisi.
pub const PAGE_SIZE: u64 = 4096;

/// Batas `ssize_t` positif (`NumericLimits<ssize_t>::max()`).
pub const SSIZE_MAX: u64 = i64::MAX as u64;

/// Jumlah slot objek maksimum. Tetap, tanpa alokasi.
pub const MAX_OBJEK: usize = 16_384;

/// Indeks tidak sah (belum terdaftar).
pub const INVALID: u32 = 0xFFFF_FFFF;

// --- status siklus hidup ---------------------------------------------------

/// Objek hidup (handle sah).
pub const AS_HIDUP: u32 = 0;
/// Objek dilepas (mati).
pub const AS_MATI: u32 = 1;

// --- verdict `buat`/`lepas` ------------------------------------------------

/// Berhasil.
pub const BM_OK: u32 = 0;
/// `size` tidak sah (⇒ `EINVAL`).
pub const BM_EINVAL: u32 = 1;
/// Arena penuh (⇒ `ENOMEM`).
pub const BM_ENOMEM: u32 = 2;
/// Handle usang/aspal (index/generasi tidak cocok).
pub const BM_EBADF: u32 = 3;

// --- pengkodean `create_admisi` --------------------------------------------
//
// `klatos_bmaya_create_admisi` mengembalikan `u64`:
//   * `0`  ⇒ `EINVAL` (ukuran tidak sah);
//   * `1`  ⇒ `ENOMEM` (arena penuh);
//   * selainnya ⇒ handle `(generasi << 32) | indeks`, generasi >= 1.
// Karena indeks < MAX_OBJEK dan generasi >= 1, handle selalu >= 1<<32 > 1,
// sehingga dua sentinel tidak pernah bertabrakan dengan handle yang sah.
/// Sentinel `EINVAL` untuk `create_admisi`.
pub const ADMISI_EINVAL: u64 = 0;
/// Sentinel `ENOMEM` untuk `create_admisi`.
pub const ADMISI_ENOMEM: u64 = 1;

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_bmaya_buat` dari produksi.
    ProdBuat = 0,
    /// Jumlah `klatos_bmaya_lepas` yang berhasil.
    ProdLepas = 1,
    /// Jumlah admisi yang ditolak karena ukuran (`EINVAL`).
    ProdTolakUkuran = 2,
    /// Jumlah admisi yang ditolak karena arena penuh (`ENOMEM`).
    ProdTolakPenuh = 3,
    /// Jumlah operasi yang menolak handle usang/aspal.
    ProdTolakStale = 4,
    /// Jumlah `klatos_bmaya_create_admisi` yang menghasilkan handle.
    ProdAdmisiOk = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Arena — satu-satunya pemilik identitas objek berkas tanpa nama
// ---------------------------------------------------------------------------

static OBJ_GEN: [AtomicU32; MAX_OBJEK] = [const { AtomicU32::new(0) }; MAX_OBJEK];
static OBJ_STATUS: [AtomicU32; MAX_OBJEK] = [const { AtomicU32::new(AS_MATI) }; MAX_OBJEK];
static OBJ_VMO_GEN: [AtomicU32; MAX_OBJEK] = [const { AtomicU32::new(0) }; MAX_OBJEK];
static OBJ_SIZE: [AtomicU64; MAX_OBJEK] = [const { AtomicU64::new(0) }; MAX_OBJEK];
static OBJ_FLAGS: [AtomicU32; MAX_OBJEK] = [const { AtomicU32::new(0) }; MAX_OBJEK];
static LOCK: AtomicU32 = AtomicU32::new(0);
/// Status interupsi pemegang LOCK; hanya pemegang yang menulis.
static LOCK_FLAGS: AtomicU64 = AtomicU64::new(0);

fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while LOCK.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    LOCK_FLAGS.store(flags, Ordering::Relaxed);
}

fn unlock() {
    let flags = LOCK_FLAGS.load(Ordering::Relaxed);
    LOCK.store(0, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Apakah `(index, generation)` menunjuk objek hidup yang sama?
#[inline]
fn hidup(index: u32, generation: u32) -> bool {
    let i = index as usize;
    generation != 0
        && i < MAX_OBJEK
        && OBJ_STATUS[i].load(Ordering::Acquire) == AS_HIDUP
        && OBJ_GEN[i].load(Ordering::Acquire) == generation
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Gerbang ukuran berkas tanpa nama (dulu `admisi_mem2::anon_ukuran`).
///
/// `1` bila ukuran sah, `0` bila tidak. Aturan persis C++ historis: nol,
/// tidak kelipatan `PAGE_SIZE`, atau melebihi `SSIZE_MAX` semuanya `EINVAL`.
pub const fn ukuran_sah(size: u64) -> u32 {
    if size == 0 {
        return 0;
    }
    if size % PAGE_SIZE != 0 {
        return 0;
    }
    if size > SSIZE_MAX {
        return 0;
    }
    1
}

/// Verdict admisi ukuran: [`BM_EINVAL`] bila tidak sah, [`BM_OK`] bila sah.
pub const fn admisi_ukuran(size: u64) -> u32 {
    if ukuran_sah(size) != 0 {
        BM_OK
    } else {
        BM_EINVAL
    }
}

/// Membuat objek baru. Mengembalikan handle `(generasi << 32) | indeks` atau
/// `0` bila arena penuh. `vmo_gen` adalah generasi `AnonymousVMObject`
/// (0 bila belum terdaftar).
pub fn buat(vmo_gen: u32, size: u64, flags: u32) -> u64 {
    lock();
    let mut slot = None;
    let mut i = 0usize;
    while i < MAX_OBJEK {
        if OBJ_STATUS[i].load(Ordering::Relaxed) == AS_MATI {
            slot = Some(i);
            break;
        }
        i += 1;
    }
    let result = if let Some(slot) = slot {
        let old = OBJ_GEN[slot].load(Ordering::Relaxed);
        let generation = old.wrapping_add(1);
        if generation == 0 {
            // Generasi membalik ke 0 (tidak akan terjadi dalam praktik):
            // pertahankan slot mati agar handle tidak pernah ambigu.
            0
        } else {
            OBJ_VMO_GEN[slot].store(vmo_gen, Ordering::Relaxed);
            OBJ_SIZE[slot].store(size, Ordering::Relaxed);
            OBJ_FLAGS[slot].store(flags, Ordering::Relaxed);
            OBJ_GEN[slot].store(generation, Ordering::Relaxed);
            OBJ_STATUS[slot].store(AS_HIDUP, Ordering::Release);
            ((generation as u64) << 32) | slot as u64
        }
    } else {
        0
    };
    unlock();
    result
}

/// Melepas objek. `1` bila berhasil, `0` bila handle usang/aspal.
pub fn lepas(index: u32, generation: u32) -> u32 {
    if !hidup(index, generation) {
        return 0;
    }
    lock();
    let i = index as usize;
    if !hidup(index, generation) {
        unlock();
        return 0;
    }
    OBJ_STATUS[i].store(AS_MATI, Ordering::Release);
    OBJ_VMO_GEN[i].store(0, Ordering::Relaxed);
    OBJ_SIZE[i].store(0, Ordering::Relaxed);
    OBJ_FLAGS[i].store(0, Ordering::Relaxed);
    unlock();
    1
}

/// Membaca generasi VMO yang diikat ke objek, atau [`INVALID`] bila usang.
pub fn vmo_gen_of(index: u32, generation: u32) -> u32 {
    if !hidup(index, generation) {
        return INVALID;
    }
    OBJ_VMO_GEN[index as usize].load(Ordering::Acquire)
}

/// Membaca ukuran objek, atau 0 bila usang.
pub fn size_of(index: u32, generation: u32) -> u64 {
    if !hidup(index, generation) {
        return 0;
    }
    OBJ_SIZE[index as usize].load(Ordering::Acquire)
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

/// Gerbang ukuran. `1` = sah.
#[no_mangle]
pub extern "C" fn klatos_bmaya_ukuran_sah(size: u64) -> u32 {
    let r = ukuran_sah(size);
    if r == 0 {
        bump(Counter::ProdTolakUkuran);
    }
    r
}

/// Membuat objek. `0` = gagal (arena penuh).
#[no_mangle]
pub extern "C" fn klatos_bmaya_buat(vmo_gen: u32, size: u64, flags: u32) -> u64 {
    let h = buat(vmo_gen, size, flags);
    if h == 0 {
        bump(Counter::ProdTolakPenuh);
    } else {
        bump(Counter::ProdBuat);
    }
    h
}

/// Apakah `(index, generation)` menunjuk objek hidup? `1`/`0`.
#[no_mangle]
pub extern "C" fn klatos_bmaya_hidup(index: u32, generation: u32) -> u32 {
    let r = if hidup(index, generation) { 1 } else { 0 };
    if r == 0 {
        bump(Counter::ProdTolakStale);
    }
    r
}

/// Melepas objek. `1` berhasil, `0` usang/aspal.
#[no_mangle]
pub extern "C" fn klatos_bmaya_lepas(index: u32, generation: u32) -> u32 {
    let r = lepas(index, generation);
    if r == 1 {
        bump(Counter::ProdLepas);
    } else {
        bump(Counter::ProdTolakStale);
    }
    r
}

/// Generasi VMO yang diikat, atau [`INVALID`].
#[no_mangle]
pub extern "C" fn klatos_bmaya_vmo_gen(index: u32, generation: u32) -> u32 {
    vmo_gen_of(index, generation)
}

/// Ukuran objek, atau 0 bila usang.
#[no_mangle]
pub extern "C" fn klatos_bmaya_size(index: u32, generation: u32) -> u64 {
    size_of(index, generation)
}

/// Admisi penuh pembuatan berkas tanpa nama: ukuran + pembuatan objek.
///
/// `0` ⇒ `EINVAL`; `1` ⇒ `ENOMEM`; selainnya ⇒ handle objek.
#[no_mangle]
pub extern "C" fn klatos_bmaya_create_admisi(size: u64, vmo_gen: u32) -> u64 {
    if ukuran_sah(size) == 0 {
        bump(Counter::ProdTolakUkuran);
        return ADMISI_EINVAL;
    }
    let h = buat(vmo_gen, size, 0);
    if h == 0 {
        bump(Counter::ProdTolakPenuh);
        ADMISI_ENOMEM
    } else {
        bump(Counter::ProdAdmisiOk);
        h
    }
}

/// Jumlah objek hidup saat ini.
#[no_mangle]
pub extern "C" fn klatos_bmaya_jumlah() -> u32 {
    let mut n = 0u32;
    let mut i = 0usize;
    while i < MAX_OBJEK {
        n += if OBJ_STATUS[i].load(Ordering::Relaxed) == AS_HIDUP {
            1
        } else {
            0
        };
        i += 1;
    }
    n
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_bmaya_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTERS[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_bmaya_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosBerkasMayaReport` di `KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosBerkasMayaReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_lepas: u32,
    pub prod_tolak_ukuran: u32,
    pub prod_tolak_penuh: u32,
    pub prod_tolak_stale: u32,
    pub prod_admisi_ok: u32,
    pub status: i32,
}

const _: [(); 64] = [(); core::mem::size_of::<KlatosBerkasMayaReport>()];

struct Matrix {
    checks: u32,
    passed: u32,
    neg_checks: u32,
    neg_passed: u32,
    dif_checks: u32,
    dif_beda: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            neg_checks: 0,
            neg_passed: 0,
            dif_checks: 0,
            dif_beda: 0,
        }
    }
    fn catat(&mut self, lulus: bool) {
        self.checks += 1;
        if lulus {
            self.passed += 1;
        }
    }
    fn neg(&mut self, lulus: bool) {
        self.neg_checks += 1;
        if lulus {
            self.neg_passed += 1;
        }
        self.catat(lulus);
    }
    fn dif(&mut self, lulus: bool) {
        self.dif_checks += 1;
        if !lulus {
            self.dif_beda += 1;
        }
        self.catat(lulus);
    }
}

/// Matriks otoritas siklus hidup berkas tanpa nama. Input SINTETIS.
fn run_matrix() -> KlatosBerkasMayaReport {
    let mut m = Matrix::new();

    // --- G1: konstanta ------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_424D_4159);
    m.catat(PAGE_SIZE == 4096);
    m.catat(SSIZE_MAX == i64::MAX as u64);
    m.catat(ADMISI_EINVAL == 0 && ADMISI_ENOMEM == 1);
    m.catat(AS_HIDUP == 0 && AS_MATI == 1);
    m.catat(BM_OK == 0 && BM_EINVAL == 1 && BM_ENOMEM == 2 && BM_EBADF == 3);

    // --- G2: gerbang ukuran -------------------------------------------------
    m.catat(ukuran_sah(0) == 0);
    m.catat(ukuran_sah(4096) == 1);
    m.catat(ukuran_sah(1) == 0);
    m.catat(ukuran_sah(4095) == 0);
    m.catat(ukuran_sah(4097) == 0);
    m.catat(ukuran_sah(1 << 20) == 1);
    m.catat(ukuran_sah(SSIZE_MAX) == 0); // tidak kelipatan PAGE_SIZE
    m.catat(ukuran_sah(SSIZE_MAX - (SSIZE_MAX % PAGE_SIZE)) == 1);
    m.catat(ukuran_sah(SSIZE_MAX + 1) == 0); // luber
    m.catat(ukuran_sah(u64::MAX) == 0);
    m.catat(admisi_ukuran(0) == BM_EINVAL);
    m.catat(admisi_ukuran(4096) == BM_OK);

    // --- G3: siklus hidup buat/hidup/lepas ----------------------------------
    let h = buat(0xA1, 4096, 0);
    m.catat(h != 0);
    let i0 = h as u32;
    let g0 = (h >> 32) as u32;
    m.catat(g0 >= 1);
    m.catat(hidup(i0, g0));
    m.catat(vmo_gen_of(i0, g0) == 0xA1);
    m.catat(size_of(i0, g0) == 4096);
    m.catat(lepas(i0, g0) == 1);
    m.catat(!hidup(i0, g0));

    // --- G4: negatif — usang/aspal/generasi salah ---------------------------
    // lepas dua kali
    m.neg(lepas(i0, g0) == 0);
    // hidup pada objek yang sudah dilepas
    m.neg(!hidup(i0, g0));
    // generasi salah pada indeks benar
    m.neg(!hidup(i0, g0.wrapping_add(1)));
    // generasi 0 selalu tidak sah
    m.neg(!hidup(i0, 0));
    // indeks di luar batas
    m.neg(!hidup(MAX_OBJEK as u32, 1));
    m.neg(!hidup(INVALID, 1));
    // lepas aspal
    m.neg(lepas(INVALID, 1) == 0);
    m.neg(lepas(MAX_OBJEK as u32, 1) == 0);
    // size_of/vmo_gen_of pada handle usang
    m.neg(size_of(i0, g0) == 0);
    m.neg(vmo_gen_of(i0, g0) == INVALID);

    // --- G5: pemakaian ulang slot menaikkan generasi (anti-ABA) -------------
    let h1 = buat(0xB2, 8192, 0);
    m.catat(h1 != 0);
    let i1 = h1 as u32;
    let g1 = (h1 >> 32) as u32;
    // slot pertama yang bebas harus sama dengan slot sebelumnya
    m.catat(i1 == i0);
    m.catat(g1 == g0.wrapping_add(1));
    // handle LAMA (generasi lama) kini ditolak
    m.neg(!hidup(i1, g0));
    m.catat(hidup(i1, g1));
    m.catat(lepas(i1, g1) == 1);

    // --- G6: admisi create + kapasitas penuh --------------------------------
    m.catat(klatos_bmaya_create_admisi(0, 0) == ADMISI_EINVAL);
    m.catat(klatos_bmaya_create_admisi(1, 0) == ADMISI_EINVAL);
    m.catat(klatos_bmaya_create_admisi(u64::MAX, 0) == ADMISI_EINVAL);
    let ah = klatos_bmaya_create_admisi(4096, 0xC3);
    m.catat(ah > 1);
    let ai = ah as u32;
    let ag = (ah >> 32) as u32;
    m.catat(hidup(ai, ag));
    m.catat(vmo_gen_of(ai, ag) == 0xC3);
    m.catat(lepas(ai, ag) == 1);

    // Isi arena sampai penuh, verifikasi ENOMEM, lalu kosongkan kembali.
    {
        let mut dibuat = 0u32;
        let mut hh = buat(0, 4096, 0);
        while hh != 0 {
            dibuat += 1;
            hh = buat(0, 4096, 0);
        }
        // Setelah loop, `hh == 0` karena arena penuh.
        m.catat(dibuat as usize <= MAX_OBJEK);
        m.catat(klatos_bmaya_jumlah() as usize == MAX_OBJEK);
        m.catat(klatos_bmaya_create_admisi(4096, 0) == ADMISI_ENOMEM);
        // Kosongkan seluruh arena: lepas tiap slot dengan generasi terkini.
        let mut i = 0u32;
        let mut dilepas = 0u32;
        while (i as usize) < MAX_OBJEK {
            let g = OBJ_GEN[i as usize].load(Ordering::Acquire);
            if OBJ_STATUS[i as usize].load(Ordering::Acquire) == AS_HIDUP {
                if lepas(i, g) == 1 {
                    dilepas += 1;
                }
            }
            i += 1;
        }
        m.catat(dilepas as usize == MAX_OBJEK);
        m.catat(klatos_bmaya_jumlah() == 0);
    }

    // --- G7: diferensial ukuran (referensi independen) ----------------------
    // Referensi ditulis ulang di sini, TIDAK memakai `ukuran_sah`.
    let ref_sah = |size: u64| -> u32 {
        let bukan_nol = (size != 0) as u32;
        let kelipatan = ((size % 4096) == 0) as u32;
        let tidak_luber = (size <= (i64::MAX as u64)) as u32;
        bukan_nol & kelipatan & tidak_luber
    };
    let mut probe: u64 = 0xC0FFEE;
    let mut n = 0u32;
    while n < 100_000 {
        probe = probe
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // campur nilai batas secara berkala
        let size = match n % 8 {
            0 => 0,
            1 => 4096,
            2 => 1,
            3 => probe & 0xFFFF,
            4 => probe & 0xFFFF_FFFF,
            5 => (i64::MAX as u64) & !0xFFFu64,
            6 => u64::MAX,
            _ => probe,
        };
        m.dif(ukuran_sah(size) == ref_sah(size));
        n += 1;
    }

    let status = if m.checks == m.passed && m.dif_beda == 0 { 0 } else { -1 };

    KlatosBerkasMayaReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_buat: klatos_bmaya_counter(Counter::ProdBuat as u32),
        prod_lepas: klatos_bmaya_counter(Counter::ProdLepas as u32),
        prod_tolak_ukuran: klatos_bmaya_counter(Counter::ProdTolakUkuran as u32),
        prod_tolak_penuh: klatos_bmaya_counter(Counter::ProdTolakPenuh as u32),
        prod_tolak_stale: klatos_bmaya_counter(Counter::ProdTolakStale as u32),
        prod_admisi_ok: klatos_bmaya_counter(Counter::ProdAdmisiOk as u32),
        status,
    }
}

/// Menjalankan matriks otoritas siklus hidup berkas tanpa nama.
#[no_mangle]
pub extern "C" fn klatos_bmaya_selftest() -> KlatosBerkasMayaReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_424D_4159);
        assert!(PAGE_SIZE == 4096);
        assert!(MAX_OBJEK == 16_384);
        assert!(ADMISI_EINVAL == 0);
        assert!(ADMISI_ENOMEM == 1);
        assert!(AS_HIDUP == 0);
        assert!(AS_MATI == 1);
        assert!(BM_EBADF == 3);
        assert!(Counter::ProdBuat as usize == 0);
        assert!(Counter::ProdAdmisiOk as usize == 5);
        assert!(N_COUNTER == 6);

        // Gerbang ukuran pada waktu kompilasi.
        assert!(ukuran_sah(0) == 0);
        assert!(ukuran_sah(4096) == 1);
        assert!(ukuran_sah(1) == 0);
        assert!(ukuran_sah(SSIZE_MAX) == 0);
        assert!(ukuran_sah(SSIZE_MAX + 1) == 0);

        // 8 (tag) + 13*4 (u32) = 60 -> padding ke 64.
        assert!(core::mem::size_of::<KlatosBerkasMayaReport>() == 64);
        assert!(core::mem::align_of::<KlatosBerkasMayaReport>() == 8);
    };
}
