// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas keputusan pledge mode-akses `open()` Jowo (P1.10).

//! ([`run_matrix`]).

use core::sync::atomic::{AtomicU32, Ordering};

/// Cermin `<fcntl.h>` `O_RDONLY`.
pub const O_RDONLY: i32 = 1 << 0;

/// Cermin `<fcntl.h>` `O_WRONLY`.
pub const O_WRONLY: i32 = 1 << 1;

/// Cermin `<fcntl.h>` `O_CREAT`.
pub const O_CREAT: i32 = 1 << 3;

/// Bit mask: pemanggil WAJIB memiliki pledge `wpath`.
pub const KSM_IZIN_WPATH: u32 = 1 << 0;

/// Bit mask: pemanggil WAJIB memiliki pledge `rpath`.
pub const KSM_IZIN_RPATH: u32 = 1 << 1;

/// Bit mask: pemanggil WAJIB memiliki pledge `cpath`.
pub const KSM_IZIN_CPATH: u32 = 1 << 2;

/// Tag laporan; cermin `KlatosKsmOpenReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4F504C47; // "KLATOPLG"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_ksm_open_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan yang mewajibkan `wpath`.
    ProdWpath = 1,
    /// Jumlah keputusan yang mewajibkan `rpath`.
    ProdRpath = 2,
    /// Jumlah keputusan yang mewajibkan `cpath`.
    ProdCpath = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mengembalikan mask pledge yang diwajibkan untuk `options`.
///
/// Urutan dan isi keputusan SAMA PERSIS dengan C++ historis
/// (`Jowo/Panggilan/open.cpp:45-53`), termasuk sifat `else if` antara
/// `O_WRONLY` dan `O_RDONLY`.
///
/// `# Safety`: tidak ada — fungsi ini murni.
#[inline]
pub const fn putuskan(options: i32) -> u32 {
    let mut mask = 0u32;

    if options & O_WRONLY != 0 {
        mask |= KSM_IZIN_WPATH;
    } else if options & O_RDONLY != 0 {
        mask |= KSM_IZIN_RPATH;
    }

    if options & O_CREAT != 0 {
        mask |= KSM_IZIN_CPATH;
    }

    mask
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan mask pledge (`1=wpath`, `2=rpath`, `4=cpath`). `0` berarti
/// tidak ada pledge yang diwajibkan.
///
/// Menaikkan `prod_calls` dan penghitung per-jenis sesuai keputusan.
#[no_mangle]
pub extern "C" fn klatos_ksm_open_decide(options: i32) -> u32 {
    let mask = putuskan(options);
    bump(Counter::ProdCalls);
    if mask & KSM_IZIN_WPATH != 0 {
        bump(Counter::ProdWpath);
    }
    if mask & KSM_IZIN_RPATH != 0 {
        bump(Counter::ProdRpath);
    }
    if mask & KSM_IZIN_CPATH != 0 {
        bump(Counter::ProdCpath);
    }
    mask
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_ksm_open_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_ksm_open_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosKsmOpenReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosKsmOpenReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub mode_checks: u32,
    pub mode_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_wpath: u32,
    pub prod_rpath: u32,
    pub prod_cpath: u32,
    pub mask_wpath: u32,
    pub mask_rpath: u32,
    pub mask_cpath: u32,
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

/// Matriks otoritas pledge `open`. Input SINTETIS — tidak memanggil
/// [`klatos_ksm_open_decide`] sehingga tidak menggeser penghitung produksi.
fn run_matrix() -> KlatosKsmOpenReport {
    let mut m = Matrix::new();

    // --- kelompok 0: pemetaan mode-akses -----------------------------------
    // O_RDONLY -> rpath saja.
    m.catat(putuskan(O_RDONLY) == KSM_IZIN_RPATH, Group::Mode);
    // O_WRONLY -> wpath saja.
    m.catat(putuskan(O_WRONLY) == KSM_IZIN_WPATH, Group::Mode);
    // O_RDWR (kedua bit) -> wpath saja (else-if historis).
    m.catat(putuskan(O_RDONLY | O_WRONLY) == KSM_IZIN_WPATH, Group::Mode);
    // 0 -> tidak ada pledge.
    m.catat(putuskan(0) == 0, Group::Mode);
    // O_CREAT saja -> cpath saja.
    m.catat(putuskan(O_CREAT) == KSM_IZIN_CPATH, Group::Mode);
    // O_RDONLY|O_CREAT -> rpath + cpath.
    m.catat(putuskan(O_RDONLY | O_CREAT) == (KSM_IZIN_RPATH | KSM_IZIN_CPATH), Group::Mode);
    // O_WRONLY|O_CREAT -> wpath + cpath.
    m.catat(putuskan(O_WRONLY | O_CREAT) == (KSM_IZIN_WPATH | KSM_IZIN_CPATH), Group::Mode);
    // O_RDWR|O_CREAT -> wpath + cpath (bukan rpath).
    m.catat(
        putuskan(O_RDONLY | O_WRONLY | O_CREAT) == (KSM_IZIN_WPATH | KSM_IZIN_CPATH),
        Group::Mode,
    );
    // Bit tak dikenal tidak menambah apa pun.
    m.catat(putuskan(0x4000) == 0, Group::Mode);
    // O_CREAT bersama bit tak dikenal tetap cpath.
    m.catat(putuskan(0x4000 | O_CREAT) == KSM_IZIN_CPATH, Group::Mode);

    // --- kelompok 1: negatif ------------------------------------------------
    m.catat(O_RDONLY == 1, Group::Neg);
    m.catat(O_WRONLY == 2, Group::Neg);
    m.catat(O_CREAT == 8, Group::Neg);
    m.catat(KSM_IZIN_WPATH == 1, Group::Neg);
    m.catat(KSM_IZIN_RPATH == 2, Group::Neg);
    m.catat(KSM_IZIN_CPATH == 4, Group::Neg);
    // wpath dan rpath TIDAK PERNAH bersama (sifat else-if).
    let mut never_both = true;
    for options in 0..=0x3FFFi32 {
        let mask = putuskan(options);
        if mask & KSM_IZIN_WPATH != 0 && mask & KSM_IZIN_RPATH != 0 {
            never_both = false;
            break;
        }
    }
    m.catat(never_both, Group::Neg);
    // Sapuan penuh: cpath tepat bila O_CREAT.
    let mut cpath_tepat = true;
    for options in 0..=0x3FFFi32 {
        let mask = putuskan(options);
        let punya_cpath = mask & KSM_IZIN_CPATH != 0;
        if punya_cpath != (options & O_CREAT != 0) {
            cpath_tepat = false;
            break;
        }
    }
    m.catat(cpath_tepat, Group::Neg);
    // Sapuan penuh: wpath tepat bila O_WRONLY; rpath tepat bila O_RDONLY dan
    // bukan O_WRONLY.
    let mut mode_tepat = true;
    for options in 0..=0x3FFFi32 {
        let mask = putuskan(options);
        let wpath_harap = options & O_WRONLY != 0;
        let rpath_harap = options & O_WRONLY == 0 && options & O_RDONLY != 0;
        if (mask & KSM_IZIN_WPATH != 0) != wpath_harap || (mask & KSM_IZIN_RPATH != 0) != rpath_harap {
            mode_tepat = false;
            break;
        }
    }
    m.catat(mode_tepat, Group::Neg);
    // Konsistensi: 1000 panggilan input sama -> keputusan sama.
    let mut last: Option<u32> = None;
    let mut ok = true;
    for _ in 0..1000 {
        let r = putuskan(O_RDONLY | O_CREAT);
        if last.is_none() {
            last = Some(r);
        } else if last != Some(r) {
            ok = false;
            break;
        }
    }
    m.catat(ok, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosKsmOpenReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        mode_checks: m.mode_checks,
        mode_passed: m.mode_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_ksm_open_counter(Counter::ProdCalls as u32),
        prod_wpath: klatos_ksm_open_counter(Counter::ProdWpath as u32),
        prod_rpath: klatos_ksm_open_counter(Counter::ProdRpath as u32),
        prod_cpath: klatos_ksm_open_counter(Counter::ProdCpath as u32),
        mask_wpath: KSM_IZIN_WPATH,
        mask_rpath: KSM_IZIN_RPATH,
        mask_cpath: KSM_IZIN_CPATH,
        status,
    }
}

/// Menjalankan matriks otoritas pledge `open`.
#[no_mangle]
pub extern "C" fn klatos_ksm_open_selftest() -> KlatosKsmOpenReport {
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
        assert!(O_CREAT == 8);
        assert!(KSM_IZIN_WPATH == 1);
        assert!(KSM_IZIN_RPATH == 2);
        assert!(KSM_IZIN_CPATH == 4);
        assert!(REPORT_TAG == 0x4B4C_4154_4F504C47);

        assert!(putuskan(O_RDONLY) == KSM_IZIN_RPATH);
        assert!(putuskan(O_WRONLY) == KSM_IZIN_WPATH);
        assert!(putuskan(O_RDONLY | O_WRONLY) == KSM_IZIN_WPATH);
        assert!(putuskan(0) == 0);
        assert!(putuskan(O_CREAT) == KSM_IZIN_CPATH);
        assert!(putuskan(O_RDONLY | O_CREAT) == KSM_IZIN_RPATH | KSM_IZIN_CPATH);

        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdWpath as usize == 1);
        assert!(Counter::ProdRpath as usize == 2);
        assert!(Counter::ProdCpath as usize == 3);

        // 8 (tag) + 13*4 (u32) + 4 (status) = 64.
        assert!(core::mem::size_of::<KlatosKsmOpenReport>() == 64);
        assert!(core::mem::align_of::<KlatosKsmOpenReport>() == 8);
    };
}
