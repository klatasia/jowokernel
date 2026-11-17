// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! KR-29 — **OTORITAS KEKEKALAN BENDERA MOUNT (sticky-immutable latch)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATPSNG".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5053_4E47;

/// Ajaib penanda blok terinisialisasi: "PSNG".
pub const AJAIB: u32 = 0x5053_4E47;

/// `MS_IMMUTABLE` — cermin `Jowo/Kontrak/POSIX/unistd.h:37` (`1 << 10`).
pub const MS_IMMUTABLE: i32 = 1 << 10;

/// `-EINVAL`.
pub const E_INVAL: i32 = 22;

// --- blok keadaan (disematkan di objek C++) ---------------------------------

/// Blok keadaan bendera mount. Disematkan di `Jowo::Mount`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BenderaPasang {
    /// Bendera efektif saat ini.
    pub bendera: i32,
    /// Latch: 1 bila `MS_IMMUTABLE` pernah diminta.
    pub tetap: u32,
    /// Ajaib penanda.
    pub ajaib: u32,
    /// Cadangan.
    pub cadangan: u32,
}

impl BenderaPasang {
    /// Blok kosong.
    pub const fn kosong() -> Self {
        BenderaPasang { bendera: 0, tetap: 0, ajaib: 0, cadangan: 0 }
    }
}

// --- penghitung audit -------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    ProdBuat = 0,
    ProdSet = 1,
    ProdTetap = 2,
    TolakParam = 3,
}
const N_COUNTER: usize = 4;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- kebijakan murni (dipakai produksi + matriks) ---------------------------

/// Terapkan kebijakan latch tulis-sekali. Mengembalikan bendera efektif.
#[inline]
pub fn terapkan(b: &mut BenderaPasang, flags: i32) -> i32 {
    if (flags & MS_IMMUTABLE) != 0 {
        b.tetap = 1;
    }
    b.bendera = flags;
    if b.tetap != 0 {
        b.bendera |= MS_IMMUTABLE;
    }
    b.bendera
}

/// Orakel historis: cermin literal `SetOnce` + `m_flags` C++. Dipakai HANYA
/// oleh matriks diferensial (struktur berbeda supaya kesepakatan bermakna).
#[inline]
pub fn kebijakan_lama(bendera: &mut i32, tetap: &mut bool, flags: i32) -> i32 {
    let mut set_flag = false;
    if (flags & MS_IMMUTABLE) != 0 {
        set_flag = true;
    }
    if set_flag {
        *tetap = true;
    }
    *bendera = flags;
    if *tetap {
        *bendera = *bendera | MS_IMMUTABLE;
    }
    *bendera
}

// --- pintu C ABI ------------------------------------------------------------

/// Inisialisasi blok (idempoten). `0` selalu.
///
/// # Safety
/// `k` harus dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_pasang_buat(k: *mut BenderaPasang) -> i32 {
    if k.is_null() {
        bump(Counter::TolakParam);
        return -E_INVAL;
    }
    let b = unsafe { &mut *k };
    *b = BenderaPasang::kosong();
    b.ajaib = AJAIB;
    bump(Counter::ProdBuat);
    0
}

/// Terapkan bendera. Mengembalikan bendera efektif. `-EINVAL` bila null.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_pasang_set(k: *mut BenderaPasang, flags: i32) -> i32 {
    if k.is_null() {
        bump(Counter::TolakParam);
        return -E_INVAL;
    }
    let b = unsafe { &mut *k };
    if b.ajaib != AJAIB {
        b.ajaib = AJAIB;
    }
    let r = terapkan(b, flags);
    bump(Counter::ProdSet);
    if b.tetap != 0 {
        bump(Counter::ProdTetap);
    }
    r
}

/// Bendera efektif saat ini.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_pasang_bendera(k: *const BenderaPasang) -> i32 {
    if k.is_null() {
        return 0;
    }
    unsafe { (*k).bendera }
}

/// `1` bila latch kekal pernah diset, `0` bila belum.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_pasang_tetap(k: *const BenderaPasang) -> u32 {
    if k.is_null() {
        return 0;
    }
    unsafe { (*k).tetap }
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_pasang_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
#[no_mangle]
pub extern "C" fn klatos_pasang_reset_counter() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas bendera mount (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosPasangReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_set: u32,
    pub prod_tetap: u32,
    pub prod_tolak: u32,
    pub cadangan0: u32,
    pub cadangan1: u32,
    pub cadangan2: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosPasangReport>()];

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
        Matrix { checks: 0, passed: 0, neg_checks: 0, neg_passed: 0, dif_checks: 0, dif_beda: 0 }
    }
    fn catat(&mut self, l: bool) {
        self.checks += 1;
        if l {
            self.passed += 1;
        }
    }
    fn neg(&mut self, l: bool) {
        self.neg_checks += 1;
        if l {
            self.neg_passed += 1;
        }
    }
    fn dif(&mut self, l: bool) {
        self.dif_checks += 1;
        if !l {
            self.dif_beda += 1;
        }
    }
}

fn run_matrix() -> KlatosPasangReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_5053_4E47);
    m.catat(AJAIB == 0x5053_4E47);
    m.catat(MS_IMMUTABLE == 1024);
    m.catat(E_INVAL == 22);
    m.catat(core::mem::size_of::<BenderaPasang>() == 16);

    let _ = klatos_pasang_reset_counter();

    // A) Dasar.
    let mut b = BenderaPasang::kosong();
    let pb = &mut b as *mut BenderaPasang;
    m.catat(unsafe { klatos_pasang_buat(pb) } == 0);
    m.catat(unsafe { klatos_pasang_bendera(pb) } == 0);
    m.catat(unsafe { klatos_pasang_tetap(pb) } == 0);

    // B) Set biasa (tanpa kekal).
    m.catat(unsafe { klatos_pasang_set(pb, 1 | 2 | 4) } == (1 | 2 | 4));
    m.catat(unsafe { klatos_pasang_bendera(pb) } == (1 | 2 | 4));
    m.catat(unsafe { klatos_pasang_tetap(pb) } == 0);
    // Set ulang menimpa.
    m.catat(unsafe { klatos_pasang_set(pb, 2) } == 2);
    m.catat(unsafe { klatos_pasang_tetap(pb) } == 0);

    // C) Set kekal.
    m.catat(unsafe { klatos_pasang_set(pb, MS_IMMUTABLE | 1) } == (MS_IMMUTABLE | 1));
    m.catat(unsafe { klatos_pasang_tetap(pb) } == 1);
    // Sticky: set berikutnya TANPA MS_IMMUTABLE tetap mengandungnya.
    m.catat(unsafe { klatos_pasang_set(pb, 2) } == (2 | MS_IMMUTABLE));
    m.catat(unsafe { klatos_pasang_set(pb, 0) } == MS_IMMUTABLE);
    m.catat(unsafe { klatos_pasang_tetap(pb) } == 1);
    // Set kekal berulang tetap idempoten.
    m.catat(unsafe { klatos_pasang_set(pb, MS_IMMUTABLE) } == MS_IMMUTABLE);

    // D) Param null.
    m.neg(unsafe { klatos_pasang_buat(core::ptr::null_mut()) } == -E_INVAL);
    m.neg(unsafe { klatos_pasang_set(core::ptr::null_mut(), 1) } == -E_INVAL);
    m.neg(unsafe { klatos_pasang_bendera(core::ptr::null()) } == 0);
    m.neg(unsafe { klatos_pasang_tetap(core::ptr::null()) } == 0);

    // E) Diferensial vs orakel historis (LCG deterministik).
    let mut br = BenderaPasang::kosong();
    let mut bendera_lama: i32 = 0;
    let mut tetap_lama = false;
    let mut seed: u64 = 0x2026_1005_0000_2911;
    let mut step = 0u32;
    while step < 20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        // Bendera acak dari bit rendah + paksa MS_IMMUTABLE kadang.
        let mut flags = (seed >> 7) as i32 & 0x7FF;
        if (seed >> 40) & 1 == 0 {
            flags |= MS_IMMUTABLE;
        }
        let rr = terapkan(&mut br, flags);
        let ro = kebijakan_lama(&mut bendera_lama, &mut tetap_lama, flags);
        m.dif(rr == ro);
        m.dif((br.tetap != 0) == tetap_lama);
        step += 1;
    }

    // F) Determinisme.
    {
        let mut x = BenderaPasang::kosong();
        let px = &mut x as *mut BenderaPasang;
        let _ = unsafe { klatos_pasang_buat(px) };
        let first = unsafe { klatos_pasang_set(px, 4) };
        let mut ok = true;
        for _ in 0..1000 {
            let mut y = BenderaPasang::kosong();
            let py = &mut y as *mut BenderaPasang;
            let _ = unsafe { klatos_pasang_buat(py) };
            if unsafe { klatos_pasang_set(py, 4) } != first {
                ok = false;
                break;
            }
        }
        m.neg(ok);
    }

    let prod_buat = COUNTERS[Counter::ProdBuat as usize].load(Ordering::Relaxed);
    let prod_set = COUNTERS[Counter::ProdSet as usize].load(Ordering::Relaxed);
    let prod_tetap = COUNTERS[Counter::ProdTetap as usize].load(Ordering::Relaxed);
    let prod_tolak = COUNTERS[Counter::TolakParam as usize].load(Ordering::Relaxed);
    let _ = klatos_pasang_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && m.dif_checks > 0;
    KlatosPasangReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_buat,
        prod_set,
        prod_tetap,
        prod_tolak,
        cadangan0: 0,
        cadangan1: 0,
        cadangan2: 0,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas bendera mount.
#[no_mangle]
pub extern "C" fn klatos_pasang_selftest() -> KlatosPasangReport {
    assert!(core::mem::size_of::<KlatosPasangReport>() == 64);
    assert!(core::mem::offset_of!(KlatosPasangReport, status) == 60);
    run_matrix()
}
