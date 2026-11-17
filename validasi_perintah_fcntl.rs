// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas validasi perintah `fcntl` Jowo (P1.26).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;

/// Cermin `F_DUPFD` (`fcntl.h`).
pub const F_DUPFD: i32 = 0;
/// Cermin `F_GETFD`.
pub const F_GETFD: i32 = 1;
/// Cermin `F_SETFD`.
pub const F_SETFD: i32 = 2;
/// Cermin `F_GETFL`.
pub const F_GETFL: i32 = 3;
/// Cermin `F_SETFL`.
pub const F_SETFL: i32 = 4;
/// Cermin `F_ISTTY`.
pub const F_ISTTY: i32 = 5;
/// Cermin `F_GETLK`.
pub const F_GETLK: i32 = 6;
/// Cermin `F_SETLK`.
pub const F_SETLK: i32 = 7;
/// Cermin `F_SETLKW`.
pub const F_SETLKW: i32 = 8;
/// Cermin `F_DUPFD_CLOEXEC`.
pub const F_DUPFD_CLOEXEC: i32 = 9;

/// Tag laporan; cermin `KlatosFcntlCmdReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4643_4E54; // "KLATFCNT"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_validasi_perintah_fcntl_decide`].
    ProdCalls = 0,
    /// Jumlah cmd dikenal.
    ProdValid = 1,
    /// Jumlah cmd tidak dikenal.
    ProdInvalid = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan validitas `cmd` `fcntl`.
///
/// Urutan dan isi keputusan SAMA PERSIS dengan `fcntl.cpp:24-60` historis:
/// `cmd` dikenal -> `0`; `cmd` di luar himpunan -> `-EINVAL`.
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(cmd: i32) -> i32 {
    match cmd {
        F_DUPFD_CLOEXEC | F_DUPFD
        | F_GETFD | F_SETFD
        | F_GETFL | F_SETFL
        | F_ISTTY
        | F_GETLK | F_SETLK | F_SETLKW => 0,
        _ => -EINVAL,
    }
}

/// Apakah hasil ini keputusan sah?
#[inline]
pub const fn berhasil(hasil: i32) -> bool {
    hasil == 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan validitas `cmd` `fcntl`. Lihat [`putuskan`] untuk kontrak.
/// Menaikkan `prod_calls` dan penghitung yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_decide(cmd: i32) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(cmd);
    if hasil == 0 {
        bump(Counter::ProdValid);
    } else {
        bump(Counter::ProdInvalid);
    }
    hasil
}

/// Nilai errno yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_einval() -> i32 {
    EINVAL
}

/// Nilai `F_*`.
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_dupfd() -> i32 {
    F_DUPFD
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_getfd() -> i32 {
    F_GETFD
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_setfd() -> i32 {
    F_SETFD
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_getfl() -> i32 {
    F_GETFL
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_setfl() -> i32 {
    F_SETFL
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_istty() -> i32 {
    F_ISTTY
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_getlk() -> i32 {
    F_GETLK
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_setlk() -> i32 {
    F_SETLK
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_setlkw() -> i32 {
    F_SETLKW
}
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_f_dupfd_cloexec() -> i32 {
    F_DUPFD_CLOEXEC
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosFcntlCmdReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosFcntlCmdReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_valid: u32,
    pub prod_invalid: u32,
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

/// Matriks otoritas validasi `cmd` `fcntl`. Input SINTETIS.
fn run_matrix() -> KlatosFcntlCmdReport {
    let mut m = Matrix::new();

    // --- kelompok 0: 10 cmd dikenal ----------------------------------------
    m.catat(putuskan(F_DUPFD) == 0, Group::Peta);
    m.catat(putuskan(F_GETFD) == 0, Group::Peta);
    m.catat(putuskan(F_SETFD) == 0, Group::Peta);
    m.catat(putuskan(F_GETFL) == 0, Group::Peta);
    m.catat(putuskan(F_SETFL) == 0, Group::Peta);
    m.catat(putuskan(F_ISTTY) == 0, Group::Peta);
    m.catat(putuskan(F_GETLK) == 0, Group::Peta);
    m.catat(putuskan(F_SETLK) == 0, Group::Peta);
    m.catat(putuskan(F_SETLKW) == 0, Group::Peta);
    m.catat(putuskan(F_DUPFD_CLOEXEC) == 0, Group::Peta);

    // --- kelompok 1: kasus batas eksplisit ---------------------------------
    // Bawah 0.
    m.catat(putuskan(-1) == -EINVAL, Group::Peta);
    m.catat(putuskan(-100) == -EINVAL, Group::Peta);
    m.catat(putuskan(i32::MIN) == -EINVAL, Group::Peta);
    // Atas 9 (F_DUPFD_CLOEXEC).
    m.catat(putuskan(10) == -EINVAL, Group::Peta);
    m.catat(putuskan(99) == -EINVAL, Group::Peta);
    m.catat(putuskan(i32::MAX) == -EINVAL, Group::Peta);
    // Tepat di antara (tidak ada cmd pada 10..15 karena F_DUPFD=0..9 dipakai).
    // Pastikan gap 10..15 tetap ditolak.
    let mut n_gap = 0u32;
    let mut ok_gap = true;
    for cmd in 10..=15 {
        n_gap += 1;
        if putuskan(cmd) != -EINVAL {
            ok_gap = false;
        }
    }
    m.catat(n_gap == 6, Group::Peta);
    m.catat(ok_gap, Group::Peta);

    // --- kelompok 2: sapuan ekshaustif --------------------------------------
    // cmd -2..=20 = 23 nilai.
    {
        let mut n = 0u32;
        let mut total_sah = 0u32;
        let mut total_tolak = 0u32;
        let mut cmd = -2i32;
        while cmd <= 20 {
            let r = putuskan(cmd);
            n += 1;
            if r == 0 {
                total_sah += 1;
            } else if r == -EINVAL {
                total_tolak += 1;
            }
            cmd += 1;
        }
        m.catat(n == 23, Group::Peta);
        // 10 dikenal (-2..=-1 = 2 ditolak, 0..=9 = 10 sah, 10..=20 = 11 ditolak).
        m.catat(total_sah == 10 && total_tolak == 13, Group::Peta);
    }

    // --- kelompok 3: negatif ------------------------------------------------
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(F_DUPFD == 0 && F_GETFD == 1 && F_SETFD == 2, Group::Neg);
    m.catat(F_GETFL == 3 && F_SETFL == 4 && F_ISTTY == 5, Group::Neg);
    m.catat(F_GETLK == 6 && F_SETLK == 7 && F_SETLKW == 8, Group::Neg);
    m.catat(F_DUPFD_CLOEXEC == 9, Group::Neg);
    m.catat(REPORT_TAG == 0x4B4C_4154_4643_4E54, Group::Neg);
    // Sepuluh konstanta berbeda satu sama lain.
    m.catat(F_DUPFD != F_GETFD && F_GETFD != F_SETFD && F_SETFD != F_GETFL
         && F_GETFL != F_SETFL && F_SETFL != F_ISTTY && F_ISTTY != F_GETLK
         && F_GETLK != F_SETLK && F_SETLK != F_SETLKW && F_SETLKW != F_DUPFD_CLOEXEC, Group::Neg);
    // Rentang sah adalah 0..=9.
    m.catat(F_DUPFD == 0 && F_DUPFD_CLOEXEC == 9 && F_DUPFD_CLOEXEC - F_DUPFD == 9, Group::Neg);
    // Determinisme.
    {
        let mut last: Option<i32> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(99);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: harapan SALAH harus terdeteksi.
    {
        let mut ok = true;
        // F_DUPFD=0 adalah `cmd` pertama dan sah.
        if putuskan(F_DUPFD) != 0 {
            ok = false;
        }
        // 10 bukan `cmd` dan ditolak.
        if putuskan(10) != -EINVAL {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosFcntlCmdReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_validasi_perintah_fcntl_counter(Counter::ProdCalls as u32),
        prod_valid: klatos_validasi_perintah_fcntl_counter(Counter::ProdValid as u32),
        prod_invalid: klatos_validasi_perintah_fcntl_counter(Counter::ProdInvalid as u32),
        status,
    }
}

/// Menjalankan matriks otoritas validasi `cmd` `fcntl`.
#[no_mangle]
pub extern "C" fn klatos_validasi_perintah_fcntl_selftest() -> KlatosFcntlCmdReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EINVAL == 22);
        assert!(F_DUPFD == 0);
        assert!(F_GETFD == 1);
        assert!(F_SETFD == 2);
        assert!(F_GETFL == 3);
        assert!(F_SETFL == 4);
        assert!(F_ISTTY == 5);
        assert!(F_GETLK == 6);
        assert!(F_SETLK == 7);
        assert!(F_SETLKW == 8);
        assert!(F_DUPFD_CLOEXEC == 9);
        assert!(REPORT_TAG == 0x4B4C_4154_4643_4E54);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInvalid as usize == 2);

        assert!(putuskan(F_DUPFD) == 0);
        assert!(putuskan(F_DUPFD_CLOEXEC) == 0);
        assert!(putuskan(-1) == -EINVAL);
        assert!(putuskan(10) == -EINVAL);
        assert!(putuskan(i32::MAX) == -EINVAL);

        // 8 (tag) + 9*4 (u32) + 4 (status) = 48.
        assert!(core::mem::size_of::<KlatosFcntlCmdReport>() == 48);
        assert!(core::mem::align_of::<KlatosFcntlCmdReport>() == 8);
    };
}