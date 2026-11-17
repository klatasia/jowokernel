// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS ADMISI STRUKTURAL `rename`** (`VirtualFileSystem.cpp`).
//!


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `EXDEV` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 18).
pub const EXDEV: i32 = 18;
/// `EISDIR` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 21).
pub const EISDIR: i32 = 21;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `ENOTEMPTY` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 43).
pub const ENOTEMPTY: i32 = 43;
/// `EDIRINTOSELF` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 76).
pub const EDIRINTOSELF: i32 = 76;

/// Tag laporan; cermin `KlatosAdmisiRenameReport` di `Jowo/Kontrak/KlatSeam83.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_524E_414D; // "KLATRNAM"

/// Penghitung audit (§21). Satu per kelompok verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `rename_awal_admisi` dari produksi.
    ProdAwal = 0,
    /// `rename_tujuan_perlu` + `rename_tujuan_admisi` dari produksi.
    ProdTujuan = 1,
    /// `rename_fs_admisi` dari produksi.
    ProdFs = 2,
    /// `rename_leluhur_admisi` dari produksi.
    ProdLeluhur = 3,
    /// `rename_nama_admisi` dari produksi.
    ProdNama = 4,
    /// `rename_noop_admisi` dari produksi.
    ProdNoop = 5,
    /// `rename_tipe_admisi` dari produksi.
    ProdTipe = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// `[EPERM]` `rename:725-727`: tanpa induk lama ATAU tanpa induk baru.
///
/// ```cpp
/// if (!old_parent_custody || !new_parent_custody) return EPERM;
/// ```
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn rename_awal_admisi(old_parent_ada: u32, new_parent_ada: u32) -> i32 {
    if old_parent_ada == 0 || new_parent_ada == 0 {
        return -EPERM;
    }
    0
}

/// Keputusan PERLU-TELUSUR `rename:732`: apakah direktori tujuan harus
/// ditelusuri untuk menghitung anak (guard `ENOTEMPTY`).
///
/// ```cpp
/// if (old_inode.index() != new_inode.index()
///     && old_inode.is_directory() && new_inode.is_directory()) { /* traverse */ }
/// ```
///
/// Mengembalikan `1` bila traverse diperlukan, `0` bila tidak. Ini keputusan
/// (bukan sekadar optimisasi): traverse MAHAL dan hanya sah bila ketiga fakta
/// itu benar.
#[inline]
pub const fn rename_tujuan_perlu(index_sama: u32, old_dir: u32, new_dir: u32) -> u32 {
    if index_sama == 0 && old_dir != 0 && new_dir != 0 {
        1
    } else {
        0
    }
}

/// `[ENOTEMPTY]` `rename:738-739`: direktori tujuan punya lebih dari dua entri
/// (`"."` + `".."`).
///
/// ```cpp
/// if (child_count > 2) return ENOTEMPTY;
/// ```
///
/// `anak_lebih_2` dihitung C++ lewat `traverse_as_directory` — MEKANISME.
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn rename_tujuan_admisi(anak_lebih_2: u32) -> i32 {
    if anak_lebih_2 != 0 {
        return -ENOTEMPTY;
    }
    0
}

/// `[EXDEV]` `rename:746-747`: sumber dan tujuan di sistem berkas BERBEDA.
///
/// ```cpp
/// if (&old_parent_inode.fs() != &new_parent_inode.fs()) return EXDEV;
/// ```
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn rename_fs_admisi(fs_sama: u32) -> i32 {
    if fs_sama == 0 {
        return -EXDEV;
    }
    0
}

/// `[EDIRINTOSELF]` `rename:750-751`: satu leluhur induk TUJUAN adalah inode
/// SUMBER — yaitu memindahkan direktori ke dalam dirinya sendiri.
///
/// ```cpp
/// for (auto* a = new_parent_custody.ptr(); a; a = a->parent())
///     if (&old_inode == &a->inode()) return EDIRINTOSELF;
/// ```
///
/// Dipanggil SEKALI PER LELUHUR (penelusuran rantai = MEKANISME C++).
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn rename_leluhur_admisi(inode_sama: u32) -> i32 {
    if inode_sama != 0 {
        return -EDIRINTOSELF;
    }
    0
}

/// `[EINVAL]` `rename:777-782`: nama dasar kosong, `"."`, atau `".."`.
/// Dipanggil DUA KALI (nama lama, nama baru) dengan bentuk yang sama.
///
/// ```cpp
/// if (basename.is_empty() || basename == "."sv || basename == ".."sv) return EINVAL;
/// ```
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn rename_nama_admisi(kosong: u32, dot: u32, dotdot: u32) -> i32 {
    if kosong != 0 || dot != 0 || dotdot != 0 {
        return -EINVAL;
    }
    0
}

/// No-op `rename:784-785,795-796`: tidak ada yang perlu dilakukan, dan itu
/// SUKSES (bukan errno).
///
/// ```cpp
/// if (old_basename == new_basename && old_parent_inode.index() == new_parent_inode.index()) return {};
/// ...
/// if (&new_inode == &old_inode) return {};
/// ```
///
/// Dua bentuk digabung ke satu keputusan; pemanggil mengisi argumen yang tidak
/// relevan dengan `0`. Mengembalikan `1` bila no-op (sukses), `0` bila lanjut.
#[inline]
pub const fn rename_noop_admisi(nama_induk_sama: u32, inode_sama: u32) -> u32 {
    if nama_induk_sama != 0 || inode_sama != 0 {
        1
    } else {
        0
    }
}

/// `[EISDIR]` `rename:807-808`: tujuan direktori tetapi sumber bukan direktori.
///
/// ```cpp
/// if (new_inode.is_directory() && !old_inode.is_directory()) return EISDIR;
/// ```
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn rename_tipe_admisi(new_is_dir: u32, old_is_dir: u32) -> i32 {
    if new_is_dir != 0 && old_is_dir == 0 {
        return -EISDIR;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_rename_awal(old: u32, new: u32) -> i32 {
    match (old, new) {
        (0, _) | (_, 0) => -EPERM,
        _ => 0,
    }
}

fn naif_rename_tujuan_perlu(index_sama: u32, old_dir: u32, new_dir: u32) -> u32 {
    (index_sama | (1 - old_dir) | (1 - new_dir)) & 1 ^ 1
}

fn naif_rename_tujuan(anak_lebih_2: u32) -> i32 {
    match anak_lebih_2 {
        0 => 0,
        _ => -ENOTEMPTY,
    }
}

fn naif_rename_fs(fs_sama: u32) -> i32 {
    match fs_sama {
        0 => -EXDEV,
        _ => 0,
    }
}

fn naif_rename_leluhur(inode_sama: u32) -> i32 {
    match inode_sama {
        0 => 0,
        _ => -EDIRINTOSELF,
    }
}

fn naif_rename_nama(kosong: u32, dot: u32, dotdot: u32) -> i32 {
    let mut e = 0;
    if kosong == 1 {
        e = -EINVAL;
    }
    if e == 0 && dot == 1 {
        e = -EINVAL;
    }
    if e == 0 && dotdot == 1 {
        e = -EINVAL;
    }
    e
}

fn naif_rename_noop(nama_induk_sama: u32, inode_sama: u32) -> u32 {
    match nama_induk_sama + inode_sama {
        0 => 0,
        _ => 1,
    }
}

fn naif_rename_tipe(new_is_dir: u32, old_is_dir: u32) -> i32 {
    let mut e = 0;
    if new_is_dir == 1 {
        if old_is_dir == 0 {
            e = -EISDIR;
        }
    }
    e
}

/// Kontrol NEGATIF: `rename_tipe_admisi` yang LUPA syarat `!old_is_dir`.
/// Sengaja salah; berdaya pada `new_is_dir == 1 && old_is_dir == 1`:
/// produksi `0`, yang salah `-EISDIR`.
fn rename_tipe_admisi_salah(new_is_dir: u32, _old_is_dir: u32) -> i32 {
    if new_is_dir != 0 {
        return -EISDIR;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_rename_awal(old_parent_ada: u32, new_parent_ada: u32) -> i32 {
    bump(Counter::ProdAwal);
    rename_awal_admisi(old_parent_ada, new_parent_ada)
}

#[no_mangle]
pub extern "C" fn klatos_rename_tujuan_perlu(index_sama: u32, old_dir: u32, new_dir: u32) -> u32 {
    bump(Counter::ProdTujuan);
    rename_tujuan_perlu(index_sama, old_dir, new_dir)
}

#[no_mangle]
pub extern "C" fn klatos_rename_tujuan(anak_lebih_2: u32) -> i32 {
    bump(Counter::ProdTujuan);
    rename_tujuan_admisi(anak_lebih_2)
}

#[no_mangle]
pub extern "C" fn klatos_rename_fs(fs_sama: u32) -> i32 {
    bump(Counter::ProdFs);
    rename_fs_admisi(fs_sama)
}

#[no_mangle]
pub extern "C" fn klatos_rename_leluhur(inode_sama: u32) -> i32 {
    bump(Counter::ProdLeluhur);
    rename_leluhur_admisi(inode_sama)
}

#[no_mangle]
pub extern "C" fn klatos_rename_nama(kosong: u32, dot: u32, dotdot: u32) -> i32 {
    bump(Counter::ProdNama);
    rename_nama_admisi(kosong, dot, dotdot)
}

#[no_mangle]
pub extern "C" fn klatos_rename_noop(nama_induk_sama: u32, inode_sama: u32) -> u32 {
    bump(Counter::ProdNoop);
    rename_noop_admisi(nama_induk_sama, inode_sama)
}

#[no_mangle]
pub extern "C" fn klatos_rename_tipe(new_is_dir: u32, old_is_dir: u32) -> i32 {
    bump(Counter::ProdTipe);
    rename_tipe_admisi(new_is_dir, old_is_dir)
}

#[no_mangle]
pub extern "C" fn klatos_rename_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn klatos_rename_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiRenameReport` di `Jowo/Kontrak/KlatSeam83.h`.
#[repr(C)]
pub struct KlatosAdmisiRenameReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub awal_checks: u32,
    pub awal_passed: u32,
    pub tujuan_checks: u32,
    pub tujuan_passed: u32,
    pub fs_checks: u32,
    pub fs_passed: u32,
    pub leluhur_checks: u32,
    pub leluhur_passed: u32,
    pub nama_checks: u32,
    pub nama_passed: u32,
    pub noop_checks: u32,
    pub noop_passed: u32,
    pub tipe_checks: u32,
    pub tipe_passed: u32,
    pub prod_awal: u32,
    pub prod_tujuan: u32,
    pub prod_fs: u32,
    pub prod_leluhur: u32,
    pub prod_nama: u32,
    pub prod_noop: u32,
    pub prod_tipe: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Awal,
    Tujuan,
    Fs,
    Leluhur,
    Nama,
    Noop,
    Tipe,
}

struct Matrix {
    checks: u32,
    passed: u32,
    g: [(u32, u32); 7],
}

impl Matrix {
    const fn new() -> Self {
        Matrix { checks: 0, passed: 0, g: [(0, 0); 7] }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        let i = group as usize;
        self.g[i].0 += 1;
        if lulus {
            self.passed += 1;
            self.g[i].1 += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan oleh `run_matrix`.
///
/// awal 4 + tujuan_perlu 8 + tujuan 2 + fs 2 + leluhur 2 + nama 8 + noop 4 + tipe 4 = **34**.
pub const DIFF_EXPECTED: u32 = 34;

fn run_matrix() -> KlatosAdmisiRenameReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_524E_414D, Group::Awal);
    m.catat(
        EPERM == 1 && EXDEV == 18 && EISDIR == 21 && EINVAL == 22 && ENOTEMPTY == 43 && EDIRINTOSELF == 76,
        Group::Awal,
    );

    // --- awal (EPERM) -----------------------------------------------------
    m.catat(rename_awal_admisi(1, 1) == 0, Group::Awal);
    m.catat(rename_awal_admisi(0, 1) == -EPERM, Group::Awal);
    m.catat(rename_awal_admisi(1, 0) == -EPERM, Group::Awal);
    m.catat(rename_awal_admisi(0, 0) == -EPERM, Group::Awal);

    // --- tujuan -----------------------------------------------------------
    m.catat(rename_tujuan_perlu(0, 1, 1) == 1, Group::Tujuan);
    m.catat(rename_tujuan_perlu(1, 1, 1) == 0, Group::Tujuan);
    m.catat(rename_tujuan_perlu(0, 0, 1) == 0, Group::Tujuan);
    m.catat(rename_tujuan_perlu(0, 1, 0) == 0, Group::Tujuan);
    m.catat(rename_tujuan_admisi(0) == 0, Group::Tujuan);
    m.catat(rename_tujuan_admisi(1) == -ENOTEMPTY, Group::Tujuan);

    // --- fs (EXDEV) -------------------------------------------------------
    m.catat(rename_fs_admisi(1) == 0, Group::Fs);
    m.catat(rename_fs_admisi(0) == -EXDEV, Group::Fs);

    // --- leluhur (EDIRINTOSELF) ------------------------------------------
    m.catat(rename_leluhur_admisi(0) == 0, Group::Leluhur);
    m.catat(rename_leluhur_admisi(1) == -EDIRINTOSELF, Group::Leluhur);

    // --- nama (EINVAL) ----------------------------------------------------
    m.catat(rename_nama_admisi(0, 0, 0) == 0, Group::Nama);
    m.catat(rename_nama_admisi(1, 0, 0) == -EINVAL, Group::Nama);
    m.catat(rename_nama_admisi(0, 1, 0) == -EINVAL, Group::Nama);
    m.catat(rename_nama_admisi(0, 0, 1) == -EINVAL, Group::Nama);
    m.catat(rename_nama_admisi(1, 1, 1) == -EINVAL, Group::Nama);

    // --- no-op ------------------------------------------------------------
    m.catat(rename_noop_admisi(0, 0) == 0, Group::Noop);
    m.catat(rename_noop_admisi(1, 0) == 1, Group::Noop);
    m.catat(rename_noop_admisi(0, 1) == 1, Group::Noop);
    m.catat(rename_noop_admisi(1, 1) == 1, Group::Noop);

    // --- tipe (EISDIR) ----------------------------------------------------
    m.catat(rename_tipe_admisi(1, 0) == -EISDIR, Group::Tipe);
    m.catat(rename_tipe_admisi(1, 1) == 0, Group::Tipe);
    m.catat(rename_tipe_admisi(0, 0) == 0, Group::Tipe);
    m.catat(rename_tipe_admisi(0, 1) == 0, Group::Tipe);

    // --- kontrol NEGATIF: `!old_is_dir` berdaya ---------------------------
    {
        m.catat(rename_tipe_admisi(1, 1) == 0, Group::Tipe);
        m.catat(rename_tipe_admisi_salah(1, 1) == -EISDIR, Group::Tipe);
        m.catat(
            rename_tipe_admisi(1, 1) != rename_tipe_admisi_salah(1, 1),
            Group::Tipe,
        );
        // Pada non-direktori keduanya HARUS sama.
        m.catat(
            rename_tipe_admisi(1, 0) == rename_tipe_admisi_salah(1, 0),
            Group::Tipe,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    for old in 0..2u32 {
        for new in 0..2u32 {
            if rename_awal_admisi(old, new) != naif_rename_awal(old, new) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    for index_sama in 0..2u32 {
        for old_dir in 0..2u32 {
            for new_dir in 0..2u32 {
                if rename_tujuan_perlu(index_sama, old_dir, new_dir)
                    != naif_rename_tujuan_perlu(index_sama, old_dir, new_dir)
                {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    for anak in 0..2u32 {
        if rename_tujuan_admisi(anak) != naif_rename_tujuan(anak) {
            beda += 1;
        }
        diukur += 1;
    }
    for fs in 0..2u32 {
        if rename_fs_admisi(fs) != naif_rename_fs(fs) {
            beda += 1;
        }
        diukur += 1;
    }
    for lel in 0..2u32 {
        if rename_leluhur_admisi(lel) != naif_rename_leluhur(lel) {
            beda += 1;
        }
        diukur += 1;
    }
    for kosong in 0..2u32 {
        for dot in 0..2u32 {
            for dotdot in 0..2u32 {
                if rename_nama_admisi(kosong, dot, dotdot) != naif_rename_nama(kosong, dot, dotdot) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    for nis in 0..2u32 {
        for is in 0..2u32 {
            if rename_noop_admisi(nis, is) != naif_rename_noop(nis, is) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    for nd in 0..2u32 {
        for od in 0..2u32 {
            if rename_tipe_admisi(nd, od) != naif_rename_tipe(nd, od) {
                beda += 1;
            }
            diukur += 1;
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Awal);
    m.catat(beda == 0, Group::Awal);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiRenameReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        awal_checks: m.g[Group::Awal as usize].0,
        awal_passed: m.g[Group::Awal as usize].1,
        tujuan_checks: m.g[Group::Tujuan as usize].0,
        tujuan_passed: m.g[Group::Tujuan as usize].1,
        fs_checks: m.g[Group::Fs as usize].0,
        fs_passed: m.g[Group::Fs as usize].1,
        leluhur_checks: m.g[Group::Leluhur as usize].0,
        leluhur_passed: m.g[Group::Leluhur as usize].1,
        nama_checks: m.g[Group::Nama as usize].0,
        nama_passed: m.g[Group::Nama as usize].1,
        noop_checks: m.g[Group::Noop as usize].0,
        noop_passed: m.g[Group::Noop as usize].1,
        tipe_checks: m.g[Group::Tipe as usize].0,
        tipe_passed: m.g[Group::Tipe as usize].1,
        prod_awal: klatos_rename_counter(Counter::ProdAwal as u32),
        prod_tujuan: klatos_rename_counter(Counter::ProdTujuan as u32),
        prod_fs: klatos_rename_counter(Counter::ProdFs as u32),
        prod_leluhur: klatos_rename_counter(Counter::ProdLeluhur as u32),
        prod_nama: klatos_rename_counter(Counter::ProdNama as u32),
        prod_noop: klatos_rename_counter(Counter::ProdNoop as u32),
        prod_tipe: klatos_rename_counter(Counter::ProdTipe as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi struktural `rename`.
#[no_mangle]
pub extern "C" fn klatos_rename_selftest() -> KlatosAdmisiRenameReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_524E_414D);
        assert!(EPERM == 1);
        assert!(EXDEV == 18);
        assert!(EISDIR == 21);
        assert!(EINVAL == 22);
        assert!(ENOTEMPTY == 43);
        assert!(EDIRINTOSELF == 76);
        assert!(Counter::ProdAwal as usize == 0);
        assert!(Counter::ProdTipe as usize == 6);
        assert!(N_COUNTER == 7);
        assert!(DIFF_EXPECTED == 34);

        // 8 (tag) + 8 (checks/passed) + 7*2*4 (grup) + 7*4 (prod) + 2*4 (diff) + 4 (status) = 112.
        assert!(core::mem::size_of::<KlatosAdmisiRenameReport>() == 112);
        assert!(core::mem::align_of::<KlatosAdmisiRenameReport>() == 8);
    };
}
