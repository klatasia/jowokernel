// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.70 / WS1 — **OTORITAS PENCARIAN & CAKUPAN WILAYAH**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosWilayahCariReport` di `Jowo/Kontrak/KlatSeam70.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5743_4152; // "KLATWCAR"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_wcar_cocok_persis` dari produksi.
    ProdCocok = 0,
    /// Jumlah `klatos_wcar_serap` dari produksi.
    ProdSerap = 1,
    /// Jumlah `klatos_wcar_cakup` dari produksi.
    ProdCakup = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Kecocokan persis: ukuran wilayah == ukuran rentang (yang sudah dibulatkan
/// halaman). Cermin `AddressSpace.cpp:272` (`region.size() != rounded`).
#[inline]
pub const fn cocok_persis(ukuran_wilayah: u64, ukuran_rentang: u64) -> u32 {
    if ukuran_wilayah == ukuran_rentang {
        1
    } else {
        0
    }
}

/// Serapan: ukuran bagian `a` yang berada DI DALAM `b`.
/// Cermin `iter_range.intersect(range).size()` (`AddressSpace.cpp:306`).
///
/// `max(0, min(a_hi, b_hi) - max(a_lo, b_lo))`. Bernilai `> 0` **tepat bila**
/// kedua rentang bertumpang (tepi bersebelahan dan rentang kosong ⇒ 0), jadi
/// nilai ini sekaligus menjadi predikat tumpang-tindih.
#[inline]
pub const fn serap(a_lo: u64, a_hi: u64, b_lo: u64, b_hi: u64) -> u64 {
    let lo = if a_lo > b_lo { a_lo } else { b_lo };
    let hi = if a_hi < b_hi { a_hi } else { b_hi };
    if hi > lo {
        hi - lo
    } else {
        0
    }
}

/// Cakupan lengkap: akumulasi serapan sudah mencapai ukuran rentang.
/// Cermin `AddressSpace.cpp:307` (`total_size_collected == range.size()`).
#[inline]
pub const fn cakup(sudah: u64, perlu: u64) -> u32 {
    if sudah == perlu {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_wcar_cocok_persis(ukuran_wilayah: u64, ukuran_rentang: u64) -> u32 {
    bump(Counter::ProdCocok);
    cocok_persis(ukuran_wilayah, ukuran_rentang)
}

#[no_mangle]
pub extern "C" fn klatos_wcar_serap(a_lo: u64, a_hi: u64, b_lo: u64, b_hi: u64) -> u64 {
    bump(Counter::ProdSerap);
    serap(a_lo, a_hi, b_lo, b_hi)
}

#[no_mangle]
pub extern "C" fn klatos_wcar_cakup(sudah: u64, perlu: u64) -> u32 {
    bump(Counter::ProdCakup);
    cakup(sudah, perlu)
}

#[no_mangle]
pub extern "C" fn klatos_wcar_counter(idx: u32) -> u32 {
    if (idx as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[idx as usize].load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn klatos_wcar_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosWilayahCariReport` di `Jowo/Kontrak/KlatSeam70.h`.
#[repr(C)]
pub struct KlatosWilayahCariReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub cocok_checks: u32,
    pub cocok_passed: u32,
    pub serap_checks: u32,
    pub serap_passed: u32,
    pub cakup_checks: u32,
    pub cakup_passed: u32,
    pub prod_cocok: u32,
    pub prod_serap: u32,
    pub prod_cakup: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Cocok,
    Serap,
    Cakup,
}

struct Matrix {
    checks: u32,
    passed: u32,
    cocok_checks: u32,
    cocok_passed: u32,
    serap_checks: u32,
    serap_passed: u32,
    cakup_checks: u32,
    cakup_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            cocok_checks: 0,
            cocok_passed: 0,
            serap_checks: 0,
            serap_passed: 0,
            cakup_checks: 0,
            cakup_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Cocok => {
                self.cocok_checks += 1;
                if lulus {
                    self.cocok_passed += 1;
                }
            }
            Group::Serap => {
                self.serap_checks += 1;
                if lulus {
                    self.serap_passed += 1;
                }
            }
            Group::Cakup => {
                self.cakup_checks += 1;
                if lulus {
                    self.cakup_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Matriks otoritas pencarian/cakupan. Input SINTETIS (tanpa alokasi).
fn run_matrix() -> KlatosWilayahCariReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kecocokan persis --------------------------------------
    m.catat(cocok_persis(4096, 4096) == 1, Group::Cocok);
    m.catat(cocok_persis(8192, 4096) == 0, Group::Cocok);
    m.catat(cocok_persis(0, 0) == 1, Group::Cocok);
    m.catat(cocok_persis(4096, 8192) == 0, Group::Cocok);
    m.catat(cocok_persis(0, 4096) == 0, Group::Cocok);

    // --- kelompok 1: serapan (F-261) ---------------------------------------
    // Wilayah tepat sama dengan rentang ⇒ serap = ukuran penuh.
    m.catat(serap(0x1000, 0x3000, 0x1000, 0x3000) == 0x2000, Group::Serap);
    // Wilayah lebih besar: hanya bagian DALAM rentang yang dihitung.
    m.catat(serap(0x0000, 0x4000, 0x1000, 0x3000) == 0x2000, Group::Serap);
    // Wilayah lebih besar di kiri: serap = seluruh rentang.
    m.catat(serap(0x0000, 0x2000, 0x1000, 0x3000) == 0x1000, Group::Serap);
    // Wilayah di kiri rentang: serap 0.
    m.catat(serap(0x0000, 0x1000, 0x1000, 0x3000) == 0, Group::Serap);
    // Wilayah di kanan rentang: serap 0.
    m.catat(serap(0x3000, 0x4000, 0x1000, 0x3000) == 0, Group::Serap);
    // Tepi bersebelahan: [0x1000,0x2000) vs [0x2000,0x3000) ⇒ 0 (tegas).
    m.catat(serap(0x1000, 0x2000, 0x2000, 0x3000) == 0, Group::Serap);
    // Serap > 0 ⟺ tumpang (satu bit pun).
    m.catat(serap(0x1000, 0x2001, 0x2000, 0x3000) == 1, Group::Serap);
    // Rentang kosong tidak pernah menyerap.
    m.catat(serap(0x1000, 0x2000, 0x2000, 0x2000) == 0, Group::Serap);
    // F-261 regression: rumus LAMA (luar-rentang) memberi 0x1000 untuk wilayah
    // yang berada sepenuhnya DI LUAR rentang; rumus baru harus memberi 0.
    m.catat(serap(0x0000, 0x1000, 0x1000, 0x3000) == 0, Group::Serap);
    m.catat(serap(0x3000, 0x4000, 0x1000, 0x3000) == 0, Group::Serap);

    // --- kelompok 2: cakupan / keluar-awal ---------------------------------
    // Akumulasi dua wilayah [0x1000,0x2000)+[0x2000,0x3000) menutup
    // rentang [0x1000,0x3000): cakup benar pada 0x2000, bukan lebih awal.
    let mut total = 0u64;
    total += serap(0x1000, 0x2000, 0x1000, 0x3000);
    m.catat(cakup(total, 0x2000) == 0, Group::Cakup);
    total += serap(0x2000, 0x3000, 0x1000, 0x3000);
    m.catat(cakup(total, 0x2000) == 1, Group::Cakup);
    // Cakupan nol rentang = langsung lengkap (rentang kosong).
    m.catat(cakup(0, 0) == 1, Group::Cakup);
    // Cakupan belum penuh.
    m.catat(cakup(0x1000, 0x2000) == 0, Group::Cakup);
    // Cakupan melebihi target tidak pernah dianggap "==" (harus tepat).
    m.catat(cakup(0x3000, 0x2000) == 0, Group::Cakup);
    // F-261: akumulasi dari wilayah DI LUAR tidak boleh menutup rentang.
    let mut total_luar = 0u64;
    total_luar += serap(0x0000, 0x1000, 0x1000, 0x3000);
    m.catat(cakup(total_luar, 0x2000) == 0, Group::Cakup);

    KlatosWilayahCariReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        cocok_checks: m.cocok_checks,
        cocok_passed: m.cocok_passed,
        serap_checks: m.serap_checks,
        serap_passed: m.serap_passed,
        cakup_checks: m.cakup_checks,
        cakup_passed: m.cakup_passed,
        prod_cocok: COUNTERS[Counter::ProdCocok as usize].load(Ordering::Relaxed),
        prod_serap: COUNTERS[Counter::ProdSerap as usize].load(Ordering::Relaxed),
        prod_cakup: COUNTERS[Counter::ProdCakup as usize].load(Ordering::Relaxed),
        status: if m.checks == m.passed { 0 } else { 1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_wcar_selftest() -> KlatosWilayahCariReport {
    run_matrix()
}
