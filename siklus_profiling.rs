// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


// KR-30 — **OTORITAS GERBANG PROFILING PANGGILAN SISTEM**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATPRFL".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5052_464C;

/// Kode pemilihan jalur.
pub const JALUR_ORIGINAL: u32 = 0;
pub const JALUR_PSEUDO: u32 = 1;
pub const JALUR_INVALID: u32 = 2;

// --- penghitung audit -------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    ProdGerbang = 0,
    ProdHasil = 1,
    ProdJalur = 2,
    ProdCatat = 3,
    TolakParam = 4,
}
const N_COUNTER: usize = 5;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- kebijakan murni --------------------------------------------------------

/// Gerbang admission profiling.
#[inline]
pub fn gerbang(profiling_enabled: bool, suppressed: bool, buf_null: bool) -> bool {
    profiling_enabled && !suppressed && !buf_null
}

/// Penyandian hasil: `1` bila galat (`rc < 0`), `0` bila sukses.
#[inline]
pub fn hasil_error(rc: i64) -> u32 {
    if rc < 0 { 1 } else { 0 }
}

/// Nilai hasil tersandi: kode errno bila galat, `0` bila sukses.
#[inline]
pub fn hasil_nilai(rc: i64) -> i64 {
    if rc < 0 { -rc } else { 0 }
}

/// Pemilihan jalur.
#[inline]
pub fn jalur_pilih(has_original: bool, has_pseudo: bool) -> u32 {
    if has_original {
        JALUR_ORIGINAL
    } else if has_pseudo {
        JALUR_PSEUDO
    } else {
        JALUR_INVALID
    }
}

// --- pintu C ABI ------------------------------------------------------------

/// Gerbang admission profiling (1 = catat).
#[no_mangle]
pub extern "C" fn klatos_prof_gerbang(pe: u32, sup: u32, buf_null: u32) -> u32 {
    bump(Counter::ProdGerbang);
    if gerbang(pe != 0, sup != 0, buf_null != 0) { 1 } else { 0 }
}

/// Penanda galat hasil (1 = galat).
#[no_mangle]
pub extern "C" fn klatos_prof_hasil_error(rc: i64) -> u32 {
    bump(Counter::ProdHasil);
    hasil_error(rc)
}

/// Nilai hasil tersandi.
#[no_mangle]
pub extern "C" fn klatos_prof_hasil_nilai(rc: i64) -> i64 {
    hasil_nilai(rc)
}

/// Pemilihan jalur (`JALUR_*`).
#[no_mangle]
pub extern "C" fn klatos_prof_jalur_pilih(has_original: u32, has_pseudo: u32) -> u32 {
    bump(Counter::ProdJalur);
    jalur_pilih(has_original != 0, has_pseudo != 0)
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_prof_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
#[no_mangle]
pub extern "C" fn klatos_prof_reset_counter() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas profiling (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosProfReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_gerbang: u32,
    pub prod_hasil: u32,
    pub prod_jalur: u32,
    pub prod_catat: u32,
    pub cadangan0: u32,
    pub cadangan1: u32,
    pub cadangan2: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosProfReport>()];

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

fn run_matrix() -> KlatosProfReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_5052_464C);
    m.catat(JALUR_ORIGINAL == 0 && JALUR_PSEUDO == 1 && JALUR_INVALID == 2);

    let _ = klatos_prof_reset_counter();

    // A) Gerbang admission.
    m.catat(gerbang(true, false, false));
    m.neg(!gerbang(false, false, false)); // profiling mati di pintu masuk
    m.neg(!gerbang(true, true, false));   // ditekan di akhir
    m.neg(!gerbang(true, false, true));   // penyangga null
    m.catat(klatos_prof_gerbang(1, 0, 0) == 1);
    m.catat(klatos_prof_gerbang(0, 0, 0) == 0);

    // B) Penyandian hasil.
    m.catat(hasil_error(-13) == 1 && hasil_nilai(-13) == 13);
    m.catat(hasil_error(0) == 0 && hasil_nilai(0) == 0);
    m.catat(hasil_error(42) == 0 && hasil_nilai(42) == 0);
    m.catat(klatos_prof_hasil_error(-2) == 1);
    m.catat(klatos_prof_hasil_nilai(-2) == 2);

    // C) Pemilihan jalur.
    m.catat(jalur_pilih(true, true) == JALUR_ORIGINAL);
    m.catat(jalur_pilih(false, true) == JALUR_PSEUDO);
    m.catat(jalur_pilih(false, false) == JALUR_INVALID);
    m.catat(klatos_prof_jalur_pilih(0, 0) == JALUR_INVALID);

    // D) Diferensial gerbang vs orakel (LCG deterministik).
    let mut seed: u64 = 0x2026_1005_0000_3002;
    let mut step = 0u32;
    while step < 20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let pe = (seed >> 3) & 1 == 1;
        let sup = (seed >> 4) & 1 == 1;
        let null_ = (seed >> 5) & 1 == 1;
        let r = gerbang(pe, sup, null_);
        let o = pe && !sup && !null_;
        m.dif(r == o);
        step += 1;
    }

    // E) Determinisme.
    {
        let first = jalur_pilih(false, true);
        let mut ok = true;
        for _ in 0..1000 {
            if jalur_pilih(false, true) != first {
                ok = false;
                break;
            }
        }
        m.neg(ok);
    }

    let prod_gerbang = COUNTERS[Counter::ProdGerbang as usize].load(Ordering::Relaxed);
    let prod_hasil = COUNTERS[Counter::ProdHasil as usize].load(Ordering::Relaxed);
    let prod_jalur = COUNTERS[Counter::ProdJalur as usize].load(Ordering::Relaxed);
    let prod_catat = COUNTERS[Counter::ProdCatat as usize].load(Ordering::Relaxed);
    let _ = klatos_prof_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && m.dif_checks > 0;
    KlatosProfReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_gerbang,
        prod_hasil,
        prod_jalur,
        prod_catat,
        cadangan0: 0,
        cadangan1: 0,
        cadangan2: 0,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas profiling.
#[no_mangle]
pub extern "C" fn klatos_prof_selftest() -> KlatosProfReport {
    assert!(core::mem::size_of::<KlatosProfReport>() == 64);
    assert!(core::mem::offset_of!(KlatosProfReport, status) == 60);
    run_matrix()
}
