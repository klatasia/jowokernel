// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas aksi default sinyal Jowo (P1.12).


use core::sync::atomic::{AtomicU32, Ordering};

/// Cermin `SIGINVAL` (`signal_numbers.h:12`).
pub const SIGINVAL: u32 = 0;

/// Cermin `NSIG` (`signal_numbers.h:45`).
pub const NSIG: u32 = 33;

/// Aksi default: hentikan proses.
pub const AKSI_TERMINATE: u8 = 0;
/// Aksi default: abaikan sinyal.
pub const AKSI_IGNORE: u8 = 1;
/// Aksi default: hentikan proses dan buang core.
pub const AKSI_DUMPCORE: u8 = 2;
/// Aksi default: hentikan proses.
pub const AKSI_STOP: u8 = 3;
/// Aksi default: lanjutkan proses.
pub const AKSI_CONTINUE: u8 = 4;

/// Penanda C ABI: sinyal di luar kontrak (`0` atau `>= NSIG`).
pub const AKSI_TOLAK: i32 = -1;

/// Tag laporan; cermin `KlatosSignalActionReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_53494741; // "KLATSIGA"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_signal_action_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan `Terminate`.
    ProdTerminate = 1,
    /// Jumlah keputusan `Ignore`.
    ProdIgnore = 2,
    /// Jumlah keputusan `DumpCore`.
    ProdDumpCore = 3,
    /// Jumlah keputusan `Stop`.
    ProdStop = 4,
    /// Jumlah keputusan `Continue`.
    ProdContinue = 5,
    /// Jumlah penolakan (sinyal di luar kontrak).
    ProdInvalid = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mengembalikan aksi default untuk `signal`, atau `None` bila sinyal di luar
/// kontrak (`0` atau `>= NSIG`).
///
/// SAMA PERSIS dengan `switch` C++ historis (`Thread.cpp:804-845`).
///
/// `# Safety`: tidak ada — fungsi ini murni.
#[inline]
pub const fn putuskan(signal: u32) -> Option<u8> {
    match signal {
        // Terminate.
        1 | 2 | 9 | 13 | 14 | 10 | 12 | 26 | 16 | 29 | 27 | 15 | 32 => Some(AKSI_TERMINATE),
        // Ignore.
        17 | 23 | 28 | 30 => Some(AKSI_IGNORE),
        // DumpCore.
        3 | 4 | 5 | 6 | 7 | 8 | 11 | 24 | 25 | 31 => Some(AKSI_DUMPCORE),
        // Continue.
        18 => Some(AKSI_CONTINUE),
        // Stop.
        19 | 20 | 21 | 22 => Some(AKSI_STOP),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan `0..=4` (aksi default) atau `-1` (sinyal di luar kontrak).
///
/// Menaikkan `prod_calls` dan penghitung per-aksi.
#[no_mangle]
pub extern "C" fn klatos_signal_action_decide(signal: u32) -> i32 {
    bump(Counter::ProdCalls);
    match putuskan(signal) {
        Some(AKSI_TERMINATE) => {
            bump(Counter::ProdTerminate);
            AKSI_TERMINATE as i32
        }
        Some(AKSI_IGNORE) => {
            bump(Counter::ProdIgnore);
            AKSI_IGNORE as i32
        }
        Some(AKSI_DUMPCORE) => {
            bump(Counter::ProdDumpCore);
            AKSI_DUMPCORE as i32
        }
        Some(AKSI_STOP) => {
            bump(Counter::ProdStop);
            AKSI_STOP as i32
        }
        Some(AKSI_CONTINUE) => {
            bump(Counter::ProdContinue);
            AKSI_CONTINUE as i32
        }
        _ => {
            bump(Counter::ProdInvalid);
            AKSI_TOLAK
        }
    }
}

/// Cermin `NSIG`. Konstanta murni.
#[no_mangle]
pub extern "C" fn klatos_signal_action_nsig() -> u32 {
    NSIG
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_signal_action_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_signal_action_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosSignalActionReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosSignalActionReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_terminate: u32,
    pub prod_ignore: u32,
    pub prod_dumpcore: u32,
    pub prod_stop: u32,
    pub prod_continue: u32,
    pub prod_invalid: u32,
    pub nsig: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Peta,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    peta_checks: u32,
    peta_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            peta_checks: 0,
            peta_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Peta => {
                self.peta_checks += 1;
                if lulus {
                    self.peta_passed += 1;
                }
            }
            Group::Neg => {
                self.neg_checks += 1;
                if lulus {
                    self.neg_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Acuan diferensial: `switch` C++ historis, ditulis ulang di Rust HANYA
/// sebagai pembanding matriks. Sengaja TERPISAH dari [`putuskan`] supaya
/// keduanya harus setuju.
const fn acuan_cpp(signal: u32) -> Option<u8> {
    match signal {
        1 => Some(AKSI_TERMINATE),   // SIGHUP
        2 => Some(AKSI_TERMINATE),   // SIGINT
        9 => Some(AKSI_TERMINATE),   // SIGKILL
        13 => Some(AKSI_TERMINATE),  // SIGPIPE
        14 => Some(AKSI_TERMINATE),  // SIGALRM
        10 => Some(AKSI_TERMINATE),  // SIGUSR1
        12 => Some(AKSI_TERMINATE),  // SIGUSR2
        26 => Some(AKSI_TERMINATE),  // SIGVTALRM
        16 => Some(AKSI_TERMINATE),  // SIGSTKFLT
        29 => Some(AKSI_TERMINATE),  // SIGIO
        27 => Some(AKSI_TERMINATE),  // SIGPROF
        15 => Some(AKSI_TERMINATE),  // SIGTERM
        32 => Some(AKSI_TERMINATE),  // SIGCANCEL
        17 => Some(AKSI_IGNORE),     // SIGCHLD
        23 => Some(AKSI_IGNORE),     // SIGURG
        28 => Some(AKSI_IGNORE),     // SIGWINCH
        30 => Some(AKSI_IGNORE),     // SIGINFO
        3 => Some(AKSI_DUMPCORE),    // SIGQUIT
        4 => Some(AKSI_DUMPCORE),    // SIGILL
        5 => Some(AKSI_DUMPCORE),    // SIGTRAP
        6 => Some(AKSI_DUMPCORE),    // SIGABRT
        7 => Some(AKSI_DUMPCORE),    // SIGBUS
        8 => Some(AKSI_DUMPCORE),    // SIGFPE
        11 => Some(AKSI_DUMPCORE),   // SIGSEGV
        24 => Some(AKSI_DUMPCORE),   // SIGXCPU
        25 => Some(AKSI_DUMPCORE),   // SIGXFSZ
        31 => Some(AKSI_DUMPCORE),   // SIGSYS
        18 => Some(AKSI_CONTINUE),   // SIGCONT
        19 => Some(AKSI_STOP),       // SIGSTOP
        20 => Some(AKSI_STOP),       // SIGTSTP
        21 => Some(AKSI_STOP),       // SIGTTIN
        22 => Some(AKSI_STOP),       // SIGTTOU
        _ => None,
    }
}

/// Matriks otoritas aksi default sinyal. Input SINTETIS.
fn run_matrix() -> KlatosSignalActionReport {
    let mut m = Matrix::new();

    // --- kelompok 0: pemetaan penuh 1..=32 --------------------------------
    let mut peta_ok = true;
    let mut diukur = 0u32;
    for signal in 1..NSIG {
        diukur += 1;
        if putuskan(signal) != acuan_cpp(signal) {
            peta_ok = false;
        }
    }
    m.catat(diukur == 32, Group::Peta);
    m.catat(peta_ok, Group::Peta);
    // Setiap sinyal 1..=32 WAJIB terpetakan (tidak ada yang jatuh ke `_`).
    let mut semua_terpetakan = true;
    for signal in 1..NSIG {
        if putuskan(signal).is_none() {
            semua_terpetakan = false;
        }
    }
    m.catat(semua_terpetakan, Group::Peta);
    // Jumlah per-aksi.
    let mut n_term = 0;
    let mut n_ign = 0;
    let mut n_core = 0;
    let mut n_stop = 0;
    let mut n_cont = 0;
    for signal in 1..NSIG {
        match putuskan(signal) {
            Some(AKSI_TERMINATE) => n_term += 1,
            Some(AKSI_IGNORE) => n_ign += 1,
            Some(AKSI_DUMPCORE) => n_core += 1,
            Some(AKSI_STOP) => n_stop += 1,
            Some(AKSI_CONTINUE) => n_cont += 1,
            _ => {}
        }
    }
    m.catat(n_term == 13, Group::Peta);
    m.catat(n_ign == 4, Group::Peta);
    m.catat(n_core == 10, Group::Peta);
    m.catat(n_stop == 4, Group::Peta);
    m.catat(n_cont == 1, Group::Peta);
    m.catat(n_term + n_ign + n_core + n_stop + n_cont == 32, Group::Peta);

    // --- kelompok 1: negatif ------------------------------------------------
    m.catat(NSIG == 33, Group::Neg);
    m.catat(SIGINVAL == 0, Group::Neg);
    m.catat(putuskan(0).is_none(), Group::Neg);
    m.catat(putuskan(NSIG).is_none(), Group::Neg);
    m.catat(putuskan(NSIG + 100).is_none(), Group::Neg);
    m.catat(putuskan(u32::MAX).is_none(), Group::Neg);
    m.catat(AKSI_TOLAK == -1, Group::Neg);
    // Kontrol: kelas yang berbeda benar-benar berbeda.
    m.catat(putuskan(17) == Some(AKSI_IGNORE) && putuskan(18) == Some(AKSI_CONTINUE), Group::Neg);
    m.catat(putuskan(19) == Some(AKSI_STOP) && putuskan(6) == Some(AKSI_DUMPCORE), Group::Neg);
    // Konsistensi: 1000 panggilan input sama -> keputusan sama.
    let mut last: Option<Option<u8>> = None;
    let mut ok = true;
    for _ in 0..1000 {
        let r = putuskan(17);
        if last.is_none() {
            last = Some(r);
        } else if last != Some(r) {
            ok = false;
            break;
        }
    }
    m.catat(ok, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosSignalActionReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_signal_action_counter(Counter::ProdCalls as u32),
        prod_terminate: klatos_signal_action_counter(Counter::ProdTerminate as u32),
        prod_ignore: klatos_signal_action_counter(Counter::ProdIgnore as u32),
        prod_dumpcore: klatos_signal_action_counter(Counter::ProdDumpCore as u32),
        prod_stop: klatos_signal_action_counter(Counter::ProdStop as u32),
        prod_continue: klatos_signal_action_counter(Counter::ProdContinue as u32),
        prod_invalid: klatos_signal_action_counter(Counter::ProdInvalid as u32),
        nsig: NSIG,
        status,
    }
}

/// Menjalankan matriks otoritas aksi default sinyal.
#[no_mangle]
pub extern "C" fn klatos_signal_action_selftest() -> KlatosSignalActionReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(NSIG == 33);
        assert!(SIGINVAL == 0);
        assert!(AKSI_TERMINATE == 0);
        assert!(AKSI_IGNORE == 1);
        assert!(AKSI_DUMPCORE == 2);
        assert!(AKSI_STOP == 3);
        assert!(AKSI_CONTINUE == 4);
        assert!(AKSI_TOLAK == -1);
        assert!(REPORT_TAG == 0x4B4C_4154_53494741);

        // Kelas kunci byte-identik.
        assert!(matches!(putuskan(1), Some(AKSI_TERMINATE)));   // SIGHUP
        assert!(matches!(putuskan(17), Some(AKSI_IGNORE)));     // SIGCHLD
        assert!(matches!(putuskan(11), Some(AKSI_DUMPCORE)));   // SIGSEGV
        assert!(matches!(putuskan(18), Some(AKSI_CONTINUE)));   // SIGCONT
        assert!(matches!(putuskan(19), Some(AKSI_STOP)));       // SIGSTOP
        assert!(putuskan(0).is_none());
        assert!(putuskan(NSIG).is_none());

        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInvalid as usize == 6);

        // 8 (tag) + 14*4 (u32) + 4 (status) = 68 -> 72 dengan padding.
        assert!(core::mem::size_of::<KlatosSignalActionReport>() == 72);
        assert!(core::mem::align_of::<KlatosSignalActionReport>() == 8);
    };
}
