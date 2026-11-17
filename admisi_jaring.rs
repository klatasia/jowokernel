// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


//! P2.08 — network socket admission residuals (Explore batch 6).


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EAGAIN: i32 = 11;
pub const EINVAL: i32 = 22;
pub const EPIPE: i32 = 32;

pub const REPORT_TAG: u64 = 0x4B4C_4154_4A52_4E47; // KLATJRG

#[derive(Clone, Copy)]
pub enum Counter {
    AkanBlokir = 0,
    IovTerlaluBesar = 1,
    SudahDitutup = 2,
}

const N_COUNTER: usize = 3;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// A non-blocking socket has no data ready (`accept` / `sendmsg`).
#[inline]
pub const fn akan_blokir(akan: u32) -> i32 {
    if akan != 0 {
        -EAGAIN
    } else {
        0
    }
}

/// An `iovec` entry longer than `NumericLimits<ssize_t>::max()`.
#[inline]
pub const fn iov_terlalu_besar(besar: u32) -> i32 {
    if besar != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sendmsg` on a socket shut down for writing.
#[inline]
pub const fn sudah_ditutup(ditutup: u32) -> i32 {
    if ditutup != 0 {
        -EPIPE
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

abi1!(klatos_jaring_akan_blokir, akan_blokir, AkanBlokir);
abi1!(klatos_jaring_iov_terlalu_besar, iov_terlalu_besar, IovTerlaluBesar);
abi1!(klatos_jaring_sudah_ditutup, sudah_ditutup, SudahDitutup);

#[no_mangle]
pub extern "C" fn klatos_jaring_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_jaring_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiJaringReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiJaringReport {
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
    x!(REPORT_TAG == 0x4B4C_4154_4A52_4E47);
    x!(akan_blokir(1) == -EAGAIN && akan_blokir(0) == 0);
    x!(iov_terlalu_besar(1) == -EINVAL && iov_terlalu_besar(0) == 0);
    x!(sudah_ditutup(1) == -EPIPE && sudah_ditutup(0) == 0);

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
        q!(akan_blokir(v), if v != 0 { -EAGAIN } else { 0 });
        q!(iov_terlalu_besar(v), if v != 0 { -EINVAL } else { 0 });
        q!(sudah_ditutup(v), if v != 0 { -EPIPE } else { 0 });
    }

    x!(d == 6);
    x!(b == 0);

    KlatosAdmisiJaringReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_jaring_selftest() -> KlatosAdmisiJaringReport {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 3);
        assert!(core::mem::size_of::<KlatosAdmisiJaringReport>() == 32);
    };
}
