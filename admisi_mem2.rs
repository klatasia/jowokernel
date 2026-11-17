// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT
//! P2.01B — memory/VM admission batch 2.


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EPERM: i32 = 1;
pub const ENOMEM: i32 = 12;
pub const EACCES: i32 = 13;
pub const EFAULT: i32 = 14;
pub const EINVAL: i32 = 22;
pub const ENAMETOOLONG: i32 = 34;
pub const ENOTIMPL: i32 = 39;

pub const REPORT_TAG: u64 = 0x4B4C_4154_4D45_4D32; // KLATMEM2

/// Per-rule hit counters, one slot per exported predicate.
#[derive(Clone, Copy)]
pub enum Counter {
    RegionAda = 0,
    MadviseAnonim = 1,
    MadvisePurgeable = 2,
    MadviseTakDikenal = 3,
    MprotectCakupan = 4,
    MprotectTakCocok = 5,
    MremapTakDidukung = 6,
    AnnotasiFlags = 7,
    AnnotasiAlamatNol = 8,
    AnnotasiSyscallRegion = 9,
    AnnotasiTipe = 10,
    AnnotasiImmutable = 11,
    MsyncFlagsValid = 12,
    MsyncSinkronSama = 13,
    MsyncTakTerpetakan = 14,
    AlamatSelaras = 15,
    PurgeIzin = 16,
    BulatLuber = 17,
    RegiontreeUkuranNol = 18,
    InodeBacaIzin = 19,
    InodeTulisIzin = 20,
}

const N_COUNTER: usize = 21;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// `sys$madvise`/`sys$set_mmap_name`/`sys$mremap`/`sys$annotate_mapping`:
/// the range is not covered by any region.
#[inline]
pub const fn region_ada(ada: u32) -> i32 {
    if ada == 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$madvise`: `MADV_SET_*` on a non-anonymous vmobject.
#[inline]
pub const fn madvise_anonim(anonim: u32) -> i32 {
    if anonim == 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$madvise`: `MADV_SET_*` on a non-purgeable anonymous vmobject.
#[inline]
pub const fn madvise_purgeable(purgeable: u32) -> i32 {
    if purgeable == 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$madvise`: the advice value is not one this kernel implements.
#[inline]
pub const fn madvise_tak_dikenal() -> i32 {
    -EINVAL
}

/// `sys$mprotect`: the intersecting regions do not jointly cover the range.
#[inline]
pub const fn mprotect_cakupan(lengkap: u32) -> i32 {
    if lengkap == 0 {
        -ENOMEM
    } else {
        0
    }
}

/// `sys$mprotect`: the range matches no region at all.
#[inline]
pub const fn mprotect_tak_cocok() -> i32 {
    -EINVAL
}

/// `sys$mremap`: a remap request other than the shared-inode→private upgrade.
#[inline]
pub const fn mremap_tak_didukung() -> i32 {
    -ENOTIMPL
}

/// `sys$annotate_mapping`: `flags == None`.
#[inline]
pub const fn annotasi_flags(nol: u32) -> i32 {
    if nol != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$annotate_mapping`: a null address.
#[inline]
pub const fn annotasi_alamat_nol(nol: u32) -> i32 {
    if nol != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$annotate_mapping`: the process enforces syscall regions and the caller
/// asked for `SyscallCode`.
#[inline]
pub const fn annotasi_syscall_region(aktif: u32) -> i32 {
    if aktif != 0 {
        -EPERM
    } else {
        0
    }
}

/// `sys$annotate_mapping`: the region is neither an mmap region nor an
/// initially-loaded executable segment.
#[inline]
pub const fn annotasi_tipe(valid: u32) -> i32 {
    if valid == 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$annotate_mapping`: the region is immutable.
#[inline]
pub const fn annotasi_immutable(immutable: u32) -> i32 {
    if immutable != 0 {
        -EPERM
    } else {
        0
    }
}

/// `sys$msync`: the flags word carries bits outside `MS_SYNC|MS_ASYNC|MS_INVALIDATE`.
#[inline]
pub const fn msync_flags_valid(valid: u32) -> i32 {
    if valid == 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$msync`: `MS_SYNC` and `MS_ASYNC` were both set or both clear.
#[inline]
pub const fn msync_sinkron_sama(sama: u32) -> i32 {
    if sama != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$msync`: the range is not fully backed by mmap regions.
#[inline]
pub const fn msync_tak_terpetakan(kosong: u32, ada_non_mmap: u32, tidak_lengkap: u32) -> i32 {
    if kosong != 0 || ada_non_mmap != 0 || tidak_lengkap != 0 {
        -ENOMEM
    } else {
        0
    }
}

/// `AddressSpace::allocate_region*` / `sys$msync`: an address that is not
/// page-aligned where alignment is mandatory.
#[inline]
pub const fn alamat_selaras(tak_selaras: u32) -> i32 {
    if tak_selaras != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$purge`: the caller is not the superuser.
#[inline]
pub const fn purge_izin(izin: u32) -> i32 {
    if izin == 0 {
        -EPERM
    } else {
        0
    }
}

/// `page_round_up`: the value is too large to round up without wrapping.
#[inline]
pub const fn bulat_luber(luber: u32) -> i32 {
    if luber != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `RegionTree::allocate_range_*`: a zero-size placement request.
#[inline]
pub const fn regiontree_ukuran_nol(nol: u32) -> i32 {
    if nol != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `validate_inode_mmap_prot`: a read mapping of an inode that is not readable.
#[inline]
pub const fn inode_baca_izin(izin: u32) -> i32 {
    if izin == 0 {
        -EACCES
    } else {
        0
    }
}

/// `validate_inode_mmap_prot`: a shared write mapping of an inode that is not
/// writable.
#[inline]
pub const fn inode_tulis_izin(izin: u32) -> i32 {
    if izin == 0 {
        -EACCES
    } else {
        0
    }
}

macro_rules! abi1 {
    ($n:ident, $f:ident, $c:ident) => {
        #[no_mangle]
        pub extern "C" fn $n(v: u32) -> i32 {
            bump(Counter::$c);
            $f(v)
        }
    };
}

abi1!(klatos_mem2_region_ada, region_ada, RegionAda);
abi1!(klatos_mem2_madvise_anonim, madvise_anonim, MadviseAnonim);
abi1!(klatos_mem2_madvise_purgeable, madvise_purgeable, MadvisePurgeable);
abi1!(klatos_mem2_mprotect_cakupan, mprotect_cakupan, MprotectCakupan);
abi1!(klatos_mem2_annotasi_flags, annotasi_flags, AnnotasiFlags);
abi1!(klatos_mem2_annotasi_alamat_nol, annotasi_alamat_nol, AnnotasiAlamatNol);
abi1!(
    klatos_mem2_annotasi_syscall_region,
    annotasi_syscall_region,
    AnnotasiSyscallRegion
);
abi1!(klatos_mem2_annotasi_tipe, annotasi_tipe, AnnotasiTipe);
abi1!(klatos_mem2_annotasi_immutable, annotasi_immutable, AnnotasiImmutable);
abi1!(klatos_mem2_msync_flags_valid, msync_flags_valid, MsyncFlagsValid);
abi1!(klatos_mem2_msync_sinkron_sama, msync_sinkron_sama, MsyncSinkronSama);
abi1!(klatos_mem2_alamat_selaras, alamat_selaras, AlamatSelaras);
abi1!(klatos_mem2_purge_izin, purge_izin, PurgeIzin);
abi1!(klatos_mem2_bulat_luber, bulat_luber, BulatLuber);
abi1!(
    klatos_mem2_regiontree_ukuran_nol,
    regiontree_ukuran_nol,
    RegiontreeUkuranNol
);
abi1!(klatos_mem2_inode_baca_izin, inode_baca_izin, InodeBacaIzin);
abi1!(klatos_mem2_inode_tulis_izin, inode_tulis_izin, InodeTulisIzin);

#[no_mangle]
pub extern "C" fn klatos_mem2_madvise_tak_dikenal() -> i32 {
    bump(Counter::MadviseTakDikenal);
    madvise_tak_dikenal()
}

#[no_mangle]
pub extern "C" fn klatos_mem2_mprotect_tak_cocok() -> i32 {
    bump(Counter::MprotectTakCocok);
    mprotect_tak_cocok()
}

#[no_mangle]
pub extern "C" fn klatos_mem2_mremap_tak_didukung() -> i32 {
    bump(Counter::MremapTakDidukung);
    mremap_tak_didukung()
}

#[no_mangle]
pub extern "C" fn klatos_mem2_msync_tak_terpetakan(a: u32, b: u32, c: u32) -> i32 {
    bump(Counter::MsyncTakTerpetakan);
    msync_tak_terpetakan(a, b, c)
}

#[no_mangle]
pub extern "C" fn klatos_mem2_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_mem2_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiMem2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiMem2Report {
    let mut c = 0;
    let mut p = 0;
    macro_rules! x {
        ($e:expr) => {{
            c += 1;
            if $e {
                p += 1;
            }
        }};
    }
    x!(REPORT_TAG == 0x4B4C_4154_4D45_4D32);
    x!(region_ada(0) == -EINVAL && region_ada(1) == 0);
    x!(madvise_anonim(0) == -EINVAL && madvise_anonim(1) == 0);
    x!(madvise_purgeable(0) == -EINVAL && madvise_purgeable(1) == 0);
    x!(madvise_tak_dikenal() == -EINVAL);
    x!(mprotect_cakupan(0) == -ENOMEM && mprotect_cakupan(1) == 0);
    x!(mprotect_tak_cocok() == -EINVAL);
    x!(mremap_tak_didukung() == -ENOTIMPL);
    x!(annotasi_flags(1) == -EINVAL && annotasi_flags(0) == 0);
    x!(annotasi_alamat_nol(1) == -EINVAL && annotasi_alamat_nol(0) == 0);
    x!(annotasi_syscall_region(1) == -EPERM && annotasi_syscall_region(0) == 0);
    x!(annotasi_tipe(0) == -EINVAL && annotasi_tipe(1) == 0);
    x!(annotasi_immutable(1) == -EPERM && annotasi_immutable(0) == 0);
    x!(msync_flags_valid(0) == -EINVAL && msync_flags_valid(1) == 0);
    x!(msync_sinkron_sama(1) == -EINVAL && msync_sinkron_sama(0) == 0);
    x!(msync_tak_terpetakan(0, 0, 0) == 0);
    x!(msync_tak_terpetakan(1, 0, 0) == -ENOMEM);
    x!(msync_tak_terpetakan(0, 1, 0) == -ENOMEM);
    x!(msync_tak_terpetakan(0, 0, 1) == -ENOMEM);
    x!(alamat_selaras(1) == -EINVAL && alamat_selaras(0) == 0);
    x!(purge_izin(0) == -EPERM && purge_izin(1) == 0);
    x!(bulat_luber(1) == -EINVAL && bulat_luber(0) == 0);
    x!(regiontree_ukuran_nol(1) == -EINVAL && regiontree_ukuran_nol(0) == 0);
    x!(inode_baca_izin(0) == -EACCES && inode_baca_izin(1) == 0);
    x!(inode_tulis_izin(0) == -EACCES && inode_tulis_izin(1) == 0);

    let mut d = 0;
    let mut b = 0;
    macro_rules! q {
        ($f:expr, $n:expr) => {{
            d += 1;
            if $f != $n {
                b += 1;
            }
        }};
    }
    for v in 0..2 {
        q!(region_ada(v), if v == 0 { -EINVAL } else { 0 });
        q!(madvise_anonim(v), if v == 0 { -EINVAL } else { 0 });
        q!(madvise_purgeable(v), if v == 0 { -EINVAL } else { 0 });
        q!(mprotect_cakupan(v), if v == 0 { -ENOMEM } else { 0 });
        q!(annotasi_flags(v), if v != 0 { -EINVAL } else { 0 });
        q!(annotasi_alamat_nol(v), if v != 0 { -EINVAL } else { 0 });
        q!(
            annotasi_syscall_region(v),
            if v != 0 { -EPERM } else { 0 }
        );
        q!(annotasi_tipe(v), if v == 0 { -EINVAL } else { 0 });
        q!(annotasi_immutable(v), if v != 0 { -EPERM } else { 0 });
        q!(msync_flags_valid(v), if v == 0 { -EINVAL } else { 0 });
        q!(msync_sinkron_sama(v), if v != 0 { -EINVAL } else { 0 });
        q!(alamat_selaras(v), if v != 0 { -EINVAL } else { 0 });
        q!(purge_izin(v), if v == 0 { -EPERM } else { 0 });
        q!(bulat_luber(v), if v != 0 { -EINVAL } else { 0 });
        q!(regiontree_ukuran_nol(v), if v != 0 { -EINVAL } else { 0 });
        q!(inode_baca_izin(v), if v == 0 { -EACCES } else { 0 });
        q!(inode_tulis_izin(v), if v == 0 { -EACCES } else { 0 });
    }
    q!(madvise_tak_dikenal(), -EINVAL);
    q!(mprotect_tak_cocok(), -EINVAL);
    q!(mremap_tak_didukung(), -ENOTIMPL);
    for i in 0..2 {
        for j in 0..2 {
            for k in 0..2 {
                q!(
                    msync_tak_terpetakan(i, j, k),
                    if i != 0 || j != 0 || k != 0 {
                        -ENOMEM
                    } else {
                        0
                    }
                );
            }
        }
    }
    // F-P8.4-C — diferensial `anon_ukuran` (8 entri) dipindahkan: cakupan
    // kebijakan ukuran berkas tanpa nama kini dimiliki penuh oleh
    // `siklus_berkas_maya::ukuran_sah` (diferensial 100.000 kasus).
    x!(d == 45);
    x!(b == 0);

    KlatosAdmisiMem2Report {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_mem2_selftest() -> KlatosAdmisiMem2Report {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 21);
        assert!(core::mem::size_of::<KlatosAdmisiMem2Report>() == 32);
    };
}
