// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! filesystem admission batch 2 (FUSE + residual one-liners) —
//! Explore batch 4 tail.



//! this x86_64 target).
#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EIO: i32 = 5;
pub const EINVAL: i32 = 22;

pub const REPORT_TAG: u64 = 0x4B4C_4154_4653_5232; // KLATFSR2

#[derive(Clone, Copy)]
pub enum Counter {
    FusePerangkatSalah = 0,
    FuseHeaderRusak = 1,
    FuseResponsTakValid = 2,
    TulisSingkat = 3,
    SisaTakCukup = 4,
    AnonOffsetBukanNol = 5,
}

const N_COUNTER: usize = 6;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// `FUSEConnection::try_create`: the description is not a device, or its
/// device class is not `FUSEDevice`.
#[inline]
pub const fn fuse_perangkat_salah(salah: u32) -> i32 {
    if salah != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `FUSEConnection::validate_response`: short/mismatched/oversized header.
#[inline]
pub const fn fuse_header_rusak(rusak: u32) -> i32 {
    if rusak != 0 {
        -EINVAL
    } else {
        0
    }
}

/// A FUSE reply failed `validate_response`; the caller reports `EIO`.
#[inline]
pub const fn fuse_respons_tak_valid(tak_valid: u32) -> i32 {
    if tak_valid != 0 {
        -EIO
    } else {
        0
    }
}

/// `Ext2FSInode::flush_block_list`: fewer bytes written than the serialized
/// directory holds.
#[inline]
pub const fn tulis_singkat(singkat: u32) -> i32 {
    if singkat != 0 {
        -EIO
    } else {
        0
    }
}

/// The output buffer cannot hold the buffered text-decoder remainder.
#[inline]
pub const fn sisa_tak_cukup(tak_cukup: u32) -> i32 {
    if tak_cukup != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `AnonymousFile`: an anonymous mapping must start at offset 0.
#[inline]
pub const fn anon_offset_bukan_nol(bukan_nol: u32) -> i32 {
    if bukan_nol != 0 {
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

abi1!(klatos_berkas2_fuse_perangkat_salah, fuse_perangkat_salah, FusePerangkatSalah);
abi1!(klatos_berkas2_fuse_header_rusak, fuse_header_rusak, FuseHeaderRusak);
abi1!(klatos_berkas2_fuse_respons_tak_valid, fuse_respons_tak_valid, FuseResponsTakValid);
abi1!(klatos_berkas2_tulis_singkat, tulis_singkat, TulisSingkat);
abi1!(klatos_berkas2_sisa_tak_cukup, sisa_tak_cukup, SisaTakCukup);
abi1!(klatos_berkas2_anon_offset_bukan_nol, anon_offset_bukan_nol, AnonOffsetBukanNol);

#[no_mangle]
pub extern "C" fn klatos_berkas2_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_berkas2_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiBerkas2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiBerkas2Report {
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
    x!(REPORT_TAG == 0x4B4C_4154_4653_5232);
    x!(fuse_perangkat_salah(1) == -EINVAL && fuse_perangkat_salah(0) == 0);
    x!(fuse_header_rusak(1) == -EINVAL && fuse_header_rusak(0) == 0);
    x!(fuse_respons_tak_valid(1) == -EIO && fuse_respons_tak_valid(0) == 0);
    x!(tulis_singkat(1) == -EIO && tulis_singkat(0) == 0);
    x!(sisa_tak_cukup(1) == -EINVAL && sisa_tak_cukup(0) == 0);
    x!(anon_offset_bukan_nol(1) == -EINVAL && anon_offset_bukan_nol(0) == 0);

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
        q!(fuse_perangkat_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(fuse_header_rusak(v), if v != 0 { -EINVAL } else { 0 });
        q!(fuse_respons_tak_valid(v), if v != 0 { -EIO } else { 0 });
        q!(tulis_singkat(v), if v != 0 { -EIO } else { 0 });
        q!(sisa_tak_cukup(v), if v != 0 { -EINVAL } else { 0 });
        q!(anon_offset_bukan_nol(v), if v != 0 { -EINVAL } else { 0 });
    }

    x!(d == 12);
    x!(b == 0);

    KlatosAdmisiBerkas2Report {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_berkas2_selftest() -> KlatosAdmisiBerkas2Report {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 6);
        assert!(core::mem::size_of::<KlatosAdmisiBerkas2Report>() == 32);
    };
}
