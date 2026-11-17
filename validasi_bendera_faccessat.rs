// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas validasi bendera `faccessat` Jowo (P1.27).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;

/// Cermin `AT_SYMLINK_NOFOLLOW` (`fcntl.h`).
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
/// Cermin `AT_EACCESS` (`fcntl.h`).
pub const AT_EACCESS: i32 = 0x400;

/// Topeng sah: bit-bit yang boleh ada pada `flags`. Bit di luar topeng
/// akan ditolak.
pub const MASK_SAH: i32 = AT_SYMLINK_NOFOLLOW | AT_EACCESS;

/// Tag laporan; cermin `KlatosFaccessatFlagsReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4641_4343; // "KLATFACC"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_validasi_bendera_faccessat_putuskan`].
    ProdCalls = 0,
    /// Jumlah `flags` sah (di dalam topeng).
    ProdValid = 1,
    /// Jumlah `flags` ditolak (ada bit di luar topeng).
    ProdInvalid = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan validitas `flags` `faccessat`.
///
/// SAMA PERSIS dengan `faccessat.cpp:23` historis.
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(flags: i32) -> i32 {
    if (flags & !MASK_SAH) != 0 {
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

/// Memutuskan validitas `flags` `faccessat`. Lihat [`putuskan`] untuk kontrak.
/// Menaikkan `prod_calls` dan penghitung yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_faccessat_putuskan(flags: i32) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(flags);
    if hasil == 0 {
        bump(Counter::ProdValid);
    } else {
        bump(Counter::ProdInvalid);
    }
    hasil
}

/// Nilai errno dan topeng yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_faccessat_einval() -> i32 {
    EINVAL
}
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_faccessat_at_symlink_nofollow() -> i32 {
    AT_SYMLINK_NOFOLLOW
}
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_faccessat_at_eaccess() -> i32 {
    AT_EACCESS
}
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_faccessat_mask_sah() -> i32 {
    MASK_SAH
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_faccessat_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_faccessat_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosFaccessatFlagsReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosFaccessatFlagsReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_valid: u32,
    pub prod_invalid: u32,
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

/// Matriks otoritas validasi `flags` `faccessat`. Input SINTETIS.
fn run_matrix() -> KlatosFaccessatFlagsReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // 0 (tidak ada bendera) -> sah.
    m.catat(putuskan(0) == 0, Group::Peta);
    // Tepat satu bendera sah.
    m.catat(putuskan(AT_SYMLINK_NOFOLLOW) == 0, Group::Peta);
    m.catat(putuskan(AT_EACCESS) == 0, Group::Peta);
    // Keduanya -> sah.
    m.catat(putuskan(AT_SYMLINK_NOFOLLOW | AT_EACCESS) == 0, Group::Peta);
    // Bit di luar topeng ditolak.
    m.catat(putuskan(0x001) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x002) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x004) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x008) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x010) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x020) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x040) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x080) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x200) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x800) == -EINVAL, Group::Peta);
    // Bit TEPAT di kedua sisi topeng ditolak.
    m.catat(putuskan(0x0FF) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x800) == -EINVAL, Group::Peta);
    // Kombinasi sah + terganggu.
    m.catat(putuskan(AT_SYMLINK_NOFOLLOW | 0x001) == -EINVAL, Group::Peta);
    m.catat(putuskan(AT_EACCESS | 0x200) == -EINVAL, Group::Peta);
    m.catat(putuskan(AT_SYMLINK_NOFOLLOW | AT_EACCESS | 0x001) == -EINVAL, Group::Peta);
    // Negatif ditolak.
    m.catat(putuskan(-1) == -EINVAL, Group::Peta);
    m.catat(putuskan(i32::MIN) == -EINVAL, Group::Peta);
    m.catat(putuskan(i32::MAX) == -EINVAL, Group::Peta);

    // --- kelompok 1: sapuan ekshaustif --------------------------------------
    // 16 bit rendah -> 65536 kombinasi. Hanya topeng yang sah.
    {
        let mut n = 0u32;
        let mut total_sah = 0u32;
        let mut total_tolak = 0u32;
        let mut beda = 0u32;
        for flags in 0i32..=65535 {
            let r = putuskan(flags);
            n += 1;
            if r == 0 {
                total_sah += 1;
            } else if r == -EINVAL {
                total_tolak += 1;
            }
            // Deterministik.
            if r != putuskan(flags) {
                beda += 1;
            }
        }
        m.catat(n == 65536, Group::Peta);
        m.catat(beda == 0, Group::Peta);
        // Topeng sah = bit 8 (0x100) + bit 10 (0x400) = 3 kombinasi: 0, 0x100,
        // 0x400, 0x500.
        m.catat(total_sah == 4 && total_tolak == 65532, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(AT_SYMLINK_NOFOLLOW == 0x100, Group::Neg);
    m.catat(AT_EACCESS == 0x400, Group::Neg);
    m.catat(MASK_SAH == 0x500, Group::Neg);
    m.catat(REPORT_TAG == 0x4B4C_4154_4641_4343, Group::Neg);
    // Dua bit topeng berbeda dan berdiri sendiri.
    m.catat(AT_SYMLINK_NOFOLLOW != AT_EACCESS
         && (AT_SYMLINK_NOFOLLOW & AT_EACCESS) == 0, Group::Neg);
    // Topeng TIDAK mencakup seluruh bit 16-bit rendah.
    m.catat((!MASK_SAH & 0xFFFF) != 0, Group::Neg);
    // Determinisme.
    {
        let mut last: Option<i32> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(0x12345 & MASK_SAH);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: harapan SALAH harus terdeteksi.
    {
        let mut ok = true;
        // 0x100 adalah AT_SYMLINK_NOFOLLOW dan sah.
        if putuskan(0x100) != 0 {
            ok = false;
        }
        // 0x200 adalah bit di antara topeng dan ditolak.
        if putuskan(0x200) != -EINVAL {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosFaccessatFlagsReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_validasi_bendera_faccessat_counter(Counter::ProdCalls as u32),
        prod_valid: klatos_validasi_bendera_faccessat_counter(Counter::ProdValid as u32),
        prod_invalid: klatos_validasi_bendera_faccessat_counter(Counter::ProdInvalid as u32),
        status,
    }
}

/// Menjalankan matriks otoritas validasi `flags` `faccessat`.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_faccessat_selftest() -> KlatosFaccessatFlagsReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EINVAL == 22);
        assert!(AT_SYMLINK_NOFOLLOW == 0x100);
        assert!(AT_EACCESS == 0x400);
        assert!(MASK_SAH == 0x500);
        assert!(REPORT_TAG == 0x4B4C_4154_4641_4343);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInvalid as usize == 2);

        assert!(putuskan(0) == 0);
        assert!(putuskan(AT_SYMLINK_NOFOLLOW) == 0);
        assert!(putuskan(AT_EACCESS) == 0);
        assert!(putuskan(AT_SYMLINK_NOFOLLOW | AT_EACCESS) == 0);
        assert!(putuskan(0x001) == -EINVAL);
        assert!(putuskan(0x200) == -EINVAL);
        assert!(putuskan(-1) == -EINVAL);

        // 8 (tag) + 9*4 (u32) + 4 (status) = 48.
        assert!(core::mem::size_of::<KlatosFaccessatFlagsReport>() == 48);
        assert!(core::mem::align_of::<KlatosFaccessatFlagsReport>() == 8);
    };
}