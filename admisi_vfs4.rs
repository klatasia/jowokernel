// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.86 / WS4 — **OTORITAS ADMISI VFS, BATCH 4**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `ENOENT` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 2).
pub const ENOENT: i32 = 2;
/// `EIO` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 5).
pub const EIO: i32 = 5;
/// `EBADF` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 9).
pub const EBADF: i32 = 9;
/// `ENODEV` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 19).
pub const ENODEV: i32 = 19;
/// `ENOTDIR` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 20).
pub const ENOTDIR: i32 = 20;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `EOVERFLOW` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 36).
pub const EOVERFLOW: i32 = 36;

/// Tag laporan; cermin `KlatosAdmisiVfs4Report` di `Jowo/Kontrak/KlatSeam87.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5646_5334; // "KLATVFS4"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_vfs4_copy_mount` dari produksi.
    ProdCopyMount = 0,
    /// `klatos_vfs4_buka_perangkat` dari produksi.
    ProdBukaPerangkat = 1,
    /// `klatos_vfs4_resolusi_path` dari produksi.
    ProdResolusiPath = 2,
    /// `klatos_vfs4_mount_cari` dari produksi.
    ProdMountCari = 3,
    /// `klatos_vfs4_ofd_rentang` dari produksi.
    ProdOfdRentang = 4,
    /// `klatos_vfs4_ofd_dir_entries` dari produksi.
    ProdOfdDir = 5,
    /// `klatos_vfs4_ofd_flock` dari produksi.
    ProdOfdFlock = 6,
    /// `klatos_vfs4_ofd_custody` dari produksi.
    ProdOfdCustody = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Admisi `VirtualFileSystem::copy_mount`. Transkripsi `:292-296`:
///
/// ```cpp
/// if (&original_custody.inode() != &original_custody.inode().fs().root_inode())
///     return EINVAL;
/// ```
///
/// `adalah_root` adalah FAKTA identitas inode yang dihitung C++.
/// Mengembalikan `0` (boleh) atau `-EINVAL`.
#[inline]
pub const fn copy_mount_admisi(adalah_root: u32) -> i32 {
    if adalah_root == 0 {
        return -EINVAL;
    }
    0
}

/// Admisi `VirtualFileSystem::open` cabang perangkat. Transkripsi `:508-516`:
///
/// ```cpp
/// Device::run_by_type_and_major_minor_numbers(..., [&](RefPtr<Device> found) { device = move(found); });
/// if (device == nullptr) return ENODEV;
/// ```
///
/// Pencarian perangkat tetap MEKANISME; yang dipindahkan adalah VERDICT
/// "tidak ada perangkat terdaftar". Mengembalikan `0` atau `-ENODEV`.
#[inline]
pub const fn buka_perangkat_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENODEV;
    }
    0
}

/// Admisi `VirtualFileSystem::resolve_path_without_veil` jalur kosong.
/// Transkripsi `:1328-1329`:
///
/// ```cpp
/// if (path.is_empty()) return EINVAL;
/// ```
///
/// Berada SESUDAH cek rekursi `ELOOP` (P1.84). Mengembalikan `0` atau `-EINVAL`.
#[inline]
pub const fn resolusi_path_admisi(path_kosong: u32) -> i32 {
    if path_kosong != 0 {
        return -EINVAL;
    }
    0
}

/// Admisi pencarian mount `VFSRootContext`. Transkripsi `:310-323` dan
/// `:341-349`:
///
/// ```cpp
/// for (auto& mount : details.mounts) { if (...) { callback(mount); return {}; } }
/// return Error::from_errno(ENODEV);   // / return ENODEV;
/// ```
///
/// `ditemukan` adalah FAKTA "ada entri mount yang cocok". Mengembalikan `0`
/// atau `-ENODEV`.
#[inline]
pub const fn mount_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENODEV;
    }
    0
}

/// Admisi rentang baca/tulis `OpenFileDescription`. Transkripsi `:148-149`,
/// `:155-156`, `:163-164`, `:177-178`:
///
/// ```cpp
/// if (Checked<u64>::addition_would_overflow(offset, count)) return EOVERFLOW;
/// ```
///
/// `meluap` adalah FAKTA predikat aritmetika yang dihitung C++.
/// Mengembalikan `0` atau `-EOVERFLOW`.
#[inline]
pub const fn ofd_rentang_admisi(meluap: u32) -> i32 {
    if meluap != 0 {
        return -EOVERFLOW;
    }
    0
}

/// Admisi `OpenFileDescription::get_dir_entries`. Transkripsi `:200-207`:
///
/// ```cpp
/// if (!is_directory()) return ENOTDIR;
/// auto metadata = this->metadata();
/// if (!metadata.is_valid()) return EIO;
/// ```
///
/// Urutan kebijakan: `ENOTDIR` menang atas `EIO`. Mengembalikan `0`,
/// `-ENOTDIR`, atau `-EIO`.
#[inline]
pub const fn ofd_dir_entries_admisi(adalah_dir: u32, metadata_valid: u32) -> i32 {
    if adalah_dir == 0 {
        return -ENOTDIR;
    }
    if metadata_valid == 0 {
        return -EIO;
    }
    0
}

/// Admisi `OpenFileDescription::apply_flock`/`get_flock`. Transkripsi
/// `:469-480`:
///
/// ```cpp
/// if (!m_inode) return EBADF;
/// ```
///
/// Mengembalikan `0` atau `-EBADF`.
#[inline]
pub const fn ofd_flock_admisi(punya_inode: u32) -> i32 {
    if punya_inode == 0 {
        return -EBADF;
    }
    0
}

/// Admisi `OpenFileDescription::original_absolute_path`. Transkripsi `:377-382`:
///
/// ```cpp
/// if (auto custody = this->custody()) return custody->try_serialize_absolute_path();
/// return ENOENT;
/// ```
///
/// Mengembalikan `0` atau `-ENOENT`.
#[inline]
pub const fn ofd_custody_admisi(punya_custody: u32) -> i32 {
    if punya_custody == 0 {
        return -ENOENT;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_copy_mount_admisi(adalah_root: u32) -> i32 {
    match adalah_root {
        0 => -EINVAL,
        _ => 0,
    }
}

fn naif_buka_perangkat_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 1 {
        0
    } else {
        -ENODEV
    }
}

fn naif_resolusi_path_admisi(path_kosong: u32) -> i32 {
    match path_kosong {
        0 => 0,
        _ => -EINVAL,
    }
}

fn naif_mount_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        -ENODEV
    } else {
        0
    }
}

fn naif_ofd_rentang_admisi(meluap: u32) -> i32 {
    if meluap == 1 {
        -EOVERFLOW
    } else {
        0
    }
}

fn naif_ofd_dir_entries_admisi(adalah_dir: u32, metadata_valid: u32) -> i32 {
    let mut e = 0;
    if adalah_dir == 0 {
        e = -ENOTDIR;
    } else if metadata_valid == 0 {
        e = -EIO;
    }
    e
}

fn naif_ofd_flock_admisi(punya_inode: u32) -> i32 {
    match punya_inode {
        0 => -EBADF,
        _ => 0,
    }
}

fn naif_ofd_custody_admisi(punya_custody: u32) -> i32 {
    match punya_custody {
        0 => -ENOENT,
        _ => 0,
    }
}

/// Kontrol NEGATIF: `copy_mount_admisi` yang MENJATUHKAN penjaga `EINVAL`.
/// Sengaja SALAH; berdaya pada `adalah_root == 0`.
fn copy_mount_salah(_adalah_root: u32) -> i32 {
    0
}

/// Kontrol NEGATIF: `mount_cari_admisi` yang MENJATUHKAN penjaga `ENODEV`.
/// Sengaja SALAH; berdaya pada `ditemukan == 0`.
fn mount_cari_salah(_ditemukan: u32) -> i32 {
    0
}

/// Kontrol NEGATIF: `ofd_dir_entries_admisi` yang MENJATUHKAN penjaga
/// `ENOTDIR`. Sengaja SALAH; berdaya pada `adalah_dir == 0`.
fn ofd_dir_entries_salah(_adalah_dir: u32, metadata_valid: u32) -> i32 {
    if metadata_valid == 0 {
        return -EIO;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Admisi `copy_mount()`. Lihat [`copy_mount_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_copy_mount(adalah_root: u32) -> i32 {
    bump(Counter::ProdCopyMount);
    copy_mount_admisi(adalah_root)
}

/// Admisi perangkat tak terdaftar pada `open()`. Lihat [`buka_perangkat_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_buka_perangkat(ditemukan: u32) -> i32 {
    bump(Counter::ProdBukaPerangkat);
    buka_perangkat_admisi(ditemukan)
}

/// Admisi jalur kosong. Lihat [`resolusi_path_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_resolusi_path(path_kosong: u32) -> i32 {
    bump(Counter::ProdResolusiPath);
    resolusi_path_admisi(path_kosong)
}

/// Admisi pencarian mount. Lihat [`mount_cari_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_mount_cari(ditemukan: u32) -> i32 {
    bump(Counter::ProdMountCari);
    mount_cari_admisi(ditemukan)
}

/// Admisi rentang baca/tulis. Lihat [`ofd_rentang_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_ofd_rentang(meluap: u32) -> i32 {
    bump(Counter::ProdOfdRentang);
    ofd_rentang_admisi(meluap)
}

/// Admisi `get_dir_entries()`. Lihat [`ofd_dir_entries_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_ofd_dir_entries(adalah_dir: u32, metadata_valid: u32) -> i32 {
    bump(Counter::ProdOfdDir);
    ofd_dir_entries_admisi(adalah_dir, metadata_valid)
}

/// Admisi `flock` tanpa inode. Lihat [`ofd_flock_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_ofd_flock(punya_inode: u32) -> i32 {
    bump(Counter::ProdOfdFlock);
    ofd_flock_admisi(punya_inode)
}

/// Admisi `original_absolute_path()` tanpa custody. Lihat [`ofd_custody_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_ofd_custody(punya_custody: u32) -> i32 {
    bump(Counter::ProdOfdCustody);
    ofd_custody_admisi(punya_custody)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_vfs4_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_vfs4_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiVfs4Report` di `Jowo/Kontrak/KlatSeam87.h`.
#[repr(C)]
pub struct KlatosAdmisiVfs4Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub struktur_checks: u32,
    pub struktur_passed: u32,
    pub ofd_checks: u32,
    pub ofd_passed: u32,
    pub prod_copy_mount: u32,
    pub prod_buka_perangkat: u32,
    pub prod_resolusi_path: u32,
    pub prod_mount_cari: u32,
    pub prod_ofd_rentang: u32,
    pub prod_ofd_dir: u32,
    pub prod_ofd_flock: u32,
    pub prod_ofd_custody: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Struktur,
    Ofd,
}

struct Matrix {
    checks: u32,
    passed: u32,
    struktur_checks: u32,
    struktur_passed: u32,
    ofd_checks: u32,
    ofd_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            struktur_checks: 0,
            struktur_passed: 0,
            ofd_checks: 0,
            ofd_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Struktur => {
                self.struktur_checks += 1;
                if lulus {
                    self.struktur_passed += 1;
                }
            }
            Group::Ofd => {
                self.ofd_checks += 1;
                if lulus {
                    self.ofd_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan.
///
/// copy_mount 2 + buka_perangkat 2 + resolusi_path 2 + mount_cari 2 +
/// ofd_rentang 2 + ofd_dir_entries 2×2=4 + ofd_flock 2 + ofd_custody 2 = **18**.
pub const DIFF_EXPECTED: u32 = 18;

fn run_matrix() -> KlatosAdmisiVfs4Report {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5646_5334, Group::Struktur);
    m.catat(
        ENOENT == 2 && EIO == 5 && EBADF == 9 && ENODEV == 19 && ENOTDIR == 20 && EINVAL == 22 && EOVERFLOW == 36,
        Group::Struktur,
    );

    // --- struktur VFS -----------------------------------------------------
    m.catat(copy_mount_admisi(1) == 0, Group::Struktur);
    m.catat(copy_mount_admisi(0) == -EINVAL, Group::Struktur);
    m.catat(buka_perangkat_admisi(1) == 0, Group::Struktur);
    m.catat(buka_perangkat_admisi(0) == -ENODEV, Group::Struktur);
    m.catat(resolusi_path_admisi(0) == 0, Group::Struktur);
    m.catat(resolusi_path_admisi(1) == -EINVAL, Group::Struktur);
    m.catat(mount_cari_admisi(1) == 0, Group::Struktur);
    m.catat(mount_cari_admisi(0) == -ENODEV, Group::Struktur);

    // --- OpenFileDescription ----------------------------------------------
    m.catat(ofd_rentang_admisi(0) == 0, Group::Ofd);
    m.catat(ofd_rentang_admisi(1) == -EOVERFLOW, Group::Ofd);
    m.catat(ofd_dir_entries_admisi(1, 1) == 0, Group::Ofd);
    m.catat(ofd_dir_entries_admisi(0, 1) == -ENOTDIR, Group::Ofd);
    m.catat(ofd_dir_entries_admisi(1, 0) == -EIO, Group::Ofd);
    // ENOTDIR menang atas EIO.
    m.catat(ofd_dir_entries_admisi(0, 0) == -ENOTDIR, Group::Ofd);
    m.catat(ofd_flock_admisi(1) == 0, Group::Ofd);
    m.catat(ofd_flock_admisi(0) == -EBADF, Group::Ofd);
    m.catat(ofd_custody_admisi(1) == 0, Group::Ofd);
    m.catat(ofd_custody_admisi(0) == -ENOENT, Group::Ofd);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        m.catat(copy_mount_admisi(0) == -EINVAL, Group::Struktur);
        m.catat(copy_mount_salah(0) == 0, Group::Struktur);
        m.catat(copy_mount_admisi(0) != copy_mount_salah(0), Group::Struktur);
        m.catat(copy_mount_admisi(1) == copy_mount_salah(1), Group::Struktur);

        m.catat(mount_cari_admisi(0) == -ENODEV, Group::Struktur);
        m.catat(mount_cari_salah(0) == 0, Group::Struktur);
        m.catat(mount_cari_admisi(0) != mount_cari_salah(0), Group::Struktur);
        m.catat(mount_cari_admisi(1) == mount_cari_salah(1), Group::Struktur);

        m.catat(ofd_dir_entries_admisi(0, 1) == -ENOTDIR, Group::Ofd);
        m.catat(ofd_dir_entries_salah(0, 1) == 0, Group::Ofd);
        m.catat(ofd_dir_entries_admisi(0, 1) != ofd_dir_entries_salah(0, 1), Group::Ofd);
        m.catat(ofd_dir_entries_admisi(1, 0) == ofd_dir_entries_salah(1, 0), Group::Ofd);
    }

    // --- diferensial produksi vs oracle ----------------------------------
    for v in 0..2u32 {
        if copy_mount_admisi(v) != naif_copy_mount_admisi(v) {
            beda += 1;
        }
        diukur += 1;
    }
    for v in 0..2u32 {
        if buka_perangkat_admisi(v) != naif_buka_perangkat_admisi(v) {
            beda += 1;
        }
        diukur += 1;
    }
    for v in 0..2u32 {
        if resolusi_path_admisi(v) != naif_resolusi_path_admisi(v) {
            beda += 1;
        }
        diukur += 1;
    }
    for v in 0..2u32 {
        if mount_cari_admisi(v) != naif_mount_cari_admisi(v) {
            beda += 1;
        }
        diukur += 1;
    }
    for v in 0..2u32 {
        if ofd_rentang_admisi(v) != naif_ofd_rentang_admisi(v) {
            beda += 1;
        }
        diukur += 1;
    }
    for d in 0..2u32 {
        for mv in 0..2u32 {
            if ofd_dir_entries_admisi(d, mv) != naif_ofd_dir_entries_admisi(d, mv) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    for v in 0..2u32 {
        if ofd_flock_admisi(v) != naif_ofd_flock_admisi(v) {
            beda += 1;
        }
        diukur += 1;
    }
    for v in 0..2u32 {
        if ofd_custody_admisi(v) != naif_ofd_custody_admisi(v) {
            beda += 1;
        }
        diukur += 1;
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Struktur);
    m.catat(beda == 0, Group::Struktur);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiVfs4Report {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        struktur_checks: m.struktur_checks,
        struktur_passed: m.struktur_passed,
        ofd_checks: m.ofd_checks,
        ofd_passed: m.ofd_passed,
        prod_copy_mount: klatos_vfs4_counter(Counter::ProdCopyMount as u32),
        prod_buka_perangkat: klatos_vfs4_counter(Counter::ProdBukaPerangkat as u32),
        prod_resolusi_path: klatos_vfs4_counter(Counter::ProdResolusiPath as u32),
        prod_mount_cari: klatos_vfs4_counter(Counter::ProdMountCari as u32),
        prod_ofd_rentang: klatos_vfs4_counter(Counter::ProdOfdRentang as u32),
        prod_ofd_dir: klatos_vfs4_counter(Counter::ProdOfdDir as u32),
        prod_ofd_flock: klatos_vfs4_counter(Counter::ProdOfdFlock as u32),
        prod_ofd_custody: klatos_vfs4_counter(Counter::ProdOfdCustody as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi VFS batch 4.
#[no_mangle]
pub extern "C" fn klatos_vfs4_selftest() -> KlatosAdmisiVfs4Report {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5646_5334);
        assert!(ENOENT == 2);
        assert!(EIO == 5);
        assert!(EBADF == 9);
        assert!(ENODEV == 19);
        assert!(ENOTDIR == 20);
        assert!(EINVAL == 22);
        assert!(EOVERFLOW == 36);
        assert!(Counter::ProdCopyMount as usize == 0);
        assert!(Counter::ProdOfdCustody as usize == 7);
        assert!(N_COUNTER == 8);
        assert!(DIFF_EXPECTED == 18);

        assert!(copy_mount_admisi(1) == 0 && copy_mount_admisi(0) == -EINVAL);
        assert!(buka_perangkat_admisi(0) == -ENODEV);
        assert!(resolusi_path_admisi(1) == -EINVAL);
        assert!(mount_cari_admisi(0) == -ENODEV);
        assert!(ofd_rentang_admisi(1) == -EOVERFLOW);
        assert!(ofd_dir_entries_admisi(0, 0) == -ENOTDIR);
        assert!(ofd_flock_admisi(0) == -EBADF);
        assert!(ofd_custody_admisi(0) == -ENOENT);

        // 8 (tag) + 17*4 (u32) + 4 (status) = 80.
        assert!(core::mem::size_of::<KlatosAdmisiVfs4Report>() == 80);
        assert!(core::mem::align_of::<KlatosAdmisiVfs4Report>() == 8);
    };
}
