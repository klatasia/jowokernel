// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


//! P2.04 — filesystem admission batch 1 (RAMFS + base `Inode` + Ext2FS


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EIO: i32 = 5;
pub const EAGAIN: i32 = 11;
pub const ENOTDIR: i32 = 20;
pub const EINVAL: i32 = 22;
pub const ENOSPC: i32 = 28;
pub const ENOTSUP: i32 = 74;

pub const REPORT_TAG: u64 = 0x4B4C_4154_4252_4B53; // KLATBRKS

#[derive(Clone, Copy)]
pub enum Counter {
    BukanDirektori = 0,
    FlockAkanBlokir = 1,
    FlockTakDidukung = 2,
    FlockWhenceSalah = 3,
    RuangHabis = 4,
    MetadataRusak = 5,
}

const N_COUNTER: usize = 6;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// `traverse_as_directory` was called on an inode that is not a directory.
#[inline]
pub const fn bukan_direktori(bukan: u32) -> i32 {
    if bukan != 0 {
        -ENOTDIR
    } else {
        0
    }
}

/// `apply_flock`: the lock cannot be granted now and the caller asked not to
/// block.
#[inline]
pub const fn flock_akan_blokir(blokir: u32) -> i32 {
    if blokir != 0 {
        -EAGAIN
    } else {
        0
    }
}

/// `normalize_flock`: `SEEK_END` is not implemented.
#[inline]
pub const fn flock_tak_didukung(tak_didukung: u32) -> i32 {
    if tak_didukung != 0 {
        -ENOTSUP
    } else {
        0
    }
}

/// `normalize_flock`: `whence` is none of `SEEK_SET`/`SEEK_CUR`/`SEEK_END`.
#[inline]
pub const fn flock_whence_salah(salah: u32) -> i32 {
    if salah != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `Ext2FS::allocate_blocks`/`allocate_inode`: the filesystem is full.
#[inline]
pub const fn ruang_habis(habis: u32) -> i32 {
    if habis != 0 {
        -ENOSPC
    } else {
        0
    }
}

/// `Ext2FS::allocate_inode`: the block-group descriptor claims free inodes but
/// the bitmap has none — metadata is inconsistent.
#[inline]
pub const fn metadata_rusak(rusak: u32) -> i32 {
    if rusak != 0 {
        -EIO
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

abi1!(klatos_berkas_bukan_direktori, bukan_direktori, BukanDirektori);
abi1!(klatos_berkas_flock_akan_blokir, flock_akan_blokir, FlockAkanBlokir);
abi1!(klatos_berkas_flock_tak_didukung, flock_tak_didukung, FlockTakDidukung);
abi1!(klatos_berkas_flock_whence_salah, flock_whence_salah, FlockWhenceSalah);
abi1!(klatos_berkas_ruang_habis, ruang_habis, RuangHabis);
abi1!(klatos_berkas_metadata_rusak, metadata_rusak, MetadataRusak);

#[no_mangle]
pub extern "C" fn klatos_berkas_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_berkas_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiBerkasReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiBerkasReport {
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
    x!(REPORT_TAG == 0x4B4C_4154_4252_4B53);
    x!(bukan_direktori(1) == -ENOTDIR && bukan_direktori(0) == 0);
    x!(flock_akan_blokir(1) == -EAGAIN && flock_akan_blokir(0) == 0);
    x!(flock_tak_didukung(1) == -ENOTSUP && flock_tak_didukung(0) == 0);
    x!(flock_whence_salah(1) == -EINVAL && flock_whence_salah(0) == 0);
    x!(ruang_habis(1) == -ENOSPC && ruang_habis(0) == 0);
    x!(metadata_rusak(1) == -EIO && metadata_rusak(0) == 0);

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
    for v in 0..2u32 {
        q!(bukan_direktori(v), if v != 0 { -ENOTDIR } else { 0 });
        q!(flock_akan_blokir(v), if v != 0 { -EAGAIN } else { 0 });
        q!(flock_tak_didukung(v), if v != 0 { -ENOTSUP } else { 0 });
        q!(flock_whence_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(ruang_habis(v), if v != 0 { -ENOSPC } else { 0 });
        q!(metadata_rusak(v), if v != 0 { -EIO } else { 0 });
    }

    x!(d == 12);
    x!(b == 0);

    KlatosAdmisiBerkasReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_berkas_selftest() -> KlatosAdmisiBerkasReport {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 6);
        assert!(core::mem::size_of::<KlatosAdmisiBerkasReport>() == 32);
    };
}
