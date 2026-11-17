// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas mode berkas saat pembuatan (P1.63).


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EPERM: i32 = 1;

/// `S_IFMT` — cermin `Jowo/Kontrak/POSIX/sys/stat.h:19`.
pub const S_IFMT: u16 = 0o170000;
/// `S_IFDIR` — cermin `Jowo/Kontrak/POSIX/sys/stat.h:20`.
pub const S_IFDIR: u16 = 0o040000;
/// `S_IFCHR` — cermin `Jowo/Kontrak/POSIX/sys/stat.h:21`.
pub const S_IFCHR: u16 = 0o020000;
/// `S_IFBLK` — cermin `Jowo/Kontrak/POSIX/sys/stat.h:22`.
pub const S_IFBLK: u16 = 0o060000;
/// `S_IFREG` — cermin `Jowo/Kontrak/POSIX/sys/stat.h:23`.
pub const S_IFREG: u16 = 0o100000;
/// `S_IFIFO` — cermin `Jowo/Kontrak/POSIX/sys/stat.h:24`.
pub const S_IFIFO: u16 = 0o010000;
/// `S_IFSOCK` — cermin `Jowo/Kontrak/POSIX/sys/stat.h:26`.
pub const S_IFSOCK: u16 = 0o140000;

/// Tag laporan; cermin `KlatosModBerkasReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4D4F_4446; // "KLATMODF"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    ProdMode = 0,
    ProdMknod = 1,
    ProdMknodTolak = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mode efektif untuk berkas/direktori/node baru. Rumus identik dengan
/// ekspresi C++ `mode & ~umask()`.
#[inline]
pub const fn mode_efektif(mode: u16, umask: u16) -> u16 {
    mode & !umask
}

/// Apakah `mknod` diizinkan untuk kombinasi pemanggil + jenis inode.
/// Mengembalikan `0` (terima) atau `-EPERM (-1)`.
#[inline]
pub const fn jenis_mknod_diizinkan(superuser: bool, mode: u16) -> i32 {
    if superuser {
        return 0;
    }
    let kind = mode & S_IFMT;
    if kind == S_IFREG || kind == S_IFIFO || kind == S_IFSOCK {
        0
    } else {
        -EPERM
    }
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_mode_efektif(mode: u16, umask: u16) -> u16 {
    bump(Counter::ProdMode);
    mode_efektif(mode, umask)
}

#[no_mangle]
pub extern "C" fn klatos_jenis_mknod(superuser: u32, mode: u16) -> i32 {
    bump(Counter::ProdMknod);
    let r = jenis_mknod_diizinkan(superuser != 0, mode);
    if r != 0 {
        bump(Counter::ProdMknodTolak);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_mod_counter(idx: u32) -> u32 {
    if (idx as usize) < N_COUNTER {
        COUNTERS[idx as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_mod_reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Laporan + selftest
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosModBerkasReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_mode: u32,
    pub prod_mknod: u32,
    pub prod_mknod_tolak: u32,
}

const _: [(); 32] = [(); core::mem::size_of::<KlatosModBerkasReport>()];

#[no_mangle]
pub extern "C" fn klatos_mod_selftest() -> KlatosModBerkasReport {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;

    macro_rules! check {
        ($cond:expr) => {
            checks += 1;
            if $cond {
                passed += 1;
            }
        };
    }

    // G1 — konstanta cermin POSIX
    check!(EPERM == 1);
    check!(S_IFMT == 0o170000);
    check!(S_IFDIR == 0o040000);
    check!(S_IFCHR == 0o020000);
    check!(S_IFBLK == 0o060000);
    check!(S_IFREG == 0o100000);
    check!(S_IFIFO == 0o010000);
    check!(S_IFSOCK == 0o140000);
    check!(REPORT_TAG == 0x4B4C_4154_4D4F_4446);

    // G2 — mode_efektif dasar. Rumus = mode & !umask pada 16-bit.
    // (0o600 & !0o077) = 0o600 (bit atas tidak terpengaruh), bukan 0.
    check!(mode_efektif(0o755, 0o022) == 0o755);
    check!(mode_efektif(0o777, 0o777) == 0);
    check!(mode_efektif(0, 0o022) == 0);
    check!(mode_efektif(0o1234, 0) == 0o1234);
    check!(mode_efektif(0o666, 0o022) == 0o644);
    check!(mode_efektif(0o600, 0o077) == 0o600);
    check!(mode_efektif(0o644, 0o022) == 0o644);

    // G3 — mknod superuser
    check!(jenis_mknod_diizinkan(true, S_IFREG | 0o644) == 0);
    check!(jenis_mknod_diizinkan(true, S_IFCHR | 0o644) == 0);
    check!(jenis_mknod_diizinkan(true, S_IFBLK | 0o644) == 0);
    check!(jenis_mknod_diizinkan(true, S_IFDIR | 0o755) == 0);
    check!(jenis_mknod_diizinkan(true, 0) == 0);

    // G4 — mknod non-superuser, jenis yang diizinkan
    check!(jenis_mknod_diizinkan(false, S_IFREG | 0o644) == 0);
    check!(jenis_mknod_diizinkan(false, S_IFIFO | 0o644) == 0);
    check!(jenis_mknod_diizinkan(false, S_IFSOCK | 0o644) == 0);

    // G5 — mknod non-superuser, jenis yang DILARANG
    check!(jenis_mknod_diizinkan(false, S_IFCHR | 0o644) == -EPERM);
    check!(jenis_mknod_diizinkan(false, S_IFBLK | 0o644) == -EPERM);
    check!(jenis_mknod_diizinkan(false, S_IFDIR | 0o755) == -EPERM);
    check!(jenis_mknod_diizinkan(false, 0) == -EPERM);

    // G6 — kombinasi sanitasi + umask (cermin open.cpp:75,81)
    let req: u16 = 0o7777;
    let san: u16 = req & 0o777;
    check!(mode_efektif(san, 0o022) == 0o755);
    let req2: u16 = 0o664;
    let san2: u16 = req2 & 0o777;
    check!(mode_efektif(san2, 0o022) == 0o644);

    // G7 — differential 1000-iter mode_efektif vs rumus C++
    let mut diff_ok: u32 = 1;
    for i in 0u32..1000 {
        let mode: u16 = (i & 0xFFFF) as u16;
        let umask: u16 = ((i.wrapping_mul(31)) & 0xFFFF) as u16;
        let cpp = (mode as u32) & (!(umask as u32)) & 0xFFFF;
        let rust = mode_efektif(mode, umask) as u32;
        if cpp != rust {
            diff_ok = 0;
            break;
        }
    }
    check!(diff_ok == 1);

    // G7b — differential mknod
    let mut diff_mknod_ok: u32 = 1;
    let kinds = [S_IFREG, S_IFCHR, S_IFBLK, S_IFIFO, S_IFSOCK, S_IFDIR];
    for kind in kinds.iter() {
        for perm_step in 0u16..=10 {
            let perm: u16 = perm_step.wrapping_mul(73);
            for su in [false, true].iter() {
                let m = kind | perm;
                let r = jenis_mknod_diizinkan(*su, m);
                let expected: i32 = if *su
                    || *kind == S_IFREG
                    || *kind == S_IFIFO
                    || *kind == S_IFSOCK
                {
                    0
                } else {
                    -EPERM
                };
                if r != expected {
                    diff_mknod_ok = 0;
                }
            }
        }
    }
    check!(diff_mknod_ok == 1);

    // G8 — determinisme
    check!(mode_efektif(0o666, 0o022) == mode_efektif(0o666, 0o022));
    check!(
        jenis_mknod_diizinkan(false, S_IFCHR | 0o644)
            == jenis_mknod_diizinkan(false, S_IFCHR | 0o644)
    );

    KlatosModBerkasReport {
        tag: REPORT_TAG,
        checks,
        passed,
        prod_mode: COUNTERS[Counter::ProdMode as usize].load(Ordering::Relaxed),
        prod_mknod: COUNTERS[Counter::ProdMknod as usize].load(Ordering::Relaxed),
        prod_mknod_tolak: COUNTERS[Counter::ProdMknodTolak as usize].load(Ordering::Relaxed),
    }
}

#[no_mangle]
pub extern "C" fn klatos_mod_report() -> KlatosModBerkasReport {
    KlatosModBerkasReport {
        tag: REPORT_TAG,
        checks: 0,
        passed: 0,
        prod_mode: COUNTERS[Counter::ProdMode as usize].load(Ordering::Relaxed),
        prod_mknod: COUNTERS[Counter::ProdMknod as usize].load(Ordering::Relaxed),
        prod_mknod_tolak: COUNTERS[Counter::ProdMknodTolak as usize].load(Ordering::Relaxed),
    }
}

