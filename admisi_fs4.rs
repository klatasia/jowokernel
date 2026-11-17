// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS BERKAS-SISTEM, BATCH 4** (Plan9FS + FUSE +


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `ENOENT` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 2).
pub const ENOENT: i32 = 2;
/// `EINTR` — indeks 4.
pub const EINTR: i32 = 4;
/// `EIO` — indeks 5.
pub const EIO: i32 = 5;
/// `EACCES` — indeks 13.
pub const EACCES: i32 = 13;
/// `ENOTDIR` — indeks 20.
pub const ENOTDIR: i32 = 20;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `EROFS` — indeks 30.
pub const EROFS: i32 = 30;
/// `ENOTIMPL` — indeks 39.
pub const ENOTIMPL: i32 = 39;
/// `ENOTSUP` — indeks 74.
pub const ENOTSUP: i32 = 74;

/// Tag laporan; cermin `KlatosAdmisiFs4Report` di `Jowo/Kontrak/KlatSeam91.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4653_4234; // "KLATFSB4"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_fs4_plan9_tak_diimplementasi`.
    ProdPlan9TakDiimplementasi = 0,
    /// `klatos_fs4_plan9_terputus`.
    ProdPlan9Terputus = 1,
    /// `klatos_fs4_plan9_io`.
    ProdPlan9Io = 2,
    /// `klatos_fs4_fuse_io`.
    ProdFuseIo = 3,
    /// `klatos_fs4_fuse_respons`.
    ProdFuseRespons = 4,
    /// `klatos_fs4_dev_ro`.
    ProdDevRo = 5,
    /// `klatos_fs4_dev_cari`.
    ProdDevCari = 6,
    /// `klatos_fs4_dev_traverse`.
    ProdDevTraverse = 7,
    /// `klatos_fs4_unshared_operasi`.
    ProdUnsharedOperasi = 8,
    /// `klatos_fs4_unshared_traverse`.
    ProdUnsharedTraverse = 9,
    /// `klatos_fs4_unshared_akses`.
    ProdUnsharedAkses = 10,
    /// `klatos_fs4_unshared_argumen`.
    ProdUnsharedArgumen = 11,
}

const N_COUNTER: usize = 12;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// `Plan9FS::rename` + metode tulis `Plan9FSInode` belum diimplementasi.
/// Satu verdict, tujuh pemanggil. Transkripsi `Plan9FS/FileSystem.cpp:100`
/// dan `Plan9FS/Inode.cpp:242,257,263,269,275,281`.
#[inline]
pub const fn plan9_tak_diimplementasi_admisi() -> i32 {
    -ENOTIMPL
}

/// Blokir I/O 9P terputus sinyal. Satu verdict, tiga pemanggil.
/// Transkripsi `Plan9FS/FileSystem.cpp:220,238,296`.
#[inline]
pub const fn plan9_terputus_admisi() -> i32 {
    -EINTR
}

/// Balasan 9P tidak valid. Transkripsi `:243,300,318,324`.
#[inline]
pub const fn plan9_io_admisi(valid: u32) -> i32 {
    if valid == 0 {
        return -EIO;
    }
    0
}

/// Balasan FUSE terlalu pendek. Transkripsi `FUSE/Inode.cpp:90,155,220,240`.
#[inline]
pub const fn fuse_io_admisi(valid: u32) -> i32 {
    if valid == 0 {
        return -EIO;
    }
    0
}

/// Flag mount FUSE tak dikenal / balasan MKDIR-MKNOD tak valid.
/// Transkripsi `FUSE/FileSystem.cpp:47` dan `FUSE/Inode.cpp:302`.
#[inline]
pub const fn fuse_respons_admisi(valid: u32) -> i32 {
    if valid == 0 {
        return -EINVAL;
    }
    0
}

/// Berkas-sistem perangkat read-only (devpts + devloop). Satu verdict,
/// dua belas pemanggil.
/// Transkripsi `DevPtsFS/FileSystem.cpp:53`, `DevPtsFS/Inode.cpp:97-117`,
/// `DevLoopFS/FileSystem.cpp:61`, `DevLoopFS/Inode.cpp:92-112`.
#[inline]
pub const fn dev_fs_ro_admisi() -> i32 {
    -EROFS
}

/// Inode perangkat tidak ditemukan.
/// Transkripsi `DevPtsFS/FileSystem.cpp:82` dan `DevLoopFS/FileSystem.cpp:77`.
#[inline]
pub const fn dev_fs_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENOENT;
    }
    0
}

/// `traverse_as_directory` pada inode bukan direktori.
/// Transkripsi `DevPtsFS/Inode.cpp:58` dan `DevLoopFS/Inode.cpp:55`.
#[inline]
pub const fn dev_fs_traverse_admisi(adalah_dir: u32) -> i32 {
    if adalah_dir == 0 {
        return -ENOTDIR;
    }
    0
}

/// `UnsharedResourceFile` operasi tak didukung. Transkripsi `:71`.
#[inline]
pub const fn unshared_operasi_admisi() -> i32 {
    -ENOTSUP
}

/// `attach_root_filesystem_at_fd` bukan direktori. Transkripsi `:91`.
#[inline]
pub const fn unshared_traverse_admisi(adalah_dir: u32) -> i32 {
    if adalah_dir == 0 {
        return -ENOTDIR;
    }
    0
}

/// `attach_root_filesystem_at_fd` tanpa izin eksekusi. Transkripsi `:93`.
#[inline]
pub const fn unshared_akses_admisi(diizinkan: u32) -> i32 {
    if diizinkan == 0 {
        return -EACCES;
    }
    0
}

/// Permintaan ioctl `UnsharedResourceFile` tak dikenal. Transkripsi `:101`.
#[inline]
pub const fn unshared_argumen_admisi(valid: u32) -> i32 {
    if valid == 0 {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_plan9_io_admisi(valid: u32) -> i32 {
    match valid {
        1 => 0,
        _ => -EIO,
    }
}
fn naif_fuse_io_admisi(valid: u32) -> i32 {
    match valid {
        1 => 0,
        _ => -EIO,
    }
}
fn naif_fuse_respons_admisi(valid: u32) -> i32 {
    match valid {
        1 => 0,
        _ => -EINVAL,
    }
}
fn naif_dev_fs_cari_admisi(ditemukan: u32) -> i32 {
    match ditemukan {
        0 => -ENOENT,
        _ => 0,
    }
}
fn naif_dev_fs_traverse_admisi(adalah_dir: u32) -> i32 {
    match adalah_dir {
        0 => -ENOTDIR,
        _ => 0,
    }
}
fn naif_unshared_traverse_admisi(adalah_dir: u32) -> i32 {
    match adalah_dir {
        1 => 0,
        _ => -ENOTDIR,
    }
}
fn naif_unshared_akses_admisi(diizinkan: u32) -> i32 {
    match diizinkan {
        0 => -EACCES,
        _ => 0,
    }
}
fn naif_unshared_argumen_admisi(valid: u32) -> i32 {
    match valid {
        0 => -EINVAL,
        _ => 0,
    }
}

/// Kontrol NEGATIF: `unshared_akses_admisi` yang salah mengembalikan `EPERM`
/// (bukan `EACCES`). Sengaja SALAH; berdaya pada `diizinkan == 0`.
fn unshared_akses_salah(diizinkan: u32) -> i32 {
    if diizinkan == 0 {
        return -1; // EPERM
    }
    0
}

/// Kontrol NEGATIF: `dev_fs_traverse_admisi` yang salah mengembalikan `ENOENT`
/// (bukan `ENOTDIR`). Sengaja SALAH; berdaya pada `adalah_dir == 0`.
fn dev_fs_traverse_salah(adalah_dir: u32) -> i32 {
    if adalah_dir == 0 {
        return -ENOENT;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`plan9_tak_diimplementasi_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_plan9_tak_diimplementasi() -> i32 {
    bump(Counter::ProdPlan9TakDiimplementasi);
    plan9_tak_diimplementasi_admisi()
}

/// Lihat [`plan9_terputus_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_plan9_terputus() -> i32 {
    bump(Counter::ProdPlan9Terputus);
    plan9_terputus_admisi()
}

/// Lihat [`plan9_io_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_plan9_io(valid: u32) -> i32 {
    bump(Counter::ProdPlan9Io);
    plan9_io_admisi(valid)
}

/// Lihat [`fuse_io_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_fuse_io(valid: u32) -> i32 {
    bump(Counter::ProdFuseIo);
    fuse_io_admisi(valid)
}

/// Lihat [`fuse_respons_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_fuse_respons(valid: u32) -> i32 {
    bump(Counter::ProdFuseRespons);
    fuse_respons_admisi(valid)
}

/// Lihat [`dev_fs_ro_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_dev_ro() -> i32 {
    bump(Counter::ProdDevRo);
    dev_fs_ro_admisi()
}

/// Lihat [`dev_fs_cari_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_dev_cari(ditemukan: u32) -> i32 {
    bump(Counter::ProdDevCari);
    dev_fs_cari_admisi(ditemukan)
}

/// Lihat [`dev_fs_traverse_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_dev_traverse(adalah_dir: u32) -> i32 {
    bump(Counter::ProdDevTraverse);
    dev_fs_traverse_admisi(adalah_dir)
}

/// Lihat [`unshared_operasi_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_unshared_operasi() -> i32 {
    bump(Counter::ProdUnsharedOperasi);
    unshared_operasi_admisi()
}

/// Lihat [`unshared_traverse_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_unshared_traverse(adalah_dir: u32) -> i32 {
    bump(Counter::ProdUnsharedTraverse);
    unshared_traverse_admisi(adalah_dir)
}

/// Lihat [`unshared_akses_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_unshared_akses(diizinkan: u32) -> i32 {
    bump(Counter::ProdUnsharedAkses);
    unshared_akses_admisi(diizinkan)
}

/// Lihat [`unshared_argumen_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs4_unshared_argumen(valid: u32) -> i32 {
    bump(Counter::ProdUnsharedArgumen);
    unshared_argumen_admisi(valid)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_fs4_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_fs4_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiFs4Report` di `Jowo/Kontrak/KlatSeam91.h`.
#[repr(C)]
pub struct KlatosAdmisiFs4Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub jaring_checks: u32,
    pub jaring_passed: u32,
    pub perangkat_checks: u32,
    pub perangkat_passed: u32,
    pub prod_plan9_tak_diimplementasi: u32,
    pub prod_plan9_terputus: u32,
    pub prod_plan9_io: u32,
    pub prod_fuse_io: u32,
    pub prod_fuse_respons: u32,
    pub prod_dev_ro: u32,
    pub prod_dev_cari: u32,
    pub prod_dev_traverse: u32,
    pub prod_unshared_operasi: u32,
    pub prod_unshared_traverse: u32,
    pub prod_unshared_akses: u32,
    pub prod_unshared_argumen: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Jaring,
    Perangkat,
}

struct Matrix {
    checks: u32,
    passed: u32,
    jaring_checks: u32,
    jaring_passed: u32,
    perangkat_checks: u32,
    perangkat_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            jaring_checks: 0,
            jaring_passed: 0,
            perangkat_checks: 0,
            perangkat_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Jaring => {
                self.jaring_checks += 1;
                if lulus {
                    self.jaring_passed += 1;
                }
            }
            Group::Perangkat => {
                self.perangkat_checks += 1;
                if lulus {
                    self.perangkat_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **20**.
pub const DIFF_EXPECTED: u32 = 20;

fn run_matrix() -> KlatosAdmisiFs4Report {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4653_4234, Group::Jaring);
    m.catat(
        ENOENT == 2 && EINTR == 4 && EIO == 5 && EACCES == 13 && ENOTDIR == 20 && EINVAL == 22
            && EROFS == 30 && ENOTIMPL == 39 && ENOTSUP == 74,
        Group::Jaring,
    );

    // --- kasus utama ------------------------------------------------------
    m.catat(plan9_tak_diimplementasi_admisi() == -ENOTIMPL, Group::Jaring);
    m.catat(plan9_terputus_admisi() == -EINTR, Group::Jaring);
    m.catat(plan9_io_admisi(0) == -EIO, Group::Jaring);
    m.catat(plan9_io_admisi(1) == 0, Group::Jaring);
    m.catat(fuse_io_admisi(0) == -EIO, Group::Jaring);
    m.catat(fuse_respons_admisi(0) == -EINVAL, Group::Jaring);
    m.catat(fuse_respons_admisi(1) == 0, Group::Jaring);

    m.catat(dev_fs_ro_admisi() == -EROFS, Group::Perangkat);
    m.catat(dev_fs_cari_admisi(0) == -ENOENT, Group::Perangkat);
    m.catat(dev_fs_cari_admisi(1) == 0, Group::Perangkat);
    m.catat(dev_fs_traverse_admisi(0) == -ENOTDIR, Group::Perangkat);
    m.catat(dev_fs_traverse_admisi(1) == 0, Group::Perangkat);
    m.catat(unshared_operasi_admisi() == -ENOTSUP, Group::Perangkat);
    m.catat(unshared_traverse_admisi(0) == -ENOTDIR, Group::Perangkat);
    m.catat(unshared_akses_admisi(0) == -EACCES, Group::Perangkat);
    m.catat(unshared_akses_admisi(1) == 0, Group::Perangkat);
    m.catat(unshared_argumen_admisi(0) == -EINVAL, Group::Perangkat);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // EACCES bukan EPERM pada izin eksekusi hilang.
        m.catat(unshared_akses_admisi(0) == -EACCES, Group::Perangkat);
        m.catat(unshared_akses_salah(0) == -1, Group::Perangkat);
        m.catat(unshared_akses_admisi(0) != unshared_akses_salah(0), Group::Perangkat);
        m.catat(unshared_akses_admisi(1) == unshared_akses_salah(1), Group::Perangkat);

        // ENOTDIR bukan ENOENT pada traverse inode non-dir.
        m.catat(dev_fs_traverse_admisi(0) == -ENOTDIR, Group::Perangkat);
        m.catat(dev_fs_traverse_salah(0) == -ENOENT, Group::Perangkat);
        m.catat(dev_fs_traverse_admisi(0) != dev_fs_traverse_salah(0), Group::Perangkat);
        m.catat(dev_fs_traverse_admisi(1) == dev_fs_traverse_salah(1), Group::Perangkat);

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if plan9_tak_diimplementasi_admisi() != -ENOTIMPL {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Jaring);
    }

    // --- diferensial produksi vs oracle (20 kasus) ------------------------
    macro_rules! ukur {
        ($g:expr, $p:expr, $n:expr) => {{
            if $p != $n {
                beda += 1;
            }
            diukur += 1;
            let _ = $g;
        }};
    }

    for v in 0..2u32 {
        ukur!(Group::Jaring, plan9_io_admisi(v), naif_plan9_io_admisi(v));
        ukur!(Group::Jaring, fuse_io_admisi(v), naif_fuse_io_admisi(v));
        ukur!(Group::Jaring, fuse_respons_admisi(v), naif_fuse_respons_admisi(v));
        ukur!(Group::Perangkat, dev_fs_cari_admisi(v), naif_dev_fs_cari_admisi(v));
        ukur!(Group::Perangkat, dev_fs_traverse_admisi(v), naif_dev_fs_traverse_admisi(v));
        ukur!(Group::Perangkat, unshared_traverse_admisi(v), naif_unshared_traverse_admisi(v));
        ukur!(Group::Perangkat, unshared_akses_admisi(v), naif_unshared_akses_admisi(v));
        ukur!(Group::Perangkat, unshared_argumen_admisi(v), naif_unshared_argumen_admisi(v));
    }
    ukur!(Group::Jaring, plan9_tak_diimplementasi_admisi(), -ENOTIMPL);
    ukur!(Group::Jaring, plan9_terputus_admisi(), -EINTR);
    ukur!(Group::Perangkat, dev_fs_ro_admisi(), -EROFS);
    ukur!(Group::Perangkat, unshared_operasi_admisi(), -ENOTSUP);

    m.catat(diukur == DIFF_EXPECTED, Group::Jaring);
    m.catat(beda == 0, Group::Jaring);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiFs4Report {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        jaring_checks: m.jaring_checks,
        jaring_passed: m.jaring_passed,
        perangkat_checks: m.perangkat_checks,
        perangkat_passed: m.perangkat_passed,
        prod_plan9_tak_diimplementasi: klatos_fs4_counter(Counter::ProdPlan9TakDiimplementasi as u32),
        prod_plan9_terputus: klatos_fs4_counter(Counter::ProdPlan9Terputus as u32),
        prod_plan9_io: klatos_fs4_counter(Counter::ProdPlan9Io as u32),
        prod_fuse_io: klatos_fs4_counter(Counter::ProdFuseIo as u32),
        prod_fuse_respons: klatos_fs4_counter(Counter::ProdFuseRespons as u32),
        prod_dev_ro: klatos_fs4_counter(Counter::ProdDevRo as u32),
        prod_dev_cari: klatos_fs4_counter(Counter::ProdDevCari as u32),
        prod_dev_traverse: klatos_fs4_counter(Counter::ProdDevTraverse as u32),
        prod_unshared_operasi: klatos_fs4_counter(Counter::ProdUnsharedOperasi as u32),
        prod_unshared_traverse: klatos_fs4_counter(Counter::ProdUnsharedTraverse as u32),
        prod_unshared_akses: klatos_fs4_counter(Counter::ProdUnsharedAkses as u32),
        prod_unshared_argumen: klatos_fs4_counter(Counter::ProdUnsharedArgumen as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas berkas-sistem batch 4.
#[no_mangle]
pub extern "C" fn klatos_fs4_selftest() -> KlatosAdmisiFs4Report {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4653_4234);
        assert!(ENOENT == 2);
        assert!(EINTR == 4);
        assert!(EIO == 5);
        assert!(EACCES == 13);
        assert!(ENOTDIR == 20);
        assert!(EINVAL == 22);
        assert!(EROFS == 30);
        assert!(ENOTIMPL == 39);
        assert!(ENOTSUP == 74);
        assert!(Counter::ProdPlan9TakDiimplementasi as usize == 0);
        assert!(Counter::ProdUnsharedArgumen as usize == 11);
        assert!(N_COUNTER == 12);
        assert!(DIFF_EXPECTED == 20);

        assert!(plan9_tak_diimplementasi_admisi() == -ENOTIMPL);
        assert!(unshared_akses_admisi(0) == -EACCES);
        assert!(dev_fs_traverse_admisi(0) == -ENOTDIR);

        // 8 (tag) + 20*4 (u32) + 4 (status) = 92 → 96 dengan padding 8-byte.
        assert!(core::mem::size_of::<KlatosAdmisiFs4Report>() == 96);
        assert!(core::mem::align_of::<KlatosAdmisiFs4Report>() == 8);
    };
}
