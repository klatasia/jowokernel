// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas keputusan `chown()` (P1.65).


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EPERM: i32 = 1;
/// `EROFS` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EROFS: i32 = 30;

/// Sentinel `a_uid != -1` / `a_gid != -1` — cermin `(uid_t)-1` POSIX.
pub const NO_CHANGE: u32 = 0xFFFF_FFFF;

/// `S_ISUID` — cermin `<sys/stat.h>` (`04000`).
pub const SETUID_MASK: u16 = 0o4000;
/// `S_ISGID` — cermin `<sys/stat.h>` (`02000`).
pub const SETGID_MASK: u16 = 0o2000;
/// Bit yang di-strip dari mode saat chown (SUID | SGID).
pub const STRIP_MASK: u16 = SETUID_MASK | SETGID_MASK;

/// Tag laporan; cermin `KlatosModChownReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4348_5754; // "KLATCHWT"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_izin_chown` yang dipanggil dari produksi.
    ProdChown = 0,
    /// Jumlah penolakan `-EPERM`.
    ProdChownEperm = 1,
    /// Jumlah penolakan `-EROFS`.
    ProdChownErofs = 2,
    /// Jumlah `klatos_mode_strip_setids` yang dipanggil dari produksi.
    ProdStrip = 3,
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

/// Keputusan izin `chown()`. Mengembalikan `0` (terima), `-EPERM`, atau
/// `-EROFS`. Argumen `is_superuser`/`is_readonly`/`in_group_a_gid` memakai
/// `u32` agar aman untuk dipanggil dari C (banding `!= 0`).
///
/// Cermin `VirtualFileSystem::chown` (`KlatKernel/Efek/Berkas/VirtualFileSystem.cpp:720-743`):
/// ```cpp
/// if (credentials.euid() != inode.metadata().uid && !credentials.is_superuser())
///     return EPERM;
/// if (a_uid != (uid_t)-1) {
///     if (credentials.euid() != a_uid && !credentials.is_superuser())
///         return EPERM;
/// }
/// if (a_gid != (gid_t)-1) {
///     if (!credentials.in_group(a_gid) && !credentials.is_superuser())
///         return EPERM;
/// }
/// if (custody.is_readonly())
///     return EROFS;
/// ```
#[inline]
pub const fn izin_chown(
    euid: u32,
    uid_inode: u32,
    a_uid: u32,
    a_gid: u32,
    in_group_a_gid: u32,
    is_superuser: u32,
    is_readonly: u32,
) -> i32 {
    if euid != uid_inode && is_superuser == 0 {
        return -EPERM;
    }
    if a_uid != NO_CHANGE {
        if euid != a_uid && is_superuser == 0 {
            return -EPERM;
        }
    }
    if a_gid != NO_CHANGE {
        if in_group_a_gid == 0 && is_superuser == 0 {
            return -EPERM;
        }
    }
    if is_readonly != 0 {
        return -EROFS;
    }
    0
}

/// Mode efektif `chown()` — ketika SUID atau SGID terpasang pada inode,
/// bit `04000 | 02000` di-strip dari mode. Kalau `has_setid == 0`, mode
/// asli dikembalikan tanpa perubahan.
///
/// Cermin `VirtualFileSystem::chown` (`KlatKernel/Efek/Berkas/VirtualFileSystem.cpp:747-750`):
/// ```cpp
/// if (metadata.is_setuid() || metadata.is_setgid()) {
///     TRY(inode.chmod(metadata.mode & ~(04000 | 02000)));
/// }
/// ```
#[inline]
pub const fn mode_strip_setids(current_mode: u16, has_setid: u32) -> u16 {
    if has_setid == 0 {
        current_mode
    } else {
        current_mode & !STRIP_MASK
    }
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_izin_chown(
    euid: u32,
    uid_inode: u32,
    a_uid: u32,
    a_gid: u32,
    in_group_a_gid: u32,
    is_superuser: u32,
    is_readonly: u32,
) -> i32 {
    bump(Counter::ProdChown);
    let r = izin_chown(euid, uid_inode, a_uid, a_gid, in_group_a_gid, is_superuser, is_readonly);
    if r == -EPERM {
        bump(Counter::ProdChownEperm);
    } else if r == -EROFS {
        bump(Counter::ProdChownErofs);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_mode_strip_setids(current_mode: u16, has_setid: u32) -> u16 {
    bump(Counter::ProdStrip);
    mode_strip_setids(current_mode, has_setid)
}

#[no_mangle]
pub extern "C" fn klatos_modchown_counter(idx: u32) -> u32 {
    if (idx as usize) < N_COUNTER {
        COUNTERS[idx as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_modchown_reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Laporan + selftest
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosModChownReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_chown: u32,
    pub prod_chown_eperm: u32,
    pub prod_chown_erofs: u32,
    pub prod_strip: u32,
}

const _: [(); 32] = [(); core::mem::size_of::<KlatosModChownReport>()];

#[no_mangle]
pub extern "C" fn klatos_modchown_selftest() -> KlatosModChownReport {
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
    check!(NO_CHANGE == 0xFFFF_FFFF);
    check!(SETUID_MASK == 0o4000);
    check!(SETGID_MASK == 0o2000);
    check!(STRIP_MASK == 0o6000);
    check!(REPORT_TAG == 0x4B4C_4154_4348_5754);

    // G2 — izin: pemilik boleh walau a_uid = NO_CHANGE
    check!(izin_chown(1000, 1000, NO_CHANGE, NO_CHANGE, 0, 0, 0) == 0);
    check!(izin_chown(0, 0, NO_CHANGE, NO_CHANGE, 0, 0, 0) == 0);
    // G2b — izin: superuser boleh walau bukan pemilik
    check!(izin_chown(0, 1000, NO_CHANGE, NO_CHANGE, 0, 1, 0) == 0);
    // G2c — izin: bukan pemilik + bukan superuser ⇒ EPERM
    check!(izin_chown(1000, 0, NO_CHANGE, NO_CHANGE, 0, 0, 0) == -EPERM);
    check!(izin_chown(999, 1000, NO_CHANGE, NO_CHANGE, 0, 0, 0) == -EPERM);

    // G3 — a_uid bukan NO_CHANGE: pemanggil == a_uid → OK
    check!(izin_chown(1000, 1000, 1000, NO_CHANGE, 0, 0, 0) == 0);
    check!(izin_chown(1000, 1000, 1234, NO_CHANGE, 0, 0, 0) == -EPERM);
    check!(izin_chown(500, 500, 500, NO_CHANGE, 0, 0, 0) == 0);
    // G3b — superuser + a_uid != euid → OK
    check!(izin_chown(0, 1000, 1234, NO_CHANGE, 0, 1, 0) == 0);
    check!(izin_chown(1000, 0, 1234, NO_CHANGE, 0, 1, 0) == 0);

    // G4 — a_gid bukan NO_CHANGE: anggota grup → OK; bukan anggota + bukan superuser → EPERM
    check!(izin_chown(1000, 1000, NO_CHANGE, 100, 1, 1, 0) == 0);
    check!(izin_chown(1000, 1000, NO_CHANGE, 100, 0, 0, 0) == -EPERM);
    // G4b — superuser boleh walau bukan anggota
    check!(izin_chown(0, 0, NO_CHANGE, 100, 0, 1, 0) == 0);

    // G5 — readonly ⇒ EROFS (bahkan untuk pemilik/superuser)
    check!(izin_chown(1000, 1000, NO_CHANGE, NO_CHANGE, 0, 0, 1) == -EROFS);
    check!(izin_chown(0, 0, NO_CHANGE, NO_CHANGE, 0, 1, 1) == -EROFS);
    // G5b — urutan cek = EPERM dulu (a_uid), baru a_gid, baru EROFS
    check!(izin_chown(1000, 0, 1234, 100, 0, 0, 1) == -EPERM);

    // G6 — mode_strip_setids dasar
    // Tidak ada setid → mode asli
    check!(mode_strip_setids(0o100644, 0) == 0o100644);
    check!(mode_strip_setids(0o040755, 0) == 0o040755);
    check!(mode_strip_setids(0, 0) == 0);
    // SUID di-strip
    check!(mode_strip_setids(0o104755, 1) == 0o100755);
    // SGID di-strip
    check!(mode_strip_setids(0o102755, 1) == 0o100755);
    // SUID+SGID keduanya di-strip
    check!(mode_strip_setids(0o106755, 1) == 0o100755);
    // SUID+SGID pada jenis direktori
    check!(mode_strip_setids(0o040755, 0) == 0o040755);
    check!(mode_strip_setids(0o040755, 1) == 0o040755); // tidak ada SUID/SGID, hasil = asli
    check!(mode_strip_setids(0o046755, 1) == 0o040755); // SUID+SGID di-strip (direktori)

    // G7 — diferensial 4096-iter izin_chown vs oracle C++
    let mut diff_ok: u32 = 1;
    for i in 0u32..4096 {
        let eu = i & 0xFF;
        let ui = (i >> 3) & 0xFF;
        let au = if (i & 1) == 0 { NO_CHANGE } else { (i >> 4) & 0xFF };
        let ag = if (i & 2) == 0 { NO_CHANGE } else { (i >> 5) & 0xFF };
        let ingrp = (i >> 6) & 1;
        let su = (i >> 7) & 1;
        let ro = (i >> 8) & 1;
        let rust = izin_chown(eu, ui, au, ag, ingrp, su, ro);
        let expected: i32 = if eu != ui && su == 0 {
            -EPERM
        } else if au != NO_CHANGE && eu != au && su == 0 {
            -EPERM
        } else if ag != NO_CHANGE && ingrp == 0 && su == 0 {
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

    // G8 — diferensial 4096-iter mode_strip_setids vs rumus C++
    let mut diff_s_ok: u32 = 1;
    for i in 0u32..4096 {
        let cur = (i & 0xFFFF) as u16;
        let has_setid = (i >> 13) & 1;
        let rust = mode_strip_setids(cur, has_setid);
        let expected = if has_setid == 0 {
            cur as i32
        } else {
            (cur as i32) & !((STRIP_MASK) as i32)
        };
        if (rust as i32) != (expected & 0xFFFF) {
            diff_s_ok = 0;
        }
    }
    check!(diff_s_ok == 1);

    // G9 — determinisme + urutan parameter
    check!(izin_chown(1000, 1000, NO_CHANGE, NO_CHANGE, 0, 0, 0)
        == izin_chown(1000, 1000, NO_CHANGE, NO_CHANGE, 0, 0, 0));
    check!(mode_strip_setids(0o106755, 1) == mode_strip_setids(0o106755, 1));
    // G9b — parameter order: euid vs uid_inode
    check!(izin_chown(1000, 0, NO_CHANGE, NO_CHANGE, 0, 0, 0) == -EPERM);
    check!(izin_chown(0, 0, NO_CHANGE, NO_CHANGE, 0, 0, 0) == 0);

    KlatosModChownReport {
        tag: REPORT_TAG,
        checks,
        passed,
        prod_chown: COUNTERS[Counter::ProdChown as usize].load(Ordering::Relaxed),
        prod_chown_eperm: COUNTERS[Counter::ProdChownEperm as usize].load(Ordering::Relaxed),
        prod_chown_erofs: COUNTERS[Counter::ProdChownErofs as usize].load(Ordering::Relaxed),
        prod_strip: COUNTERS[Counter::ProdStrip as usize].load(Ordering::Relaxed),
    }
}

#[no_mangle]
pub extern "C" fn klatos_modchown_report() -> KlatosModChownReport {
    KlatosModChownReport {
        tag: REPORT_TAG,
        checks: 0,
        passed: 0,
        prod_chown: COUNTERS[Counter::ProdChown as usize].load(Ordering::Relaxed),
        prod_chown_eperm: COUNTERS[Counter::ProdChownEperm as usize].load(Ordering::Relaxed),
        prod_chown_erofs: COUNTERS[Counter::ProdChownErofs as usize].load(Ordering::Relaxed),
        prod_strip: COUNTERS[Counter::ProdStrip as usize].load(Ordering::Relaxed),
    }
}