// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


//! generic syscall admission residuals, batch 2 (Explore batch 8).


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const ENOENT: i32 = 2;
pub const EBADF: i32 = 9;
pub const ENOMEM: i32 = 12;
pub const EINVAL: i32 = 22;
pub const ENOTSUP: i32 = 74;

pub const REPORT_TAG: u64 = 0x4B4C_4154_5041_4E32; // KLATPAN2

#[derive(Clone, Copy)]
pub enum Counter {
    TimerHabis = 0,
    PanjangFtruncateNegatif = 1,
    BukanInodeWatcher = 2,
    WatcherTakDidukung = 3,
    BukanSymlink = 4,
    RusageTakDikenal = 5,
    InodeTakAda = 6,
    SysconfTakDikenal = 7,
    TanpaInode = 8,
    TanpaCustody = 9,
}

const N_COUNTER: usize = 10;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// `alarm`: `TimerQueue::add_timer_without_id` refused the timer.
#[inline]
pub const fn timer_habis(habis: u32) -> i32 {
    if habis != 0 {
        -ENOMEM
    } else {
        0
    }
}

/// `ftruncate`: a negative length.
#[inline]
pub const fn panjang_ftruncate_negatif(negatif: u32) -> i32 {
    if negatif != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `inode_watcher`: the description is not an inode watcher.
#[inline]
pub const fn bukan_inode_watcher(bukan: u32) -> i32 {
    if bukan != 0 {
        -EBADF
    } else {
        0
    }
}

/// `inode_watcher`: the filesystem does not support watchers.
#[inline]
pub const fn watcher_tak_didukung(tak: u32) -> i32 {
    if tak != 0 {
        -ENOTSUP
    } else {
        0
    }
}

/// `readlink`: the target is not a symlink.
#[inline]
pub const fn bukan_symlink(bukan: u32) -> i32 {
    if bukan != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `getrusage`: an unknown `who` target.
#[inline]
pub const fn rusage_tak_dikenal(tak: u32) -> i32 {
    if tak != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `statvfs`: the description has no inode.
#[inline]
pub const fn inode_tak_ada(tak: u32) -> i32 {
    if tak != 0 {
        -ENOENT
    } else {
        0
    }
}

/// `sysconf`: an unknown configuration name.
#[inline]
pub const fn sysconf_tak_dikenal(tak: u32) -> i32 {
    if tak != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `utimensat`: the description has no inode.
#[inline]
pub const fn tanpa_inode(tanpa: u32) -> i32 {
    if tanpa != 0 {
        -EBADF
    } else {
        0
    }
}

/// `utimensat`: the description has no custody.
#[inline]
pub const fn tanpa_custody(tanpa: u32) -> i32 {
    if tanpa != 0 {
        -EBADF
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

abi1!(klatos_pang2_timer_habis, timer_habis, TimerHabis);
abi1!(klatos_pang2_panjang_ftruncate_negatif, panjang_ftruncate_negatif, PanjangFtruncateNegatif);
abi1!(klatos_pang2_bukan_inode_watcher, bukan_inode_watcher, BukanInodeWatcher);
abi1!(klatos_pang2_watcher_tak_didukung, watcher_tak_didukung, WatcherTakDidukung);
abi1!(klatos_pang2_bukan_symlink, bukan_symlink, BukanSymlink);
abi1!(klatos_pang2_rusage_tak_dikenal, rusage_tak_dikenal, RusageTakDikenal);
abi1!(klatos_pang2_inode_tak_ada, inode_tak_ada, InodeTakAda);
abi1!(klatos_pang2_sysconf_tak_dikenal, sysconf_tak_dikenal, SysconfTakDikenal);
abi1!(klatos_pang2_tanpa_inode, tanpa_inode, TanpaInode);
abi1!(klatos_pang2_tanpa_custody, tanpa_custody, TanpaCustody);

#[no_mangle]
pub extern "C" fn klatos_pang2_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_pang2_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiPanggilan2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiPanggilan2Report {
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
    x!(REPORT_TAG == 0x4B4C_4154_5041_4E32);
    x!(timer_habis(1) == -ENOMEM && timer_habis(0) == 0);
    x!(panjang_ftruncate_negatif(1) == -EINVAL && panjang_ftruncate_negatif(0) == 0);
    x!(bukan_inode_watcher(1) == -EBADF && bukan_inode_watcher(0) == 0);
    x!(watcher_tak_didukung(1) == -ENOTSUP && watcher_tak_didukung(0) == 0);
    x!(bukan_symlink(1) == -EINVAL && bukan_symlink(0) == 0);
    x!(rusage_tak_dikenal(1) == -EINVAL && rusage_tak_dikenal(0) == 0);
    x!(inode_tak_ada(1) == -ENOENT && inode_tak_ada(0) == 0);
    x!(sysconf_tak_dikenal(1) == -EINVAL && sysconf_tak_dikenal(0) == 0);
    x!(tanpa_inode(1) == -EBADF && tanpa_inode(0) == 0);
    x!(tanpa_custody(1) == -EBADF && tanpa_custody(0) == 0);

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
        q!(timer_habis(v), if v != 0 { -ENOMEM } else { 0 });
        q!(panjang_ftruncate_negatif(v), if v != 0 { -EINVAL } else { 0 });
        q!(bukan_inode_watcher(v), if v != 0 { -EBADF } else { 0 });
        q!(watcher_tak_didukung(v), if v != 0 { -ENOTSUP } else { 0 });
        q!(bukan_symlink(v), if v != 0 { -EINVAL } else { 0 });
        q!(rusage_tak_dikenal(v), if v != 0 { -EINVAL } else { 0 });
        q!(inode_tak_ada(v), if v != 0 { -ENOENT } else { 0 });
        q!(sysconf_tak_dikenal(v), if v != 0 { -EINVAL } else { 0 });
        q!(tanpa_inode(v), if v != 0 { -EBADF } else { 0 });
        q!(tanpa_custody(v), if v != 0 { -EBADF } else { 0 });
    }

    x!(d == 20);
    x!(b == 0);

    KlatosAdmisiPanggilan2Report {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_pang2_selftest() -> KlatosAdmisiPanggilan2Report {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 10);
        assert!(core::mem::size_of::<KlatosAdmisiPanggilan2Report>() == 32);
    };
}
