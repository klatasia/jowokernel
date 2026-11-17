// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas mutasi entri direktori (P1.66).


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EPERM: i32 = 1;
/// `EACCES` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EACCES: i32 = 13;
/// `EROFS` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EROFS: i32 = 30;

/// `S_IXGRP` — cermin `<sys/stat.h>` (`0010`).
pub const S_IXGRP: u16 = 0o0010;

/// Tag laporan; cermin `KlatosModHapusReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4841_5053; // "KLATHAPS"

/// Penghitung untuk audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_izin_hapus` (unlink + rmdir).
    ProdHapus = 0,
    /// Jumlah penolakan `-EACCES` dari `klatos_izin_hapus`.
    ProdHapusEacces = 1,
    /// Jumlah penolakan `-EROFS` dari `klatos_hapus_erofs`.
    ProdHapusErofs = 2,
    /// Jumlah `klatos_izin_rename` + `klatos_izin_rename_tujuan`.
    ProdRename = 3,
    /// Jumlah penolakan `-EACCES` dari jalur rename.
    ProdRenameEacces = 4,
    /// Jumlah penolakan `-EROFS` dari `klatos_rename_erofs`.
    ProdRenameErofs = 5,
    /// Jumlah `klatos_izin_link`.
    ProdLink = 6,
    /// Jumlah penolakan `-EPERM` dari `klatos_izin_link`.
    ProdLinkEperm = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Izin hapus entri (`unlink` dan `rmdir` memakai syarat yang identik).
/// `0` (terima) atau `-EACCES`.
///
/// Cermin `VirtualFileSystem::unlink` (`VirtualFileSystem.cpp:841-848`) dan
/// `VirtualFileSystem::rmdir` (`:920-936`):
/// ```cpp
/// if (!parent_inode.metadata().may_write(credentials))
///     return EACCES;
/// if (parent_inode.metadata().is_sticky()) {
///     if (!credentials.is_superuser()
///         && parent_inode.metadata().uid != credentials.euid()
///         && inode.metadata().uid != credentials.euid())
///         return EACCES;
/// }
/// ```
#[inline]
pub const fn izin_hapus(
    parent_may_write: u32,
    parent_is_sticky: u32,
    parent_uid: u32,
    inode_uid: u32,
    euid: u32,
    is_superuser: u32,
) -> i32 {
    if parent_may_write == 0 {
        return -EACCES;
    }
    if parent_is_sticky != 0
        && is_superuser == 0
        && parent_uid != euid
        && inode_uid != euid
    {
        return -EACCES;
    }
    0
}

/// Cek read-only untuk jalur hapus. Dipanggil C++ **setelah** cek
/// `ENOTEMPTY` pada `rmdir`, persis seperti sebelumnya. `0` atau `-EROFS`.
#[inline]
pub const fn hapus_erofs(is_readonly: u32) -> i32 {
    if is_readonly != 0 {
        return -EROFS;
    }
    0
}

/// Izin rename — bagian izin induk (tulis + sticky lama). `0` atau `-EACCES`.
///
/// Cermin `VirtualFileSystem::rename` (`VirtualFileSystem.cpp:672-681`):
/// ```cpp
/// if (!new_parent_inode.metadata().may_write(credentials)) return EACCES;
/// if (!old_parent_inode.metadata().may_write(credentials)) return EACCES;
/// if (old_parent_inode.metadata().is_sticky()) {
///     if (!credentials.is_superuser()
///         && old_parent_inode.metadata().uid != credentials.euid()
///         && old_inode.metadata().uid != credentials.euid())
///         return EACCES;
/// }
/// ```
#[inline]
pub const fn izin_rename(
    old_may_write: u32,
    new_may_write: u32,
    old_is_sticky: u32,
    old_parent_uid: u32,
    old_inode_uid: u32,
    euid: u32,
    is_superuser: u32,
) -> i32 {
    if new_may_write == 0 {
        return -EACCES;
    }
    if old_may_write == 0 {
        return -EACCES;
    }
    if old_is_sticky != 0 && is_superuser == 0 && old_parent_uid != euid && old_inode_uid != euid {
        return -EACCES;
    }
    0
}

/// Cek read-only jalur rename (salah satu sisi). `0` atau `-EROFS`.
///
/// Cermin `VirtualFileSystem::rename` (`VirtualFileSystem.cpp:683-684`):
/// ```cpp
/// if (old_parent_custody->is_readonly() || new_parent_custody->is_readonly())
///     return EROFS;
/// ```
#[inline]
pub const fn rename_erofs(old_readonly: u32, new_readonly: u32) -> i32 {
    if old_readonly != 0 || new_readonly != 0 {
        return -EROFS;
    }
    0
}

/// Izin rename — sticky induk TUJUAN, hanya ketika tujuan sudah ada.
/// `0` atau `-EACCES`.
///
/// Cermin `VirtualFileSystem::rename` (`VirtualFileSystem.cpp:707-710`):
/// ```cpp
/// if (new_parent_inode.metadata().is_sticky()) {
///     if (!credentials.is_superuser()
///         && new_inode.metadata().uid != credentials.euid())
///         return EACCES;
/// }
/// ```
#[inline]
pub const fn izin_rename_tujuan(
    new_is_sticky: u32,
    new_inode_uid: u32,
    euid: u32,
    is_superuser: u32,
) -> i32 {
    if new_is_sticky != 0 && is_superuser == 0 && new_inode_uid != euid {
        return -EACCES;
    }
    0
}

/// Bolehkah hard-link ke inode ini? `1` (boleh) / `0` (tidak).
///
/// Cermin `hard_link_allowed` (`VirtualFileSystem.cpp:775-790`):
/// ```cpp
/// if (credentials.euid() == metadata.uid) return true;
/// if (metadata.is_regular_file()
///     && !metadata.is_setuid()
///     && !(metadata.is_setgid() && metadata.mode & S_IXGRP)
///     && metadata.may_write(credentials)) return true;
/// return false;
/// ```
#[inline]
pub const fn hard_link_diizinkan(
    euid: u32,
    inode_uid: u32,
    is_regular: u32,
    is_setuid: u32,
    is_setgid: u32,
    has_ixgrp: u32,
    may_write: u32,
) -> u32 {
    if euid == inode_uid {
        return 1;
    }
    if is_regular != 0
        && is_setuid == 0
        && (is_setgid == 0 || has_ixgrp == 0)
        && may_write != 0
    {
        return 1;
    }
    0
}

/// Izin `link()`. `0`, `-EACCES`, `-EPERM`, atau `-EROFS` — urutan persis
/// seperti C++ (`EACCES` tulis → `EPERM` direktori → `EROFS` →
/// `EPERM` hard-link).
///
/// Cermin `VirtualFileSystem::link` (`VirtualFileSystem.cpp:812-822`):
/// ```cpp
/// if (!parent_inode.metadata().may_write(credentials)) return EACCES;
/// if (old_inode.is_directory()) return EPERM;
/// if (parent_custody->is_readonly()) return EROFS;
/// if (!hard_link_allowed(credentials, old_inode)) return EPERM;
/// ```
#[inline]
pub const fn izin_link(
    parent_may_write: u32,
    is_readonly: u32,
    is_dir_source: u32,
    hard_link_allowed: u32,
) -> i32 {
    if parent_may_write == 0 {
        return -EACCES;
    }
    if is_dir_source != 0 {
        return -EPERM;
    }
    if is_readonly != 0 {
        return -EROFS;
    }
    if hard_link_allowed == 0 {
        return -EPERM;
    }
    0
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_izin_hapus(
    parent_may_write: u32,
    parent_is_sticky: u32,
    parent_uid: u32,
    inode_uid: u32,
    euid: u32,
    is_superuser: u32,
) -> i32 {
    bump(Counter::ProdHapus);
    let r = izin_hapus(
        parent_may_write,
        parent_is_sticky,
        parent_uid,
        inode_uid,
        euid,
        is_superuser,
    );
    if r == -EACCES {
        bump(Counter::ProdHapusEacces);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_hapus_erofs(is_readonly: u32) -> i32 {
    let r = hapus_erofs(is_readonly);
    if r == -EROFS {
        bump(Counter::ProdHapusErofs);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_izin_rename(
    old_may_write: u32,
    new_may_write: u32,
    old_is_sticky: u32,
    old_parent_uid: u32,
    old_inode_uid: u32,
    euid: u32,
    is_superuser: u32,
) -> i32 {
    bump(Counter::ProdRename);
    let r = izin_rename(
        old_may_write,
        new_may_write,
        old_is_sticky,
        old_parent_uid,
        old_inode_uid,
        euid,
        is_superuser,
    );
    if r == -EACCES {
        bump(Counter::ProdRenameEacces);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_izin_rename_tujuan(
    new_is_sticky: u32,
    new_inode_uid: u32,
    euid: u32,
    is_superuser: u32,
) -> i32 {
    bump(Counter::ProdRename);
    let r = izin_rename_tujuan(new_is_sticky, new_inode_uid, euid, is_superuser);
    if r == -EACCES {
        bump(Counter::ProdRenameEacces);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_rename_erofs(old_readonly: u32, new_readonly: u32) -> i32 {
    let r = rename_erofs(old_readonly, new_readonly);
    if r == -EROFS {
        bump(Counter::ProdRenameErofs);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_hard_link_diizinkan(
    euid: u32,
    inode_uid: u32,
    is_regular: u32,
    is_setuid: u32,
    is_setgid: u32,
    has_ixgrp: u32,
    may_write: u32,
) -> u32 {
    hard_link_diizinkan(
        euid,
        inode_uid,
        is_regular,
        is_setuid,
        is_setgid,
        has_ixgrp,
        may_write,
    )
}

#[no_mangle]
pub extern "C" fn klatos_izin_link(
    parent_may_write: u32,
    is_readonly: u32,
    is_dir_source: u32,
    hard_link_allowed: u32,
) -> i32 {
    bump(Counter::ProdLink);
    let r = izin_link(parent_may_write, is_readonly, is_dir_source, hard_link_allowed);
    if r == -EPERM {
        bump(Counter::ProdLinkEperm);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_modhapus_counter(idx: u32) -> u32 {
    if (idx as usize) < N_COUNTER {
        COUNTERS[idx as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_modhapus_reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Laporan + selftest
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosModHapusReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_hapus: u32,
    pub prod_hapus_eacces: u32,
    pub prod_hapus_erofs: u32,
    pub prod_rename: u32,
    pub prod_rename_eacces: u32,
    pub prod_rename_erofs: u32,
    pub prod_link: u32,
    pub prod_link_eperm: u32,
}

const _: [(); 48] = [(); core::mem::size_of::<KlatosModHapusReport>()];

#[no_mangle]
pub extern "C" fn klatos_modhapus_selftest() -> KlatosModHapusReport {
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
    check!(EACCES == 13);
    check!(EROFS == 30);
    check!(S_IXGRP == 0o0010);
    check!(REPORT_TAG == 0x4B4C_4154_4841_5053);

    // G2 — izin_hapus: induk tak bisa ditulisi ⇒ EACCES (menang atas segalanya)
    check!(izin_hapus(0, 0, 0, 0, 0, 1) == -EACCES);
    check!(izin_hapus(0, 1, 7, 8, 9, 0) == -EACCES);

    // G3 — izin_hapus: induk bisa ditulisi, tanpa sticky ⇒ 0
    check!(izin_hapus(1, 0, 7, 8, 9, 0) == 0);
    check!(izin_hapus(1, 0, 7, 8, 9, 1) == 0);

    // G4 — izin_hapus: sticky tanpa superuser
    // pemanggil adalah pemilik induk ⇒ 0
    check!(izin_hapus(1, 1, 500, 999, 500, 0) == 0);
    // pemanggil adalah pemilik inode ⇒ 0
    check!(izin_hapus(1, 1, 500, 501, 501, 0) == 0);
    // bukan pemilik keduanya ⇒ EACCES
    check!(izin_hapus(1, 1, 500, 501, 502, 0) == -EACCES);
    // superuser ⇒ 0
    check!(izin_hapus(1, 1, 500, 501, 502, 1) == 0);

    // G5 — hapus_erofs
    check!(hapus_erofs(0) == 0);
    check!(hapus_erofs(1) == -EROFS);

    // G6 — izin_rename: urutan cek tulis (induk BARU dulu, lalu LAMA)
    check!(izin_rename(1, 0, 0, 0, 0, 0, 0) == -EACCES);
    check!(izin_rename(0, 1, 0, 0, 0, 0, 0) == -EACCES);
    check!(izin_rename(0, 0, 0, 0, 0, 0, 0) == -EACCES);
    check!(izin_rename(1, 1, 0, 7, 8, 9, 0) == 0);

    // G7 — izin_rename: sticky induk lama
    check!(izin_rename(1, 1, 1, 500, 999, 500, 0) == 0);
    check!(izin_rename(1, 1, 1, 500, 501, 501, 0) == 0);
    check!(izin_rename(1, 1, 1, 500, 501, 502, 0) == -EACCES);
    check!(izin_rename(1, 1, 1, 500, 501, 502, 1) == 0);

    // G8 — rename_erofs (salah satu sisi)
    check!(rename_erofs(0, 0) == 0);
    check!(rename_erofs(1, 0) == -EROFS);
    check!(rename_erofs(0, 1) == -EROFS);
    check!(rename_erofs(1, 1) == -EROFS);

    // G9 — izin_rename_tujuan (sticky induk baru, tujuan ada)
    check!(izin_rename_tujuan(0, 0, 0, 0) == 0);
    check!(izin_rename_tujuan(1, 500, 500, 0) == 0);
    check!(izin_rename_tujuan(1, 500, 501, 0) == -EACCES);
    check!(izin_rename_tujuan(1, 500, 501, 1) == 0);

    // G10 — hard_link_diizinkan
    // pemilik inode ⇒ 1 (walau bukan berkas biasa)
    check!(hard_link_diizinkan(500, 500, 0, 1, 1, 1, 0) == 1);
    // bukan pemilik + berkas biasa + tanpa setid + boleh tulis ⇒ 1
    check!(hard_link_diizinkan(500, 501, 1, 0, 0, 0, 1) == 1);
    // bukan pemilik + bukan berkas biasa ⇒ 0
    check!(hard_link_diizinkan(500, 501, 0, 0, 0, 0, 1) == 0);
    // bukan pemilik + SUID ⇒ 0
    check!(hard_link_diizinkan(500, 501, 1, 1, 0, 0, 1) == 0);
    // bukan pemilik + SGID + S_IXGRP ⇒ 0
    check!(hard_link_diizinkan(500, 501, 1, 0, 1, 1, 1) == 0);
    // bukan pemilik + SGID tanpa S_IXGRP ⇒ 1
    check!(hard_link_diizinkan(500, 501, 1, 0, 1, 0, 1) == 1);
    // bukan pemilik + tidak boleh tulis ⇒ 0
    check!(hard_link_diizinkan(500, 501, 1, 0, 0, 0, 0) == 0);

    // G11 — izin_link: urutan EACCES → EPERM(direktori) → EROFS → EPERM(hard-link)
    check!(izin_link(0, 0, 0, 1) == -EACCES);
    check!(izin_link(0, 1, 1, 0) == -EACCES);
    check!(izin_link(1, 0, 1, 1) == -EPERM);
    check!(izin_link(1, 0, 1, 0) == -EPERM);
    check!(izin_link(1, 1, 0, 1) == -EROFS);
    check!(izin_link(1, 0, 0, 0) == -EPERM);
    check!(izin_link(1, 0, 0, 1) == 0);

    // G12 — diferensial 4096-iter izin_hapus vs oracle
    let mut d_hapus: u32 = 1;
    for i in 0u32..4096 {
        let pw = i & 1;
        let st = (i >> 1) & 1;
        let pu = (i >> 2) & 0x3F;
        let iu = (i >> 8) & 0x3F;
        let eu = (i >> 4) & 0x3F;
        let su = (i >> 10) & 1;
        let rust = izin_hapus(pw, st, pu, iu, eu, su);
        let expected: i32 = if pw == 0 {
            -EACCES
        } else if st != 0 && su == 0 && pu != eu && iu != eu {
            -EACCES
        } else {
            0
        };
        if rust != expected {
            d_hapus = 0;
        }
    }
    check!(d_hapus == 1);

    // G13 — diferensial 16384-iter hard_link_diizinkan vs oracle
    let mut d_link: u32 = 1;
    for i in 0u32..16384 {
        let eu = i & 0x1F;
        let iu = (i >> 5) & 0x1F;
        let reg = (i >> 10) & 1;
        let suid = (i >> 11) & 1;
        let sgid = (i >> 12) & 1;
        let ixg = (i >> 13) & 1;
        let mw = (i >> 14) & 1;
        let rust = hard_link_diizinkan(eu, iu, reg, suid, sgid, ixg, mw);
        let expected: u32 = if eu == iu {
            1
        } else if reg != 0 && suid == 0 && (sgid == 0 || ixg == 0) && mw != 0 {
            1
        } else {
            0
        };
        if rust != expected {
            d_link = 0;
        }
    }
    check!(d_link == 1);

    // G14 — determinisme + urutan parameter
    check!(izin_hapus(1, 1, 500, 501, 502, 0) == izin_hapus(1, 1, 500, 501, 502, 0));
    check!(izin_rename(1, 1, 1, 500, 501, 502, 0) == izin_rename(1, 1, 1, 500, 501, 502, 0));
    // urutan argumen izin_hapus: parent_uid sebelum inode_uid
    check!(izin_hapus(1, 1, 501, 500, 501, 0) == 0);
    check!(izin_hapus(1, 1, 501, 500, 502, 0) == -EACCES);

    KlatosModHapusReport {
        tag: REPORT_TAG,
        checks,
        passed,
        prod_hapus: COUNTERS[Counter::ProdHapus as usize].load(Ordering::Relaxed),
        prod_hapus_eacces: COUNTERS[Counter::ProdHapusEacces as usize].load(Ordering::Relaxed),
        prod_hapus_erofs: COUNTERS[Counter::ProdHapusErofs as usize].load(Ordering::Relaxed),
        prod_rename: COUNTERS[Counter::ProdRename as usize].load(Ordering::Relaxed),
        prod_rename_eacces: COUNTERS[Counter::ProdRenameEacces as usize].load(Ordering::Relaxed),
        prod_rename_erofs: COUNTERS[Counter::ProdRenameErofs as usize].load(Ordering::Relaxed),
        prod_link: COUNTERS[Counter::ProdLink as usize].load(Ordering::Relaxed),
        prod_link_eperm: COUNTERS[Counter::ProdLinkEperm as usize].load(Ordering::Relaxed),
    }
}

#[no_mangle]
pub extern "C" fn klatos_modhapus_report() -> KlatosModHapusReport {
    KlatosModHapusReport {
        tag: REPORT_TAG,
        checks: 0,
        passed: 0,
        prod_hapus: COUNTERS[Counter::ProdHapus as usize].load(Ordering::Relaxed),
        prod_hapus_eacces: COUNTERS[Counter::ProdHapusEacces as usize].load(Ordering::Relaxed),
        prod_hapus_erofs: COUNTERS[Counter::ProdHapusErofs as usize].load(Ordering::Relaxed),
        prod_rename: COUNTERS[Counter::ProdRename as usize].load(Ordering::Relaxed),
        prod_rename_eacces: COUNTERS[Counter::ProdRenameEacces as usize].load(Ordering::Relaxed),
        prod_rename_erofs: COUNTERS[Counter::ProdRenameErofs as usize].load(Ordering::Relaxed),
        prod_link: COUNTERS[Counter::ProdLink as usize].load(Ordering::Relaxed),
        prod_link_eperm: COUNTERS[Counter::ProdLinkEperm as usize].load(Ordering::Relaxed),
    }
}
