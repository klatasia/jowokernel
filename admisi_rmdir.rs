// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS ADMISI STRUKTURAL `rmdir`** (`VirtualFileSystem.cpp`).
//!
//!


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EBUSY` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 16).
pub const EBUSY: i32 = 16;
/// `ENOTDIR` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 20).
pub const ENOTDIR: i32 = 20;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `ENOTEMPTY` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 43).
pub const ENOTEMPTY: i32 = 43;

/// Tag laporan; cermin `KlatosAdmisiRmdirReport` di `Jowo/Kontrak/KlatSeam82.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_524D_4449; // "KLATRMDI"

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_rmdir_awal` dari produksi.
    ProdAwal = 0,
    /// `klatos_rmdir_kosong` dari produksi.
    ProdKosong = 1,
}

const N_COUNTER: usize = 2;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Admisi struktural `rmdir` BAGIAN 1, SEBELUM `izin_hapus`. Transkripsi
/// `rmdir:1022-1035`:
///
/// ```cpp
/// // [EINVAL] The path argument contains a last component that is dot.
/// if (last_component == "."sv) return EINVAL;
/// // [ENOTDIR] A component of path names an existing file that is neither a
/// //           directory nor a symbolic link to a directory.
/// if (!inode.is_directory()) return ENOTDIR;
/// // [EBUSY] ... NOTE: If there is no parent, that means we're trying to
/// //         rmdir the root directory!
/// if (!parent_custody) return EBUSY;
/// ```
///
/// **Urutan adalah kebijakan.** `ENOTDIR` menang atas `EBUSY`, dan `EINVAL`
/// menang atas keduanya.
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn rmdir_awal_admisi(last_dot: u32, adalah_direktori: u32, parent_ada: u32) -> i32 {
    if last_dot != 0 {
        return -EINVAL;
    }
    if adalah_direktori == 0 {
        return -ENOTDIR;
    }
    if parent_ada == 0 {
        return -EBUSY;
    }
    0
}

/// Admisi struktural `rmdir` BAGIAN 2, SESUDAH `izin_hapus` dan SEBELUM
/// `hapus_erofs`. Transkripsi `rmdir:1063-1066`:
///
/// ```cpp
/// // [ENOTEMPTY] The path argument names a directory that is not an empty
/// //             directory, or there are hard links to the directory other
/// //             than dot or a single entry in dot-dot.
/// if (child_count != 2) return ENOTEMPTY;
/// ```
///
/// `jumlah_anak` dihitung C++ lewat `inode.traverse_as_directory` — MEKANISME.
/// Direktori kosong memiliki tepat dua entri: `"."` dan `".."`.
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn rmdir_kosong_admisi(jumlah_anak: u32) -> i32 {
    if jumlah_anak != 2 {
        return -ENOTEMPTY;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_rmdir_awal_admisi(last_dot: u32, dir: u32, parent: u32) -> i32 {
    let mut e = 0;
    if last_dot == 1 {
        e = -EINVAL;
    }
    if e == 0 && dir == 0 {
        e = -ENOTDIR;
    }
    if e == 0 && parent == 0 {
        e = -EBUSY;
    }
    e
}

fn naif_rmdir_kosong_admisi(anak: u32) -> i32 {
    match anak {
        2 => 0,
        _ => -ENOTEMPTY,
    }
}

/// Kontrol NEGATIF: `rmdir_awal_admisi` yang menukar urutan `ENOTDIR` dan
/// `EBUSY`. Sengaja salah; berdaya pada `!adalah_direktori && !parent_ada`:
/// produksi `-ENOTDIR`, yang salah `-EBUSY`.
fn rmdir_awal_admisi_salah(last_dot: u32, dir: u32, parent: u32) -> i32 {
    if last_dot != 0 {
        return -EINVAL;
    }
    if parent == 0 {
        return -EBUSY;
    }
    if dir == 0 {
        return -ENOTDIR;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Admisi struktural `rmdir` bagian 1. Lihat [`rmdir_awal_admisi`].
#[no_mangle]
pub extern "C" fn klatos_rmdir_awal(last_dot: u32, adalah_direktori: u32, parent_ada: u32) -> i32 {
    bump(Counter::ProdAwal);
    rmdir_awal_admisi(last_dot, adalah_direktori, parent_ada)
}

/// Admisi struktural `rmdir` bagian 2. Lihat [`rmdir_kosong_admisi`].
#[no_mangle]
pub extern "C" fn klatos_rmdir_kosong(jumlah_anak: u32) -> i32 {
    bump(Counter::ProdKosong);
    rmdir_kosong_admisi(jumlah_anak)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_rmdir_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_rmdir_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiRmdirReport` di `Jowo/Kontrak/KlatSeam82.h` (56 byte).
#[repr(C)]
pub struct KlatosAdmisiRmdirReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub awal_checks: u32,
    pub awal_passed: u32,
    pub kosong_checks: u32,
    pub kosong_passed: u32,
    pub prod_awal: u32,
    pub prod_kosong: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Awal,
    Kosong,
}

struct Matrix {
    checks: u32,
    passed: u32,
    awal_checks: u32,
    awal_passed: u32,
    kosong_checks: u32,
    kosong_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            awal_checks: 0,
            awal_passed: 0,
            kosong_checks: 0,
            kosong_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Awal => {
                self.awal_checks += 1;
                if lulus {
                    self.awal_passed += 1;
                }
            }
            Group::Kosong => {
                self.kosong_checks += 1;
                if lulus {
                    self.kosong_passed += 1;
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
/// awal `2×2×2` = 8; kosong `0..=4` = 5 ⇒ 8+5 = **13**.
pub const DIFF_EXPECTED: u32 = 13;

fn run_matrix() -> KlatosAdmisiRmdirReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_524D_4449, Group::Awal);
    m.catat(EBUSY == 16 && ENOTDIR == 20 && EINVAL == 22 && ENOTEMPTY == 43, Group::Awal);

    // --- awal -------------------------------------------------------------
    m.catat(rmdir_awal_admisi(0, 1, 1) == 0, Group::Awal);
    m.catat(rmdir_awal_admisi(1, 1, 1) == -EINVAL, Group::Awal);
    m.catat(rmdir_awal_admisi(0, 0, 1) == -ENOTDIR, Group::Awal);
    m.catat(rmdir_awal_admisi(0, 1, 0) == -EBUSY, Group::Awal);
    // Presedensi: EINVAL > ENOTDIR > EBUSY.
    m.catat(rmdir_awal_admisi(1, 0, 1) == -EINVAL, Group::Awal);
    m.catat(rmdir_awal_admisi(1, 1, 0) == -EINVAL, Group::Awal);
    m.catat(rmdir_awal_admisi(0, 0, 0) == -ENOTDIR, Group::Awal);
    m.catat(rmdir_awal_admisi(1, 0, 0) == -EINVAL, Group::Awal);

    // --- kosong -----------------------------------------------------------
    m.catat(rmdir_kosong_admisi(2) == 0, Group::Kosong);
    m.catat(rmdir_kosong_admisi(0) == -ENOTEMPTY, Group::Kosong);
    m.catat(rmdir_kosong_admisi(1) == -ENOTEMPTY, Group::Kosong);
    m.catat(rmdir_kosong_admisi(3) == -ENOTEMPTY, Group::Kosong);
    m.catat(rmdir_kosong_admisi(100) == -ENOTEMPTY, Group::Kosong);

    // --- kontrol NEGATIF: urutan ENOTDIR/EBUSY berdaya --------------------
    {
        // (0,0,0): produksi ENOTDIR; yang salah EBUSY.
        m.catat(rmdir_awal_admisi(0, 0, 0) == -ENOTDIR, Group::Awal);
        m.catat(rmdir_awal_admisi_salah(0, 0, 0) == -EBUSY, Group::Awal);
        m.catat(rmdir_awal_admisi(0, 0, 0) != rmdir_awal_admisi_salah(0, 0, 0), Group::Awal);
        // Pada direktori yang ADA keduanya HARUS sama.
        m.catat(
            rmdir_awal_admisi(0, 1, 0) == rmdir_awal_admisi_salah(0, 1, 0),
            Group::Awal,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // awal: 2 x 2 x 2 = 8
    for dot in 0..2u32 {
        for dir in 0..2u32 {
            for parent in 0..2u32 {
                if rmdir_awal_admisi(dot, dir, parent) != naif_rmdir_awal_admisi(dot, dir, parent) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // kosong: 0..=4 = 5
    for anak in 0..5u32 {
        if rmdir_kosong_admisi(anak) != naif_rmdir_kosong_admisi(anak) {
            beda += 1;
        }
        diukur += 1;
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Awal);
    m.catat(beda == 0, Group::Awal);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiRmdirReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        awal_checks: m.awal_checks,
        awal_passed: m.awal_passed,
        kosong_checks: m.kosong_checks,
        kosong_passed: m.kosong_passed,
        prod_awal: klatos_rmdir_counter(Counter::ProdAwal as u32),
        prod_kosong: klatos_rmdir_counter(Counter::ProdKosong as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi struktural `rmdir`.
#[no_mangle]
pub extern "C" fn klatos_rmdir_selftest() -> KlatosAdmisiRmdirReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_524D_4449);
        assert!(EBUSY == 16);
        assert!(ENOTDIR == 20);
        assert!(EINVAL == 22);
        assert!(ENOTEMPTY == 43);
        assert!(Counter::ProdAwal as usize == 0);
        assert!(Counter::ProdKosong as usize == 1);
        assert!(N_COUNTER == 2);
        assert!(DIFF_EXPECTED == 13);

        // 8 (tag) + 11*4 (u32: checks/passed/awal_*/kosong_*/prod_*/diff_*/status)
        // = 52 -> align 8 -> 56.
        assert!(core::mem::size_of::<KlatosAdmisiRmdirReport>() == 56);
        assert!(core::mem::align_of::<KlatosAdmisiRmdirReport>() == 8);
    };
}
