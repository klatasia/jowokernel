// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.61 — **OTORITAS PENJAGA PAGE FAULT KERNEL**.


use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATPPFG".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5050_4647;

/// Fault boleh dilanjutkan ke pencarian Region.
pub const PPF_OK: i32 = 0;
/// Fault di dalam seksi `READONLY_AFTER_INIT`.
pub const PPF_RO: i32 = 1;
/// Fault di dalam seksi `UNMAP_AFTER_INIT`.
pub const PPF_UNMAP: i32 = 2;
/// Fault di dalam seksi `KSYMS`.
pub const PPF_KSYMS: i32 = 3;
/// Fault terjadi di dalam handler IRQ.
pub const PPF_IRQ: i32 = 4;
/// Argumen tidak sah.
pub const PPF_ARG: i32 = 5;

/// Penghitung audit produksi.
#[repr(u32)]
pub enum Counter {
    /// Fault di seksi RO-after-init.
    Ro = 0,
    /// Fault di seksi unmap-after-init.
    Unmap = 1,
    /// Fault di seksi ksyms.
    Ksyms = 2,
    /// Fault di dalam IRQ.
    Irq = 3,
    /// Fault bersih (lanjut).
    Ok = 4,
    /// Argumen tidak sah.
    ArgSalah = 5,
}

/// Jumlah penghitung.
pub const N_COUNTER: usize = 6;

static COUNTER: [AtomicU32; N_COUNTER] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

#[inline]
fn bump(c: Counter) {
    COUNTER[c as usize].fetch_add(1, Ordering::Relaxed);
}

/// Laporan selftest (72 byte).
#[repr(C)]
pub struct KlatosPpfReport {
    /// `REPORT_TAG`.
    pub tag: u64,
    /// Jumlah pemeriksaan struktural.
    pub checks: u32,
    /// Pemeriksaan struktural yang lulus.
    pub passed: u32,
    /// Jumlah iterasi diferensial.
    pub peta_checks: u32,
    /// Iterasi diferensial yang cocok dengan oracle.
    pub peta_passed: u32,
    /// Jumlah kasus negatif.
    pub neg_checks: u32,
    /// Kasus negatif yang lulus.
    pub neg_passed: u32,
    /// Produksi: fault seksi RO.
    pub prod_ro: u32,
    /// Produksi: fault seksi unmap.
    pub prod_unmap: u32,
    /// Produksi: fault seksi ksyms.
    pub prod_ksyms: u32,
    /// Produksi: fault di IRQ.
    pub prod_irq: u32,
    /// Produksi: fault bersih.
    pub prod_ok: u32,
    /// Produksi: argumen salah.
    pub prod_arg_salah: u32,
    /// Diferensial: jumlah kasus.
    pub diferensial_ukur: u32,
    /// Diferensial: jumlah ketidakcocokan.
    pub diferensial_beda: u32,
    /// 0 = lulus.
    pub status: i32,
}

#[inline]
fn b(x: u32) -> bool {
    x != 0
}

#[inline]
fn sah(x: u32) -> bool {
    x <= 1
}

/// `faulted_in_range`: `lo <= v < hi` (batas atas eksklusif).
#[inline]
pub fn dalam(v: u64, lo: u64, hi: u64) -> bool {
    v >= lo && v < hi
}

/// Gerbang penjaga page fault kernel.
///
/// Mengembalikan [`PPF_OK`] bila fault boleh dilanjutkan, atau kode alasan
/// (`PPF_RO`..`PPF_IRQ`) yang di C++ berarti `PageFaultResponse::ShouldCrash`.
#[inline]
pub fn penjaga(
    vaddr: u64,
    ro_lo: u64,
    ro_hi: u64,
    unmap_lo: u64,
    unmap_hi: u64,
    ksyms_lo: u64,
    ksyms_hi: u64,
    in_irq: u32,
) -> i32 {
    if !sah(in_irq) {
        return PPF_ARG;
    }
    if dalam(vaddr, ro_lo, ro_hi) {
        return PPF_RO;
    }
    if dalam(vaddr, unmap_lo, unmap_hi) {
        return PPF_UNMAP;
    }
    if dalam(vaddr, ksyms_lo, ksyms_hi) {
        return PPF_KSYMS;
    }
    if b(in_irq) {
        return PPF_IRQ;
    }
    PPF_OK
}

/// C-ABI [`penjaga`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_ppf_guard(
    vaddr: u64,
    ro_lo: u64,
    ro_hi: u64,
    unmap_lo: u64,
    unmap_hi: u64,
    ksyms_lo: u64,
    ksyms_hi: u64,
    in_irq: u32,
) -> i32 {
    let r = penjaga(vaddr, ro_lo, ro_hi, unmap_lo, unmap_hi, ksyms_lo, ksyms_hi, in_irq);
    match r {
        PPF_RO => bump(Counter::Ro),
        PPF_UNMAP => bump(Counter::Unmap),
        PPF_KSYMS => bump(Counter::Ksyms),
        PPF_IRQ => bump(Counter::Irq),
        PPF_OK => bump(Counter::Ok),
        _ => bump(Counter::ArgSalah),
    }
    r
}

/// Baca penghitung audit produksi.
#[no_mangle]
pub extern "C" fn klatos_ppf_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTER[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Nolkan seluruh penghitung; mengembalikan jumlah yang dinolkan.
#[no_mangle]
pub extern "C" fn klatos_ppf_reset() -> u32 {
    let mut n = 0u32;
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTER[i].store(0, Ordering::Relaxed);
        n += 1;
        i += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// Selftest / matriks
// ---------------------------------------------------------------------------

struct Matriks {
    checks: u32,
    passed: u32,
    peta_checks: u32,
    peta_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
    ukur: u32,
    beda: u32,
}

impl Matriks {
    const fn new() -> Self {
        Self { checks: 0, passed: 0, peta_checks: 0, peta_passed: 0, neg_checks: 0, neg_passed: 0, ukur: 0, beda: 0 }
    }
    #[inline]
    fn cek(&mut self, ok: bool) {
        self.checks += 1;
        if ok {
            self.passed += 1;
        }
    }
    #[inline]
    fn cek_neg(&mut self, ok: bool) {
        self.neg_checks += 1;
        if ok {
            self.neg_passed += 1;
        }
    }
    #[inline]
    fn diferensial(&mut self, ok: bool) {
        self.ukur += 1;
        self.peta_checks += 1;
        if ok {
            self.peta_passed += 1;
        } else {
            self.beda += 1;
        }
    }
}

/// Oracle independen: penjaga (implementasi kedua).
fn o_penjaga(
    v: u64,
    rl: u64,
    rh: u64,
    ul: u64,
    uh: u64,
    kl: u64,
    kh: u64,
    irq: u32,
) -> i32 {
    if irq > 1 {
        return PPF_ARG;
    }
    let di = |lo: u64, hi: u64| lo <= v && v < hi;
    if di(rl, rh) {
        return PPF_RO;
    }
    if di(ul, uh) {
        return PPF_UNMAP;
    }
    if di(kl, kh) {
        return PPF_KSYMS;
    }
    if irq == 1 {
        return PPF_IRQ;
    }
    PPF_OK
}

const PETA: [([u64; 8], i32); 14] = [
    // vaddr, ro_lo, ro_hi, unmap_lo, unmap_hi, ksyms_lo, ksyms_hi, in_irq, harap
    ([0x1000, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0], PPF_RO),
    ([0x1FFF, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0], PPF_RO),
    ([0x2000, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0], PPF_OK),
    ([0x3000, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0], PPF_UNMAP),
    ([0x4000, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0], PPF_OK),
    ([0x5000, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0], PPF_KSYMS),
    ([0x6000, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0], PPF_OK),
    ([0x7000, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0], PPF_OK),
    ([0x7000, 0x1000, 0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 1], PPF_IRQ),
    // seksi kosong tidak pernah cocok
    ([0x1000, 0x1000, 0x1000, 0, 0, 0, 0, 0], PPF_OK),
    // IRQ diperiksa TERAKHIR: fault di seksi RO saat IRQ tetap RO
    ([0x1500, 0x1000, 0x2000, 0, 0, 0, 0, 1], PPF_RO),
    // urutan: RO menang atas UNMAP bila tumpang
    ([0x1500, 0x1000, 0x3000, 0x1400, 0x1600, 0, 0, 0], PPF_RO),
    // UNMAP menang atas KSYMS
    ([0x1500, 0, 0, 0x1000, 0x3000, 0x1400, 0x1600, 0], PPF_UNMAP),
    // in_irq=2 -> ARG
    ([0x1500, 0, 0, 0, 0, 0, 0, 2], PPF_ARG),
];

/// Jalankan matriks lengkap; mengembalikan laporan.
pub fn jalankan_matriks() -> KlatosPpfReport {
    let mut m = Matriks::new();

    // G1 — konstanta.
    m.cek(PPF_OK != PPF_RO && PPF_RO != PPF_UNMAP && PPF_UNMAP != PPF_KSYMS);
    m.cek(PPF_KSYMS != PPF_IRQ && PPF_IRQ != PPF_ARG);
    m.cek(N_COUNTER == 6);

    // G2 — batas: bawah inklusif, atas eksklusif.
    m.cek(penjaga(0x1000, 0x1000, 0x2000, 0, 0, 0, 0, 0) == PPF_RO);
    m.cek(penjaga(0x1FFF, 0x1000, 0x2000, 0, 0, 0, 0, 0) == PPF_RO);
    m.cek(penjaga(0x2000, 0x1000, 0x2000, 0, 0, 0, 0, 0) == PPF_OK);
    m.cek(penjaga(0xFFF, 0x1000, 0x2000, 0, 0, 0, 0, 0) == PPF_OK);

    // G3 — seksi kosong.
    m.cek(penjaga(0x1000, 0x1000, 0x1000, 0, 0, 0, 0, 0) == PPF_OK);

    // G4 — urutan.
    m.cek(penjaga(0x1500, 0x1000, 0x3000, 0x1400, 0x1600, 0, 0, 0) == PPF_RO);
    m.cek(penjaga(0x1500, 0, 0, 0x1000, 0x3000, 0x1400, 0x1600, 0) == PPF_UNMAP);
    m.cek(penjaga(0x1500, 0x1000, 0x2000, 0, 0, 0, 0, 1) == PPF_RO);

    // G5 — IRQ hanya bila bukan seksi.
    m.cek(penjaga(0x7000, 0x1000, 0x2000, 0, 0, 0, 0, 1) == PPF_IRQ);
    m.cek(penjaga(0x7000, 0x1000, 0x2000, 0, 0, 0, 0, 0) == PPF_OK);

    // G6 — peta tetap.
    for (arg, exp) in PETA.iter() {
        let got = penjaga(arg[0], arg[1], arg[2], arg[3], arg[4], arg[5], arg[6], arg[7] as u32);
        m.cek(got == *exp);
    }

    // G7 — diferensial acak vs oracle (LCG).
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let n: u32 = 60_000;
    let mut i = 0u32;
    while i < n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = state >> 33;
        // Seksi kecil berjajar agar tumpang/urutan benar-benar teruji.
        let ro_lo = (r & 0xFFFF_0000) + 0x1000;
        let ro_hi = ro_lo + ((r >> 16) & 0x3FFF) + 1;
        let unmap_lo = ro_hi + ((r >> 32) & 0xFFF);
        let unmap_hi = unmap_lo + ((r >> 40) & 0x1FFF) + 1;
        let ksyms_lo = unmap_hi + ((r >> 48) & 0xFFF);
        let ksyms_hi = ksyms_lo + ((r >> 56) & 0x1FF) + 1;
        let v = (r & 0xFFFF_FFFF) + 0x1000;
        let irq = (r & 1) as u32;
        m.diferensial(
            penjaga(v, ro_lo, ro_hi, unmap_lo, unmap_hi, ksyms_lo, ksyms_hi, irq)
                == o_penjaga(v, ro_lo, ro_hi, unmap_lo, unmap_hi, ksyms_lo, ksyms_hi, irq),
        );
        i += 1;
    }

    // G8 — determinisme.
    let mut state2: u64 = 0x1234_5678_9ABC_DEF0;
    let mut sama = true;
    let mut j = 0u32;
    while j < 4096 {
        state2 = state2.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = state2 >> 33;
        let a = penjaga(r & 0xFFFF, 0x1000, 0x8000, 0x8000, 0x9000, 0x9000, 0xA000, (r & 1) as u32);
        let b2 = penjaga(r & 0xFFFF, 0x1000, 0x8000, 0x8000, 0x9000, 0x9000, 0xA000, (r & 1) as u32);
        if a != b2 {
            sama = false;
        }
        j += 1;
    }
    m.cek(sama);

    // G9 — negatif.
    m.cek_neg(penjaga(0x1000, 0, 0, 0, 0, 0, 0, 2) == PPF_ARG);
    m.cek_neg(penjaga(0x1000, 0, 0, 0, 0, 0, 0, 0xFFFF_FFFF) == PPF_ARG);
    // batas atas eksklusif tetap berlaku di seksi tengah
    m.cek_neg(penjaga(0x4000, 0x3000, 0x4000, 0, 0, 0, 0, 0) == PPF_OK);
    // seksi kosong di posisi tengah
    m.cek_neg(penjaga(0x8000, 0, 0, 0x8000, 0x8000, 0, 0, 0) == PPF_OK);

    let prod_ro = COUNTER[Counter::Ro as usize].load(Ordering::Relaxed);
    let prod_unmap = COUNTER[Counter::Unmap as usize].load(Ordering::Relaxed);
    let prod_ksyms = COUNTER[Counter::Ksyms as usize].load(Ordering::Relaxed);
    let prod_irq = COUNTER[Counter::Irq as usize].load(Ordering::Relaxed);
    let prod_ok = COUNTER[Counter::Ok as usize].load(Ordering::Relaxed);
    let prod_arg_salah = COUNTER[Counter::ArgSalah as usize].load(Ordering::Relaxed);

    let status = if m.passed == m.checks && m.peta_passed == m.peta_checks && m.neg_passed == m.neg_checks && m.beda == 0 {
        0
    } else {
        1
    };

    KlatosPpfReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_ro,
        prod_unmap,
        prod_ksyms,
        prod_irq,
        prod_ok,
        prod_arg_salah,
        diferensial_ukur: m.ukur,
        diferensial_beda: m.beda,
        status,
    }
}

/// C-ABI selftest.
#[no_mangle]
pub extern "C" fn klatos_ppf_selftest() -> KlatosPpfReport {
    jalankan_matriks()
}

/// Pemeriksaan ukuran tipe pada waktu kompilasi.
pub mod conformance {
    use super::*;
    const _: () = {
        assert!(core::mem::size_of::<KlatosPpfReport>() == 72);
    };
}
