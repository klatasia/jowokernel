// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


//! P2.09 — generic syscall admission residuals (Explore batch 7).


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EPERM: i32 = 1;
pub const EBADF: i32 = 9;
pub const EACCES: i32 = 13;
pub const EFAULT: i32 = 14;
pub const ENODEV: i32 = 19;
pub const ENOTDIR: i32 = 20;
pub const EISDIR: i32 = 21;
pub const EINVAL: i32 = 22;
pub const EFBIG: i32 = 27;
pub const ESPIPE: i32 = 29;
pub const ENAMETOOLONG: i32 = 34;
pub const ENOBUFS: i32 = 59;

pub const REPORT_TAG: u64 = 0x4B4C_4154_5041_4E47; // KLATPANG

#[derive(Clone, Copy)]
pub enum Counter {
    BukanTerbaca = 0,
    DirektoriUntukBaca = 1,
    IovNegatif = 2,
    IovLuarBatas = 3,
    PanjangIovLuber = 4,
    SeekTakDidukungSaatMenulis = 5,
    UkuranLuber = 6,
    OffsetNegatif = 7,
    TakBisaSeek = 8,
    BukanTertulis = 9,
    PanjangFallocateSalah = 10,
    UkuranFallocateLuber = 11,
    FifoFallocate = 12,
    BukanBerkasReguler = 13,
    BukanSuperuser = 14,
    JamBukanRealtime = 15,
    BenderaJamSalah = 16,
    ChdirBukanDirektori = 17,
    ChdirTakBolehEksekusi = 18,
    NamaPetaPanjang = 19,
    NfdsTerlaluBanyak = 20,
    NfdsLuber = 21,
    ArgFdNegatif = 22,
    PerintahTakDitangani = 23,
}

const N_COUNTER: usize = 24;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// read: the description is not readable.
#[inline]
pub const fn bukan_terbaca(bukan: u32) -> i32 {
    if bukan != 0 {
        -EBADF
    } else {
        0
    }
}

/// read: the description is a directory.
#[inline]
pub const fn direktori_untuk_baca(dir: u32) -> i32 {
    if dir != 0 {
        -EISDIR
    } else {
        0
    }
}

/// read/write: a negative iovec count.
#[inline]
pub const fn iov_negatif(negatif: u32) -> i32 {
    if negatif != 0 {
        -EINVAL
    } else {
        0
    }
}

/// read/write: an iovec count above `IOV_MAX`.
#[inline]
pub const fn iov_luar_batas(luar: u32) -> i32 {
    if luar != 0 {
        -EFAULT
    } else {
        0
    }
}

/// read/write: the summed iovec length exceeds `NumericLimits<i32>::max()`.
#[inline]
pub const fn panjang_iov_luber(luber: u32) -> i32 {
    if luber != 0 {
        -EINVAL
    } else {
        0
    }
}

/// read/write: a base offset was given but the description is not seekable.
#[inline]
pub const fn seek_tak_didukung_saat_menulis(tak: u32) -> i32 {
    if tak != 0 {
        -ESPIPE
    } else {
        0
    }
}

/// A size exceeds `NumericLimits<ssize_t>::max()` (read/write/chdir).
#[inline]
pub const fn ukuran_luber(luber: u32) -> i32 {
    if luber != 0 {
        -EINVAL
    } else {
        0
    }
}

/// read/fallocate: a negative offset.
#[inline]
pub const fn offset_negatif(negatif: u32) -> i32 {
    if negatif != 0 {
        -EINVAL
    } else {
        0
    }
}

/// read: `pread`-style access on a non-seekable description.
#[inline]
pub const fn tak_bisa_seek(tak: u32) -> i32 {
    if tak != 0 {
        -EINVAL
    } else {
        0
    }
}

/// write/fallocate: the description is not writable.
#[inline]
pub const fn bukan_tertulis(bukan: u32) -> i32 {
    if bukan != 0 {
        -EBADF
    } else {
        0
    }
}

/// fallocate: a non-positive length.
#[inline]
pub const fn panjang_fallocate_salah(salah: u32) -> i32 {
    if salah != 0 {
        -EINVAL
    } else {
        0
    }
}

/// fallocate: `Checked` offset+length overflow.
#[inline]
pub const fn ukuran_fallocate_luber(luber: u32) -> i32 {
    if luber != 0 {
        -EFBIG
    } else {
        0
    }
}

/// fallocate: the description is a FIFO.
#[inline]
pub const fn fifo_fallocate(fifo: u32) -> i32 {
    if fifo != 0 {
        -ESPIPE
    } else {
        0
    }
}

/// fallocate: the description is not a regular file.
#[inline]
pub const fn bukan_berkas_reguler(bukan: u32) -> i32 {
    if bukan != 0 {
        -ENODEV
    } else {
        0
    }
}

/// clock/keymap: the caller is not the superuser.
#[inline]
pub const fn bukan_superuser(bukan: u32) -> i32 {
    if bukan != 0 {
        -EPERM
    } else {
        0
    }
}

/// `clock_settime`: a clock id other than `CLOCK_REALTIME`.
#[inline]
pub const fn jam_bukan_realtime(bukan: u32) -> i32 {
    if bukan != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `clock_nanosleep`: flags other than 0 / `TIMER_ABSTIME`.
#[inline]
pub const fn bendera_jam_salah(salah: u32) -> i32 {
    if salah != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `chdir`: the target is not a directory.
#[inline]
pub const fn chdir_bukan_direktori(bukan: u32) -> i32 {
    if bukan != 0 {
        -ENOTDIR
    } else {
        0
    }
}

/// `chdir`: the caller may not execute the target directory.
#[inline]
pub const fn chdir_tak_boleh_eksekusi(tak: u32) -> i32 {
    if tak != 0 {
        -EACCES
    } else {
        0
    }
}

/// keymap: the map name is longer than `map_name_max_size`.
#[inline]
pub const fn nama_peta_panjang(panjang: u32) -> i32 {
    if panjang != 0 {
        -ENAMETOOLONG
    } else {
        0
    }
}

/// `poll`: more fds than the process may open.
#[inline]
pub const fn nfds_terlalu_banyak(banyak: u32) -> i32 {
    if banyak != 0 {
        -ENOBUFS
    } else {
        0
    }
}

/// `poll`: the fd-count arithmetic overflowed.
#[inline]
pub const fn nfds_luber(luber: u32) -> i32 {
    if luber != 0 {
        -EFAULT
    } else {
        0
    }
}

/// `fcntl`: a negative argument fd for `F_DUPFD`/`F_DUPFD_CLOEXEC`.
#[inline]
pub const fn arg_fd_negatif(negatif: u32) -> i32 {
    if negatif != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `fcntl`: a command the C++ switch does not handle (see F-P209-01).
#[inline]
pub const fn perintah_tak_ditangani(tak: u32) -> i32 {
    if tak != 0 {
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

abi1!(klatos_pang_bukan_terbaca, bukan_terbaca, BukanTerbaca);
abi1!(klatos_pang_direktori_untuk_baca, direktori_untuk_baca, DirektoriUntukBaca);
abi1!(klatos_pang_iov_negatif, iov_negatif, IovNegatif);
abi1!(klatos_pang_iov_luar_batas, iov_luar_batas, IovLuarBatas);
abi1!(klatos_pang_panjang_iov_luber, panjang_iov_luber, PanjangIovLuber);
abi1!(klatos_pang_seek_tak_didukung_saat_menulis, seek_tak_didukung_saat_menulis, SeekTakDidukungSaatMenulis);
abi1!(klatos_pang_ukuran_luber, ukuran_luber, UkuranLuber);
abi1!(klatos_pang_offset_negatif, offset_negatif, OffsetNegatif);
abi1!(klatos_pang_tak_bisa_seek, tak_bisa_seek, TakBisaSeek);
abi1!(klatos_pang_bukan_tertulis, bukan_tertulis, BukanTertulis);
abi1!(klatos_pang_panjang_fallocate_salah, panjang_fallocate_salah, PanjangFallocateSalah);
abi1!(klatos_pang_ukuran_fallocate_luber, ukuran_fallocate_luber, UkuranFallocateLuber);
abi1!(klatos_pang_fifo_fallocate, fifo_fallocate, FifoFallocate);
abi1!(klatos_pang_bukan_berkas_reguler, bukan_berkas_reguler, BukanBerkasReguler);
abi1!(klatos_pang_bukan_superuser, bukan_superuser, BukanSuperuser);
abi1!(klatos_pang_jam_bukan_realtime, jam_bukan_realtime, JamBukanRealtime);
abi1!(klatos_pang_bendera_jam_salah, bendera_jam_salah, BenderaJamSalah);
abi1!(klatos_pang_chdir_bukan_direktori, chdir_bukan_direktori, ChdirBukanDirektori);
abi1!(klatos_pang_chdir_tak_boleh_eksekusi, chdir_tak_boleh_eksekusi, ChdirTakBolehEksekusi);
abi1!(klatos_pang_nama_peta_panjang, nama_peta_panjang, NamaPetaPanjang);
abi1!(klatos_pang_nfds_terlalu_banyak, nfds_terlalu_banyak, NfdsTerlaluBanyak);
abi1!(klatos_pang_nfds_luber, nfds_luber, NfdsLuber);
abi1!(klatos_pang_arg_fd_negatif, arg_fd_negatif, ArgFdNegatif);
abi1!(klatos_pang_perintah_tak_ditangani, perintah_tak_ditangani, PerintahTakDitangani);

#[no_mangle]
pub extern "C" fn klatos_pang_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_pang_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiPanggilanReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiPanggilanReport {
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
    x!(REPORT_TAG == 0x4B4C_4154_5041_4E47);
    x!(bukan_terbaca(1) == -EBADF && bukan_terbaca(0) == 0);
    x!(direktori_untuk_baca(1) == -EISDIR && direktori_untuk_baca(0) == 0);
    x!(iov_negatif(1) == -EINVAL && iov_negatif(0) == 0);
    x!(iov_luar_batas(1) == -EFAULT && iov_luar_batas(0) == 0);
    x!(panjang_iov_luber(1) == -EINVAL && panjang_iov_luber(0) == 0);
    x!(seek_tak_didukung_saat_menulis(1) == -ESPIPE && seek_tak_didukung_saat_menulis(0) == 0);
    x!(ukuran_luber(1) == -EINVAL && ukuran_luber(0) == 0);
    x!(offset_negatif(1) == -EINVAL && offset_negatif(0) == 0);
    x!(tak_bisa_seek(1) == -EINVAL && tak_bisa_seek(0) == 0);
    x!(bukan_tertulis(1) == -EBADF && bukan_tertulis(0) == 0);
    x!(panjang_fallocate_salah(1) == -EINVAL && panjang_fallocate_salah(0) == 0);
    x!(ukuran_fallocate_luber(1) == -EFBIG && ukuran_fallocate_luber(0) == 0);
    x!(fifo_fallocate(1) == -ESPIPE && fifo_fallocate(0) == 0);
    x!(bukan_berkas_reguler(1) == -ENODEV && bukan_berkas_reguler(0) == 0);
    x!(bukan_superuser(1) == -EPERM && bukan_superuser(0) == 0);
    x!(jam_bukan_realtime(1) == -EINVAL && jam_bukan_realtime(0) == 0);
    x!(bendera_jam_salah(1) == -EINVAL && bendera_jam_salah(0) == 0);
    x!(chdir_bukan_direktori(1) == -ENOTDIR && chdir_bukan_direktori(0) == 0);
    x!(chdir_tak_boleh_eksekusi(1) == -EACCES && chdir_tak_boleh_eksekusi(0) == 0);
    x!(nama_peta_panjang(1) == -ENAMETOOLONG && nama_peta_panjang(0) == 0);
    x!(nfds_terlalu_banyak(1) == -ENOBUFS && nfds_terlalu_banyak(0) == 0);
    x!(nfds_luber(1) == -EFAULT && nfds_luber(0) == 0);
    x!(arg_fd_negatif(1) == -EINVAL && arg_fd_negatif(0) == 0);
    x!(perintah_tak_ditangani(1) == -EINVAL && perintah_tak_ditangani(0) == 0);

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
        q!(bukan_terbaca(v), if v != 0 { -EBADF } else { 0 });
        q!(direktori_untuk_baca(v), if v != 0 { -EISDIR } else { 0 });
        q!(iov_negatif(v), if v != 0 { -EINVAL } else { 0 });
        q!(iov_luar_batas(v), if v != 0 { -EFAULT } else { 0 });
        q!(panjang_iov_luber(v), if v != 0 { -EINVAL } else { 0 });
        q!(seek_tak_didukung_saat_menulis(v), if v != 0 { -ESPIPE } else { 0 });
        q!(ukuran_luber(v), if v != 0 { -EINVAL } else { 0 });
        q!(offset_negatif(v), if v != 0 { -EINVAL } else { 0 });
        q!(tak_bisa_seek(v), if v != 0 { -EINVAL } else { 0 });
        q!(bukan_tertulis(v), if v != 0 { -EBADF } else { 0 });
        q!(panjang_fallocate_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(ukuran_fallocate_luber(v), if v != 0 { -EFBIG } else { 0 });
        q!(fifo_fallocate(v), if v != 0 { -ESPIPE } else { 0 });
        q!(bukan_berkas_reguler(v), if v != 0 { -ENODEV } else { 0 });
        q!(bukan_superuser(v), if v != 0 { -EPERM } else { 0 });
        q!(jam_bukan_realtime(v), if v != 0 { -EINVAL } else { 0 });
        q!(bendera_jam_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(chdir_bukan_direktori(v), if v != 0 { -ENOTDIR } else { 0 });
        q!(chdir_tak_boleh_eksekusi(v), if v != 0 { -EACCES } else { 0 });
        q!(nama_peta_panjang(v), if v != 0 { -ENAMETOOLONG } else { 0 });
        q!(nfds_terlalu_banyak(v), if v != 0 { -ENOBUFS } else { 0 });
        q!(nfds_luber(v), if v != 0 { -EFAULT } else { 0 });
        q!(arg_fd_negatif(v), if v != 0 { -EINVAL } else { 0 });
        q!(perintah_tak_ditangani(v), if v != 0 { -EINVAL } else { 0 });
    }

    x!(d == 48);
    x!(b == 0);

    KlatosAdmisiPanggilanReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_pang_selftest() -> KlatosAdmisiPanggilanReport {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 24);
        assert!(core::mem::size_of::<KlatosAdmisiPanggilanReport>() == 32);
    };
}
