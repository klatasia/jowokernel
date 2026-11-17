// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.62 — **OTORITAS AKUNTANSI MEMORI (BOOKKEEPING)**.


use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATAMEM".
pub const REPORT_TAG: u64 = 0x4B4C_4154_414D_454D;

// --- sumber `amount_dirty` -------------------------------------------------

/// Sumber = `amount_resident()` (region non-inode).
pub const AM_SUMBER_RESIDENT: u32 = 0;
/// Sumber = `InodeVMObject::amount_dirty()`.
pub const AM_SUMBER_INODE: u32 = 1;
/// Argumen tidak sah.
pub const AM_ARG: u32 = 0xFFFF_FFFF;

// --- purgeable -------------------------------------------------------------

/// Tidak dihitung.
pub const AM_PURGE_LEWAT: u32 = 0;
/// Purgeable volatil.
pub const AM_PURGE_VOLATIL: u32 = 1;
/// Purgeable non-volatil.
pub const AM_PURGE_NONVOLATIL: u32 = 2;

// --- pagemap ---------------------------------------------------------------

/// Halaman tidak ada.
pub const AM_PETA_NONE: u32 = 0;
/// Halaman shared-zero / lazy-committed.
pub const AM_PETA_ZERO: u32 = 1;
/// Halaman hadir (fisik).
pub const AM_PETA_PRESENT: u32 = 2;

/// Penghitung audit produksi.
#[repr(u32)]
pub enum Counter {
    /// Predikat resident dipanggil.
    Resident = 0,
    /// Predikat shared dipanggil.
    Shared = 1,
    /// Pemilihan sumber dirty.
    Dirty = 2,
    /// Inklusi inode.
    Inode = 3,
    /// Purgeable volatil.
    PurgeVol = 4,
    /// Purgeable non-volatil.
    PurgeNon = 5,
    /// Argumen tidak sah.
    ArgSalah = 6,
    /// Klasifikasi pagemap.
    Peta = 7,
}

/// Jumlah penghitung.
pub const N_COUNTER: usize = 8;

static COUNTER: [AtomicU32; N_COUNTER] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
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
pub struct KlatosAmReport {
    /// `REPORT_TAG`.
    pub tag: u64,
    /// Jumlah pemeriksaan struktural.
    pub checks: u32,
    /// Pemeriksaan struktural yang lulus.
    pub passed: u32,
    /// Jumlah iterasi diferensial.
    pub peta_checks: u32,
    /// Iterasi diferensial yang cocok.
    pub peta_passed: u32,
    /// Jumlah kasus negatif.
    pub neg_checks: u32,
    /// Kasus negatif yang lulus.
    pub neg_passed: u32,
    /// Produksi: predikat resident.
    pub prod_resident: u32,
    /// Produksi: predikat shared.
    pub prod_shared: u32,
    /// Produksi: pemilihan sumber dirty.
    pub prod_dirty: u32,
    /// Produksi: inklusi inode.
    pub prod_inode: u32,
    /// Produksi: purgeable volatil.
    pub prod_pv: u32,
    /// Produksi: purgeable non-volatil.
    pub prod_pnv: u32,
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

/// Predikat resident per halaman (`Region::amount_resident`).
#[inline]
pub fn halaman_resident(present: u32, is_shared_zero: u32, is_lazy: u32) -> u32 {
    if !sah(present) || !sah(is_shared_zero) || !sah(is_lazy) {
        return AM_ARG;
    }
    if b(present) && !b(is_shared_zero) && !b(is_lazy) {
        1
    } else {
        0
    }
}

/// Predikat shared per halaman (`Region::amount_shared`).
#[inline]
pub fn halaman_shared(present: u32, ref_gt1: u32, is_shared_zero: u32, is_lazy: u32) -> u32 {
    if !sah(present) || !sah(ref_gt1) || !sah(is_shared_zero) || !sah(is_lazy) {
        return AM_ARG;
    }
    if b(present) && b(ref_gt1) && !b(is_shared_zero) && !b(is_lazy) {
        1
    } else {
        0
    }
}

/// Pemilihan sumber `amount_dirty` (`Region::amount_dirty`).
#[inline]
pub fn sumber_dirty(is_inode: u32) -> u32 {
    if !sah(is_inode) {
        return AM_ARG;
    }
    if b(is_inode) {
        AM_SUMBER_INODE
    } else {
        AM_SUMBER_RESIDENT
    }
}

/// Inklusi `AddressSpace::amount_dirty_private` (lewati region shared).
#[inline]
pub fn sertakan_dirty_privat(is_shared: u32) -> u32 {
    if !sah(is_shared) {
        return AM_ARG;
    }
    if b(is_shared) {
        0
    } else {
        1
    }
}

/// Inklusi `AddressSpace::amount_clean_inode`.
#[inline]
pub fn sertakan_inode(is_inode: u32) -> u32 {
    if !sah(is_inode) {
        return AM_ARG;
    }
    if b(is_inode) {
        1
    } else {
        0
    }
}

/// Inklusi `AddressSpace::amount_purgeable_*`.
#[inline]
pub fn purgeable(is_anon: u32, is_purgeable: u32, is_volatile: u32) -> u32 {
    if !sah(is_anon) || !sah(is_purgeable) || !sah(is_volatile) {
        return AM_ARG;
    }
    if !b(is_anon) || !b(is_purgeable) {
        return AM_PURGE_LEWAT;
    }
    if b(is_volatile) {
        AM_PURGE_VOLATIL
    } else {
        AM_PURGE_NONVOLATIL
    }
}

/// Klasifikasi satu halaman untuk `pagemap` (`ProcessExposed.cpp`).
///
/// Mengembalikan [`AM_PETA_NONE`] / [`AM_PETA_ZERO`] / [`AM_PETA_PRESENT`]
/// (atau [`AM_ARG`]).
#[inline]
pub fn pagemap(present: u32, is_shared_zero: u32, is_lazy: u32) -> u32 {
    if !sah(present) || !sah(is_shared_zero) || !sah(is_lazy) {
        return AM_ARG;
    }
    if !b(present) {
        return AM_PETA_NONE;
    }
    if b(is_shared_zero) || b(is_lazy) {
        return AM_PETA_ZERO;
    }
    AM_PETA_PRESENT
}

// ---------------------------------------------------------------------------
// Permukaan C-ABI
// ---------------------------------------------------------------------------

/// C-ABI [`halaman_resident`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_am_halaman_resident(present: u32, is_shared_zero: u32, is_lazy: u32) -> u32 {
    let r = halaman_resident(present, is_shared_zero, is_lazy);
    if r == AM_ARG {
        bump(Counter::ArgSalah);
    } else {
        bump(Counter::Resident);
    }
    r
}

/// C-ABI [`halaman_shared`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_am_halaman_shared(present: u32, ref_gt1: u32, is_shared_zero: u32, is_lazy: u32) -> u32 {
    let r = halaman_shared(present, ref_gt1, is_shared_zero, is_lazy);
    if r == AM_ARG {
        bump(Counter::ArgSalah);
    } else {
        bump(Counter::Shared);
    }
    r
}

/// C-ABI [`sumber_dirty`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_am_sumber_dirty(is_inode: u32) -> u32 {
    let r = sumber_dirty(is_inode);
    if r == AM_ARG {
        bump(Counter::ArgSalah);
    } else {
        bump(Counter::Dirty);
    }
    r
}

/// C-ABI [`sertakan_dirty_privat`].
#[no_mangle]
pub extern "C" fn klatos_am_sertakan_dirty_privat(is_shared: u32) -> u32 {
    let r = sertakan_dirty_privat(is_shared);
    if r == AM_ARG {
        bump(Counter::ArgSalah);
    }
    r
}

/// C-ABI [`sertakan_inode`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_am_sertakan_inode(is_inode: u32) -> u32 {
    let r = sertakan_inode(is_inode);
    if r == AM_ARG {
        bump(Counter::ArgSalah);
    } else {
        bump(Counter::Inode);
    }
    r
}

/// C-ABI [`purgeable`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_am_purgeable(is_anon: u32, is_purgeable: u32, is_volatile: u32) -> u32 {
    let r = purgeable(is_anon, is_purgeable, is_volatile);
    if r == AM_ARG {
        bump(Counter::ArgSalah);
    } else if r == AM_PURGE_VOLATIL {
        bump(Counter::PurgeVol);
    } else if r == AM_PURGE_NONVOLATIL {
        bump(Counter::PurgeNon);
    }
    r
}

/// C-ABI [`pagemap`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_am_pagemap(present: u32, is_shared_zero: u32, is_lazy: u32) -> u32 {
    let r = pagemap(present, is_shared_zero, is_lazy);
    if r == AM_ARG {
        bump(Counter::ArgSalah);
    } else {
        bump(Counter::Peta);
    }
    r
}

/// Baca penghitung audit produksi.
#[no_mangle]
pub extern "C" fn klatos_am_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTER[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Nolkan seluruh penghitung; mengembalikan jumlah yang dinolkan.
#[no_mangle]
pub extern "C" fn klatos_am_reset() -> u32 {
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

/// Oracle independen: resident.
fn o_resident(p: u32, z: u32, l: u32) -> u32 {
    if p > 1 || z > 1 || l > 1 {
        return AM_ARG;
    }
    ((p == 1) as u32) & (((z == 0) as u32) & ((l == 0) as u32))
}

/// Oracle independen: shared.
fn o_shared(p: u32, r: u32, z: u32, l: u32) -> u32 {
    if p > 1 || r > 1 || z > 1 || l > 1 {
        return AM_ARG;
    }
    ((p == 1) as u32) & ((r == 1) as u32) & (((z == 0) as u32) & ((l == 0) as u32))
}

/// Oracle independen: purgeable.
fn o_purgeable(a: u32, p: u32, v: u32) -> u32 {
    if a > 1 || p > 1 || v > 1 {
        return AM_ARG;
    }
    if a != 1 || p != 1 {
        return AM_PURGE_LEWAT;
    }
    if v == 1 {
        AM_PURGE_VOLATIL
    } else {
        AM_PURGE_NONVOLATIL
    }
}

/// Oracle independen: pagemap.
fn o_pagemap(p: u32, z: u32, l: u32) -> u32 {
    if p > 1 || z > 1 || l > 1 {
        return AM_ARG;
    }
    if p != 1 {
        return AM_PETA_NONE;
    }
    if z == 1 || l == 1 {
        return AM_PETA_ZERO;
    }
    AM_PETA_PRESENT
}

/// Jalankan matriks lengkap; mengembalikan laporan.
pub fn jalankan_matriks() -> KlatosAmReport {
    let mut m = Matriks::new();

    // G1 — konstanta.
    m.cek(AM_SUMBER_RESIDENT != AM_SUMBER_INODE);
    m.cek(AM_PURGE_LEWAT != AM_PURGE_VOLATIL && AM_PURGE_VOLATIL != AM_PURGE_NONVOLATIL);
    m.cek(N_COUNTER == 8);
    m.cek(AM_PETA_NONE != AM_PETA_ZERO && AM_PETA_ZERO != AM_PETA_PRESENT);

    // G2 — resident: zero/lazy TIDAK dihitung.
    m.cek(halaman_resident(1, 0, 0) == 1);
    m.cek(halaman_resident(1, 1, 0) == 0);
    m.cek(halaman_resident(1, 0, 1) == 0);
    m.cek(halaman_resident(0, 0, 0) == 0);
    m.cek(halaman_resident(1, 1, 1) == 0);

    // G3 — shared: butuh ref>1 DAN bukan zero/lazy.
    m.cek(halaman_shared(1, 1, 0, 0) == 1);
    m.cek(halaman_shared(1, 0, 0, 0) == 0);
    m.cek(halaman_shared(1, 1, 1, 0) == 0);
    m.cek(halaman_shared(1, 1, 0, 1) == 0);
    m.cek(halaman_shared(0, 1, 0, 0) == 0);

    // G4 — sumber dirty.
    m.cek(sumber_dirty(0) == AM_SUMBER_RESIDENT);
    m.cek(sumber_dirty(1) == AM_SUMBER_INODE);

    // G5 — inklusi.
    m.cek(sertakan_dirty_privat(0) == 1);
    m.cek(sertakan_dirty_privat(1) == 0);
    m.cek(sertakan_inode(1) == 1);
    m.cek(sertakan_inode(0) == 0);

    // G6 — purgeable.
    m.cek(purgeable(1, 1, 1) == AM_PURGE_VOLATIL);
    m.cek(purgeable(1, 1, 0) == AM_PURGE_NONVOLATIL);
    m.cek(purgeable(1, 0, 1) == AM_PURGE_LEWAT);
    m.cek(purgeable(0, 1, 1) == AM_PURGE_LEWAT);
    m.cek(purgeable(0, 0, 0) == AM_PURGE_LEWAT);

    // G6b — pagemap.
    m.cek(pagemap(0, 0, 0) == AM_PETA_NONE);
    m.cek(pagemap(1, 0, 0) == AM_PETA_PRESENT);
    m.cek(pagemap(1, 1, 0) == AM_PETA_ZERO);
    m.cek(pagemap(1, 0, 1) == AM_PETA_ZERO);
    m.cek(pagemap(1, 1, 1) == AM_PETA_ZERO);

    // G7 — diferensial acak vs oracle (LCG).
    let mut state: u64 = 0xC0FF_EE00_1234_5678;
    let n: u32 = 80_000;
    let mut i = 0u32;
    while i < n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = state >> 33;
        let p = (r & 1) as u32;
        let z = ((r >> 1) & 1) as u32;
        let l = ((r >> 2) & 1) as u32;
        let g = ((r >> 3) & 1) as u32;
        let a = ((r >> 4) & 1) as u32;
        let pu = ((r >> 5) & 1) as u32;
        let v = ((r >> 6) & 1) as u32;
        let ino = ((r >> 7) & 1) as u32;
        let sh = ((r >> 8) & 1) as u32;

        m.diferensial(halaman_resident(p, z, l) == o_resident(p, z, l));
        m.diferensial(halaman_shared(p, g, z, l) == o_shared(p, g, z, l));
        m.diferensial(sumber_dirty(ino) == (if ino == 1 { AM_SUMBER_INODE } else { AM_SUMBER_RESIDENT }));
        m.diferensial(sertakan_dirty_privat(sh) == (if sh == 1 { 0 } else { 1 }));
        m.diferensial(sertakan_inode(ino) == ino);
        m.diferensial(purgeable(a, pu, v) == o_purgeable(a, pu, v));
        m.diferensial(pagemap(p, z, l) == o_pagemap(p, z, l));
        i += 1;
    }

    // G8 — determinisme.
    let mut state2: u64 = 0xDEAD_BEEF_CAFE_0001;
    let mut sama = true;
    let mut j = 0u32;
    while j < 4096 {
        state2 = state2.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = state2 >> 33;
        let x = halaman_resident((r & 1) as u32, ((r >> 1) & 1) as u32, ((r >> 2) & 1) as u32);
        let y = halaman_resident((r & 1) as u32, ((r >> 1) & 1) as u32, ((r >> 2) & 1) as u32);
        if x != y {
            sama = false;
        }
        j += 1;
    }
    m.cek(sama);

    // G9 — negatif.
    m.cek_neg(halaman_resident(2, 0, 0) == AM_ARG);
    m.cek_neg(halaman_shared(1, 2, 0, 0) == AM_ARG);
    m.cek_neg(sumber_dirty(2) == AM_ARG);
    m.cek_neg(sertakan_dirty_privat(2) == AM_ARG);
    m.cek_neg(sertakan_inode(2) == AM_ARG);
    m.cek_neg(purgeable(0, 0, 2) == AM_ARG);
    m.cek_neg(pagemap(2, 0, 0) == AM_ARG);

    let prod_resident = COUNTER[Counter::Resident as usize].load(Ordering::Relaxed);
    let prod_shared = COUNTER[Counter::Shared as usize].load(Ordering::Relaxed);
    let prod_dirty = COUNTER[Counter::Dirty as usize].load(Ordering::Relaxed);
    let prod_inode = COUNTER[Counter::Inode as usize].load(Ordering::Relaxed);
    let prod_pv = COUNTER[Counter::PurgeVol as usize].load(Ordering::Relaxed);
    let prod_pnv = COUNTER[Counter::PurgeNon as usize].load(Ordering::Relaxed);

    let status = if m.passed == m.checks && m.peta_passed == m.peta_checks && m.neg_passed == m.neg_checks && m.beda == 0 {
        0
    } else {
        1
    };

    KlatosAmReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_resident,
        prod_shared,
        prod_dirty,
        prod_inode,
        prod_pv,
        prod_pnv,
        diferensial_ukur: m.ukur,
        diferensial_beda: m.beda,
        status,
    }
}

/// C-ABI selftest.
#[no_mangle]
pub extern "C" fn klatos_am_selftest() -> KlatosAmReport {
    jalankan_matriks()
}

/// Pemeriksaan ukuran tipe pada waktu kompilasi.
pub mod conformance {
    use super::*;
    const _: () = {
        assert!(core::mem::size_of::<KlatosAmReport>() == 72);
    };
}
