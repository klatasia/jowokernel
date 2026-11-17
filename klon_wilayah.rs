// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! KR-29 — **OTORITAS RENCANA KLON WILAYAH (Region::try_clone)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATKLNW".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4B4C_4E57;

/// Bit rencana: jenis klon = CoW (bila bit0 = 1) atau shared (bit0 = 0).
pub const RENCANA_COW: u32 = 1 << 0;
/// Bit rencana: wilayah INDUK harus di-`remap()`.
pub const RENCANA_REMAP_INDUK: u32 = 1 << 1;
/// Bit rencana: wilayah klon di-`set_stack(true)`.
pub const RENCANA_SET_STACK: u32 = 1 << 2;
/// Sentinel: kombinasi invarian TAK SAH (cermin `VERIFY` yang gagal).
pub const RENCANA_TAK_SAH: u32 = 0xFFFF_FFFF;

// --- penghitung audit -------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    ProdRencana = 0,
    ProdShared = 1,
    ProdCow = 2,
    TolakInvarian = 3,
}
const N_COUNTER: usize = 4;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- kebijakan murni --------------------------------------------------------

/// Hitung rencana klon. Lihat dokumentasi modul.
#[inline]
pub fn rencana(
    is_shared: bool,
    is_writable: bool,
    is_stack: bool,
    vm_is_inode: bool,
    vm_is_shared_inode: bool,
    vm_is_private_inode: bool,
    vm_is_anonymous: bool,
) -> u32 {
    if is_shared {
        // VERIFY(!is_stack()); if (is_inode()) VERIFY(is_shared_inode());
        if is_stack {
            return RENCANA_TAK_SAH;
        }
        if vm_is_inode && !vm_is_shared_inode {
            return RENCANA_TAK_SAH;
        }
        // Klon memakai VMObject yang sama; induk TIDAK di-remap.
        return 0;
    }
    // if (is_inode()) VERIFY(is_private_inode());
    if vm_is_inode && !vm_is_private_inode {
        return RENCANA_TAK_SAH;
    }
    // if (is_stack()) VERIFY(is_anonymous());
    if is_stack && !vm_is_anonymous {
        return RENCANA_TAK_SAH;
    }
    let mut p = RENCANA_COW;
    if is_writable {
        p |= RENCANA_REMAP_INDUK;
    }
    if is_stack {
        p |= RENCANA_SET_STACK;
    }
    p
}

/// Orakel historis: cermin literal `Region::try_clone`. Dipakai HANYA oleh
/// matriks diferensial (struktur berbeda supaya kesepakatan bermakna).
#[inline]
pub fn kebijakan_lama(
    is_shared: bool,
    is_writable: bool,
    is_stack: bool,
    vm_is_inode: bool,
    vm_is_shared_inode: bool,
    vm_is_private_inode: bool,
    vm_is_anonymous: bool,
) -> u32 {
    if is_shared {
        if is_stack {
            return RENCANA_TAK_SAH;
        }
        if vm_is_inode {
            if !vm_is_shared_inode {
                return RENCANA_TAK_SAH;
            }
        }
        return 0;
    }
    if vm_is_inode {
        if !vm_is_private_inode {
            return RENCANA_TAK_SAH;
        }
    }
    if is_stack {
        if !vm_is_anonymous {
            return RENCANA_TAK_SAH;
        }
    }
    let mut p = 1u32; // COW
    if is_writable {
        p = p | 2;
    }
    if is_stack {
        p = p | 4;
    }
    p
}

// --- pintu C ABI ------------------------------------------------------------

/// Hitung rencana klon wilayah. Mengembalikan bit [`RENCANA_*`] atau
/// [`RENCANA_TAK_SAH`]. Tanpa pointer.
#[no_mangle]
pub extern "C" fn klatos_wilayah_klon_rencana(
    is_shared: u32,
    is_writable: u32,
    is_stack: u32,
    vm_is_inode: u32,
    vm_is_shared_inode: u32,
    vm_is_private_inode: u32,
    vm_is_anonymous: u32,
) -> u32 {
    bump(Counter::ProdRencana);
    let r = rencana(
        is_shared != 0,
        is_writable != 0,
        is_stack != 0,
        vm_is_inode != 0,
        vm_is_shared_inode != 0,
        vm_is_private_inode != 0,
        vm_is_anonymous != 0,
    );
    if r == RENCANA_TAK_SAH {
        bump(Counter::TolakInvarian);
    } else if (r & RENCANA_COW) != 0 {
        bump(Counter::ProdCow);
    } else {
        bump(Counter::ProdShared);
    }
    r
}

/// Nilai konstanta rencana (untuk C++).
#[no_mangle]
pub extern "C" fn klatos_wilayah_klon_cow() -> u32 {
    RENCANA_COW
}
#[no_mangle]
pub extern "C" fn klatos_wilayah_klon_remap_induk() -> u32 {
    RENCANA_REMAP_INDUK
}
#[no_mangle]
pub extern "C" fn klatos_wilayah_klon_set_stack() -> u32 {
    RENCANA_SET_STACK
}
#[no_mangle]
pub extern "C" fn klatos_wilayah_klon_tak_sah() -> u32 {
    RENCANA_TAK_SAH
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_wilayah_klon_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
#[no_mangle]
pub extern "C" fn klatos_wilayah_klon_reset_counter() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosKlonReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_rencana: u32,
    pub prod_shared: u32,
    pub prod_cow: u32,
    pub tolak_invarian: u32,
    pub cadangan0: u32,
    pub cadangan1: u32,
    pub cadangan2: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosKlonReport>()];

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

fn run_matrix() -> KlatosKlonReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_4B4C_4E57);
    m.catat(RENCANA_COW == 1 && RENCANA_REMAP_INDUK == 2 && RENCANA_SET_STACK == 4);
    m.catat(RENCANA_TAK_SAH == 0xFFFF_FFFF);

    // A) Shared anonim.
    let r = rencana(true, false, false, false, false, false, false);
    m.catat(r == 0);
    // B) Shared inode bersama.
    m.catat(rencana(true, false, false, true, true, false, false) == 0);
    // C) Shared + stack -> tak sah.
    m.neg(rencana(true, false, true, false, false, false, false) == RENCANA_TAK_SAH);
    // D) Shared inode privat -> tak sah.
    m.neg(rencana(true, false, false, true, false, true, false) == RENCANA_TAK_SAH);
    // E) CoW anonim writable.
    let r = rencana(false, true, false, false, false, false, true);
    m.catat(r == (RENCANA_COW | RENCANA_REMAP_INDUK));
    // F) CoW anonim read-only (induk tak di-remap).
    m.catat(rencana(false, false, false, false, false, false, true) == RENCANA_COW);
    // G) CoW stack anonim writable -> set_stack + remap.
    let r = rencana(false, true, true, false, false, false, true);
    m.catat(r == (RENCANA_COW | RENCANA_REMAP_INDUK | RENCANA_SET_STACK));
    // H) CoW stack non-anonim -> tak sah.
    m.neg(rencana(false, true, true, false, false, false, false) == RENCANA_TAK_SAH);
    // I) CoW inode privat.
    m.catat(rencana(false, false, false, true, false, true, false) == RENCANA_COW);
    // J) CoW inode shared (bukan privat) -> tak sah.
    m.neg(rencana(false, false, false, true, true, false, false) == RENCANA_TAK_SAH);

    // K) Diferensial: SELURUH 2^7 = 128 kombinasi input.
    let mut diukur = 0u32;
    let mut beda = 0u32;
    for bits in 0u32..128 {
        let a = (bits & 1) != 0;
        let b = (bits & 2) != 0;
        let c = (bits & 4) != 0;
        let d = (bits & 8) != 0;
        let e = (bits & 16) != 0;
        let f = (bits & 32) != 0;
        let g = (bits & 64) != 0;
        let rr = rencana(a, b, c, d, e, f, g);
        let ro = kebijakan_lama(a, b, c, d, e, f, g);
        diukur += 1;
        if rr != ro {
            beda += 1;
        }
    }
    m.catat(diukur == 128);
    m.catat(beda == 0);
    m.dif(beda == 0);

    // L) Determinisme.
    {
        let mut ok = true;
        let first = rencana(false, true, false, true, false, true, false);
        for _ in 0..1000 {
            if rencana(false, true, false, true, false, true, false) != first {
                ok = false;
                break;
            }
        }
        m.neg(ok);
    }

    let prod_rencana = COUNTERS[Counter::ProdRencana as usize].load(Ordering::Relaxed);
    let prod_shared = COUNTERS[Counter::ProdShared as usize].load(Ordering::Relaxed);
    let prod_cow = COUNTERS[Counter::ProdCow as usize].load(Ordering::Relaxed);
    let tolak_invarian = COUNTERS[Counter::TolakInvarian as usize].load(Ordering::Relaxed);
    let _ = klatos_wilayah_klon_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && m.dif_checks > 0;
    KlatosKlonReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_rencana,
        prod_shared,
        prod_cow,
        tolak_invarian,
        cadangan0: 0,
        cadangan1: 0,
        cadangan2: 0,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas rencana klon wilayah.
#[no_mangle]
pub extern "C" fn klatos_wilayah_klon_selftest() -> KlatosKlonReport {
    assert!(core::mem::size_of::<KlatosKlonReport>() == 64);
    assert!(core::mem::offset_of!(KlatosKlonReport, status) == 60);
    run_matrix()
}
