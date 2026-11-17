// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


// KR-30 — **OTORITAS KEBIJAKAN SISTEM (uname / realpath / utime / nofollow)**.

#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATSYSK".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5359_534B;

/// `UTSNAME_ENTRY_LEN` — cermin `Jowo/Kontrak/POSIX/sys/utsname.h:18`.
pub const UTSNAME_ENTRY_LEN: u32 = 65;
/// `O_NOFOLLOW_NOERROR` — cermin `KlatKernel/Efek/Berkas/VirtualFileSystem.h:35`.
pub const O_NOFOLLOW_NOERROR: i32 = 1 << 29;
/// Nama sistem tetap.
pub const SYSNAME: &[u8] = b"jowokernel";

// --- penghitung audit -------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    ProdSysname = 0,
    ProdSalin = 1,
    ProdHostname = 2,
    ProdNofollow = 3,
    ProdRealpath = 4,
    ProdUtime = 5,
    TolakParam = 6,
}
const N_COUNTER: usize = 7;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- kebijakan murni --------------------------------------------------------

/// Klamp panjang salinan: `min(len, kapasitas-1)`; 0 bila kapasitas 0.
#[inline]
pub fn salin_panjang(len: u32, kapasitas: u32) -> u32 {
    if kapasitas == 0 {
        0
    } else {
        let maks = kapasitas - 1;
        if len < maks { len } else { maks }
    }
}

/// Admisi panjang hostname: `len <= UTSNAME_ENTRY_LEN - 1`.
#[inline]
pub fn hostname_sah(len: u32) -> bool {
    len <= UTSNAME_ENTRY_LEN - 1
}

/// Pemetaan bendera `follow_symlinks`.
#[inline]
pub fn nofollow(follow: bool) -> i32 {
    if follow { 0 } else { O_NOFOLLOW_NOERROR }
}

/// Ukuran ideal realpath = `len + 1`.
#[inline]
pub fn realpath_ideal(len: u32) -> u64 {
    len as u64 + 1
}

/// Ukuran yang benar-benar disalin = `min(ideal, kapasitas)`.
#[inline]
pub fn realpath_salin(ideal: u64, kapasitas: u64) -> u64 {
    if ideal < kapasitas { ideal } else { kapasitas }
}

/// Keputusan stempel-waktu bawaan `sys$utime`.
#[inline]
pub fn utime_default(buf_null: bool) -> bool {
    buf_null
}

// --- pintu C ABI ------------------------------------------------------------

/// Salin `SYSNAME` ke `buf` (maks `cap` byte). Mengembalikan jumlah byte.
///
/// # Safety
/// `buf` harus dapat ditulis sebanyak `min(len(SYSNAME), cap)` byte.
#[no_mangle]
pub unsafe extern "C" fn klatos_sys_sysname(buf: *mut u8, cap: u32) -> u32 {
    if buf.is_null() {
        bump(Counter::TolakParam);
        return 0;
    }
    let n = salin_panjang(SYSNAME.len() as u32, cap + 1);
    let mut i = 0u32;
    while i < n {
        unsafe { core::ptr::write(buf.add(i as usize), SYSNAME[i as usize]) };
        i += 1;
    }
    bump(Counter::ProdSysname);
    n
}

/// Klamp panjang salinan.
#[no_mangle]
pub extern "C" fn klatos_sys_salin_panjang(len: u32, kapasitas: u32) -> u32 {
    bump(Counter::ProdSalin);
    salin_panjang(len, kapasitas)
}

/// Admisi panjang hostname (1 sah / 0 tidak).
#[no_mangle]
pub extern "C" fn klatos_sys_hostname_sah(len: u32) -> u32 {
    bump(Counter::ProdHostname);
    if hostname_sah(len) { 1 } else { 0 }
}

/// Pemetaan bendera `follow_symlinks` -> `0` atau `O_NOFOLLOW_NOERROR`.
#[no_mangle]
pub extern "C" fn klatos_sys_nofollow(follow: u32) -> i32 {
    bump(Counter::ProdNofollow);
    nofollow(follow != 0)
}

/// Ukuran ideal realpath.
#[no_mangle]
pub extern "C" fn klatos_sys_realpath_ideal(len: u32) -> u64 {
    bump(Counter::ProdRealpath);
    realpath_ideal(len)
}

/// Ukuran yang disalin realpath.
#[no_mangle]
pub extern "C" fn klatos_sys_realpath_salin(ideal: u64, kapasitas: u64) -> u64 {
    realpath_salin(ideal, kapasitas)
}

/// Keputusan stempel-waktu bawaan (1 = pakai `{now,now}`).
#[no_mangle]
pub extern "C" fn klatos_sys_utime_default(buf_null: u32) -> u32 {
    bump(Counter::ProdUtime);
    if utime_default(buf_null != 0) { 1 } else { 0 }
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_sys_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
#[no_mangle]
pub extern "C" fn klatos_sys_reset_counter() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas kebijakan sistem (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosSysReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_sysname: u32,
    pub prod_salin: u32,
    pub prod_hostname: u32,
    pub prod_nofollow: u32,
    pub prod_realpath: u32,
    pub prod_utime: u32,
    pub cadangan0: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosSysReport>()];

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

fn run_matrix() -> KlatosSysReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_5359_534B);
    m.catat(UTSNAME_ENTRY_LEN == 65);
    m.catat(O_NOFOLLOW_NOERROR == (1 << 29));
    m.catat(SYSNAME == b"jowokernel");

    let _ = klatos_sys_reset_counter();

    // A) sysname.
    let mut buf = [0u8; 16];
    let n = unsafe { klatos_sys_sysname(buf.as_mut_ptr(), 16) };
    m.catat(n == 10);
    m.catat(&buf[..10] == b"jowokernel");
    m.neg(unsafe { klatos_sys_sysname(core::ptr::null_mut(), 8) } == 0);

    // B) klamp salin.
    m.catat(salin_panjang(3, 65) == 3);
    m.catat(salin_panjang(64, 65) == 64);
    m.catat(salin_panjang(100, 65) == 64);
    m.catat(salin_panjang(5, 0) == 0);

    // C) hostname.
    m.catat(hostname_sah(0));
    m.catat(hostname_sah(64));
    m.neg(!hostname_sah(65));

    // D) nofollow.
    m.catat(nofollow(true) == 0);
    m.catat(nofollow(false) == O_NOFOLLOW_NOERROR);

    // E) realpath.
    m.catat(realpath_ideal(9) == 10);
    m.catat(realpath_salin(10, 100) == 10);
    m.catat(realpath_salin(10, 4) == 4);
    m.catat(realpath_salin(10, 10) == 10);

    // F) utime default.
    m.catat(utime_default(true));
    m.neg(!utime_default(false));

    // G) Diferensial klamp vs orakel (LCG deterministik).
    let mut seed: u64 = 0x2026_1005_0000_3000;
    let mut step = 0u32;
    while step < 20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let len = (seed >> 7) as u32 & 0x1FF;
        let cap = (seed >> 23) as u32 & 0x1FF;
        let r = salin_panjang(len, cap);
        let o = if cap == 0 { 0 } else { core::cmp::min(len, cap - 1) };
        m.dif(r == o);
        step += 1;
    }

    // H) Determinisme.
    {
        let first = salin_panjang(100, 65);
        let mut ok = true;
        for _ in 0..1000 {
            if salin_panjang(100, 65) != first {
                ok = false;
                break;
            }
        }
        m.neg(ok);
    }

    let prod_sysname = COUNTERS[Counter::ProdSysname as usize].load(Ordering::Relaxed);
    let prod_salin = COUNTERS[Counter::ProdSalin as usize].load(Ordering::Relaxed);
    let prod_hostname = COUNTERS[Counter::ProdHostname as usize].load(Ordering::Relaxed);
    let prod_nofollow = COUNTERS[Counter::ProdNofollow as usize].load(Ordering::Relaxed);
    let prod_realpath = COUNTERS[Counter::ProdRealpath as usize].load(Ordering::Relaxed);
    let prod_utime = COUNTERS[Counter::ProdUtime as usize].load(Ordering::Relaxed);
    let _ = klatos_sys_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && m.dif_checks > 0;
    KlatosSysReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_sysname,
        prod_salin,
        prod_hostname,
        prod_nofollow,
        prod_realpath,
        prod_utime,
        cadangan0: 0,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas kebijakan sistem.
#[no_mangle]
pub extern "C" fn klatos_sys_selftest() -> KlatosSysReport {
    assert!(core::mem::size_of::<KlatosSysReport>() == 64);
    assert!(core::mem::offset_of!(KlatosSysReport, status) == 60);
    run_matrix()
}
