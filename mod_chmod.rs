// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas keputusan `chmod()` (P1.64).


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EPERM: i32 = 1;
/// `EROFS` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EROFS: i32 = 30;

/// Topeng bit izin (12 bit rendah `07777`); cermin `VirtualFileSystem.cpp:611`.
pub const MODE_MASK: u16 = 0o7777;

/// Tag laporan; cermin `KlatosModChmodReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4348_4D44; // "KLATCHMD"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_izin_chmod` yang dipanggil dari produksi.
    ProdChmod = 0,
    /// Jumlah penolakan `-EPERM` (bukan pemilik + bukan superuser).
    ProdChmodEperm = 1,
    /// Jumlah penolakan `-EROFS` (deskriptor read-only).
    ProdChmodErofs = 2,
    /// Jumlah `klatos_mode_efektif_chmod` yang dipanggil dari produksi.
    ProdMmask = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Keputusan izin `chmod()`. Mengembalikan `0` (terima), `-EPERM`, atau
/// `-EROFS`. Argumen `is_superuser`/`is_readonly` memakai `u32` agar aman
/// untuk dipanggil dari C (banding `!= 0`).
///
/// Cermin `VirtualFileSystem::chmod` (`KlatKernel/Efek/Berkas/VirtualFileSystem.cpp:601-608`):
/// ```cpp
/// if (credentials.euid() != inode.metadata().uid && !credentials.is_superuser())
///     return EPERM;
/// if (custody.is_readonly())
///     return EROFS;
/// ```
#[inline]
pub const fn izin_chmod(
    uid_caller: u32,
    uid_inode: u32,
    is_superuser: u32,
    is_readonly: u32,
) -> i32 {
    if uid_caller != uid_inode && is_superuser == 0 {
        return -EPERM;
    }
    if is_readonly != 0 {
        return -EROFS;
    }
    0
}

/// Mode efektif `chmod()` — bit izin baru menggantikan bit izin lama,
/// sedangkan bit jenis (upper 4 bit) tetap. Rumus identik dengan ekspresi
/// C++ `(inode.mode() & ~07777u) | (mode & 07777u)`.
#[inline]
pub const fn mode_efektif_chmod(current_mode: u16, new_mode: u16) -> u16 {
    (current_mode & !MODE_MASK) | (new_mode & MODE_MASK)
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_izin_chmod(
    uid_caller: u32,
    uid_inode: u32,
    is_superuser: u32,
    is_readonly: u32,
) -> i32 {
    bump(Counter::ProdChmod);
    let r = izin_chmod(uid_caller, uid_inode, is_superuser, is_readonly);
    if r == -EPERM {
        bump(Counter::ProdChmodEperm);
    } else if r == -EROFS {
        bump(Counter::ProdChmodErofs);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_mode_efektif_chmod(current_mode: u16, new_mode: u16) -> u16 {
    bump(Counter::ProdMmask);
    mode_efektif_chmod(current_mode, new_mode)
}

#[no_mangle]
pub extern "C" fn klatos_modchmod_counter(idx: u32) -> u32 {
    if (idx as usize) < N_COUNTER {
        COUNTERS[idx as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_modchmod_reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Laporan + selftest
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosModChmodReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_chmod: u32,
    pub prod_chmod_eperm: u32,
    pub prod_chmod_erofs: u32,
    pub prod_mmask: u32,
}

const _: [(); 32] = [(); core::mem::size_of::<KlatosModChmodReport>()];

#[no_mangle]
pub extern "C" fn klatos_modchmod_selftest() -> KlatosModChmodReport {
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
    check!(EROFS == 30);
    check!(MODE_MASK == 0o7777);
    check!(REPORT_TAG == 0x4B4C_4154_4348_4D44);

    // G2 — izin: pemilik selalu boleh (bukan superuser, bukan readonly)
    check!(izin_chmod(1000, 1000, 0, 0) == 0);
    check!(izin_chmod(0, 0, 0, 0) == 0);
    // G2b — izin: superuser boleh walau bukan pemilik
    check!(izin_chmod(0, 1000, 1, 0) == 0);
    check!(izin_chmod(1000, 0, 1, 0) == 0);
    // G2c — izin: bukan pemilik + bukan superuser ⇒ EPERM
    check!(izin_chmod(1000, 0, 0, 0) == -EPERM);
    check!(izin_chmod(999, 1000, 0, 0) == -EPERM);
    // G2d — izin: readonly ⇒ EROFS (bahkan untuk pemilik/superuser)
    check!(izin_chmod(1000, 1000, 0, 1) == -EROFS);
    check!(izin_chmod(1000, 0, 1, 1) == -EROFS);
    // G2e — izin: urutan cek = EPERM dulu, baru EROFS
    check!(izin_chmod(1000, 0, 0, 1) == -EPERM);

    // G3 — mode_efektif_chmod dasar
    // Bit jenis (atas) dipertahankan; bit izin (bawah) diganti.
    check!(mode_efektif_chmod(0o100644, 0o755) == 0o100755);
    check!(mode_efektif_chmod(0o040755, 0o700) == 0o040700);
    check!(mode_efektif_chmod(0o120777, 0o000) == 0o120000);
    // G3b — kalau new_mode sudah melewati MODE_MASK, bit setuid/setgid/sticky
    //       di atas MODE_MASK ikut berubah — cermin `mode & 07777u`.
    //       Catatan: SUID/SGID/sticky adalah bagian dari MODE_MASK (0o7777
    //       = 12 bit), jadi `(new_mode & 07777u)` selalu men-AND ke 12 bit
    //       rendah saja. Jenis (atas) tetap.
    check!(mode_efektif_chmod(0o100600, 0o7777) == 0o107777);
    check!(mode_efektif_chmod(0o100600, 0) == 0o100000);
    // G3c — new_mode == 0 ⇒ bit izin menjadi 0, jenis tetap.
    check!(mode_efektif_chmod(0o40755, 0) == 0o40000);
    // G3d — current_mode == 0 (tak ada jenis) ⇒ hasil = new_mode & MODE_MASK.
    check!(mode_efektif_chmod(0, 0o644) == 0o644);

    // G4 — kombinasi: izin lulus (0) + mode_mask valid
    let r_izin = izin_chmod(1000, 1000, 0, 0);
    let r_mode = mode_efektif_chmod(0o100644, 0o755);
    check!(r_izin == 0 && r_mode == 0o100755);

    // G5 — diferensial 4096-iter izin_chmod vs rumus C++
    let mut diff_ok: u32 = 1;
    for i in 0u32..4096 {
        let uc = i & 0xFF;
        let ui = (i >> 3) & 0xFF;
        let su = (i >> 1) & 1;
        let ro = (i >> 2) & 1;
        let rust = izin_chmod(uc, ui, su, ro);
        // Oracle: kalau euid != uid && !superuser -> -1; else if readonly -> -30; else 0.
        let expected: i32 = if uc != ui && su == 0 {
            -EPERM
        } else if ro != 0 {
            -EROFS
        } else {
            0
        };
        if rust != expected {
            diff_ok = 0;
        }
    }
    check!(diff_ok == 1);

    // G6 — diferensial 4096-iter mode_efektif_chmod vs rumus C++
    let mut diff_m_ok: u32 = 1;
    for i in 0u32..4096 {
        let cur = (i & 0xFFFF) as u16;
        let new = ((i.wrapping_mul(31)) & 0xFFFF) as u16;
        let rust = mode_efektif_chmod(cur, new);
        let cpp = ((cur as i32 & !((MODE_MASK) as i32))
            | (new as i32 & (MODE_MASK as i32)))
            & 0xFFFF;
        if (rust as i32) != cpp {
            diff_m_ok = 0;
        }
    }
    check!(diff_m_ok == 1);

    // G7 — determinisme
    check!(izin_chmod(1000, 1000, 0, 0) == izin_chmod(1000, 1000, 0, 0));
    check!(mode_efektif_chmod(0o100644, 0o755) == mode_efektif_chmod(0o100644, 0o755));
    // G7b — urutan parameter tidak ambigu
    check!(izin_chmod(1000, 0, 0, 0) == -EPERM);
    check!(izin_chmod(0, 0, 0, 0) == 0);

    KlatosModChmodReport {
        tag: REPORT_TAG,
        checks,
        passed,
        prod_chmod: COUNTERS[Counter::ProdChmod as usize].load(Ordering::Relaxed),
        prod_chmod_eperm: COUNTERS[Counter::ProdChmodEperm as usize].load(Ordering::Relaxed),
        prod_chmod_erofs: COUNTERS[Counter::ProdChmodErofs as usize].load(Ordering::Relaxed),
        prod_mmask: COUNTERS[Counter::ProdMmask as usize].load(Ordering::Relaxed),
    }
}

#[no_mangle]
pub extern "C" fn klatos_modchmod_report() -> KlatosModChmodReport {
    KlatosModChmodReport {
        tag: REPORT_TAG,
        checks: 0,
        passed: 0,
        prod_chmod: COUNTERS[Counter::ProdChmod as usize].load(Ordering::Relaxed),
        prod_chmod_eperm: COUNTERS[Counter::ProdChmodEperm as usize].load(Ordering::Relaxed),
        prod_chmod_erofs: COUNTERS[Counter::ProdChmodErofs as usize].load(Ordering::Relaxed),
        prod_mmask: COUNTERS[Counter::ProdMmask as usize].load(Ordering::Relaxed),
    }
}