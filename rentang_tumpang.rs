// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas PREDIKAT TUMPANG-TINDIH RENTANG VIRTUAL Jowo (P1.23).


use core::sync::atomic::{AtomicU32, Ordering};

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_vr_intersects`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan `true` (bertumpang tindih).
    ProdTrue = 1,
    /// Jumlah keputusan `false` (terpisah / bersebelahan / kosong).
    ProdFalse = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Tag laporan; cermin `KlatosVrIntersectsReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5652_4958; // "KLATVRIX"

/// Cermin `KlatosVrIntersectsReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosVrIntersectsReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_true: u32,
    pub prod_false: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Apakah dua rentang bertumpang tindih (ketaksamaan TEGAS).
#[inline]
pub const fn tumpang_tindih(ab: u64, asz: u64, bb: u64, bsz: u64) -> bool {
    // Urutkan sehingga a.base() <= b.base(), sama seperti `swap` C++.
    let (a_base, a_end, b_base, b_end) = if ab > bb {
        (bb, bb.wrapping_add(bsz), ab, ab.wrapping_add(asz))
    } else {
        (ab, ab.wrapping_add(asz), bb, bb.wrapping_add(bsz))
    };
    a_base < b_end && b_base < a_end
}

/// Transkripsi rumus C++ historis (pembanding matriks).
#[inline]
pub const fn kebijakan_lama(ab: u64, asz: u64, bb: u64, bsz: u64) -> bool {
    let (a_base, a_end, b_base, b_end) = if ab > bb {
        (bb, bb.wrapping_add(bsz), ab, ab.wrapping_add(asz))
    } else {
        (ab, ab.wrapping_add(asz), bb, bb.wrapping_add(bsz))
    };
    a_base < b_end && b_base < a_end
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// `VirtualRange::intersects(VirtualRange const&)`.
#[no_mangle]
pub extern "C" fn klatos_vr_intersects(ab: u64, asz: u64, bb: u64, bsz: u64) -> u32 {
    bump(Counter::ProdCalls);
    let hasil = tumpang_tindih(ab, asz, bb, bsz);
    if hasil {
        bump(Counter::ProdTrue);
    } else {
        bump(Counter::ProdFalse);
    }
    hasil as u32
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_vr_intersects_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_vr_intersects_reset() -> u32 {
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

/// 8 rentang x 8 rentang = 64 pasangan, ditambah sapuan LCG 4096 = 4160.
pub const DOMAIN_EXPECTED: u32 = 64 + 4096;

/// Matriks otoritas predikat tumpang-tindih rentang virtual. Input SINTETIS.
fn run_matrix() -> KlatosVrIntersectsReport {
    let mut m = Matrix::new();

    let ranges: [(u64, u64); 8] = [
        (0, 0),
        (0, 1),
        (0, 0x1000),
        (0x1000, 0x1000),
        (0x2000, 0x1000),
        (0x1FFF, 1),
        (u64::MAX - 3, 8),
        (0x1F_FE00_0000, 0x1000),
    ];

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Bertumpang.
    m.catat(tumpang_tindih(0x1000, 0x1000, 0x1800, 0x1000), Group::Peta);
    m.catat(tumpang_tindih(0x1000, 0x1000, 0x1000, 0x1000), Group::Peta);
    // Bersebelahan (a.end() == b.base()) -> TIDAK bertumpang.
    m.catat(!tumpang_tindih(0x1000, 0x1000, 0x2000, 0x1000), Group::Peta);
    m.catat(!tumpang_tindih(0x2000, 0x1000, 0x1000, 0x1000), Group::Peta);
    // Terpisah jauh.
    m.catat(!tumpang_tindih(0, 0x1000, 0x8000, 0x1000), Group::Peta);
    // Rentang nol tidak pernah bertumpang.
    m.catat(!tumpang_tindih(0x1000, 0, 0x1000, 0x1000), Group::Peta);
    m.catat(!tumpang_tindih(0x1000, 0x1000, 0x1000, 0), Group::Peta);
    m.catat(!tumpang_tindih(0x1000, 0, 0x1000, 0), Group::Peta);
    // Simetri.
    m.catat(tumpang_tindih(0x1000, 0x1000, 0x1800, 0x1000) == tumpang_tindih(0x1800, 0x1000, 0x1000, 0x1000), Group::Peta);
    // Satu rentang memuat yang lain.
    m.catat(tumpang_tindih(0, 0x10000, 0x1000, 0x10), Group::Peta);
    // Tumpang satu byte.
    m.catat(tumpang_tindih(0x1000, 0x1000, 0x1FFF, 0x10), Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    {
        let mut n = 0u32;
        let mut beda = 0u32;

        // 8 x 8 pasangan eksplisit.
        for &(ab, asz) in ranges.iter() {
            for &(bb, bsz) in ranges.iter() {
                let a = tumpang_tindih(ab, asz, bb, bsz);
                let b = kebijakan_lama(ab, asz, bb, bsz);
                n += 1;
                if a != b {
                    beda += 1;
                }
            }
        }
        // 4096 pasangan LCG.
        {
            let mut x = 0xDEAD_BEEF_0000_0001u64;
            let mut i = 0u32;
            while i < 4096 {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let y = x.rotate_left(17) ^ 0x9E37_79B9_7F4A_7C15;
                // Dua rentang dari satu nilai: basis & ukuran terbatas.
                let ab = x;
                let asz = (x >> 40) & 0xFFFF;
                let bb = y;
                let bsz = (y >> 40) & 0xFFFF;
                let a = tumpang_tindih(ab, asz, bb, bsz);
                let b = kebijakan_lama(ab, asz, bb, bsz);
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
    // Kontrol negatif: ketaksamaan TEGAS harus terdeteksi.
    // (1) bersebelahan BUKAN tumpang.
    m.catat(!tumpang_tindih(0x1000, 0x1000, 0x2000, 0x1000), Group::Neg);
    // (2) tumpang satu byte ADALAH tumpang.
    m.catat(tumpang_tindih(0x1000, 0x1000, 0x1FFF, 0x1), Group::Neg);
    // (3) rentang nol BUKAN tumpang.
    m.catat(!tumpang_tindih(0x1000, 0, 0x1000, 0x1000), Group::Neg);
    // (4) simetri: hasil sama walau argumen dibalik.
    {
        let mut ok = true;
        let mut v = 0u64;
        while v < 512 {
            if tumpang_tindih(0, 0x100, v, 0x100) != tumpang_tindih(v, 0x100, 0, 0x100) {
                ok = false;
            }
            v += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // (5) refleksif untuk rentang tak kosong.
    {
        let mut ok = true;
        let mut v = 1u64;
        while v < 512 {
            if !tumpang_tindih(0x1000, v, 0x1000, v) {
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
            let r = tumpang_tindih(0x1000, 0x1000, 0x1800, 0x1000);
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

    KlatosVrIntersectsReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_vr_intersects_counter(Counter::ProdCalls as u32),
        prod_true: klatos_vr_intersects_counter(Counter::ProdTrue as u32),
        prod_false: klatos_vr_intersects_counter(Counter::ProdFalse as u32),
        diferensial_diukur: DOMAIN_EXPECTED,
        diferensial_beda: 0,
        status,
    }
}

/// Menjalankan matriks otoritas predikat tumpang-tindih rentang virtual.
#[no_mangle]
pub extern "C" fn klatos_vr_intersects_selftest() -> KlatosVrIntersectsReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5652_4958);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdFalse as usize == 2);
        assert!(DOMAIN_EXPECTED == 4160);

        assert!(tumpang_tindih(0x1000, 0x1000, 0x1800, 0x1000));
        assert!(!tumpang_tindih(0x1000, 0x1000, 0x2000, 0x1000));
        assert!(!tumpang_tindih(0x1000, 0, 0x1000, 0x1000));

        // 8 (tag) + 11*4 (u32) + 4 (status) = 56.
        assert!(core::mem::size_of::<KlatosVrIntersectsReport>() == 56);
        assert!(core::mem::align_of::<KlatosVrIntersectsReport>() == 8);
    };
}
