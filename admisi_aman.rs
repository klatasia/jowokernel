// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT
//! P2.02 — security / capability admission (priority #9 batch 1).


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EPERM: i32 = 1;
pub const ENODEV: i32 = 19;
pub const EINVAL: i32 = 22;
pub const EAFNOSUPPORT: i32 = 40;
pub const ENOTSOCK: i32 = 41;
pub const ENOTCONN: i32 = 63;
pub const ENOTSUP: i32 = 74;

pub const REPORT_TAG: u64 = 0x4B4C_4154_414D_414E; // KLATAMAN

#[derive(Clone, Copy)]
pub enum Counter {
    SuperuserIzin = 0,
    JailIzin = 1,
    SocketBukanSocket = 2,
    SocketBukanLokal = 3,
    SocketTakTerhubung = 4,
    VeilTerkunci = 5,
    IzinMeningkat = 6,
    JanjiTakDikenal = 7,
    GetrandomUkuran = 8,
    MountJenisSalah = 9,
    BindTanpaCustody = 10,
    UnshareTipeSalah = 11,
    UnshareBenderaKosong = 12,
    UnshareSumberSalah = 13,
    UnshareTipeTakDidukung = 14,
}

const N_COUNTER: usize = 15;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// A privileged mount/unshare syscall was issued by a non-superuser.
#[inline]
pub const fn superuser_izin(superuser: u32) -> i32 {
    if superuser == 0 {
        -EPERM
    } else {
        0
    }
}

/// A jailed process attempted a namespace operation.
#[inline]
pub const fn jail_izin(jailed: u32) -> i32 {
    if jailed != 0 {
        -EPERM
    } else {
        0
    }
}

/// `sys$sendfd`/`sys$recvfd`: the descriptor is not a socket.
#[inline]
pub const fn socket_bukan_socket(bukan: u32) -> i32 {
    if bukan != 0 {
        -ENOTSOCK
    } else {
        0
    }
}

/// `sys$sendfd`/`sys$recvfd`: the socket is not a local (AF_LOCAL) socket.
#[inline]
pub const fn socket_bukan_lokal(bukan: u32) -> i32 {
    if bukan != 0 {
        -EAFNOSUPPORT
    } else {
        0
    }
}

/// `sys$sendfd`: the local socket is not connected.
#[inline]
pub const fn socket_tak_terhubung(belum: u32) -> i32 {
    if belum != 0 {
        -ENOTCONN
    } else {
        0
    }
}

/// `update_unakses_jalur_data`: the akses_jalur was locked (possibly by another thread).
#[inline]
pub const fn akses_jalur_terkunci(terkunci: u32) -> i32 {
    if terkunci != 0 {
        -EPERM
    } else {
        0
    }
}

/// `update_unakses_jalur_data`: an explicitly-akses-jalur node cannot have its
/// permissions elevated — `new_permissions & ~existing_permissions`.
#[inline]
pub const fn izin_meningkat(baru: u32, lama: u32) -> i32 {
    if (baru & !lama) != 0 {
        -EPERM
    } else {
        0
    }
}

/// `parse_pledge`: the spec string contained a token that is not a known
/// promise.
#[inline]
pub const fn janji_tak_dikenal(tak_valid: u32) -> i32 {
    if tak_valid != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$getrandom`: `buffer_size > NumericLimits<ssize_t>::max()`.
#[inline]
pub const fn getrandom_ukuran(terlalu_besar: u32) -> i32 {
    if terlalu_besar != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$fsmount`: the mount fd is not a `MountFile`.
#[inline]
pub const fn mount_jenis_salah(bukan_mount: u32) -> i32 {
    if bukan_mount != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$bindmount`: the source descriptor has no custody (not an inode).
#[inline]
pub const fn bind_tanpa_custody(tanpa: u32) -> i32 {
    if tanpa != 0 {
        -ENODEV
    } else {
        0
    }
}

/// `sys$unshare_open`/`sys$unshare_enter`: `params.type < 0`.
#[inline]
pub const fn unshare_tipe_salah(negatif: u32) -> i32 {
    if negatif != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$unshare_enter`: `params.flags == UnshareEnterFlags::None`.
#[inline]
pub const fn unshare_bendera_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$unshare_create`: the descriptor is not an `UnsharedResourceFile`.
#[inline]
pub const fn unshare_sumber_salah(bukan: u32) -> i32 {
    if bukan != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `sys$unshare_open`/`sys$unshare_enter`: the (non-negative) type is not a
/// known `UnshareType`.
#[inline]
pub const fn unshare_tipe_tak_didukung() -> i32 {
    -ENOTSUP
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

abi1!(klatos_aman_superuser_izin, superuser_izin, SuperuserIzin);
abi1!(klatos_aman_jail_izin, jail_izin, JailIzin);
abi1!(klatos_aman_socket_bukan_socket, socket_bukan_socket, SocketBukanSocket);
abi1!(klatos_aman_socket_bukan_lokal, socket_bukan_lokal, SocketBukanLokal);
abi1!(klatos_aman_socket_tak_terhubung, socket_tak_terhubung, SocketTakTerhubung);
abi1!(klatos_aman_akses_jalur_terkunci, akses_jalur_terkunci, VeilTerkunci);
abi1!(klatos_aman_janji_tak_dikenal, janji_tak_dikenal, JanjiTakDikenal);
abi1!(klatos_aman_getrandom_ukuran, getrandom_ukuran, GetrandomUkuran);
abi1!(klatos_aman_mount_jenis_salah, mount_jenis_salah, MountJenisSalah);
abi1!(klatos_aman_bind_tanpa_custody, bind_tanpa_custody, BindTanpaCustody);
abi1!(klatos_aman_unshare_tipe_salah, unshare_tipe_salah, UnshareTipeSalah);
abi1!(klatos_aman_unshare_bendera_kosong, unshare_bendera_kosong, UnshareBenderaKosong);
abi1!(klatos_aman_unshare_sumber_salah, unshare_sumber_salah, UnshareSumberSalah);

#[no_mangle]
pub extern "C" fn klatos_aman_izin_meningkat(baru: u32, lama: u32) -> i32 {
    bump(Counter::IzinMeningkat);
    izin_meningkat(baru, lama)
}

#[no_mangle]
pub extern "C" fn klatos_aman_unshare_tipe_tak_didukung() -> i32 {
    bump(Counter::UnshareTipeTakDidukung);
    unshare_tipe_tak_didukung()
}

#[no_mangle]
pub extern "C" fn klatos_aman_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_aman_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiAmanReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiAmanReport {
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
    x!(REPORT_TAG == 0x4B4C_4154_414D_414E);
    x!(superuser_izin(1) == 0 && superuser_izin(0) == -EPERM);
    x!(jail_izin(0) == 0 && jail_izin(1) == -EPERM);
    x!(socket_bukan_socket(0) == 0 && socket_bukan_socket(1) == -ENOTSOCK);
    x!(socket_bukan_lokal(0) == 0 && socket_bukan_lokal(1) == -EAFNOSUPPORT);
    x!(socket_tak_terhubung(0) == 0 && socket_tak_terhubung(1) == -ENOTCONN);
    x!(akses_jalur_terkunci(0) == 0 && akses_jalur_terkunci(1) == -EPERM);
    x!(izin_meningkat(0b001, 0b011) == 0
        && izin_meningkat(0b100, 0b011) == -EPERM
        && izin_meningkat(0, 0) == 0);
    x!(janji_tak_dikenal(0) == 0 && janji_tak_dikenal(1) == -EINVAL);
    x!(getrandom_ukuran(0) == 0 && getrandom_ukuran(1) == -EINVAL);
    x!(mount_jenis_salah(0) == 0 && mount_jenis_salah(1) == -EINVAL);
    x!(bind_tanpa_custody(0) == 0 && bind_tanpa_custody(1) == -ENODEV);
    x!(unshare_tipe_salah(0) == 0 && unshare_tipe_salah(1) == -EINVAL);
    x!(unshare_bendera_kosong(0) == 0 && unshare_bendera_kosong(1) == -EINVAL);
    x!(unshare_sumber_salah(0) == 0 && unshare_sumber_salah(1) == -EINVAL);
    x!(unshare_tipe_tak_didukung() == -ENOTSUP);

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
        q!(superuser_izin(v), if v == 0 { -EPERM } else { 0 });
        q!(jail_izin(v), if v != 0 { -EPERM } else { 0 });
        q!(socket_bukan_socket(v), if v != 0 { -ENOTSOCK } else { 0 });
        q!(socket_bukan_lokal(v), if v != 0 { -EAFNOSUPPORT } else { 0 });
        q!(socket_tak_terhubung(v), if v != 0 { -ENOTCONN } else { 0 });
        q!(akses_jalur_terkunci(v), if v != 0 { -EPERM } else { 0 });
        q!(janji_tak_dikenal(v), if v != 0 { -EINVAL } else { 0 });
        q!(getrandom_ukuran(v), if v != 0 { -EINVAL } else { 0 });
        q!(mount_jenis_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(bind_tanpa_custody(v), if v != 0 { -ENODEV } else { 0 });
        q!(unshare_tipe_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(unshare_bendera_kosong(v), if v != 0 { -EINVAL } else { 0 });
        q!(unshare_sumber_salah(v), if v != 0 { -EINVAL } else { 0 });
    }
    // Historical C++: `new_permissions & ~matching_node.permissions()`.
    for baru in 0..4u32 {
        for lama in 0..4u32 {
            q!(izin_meningkat(baru, lama), if (baru & !lama) != 0 { -EPERM } else { 0 });
        }
    }
    q!(unshare_tipe_tak_didukung(), -ENOTSUP);

    x!(d == 43);
    x!(b == 0);

    KlatosAdmisiAmanReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_aman_selftest() -> KlatosAdmisiAmanReport {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 15);
        assert!(core::mem::size_of::<KlatosAdmisiAmanReport>() == 32);
    };
}
