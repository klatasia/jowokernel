// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT
// KR-30 — **OTORITAS RUTE & REGISTRI ADAPTER JARINGAN**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATNETW".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4E45_5457;

/// Kelas PCI perangkat jaringan.
pub const KELAS_JARINGAN: u8 = 0x02;

/// Jumlah driver pada tabel probe.
pub const DRIVER_JUMLAH: u32 = 3;

// --- penghitung audit -------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    ProdV4 = 0,
    ProdV6 = 1,
    ProdKelas = 2,
    ProdDriver = 3,
    ProdNama = 4,
    TolakParam = 5,
}
const N_COUNTER: usize = 6;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- kebijakan murni --------------------------------------------------------

/// Rute loopback IPv4.
#[inline]
pub fn v4_loopback(a: u8, b: u8, c: u8, d: u8) -> bool {
    (a == 0 && b == 0 && c == 0 && d == 0) || a == 127
}

/// Rute loopback IPv6.
#[inline]
pub fn v6_loopback(is_loopback: bool) -> bool {
    is_loopback
}

/// Saringan kelas PCI.
#[inline]
pub fn kelas_jaringan(class_code: u8) -> bool {
    class_code == KELAS_JARINGAN
}

/// Indeks driver untuk slot urutan; `-1` bila di luar tabel.
#[inline]
pub fn driver_slot(slot: u32) -> i32 {
    if slot < DRIVER_JUMLAH { slot as i32 } else { -1 }
}

/// Tulis `v` desimal ke `buf` pada `pos`; mengembalikan pos baru.
fn tulis_u32(buf: &mut [u8], mut pos: usize, mut v: u32) -> usize {
    if v == 0 {
        if pos < buf.len() {
            buf[pos] = b'0';
            pos += 1;
        }
        return pos;
    }
    let mut tmp = [0u8; 10];
    let mut n = 0usize;
    while v > 0 {
        tmp[n] = b'0' + (v % 10) as u8;
        v /= 10;
        n += 1;
    }
    while n > 0 {
        n -= 1;
        if pos < buf.len() {
            buf[pos] = tmp[n];
            pos += 1;
        }
    }
    pos
}

/// Bentuk nama antarmuka `ep{bus}s{device}`. Mengembalikan panjang nama.
pub fn nama_antarmuka(buf: &mut [u8], bus: u32, device: u32) -> u32 {
    let mut pos = 0usize;
    for ch in [b'e', b'p'] {
        if pos < buf.len() {
            buf[pos] = ch;
            pos += 1;
        }
    }
    pos = tulis_u32(buf, pos, bus);
    if pos < buf.len() {
        buf[pos] = b's';
        pos += 1;
    }
    pos = tulis_u32(buf, pos, device);
    pos as u32
}

// --- pintu C ABI ------------------------------------------------------------

/// Rute loopback IPv4 (1 = loopback).
#[no_mangle]
pub extern "C" fn klatos_netw_v4_loopback(a: u32, b: u32, c: u32, d: u32) -> u32 {
    bump(Counter::ProdV4);
    if v4_loopback(a as u8, b as u8, c as u8, d as u8) { 1 } else { 0 }
}

/// Rute loopback IPv6 (1 = loopback).
#[no_mangle]
pub extern "C" fn klatos_netw_v6_loopback(is_loopback: u32) -> u32 {
    bump(Counter::ProdV6);
    if v6_loopback(is_loopback != 0) { 1 } else { 0 }
}

/// Saringan kelas PCI (1 = jaringan).
#[no_mangle]
pub extern "C" fn klatos_netw_kelas_jaringan(class_code: u32) -> u32 {
    bump(Counter::ProdKelas);
    if kelas_jaringan(class_code as u8) { 1 } else { 0 }
}

/// Indeks driver untuk slot urutan probe.
#[no_mangle]
pub extern "C" fn klatos_netw_driver_slot(slot: u32) -> i32 {
    bump(Counter::ProdDriver);
    driver_slot(slot)
}

/// Jumlah driver pada tabel probe.
#[no_mangle]
pub extern "C" fn klatos_netw_driver_jumlah() -> u32 {
    DRIVER_JUMLAH
}

/// Bentuk nama antarmuka ke `buf` (maks `cap`). Mengembalikan panjang.
///
/// # Safety
/// `buf` dapat ditulis sebanyak `cap` byte.
#[no_mangle]
pub unsafe extern "C" fn klatos_netw_nama_antarmuka(
    bus: u32,
    device: u32,
    buf: *mut u8,
    cap: u32,
) -> u32 {
    if buf.is_null() || cap == 0 {
        bump(Counter::TolakParam);
        return 0;
    }
    let s = unsafe { core::slice::from_raw_parts_mut(buf, cap as usize) };
    let n = nama_antarmuka(s, bus, device);
    bump(Counter::ProdNama);
    n
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_netw_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
#[no_mangle]
pub extern "C" fn klatos_netw_reset_counter() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas jaringan (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosNetReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_v4: u32,
    pub prod_v6: u32,
    pub prod_kelas: u32,
    pub prod_driver: u32,
    pub prod_nama: u32,
    pub cadangan0: u32,
    pub cadangan1: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosNetReport>()];

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

fn run_matrix() -> KlatosNetReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_4E45_5457);
    m.catat(KELAS_JARINGAN == 0x02);
    m.catat(DRIVER_JUMLAH == 3);

    let _ = klatos_netw_reset_counter();

    // A) Rute loopback IPv4.
    m.catat(v4_loopback(0, 0, 0, 0));
    m.catat(v4_loopback(127, 0, 0, 1));
    m.neg(!v4_loopback(10, 0, 0, 1));
    m.catat(unsafe { klatos_netw_v4_loopback(0, 0, 0, 0) } == 1);
    m.catat(unsafe { klatos_netw_v4_loopback(127, 1, 2, 3) } == 1);
    m.catat(unsafe { klatos_netw_v4_loopback(8, 8, 8, 8) } == 0);

    // B) Rute loopback IPv6.
    m.catat(unsafe { klatos_netw_v6_loopback(1) } == 1);
    m.catat(unsafe { klatos_netw_v6_loopback(0) } == 0);

    // C) Kelas PCI.
    m.catat(unsafe { klatos_netw_kelas_jaringan(0x02) } == 1);
    m.catat(unsafe { klatos_netw_kelas_jaringan(0x01) } == 0);

    // D) Urutan driver.
    m.catat(driver_slot(0) == 0 && driver_slot(1) == 1 && driver_slot(2) == 2);
    m.catat(driver_slot(3) == -1);
    m.catat(unsafe { klatos_netw_driver_slot(2) } == 2);
    m.catat(unsafe { klatos_netw_driver_slot(9) } == -1);

    // E) Nama antarmuka.
    {
        let mut b = [0u8; 16];
        let n = nama_antarmuka(&mut b, 0, 31);
        m.catat(n == 6 && &b[..6] == b"ep0s31");
        let n2 = nama_antarmuka(&mut b, 12, 5);
        m.catat(n2 == 6 && &b[..6] == b"ep12s5");
        let n3 = nama_antarmuka(&mut b, 0, 0);
        m.catat(n3 == 5 && &b[..5] == b"ep0s0");
    }
    m.neg(unsafe { klatos_netw_nama_antarmuka(1, 1, core::ptr::null_mut(), 8) } == 0);

    // F) Diferensial v4 vs orakel (LCG deterministik).
    let mut seed: u64 = 0x2026_1005_0000_3001;
    let mut step = 0u32;
    while step < 20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let a = (seed >> 3) as u8;
        let b = (seed >> 11) as u8;
        let c = (seed >> 19) as u8;
        let d = (seed >> 27) as u8;
        let r = v4_loopback(a, b, c, d);
        let o = (a == 0 && b == 0 && c == 0 && d == 0) || a == 127;
        m.dif(r == o);
        step += 1;
    }

    // G) Determinisme.
    {
        let first = driver_slot(1);
        let mut ok = true;
        for _ in 0..1000 {
            if driver_slot(1) != first {
                ok = false;
                break;
            }
        }
        m.neg(ok);
    }

    let prod_v4 = COUNTERS[Counter::ProdV4 as usize].load(Ordering::Relaxed);
    let prod_v6 = COUNTERS[Counter::ProdV6 as usize].load(Ordering::Relaxed);
    let prod_kelas = COUNTERS[Counter::ProdKelas as usize].load(Ordering::Relaxed);
    let prod_driver = COUNTERS[Counter::ProdDriver as usize].load(Ordering::Relaxed);
    let prod_nama = COUNTERS[Counter::ProdNama as usize].load(Ordering::Relaxed);
    let _ = klatos_netw_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && m.dif_checks > 0;
    KlatosNetReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_v4,
        prod_v6,
        prod_kelas,
        prod_driver,
        prod_nama,
        cadangan0: 0,
        cadangan1: 0,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas jaringan.
#[no_mangle]
pub extern "C" fn klatos_netw_selftest() -> KlatosNetReport {
    assert!(core::mem::size_of::<KlatosNetReport>() == 64);
    assert!(core::mem::offset_of!(KlatosNetReport, status) == 60);
    run_matrix()
}
