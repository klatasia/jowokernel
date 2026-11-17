// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas KLASIFIKASI RUANG WILAYAH Jowo (P1.22).


use core::sync::atomic::{AtomicU32, Ordering};

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan `klatos_region_is_kernel`.
    ProdKernelCalls = 0,
    /// Jumlah keputusan `true` pada `klatos_region_is_kernel`.
    ProdKernelTrue = 1,
    /// Jumlah total pemanggilan `klatos_region_is_user`.
    ProdUserCalls = 2,
    /// Jumlah keputusan `true` pada `klatos_region_is_user`.
    ProdUserTrue = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Tag laporan; cermin `KlatosWilayahRuangReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_574C_5948; // "KLATWLYH"

/// Cermin `KlatosWilayahRuangReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosWilayahRuangReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_kernel_calls: u32,
    pub prod_kernel_true: u32,
    pub prod_user_calls: u32,
    pub prod_user_true: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Apakah `vaddr` berada di wilayah kernel (dua batas).
#[inline]
pub const fn adalah_wilayah_kernel(vaddr: u64, user_range_base: u64, kernel_mapping_base: u64) -> bool {
    vaddr < user_range_base || vaddr >= kernel_mapping_base
}

/// Apakah `vaddr` berada di wilayah pengguna. Cermin `!is_kernel()`.
#[inline]
pub const fn adalah_wilayah_pengguna(vaddr: u64, user_range_base: u64, kernel_mapping_base: u64) -> bool {
    !adalah_wilayah_kernel(vaddr, user_range_base, kernel_mapping_base)
}

/// Transkripsi rumus C++ historis (pembanding matriks).
#[inline]
pub const fn kebijakan_lama(vaddr: u64, user_range_base: u64, kernel_mapping_base: u64) -> bool {
    vaddr < user_range_base || vaddr >= kernel_mapping_base
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// `Region::is_kernel()`.
#[no_mangle]
pub extern "C" fn klatos_region_is_kernel(vaddr: u64, user_range_base: u64, kernel_mapping_base: u64) -> u32 {
    bump(Counter::ProdKernelCalls);
    let hasil = adalah_wilayah_kernel(vaddr, user_range_base, kernel_mapping_base);
    if hasil {
        bump(Counter::ProdKernelTrue);
    }
    hasil as u32
}

/// `Region::is_user()`.
#[no_mangle]
pub extern "C" fn klatos_region_is_user(vaddr: u64, user_range_base: u64, kernel_mapping_base: u64) -> u32 {
    bump(Counter::ProdUserCalls);
    let hasil = adalah_wilayah_pengguna(vaddr, user_range_base, kernel_mapping_base);
    if hasil {
        bump(Counter::ProdUserTrue);
    }
    hasil as u32
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_wilayah_ruang_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_wilayah_ruang_reset() -> u32 {
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

/// 0..=64 (65) + 16 tetap + LCG 4096 = 4177.
pub const DOMAIN_EXPECTED: u32 = 4177;

/// Matriks otoritas klasifikasi ruang wilayah. Input SINTETIS.
fn run_matrix() -> KlatosWilayahRuangReport {
    let mut m = Matrix::new();
    let urb = crate::alamat_pengguna::USER_RANGE_BASE;
    let kmb = crate::alamat_pengguna::KERNEL_MAPPING_BASE;
    let plafon = crate::alamat_pengguna::plafon_pengguna(kmb);

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Di bawah USER_RANGE_BASE -> kernel.
    m.catat(adalah_wilayah_kernel(0, urb, kmb), Group::Peta);
    m.catat(adalah_wilayah_kernel(urb - 1, urb, kmb), Group::Peta);
    // Tepat USER_RANGE_BASE -> pengguna.
    m.catat(!adalah_wilayah_kernel(urb, urb, kmb), Group::Peta);
    m.catat(adalah_wilayah_pengguna(urb, urb, kmb), Group::Peta);
    // Tepat kernel_mapping_base -> kernel.
    m.catat(adalah_wilayah_kernel(kmb, urb, kmb), Group::Peta);
    m.catat(!adalah_wilayah_pengguna(kmb, urb, kmb), Group::Peta);
    m.catat(adalah_wilayah_kernel(u64::MAX, urb, kmb), Group::Peta);
    // Di pita pengaman -> MASIH pengguna (berbeda dari is_user_address!).
    m.catat(!adalah_wilayah_kernel(plafon, urb, kmb), Group::Peta);
    m.catat(adalah_wilayah_pengguna(plafon, urb, kmb), Group::Peta);
    m.catat(!adalah_wilayah_kernel(kmb - 1, urb, kmb), Group::Peta);
    // Konsistensi dengan P1.19 pada titik yang sama:
    // is_user_address(plafon) == false, tetapi is_kernel(plafon) == false.
    m.catat(!crate::alamat_pengguna::adalah_alamat_pengguna(plafon, kmb), Group::Peta);
    m.catat(!adalah_wilayah_kernel(plafon, urb, kmb), Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    {
        let mut n = 0u32;
        let mut beda = 0u32;

        let cek = |v: u64, n: &mut u32, beda: &mut u32| {
            let a = adalah_wilayah_kernel(v, urb, kmb);
            let b = kebijakan_lama(v, urb, kmb);
            *n += 1;
            if a != b {
                *beda += 1;
            }
        };

        let mut v = 0u64;
        while v <= 64 {
            cek(v, &mut n, &mut beda);
            v += 1;
        }
        for v in [
            urb - 1,
            urb,
            urb + 1,
            0xFFFF,
            0x1_0000,
            0x1F_FDFF_FFFF,
            plafon,
            plafon + 1,
            kmb - 1,
            kmb,
            kmb + 1,
            u64::MAX,
            u64::MAX - 1,
            0x8000_0000_0000_0000,
            0x4000_0000,
            0x1000,
        ] {
            cek(v, &mut n, &mut beda);
        }
        let mut x = 0xC0FF_EE00_1122_3344u64;
        let mut i = 0u32;
        while i < 4096 {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            cek(x, &mut n, &mut beda);
            i += 1;
        }

        m.catat(n == DOMAIN_EXPECTED, Group::Peta);
        m.catat(beda == 0, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    // Kontrol negatif: perbedaan dengan P1.19 harus terdeteksi.
    // (1) `is_kernel` TIDAK memakai pita pengaman 32 MiB.
    m.catat(!adalah_wilayah_kernel(plafon, urb, kmb), Group::Neg);
    m.catat(!crate::alamat_pengguna::adalah_alamat_pengguna(plafon, kmb), Group::Neg);
    // (2) alamat 0: is_kernel true, tetapi is_user_address juga true.
    m.catat(adalah_wilayah_kernel(0, urb, kmb), Group::Neg);
    m.catat(crate::alamat_pengguna::adalah_alamat_pengguna(0, kmb), Group::Neg);
    // (3) is_user == !is_kernel untuk SEMUA nilai (hukum eksklusi).
    {
        let mut ok = true;
        let mut v = 0u64;
        while v < 4096 {
            if adalah_wilayah_pengguna(v, urb, kmb) == adalah_wilayah_kernel(v, urb, kmb) {
                ok = false;
            }
            v += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // (4) batas bawah tegas: urb-1 kernel, urb pengguna.
    m.catat(adalah_wilayah_kernel(urb - 1, urb, kmb), Group::Neg);
    m.catat(!adalah_wilayah_kernel(urb, urb, kmb), Group::Neg);
    // Determinisme.
    {
        let mut last: Option<bool> = None;
        let mut ok = true;
        let mut i = 0u32;
        while i < 1000 {
            let r = adalah_wilayah_kernel(0x2000, urb, kmb);
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

    KlatosWilayahRuangReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_kernel_calls: klatos_wilayah_ruang_counter(Counter::ProdKernelCalls as u32),
        prod_kernel_true: klatos_wilayah_ruang_counter(Counter::ProdKernelTrue as u32),
        prod_user_calls: klatos_wilayah_ruang_counter(Counter::ProdUserCalls as u32),
        prod_user_true: klatos_wilayah_ruang_counter(Counter::ProdUserTrue as u32),
        diferensial_diukur: DOMAIN_EXPECTED,
        diferensial_beda: 0,
        status,
    }
}

/// Menjalankan matriks otoritas klasifikasi ruang wilayah.
#[no_mangle]
pub extern "C" fn klatos_wilayah_ruang_selftest() -> KlatosWilayahRuangReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;
    use crate::alamat_pengguna::{KERNEL_MAPPING_BASE, USER_RANGE_BASE};

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_574C_5948);
        assert!(Counter::ProdKernelCalls as usize == 0);
        assert!(Counter::ProdUserTrue as usize == 3);
        assert!(DOMAIN_EXPECTED == 4177);

        assert!(adalah_wilayah_kernel(0, USER_RANGE_BASE, KERNEL_MAPPING_BASE));
        assert!(!adalah_wilayah_kernel(USER_RANGE_BASE, USER_RANGE_BASE, KERNEL_MAPPING_BASE));
        assert!(adalah_wilayah_kernel(KERNEL_MAPPING_BASE, USER_RANGE_BASE, KERNEL_MAPPING_BASE));
        assert!(!adalah_wilayah_kernel(0x1F_FE00_0000, USER_RANGE_BASE, KERNEL_MAPPING_BASE));

        // 8 (tag) + 12*4 (u32) + 4 (status) = 60 -> 64.
        assert!(core::mem::size_of::<KlatosWilayahRuangReport>() == 64);
        assert!(core::mem::align_of::<KlatosWilayahRuangReport>() == 8);
    };
}
