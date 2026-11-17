// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS BERKAS-SISTEM, BATCH 5: ProcFS**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `ENOENT` — indeks 2.
pub const ENOENT: i32 = 2;
/// `ESRCH` — indeks 3.
pub const ESRCH: i32 = 3;
/// `EIO` — indeks 5.
pub const EIO: i32 = 5;
/// `ENOEXEC` — indeks 8.
pub const ENOEXEC: i32 = 8;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `EROFS` — indeks 30.
pub const EROFS: i32 = 30;
/// `ENOBUFS` — indeks 59.
pub const ENOBUFS: i32 = 59;

/// Tag laporan; cermin `KlatosAdmisiProcReport` di `Jowo/Kontrak/KlatSeam92.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5052_4F43; // "KLATPROC"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_proc_ro`.
    ProdRo = 0,
    /// `klatos_proc_traverse`.
    ProdTraverse = 1,
    /// `klatos_proc_lookup_root`.
    ProdLookupRoot = 2,
    /// `klatos_proc_proses_hilang`.
    ProdLookupProses = 3,
    /// `klatos_proc_baca_cache`.
    ProdBacaCache = 4,
    /// `klatos_proc_refresh`.
    ProdRefresh = 5,
    /// `klatos_proc_cari`.
    ProdCari = 6,
    /// `klatos_proc_stack`.
    ProdStack = 7,
    /// `klatos_proc_perf`.
    ProdPerf = 8,
    /// `klatos_proc_biner`.
    ProdBiner = 9,
}

const N_COUNTER: usize = 10;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// `ProcFS::rename` selalu read-only. Transkripsi `ProcFS/FileSystem.cpp:49`.
#[inline]
pub const fn procfs_ro_admisi() -> i32 {
    -EROFS
}

/// `traverse_as_directory` pada proses yang sudah hilang.
/// Transkripsi `ProcFS/Inode.cpp:133,155`.
#[inline]
pub const fn procfs_traverse_admisi(proses_ada: u32) -> i32 {
    if proses_ada == 0 {
        return -EINVAL;
    }
    0
}

/// `lookup_as_root_directory`: nama bukan angka → `ESRCH`; proses hilang →
/// `ENOENT`. Transkripsi `ProcFS/Inode.cpp:164-173`.
#[inline]
pub const fn procfs_lookup_root_admisi(nama_angka: u32, proses_ada: u32) -> i32 {
    if nama_angka == 0 {
        return -ESRCH;
    }
    if proses_ada == 0 {
        return -ENOENT;
    }
    0
}

/// Proses hilang (lookup subdirektori, lookup direktori, dan jalur baca).
/// Transkripsi `ProcFS/Inode.cpp:182,204,253`.
#[inline]
pub const fn procfs_proses_hilang_admisi(proses_ada: u32) -> i32 {
    if proses_ada == 0 {
        return -ESRCH;
    }
    0
}

/// `read_bytes_locked` pada deskripsi tanpa cache data.
/// Transkripsi `ProcFS/Inode.cpp:265-267`.
#[inline]
pub const fn procfs_baca_cache_admisi(cache_ada: u32) -> i32 {
    if cache_ada == 0 {
        return -EIO;
    }
    0
}

/// `refresh_process_property_data`: proses hilang → `ESRCH`; tak dumpable →
/// `EPERM`. Transkripsi `ProcFS/Inode.cpp:344-349`.
#[inline]
pub const fn procfs_refresh_admisi(proses_ada: u32, dumpable: u32) -> i32 {
    if proses_ada == 0 {
        return -ESRCH;
    }
    if dumpable == 0 {
        return -EPERM;
    }
    0
}

/// Lookup nama tidak ditemukan (main/stacks/children/fd). Satu verdict, enam
/// pemanggil. Transkripsi `ProcFS/ProcessExposed.cpp:41,88,129,133,182,185`.
#[inline]
pub const fn procfs_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENOENT;
    }
    0
}

/// `procfs_get_thread_stack` pada tid yang sudah hilang.
/// Transkripsi `ProcFS/ProcessExposed.cpp:49`.
#[inline]
pub const fn procfs_stack_admisi(ada: u32) -> i32 {
    if ada == 0 {
        return -ESRCH;
    }
    0
}

/// `procfs_get_perf_events` tanpa perf event.
/// Transkripsi `ProcFS/ProcessExposed.cpp:246`.
#[inline]
pub const fn procfs_perf_admisi(ada: u32) -> i32 {
    if ada == 0 {
        return -ENOBUFS;
    }
    0
}

/// `procfs_get_binary_link` tanpa executable.
/// Transkripsi `ProcFS/ProcessExposed.cpp:378`.
#[inline]
pub const fn procfs_biner_admisi(ada: u32) -> i32 {
    if ada == 0 {
        return -ENOEXEC;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_procfs_traverse_admisi(proses_ada: u32) -> i32 {
    match proses_ada {
        1 => 0,
        _ => -EINVAL,
    }
}
fn naif_procfs_lookup_root_admisi(nama_angka: u32, proses_ada: u32) -> i32 {
    if nama_angka == 1 {
        if proses_ada == 1 {
            0
        } else {
            -ENOENT
        }
    } else {
        -ESRCH
    }
}
fn naif_procfs_proses_hilang_admisi(proses_ada: u32) -> i32 {
    match proses_ada {
        0 => -ESRCH,
        _ => 0,
    }
}
fn naif_procfs_baca_cache_admisi(cache_ada: u32) -> i32 {
    match cache_ada {
        0 => -EIO,
        _ => 0,
    }
}
fn naif_procfs_refresh_admisi(proses_ada: u32, dumpable: u32) -> i32 {
    if proses_ada != 1 {
        -ESRCH
    } else if dumpable != 1 {
        -EPERM
    } else {
        0
    }
}
fn naif_procfs_cari_admisi(ditemukan: u32) -> i32 {
    match ditemukan {
        0 => -ENOENT,
        _ => 0,
    }
}
fn naif_procfs_stack_admisi(ada: u32) -> i32 {
    match ada {
        1 => 0,
        _ => -ESRCH,
    }
}
fn naif_procfs_perf_admisi(ada: u32) -> i32 {
    match ada {
        0 => -ENOBUFS,
        _ => 0,
    }
}
fn naif_procfs_biner_admisi(ada: u32) -> i32 {
    match ada {
        0 => -ENOEXEC,
        _ => 0,
    }
}

/// Kontrol NEGATIF: `lookup_as_root_directory` yang salah mengembalikan
/// `ENOENT` (bukan `ESRCH`) untuk nama bukan angka. Sengaja SALAH.
fn procfs_lookup_root_salah(nama_angka: u32, proses_ada: u32) -> i32 {
    if nama_angka == 0 {
        return -ENOENT;
    }
    if proses_ada == 0 {
        return -ENOENT;
    }
    0
}

/// Kontrol NEGATIF: `refresh` yang salah mengembalikan `EACCES` (bukan `EPERM`)
/// untuk proses tak dumpable. Sengaja SALAH.
fn procfs_refresh_salah(proses_ada: u32, dumpable: u32) -> i32 {
    if proses_ada == 0 {
        return -ESRCH;
    }
    if dumpable == 0 {
        return -13; // EACCES
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`procfs_ro_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_ro() -> i32 {
    bump(Counter::ProdRo);
    procfs_ro_admisi()
}

/// Lihat [`procfs_traverse_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_traverse(proses_ada: u32) -> i32 {
    bump(Counter::ProdTraverse);
    procfs_traverse_admisi(proses_ada)
}

/// Lihat [`procfs_lookup_root_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_lookup_root(nama_angka: u32, proses_ada: u32) -> i32 {
    bump(Counter::ProdLookupRoot);
    procfs_lookup_root_admisi(nama_angka, proses_ada)
}

/// Lihat [`procfs_proses_hilang_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_proses_hilang(proses_ada: u32) -> i32 {
    bump(Counter::ProdLookupProses);
    procfs_proses_hilang_admisi(proses_ada)
}

/// Lihat [`procfs_baca_cache_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_baca_cache(cache_ada: u32) -> i32 {
    bump(Counter::ProdBacaCache);
    procfs_baca_cache_admisi(cache_ada)
}

/// Lihat [`procfs_refresh_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_refresh(proses_ada: u32, dumpable: u32) -> i32 {
    bump(Counter::ProdRefresh);
    procfs_refresh_admisi(proses_ada, dumpable)
}

/// Lihat [`procfs_cari_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_cari(ditemukan: u32) -> i32 {
    bump(Counter::ProdCari);
    procfs_cari_admisi(ditemukan)
}

/// Lihat [`procfs_stack_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_stack(ada: u32) -> i32 {
    bump(Counter::ProdStack);
    procfs_stack_admisi(ada)
}

/// Lihat [`procfs_perf_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_perf(ada: u32) -> i32 {
    bump(Counter::ProdPerf);
    procfs_perf_admisi(ada)
}

/// Lihat [`procfs_biner_admisi`].
#[no_mangle]
pub extern "C" fn klatos_proc_biner(ada: u32) -> i32 {
    bump(Counter::ProdBiner);
    procfs_biner_admisi(ada)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_proc_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_proc_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiProcReport` di `Jowo/Kontrak/KlatSeam92.h`.
#[repr(C)]
pub struct KlatosAdmisiProcReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub pohon_checks: u32,
    pub pohon_passed: u32,
    pub data_checks: u32,
    pub data_passed: u32,
    pub prod_ro: u32,
    pub prod_traverse: u32,
    pub prod_lookup_root: u32,
    pub prod_proses_hilang: u32,
    pub prod_baca_cache: u32,
    pub prod_refresh: u32,
    pub prod_cari: u32,
    pub prod_stack: u32,
    pub prod_perf: u32,
    pub prod_biner: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Pohon,
    Data,
}

struct Matrix {
    checks: u32,
    passed: u32,
    pohon_checks: u32,
    pohon_passed: u32,
    data_checks: u32,
    data_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            pohon_checks: 0,
            pohon_passed: 0,
            data_checks: 0,
            data_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Pohon => {
                self.pohon_checks += 1;
                if lulus {
                    self.pohon_passed += 1;
                }
            }
            Group::Data => {
                self.data_checks += 1;
                if lulus {
                    self.data_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **23**.
pub const DIFF_EXPECTED: u32 = 23;

fn run_matrix() -> KlatosAdmisiProcReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5052_4F43, Group::Pohon);
    m.catat(
        EPERM == 1 && ENOENT == 2 && ESRCH == 3 && EIO == 5 && ENOEXEC == 8 && EINVAL == 22
            && EROFS == 30 && ENOBUFS == 59,
        Group::Pohon,
    );

    // --- kasus utama ------------------------------------------------------
    m.catat(procfs_ro_admisi() == -EROFS, Group::Pohon);
    m.catat(procfs_traverse_admisi(0) == -EINVAL, Group::Pohon);
    m.catat(procfs_traverse_admisi(1) == 0, Group::Pohon);
    m.catat(procfs_lookup_root_admisi(0, 1) == -ESRCH, Group::Pohon);
    m.catat(procfs_lookup_root_admisi(1, 0) == -ENOENT, Group::Pohon);
    m.catat(procfs_lookup_root_admisi(1, 1) == 0, Group::Pohon);
    m.catat(procfs_proses_hilang_admisi(0) == -ESRCH, Group::Pohon);
    m.catat(procfs_proses_hilang_admisi(1) == 0, Group::Pohon);
    m.catat(procfs_cari_admisi(0) == -ENOENT, Group::Pohon);
    m.catat(procfs_cari_admisi(1) == 0, Group::Pohon);

    m.catat(procfs_baca_cache_admisi(0) == -EIO, Group::Data);
    m.catat(procfs_baca_cache_admisi(1) == 0, Group::Data);
    m.catat(procfs_refresh_admisi(0, 1) == -ESRCH, Group::Data);
    m.catat(procfs_refresh_admisi(1, 0) == -EPERM, Group::Data);
    m.catat(procfs_refresh_admisi(1, 1) == 0, Group::Data);
    m.catat(procfs_stack_admisi(0) == -ESRCH, Group::Data);
    m.catat(procfs_stack_admisi(1) == 0, Group::Data);
    m.catat(procfs_perf_admisi(0) == -ENOBUFS, Group::Data);
    m.catat(procfs_perf_admisi(1) == 0, Group::Data);
    m.catat(procfs_biner_admisi(0) == -ENOEXEC, Group::Data);
    m.catat(procfs_biner_admisi(1) == 0, Group::Data);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // ESRCH (nama bukan angka) bukan ENOENT.
        m.catat(procfs_lookup_root_admisi(0, 1) == -ESRCH, Group::Pohon);
        m.catat(procfs_lookup_root_salah(0, 1) == -ENOENT, Group::Pohon);
        m.catat(
            procfs_lookup_root_admisi(0, 1) != procfs_lookup_root_salah(0, 1),
            Group::Pohon,
        );
        m.catat(
            procfs_lookup_root_admisi(1, 1) == procfs_lookup_root_salah(1, 1),
            Group::Pohon,
        );

        // EPERM bukan EACCES pada proses tak dumpable.
        m.catat(procfs_refresh_admisi(1, 0) == -EPERM, Group::Data);
        m.catat(procfs_refresh_salah(1, 0) == -13, Group::Data);
        m.catat(
            procfs_refresh_admisi(1, 0) != procfs_refresh_salah(1, 0),
            Group::Data,
        );
        m.catat(
            procfs_refresh_admisi(1, 1) == procfs_refresh_salah(1, 1),
            Group::Data,
        );

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if procfs_ro_admisi() != -EROFS {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Pohon);
    }

    // --- diferensial produksi vs oracle (23 kasus) ------------------------
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
        ukur!(Group::Pohon, procfs_traverse_admisi(v), naif_procfs_traverse_admisi(v));
        ukur!(Group::Pohon, procfs_proses_hilang_admisi(v), naif_procfs_proses_hilang_admisi(v));
        ukur!(Group::Pohon, procfs_cari_admisi(v), naif_procfs_cari_admisi(v));
        ukur!(Group::Data, procfs_baca_cache_admisi(v), naif_procfs_baca_cache_admisi(v));
        ukur!(Group::Data, procfs_stack_admisi(v), naif_procfs_stack_admisi(v));
        ukur!(Group::Data, procfs_perf_admisi(v), naif_procfs_perf_admisi(v));
        ukur!(Group::Data, procfs_biner_admisi(v), naif_procfs_biner_admisi(v));
    }
    for a in 0..2u32 {
        for b in 0..2u32 {
            ukur!(
                Group::Pohon,
                procfs_lookup_root_admisi(a, b),
                naif_procfs_lookup_root_admisi(a, b)
            );
            ukur!(
                Group::Data,
                procfs_refresh_admisi(a, b),
                naif_procfs_refresh_admisi(a, b)
            );
        }
    }
    ukur!(Group::Pohon, procfs_ro_admisi(), -EROFS);

    m.catat(diukur == DIFF_EXPECTED, Group::Pohon);
    m.catat(beda == 0, Group::Pohon);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiProcReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        pohon_checks: m.pohon_checks,
        pohon_passed: m.pohon_passed,
        data_checks: m.data_checks,
        data_passed: m.data_passed,
        prod_ro: klatos_proc_counter(Counter::ProdRo as u32),
        prod_traverse: klatos_proc_counter(Counter::ProdTraverse as u32),
        prod_lookup_root: klatos_proc_counter(Counter::ProdLookupRoot as u32),
        prod_proses_hilang: klatos_proc_counter(Counter::ProdLookupProses as u32),
        prod_baca_cache: klatos_proc_counter(Counter::ProdBacaCache as u32),
        prod_refresh: klatos_proc_counter(Counter::ProdRefresh as u32),
        prod_cari: klatos_proc_counter(Counter::ProdCari as u32),
        prod_stack: klatos_proc_counter(Counter::ProdStack as u32),
        prod_perf: klatos_proc_counter(Counter::ProdPerf as u32),
        prod_biner: klatos_proc_counter(Counter::ProdBiner as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas ProcFS.
#[no_mangle]
pub extern "C" fn klatos_proc_selftest() -> KlatosAdmisiProcReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5052_4F43);
        assert!(EPERM == 1);
        assert!(ENOENT == 2);
        assert!(ESRCH == 3);
        assert!(EIO == 5);
        assert!(ENOEXEC == 8);
        assert!(EINVAL == 22);
        assert!(EROFS == 30);
        assert!(ENOBUFS == 59);
        assert!(Counter::ProdRo as usize == 0);
        assert!(Counter::ProdBiner as usize == 9);
        assert!(N_COUNTER == 10);
        assert!(DIFF_EXPECTED == 23);

        assert!(procfs_ro_admisi() == -EROFS);
        assert!(procfs_lookup_root_admisi(0, 1) == -ESRCH);
        assert!(procfs_refresh_admisi(1, 0) == -EPERM);

        // 8 (tag) + 19*4 (u32) + 4 (status) = 84 → 88 dengan padding 8-byte.
        assert!(core::mem::size_of::<KlatosAdmisiProcReport>() == 88);
        assert!(core::mem::align_of::<KlatosAdmisiProcReport>() == 8);
    };
}
