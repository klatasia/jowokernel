// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.85 / WS4 — **OTORITAS ADMISI VFS, BATCH 3** (`VirtualFileSystem.cpp`).


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `ENOENT` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 2).
pub const ENOENT: i32 = 2;
/// `ENOTBLK` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 15).
pub const ENOTBLK: i32 = 15;
/// `EEXIST` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 17).
pub const EEXIST: i32 = 17;
/// `EXDEV` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 18).
pub const EXDEV: i32 = 18;
/// `ENODEV` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 19).
pub const ENODEV: i32 = 19;
/// `EISDIR` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 21).
pub const EISDIR: i32 = 21;
/// `ENOTSUP` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 74).
pub const ENOTSUP: i32 = 74;

/// Tag laporan; cermin `KlatosAdmisiVfs3Report` di `Jowo/Kontrak/KlatSeam85.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5646_5333; // "KLATVFS3"

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_vfs3_link` dari produksi.
    ProdLink = 0,
    /// `klatos_vfs3_unlink` dari produksi.
    ProdUnlink = 1,
    /// `klatos_vfs3_mount_tipe` dari produksi.
    ProdMountTipe = 2,
    /// `klatos_vfs3_mount_buat` dari produksi.
    ProdMountBuat = 3,
    /// `klatos_vfs3_mount_verifikasi` dari produksi.
    ProdMountVerif = 4,
}

const N_COUNTER: usize = 5;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Admisi STRUKTURAL `VFS::link`. Transkripsi `link:945-956`:
///
/// ```cpp
/// auto new_custody_or_error = resolve_path(..., new_path, base, &parent_custody);
/// if (!new_custody_or_error.is_error()) return EEXIST;
/// if (!parent_custody)                  return ENOENT;
/// if (parent_inode.fsid() != old_inode.fsid()) return EXDEV;
/// ```
///
/// Ketiga cek berada SEBELUM jalur izin (P1.66 `mod_hapus`), jadi urutannya
/// kebijakan: `EEXIST` > `ENOENT` > `EXDEV`.
///
/// `fsid_sama` adalah FAKTA yang dihitung C++ (`parent_custody->inode().fsid()
/// == old_inode.fsid()`), dan hanya bermakna bila `parent_ada != 0`.
///
/// Mengembalikan `0` (lanjut ke jalur izin) atau errno negatif.
#[inline]
pub const fn link_struktural_admisi(resolve_ok: u32, parent_ada: u32, fsid_sama: u32) -> i32 {
    if resolve_ok != 0 {
        return -EEXIST;
    }
    if parent_ada == 0 {
        return -ENOENT;
    }
    if fsid_sama == 0 {
        return -EXDEV;
    }
    0
}

/// Admisi STRUKTURAL `VFS::unlink`. Transkripsi `unlink:996-997`:
///
/// ```cpp
/// if (inode.is_directory()) return EISDIR;
/// ```
///
/// Berada SEBELUM jalur izin (P1.66 `mod_hapus`) dan sebelum cek read-only.
/// Mengembalikan `0` (lanjut) atau `-EISDIR`.
#[inline]
pub const fn unlink_struktural_admisi(adalah_direktori: u32) -> i32 {
    if adalah_direktori != 0 {
        return -EISDIR;
    }
    0
}

/// Admisi `VFS::find_filesystem_type_initializer`. Transkripsi `:125-132`:
///
/// ```cpp
/// for (auto& initializer_entry : s_initializers) {
///     if (fs_type == initializer_entry.short_name || fs_type == initializer_entry.name)
///         return &initializer_entry;
/// }
/// return ENODEV;
/// ```
///
/// Pencarian string tetap C++ (MEKANISME); yang dipindahkan adalah VERDICT
/// "tidak ada tipe yang cocok". Mengembalikan `0` (ditemukan) atau `-ENODEV`.
#[inline]
pub const fn mount_tipe_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENODEV;
    }
    0
}

/// Admisi `create_and_initialize_filesystem_from_mount_file`. Transkripsi
/// `:185-189`:
///
/// ```cpp
/// if (file_system_initializer.requires_open_file_description) return ENOTSUP;
/// if (!file_system_initializer.create)                           return ENOTSUP;
/// ```
///
/// Mengembalikan `0` (boleh dibuat) atau `-ENOTSUP`.
#[inline]
pub const fn mount_buat_admisi(perlu_fd: u32, punya_create: u32) -> i32 {
    if perlu_fd != 0 {
        return -ENOTSUP;
    }
    if punya_create == 0 {
        return -ENOTSUP;
    }
    0
}

/// Admisi `verify_mount_file_and_description_requirements`. Transkripsi
/// `:200-218`:
///
/// ```cpp
/// if (!file_system_initializer.requires_open_file_description) return ENOTSUP;
/// if (requires_block_device && !source_description.file().is_block_device()) return ENOTBLK;
/// if (requires_seekable_file && !source_description.file().is_seekable()) {
///     dbgln("mount: this is not a seekable file");
///     return ENODEV;
/// }
/// ```
///
/// `dbgln` adalah EFEK dan tetap C++ (dipicu saat verdict `-ENODEV`).
/// Mengembalikan `0` (boleh dipasang) atau `-ENOTSUP`/`-ENOTBLK`/`-ENODEV`.
#[inline]
pub const fn mount_verifikasi_admisi(
    perlu_fd: u32,
    perlu_blkdev: u32,
    adalah_blkdev: u32,
    perlu_seekable: u32,
    adalah_seekable: u32,
) -> i32 {
    if perlu_fd == 0 {
        return -ENOTSUP;
    }
    if perlu_blkdev != 0 && adalah_blkdev == 0 {
        return -ENOTBLK;
    }
    if perlu_seekable != 0 && adalah_seekable == 0 {
        return -ENODEV;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_link_struktural_admisi(resolve_ok: u32, parent_ada: u32, fsid_sama: u32) -> i32 {
    if resolve_ok == 1 {
        -EEXIST
    } else if parent_ada == 0 {
        -ENOENT
    } else if fsid_sama == 0 {
        -EXDEV
    } else {
        0
    }
}

fn naif_unlink_struktural_admisi(adalah_direktori: u32) -> i32 {
    match adalah_direktori {
        0 => 0,
        _ => -EISDIR,
    }
}

fn naif_mount_tipe_admisi(ditemukan: u32) -> i32 {
    match ditemukan {
        0 => -ENODEV,
        _ => 0,
    }
}

fn naif_mount_buat_admisi(perlu_fd: u32, punya_create: u32) -> i32 {
    if perlu_fd == 0 && punya_create == 1 {
        0
    } else {
        -ENOTSUP
    }
}

fn naif_mount_verifikasi_admisi(
    perlu_fd: u32,
    perlu_blkdev: u32,
    adalah_blkdev: u32,
    perlu_seekable: u32,
    adalah_seekable: u32,
) -> i32 {
    let mut e = 0;
    if perlu_fd == 0 {
        e = -ENOTSUP;
    } else if perlu_blkdev == 1 && adalah_blkdev == 0 {
        e = -ENOTBLK;
    } else if perlu_seekable == 1 && adalah_seekable == 0 {
        e = -ENODEV;
    }
    e
}

/// Kontrol NEGATIF: `link_struktural_admisi` yang MENJATUHKAN penjaga `EXDEV`.
/// Sengaja SALAH; berdaya pada `resolve_ok == 0 && parent_ada == 1 &&
/// fsid_sama == 0`: produksi `-EXDEV`, yang salah `0` (meloloskan hardlink
/// lintas-device) — dan pada kasus lain keduanya HARUS sama.
fn link_struktural_salah(resolve_ok: u32, parent_ada: u32, _fsid_sama: u32) -> i32 {
    if resolve_ok != 0 {
        return -EEXIST;
    }
    if parent_ada == 0 {
        return -ENOENT;
    }
    0
}

/// Kontrol NEGATIF: `unlink_struktural_admisi` yang MENJATUHKAN penjaga
/// `EISDIR`. Sengaja SALAH; berdaya pada `adalah_direktori == 1`.
fn unlink_struktural_salah(_adalah_direktori: u32) -> i32 {
    0
}

/// Kontrol NEGATIF: `mount_verifikasi_admisi` yang MENJATUHKAN penjaga
/// `ENOTSUP`. Sengaja SALAH; berdaya pada `perlu_fd == 0`.
fn mount_verifikasi_salah(
    perlu_fd: u32,
    perlu_blkdev: u32,
    adalah_blkdev: u32,
    perlu_seekable: u32,
    adalah_seekable: u32,
) -> i32 {
    if perlu_blkdev != 0 && adalah_blkdev == 0 {
        return -ENOTBLK;
    }
    if perlu_seekable != 0 && adalah_seekable == 0 {
        return -ENODEV;
    }
    let _ = perlu_fd;
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Admisi struktural `link()`. Lihat [`link_struktural_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs3_link(resolve_ok: u32, parent_ada: u32, fsid_sama: u32) -> i32 {
    bump(Counter::ProdLink);
    link_struktural_admisi(resolve_ok, parent_ada, fsid_sama)
}

/// Admisi struktural `unlink()`. Lihat [`unlink_struktural_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs3_unlink(adalah_direktori: u32) -> i32 {
    bump(Counter::ProdUnlink);
    unlink_struktural_admisi(adalah_direktori)
}

/// Verdict tipe filesystem tak dikenal. Lihat [`mount_tipe_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs3_mount_tipe(ditemukan: u32) -> i32 {
    bump(Counter::ProdMountTipe);
    mount_tipe_admisi(ditemukan)
}

/// Verdict "boleh dibuat" mount tanpa fd. Lihat [`mount_buat_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs3_mount_buat(perlu_fd: u32, punya_create: u32) -> i32 {
    bump(Counter::ProdMountBuat);
    mount_buat_admisi(perlu_fd, punya_create)
}

/// Verdict persyaratan mount berbasis fd. Lihat [`mount_verifikasi_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs3_mount_verifikasi(
    perlu_fd: u32,
    perlu_blkdev: u32,
    adalah_blkdev: u32,
    perlu_seekable: u32,
    adalah_seekable: u32,
) -> i32 {
    bump(Counter::ProdMountVerif);
    mount_verifikasi_admisi(perlu_fd, perlu_blkdev, adalah_blkdev, perlu_seekable, adalah_seekable)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_vfs3_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_vfs3_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiVfs3Report` di `Jowo/Kontrak/KlatSeam85.h`.
#[repr(C)]
pub struct KlatosAdmisiVfs3Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub link_checks: u32,
    pub link_passed: u32,
    pub unlink_checks: u32,
    pub unlink_passed: u32,
    pub mount_checks: u32,
    pub mount_passed: u32,
    pub prod_link: u32,
    pub prod_unlink: u32,
    pub prod_mount_tipe: u32,
    pub prod_mount_buat: u32,
    pub prod_mount_verif: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Link,
    Unlink,
    Mount,
}

struct Matrix {
    checks: u32,
    passed: u32,
    link_checks: u32,
    link_passed: u32,
    unlink_checks: u32,
    unlink_passed: u32,
    mount_checks: u32,
    mount_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            link_checks: 0,
            link_passed: 0,
            unlink_checks: 0,
            unlink_passed: 0,
            mount_checks: 0,
            mount_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Link => {
                self.link_checks += 1;
                if lulus {
                    self.link_passed += 1;
                }
            }
            Group::Unlink => {
                self.unlink_checks += 1;
                if lulus {
                    self.unlink_passed += 1;
                }
            }
            Group::Mount => {
                self.mount_checks += 1;
                if lulus {
                    self.mount_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan oleh `run_matrix`.
///
/// link `2×2×2` = 8; unlink `2` = 2; mount_tipe `2` = 2; mount_buat `2×2` = 4;
/// mount_verifikasi `2^5` = 32 ⇒ 8+2+2+4+32 = **48**.
pub const DIFF_EXPECTED: u32 = 48;

fn run_matrix() -> KlatosAdmisiVfs3Report {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5646_5333, Group::Link);
    m.catat(
        ENOENT == 2 && ENOTBLK == 15 && EEXIST == 17 && EXDEV == 18 && ENODEV == 19 && EISDIR == 21 && ENOTSUP == 74,
        Group::Link,
    );

    // --- link struktural --------------------------------------------------
    m.catat(link_struktural_admisi(0, 1, 1) == 0, Group::Link);
    m.catat(link_struktural_admisi(1, 1, 1) == -EEXIST, Group::Link);
    m.catat(link_struktural_admisi(0, 0, 1) == -ENOENT, Group::Link);
    m.catat(link_struktural_admisi(0, 1, 0) == -EXDEV, Group::Link);
    // EEXIST menang atas ENOENT/EXDEV.
    m.catat(link_struktural_admisi(1, 0, 0) == -EEXIST, Group::Link);
    // ENOENT menang atas EXDEV.
    m.catat(link_struktural_admisi(0, 0, 0) == -ENOENT, Group::Link);

    // --- unlink struktural ------------------------------------------------
    m.catat(unlink_struktural_admisi(0) == 0, Group::Unlink);
    m.catat(unlink_struktural_admisi(1) == -EISDIR, Group::Unlink);

    // --- mount ------------------------------------------------------------
    m.catat(mount_tipe_admisi(1) == 0, Group::Mount);
    m.catat(mount_tipe_admisi(0) == -ENODEV, Group::Mount);
    m.catat(mount_buat_admisi(0, 1) == 0, Group::Mount);
    m.catat(mount_buat_admisi(1, 1) == -ENOTSUP, Group::Mount);
    m.catat(mount_buat_admisi(0, 0) == -ENOTSUP, Group::Mount);
    m.catat(mount_buat_admisi(1, 0) == -ENOTSUP, Group::Mount);
    m.catat(mount_verifikasi_admisi(1, 1, 1, 1, 1) == 0, Group::Mount);
    m.catat(mount_verifikasi_admisi(0, 1, 1, 1, 1) == -ENOTSUP, Group::Mount);
    m.catat(mount_verifikasi_admisi(1, 1, 0, 0, 0) == -ENOTBLK, Group::Mount);
    m.catat(mount_verifikasi_admisi(1, 0, 0, 1, 0) == -ENODEV, Group::Mount);
    // ENOTSUP menang atas ENOTBLK/ENODEV.
    m.catat(mount_verifikasi_admisi(0, 1, 0, 1, 0) == -ENOTSUP, Group::Mount);
    // ENOTBLK menang atas ENODEV.
    m.catat(mount_verifikasi_admisi(1, 1, 0, 1, 0) == -ENOTBLK, Group::Mount);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // link: penjaga EXDEV berdaya.
        m.catat(link_struktural_admisi(0, 1, 0) == -EXDEV, Group::Link);
        m.catat(link_struktural_salah(0, 1, 0) == 0, Group::Link);
        m.catat(
            link_struktural_admisi(0, 1, 0) != link_struktural_salah(0, 1, 0),
            Group::Link,
        );
        // Pada fsid_sama == 1 keduanya HARUS sama.
        m.catat(
            link_struktural_admisi(0, 1, 1) == link_struktural_salah(0, 1, 1),
            Group::Link,
        );

        // unlink: penjaga EISDIR berdaya.
        m.catat(unlink_struktural_admisi(1) == -EISDIR, Group::Unlink);
        m.catat(unlink_struktural_salah(1) == 0, Group::Unlink);
        m.catat(
            unlink_struktural_admisi(1) != unlink_struktural_salah(1),
            Group::Unlink,
        );

        // mount: penjaga ENOTSUP berdaya.
        m.catat(mount_verifikasi_admisi(0, 0, 0, 0, 0) == -ENOTSUP, Group::Mount);
        m.catat(mount_verifikasi_salah(0, 0, 0, 0, 0) == 0, Group::Mount);
        m.catat(
            mount_verifikasi_admisi(0, 0, 0, 0, 0) != mount_verifikasi_salah(0, 0, 0, 0, 0),
            Group::Mount,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // link: 2 x 2 x 2 = 8
    for resolve_ok in 0..2u32 {
        for parent_ada in 0..2u32 {
            for fsid_sama in 0..2u32 {
                if link_struktural_admisi(resolve_ok, parent_ada, fsid_sama)
                    != naif_link_struktural_admisi(resolve_ok, parent_ada, fsid_sama)
                {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // unlink: 2
    for dir in 0..2u32 {
        if unlink_struktural_admisi(dir) != naif_unlink_struktural_admisi(dir) {
            beda += 1;
        }
        diukur += 1;
    }
    // mount_tipe: 2
    for ditemukan in 0..2u32 {
        if mount_tipe_admisi(ditemukan) != naif_mount_tipe_admisi(ditemukan) {
            beda += 1;
        }
        diukur += 1;
    }
    // mount_buat: 2 x 2 = 4
    for perlu_fd in 0..2u32 {
        for punya_create in 0..2u32 {
            if mount_buat_admisi(perlu_fd, punya_create) != naif_mount_buat_admisi(perlu_fd, punya_create) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // mount_verifikasi: 2^5 = 32
    for perlu_fd in 0..2u32 {
        for perlu_blkdev in 0..2u32 {
            for adalah_blkdev in 0..2u32 {
                for perlu_seekable in 0..2u32 {
                    for adalah_seekable in 0..2u32 {
                        if mount_verifikasi_admisi(perlu_fd, perlu_blkdev, adalah_blkdev, perlu_seekable, adalah_seekable)
                            != naif_mount_verifikasi_admisi(perlu_fd, perlu_blkdev, adalah_blkdev, perlu_seekable, adalah_seekable)
                        {
                            beda += 1;
                        }
                        diukur += 1;
                    }
                }
            }
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Mount);
    m.catat(beda == 0, Group::Mount);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiVfs3Report {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        link_checks: m.link_checks,
        link_passed: m.link_passed,
        unlink_checks: m.unlink_checks,
        unlink_passed: m.unlink_passed,
        mount_checks: m.mount_checks,
        mount_passed: m.mount_passed,
        prod_link: klatos_vfs3_counter(Counter::ProdLink as u32),
        prod_unlink: klatos_vfs3_counter(Counter::ProdUnlink as u32),
        prod_mount_tipe: klatos_vfs3_counter(Counter::ProdMountTipe as u32),
        prod_mount_buat: klatos_vfs3_counter(Counter::ProdMountBuat as u32),
        prod_mount_verif: klatos_vfs3_counter(Counter::ProdMountVerif as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi VFS batch 3.
#[no_mangle]
pub extern "C" fn klatos_vfs3_selftest() -> KlatosAdmisiVfs3Report {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5646_5333);
        assert!(ENOENT == 2);
        assert!(ENOTBLK == 15);
        assert!(EEXIST == 17);
        assert!(EXDEV == 18);
        assert!(ENODEV == 19);
        assert!(EISDIR == 21);
        assert!(ENOTSUP == 74);
        assert!(Counter::ProdLink as usize == 0);
        assert!(Counter::ProdUnlink as usize == 1);
        assert!(Counter::ProdMountTipe as usize == 2);
        assert!(Counter::ProdMountBuat as usize == 3);
        assert!(Counter::ProdMountVerif as usize == 4);
        assert!(N_COUNTER == 5);
        assert!(DIFF_EXPECTED == 48);

        // 8 (tag) + 15*4 (u32) + 4 (status) = 72 -> align 8 -> 72.
        assert!(core::mem::size_of::<KlatosAdmisiVfs3Report>() == 72);
        assert!(core::mem::align_of::<KlatosAdmisiVfs3Report>() == 8);
    };
}
