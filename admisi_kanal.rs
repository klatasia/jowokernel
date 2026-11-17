// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT
//! P2.03 — IPC channel admission (priority #9 batch 2 / Explore batch 2).


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EAGAIN: i32 = 11;
pub const EINVAL: i32 = 22;
pub const EPIPE: i32 = 32;
pub const EMSGSIZE: i32 = 55;

pub const REPORT_TAG: u64 = 0x4B4C_4154_4B41_4E4C; // KLATKANL

#[derive(Clone, Copy)]
pub enum Counter {
    PesanTerlaluBesar = 0,
    KanalPutus = 1,
    AntreanPenuh = 2,
    PopTakSiap = 3,
    BenderaPipaSalah = 4,
    FdBaruLuarRentang = 5,
}

const N_COUNTER: usize = 6;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// A message exceeds `max_message_size` on send, or the queued message is
/// larger than the caller's receive buffer.
#[inline]
pub const fn pesan_terlalu_besar(terlalu: u32) -> i32 {
    if terlalu != 0 {
        -EMSGSIZE
    } else {
        0
    }
}

/// The endpoint is closed, or its peer is gone — no delivery is possible.
#[inline]
pub const fn kanal_putus(putus: u32) -> i32 {
    if putus != 0 {
        -EPIPE
    } else {
        0
    }
}

/// `queued_bytes + size > max_queued_bytes` or
/// `queue.size() >= max_queued_messages`.
#[inline]
pub const fn antrean_penuh(penuh: u32) -> i32 {
    if penuh != 0 {
        -EAGAIN
    } else {
        0
    }
}

/// `Endpoint::pop` found an empty queue while the peer is still available, so
/// the caller should retry rather than see EOF.
#[inline]
pub const fn pop_tak_siap(tak_siap: u32) -> i32 {
    if tak_siap != 0 {
        -EAGAIN
    } else {
        0
    }
}

/// `sys$pipe`: `(flags & (O_CLOEXEC | O_NONBLOCK)) != flags`.
#[inline]
pub const fn bendera_pipa_salah(salah: u32) -> i32 {
    if salah != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$dup2`: `new_fd < 0 || new_fd >= OpenFileDescriptions::max_open()`.
#[inline]
pub const fn fd_baru_luar_rentang(luar: u32) -> i32 {
    if luar != 0 {
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

abi1!(klatos_kanal_pesan_terlalu_besar, pesan_terlalu_besar, PesanTerlaluBesar);
abi1!(klatos_kanal_kanal_putus, kanal_putus, KanalPutus);
abi1!(klatos_kanal_antrean_penuh, antrean_penuh, AntreanPenuh);
abi1!(klatos_kanal_pop_tak_siap, pop_tak_siap, PopTakSiap);
abi1!(klatos_kanal_bendera_pipa_salah, bendera_pipa_salah, BenderaPipaSalah);
abi1!(klatos_kanal_fd_baru_luar_rentang, fd_baru_luar_rentang, FdBaruLuarRentang);

#[no_mangle]
pub extern "C" fn klatos_kanal_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_kanal_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiKanalReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiKanalReport {
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
    x!(REPORT_TAG == 0x4B4C_4154_4B41_4E4C);
    x!(pesan_terlalu_besar(1) == -EMSGSIZE && pesan_terlalu_besar(0) == 0);
    x!(kanal_putus(1) == -EPIPE && kanal_putus(0) == 0);
    x!(antrean_penuh(1) == -EAGAIN && antrean_penuh(0) == 0);
    x!(pop_tak_siap(1) == -EAGAIN && pop_tak_siap(0) == 0);
    x!(bendera_pipa_salah(1) == -EINVAL && bendera_pipa_salah(0) == 0);
    x!(fd_baru_luar_rentang(1) == -EINVAL && fd_baru_luar_rentang(0) == 0);

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
        q!(pesan_terlalu_besar(v), if v != 0 { -EMSGSIZE } else { 0 });
        q!(kanal_putus(v), if v != 0 { -EPIPE } else { 0 });
        q!(antrean_penuh(v), if v != 0 { -EAGAIN } else { 0 });
        q!(pop_tak_siap(v), if v != 0 { -EAGAIN } else { 0 });
        q!(bendera_pipa_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(fd_baru_luar_rentang(v), if v != 0 { -EINVAL } else { 0 });
    }

    x!(d == 12);
    x!(b == 0);

    KlatosAdmisiKanalReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_kanal_selftest() -> KlatosAdmisiKanalReport {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 6);
        assert!(core::mem::size_of::<KlatosAdmisiKanalReport>() == 32);
    };
}
