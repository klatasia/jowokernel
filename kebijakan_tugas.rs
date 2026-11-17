// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

// KR-30 — **OTORITAS KEBIJAKAN TUGAS (crash / fs-init / finalizer / kustodi)**.



#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATTGSK".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5447_534B;

/// Perutean dasar kustodi.
pub const KUSTODI_EKSPLISIT: i32 = 0;
pub const KUSTODI_ROOT: i32 = 1;
pub const KUSTODI_DIRFD: i32 = 2;

// --- penghitung audit -------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    ProdCrash = 0,
    ProdFsInit = 1,
    ProdFinalizer = 2,
    ProdKustodi = 3,
    TolakParam = 4,
}
const N_COUNTER: usize = 5;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- kebijakan murni --------------------------------------------------------

/// Kelayakan pengiriman sinyal urgent pada crash.
#[inline]
pub fn crash_layak(crashed_in_kernel: bool, has_handler: bool, ignore: bool, masked: bool) -> bool {
    !crashed_in_kernel && has_handler && !ignore && !masked
}

/// Apakah filesystem perlu init-lazy (belum diinisialisasi).
#[inline]
pub fn fs_perlu_init(sudah_init: bool) -> bool {
    !sudah_init
}

/// Konsumsi sekali-pakai `has_work`.
#[inline]
pub fn finalizer_konsumsi(has_work: bool) -> bool {
    has_work
}

/// Perutean dasar kustodi.
#[inline]
pub fn kustodi_dasar(has_base: bool, is_absolute: bool) -> i32 {
    if has_base {
        KUSTODI_EKSPLISIT
    } else if is_absolute {
        KUSTODI_ROOT
    } else {
        KUSTODI_DIRFD
    }
}

// --- pintu C ABI ------------------------------------------------------------

/// Kelayakan sinyal crash (1 = kirim).
#[no_mangle]
pub extern "C" fn klatos_tugask_crash_layak(ck: u32, hh: u32, ig: u32, mk: u32) -> u32 {
    bump(Counter::ProdCrash);
    if crash_layak(ck != 0, hh != 0, ig != 0, mk != 0) { 1 } else { 0 }
}

/// Perlu init (1 = belum, lakukan init).
#[no_mangle]
pub extern "C" fn klatos_tugask_fs_perlu_init(sudah_init: u32) -> u32 {
    bump(Counter::ProdFsInit);
    if fs_perlu_init(sudah_init != 0) { 1 } else { 0 }
}

/// Konsumsi `has_work` (menyetel *has_work=0 bila dikonsumsi). `1` bila dikonsumsi.
///
/// # Safety
/// `has_work` menunjuk u32 yang dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_tugask_finalizer_konsumsi(has_work: *mut u32) -> u32 {
    if has_work.is_null() {
        bump(Counter::TolakParam);
        return 0;
    }
    let hw = unsafe { *has_work };
    if finalizer_konsumsi(hw != 0) {
        unsafe { core::ptr::write(has_work, 0) };
        bump(Counter::ProdFinalizer);
        1
    } else {
        0
    }
}

/// Perutean dasar kustodi (`KUSTODI_*`).
#[no_mangle]
pub extern "C" fn klatos_tugask_kustodi_dasar(has_base: u32, is_absolute: u32) -> i32 {
    bump(Counter::ProdKustodi);
    kustodi_dasar(has_base != 0, is_absolute != 0)
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_tugask_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
#[no_mangle]
pub extern "C" fn klatos_tugask_reset_counter() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas kebijakan tugas (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosTugasReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_crash: u32,
    pub prod_fs_init: u32,
    pub prod_finalizer: u32,
    pub prod_kustodi: u32,
    pub cadangan0: u32,
    pub cadangan1: u32,
    pub cadangan2: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosTugasReport>()];

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

fn run_matrix() -> KlatosTugasReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_5447_534B);

    let _ = klatos_tugask_reset_counter();

    // A) Kelayakan sinyal crash.
    m.catat(crash_layak(false, true, false, false));
    m.neg(!crash_layak(true, true, false, false));  // di kernel
    m.neg(!crash_layak(false, false, false, false)); // tanpa handler
    m.neg(!crash_layak(false, true, true, false));   // diabaikan
    m.neg(!crash_layak(false, true, false, true));   // ter-mask
    m.catat(klatos_tugask_crash_layak(0, 1, 0, 0) == 1);
    m.catat(klatos_tugask_crash_layak(1, 1, 0, 0) == 0);

    // B) Init-lazy.
    m.catat(fs_perlu_init(false));
    m.neg(!fs_perlu_init(true));
    m.catat(klatos_tugask_fs_perlu_init(0) == 1);
    m.catat(klatos_tugask_fs_perlu_init(1) == 0);

    // C) Konsumsi has_work (mutasi).
    {
        let mut hw: u32 = 1;
        m.catat(unsafe { klatos_tugask_finalizer_konsumsi(&mut hw) } == 1);
        m.catat(hw == 0);
        m.catat(unsafe { klatos_tugask_finalizer_konsumsi(&mut hw) } == 0); // sudah kosong
        m.neg(unsafe { klatos_tugask_finalizer_konsumsi(core::ptr::null_mut()) } == 0);
    }

    // D) Perutean kustodi.
    m.catat(kustodi_dasar(true, true) == KUSTODI_EKSPLISIT);
    m.catat(kustodi_dasar(false, true) == KUSTODI_ROOT);
    m.catat(kustodi_dasar(false, false) == KUSTODI_DIRFD);
    m.catat(klatos_tugask_kustodi_dasar(0, 0) == KUSTODI_DIRFD);

    // E) Diferensial vs orakel (LCG deterministik).
    let mut seed: u64 = 0x2026_1005_0000_3004;
    let mut step = 0u32;
    while step < 20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let ck = (seed >> 3) & 1 == 1;
        let hh = (seed >> 4) & 1 == 1;
        let ig = (seed >> 5) & 1 == 1;
        let mk = (seed >> 6) & 1 == 1;
        let r = crash_layak(ck, hh, ig, mk);
        let o = !ck && hh && !ig && !mk;
        m.dif(r == o);
        step += 1;
    }

    // F) Determinisme.
    {
        let first = kustodi_dasar(false, true);
        let mut ok = true;
        for _ in 0..1000 {
            if kustodi_dasar(false, true) != first {
                ok = false;
                break;
            }
        }
        m.neg(ok);
    }

    let prod_crash = COUNTERS[Counter::ProdCrash as usize].load(Ordering::Relaxed);
    let prod_fs_init = COUNTERS[Counter::ProdFsInit as usize].load(Ordering::Relaxed);
    let prod_finalizer = COUNTERS[Counter::ProdFinalizer as usize].load(Ordering::Relaxed);
    let prod_kustodi = COUNTERS[Counter::ProdKustodi as usize].load(Ordering::Relaxed);
    let _ = klatos_tugask_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && m.dif_checks > 0;
    KlatosTugasReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_crash,
        prod_fs_init,
        prod_finalizer,
        prod_kustodi,
        cadangan0: 0,
        cadangan1: 0,
        cadangan2: 0,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas kebijakan tugas.
#[no_mangle]
pub extern "C" fn klatos_tugask_selftest() -> KlatosTugasReport {
    assert!(core::mem::size_of::<KlatosTugasReport>() == 64);
    assert!(core::mem::offset_of!(KlatosTugasReport, status) == 60);
    run_matrix()
}
