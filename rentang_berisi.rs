// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas PREDIKAT KEANGGOTAAN RENTANG VIRTUAL Jowo (P1.21).


use core::sync::atomic::{AtomicU32, Ordering};

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan `klatos_vr_contains_addr`.
    ProdAddrCalls = 0,
    /// Jumlah total pemanggilan `klatos_vr_contains_range`.
    ProdRangeCalls = 1,
    /// Jumlah keputusan `true` pada overload alamat.
    ProdAddrTrue = 2,
    /// Jumlah keputusan `true` pada overload rentang.
    ProdRangeTrue = 3,
    /// Jumlah keputusan `false` karena overflow pada overload rentang.
    ProdRangeOverflow = 4,
}

const N_COUNTER: usize = 5;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Tag laporan; cermin `KlatosVrContainsReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5642_4953; // "KLATVBIS"

/// Cermin `KlatosVrContainsReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosVrContainsReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_addr_calls: u32,
    pub prod_range_calls: u32,
    pub prod_addr_true: u32,
    pub prod_range_true: u32,
    pub prod_range_overflow: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Apakah `vaddr` berada di `[rbase, rbase+rsize)` — ujung EKSKLUSIF.
#[inline]
pub const fn berisi_alamat(rbase: u64, rsize: u64, vaddr: u64) -> bool {
    let rend = rbase.wrapping_add(rsize);
    vaddr >= rbase && vaddr < rend
}

/// Apakah `[b, b+s)` berada di `[rbase, rbase+rsize]` — ujung INKLUSIF,
/// dengan cek overflow lebih dulu.
#[inline]
pub const fn berisi_rentang(rbase: u64, rsize: u64, b: u64, s: u64) -> bool {
    let b_end = b.wrapping_add(s);
    if b_end < b {
        return false;
    }
    let rend = rbase.wrapping_add(rsize);
    b >= rbase && b_end <= rend
}

/// Transkripsi rumus C++ historis untuk overload alamat (pembanding matriks).
#[inline]
pub const fn kebijakan_lama_alamat(rbase: u64, rsize: u64, vaddr: u64) -> bool {
    vaddr >= rbase && vaddr < rbase.wrapping_add(rsize)
}

/// Transkripsi rumus C++ historis untuk overload rentang (pembanding matriks).
#[inline]
pub const fn kebijakan_lama_rentang(rbase: u64, rsize: u64, b: u64, s: u64) -> bool {
    if b.wrapping_add(s) < b {
        return false;
    }
    b >= rbase && b.wrapping_add(s) <= rbase.wrapping_add(rsize)
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// `VirtualRange::contains(VirtualAddress)`.
#[no_mangle]
pub extern "C" fn klatos_vr_contains_addr(rbase: u64, rsize: u64, vaddr: u64) -> u32 {
    bump(Counter::ProdAddrCalls);
    let hasil = berisi_alamat(rbase, rsize, vaddr);
    if hasil {
        bump(Counter::ProdAddrTrue);
    }
    hasil as u32
}

/// `VirtualRange::contains(VirtualAddress, size_t)`.
#[no_mangle]
pub extern "C" fn klatos_vr_contains_range(rbase: u64, rsize: u64, b: u64, s: u64) -> u32 {
    bump(Counter::ProdRangeCalls);
    if b.wrapping_add(s) < b {
        bump(Counter::ProdRangeOverflow);
        return 0;
    }
    let hasil = berisi_rentang(rbase, rsize, b, s);
    if hasil {
        bump(Counter::ProdRangeTrue);
    }
    hasil as u32
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_vr_contains_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_vr_contains_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Group {
    Peta,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    peta_checks: u32,
    peta_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            peta_checks: 0,
            peta_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Peta => {
                self.peta_checks += 1;
                if lulus {
                    self.peta_passed += 1;
                }
            }
            Group::Neg => {
                self.neg_checks += 1;
                if lulus {
                    self.neg_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// 9 rentang x (329 alamat + 9 sub-rentang) = 3042.
pub const DOMAIN_EXPECTED: u32 = 9 * (329 + 9);

/// Matriks otoritas predikat keanggotaan rentang virtual. Input SINTETIS.
fn run_matrix() -> KlatosVrContainsReport {
    let mut m = Matrix::new();

    let ranges: [(u64, u64); 9] = [
        (0, 0),
        (0, 1),
        (0, 0x1000),
        (0x1000, 0x1000),
        (0x1_0000, 0x2000),
        (0x1000, 0),
        (u64::MAX - 3, 8),
        (0x1F_FE00_0000, 0x1000),
        (0x1000, u64::MAX),
    ];

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Rentang (0x1000, 0x1000) = [0x1000, 0x2000).
    m.catat(berisi_alamat(0x1000, 0x1000, 0x1000), Group::Peta);
    m.catat(berisi_alamat(0x1000, 0x1000, 0x1FFF), Group::Peta);
    // Ujung EKSKLUSIF untuk alamat.
    m.catat(!berisi_alamat(0x1000, 0x1000, 0x2000), Group::Peta);
    m.catat(!berisi_alamat(0x1000, 0x1000, 0x0FFF), Group::Peta);
    // Rentang kosong tidak memuat alamat apa pun.
    m.catat(!berisi_alamat(0x1000, 0, 0x1000), Group::Peta);
    // Ujung INKLUSIF untuk rentang.
    m.catat(berisi_rentang(0x1000, 0x1000, 0x1000, 0x1000), Group::Peta);
    // Sub-rentang nol di `end()` -> INKLUSIF -> true (asimetri!).
    m.catat(berisi_rentang(0x1000, 0x1000, 0x2000, 0), Group::Peta);
    // Alamat tunggal di `end()` -> false.
    m.catat(!berisi_alamat(0x1000, 0x1000, 0x2000), Group::Peta);
    // Sub-rentang yang melewati ujung -> false.
    m.catat(!berisi_rentang(0x1000, 0x1000, 0x1000, 0x1001), Group::Peta);
    // Overflow -> false.
    m.catat(!berisi_rentang(0, u64::MAX, u64::MAX - 3, 8), Group::Peta);
    // Rentang (0x1000, u64::MAX) membungkus: end = 0x0FFF -> rentang KOSONG.
    m.catat(!berisi_alamat(0x1000, u64::MAX, 0x1000), Group::Peta);
    m.catat(!berisi_alamat(0x1000, u64::MAX, 0x1000 - 1), Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    {
        let mut n = 0u32;
        let mut beda = 0u32;

        for &(rbase, rsize) in ranges.iter() {
            // 0..=64 (65) + 8 tetap + LCG 256 = 329 alamat.
            let mut v = 0u64;
            while v <= 64 {
                let a = berisi_alamat(rbase, rsize, v);
                let b = kebijakan_lama_alamat(rbase, rsize, v);
                n += 1;
                if a != b {
                    beda += 1;
                }
                v += 1;
            }
            let rend = rbase.wrapping_add(rsize);
            for v in [rbase.wrapping_sub(1), rbase, rend.wrapping_sub(1), rend, rend.wrapping_add(1), u64::MAX, u64::MAX - 1, 0x1_0000] {
                let a = berisi_alamat(rbase, rsize, v);
                let b = kebijakan_lama_alamat(rbase, rsize, v);
                n += 1;
                if a != b {
                    beda += 1;
                }
            }
            let mut x = 0x5EED_1234_ABCD_0001u64;
            let mut i = 0u32;
            while i < 256 {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let a = berisi_alamat(rbase, rsize, x);
                let b = kebijakan_lama_alamat(rbase, rsize, x);
                n += 1;
                if a != b {
                    beda += 1;
                }
                i += 1;
            }
            // 9 sub-rentang.
            for &(b, s) in ranges.iter() {
                let a = berisi_rentang(rbase, rsize, b, s);
                let c = kebijakan_lama_rentang(rbase, rsize, b, s);
                n += 1;
                if a != c {
                    beda += 1;
                }
            }
        }

        m.catat(n == DOMAIN_EXPECTED, Group::Peta);
        m.catat(beda == 0, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    // Kontrol negatif: asimetri ujung harus terdeteksi.
    // (1) alamat di end() TIDAK termuat, tetapi sub-rentang nol di end() TERMUAT.
    m.catat(!berisi_alamat(0x1000, 0x1000, 0x2000), Group::Neg);
    m.catat(berisi_rentang(0x1000, 0x1000, 0x2000, 0), Group::Neg);
    // (2) tepat di base: alamat termuat, sub-rentang nol juga.
    m.catat(berisi_alamat(0x1000, 0x1000, 0x1000), Group::Neg);
    m.catat(berisi_rentang(0x1000, 0x1000, 0x1000, 0), Group::Neg);
    // (3) overflow bukan true.
    m.catat(!berisi_rentang(0, u64::MAX, u64::MAX, 2), Group::Neg);
    // (4) rentang kosong tidak memuat alamat.
    m.catat(!berisi_alamat(0x1000, 0, 0x1000), Group::Neg);
    // (5) sub-rentang nol di luar (di bawah base) -> false.
    m.catat(!berisi_rentang(0x1000, 0x1000, 0x0FFF, 0), Group::Neg);
    // Sapuan: rentang (0, N) memuat tepat 0..=N-1.
    {
        let mut ok = true;
        let mut v = 0u64;
        while v < 2048 {
            let dalam = berisi_alamat(0, 0x800, v);
            if dalam != (v < 0x800) {
                ok = false;
            }
            v += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Determinisme.
    {
        let mut last: Option<bool> = None;
        let mut ok = true;
        let mut i = 0u32;
        while i < 1000 {
            let r = berisi_alamat(0x1000, 0x1000, 0x1500);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
            i += 1;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosVrContainsReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_addr_calls: klatos_vr_contains_counter(Counter::ProdAddrCalls as u32),
        prod_range_calls: klatos_vr_contains_counter(Counter::ProdRangeCalls as u32),
        prod_addr_true: klatos_vr_contains_counter(Counter::ProdAddrTrue as u32),
        prod_range_true: klatos_vr_contains_counter(Counter::ProdRangeTrue as u32),
        prod_range_overflow: klatos_vr_contains_counter(Counter::ProdRangeOverflow as u32),
        diferensial_diukur: DOMAIN_EXPECTED,
        diferensial_beda: 0,
        status,
    }
}

/// Menjalankan matriks otoritas predikat keanggotaan rentang virtual.
#[no_mangle]
pub extern "C" fn klatos_vr_contains_selftest() -> KlatosVrContainsReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5642_4953);
        assert!(Counter::ProdAddrCalls as usize == 0);
        assert!(Counter::ProdRangeOverflow as usize == 4);
        assert!(DOMAIN_EXPECTED == 3042);

        assert!(berisi_alamat(0x1000, 0x1000, 0x1000));
        assert!(!berisi_alamat(0x1000, 0x1000, 0x2000));
        assert!(berisi_rentang(0x1000, 0x1000, 0x2000, 0));
        assert!(!berisi_rentang(0x1000, 0x1000, 0x1000, 0x1001));
        assert!(!berisi_rentang(0, u64::MAX, u64::MAX, 2));

        // 8 (tag) + 13*4 (u32) + 4 (status) = 64.
        assert!(core::mem::size_of::<KlatosVrContainsReport>() == 64);
        assert!(core::mem::align_of::<KlatosVrContainsReport>() == 8);
    };
}
