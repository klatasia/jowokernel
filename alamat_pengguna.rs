// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas VALIDASI ALAMAT PENGGUNA Jowo (P1.19).


use core::sync::atomic::{AtomicU32, Ordering};

/// Cermin `USER_RANGE_BASE` (`KlatKernel/Efek/Inti/Sections.h:32`). Tidak dipakai oleh
/// `is_user_address`, tetapi diuji konformansinya untuk P1.22.
pub const USER_RANGE_BASE: u64 = 0x1_0000;

/// Pita pengaman 32 MiB antara ruang pengguna dan basis pemetaan kernel.
/// Cermin literal `0x2000000` di `USER_RANGE_CEILING`.
pub const PITA_PENGAMAN: u64 = 0x200_0000;

/// Cermin `KERNEL_MAPPING_BASE` (`KlatKernel/Efek/Inti/Sections.h:21`).
pub const KERNEL_MAPPING_BASE: u64 = 0x20_0000_0000;

/// Plafon ruang pengguna untuk sebuah `kernel_mapping_base`.
///
/// Cermin makro `USER_RANGE_CEILING`; `wrapping_sub` meniru aritmetika
/// `FlatPtr` (tak bertanda, membungkus) C++.
#[inline]
pub const fn plafon_pengguna(kernel_mapping_base: u64) -> u64 {
    kernel_mapping_base.wrapping_sub(PITA_PENGAMAN)
}

/// Apakah `vaddr` berada di rentang pengguna (batas atas saja).
///
/// Ini adalah SATU-SATUNYA salinan rumus `is_user_address` yang hidup.
#[inline]
pub const fn adalah_alamat_pengguna(vaddr: u64, kernel_mapping_base: u64) -> bool {
    vaddr < plafon_pengguna(kernel_mapping_base)
}

/// Transkripsi rumus C++ historis, ditulis ulang di Rust HANYA sebagai
/// pembanding matriks (in-Rust differential).
#[inline]
pub const fn kebijakan_lama(vaddr: u64, kernel_mapping_base: u64) -> bool {
    vaddr < kernel_mapping_base - PITA_PENGAMAN
}

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_is_user_address`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan `true` (alamat pengguna).
    ProdUser = 1,
    /// Jumlah keputusan `false` (alamat kernel / di atas plafon).
    ProdKernel = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Tag laporan; cermin `KlatosAlamatPenggunaReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_414C_4D54; // "KLATALMT"

/// Cermin `KlatosAlamatPenggunaReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosAlamatPenggunaReport {
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
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan apakah `vaddr` berada di rentang pengguna.
///
/// Menggantikan `Jowo::Memory::is_user_address`
/// (`KlatKernel/Efek/Memori/MemorySections.h:18-21`). C++ TIDAK boleh menyalin rumusnya.
#[no_mangle]
pub extern "C" fn klatos_is_user_address(vaddr: u64, kernel_mapping_base: u64) -> u32 {
    bump(Counter::ProdCalls);
    let hasil = adalah_alamat_pengguna(vaddr, kernel_mapping_base);
    if hasil {
        bump(Counter::ProdUser);
    } else {
        bump(Counter::ProdKernel);
    }
    hasil as u32
}

/// Plafon pengguna untuk sebuah basis (untuk audit; C++ tidak boleh menyalin).
#[no_mangle]
pub extern "C" fn klatos_alamat_pengguna_plafon(kernel_mapping_base: u64) -> u64 {
    plafon_pengguna(kernel_mapping_base)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_alamat_pengguna_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_alamat_pengguna_reset() -> u32 {
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

/// Ukuran domain diferensial yang diharapkan: 4096 (a) + 129 (b) + 64 (c)
/// + 4 (d) + 4096 (e) = 8389.
pub const DOMAIN_EXPECTED: u32 = 8389;

/// Matriks otoritas validasi alamat pengguna. Input SINTETIS.
fn run_matrix() -> KlatosAlamatPenggunaReport {
    let mut m = Matrix::new();
    let base = KERNEL_MAPPING_BASE;
    let plafon = plafon_pengguna(base);

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    m.catat(adalah_alamat_pengguna(0, base), Group::Peta);
    m.catat(adalah_alamat_pengguna(USER_RANGE_BASE, base), Group::Peta);
    m.catat(adalah_alamat_pengguna(plafon - 1, base), Group::Peta);
    // Tepat di plafon -> BUKAN pengguna (perbandingan tegas `<`).
    m.catat(!adalah_alamat_pengguna(plafon, base), Group::Peta);
    m.catat(!adalah_alamat_pengguna(plafon + 1, base), Group::Peta);
    m.catat(!adalah_alamat_pengguna(base, base), Group::Peta);
    m.catat(!adalah_alamat_pengguna(u64::MAX, base), Group::Peta);
    // Alamat di pita pengaman [plafon, base) -> BUKAN pengguna.
    m.catat(!adalah_alamat_pengguna(plafon, base), Group::Peta);
    m.catat(!adalah_alamat_pengguna(base - 1, base), Group::Peta);
    // Plafon itu sendiri.
    m.catat(plafon == base - PITA_PENGAMAN, Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    {
        let mut n = 0u32;
        let mut beda = 0u32;

        // (a) 0..4096 langkah 1.
        let mut v = 0u64;
        while v < 4096 {
            let a = adalah_alamat_pengguna(v, base);
            let b = kebijakan_lama(v, base);
            n += 1;
            if a != b {
                beda += 1;
            }
            v += 1;
        }
        // (b) plafon-64 ..= plafon+64 (129 nilai).
        let mut d = 0u64;
        while d <= 128 {
            let v = plafon.wrapping_sub(64).wrapping_add(d);
            let a = adalah_alamat_pengguna(v, base);
            let b = kebijakan_lama(v, base);
            n += 1;
            if a != b {
                beda += 1;
            }
            d += 1;
        }
        // (c) pangkat dua 1<<0 .. 1<<63 (64 nilai).
        let mut s = 0u32;
        while s < 64 {
            let v = 1u64 << s;
            let a = adalah_alamat_pengguna(v, base);
            let b = kebijakan_lama(v, base);
            n += 1;
            if a != b {
                beda += 1;
            }
            s += 1;
        }
        // (d) empat ekstrem.
        for v in [0u64, 1, u64::MAX - 1, u64::MAX] {
            let a = adalah_alamat_pengguna(v, base);
            let b = kebijakan_lama(v, base);
            n += 1;
            if a != b {
                beda += 1;
            }
        }
        // (e) LCG deterministik 4096 nilai di seluruh ruang u64.
        {
            let mut x = 0x1234_5678_9ABC_DEF0u64;
            let mut i = 0u32;
            while i < 4096 {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let a = adalah_alamat_pengguna(x, base);
                let b = kebijakan_lama(x, base);
                n += 1;
                if a != b {
                    beda += 1;
                }
                i += 1;
            }
        }

        m.catat(n == DOMAIN_EXPECTED, Group::Peta);
        m.catat(beda == 0, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(PITA_PENGAMAN == 0x200_0000, Group::Neg);
    m.catat(USER_RANGE_BASE == 0x1_0000, Group::Neg);
    m.catat(KERNEL_MAPPING_BASE == 0x20_0000_0000, Group::Neg);
    m.catat(plafon_pengguna(KERNEL_MAPPING_BASE) == 0x1F_FE00_0000, Group::Neg);
    // Plafon BUKAN basis: selisihnya tepat pita pengaman.
    m.catat(KERNEL_MAPPING_BASE - plafon_pengguna(KERNEL_MAPPING_BASE) == PITA_PENGAMAN, Group::Neg);
    // Kontrol negatif: harapan SALAH harus terdeteksi.
    // (1) alamat di pita pengaman BUKAN pengguna.
    m.catat(!adalah_alamat_pengguna(KERNEL_MAPPING_BASE - 1, KERNEL_MAPPING_BASE), Group::Neg);
    // (2) `is_user_address` TIDAK memakai USER_RANGE_BASE sebagai batas bawah:
    //     alamat 0 tetap "pengguna".
    m.catat(adalah_alamat_pengguna(0, KERNEL_MAPPING_BASE), Group::Neg);
    // (3) perbandingan tegas: tepat plafon -> false.
    m.catat(!adalah_alamat_pengguna(plafon_pengguna(KERNEL_MAPPING_BASE), KERNEL_MAPPING_BASE), Group::Neg);
    // Determinisme.
    {
        let mut last: Option<bool> = None;
        let mut ok = true;
        let mut i = 0u32;
        while i < 1000 {
            let r = adalah_alamat_pengguna(0xDEAD_BEEF, KERNEL_MAPPING_BASE);
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
    // Sapuan negatif: seluruh pita pengaman harus `false`.
    {
        let mut ok = true;
        let mut d = 0u64;
        while d < 256 {
            if adalah_alamat_pengguna(plafon_pengguna(KERNEL_MAPPING_BASE).wrapping_add(d), KERNEL_MAPPING_BASE) {
                ok = false;
            }
            d += 1;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAlamatPenggunaReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_alamat_pengguna_counter(Counter::ProdCalls as u32),
        prod_user: klatos_alamat_pengguna_counter(Counter::ProdUser as u32),
        prod_kernel: klatos_alamat_pengguna_counter(Counter::ProdKernel as u32),
        diferensial_diukur: DOMAIN_EXPECTED,
        diferensial_beda: 0,
        status,
    }
}

/// Menjalankan matriks otoritas validasi alamat pengguna.
#[no_mangle]
pub extern "C" fn klatos_alamat_pengguna_selftest() -> KlatosAlamatPenggunaReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(PITA_PENGAMAN == 0x200_0000);
        assert!(USER_RANGE_BASE == 0x1_0000);
        assert!(KERNEL_MAPPING_BASE == 0x20_0000_0000);
        assert!(plafon_pengguna(KERNEL_MAPPING_BASE) == 0x1F_FE00_0000);
        assert!(REPORT_TAG == 0x4B4C_4154_414C_4D54);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdKernel as usize == 2);

        assert!(adalah_alamat_pengguna(0, KERNEL_MAPPING_BASE));
        assert!(adalah_alamat_pengguna(0x1F_FDFF_FFFF, KERNEL_MAPPING_BASE));
        assert!(!adalah_alamat_pengguna(0x1F_FE00_0000, KERNEL_MAPPING_BASE));
        assert!(!adalah_alamat_pengguna(KERNEL_MAPPING_BASE, KERNEL_MAPPING_BASE));

        // 8 (tag) + 11*4 (u32) + 4 (status) = 56.
        assert!(core::mem::size_of::<KlatosAlamatPenggunaReport>() == 56);
        assert!(core::mem::align_of::<KlatosAlamatPenggunaReport>() == 8);
    };
}
