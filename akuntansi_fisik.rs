// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.71 / WS1 — **OTORITAS AKUNTANSI HALAMAN FISIK (COMMIT/UNCOMMIT)**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosAkunFisikReport` di `Jowo/Kontrak/KlatSeam71.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4146_4953; // "KLATAFIS"

/// Status hasil; cermin konstanta di `KlatSeam71.h`.
pub const AF_OK: i32 = 0;
/// Tidak cukup halaman (admission refusal).
pub const AF_ENOMEM: i32 = 1;
/// Argumen/keadaan tidak sah (menggantikan `VERIFY` C++).
pub const AF_EINVAL: i32 = 2;

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_af_komit` dari produksi.
    ProdKomit = 0,
    /// Jumlah `klatos_af_unkomit` dari produksi.
    ProdUnkomit = 1,
    /// Jumlah `klatos_af_ambil` dari produksi.
    ProdAmbil = 2,
    /// Jumlah `klatos_af_tandai_dipakai` dari produksi.
    ProdTandai = 3,
    /// Jumlah `klatos_af_kembalikan` dari produksi.
    ProdKembali = 4,
    /// Jumlah `klatos_af_alokasi_massal` dari produksi.
    ProdMassal = 5,
    /// Jumlah `klatos_af_invariant_sah` dari produksi.
    ProdInvariant = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Hasil transisi: status + tiga penghitung setelah (atau tanpa) perubahan.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hasil {
    pub status: i32,
    pub uncommitted: u64,
    pub committed: u64,
    pub used: u64,
}

const fn ok(uncommitted: u64, committed: u64, used: u64) -> Hasil {
    Hasil { status: AF_OK, uncommitted, committed, used }
}

const fn tolak(status: i32, uncommitted: u64, committed: u64, used: u64) -> Hasil {
    Hasil { status, uncommitted, committed, used }
}

/// Commit: `uncommitted` harus menutupi `minta`, lalu dipindahkan ke `committed`.
/// Cermin `MemoryManager::commit_physical_pages`.
#[inline]
pub const fn komit(uncommitted: u64, committed: u64, used: u64, minta: u64) -> Hasil {
    if minta == 0 {
        return tolak(AF_EINVAL, uncommitted, committed, used);
    }
    if uncommitted < minta {
        return tolak(AF_ENOMEM, uncommitted, committed, used);
    }
    ok(uncommitted - minta, committed + minta, used)
}

/// Uncommit: pindahkan `minta` dari `committed` kembali ke `uncommitted`.
/// Cermin `MemoryManager::uncommit_physical_pages` (`committed >= minta` adalah
/// `VERIFY` di C++; di sini menjadi status `AF_EINVAL`).
#[inline]
pub const fn unkomit(uncommitted: u64, committed: u64, used: u64, minta: u64) -> Hasil {
    if minta == 0 || committed < minta {
        return tolak(AF_EINVAL, uncommitted, committed, used);
    }
    ok(uncommitted + minta, committed - minta, used)
}

/// Ambil satu halaman dari pool terpilih. `terkomit != 0` ⇒ pool `committed`
/// (harus `> 0`, `VERIFY` di C++), selain itu pool `uncommitted` (boleh
/// `ENOMEM`). `used` TIDAK berubah di sini — kenaikan `used` terjadi setelah
/// region benar-benar menyerahkan halaman ([`tandai_dipakai`]).
/// Cermin `MemoryManager::find_free_physical_page` bagian penghitung.
#[inline]
pub const fn ambil(uncommitted: u64, committed: u64, used: u64, terkomit: u32) -> Hasil {
    if terkomit != 0 {
        if committed == 0 {
            return tolak(AF_EINVAL, uncommitted, committed, used);
        }
        return ok(uncommitted, committed - 1, used);
    }
    if uncommitted == 0 {
        return tolak(AF_ENOMEM, uncommitted, committed, used);
    }
    ok(uncommitted - 1, committed, used)
}

/// Tandai satu halaman benar-benar dipakai (`used += 1`).
/// Cermin `++physical_pages_used` di `find_free_physical_page`.
#[inline]
pub const fn tandai_dipakai(uncommitted: u64, committed: u64, used: u64) -> Hasil {
    ok(uncommitted, committed, used + 1)
}

/// Kembalikan satu halaman: `used -= 1`, `uncommitted += 1`.
/// Cermin `MemoryManager::deallocate_physical_page` bagian penghitung.
#[inline]
pub const fn kembalikan(uncommitted: u64, committed: u64, used: u64) -> Hasil {
    if used == 0 {
        return tolak(AF_EINVAL, uncommitted, committed, used);
    }
    ok(uncommitted + 1, committed, used - 1)
}

/// Alokasi massal (kontigu): `uncommitted` harus menutupi `minta`, lalu
/// `minta` langsung menjadi `used`. Cermin
/// `MemoryManager::allocate_contiguous_physical_pages`.
#[inline]
pub const fn alokasi_massal(uncommitted: u64, committed: u64, used: u64, minta: u64) -> Hasil {
    if minta == 0 {
        return tolak(AF_EINVAL, uncommitted, committed, used);
    }
    if uncommitted < minta {
        return tolak(AF_ENOMEM, uncommitted, committed, used);
    }
    ok(uncommitted - minta, committed, used + minta)
}

/// Invariant total: `physical_pages == used + committed + uncommitted`.
/// Cermin `VERIFY` di `MemoryManager.cpp:1803-1804`.
#[inline]
pub const fn invariant_sah(physical_pages: u64, uncommitted: u64, committed: u64, used: u64) -> u32 {
    if physical_pages == used + committed + uncommitted {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Cermin `KlatosAkunFisik` di `Jowo/Kontrak/KlatSeam71.h`.
#[repr(C)]
pub struct KlatosAkunFisik {
    pub status: i32,
    pub _pad: u32,
    pub uncommitted: u64,
    pub committed: u64,
    pub used: u64,
}

const fn ffi(h: Hasil) -> KlatosAkunFisik {
    KlatosAkunFisik {
        status: h.status,
        _pad: 0,
        uncommitted: h.uncommitted,
        committed: h.committed,
        used: h.used,
    }
}

#[no_mangle]
pub extern "C" fn klatos_af_komit(uncommitted: u64, committed: u64, used: u64, minta: u64) -> KlatosAkunFisik {
    bump(Counter::ProdKomit);
    ffi(komit(uncommitted, committed, used, minta))
}

#[no_mangle]
pub extern "C" fn klatos_af_unkomit(uncommitted: u64, committed: u64, used: u64, minta: u64) -> KlatosAkunFisik {
    bump(Counter::ProdUnkomit);
    ffi(unkomit(uncommitted, committed, used, minta))
}

#[no_mangle]
pub extern "C" fn klatos_af_ambil(uncommitted: u64, committed: u64, used: u64, terkomit: u32) -> KlatosAkunFisik {
    bump(Counter::ProdAmbil);
    ffi(ambil(uncommitted, committed, used, terkomit))
}

#[no_mangle]
pub extern "C" fn klatos_af_tandai_dipakai(uncommitted: u64, committed: u64, used: u64) -> KlatosAkunFisik {
    bump(Counter::ProdTandai);
    ffi(tandai_dipakai(uncommitted, committed, used))
}

#[no_mangle]
pub extern "C" fn klatos_af_kembalikan(uncommitted: u64, committed: u64, used: u64) -> KlatosAkunFisik {
    bump(Counter::ProdKembali);
    ffi(kembalikan(uncommitted, committed, used))
}

#[no_mangle]
pub extern "C" fn klatos_af_alokasi_massal(uncommitted: u64, committed: u64, used: u64, minta: u64) -> KlatosAkunFisik {
    bump(Counter::ProdMassal);
    ffi(alokasi_massal(uncommitted, committed, used, minta))
}

#[no_mangle]
pub extern "C" fn klatos_af_invariant_sah(physical_pages: u64, uncommitted: u64, committed: u64, used: u64) -> u32 {
    bump(Counter::ProdInvariant);
    invariant_sah(physical_pages, uncommitted, committed, used)
}

#[no_mangle]
pub extern "C" fn klatos_af_counter(idx: u32) -> u32 {
    if (idx as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[idx as usize].load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn klatos_af_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAkunFisikReport` di `Jowo/Kontrak/KlatSeam71.h`.
#[repr(C)]
pub struct KlatosAkunFisikReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub komit_checks: u32,
    pub komit_passed: u32,
    pub ambil_checks: u32,
    pub ambil_passed: u32,
    pub kembali_checks: u32,
    pub kembali_passed: u32,
    pub inv_checks: u32,
    pub inv_passed: u32,
    pub prod_komit: u32,
    pub prod_unkomit: u32,
    pub prod_ambil: u32,
    pub prod_tandai: u32,
    pub prod_kembali: u32,
    pub prod_massal: u32,
    pub prod_invariant: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Komit,
    Ambil,
    Kembali,
    Inv,
}

struct Matrix {
    checks: u32,
    passed: u32,
    komit_checks: u32,
    komit_passed: u32,
    ambil_checks: u32,
    ambil_passed: u32,
    kembali_checks: u32,
    kembali_passed: u32,
    inv_checks: u32,
    inv_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            komit_checks: 0,
            komit_passed: 0,
            ambil_checks: 0,
            ambil_passed: 0,
            kembali_checks: 0,
            kembali_passed: 0,
            inv_checks: 0,
            inv_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Komit => {
                self.komit_checks += 1;
                if lulus {
                    self.komit_passed += 1;
                }
            }
            Group::Ambil => {
                self.ambil_checks += 1;
                if lulus {
                    self.ambil_passed += 1;
                }
            }
            Group::Kembali => {
                self.kembali_checks += 1;
                if lulus {
                    self.kembali_passed += 1;
                }
            }
            Group::Inv => {
                self.inv_checks += 1;
                if lulus {
                    self.inv_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Matriks otoritas akuntansi halaman fisik. Input SINTETIS (tanpa alokasi).
fn run_matrix() -> KlatosAkunFisikReport {
    let mut m = Matrix::new();

    // --- kelompok 0: commit / uncommit -------------------------------------
    // Cukup: 100 uncommitted, minta 30 ⇒ ok, 70/30/0.
    m.catat(komit(100, 0, 0, 30) == ok(70, 30, 0), Group::Komit);
    // Kurang: 10 uncommitted, minta 30 ⇒ ENOMEM, tidak berubah.
    m.catat(komit(10, 0, 0, 30) == tolak(AF_ENOMEM, 10, 0, 0), Group::Komit);
    // Persis cukup.
    m.catat(komit(30, 0, 0, 30) == ok(0, 30, 0), Group::Komit);
    // Nol diminta ⇒ EINVAL (C++ VERIFY(page_count > 0)).
    m.catat(komit(30, 0, 0, 0) == tolak(AF_EINVAL, 30, 0, 0), Group::Komit);
    // uncommit memindahkan kembali.
    m.catat(unkomit(70, 30, 0, 30) == ok(100, 0, 0), Group::Komit);
    // uncommit lebih dari committed ⇒ EINVAL.
    m.catat(unkomit(70, 30, 0, 31) == tolak(AF_EINVAL, 70, 30, 0), Group::Komit);
    // uncommit nol ⇒ EINVAL.
    m.catat(unkomit(70, 30, 0, 0) == tolak(AF_EINVAL, 70, 30, 0), Group::Komit);

    // --- kelompok 1: ambil satu halaman ------------------------------------
    // Pool committed: committed 5 ⇒ 4, used tetap.
    m.catat(ambil(10, 5, 7, 1) == ok(10, 4, 7), Group::Ambil);
    // Pool committed kosong ⇒ EINVAL (VERIFY).
    m.catat(ambil(10, 0, 7, 1) == tolak(AF_EINVAL, 10, 0, 7), Group::Ambil);
    // Pool uncommitted: 10 ⇒ 9.
    m.catat(ambil(10, 5, 7, 0) == ok(9, 5, 7), Group::Ambil);
    // Pool uncommitted kosong ⇒ ENOMEM.
    m.catat(ambil(0, 5, 7, 0) == tolak(AF_ENOMEM, 0, 5, 7), Group::Ambil);
    // Tandai dipakai menaikkan used.
    m.catat(tandai_dipakai(9, 5, 7) == ok(9, 5, 8), Group::Ambil);

    // --- kelompok 2: kembalikan halaman ------------------------------------
    m.catat(kembalikan(9, 5, 8) == ok(10, 5, 7), Group::Kembali);
    // Kembalikan saat used 0 ⇒ EINVAL.
    m.catat(kembalikan(9, 5, 0) == tolak(AF_EINVAL, 9, 5, 0), Group::Kembali);
    // Alokasi massal: cukup.
    m.catat(alokasi_massal(100, 0, 0, 40) == ok(60, 0, 40), Group::Kembali);
    // Alokasi massal: kurang ⇒ ENOMEM.
    m.catat(alokasi_massal(10, 0, 0, 40) == tolak(AF_ENOMEM, 10, 0, 0), Group::Kembali);
    // Alokasi massal nol ⇒ EINVAL.
    m.catat(alokasi_massal(10, 0, 0, 0) == tolak(AF_EINVAL, 10, 0, 0), Group::Kembali);

    // --- kelompok 3: invariant ---------------------------------------------
    m.catat(invariant_sah(100, 70, 30, 0) == 1, Group::Inv);
    m.catat(invariant_sah(100, 70, 30, 1) == 0, Group::Inv);
    m.catat(invariant_sah(0, 0, 0, 0) == 1, Group::Inv);
    m.catat(invariant_sah(1, 0, 0, 1) == 1, Group::Inv);
    m.catat(invariant_sah(2, 0, 0, 1) == 0, Group::Inv);

    // Rantai commit→ambil→tandai→kembali→unkomit harus menjaga invariant.
    // Mulai: total 100, uncommitted 100.
    let s1 = komit(100, 0, 0, 20); // 80/20/0
    let s2 = ambil(s1.uncommitted, s1.committed, s1.used, 1); // committed 19
    let s3 = tandai_dipakai(s2.uncommitted, s2.committed, s2.used); // used 1
    let s4 = kembalikan(s3.uncommitted, s3.committed, s3.used); // uncommitted 81, used 0
    m.catat(invariant_sah(100, s4.uncommitted, s4.committed, s4.used) == 1, Group::Inv);

    KlatosAkunFisikReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        komit_checks: m.komit_checks,
        komit_passed: m.komit_passed,
        ambil_checks: m.ambil_checks,
        ambil_passed: m.ambil_passed,
        kembali_checks: m.kembali_checks,
        kembali_passed: m.kembali_passed,
        inv_checks: m.inv_checks,
        inv_passed: m.inv_passed,
        prod_komit: COUNTERS[Counter::ProdKomit as usize].load(Ordering::Relaxed),
        prod_unkomit: COUNTERS[Counter::ProdUnkomit as usize].load(Ordering::Relaxed),
        prod_ambil: COUNTERS[Counter::ProdAmbil as usize].load(Ordering::Relaxed),
        prod_tandai: COUNTERS[Counter::ProdTandai as usize].load(Ordering::Relaxed),
        prod_kembali: COUNTERS[Counter::ProdKembali as usize].load(Ordering::Relaxed),
        prod_massal: COUNTERS[Counter::ProdMassal as usize].load(Ordering::Relaxed),
        prod_invariant: COUNTERS[Counter::ProdInvariant as usize].load(Ordering::Relaxed),
        status: if m.checks == m.passed { 0 } else { 1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_af_selftest() -> KlatosAkunFisikReport {
    run_matrix()
}
