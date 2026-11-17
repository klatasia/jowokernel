// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas mode akses baca/tulis deskriptor berkas Jowo (P1.11).


use core::sync::atomic::{AtomicU32, Ordering};

/// Cermin `<fcntl.h>` `O_RDONLY`.
pub const O_RDONLY: i32 = 1 << 0;

/// Cermin `<fcntl.h>` `O_WRONLY`.
pub const O_WRONLY: i32 = 1 << 1;

/// Bit hasil: deskriptor dapat dibaca.
pub const HASIL_READABLE: u32 = 1 << 0;

/// Bit hasil: deskriptor dapat ditulis.
pub const HASIL_WRITABLE: u32 = 1 << 1;

/// Tag laporan; cermin `KlatosFdModeReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_46444D44; // "KLATFDMD"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_fd_mode_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan dengan bit dapat-dibaca.
    ProdReadable = 1,
    /// Jumlah keputusan dengan bit dapat-ditulis.
    ProdWritable = 2,
    /// Jumlah keputusan tanpa kedua bit (mis. `O_PATH`).
    ProdNeither = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mengembalikan `(readable, writable)` untuk `options`.
///
/// SAMA PERSIS dengan C++ historis (`OpenFileDescription.cpp:513-514`).
///
/// `# Safety`: tidak ada — fungsi ini murni.
#[inline]
pub const fn putuskan(options: i32) -> (bool, bool) {
    let readable = options & O_RDONLY == O_RDONLY;
    let writable = options & O_WRONLY == O_WRONLY;
    (readable, writable)
}

/// Bentuk bit dari [`putuskan`] untuk C ABI: `1=readable`, `2=writable`.
#[inline]
pub const fn putuskan_mask(options: i32) -> u32 {
    let (readable, writable) = putuskan(options);
    let mut mask = 0u32;
    if readable {
        mask |= HASIL_READABLE;
    }
    if writable {
        mask |= HASIL_WRITABLE;
    }
    mask
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan `1` (dapat dibaca), `2` (dapat ditulis), `3` (keduanya),
/// atau `0` (tidak keduanya).
///
/// Menaikkan `prod_calls` dan penghitung per-kelas.
#[no_mangle]
pub extern "C" fn klatos_fd_mode_decide(options: i32) -> u32 {
    let mask = putuskan_mask(options);
    bump(Counter::ProdCalls);
    match mask {
        0 => bump(Counter::ProdNeither),
        _ => {
            if mask & HASIL_READABLE != 0 {
                bump(Counter::ProdReadable);
            }
            if mask & HASIL_WRITABLE != 0 {
                bump(Counter::ProdWritable);
            }
        }
    }
    mask
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_fd_mode_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_fd_mode_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosFdModeReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosFdModeReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub mode_checks: u32,
    pub mode_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_readable: u32,
    pub prod_writable: u32,
    pub prod_neither: u32,
    pub bit_readable: u32,
    pub bit_writable: u32,
    pub sapuan_checks: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Mode,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    mode_checks: u32,
    mode_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            mode_checks: 0,
            mode_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Mode => {
                self.mode_checks += 1;
                if lulus {
                    self.mode_passed += 1;
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

/// Matriks otoritas mode akses deskriptor. Input SINTETIS.
fn run_matrix() -> KlatosFdModeReport {
    let mut m = Matrix::new();

    // --- kelompok 0: pemetaan mode -----------------------------------------
    m.catat(putuskan(O_RDONLY) == (true, false), Group::Mode);
    m.catat(putuskan(O_WRONLY) == (false, true), Group::Mode);
    m.catat(putuskan(O_RDONLY | O_WRONLY) == (true, true), Group::Mode);
    m.catat(putuskan(0) == (false, false), Group::Mode);
    m.catat(putuskan_mask(O_RDONLY) == HASIL_READABLE, Group::Mode);
    m.catat(putuskan_mask(O_WRONLY) == HASIL_WRITABLE, Group::Mode);
    m.catat(putuskan_mask(O_RDONLY | O_WRONLY) == (HASIL_READABLE | HASIL_WRITABLE), Group::Mode);
    m.catat(putuskan_mask(0) == 0, Group::Mode);
    // Bit lain tidak mengubah keputusan.
    m.catat(putuskan_mask(O_RDONLY | (1 << 12)) == HASIL_READABLE, Group::Mode);
    m.catat(putuskan_mask(O_WRONLY | (1 << 6)) == HASIL_WRITABLE, Group::Mode);

    // --- kelompok 1: negatif ------------------------------------------------
    m.catat(O_RDONLY == 1, Group::Neg);
    m.catat(O_WRONLY == 2, Group::Neg);
    m.catat(HASIL_READABLE == 1, Group::Neg);
    m.catat(HASIL_WRITABLE == 2, Group::Neg);
    // Sapuan penuh 0..0x3FFF: keputusan Rust identik dengan rumus C++ historis
    // `(options & O_RDONLY) == O_RDONLY` / `(options & O_WRONLY) == O_WRONLY`.
    let mut sapuan_ok = true;
    let mut diukur = 0u32;
    for options in 0..=0x3FFFi32 {
        let (r, w) = putuskan(options);
        let harap_r = (options & O_RDONLY) == O_RDONLY;
        let harap_w = (options & O_WRONLY) == O_WRONLY;
        diukur += 1;
        if r != harap_r || w != harap_w {
            sapuan_ok = false;
            break;
        }
    }
    m.catat(diukur == 0x4000, Group::Neg);
    m.catat(sapuan_ok, Group::Neg);
    // Konsistensi: 1000 panggilan input sama -> keputusan sama.
    let mut last: Option<(bool, bool)> = None;
    let mut ok = true;
    for _ in 0..1000 {
        let r = putuskan(O_RDONLY | O_WRONLY);
        if last.is_none() {
            last = Some(r);
        } else if last != Some(r) {
            ok = false;
            break;
        }
    }
    m.catat(ok, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosFdModeReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        mode_checks: m.mode_checks,
        mode_passed: m.mode_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_fd_mode_counter(Counter::ProdCalls as u32),
        prod_readable: klatos_fd_mode_counter(Counter::ProdReadable as u32),
        prod_writable: klatos_fd_mode_counter(Counter::ProdWritable as u32),
        prod_neither: klatos_fd_mode_counter(Counter::ProdNeither as u32),
        bit_readable: HASIL_READABLE,
        bit_writable: HASIL_WRITABLE,
        sapuan_checks: diukur,
        status,
    }
}

/// Menjalankan matriks otoritas mode akses deskriptor.
#[no_mangle]
pub extern "C" fn klatos_fd_mode_selftest() -> KlatosFdModeReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(O_RDONLY == 1);
        assert!(O_WRONLY == 2);
        assert!(HASIL_READABLE == 1);
        assert!(HASIL_WRITABLE == 2);
        assert!(REPORT_TAG == 0x4B4C_4154_46444D44);

        // Cermin rumus C++ `(options & O_RDONLY) == O_RDONLY`.
        assert!(putuskan_mask(O_RDONLY) == 1);
        assert!(putuskan_mask(O_WRONLY) == 2);
        assert!(putuskan_mask(O_RDONLY | O_WRONLY) == 3);
        assert!(putuskan_mask(0) == 0);
        assert!(putuskan_mask(O_RDONLY | O_WRONLY | O_CREAT_LIKE) == 3);

        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdReadable as usize == 1);
        assert!(Counter::ProdWritable as usize == 2);
        assert!(Counter::ProdNeither as usize == 3);

        // 8 (tag) + 13*4 (u32) + 4 (status) = 64.
        assert!(core::mem::size_of::<KlatosFdModeReport>() == 64);
        assert!(core::mem::align_of::<KlatosFdModeReport>() == 8);
    };

    /// Nilai bit yang tidak berhubungan dengan mode akses, dipakai untuk
    /// membuktikan bit asing tidak mengubah keputusan.
    const O_CREAT_LIKE: i32 = 1 << 3;
}
