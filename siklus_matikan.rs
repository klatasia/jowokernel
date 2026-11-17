// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


// KR-30 — **OTORITAS PERINTAH & PENGURASAN DAYA (PowerStateSwitchTask)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATMATK".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4D41_544B;

/// Aksi perintah: shutdown (jangan reboot).
pub const AKSI_SHUTDOWN: i32 = 0;
/// Aksi perintah: reboot.
pub const AKSI_REBOOT: i32 = 1;
/// Perintah tak dikenal (pemanggil PANIC).
pub const AKSI_TAK_DIKENAL: i32 = -1;

/// Perintah mentah (cermin `PowerStateCommand`).
pub const CMD_SHUTDOWN: u64 = 0;
pub const CMD_REBOOT: u64 = 1;

// --- penghitung audit -------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    ProdPerintah = 0,
    ProdShutdown = 1,
    ProdReboot = 2,
    ProdLanjut = 3,
    ProdSelesai = 4,
    TolakParam = 5,
}
const N_COUNTER: usize = 6;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- kebijakan murni --------------------------------------------------------

/// Petakan perintah mentah ke aksi.
#[inline]
pub fn perintah(cmd: u64) -> i32 {
    if cmd == CMD_SHUTDOWN {
        AKSI_SHUTDOWN
    } else if cmd == CMD_REBOOT {
        AKSI_REBOOT
    } else {
        AKSI_TAK_DIKENAL
    }
}

/// Mesin keadaan pengurasan: `true` = masih harus `yield` lagi.
#[inline]
pub fn lanjut(alive_process_count: u32) -> bool {
    alive_process_count > 0
}

// --- pintu C ABI ------------------------------------------------------------

/// Petakan perintah ke aksi (`AKSI_*`).
#[no_mangle]
pub extern "C" fn klatos_matikan_perintah(cmd: u64) -> i32 {
    bump(Counter::ProdPerintah);
    let a = perintah(cmd);
    if a == AKSI_SHUTDOWN {
        bump(Counter::ProdShutdown);
    } else if a == AKSI_REBOOT {
        bump(Counter::ProdReboot);
    }
    a
}

/// Mesin keadaan pengurasan (1 = lanjut, 0 = selesai).
#[no_mangle]
pub extern "C" fn klatos_matikan_lanjut(alive_process_count: u32) -> u32 {
    if lanjut(alive_process_count) {
        bump(Counter::ProdLanjut);
        1
    } else {
        bump(Counter::ProdSelesai);
        0
    }
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_matikan_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
#[no_mangle]
pub extern "C" fn klatos_matikan_reset_counter() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas daya (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosMatikanReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_perintah: u32,
    pub prod_shutdown: u32,
    pub prod_reboot: u32,
    pub prod_lanjut: u32,
    pub prod_selesai: u32,
    pub cadangan0: u32,
    pub cadangan1: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosMatikanReport>()];

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

fn run_matrix() -> KlatosMatikanReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_4D41_544B);
    m.catat(CMD_SHUTDOWN == 0 && CMD_REBOOT == 1);

    let _ = klatos_matikan_reset_counter();

    // A) Pemetaan perintah.
    m.catat(perintah(CMD_SHUTDOWN) == AKSI_SHUTDOWN);
    m.catat(perintah(CMD_REBOOT) == AKSI_REBOOT);
    m.neg(perintah(7) == AKSI_TAK_DIKENAL);
    m.catat(klatos_matikan_perintah(CMD_SHUTDOWN) == AKSI_SHUTDOWN);
    m.catat(klatos_matikan_perintah(CMD_REBOOT) == AKSI_REBOOT);
    m.catat(klatos_matikan_perintah(7) == AKSI_TAK_DIKENAL);

    // B) Pengurasan.
    m.catat(lanjut(1));
    m.catat(lanjut(100));
    m.neg(!lanjut(0));
    m.catat(klatos_matikan_lanjut(3) == 1);
    m.catat(klatos_matikan_lanjut(0) == 0);

    // C) Simulasi pengurasan turun ke 0.
    {
        let mut n = 5u32;
        let mut iter = 0u32;
        while lanjut(n) {
            n -= 1;
            iter += 1;
        }
        m.catat(iter == 5 && n == 0);
    }

    // D) Diferensial vs orakel (LCG deterministik).
    let mut seed: u64 = 0x2026_1005_0000_3003;
    let mut step = 0u32;
    while step < 20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let n = (seed >> 9) as u32 & 0xFFFF;
        let r = lanjut(n);
        let o = n > 0;
        m.dif(r == o);
        let cmd = (seed >> 40) & 3;
        let rc = perintah(cmd);
        let ro = if cmd == 0 { 0 } else if cmd == 1 { 1 } else { -1 };
        m.dif(rc == ro);
        step += 1;
    }

    // E) Determinisme.
    {
        let first = perintah(CMD_REBOOT);
        let mut ok = true;
        for _ in 0..1000 {
            if perintah(CMD_REBOOT) != first {
                ok = false;
                break;
            }
        }
        m.neg(ok);
    }

    let prod_perintah = COUNTERS[Counter::ProdPerintah as usize].load(Ordering::Relaxed);
    let prod_shutdown = COUNTERS[Counter::ProdShutdown as usize].load(Ordering::Relaxed);
    let prod_reboot = COUNTERS[Counter::ProdReboot as usize].load(Ordering::Relaxed);
    let prod_lanjut = COUNTERS[Counter::ProdLanjut as usize].load(Ordering::Relaxed);
    let prod_selesai = COUNTERS[Counter::ProdSelesai as usize].load(Ordering::Relaxed);
    let _ = klatos_matikan_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && m.dif_checks > 0;
    KlatosMatikanReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_perintah,
        prod_shutdown,
        prod_reboot,
        prod_lanjut,
        prod_selesai,
        cadangan0: 0,
        cadangan1: 0,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas daya.
#[no_mangle]
pub extern "C" fn klatos_matikan_selftest() -> KlatosMatikanReport {
    assert!(core::mem::size_of::<KlatosMatikanReport>() == 64);
    assert!(core::mem::offset_of!(KlatosMatikanReport, status) == 60);
    run_matrix()
}
