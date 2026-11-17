// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


//! process/identity admission batch 2 (Explore batch 5b).
//!
#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EPERM: i32 = 1;
pub const ENOENT: i32 = 2;
pub const ESRCH: i32 = 3;
pub const EBADF: i32 = 9;
pub const ENOMEM: i32 = 12;
pub const EFAULT: i32 = 14;
pub const EEXIST: i32 = 17;
pub const ENOTDIR: i32 = 20;
pub const EINVAL: i32 = 22;
pub const EMFILE: i32 = 24;
pub const ENAMETOOLONG: i32 = 34;
pub const EDOM: i32 = 44;
pub const ENOBUFS: i32 = 59;
pub const EPROMISEVIOLATION: i32 = 80;

pub const REPORT_TAG: u64 = 0x4B4C_4154_5052_5332; // KLATPRS2

#[derive(Clone, Copy)]
pub enum Counter {
    FdTanpaDeskripsi = 0,
    ThreadTakDitemukan = 1,
    SumberHabis = 2,
    PathKosong = 3,
    PathTerlaluPanjang = 4,
    NamaPerfcoreHabis = 5,
    TanpaPenyangga = 6,
    PenyanggaHabis = 7,
    JanjiDilanggar = 8,
    KonteksIdSalah = 9,
    JailKonteks = 10,
    KonteksTakAda = 11,
    PathRelatifLintasKonteks = 12,
    CustodyKosong = 13,
    BukanDirektoriCustody = 14,
    TumpukanBalikKeKernel = 15,
    BacaUserGagal = 16,
    TopengProfilingSalah = 17,
    ProfilBersarang = 18,
    JenisPeristiwaSalah = 19,
}

const N_COUNTER: usize = 20;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// An fd slot that holds no `OpenFileDescription` (`open_file_description`),
/// or an IPC handle slot with no endpoint (`JowoIPCHandles::remove`).
#[inline]
pub const fn fd_tanpa_deskripsi(kosong: u32) -> i32 {
    if kosong != 0 {
        -EBADF
    } else {
        0
    }
}

/// No such thread: a negative tid, no thread in the list, or no signal
/// receiver at all.
#[inline]
pub const fn thread_tak_ditemukan(tak_ada: u32) -> i32 {
    if tak_ada != 0 {
        -ESRCH
    } else {
        0
    }
}

/// A per-process table is full (fd table / IPC handle table / pending queue).
#[inline]
pub const fn sumber_habis(habis: u32) -> i32 {
    if habis != 0 {
        -EMFILE
    } else {
        0
    }
}

/// `get_syscall_path_argument`: a zero-length path.
#[inline]
pub const fn path_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        -ENOENT
    } else {
        0
    }
}

/// `get_syscall_path_argument`: a path longer than `PATH_MAX`.
#[inline]
pub const fn path_terlalu_panjang(panjang: u32) -> i32 {
    if panjang != 0 {
        -ENAMETOOLONG
    } else {
        0
    }
}

/// `dump_perfcore`: ten filename attempts all collided.
#[inline]
pub const fn nama_perfcore_habis(habis: u32) -> i32 {
    if habis != 0 {
        -EEXIST
    } else {
        0
    }
}

/// `dump_perfcore`: `KBufferBuilder::build()` returned no buffer.
#[inline]
pub const fn tanpa_penyangga(tanpa: u32) -> i32 {
    if tanpa != 0 {
        -ENOMEM
    } else {
        0
    }
}

/// A buffer is full: no free coredump property slot, or the performance event
/// buffer is at capacity.
#[inline]
pub const fn penyangga_habis(habis: u32) -> i32 {
    if habis != 0 {
        -ENOBUFS
    } else {
        0
    }
}

/// `require_no_promises` / `require_promise`: a pledge promise was violated.
#[inline]
pub const fn janji_dilanggar(dilanggar: u32) -> i32 {
    if dilanggar != 0 {
        -EPROMISEVIOLATION
    } else {
        0
    }
}

/// `vfs_root_context_for_id`: an id that is neither `-1` nor positive.
#[inline]
pub const fn konteks_id_salah(salah: u32) -> i32 {
    if salah != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `vfs_root_context_for_id`: a jailed process named another context.
#[inline]
pub const fn jail_konteks(jailed: u32) -> i32 {
    if jailed != 0 {
        -EPERM
    } else {
        0
    }
}

/// `vfs_root_context_for_id`: the id is valid but names no context.
#[inline]
pub const fn konteks_tak_ada(tak_ada: u32) -> i32 {
    if tak_ada != 0 {
        -EDOM
    } else {
        0
    }
}

/// A relative path used across two different VFS root contexts.
#[inline]
pub const fn path_relatif_lintas_konteks(relatif: u32) -> i32 {
    if relatif != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `custody_for_dirfd`: the description carries no custody.
#[inline]
pub const fn custody_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `custody_for_dirfd`: the description's custody is not a directory.
#[inline]
pub const fn bukan_direktori_custody(bukan: u32) -> i32 {
    if bukan != 0 {
        -ENOTDIR
    } else {
        0
    }
}

/// `raw_backtrace`: a userspace stack frame points back into kernel memory.
#[inline]
pub const fn tumpukan_balik_ke_kernel(balik: u32) -> i32 {
    if balik != 0 {
        -EFAULT
    } else {
        0
    }
}

/// A user read failed: a stack unwind read, or `poke_user_data`'s
/// "no region contains this address".
#[inline]
pub const fn baca_user_gagal(gagal: u32) -> i32 {
    if gagal != 0 {
        -EFAULT
    } else {
        0
    }
}

/// `append_with_ip_and_bp`: the event type is not in `g_profiling_event_mask`.
#[inline]
pub const fn topeng_profiling_salah(salah: u32) -> i32 {
    if salah != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `append_with_ip_and_bp`: the profiler is already entered (recursion).
#[inline]
pub const fn profil_bersarang(bersarang: u32) -> i32 {
    if bersarang != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `append_with_ip_and_bp`: an unrecognised performance event type.
#[inline]
pub const fn jenis_peristiwa_salah(salah: u32) -> i32 {
    if salah != 0 {
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

abi1!(klatos_pros2_fd_tanpa_deskripsi, fd_tanpa_deskripsi, FdTanpaDeskripsi);
abi1!(klatos_pros2_thread_tak_ditemukan, thread_tak_ditemukan, ThreadTakDitemukan);
abi1!(klatos_pros2_sumber_habis, sumber_habis, SumberHabis);
abi1!(klatos_pros2_path_kosong, path_kosong, PathKosong);
abi1!(klatos_pros2_path_terlalu_panjang, path_terlalu_panjang, PathTerlaluPanjang);
abi1!(klatos_pros2_nama_perfcore_habis, nama_perfcore_habis, NamaPerfcoreHabis);
abi1!(klatos_pros2_tanpa_penyangga, tanpa_penyangga, TanpaPenyangga);
abi1!(klatos_pros2_penyangga_habis, penyangga_habis, PenyanggaHabis);
abi1!(klatos_pros2_janji_dilanggar, janji_dilanggar, JanjiDilanggar);
abi1!(klatos_pros2_konteks_id_salah, konteks_id_salah, KonteksIdSalah);
abi1!(klatos_pros2_jail_konteks, jail_konteks, JailKonteks);
abi1!(klatos_pros2_konteks_tak_ada, konteks_tak_ada, KonteksTakAda);
abi1!(klatos_pros2_path_relatif_lintas_konteks, path_relatif_lintas_konteks, PathRelatifLintasKonteks);
abi1!(klatos_pros2_custody_kosong, custody_kosong, CustodyKosong);
abi1!(klatos_pros2_bukan_direktori_custody, bukan_direktori_custody, BukanDirektoriCustody);
abi1!(klatos_pros2_tumpukan_balik_ke_kernel, tumpukan_balik_ke_kernel, TumpukanBalikKeKernel);
abi1!(klatos_pros2_baca_user_gagal, baca_user_gagal, BacaUserGagal);
abi1!(klatos_pros2_topeng_profiling_salah, topeng_profiling_salah, TopengProfilingSalah);
abi1!(klatos_pros2_profil_bersarang, profil_bersarang, ProfilBersarang);
abi1!(klatos_pros2_jenis_peristiwa_salah, jenis_peristiwa_salah, JenisPeristiwaSalah);

#[no_mangle]
pub extern "C" fn klatos_pros2_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_pros2_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiProses2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiProses2Report {
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
    x!(REPORT_TAG == 0x4B4C_4154_5052_5332);
    x!(fd_tanpa_deskripsi(1) == -EBADF && fd_tanpa_deskripsi(0) == 0);
    x!(thread_tak_ditemukan(1) == -ESRCH && thread_tak_ditemukan(0) == 0);
    x!(sumber_habis(1) == -EMFILE && sumber_habis(0) == 0);
    x!(path_kosong(1) == -ENOENT && path_kosong(0) == 0);
    x!(path_terlalu_panjang(1) == -ENAMETOOLONG && path_terlalu_panjang(0) == 0);
    x!(nama_perfcore_habis(1) == -EEXIST && nama_perfcore_habis(0) == 0);
    x!(tanpa_penyangga(1) == -ENOMEM && tanpa_penyangga(0) == 0);
    x!(penyangga_habis(1) == -ENOBUFS && penyangga_habis(0) == 0);
    x!(janji_dilanggar(1) == -EPROMISEVIOLATION && janji_dilanggar(0) == 0);
    x!(konteks_id_salah(1) == -EINVAL && konteks_id_salah(0) == 0);
    x!(jail_konteks(1) == -EPERM && jail_konteks(0) == 0);
    x!(konteks_tak_ada(1) == -EDOM && konteks_tak_ada(0) == 0);
    x!(path_relatif_lintas_konteks(1) == -EINVAL && path_relatif_lintas_konteks(0) == 0);
    x!(custody_kosong(1) == -EINVAL && custody_kosong(0) == 0);
    x!(bukan_direktori_custody(1) == -ENOTDIR && bukan_direktori_custody(0) == 0);
    x!(tumpukan_balik_ke_kernel(1) == -EFAULT && tumpukan_balik_ke_kernel(0) == 0);
    x!(baca_user_gagal(1) == -EFAULT && baca_user_gagal(0) == 0);
    x!(topeng_profiling_salah(1) == -EINVAL && topeng_profiling_salah(0) == 0);
    x!(profil_bersarang(1) == -EINVAL && profil_bersarang(0) == 0);
    x!(jenis_peristiwa_salah(1) == -EINVAL && jenis_peristiwa_salah(0) == 0);

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
        q!(fd_tanpa_deskripsi(v), if v != 0 { -EBADF } else { 0 });
        q!(thread_tak_ditemukan(v), if v != 0 { -ESRCH } else { 0 });
        q!(sumber_habis(v), if v != 0 { -EMFILE } else { 0 });
        q!(path_kosong(v), if v != 0 { -ENOENT } else { 0 });
        q!(path_terlalu_panjang(v), if v != 0 { -ENAMETOOLONG } else { 0 });
        q!(nama_perfcore_habis(v), if v != 0 { -EEXIST } else { 0 });
        q!(tanpa_penyangga(v), if v != 0 { -ENOMEM } else { 0 });
        q!(penyangga_habis(v), if v != 0 { -ENOBUFS } else { 0 });
        q!(janji_dilanggar(v), if v != 0 { -EPROMISEVIOLATION } else { 0 });
        q!(konteks_id_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(jail_konteks(v), if v != 0 { -EPERM } else { 0 });
        q!(konteks_tak_ada(v), if v != 0 { -EDOM } else { 0 });
        q!(path_relatif_lintas_konteks(v), if v != 0 { -EINVAL } else { 0 });
        q!(custody_kosong(v), if v != 0 { -EINVAL } else { 0 });
        q!(bukan_direktori_custody(v), if v != 0 { -ENOTDIR } else { 0 });
        q!(tumpukan_balik_ke_kernel(v), if v != 0 { -EFAULT } else { 0 });
        q!(baca_user_gagal(v), if v != 0 { -EFAULT } else { 0 });
        q!(topeng_profiling_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(profil_bersarang(v), if v != 0 { -EINVAL } else { 0 });
        q!(jenis_peristiwa_salah(v), if v != 0 { -EINVAL } else { 0 });
    }

    x!(d == 40);
    x!(b == 0);

    KlatosAdmisiProses2Report {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_pros2_selftest() -> KlatosAdmisiProses2Report {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 20);
        assert!(core::mem::size_of::<KlatosAdmisiProses2Report>() == 32);
    };
}
