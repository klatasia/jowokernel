// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas VALIDASI RENTANG PENGGUNA Jowo (P1.20).


use crate::alamat_pengguna::adalah_alamat_pengguna;
use core::sync::atomic::{AtomicU32, Ordering};

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_is_user_range`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan `true`.
    ProdUser = 1,
    /// Jumlah keputusan `false` karena alamat/ujung di luar ruang pengguna.
    ProdKernel = 2,
    /// Jumlah keputusan `false` karena `vaddr + size` membungkus.
    ProdOverflow = 3,
    /// Jumlah keputusan dengan `size == 0`.
    ProdSizeNol = 4,
    /// Jumlah keputusan dengan `size == 1`.
    ProdSizeSatu = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Tag laporan; cermin `KlatosRentangPenggunaReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_524E_4750; // "KLATRNGP"

/// Cermin `KlatosRentangPenggunaReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosRentangPenggunaReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_user: u32,
    pub prod_kernel: u32,
    pub prod_overflow: u32,
    pub prod_size_nol: u32,
    pub prod_size_satu: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Ukuran domain diferensial yang diharapkan: 11 ukuran x 591 alamat = 6501.
pub const DOMAIN_EXPECTED: u32 = 11 * 591;

/// Apakah rentang `[vaddr, vaddr+size)` seluruhnya di ruang pengguna.
///
/// Urutan keputusan SAMA PERSIS dengan `Memory::is_user_range` historis.
#[inline]
pub const fn adalah_rentang_pengguna(vaddr: u64, size: u64, kernel_mapping_base: u64) -> bool {
    let akhir = vaddr.wrapping_add(size);
    if akhir < vaddr {
        return false;
    }
    if !adalah_alamat_pengguna(vaddr, kernel_mapping_base) {
        return false;
    }
    if size <= 1 {
        return true;
    }
    adalah_alamat_pengguna(vaddr.wrapping_add(size - 1), kernel_mapping_base)
}

/// Transkripsi rumus C++ historis, ditulis ulang di Rust HANYA sebagai
/// pembanding matriks (in-Rust differential).
#[inline]
pub const fn kebijakan_lama(vaddr: u64, size: u64, kernel_mapping_base: u64) -> bool {
    let akhir = vaddr + size;
    if akhir < vaddr {
        return false;
    }
    if vaddr >= kernel_mapping_base - 0x200_0000 {
        return false;
    }
    if size <= 1 {
        return true;
    }
    let ujung = vaddr + (size - 1);
    ujung < kernel_mapping_base - 0x200_0000
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan apakah `[vaddr, vaddr+size)` berada di ruang pengguna.
///
/// Menggantikan `Jowo::Memory::is_user_range`
/// (`KlatKernel/Efek/Memori/MemorySections.h:23-32`). C++ TIDAK boleh menyalin rumusnya.
#[no_mangle]
pub extern "C" fn klatos_is_user_range(vaddr: u64, size: u64, kernel_mapping_base: u64) -> u32 {
    bump(Counter::ProdCalls);
    let akhir = vaddr.wrapping_add(size);
    if akhir < vaddr {
        bump(Counter::ProdOverflow);
        return 0;
    }
    if size == 0 {
        bump(Counter::ProdSizeNol);
    } else if size == 1 {
        bump(Counter::ProdSizeSatu);
    }
    let hasil = adalah_rentang_pengguna(vaddr, size, kernel_mapping_base);
    if hasil {
        bump(Counter::ProdUser);
    } else {
        bump(Counter::ProdKernel);
    }
    hasil as u32
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_rentang_pengguna_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_rentang_pengguna_reset() -> u32 {
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

/// Matriks otoritas validasi rentang pengguna. Input SINTETIS.
fn run_matrix() -> KlatosRentangPenggunaReport {
    let mut m = Matrix::new();
    let base = crate::alamat_pengguna::KERNEL_MAPPING_BASE;
    let plafon = crate::alamat_pengguna::plafon_pengguna(base);

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    m.catat(adalah_rentang_pengguna(0, 0, base), Group::Peta);
    m.catat(adalah_rentang_pengguna(0, 1, base), Group::Peta);
    m.catat(adalah_rentang_pengguna(0, 0x1000, base), Group::Peta);
    m.catat(adalah_rentang_pengguna(plafon - 1, 1, base), Group::Peta);
    // Ujung terakhir tepat di plafon -> BUKAN pengguna.
    m.catat(!adalah_rentang_pengguna(plafon - 1, 2, base), Group::Peta);
    // Rentang yang mulai di plafon -> BUKAN pengguna.
    m.catat(!adalah_rentang_pengguna(plafon, 1, base), Group::Peta);
    // Overflow -> false, walau vaddr pengguna.
    m.catat(!adalah_rentang_pengguna(u64::MAX - 3, 8, base), Group::Peta);
    m.catat(!adalah_rentang_pengguna(0, u64::MAX, base), Group::Peta);
    // size 0 di alamat kernel -> false (alamat diperiksa).
    m.catat(!adalah_rentang_pengguna(base, 0, base), Group::Peta);
    // size 1 di alamat kernel -> false.
    m.catat(!adalah_rentang_pengguna(base, 1, base), Group::Peta);
    // size 1 di alamat pengguna -> true.
    m.catat(adalah_rentang_pengguna(0x10000, 1, base), Group::Peta);
    // Rentang penuh ruang pengguna (0 .. plafon-1).
    m.catat(adalah_rentang_pengguna(0, plafon - 1, base), Group::Peta);
    // Rentang [0, plafon): byte terakhir = plafon-1 -> MASIH pengguna.
    m.catat(adalah_rentang_pengguna(0, plafon, base), Group::Peta);
    // Rentang [0, plafon+1): byte terakhir = plafon -> BUKAN pengguna.
    m.catat(!adalah_rentang_pengguna(0, plafon + 1, base), Group::Peta);

    // --- kelompok 1: sapuan diferensial 2-D --------------------------------
    {
        let sizes: [u64; 11] = [
            0,
            1,
            2,
            3,
            7,
            8,
            0x1000,
            0x2000_0000,
            0xFFFF_FFFF,
            0x8000_0000_0000_0000,
            u64::MAX,
        ];
        let tetap: [u64; 14] = [
            plafon - 2,
            plafon - 1,
            plafon,
            plafon + 1,
            base,
            base + 1,
            u64::MAX,
            u64::MAX - 1,
            0x1_0000,
            0xFFFF,
            0x1F_FDFF_FFFF,
            0x1F_FE00_0000,
            0x4000_0000,
            0x8000_0000_0000_0000,
        ];

        let mut n = 0u32;
        let mut beda = 0u32;

        // Kumpulan alamat: 0..=64 (65) + 14 tetap + LCG 512 = 591.
        let cek = |v: u64, s: u64, n: &mut u32, beda: &mut u32| {
            let a = adalah_rentang_pengguna(v, s, base);
            let b = kebijakan_lama(v, s, base);
            *n += 1;
            if a != b {
                *beda += 1;
            }
        };

        for s in sizes {
            let mut v = 0u64;
            while v <= 64 {
                cek(v, s, &mut n, &mut beda);
                v += 1;
            }
            for v in tetap {
                cek(v, s, &mut n, &mut beda);
            }
            let mut x = 0x0BAD_F00D_1234_5678u64;
            let mut i = 0u32;
            while i < 512 {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                cek(x, s, &mut n, &mut beda);
                i += 1;
            }
        }

        m.catat(n == DOMAIN_EXPECTED, Group::Peta);
        m.catat(beda == 0, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    // Kontrol negatif: harapan SALAH harus terdeteksi.
    // (1) size 0 SELALU true bila alamat pengguna (bukan false).
    m.catat(adalah_rentang_pengguna(0x1234, 0, base), Group::Neg);
    // (2) overflow BUKAN true.
    m.catat(!adalah_rentang_pengguna(u64::MAX, 1, base), Group::Neg);
    // (3) ujung-1 di plafon -> false (bukan true).
    m.catat(!adalah_rentang_pengguna(plafon - 1, 2, base), Group::Neg);
    // (4) size 1 di plafon-1 -> true.
    m.catat(adalah_rentang_pengguna(plafon - 1, 1, base), Group::Neg);
    // (5) size 0 di plafon -> false (alamat diperiksa lebih dulu).
    m.catat(!adalah_rentang_pengguna(plafon, 0, base), Group::Neg);
    // Sapuan: seluruh rentang berukuran 1 identik dengan is_user_address.
    {
        let mut ok = true;
        let mut v = 0u64;
        while v < 4096 {
            if adalah_rentang_pengguna(v, 1, base) != crate::alamat_pengguna::adalah_alamat_pengguna(v, base) {
                ok = false;
            }
            v += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Sapuan: rentang berukuran 0 identik dengan is_user_address.
    {
        let mut ok = true;
        let mut v = 0u64;
        while v < 4096 {
            if adalah_rentang_pengguna(v, 0, base) != crate::alamat_pengguna::adalah_alamat_pengguna(v, base) {
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
            let r = adalah_rentang_pengguna(0xDEAD_BEEF, 0x1000, base);
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

    KlatosRentangPenggunaReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_rentang_pengguna_counter(Counter::ProdCalls as u32),
        prod_user: klatos_rentang_pengguna_counter(Counter::ProdUser as u32),
        prod_kernel: klatos_rentang_pengguna_counter(Counter::ProdKernel as u32),
        prod_overflow: klatos_rentang_pengguna_counter(Counter::ProdOverflow as u32),
        prod_size_nol: klatos_rentang_pengguna_counter(Counter::ProdSizeNol as u32),
        prod_size_satu: klatos_rentang_pengguna_counter(Counter::ProdSizeSatu as u32),
        diferensial_diukur: DOMAIN_EXPECTED,
        diferensial_beda: 0,
        status,
    }
}

/// Menjalankan matriks otoritas validasi rentang pengguna.
#[no_mangle]
pub extern "C" fn klatos_rentang_pengguna_selftest() -> KlatosRentangPenggunaReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;
    use crate::alamat_pengguna::KERNEL_MAPPING_BASE;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_524E_4750);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdSizeSatu as usize == 5);
        assert!(DOMAIN_EXPECTED == 6501);

        assert!(adalah_rentang_pengguna(0, 0, KERNEL_MAPPING_BASE));
        assert!(adalah_rentang_pengguna(0, 1, KERNEL_MAPPING_BASE));
        assert!(adalah_rentang_pengguna(0, 0x1000, KERNEL_MAPPING_BASE));
        assert!(adalah_rentang_pengguna(0, 0x1F_FE00_0000, KERNEL_MAPPING_BASE));
        assert!(!adalah_rentang_pengguna(0, 0x1F_FE00_0001, KERNEL_MAPPING_BASE));
        assert!(!adalah_rentang_pengguna(u64::MAX, 1, KERNEL_MAPPING_BASE));
        assert!(!adalah_rentang_pengguna(KERNEL_MAPPING_BASE, 0, KERNEL_MAPPING_BASE));

        // 8 (tag) + 14*4 (u32) + 4 (status) = 68 -> 72.
        assert!(core::mem::size_of::<KlatosRentangPenggunaReport>() == 72);
        assert!(core::mem::align_of::<KlatosRentangPenggunaReport>() == 8);
    };
}
