// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas validasi kombinasi bendera `mmap` Jowo (P1.25).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;

/// Cermin `MAP_SHARED` (`sys/mman.h`).
pub const MAP_SHARED: u32 = 0x01;
/// Cermin `MAP_PRIVATE` (`sys/mman.h`).
pub const MAP_PRIVATE: u32 = 0x02;
/// Cermin `MAP_FIXED` (`sys/mman.h`).
pub const MAP_FIXED: u32 = 0x10;
/// Cermin `MAP_ANONYMOUS` (`sys/mman.h`).
pub const MAP_ANONYMOUS: u32 = 0x20;
/// Cermin `MAP_STACK` (`sys/mman.h`).
pub const MAP_STACK: u32 = 0x40;
/// Cermin `MAP_RANDOMIZED` (`sys/mman.h`).
pub const MAP_RANDOMIZED: u32 = 0x100;
/// Cermin `MAP_FIXED_NOREPLACE` (`sys/mman.h`).
pub const MAP_FIXED_NOREPLACE: u32 = 0x400;

/// Tag laporan; cermin `KlatosMmapFlagsReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4D4D_4150; // "KLATMMAP"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_kebijakan_bendera_mmap_putuskan`].
    ProdCalls = 0,
    /// Jumlah keputusan sah.
    ProdValid = 1,
    /// Jumlah penolakan shared-private (keduanya).
    ProdInvalidSharedPrivate = 2,
    /// Jumlah penolakan shared-private (tidak keduanya).
    ProdInvalidNoType = 3,
    /// Jumlah penolakan fixed + randomized.
    ProdInvalidFixedRandom = 4,
    /// Jumlah penolakan stack tanpa private/anon.
    ProdInvalidStack = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan validitas kombinasi bendera `mmap`.
///
/// Urutan dan isi keputusan SAMA PERSIS dengan `mmap.cpp:163-175` historis
/// (kecuali baris `validate_mmap_prot` yang menjadi wewenang P1.09).
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(flags: u32) -> i32 {
    let map_shared = (flags & MAP_SHARED) != 0;
    let map_private = (flags & MAP_PRIVATE) != 0;
    let map_anonymous = (flags & MAP_ANONYMOUS) != 0;
    let map_stack = (flags & MAP_STACK) != 0;
    let map_fixed = (flags & MAP_FIXED) != 0;
    let map_randomized = (flags & MAP_RANDOMIZED) != 0;
    let map_fixed_noreplace = (flags & MAP_FIXED_NOREPLACE) != 0;

    if map_shared && map_private {
        return -EINVAL;
    }
    if !map_shared && !map_private {
        return -EINVAL;
    }
    if (map_fixed || map_fixed_noreplace) && map_randomized {
        return -EINVAL;
    }
    if map_stack && (!map_private || !map_anonymous) {
        return -EINVAL;
    }
    0
}

/// Apakah hasil ini keputusan sah?
#[inline]
pub const fn berhasil(hasil: i32) -> bool {
    hasil == 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan validitas kombinasi bendera `mmap`. Lihat [`putuskan`] untuk
/// kontrak. Menaikkan `prod_calls` dan penghitung penolakan yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_putuskan(flags: u32) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(flags);
    if hasil == 0 {
        bump(Counter::ProdValid);
        return hasil;
    }
    // Tentukan penyebab penolakan.
    let map_shared = (flags & MAP_SHARED) != 0;
    let map_private = (flags & MAP_PRIVATE) != 0;
    let map_anonymous = (flags & MAP_ANONYMOUS) != 0;
    let map_stack = (flags & MAP_STACK) != 0;
    let map_fixed = (flags & MAP_FIXED) != 0;
    let map_randomized = (flags & MAP_RANDOMIZED) != 0;
    let map_fixed_noreplace = (flags & MAP_FIXED_NOREPLACE) != 0;
    if map_shared && map_private {
        bump(Counter::ProdInvalidSharedPrivate);
    } else if !map_shared && !map_private {
        bump(Counter::ProdInvalidNoType);
    } else if (map_fixed || map_fixed_noreplace) && map_randomized {
        bump(Counter::ProdInvalidFixedRandom);
    } else if map_stack && (!map_private || !map_anonymous) {
        bump(Counter::ProdInvalidStack);
    }
    hasil
}

/// Nilai errno yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_einval() -> i32 {
    EINVAL
}

/// Nilai-nilai `MAP_*`.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_map_shared() -> u32 {
    MAP_SHARED
}
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_map_private() -> u32 {
    MAP_PRIVATE
}
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_map_fixed() -> u32 {
    MAP_FIXED
}
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_map_anonymous() -> u32 {
    MAP_ANONYMOUS
}
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_map_stack() -> u32 {
    MAP_STACK
}
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_map_randomized() -> u32 {
    MAP_RANDOMIZED
}
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_map_fixed_noreplace() -> u32 {
    MAP_FIXED_NOREPLACE
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosMmapFlagsReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosMmapFlagsReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_valid: u32,
    pub prod_invalid_shared_private: u32,
    pub prod_invalid_no_type: u32,
    pub prod_invalid_fixed_random: u32,
    pub prod_invalid_stack: u32,
    pub status: i32,
}

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

/// Matriks otoritas kombinasi bendera `mmap`. Input SINTETIS.
fn run_matrix() -> KlatosMmapFlagsReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // shared XOR private.
    m.catat(putuskan(MAP_SHARED) == 0, Group::Peta);
    m.catat(putuskan(MAP_PRIVATE) == 0, Group::Peta);
    m.catat(putuskan(MAP_SHARED | MAP_PRIVATE) == -EINVAL, Group::Peta);
    m.catat(putuskan(0) == -EINVAL, Group::Peta);
    // Fixed + randomized.
    m.catat(putuskan(MAP_PRIVATE | MAP_FIXED) == 0, Group::Peta);
    m.catat(putuskan(MAP_PRIVATE | MAP_FIXED_NOREPLACE) == 0, Group::Peta);
    m.catat(putuskan(MAP_PRIVATE | MAP_FIXED | MAP_RANDOMIZED) == -EINVAL, Group::Peta);
    m.catat(putuskan(MAP_PRIVATE | MAP_FIXED_NOREPLACE | MAP_RANDOMIZED) == -EINVAL, Group::Peta);
    m.catat(putuskan(MAP_PRIVATE | MAP_RANDOMIZED) == 0, Group::Peta);
    // Stack policy.
    m.catat(putuskan(MAP_PRIVATE | MAP_ANONYMOUS | MAP_STACK) == 0, Group::Peta);
    m.catat(putuskan(MAP_PRIVATE | MAP_STACK) == -EINVAL, Group::Peta);
    m.catat(putuskan(MAP_ANONYMOUS | MAP_STACK) == -EINVAL, Group::Peta);
    m.catat(putuskan(MAP_SHARED | MAP_ANONYMOUS | MAP_STACK) == -EINVAL, Group::Peta);

    // --- kelompok 1: sapuan ekshaustif --------------------------------------
    // 8 bit aktif (SHARED, PRIVATE, FIXED, ANONYMOUS, STACK, NORESERVE,
    // RANDOMIZED, FIXED_NOREPLACE) -> 256 kombinasi.
    {
        let bit: [u32; 8] = [
            MAP_SHARED, MAP_PRIVATE, MAP_FIXED, MAP_ANONYMOUS,
            MAP_STACK, 0x8000, MAP_RANDOMIZED, MAP_FIXED_NOREPLACE,
        ];
        let mut n = 0u32;
        let mut total_sah = 0u32;
        let mut total_tolak = 0u32;
        let mut beda = 0u32;
        for i in 0u32..256 {
            let flags = if i == 0 {
                0
            } else {
                let mut f = 0u32;
                for b in 0..8 {
                    if i & (1 << b) != 0 {
                        f |= bit[b];
                    }
                }
                f
            };
            let r = putuskan(flags);
            n += 1;
            if r == 0 {
                total_sah += 1;
            } else if r == -EINVAL {
                total_tolak += 1;
            }
            // Deterministik: dua pemanggilan hasilnya sama.
            if r != putuskan(flags) {
                beda += 1;
            }
        }
        m.catat(n == 256, Group::Peta);
        m.catat(total_sah + total_tolak == 256, Group::Peta);
        // Penolakan HARUS salah satu errno yang dikenal.
        m.catat(total_tolak > 0, Group::Peta);
        m.catat(beda == 0, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(MAP_SHARED == 0x01 && MAP_PRIVATE == 0x02, Group::Neg);
    m.catat(MAP_FIXED == 0x10 && MAP_ANONYMOUS == 0x20, Group::Neg);
    m.catat(MAP_STACK == 0x40 && MAP_RANDOMIZED == 0x100, Group::Neg);
    m.catat(MAP_FIXED_NOREPLACE == 0x400, Group::Neg);
    m.catat(REPORT_TAG == 0x4B4C_4154_4D4D_4150, Group::Neg);
    // Bendera yang dipakai berbeda satu sama lain.
    m.catat(MAP_SHARED != MAP_PRIVATE
         && MAP_PRIVATE != MAP_FIXED
         && MAP_FIXED != MAP_ANONYMOUS
         && MAP_ANONYMOUS != MAP_STACK
         && MAP_STACK != MAP_RANDOMIZED
         && MAP_RANDOMIZED != MAP_FIXED_NOREPLACE, Group::Neg);
    // Determinisme.
    {
        let mut last: Option<i32> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(MAP_SHARED | MAP_PRIVATE | MAP_FIXED | MAP_RANDOMIZED);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: setiap penolakan punya alasan yang terdeteksi.
    {
        let mut ok = true;
        // shared+private harus tolak, bukan sah.
        if putuskan(MAP_SHARED | MAP_PRIVATE) != -EINVAL {
            ok = false;
        }
        // fixed+randomized harus tolak.
        if putuskan(MAP_PRIVATE | MAP_FIXED | MAP_RANDOMIZED) != -EINVAL {
            ok = false;
        }
        // stack tanpa private harus tolak.
        if putuskan(MAP_ANONYMOUS | MAP_STACK) != -EINVAL {
            ok = false;
        }
        // sah default.
        if putuskan(MAP_PRIVATE | MAP_ANONYMOUS) != 0 {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosMmapFlagsReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_kebijakan_bendera_mmap_counter(Counter::ProdCalls as u32),
        prod_valid: klatos_kebijakan_bendera_mmap_counter(Counter::ProdValid as u32),
        prod_invalid_shared_private: klatos_kebijakan_bendera_mmap_counter(Counter::ProdInvalidSharedPrivate as u32),
        prod_invalid_no_type: klatos_kebijakan_bendera_mmap_counter(Counter::ProdInvalidNoType as u32),
        prod_invalid_fixed_random: klatos_kebijakan_bendera_mmap_counter(Counter::ProdInvalidFixedRandom as u32),
        prod_invalid_stack: klatos_kebijakan_bendera_mmap_counter(Counter::ProdInvalidStack as u32),
        status,
    }
}

/// Menjalankan matriks otoritas kombinasi bendera `mmap`.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_bendera_mmap_selftest() -> KlatosMmapFlagsReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EINVAL == 22);
        assert!(MAP_SHARED == 0x01);
        assert!(MAP_PRIVATE == 0x02);
        assert!(MAP_FIXED == 0x10);
        assert!(MAP_ANONYMOUS == 0x20);
        assert!(MAP_STACK == 0x40);
        assert!(MAP_RANDOMIZED == 0x100);
        assert!(MAP_FIXED_NOREPLACE == 0x400);
        assert!(REPORT_TAG == 0x4B4C_4154_4D4D_4150);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInvalidStack as usize == 5);

        assert!(putuskan(0) == -EINVAL);
        assert!(putuskan(MAP_SHARED | MAP_PRIVATE) == -EINVAL);
        assert!(putuskan(MAP_PRIVATE | MAP_FIXED | MAP_RANDOMIZED) == -EINVAL);
        assert!(putuskan(MAP_ANONYMOUS | MAP_STACK) == -EINVAL);
        assert!(putuskan(MAP_PRIVATE) == 0);
        assert!(putuskan(MAP_PRIVATE | MAP_FIXED) == 0);
        assert!(putuskan(MAP_PRIVATE | MAP_ANONYMOUS | MAP_STACK) == 0);

        // 8 (tag) + 12*4 (u32) + 4 (status) = 60, padded ke kelipatan 8 = 64.
        assert!(core::mem::size_of::<KlatosMmapFlagsReport>() == 64);
        assert!(core::mem::align_of::<KlatosMmapFlagsReport>() == 8);
    };
}