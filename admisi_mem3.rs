// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


//! P2.01C — memory/VM admission batch 3 (VMObject / RingBuffer guards).

#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EINVAL: i32 = 22;
pub const ENOMEM: i32 = 12;
pub const ENOSPC: i32 = 28;
pub const ENOTIMPL: i32 = 39;
pub const ENOTSUP: i32 = 74;

pub const REPORT_TAG: u64 = 0x4B4C_4154_4D45_4D33; // KLATMEM3

#[derive(Clone, Copy)]
pub enum Counter {
    InodeKosong = 0,
    PaddrLuber = 1,
    RingbufferPenuh = 2,
    CloneTakDidukung = 3,
    CloneTakImpl = 4,
    ShrinkTerbalik = 5,
}

const N_COUNTER: usize = 6;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// An inode-backed VMObject was requested for an empty inode.
#[inline]
pub const fn inode_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `paddr + size` wrapped; the caller is told the allocation failed.
#[inline]
pub const fn paddr_luber(luber: u32) -> i32 {
    if luber != 0 {
        -ENOMEM
    } else {
        0
    }
}

/// `RingBuffer::reserve_space` was asked for more than the ring holds.
#[inline]
pub const fn ringbuffer_penuh(penuh: u32) -> i32 {
    if penuh != 0 {
        -ENOSPC
    } else {
        0
    }
}

/// `MMIOVMObject::try_clone` — cloning this VMObject type is unsupported.
#[inline]
pub const fn clone_tak_didukung() -> i32 {
    -ENOTSUP
}

/// `SharedFramebufferVMObject::try_clone` — cloning is not implemented.
#[inline]
pub const fn clone_tak_impl() -> i32 {
    -ENOTIMPL
}

/// `shrink_range_to_page_boundaries`: the shrunk end precedes the base.
#[inline]
pub const fn shrink_terbalik(terbalik: u32) -> i32 {
    if terbalik != 0 {
        -EINVAL
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

abi1!(klatos_mem3_inode_kosong, inode_kosong, InodeKosong);
abi1!(klatos_mem3_paddr_luber, paddr_luber, PaddrLuber);
abi1!(klatos_mem3_ringbuffer_penuh, ringbuffer_penuh, RingbufferPenuh);
abi1!(klatos_mem3_shrink_terbalik, shrink_terbalik, ShrinkTerbalik);

#[no_mangle]
pub extern "C" fn klatos_mem3_clone_tak_didukung() -> i32 {
    bump(Counter::CloneTakDidukung);
    clone_tak_didukung()
}

#[no_mangle]
pub extern "C" fn klatos_mem3_clone_tak_impl() -> i32 {
    bump(Counter::CloneTakImpl);
    clone_tak_impl()
}

#[no_mangle]
pub extern "C" fn klatos_mem3_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_mem3_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiMem3Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiMem3Report {
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
    x!(REPORT_TAG == 0x4B4C_4154_4D45_4D33);
    x!(inode_kosong(1) == -EINVAL && inode_kosong(0) == 0);
    x!(paddr_luber(1) == -ENOMEM && paddr_luber(0) == 0);
    x!(ringbuffer_penuh(1) == -ENOSPC && ringbuffer_penuh(0) == 0);
    x!(clone_tak_didukung() == -ENOTSUP);
    x!(clone_tak_impl() == -ENOTIMPL);
    x!(shrink_terbalik(1) == -EINVAL && shrink_terbalik(0) == 0);

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
        q!(inode_kosong(v), if v != 0 { -EINVAL } else { 0 });
        q!(paddr_luber(v), if v != 0 { -ENOMEM } else { 0 });
        q!(ringbuffer_penuh(v), if v != 0 { -ENOSPC } else { 0 });
        q!(shrink_terbalik(v), if v != 0 { -EINVAL } else { 0 });
    }
    q!(clone_tak_didukung(), -ENOTSUP);
    q!(clone_tak_impl(), -ENOTIMPL);
    x!(d == 10);
    x!(b == 0);

    KlatosAdmisiMem3Report {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_mem3_selftest() -> KlatosAdmisiMem3Report {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 6);
        assert!(core::mem::size_of::<KlatosAdmisiMem3Report>() == 32);
    };
}
