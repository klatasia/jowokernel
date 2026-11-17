// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas validasi `signum` sigaction Jowo (P1.24).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;

/// Cermin `NSIG` (`signal_numbers.h:45`).
pub const NSIG: i32 = 33;

/// Cermin `SIGKILL` (`signal_numbers.h:21`).
pub const SIGKILL: i32 = 9;

/// Cermin `SIGSTOP` (`signal_numbers.h:31`).
pub const SIGSTOP: i32 = 19;

/// Tag laporan; cermin `KlatosSignalNoReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5349_474E; // "KLATSIGN"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_validasi_nomor_sinyal_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan sah (`0`).
    ProdValid = 1,
    /// Jumlah penolakan `signum < 1` (di bawah rentang).
    ProdInvalidLow = 2,
    /// Jumlah penolakan `signum >= NSIG` (di atas rentang).
    ProdInvalidHigh = 3,
    /// Jumlah penolakan `signum == SIGKILL`.
    ProdInvalidKill = 4,
    /// Jumlah penolakan `signum == SIGSTOP`.
    ProdInvalidStop = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan validitas `signum`.
///
/// Urutan dan isi keputusan SAMA PERSIS dengan `sigaction.cpp:58` historis.
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(signum: i32) -> i32 {
    if signum < 1 || signum >= NSIG || signum == SIGKILL || signum == SIGSTOP {
        return -EINVAL;
    }
    0
}

/// Apakah hasil ini keputusan sah?
#[inline]
pub const fn berhasil(hasil: i32) -> bool {
    hasil == 0
}

/// Apakah `signum` masuk kategori penolakan tertentu? Dipakai matriks.
#[inline]
pub const fn adalah_penolakan(signum: i32) -> bool {
    signum < 1 || signum >= NSIG || signum == SIGKILL || signum == SIGSTOP
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan validitas `signum`. Lihat [`putuskan`] untuk kontrak.
///
/// Menaikkan `prod_calls` dan penghitung penolakan yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_validasi_nomor_sinyal_decide(signum: i32) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(signum);
    if hasil == 0 {
        bump(Counter::ProdValid);
    } else {
        if signum < 1 {
            bump(Counter::ProdInvalidLow);
        } else if signum >= NSIG {
            bump(Counter::ProdInvalidHigh);
        } else if signum == SIGKILL {
            bump(Counter::ProdInvalidKill);
        } else if signum == SIGSTOP {
            bump(Counter::ProdInvalidStop);
        }
    }
    hasil
}

/// Nilai errno yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_validasi_nomor_sinyal_einval() -> i32 {
    EINVAL
}
/// `NSIG=33`.
#[no_mangle]
pub extern "C" fn klatos_validasi_nomor_sinyal_nsig() -> i32 {
    NSIG
}
/// `SIGKILL=9`.
#[no_mangle]
pub extern "C" fn klatos_validasi_nomor_sinyal_sigkill() -> i32 {
    SIGKILL
}
/// `SIGSTOP=19`.
#[no_mangle]
pub extern "C" fn klatos_validasi_nomor_sinyal_sigstop() -> i32 {
    SIGSTOP
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_validasi_nomor_sinyal_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_validasi_nomor_sinyal_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosSignalNoReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosSignalNoReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_valid: u32,
    pub prod_invalid_low: u32,
    pub prod_invalid_high: u32,
    pub prod_invalid_kill: u32,
    pub prod_invalid_stop: u32,
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

/// Matriks otoritas validasi `signum`. Input SINTETIS.
fn run_matrix() -> KlatosSignalNoReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Bawah: < 1.
    m.catat(putuskan(0) == -EINVAL, Group::Peta);
    m.catat(putuskan(-1) == -EINVAL, Group::Peta);
    m.catat(putuskan(-100) == -EINVAL, Group::Peta);
    m.catat(putuskan(i32::MIN) == -EINVAL, Group::Peta);
    // Bawah yang dekat.
    m.catat(putuskan(0) == putuskan(-1), Group::Peta);
    // Atas: >= NSIG (33).
    m.catat(putuskan(NSIG) == -EINVAL, Group::Peta);
    m.catat(putuskan(NSIG + 1) == -EINVAL, Group::Peta);
    m.catat(putuskan(99) == -EINVAL, Group::Peta);
    m.catat(putuskan(i32::MAX) == -EINVAL, Group::Peta);
    // SIGKILL=9.
    m.catat(putuskan(SIGKILL) == -EINVAL, Group::Peta);
    // SIGSTOP=19.
    m.catat(putuskan(SIGSTOP) == -EINVAL, Group::Peta);
    // Sah: 1..8 kecuali 9 (SIGKILL).
    m.catat(putuskan(1) == 0, Group::Peta);
    m.catat(putuskan(2) == 0, Group::Peta);
    m.catat(putuskan(8) == 0, Group::Peta);
    // Sah: 10..18 (lewati SIGKILL=9).
    m.catat(putuskan(10) == 0, Group::Peta);
    m.catat(putuskan(13) == 0, Group::Peta);
    m.catat(putuskan(17) == 0, Group::Peta);
    m.catat(putuskan(18) == 0, Group::Peta);
    // Sah: 20..32 (lewati SIGSTOP=19).
    m.catat(putuskan(20) == 0, Group::Peta);
    m.catat(putuskan(22) == 0, Group::Peta);
    m.catat(putuskan(28) == 0, Group::Peta);
    m.catat(putuskan(32) == 0, Group::Peta);
    // 32 adalah NSIG-1 dan sah; 33 (NSIG) ditolak. Keduanya dekat.
    m.catat(putuskan(32) == 0 && putuskan(NSIG) == -EINVAL, Group::Peta);

    // --- kelompok 1: sapuan ekshaustif --------------------------------------
    // signum -2..=NSIG+1 = 37 nilai.
    {
        let mut n = 0u32;
        let mut total_sah = 0u32;
        let mut total_tolak = 0u32;
        let mut signum = -2i32;
        while signum <= NSIG + 1 {
            let r = putuskan(signum);
            n += 1;
            if r == 0 {
                total_sah += 1;
            } else if r == -EINVAL {
                total_tolak += 1;
            }
            signum += 1;
        }
        m.catat(n == 37, Group::Peta);
        // 30 sinyal sah (1..32 minus 9 minus 19) dan 7 ditolak (-2, -1, 0, 9,
        // 19, 33, 34).
        m.catat(total_sah == 30 && total_tolak == 7, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(NSIG == 33, Group::Neg);
    m.catat(SIGKILL == 9, Group::Neg);
    m.catat(SIGSTOP == 19, Group::Neg);
    m.catat(REPORT_TAG == 0x4B4C_4154_5349_474E, Group::Neg);
    // Empat konstanta berbeda satu sama lain.
    m.catat(EINVAL != NSIG && NSIG != SIGKILL && SIGKILL != SIGSTOP, Group::Neg);
    // SIGKILL dan SIGSTOP berada dalam rentang sah, penolakannya eksplisit.
    {
        let mut ok = true;
        if SIGKILL >= NSIG || SIGKILL < 1 {
            ok = false;
        }
        if SIGSTOP >= NSIG || SIGSTOP < 1 {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }
    // Determinisme.
    {
        let mut last: Option<i32> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(SIGKILL);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: `berhasil` benar hanya untuk `0`.
    {
        let mut ok = true;
        if berhasil(0) != true {
            ok = false;
        }
        if berhasil(-EINVAL) != false {
            ok = false;
        }
        if berhasil(1) != false {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }
    // `adalah_penolakan` cocok dengan `putuskan` untuk seluruh rentang.
    {
        let mut ok = true;
        let mut signum = -3i32;
        while signum <= NSIG + 2 {
            let r = putuskan(signum);
            let penolakan = adalah_penolakan(signum);
            if (r == -EINVAL) != penolakan {
                ok = false;
            }
            signum += 1;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosSignalNoReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_validasi_nomor_sinyal_counter(Counter::ProdCalls as u32),
        prod_valid: klatos_validasi_nomor_sinyal_counter(Counter::ProdValid as u32),
        prod_invalid_low: klatos_validasi_nomor_sinyal_counter(Counter::ProdInvalidLow as u32),
        prod_invalid_high: klatos_validasi_nomor_sinyal_counter(Counter::ProdInvalidHigh as u32),
        prod_invalid_kill: klatos_validasi_nomor_sinyal_counter(Counter::ProdInvalidKill as u32),
        prod_invalid_stop: klatos_validasi_nomor_sinyal_counter(Counter::ProdInvalidStop as u32),
        status,
    }
}

/// Menjalankan matriks otoritas validasi `signum`.
#[no_mangle]
pub extern "C" fn klatos_validasi_nomor_sinyal_selftest() -> KlatosSignalNoReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EINVAL == 22);
        assert!(NSIG == 33);
        assert!(SIGKILL == 9);
        assert!(SIGSTOP == 19);
        assert!(REPORT_TAG == 0x4B4C_4154_5349_474E);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInvalidStop as usize == 5);

        assert!(putuskan(0) == -EINVAL);
        assert!(putuskan(-1) == -EINVAL);
        assert!(putuskan(NSIG) == -EINVAL);
        assert!(putuskan(SIGKILL) == -EINVAL);
        assert!(putuskan(SIGSTOP) == -EINVAL);
        assert!(putuskan(1) == 0);
        assert!(putuskan(8) == 0);
        assert!(putuskan(10) == 0);
        assert!(putuskan(18) == 0);
        assert!(putuskan(20) == 0);
        assert!(putuskan(32) == 0);

        // 8 (tag) + 12*4 (u32) + 4 (status) = 60, padded ke kelipatan 8 = 64.
        assert!(core::mem::size_of::<KlatosSignalNoReport>() == 64);
        assert!(core::mem::align_of::<KlatosSignalNoReport>() == 8);
    };
}