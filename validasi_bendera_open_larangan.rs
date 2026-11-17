// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas penolakan bendera terlarang `open()` Jowo (P1.28).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;

/// Cermin `O_NOFOLLOW_NOERROR` (`VirtualFileSystem.h`).
pub const O_NOFOLLOW_NOERROR: i32 = 1 << 29;

/// Cermin `O_UNLINK_INTERNAL` (`VirtualFileSystem.h`).
pub const O_UNLINK_INTERNAL: i32 = 1 << 30;

/// Gabungan kedua bendera terlarang.
pub const MASK_TERLARANG: i32 = O_NOFOLLOW_NOERROR | O_UNLINK_INTERNAL;

/// Tag laporan; cermin `KlatosOpenLaranganReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4F50_4E4C; // "KLATOPNL"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_validasi_bendera_open_larangan_putuskan`].
    ProdCalls = 0,
    /// Jumlah `options` sah.
    ProdValid = 1,
    /// Jumlah `options` ditolak karena `O_NOFOLLOW_NOERROR`.
    ProdInvalidNofollow = 2,
    /// Jumlah `options` ditolak karena `O_UNLINK_INTERNAL`.
    ProdInvalidUnlink = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan validitas `options` `open` terhadap bendera terlarang.
///
/// SAMA PERSIS dengan `open.cpp:30-34` historis. Urutan penolakan mengikuti
/// sumber: `O_NOFOLLOW_NOERROR` lebih dulu.
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(options: i32) -> i32 {
    if options & O_NOFOLLOW_NOERROR != 0 {
        return -EINVAL;
    }
    if options & O_UNLINK_INTERNAL != 0 {
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

/// Memutuskan validitas `options` `open` terhadap bendera terlarang.
/// Lihat [`putuskan`] untuk kontrak. Menaikkan `prod_calls` dan penghitung
/// yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_open_larangan_putuskan(options: i32) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(options);
    if hasil == 0 {
        bump(Counter::ProdValid);
    } else if options & O_NOFOLLOW_NOERROR != 0 {
        bump(Counter::ProdInvalidNofollow);
    } else if options & O_UNLINK_INTERNAL != 0 {
        bump(Counter::ProdInvalidUnlink);
    }
    hasil
}

/// Nilai errno dan topeng yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_open_larangan_einval() -> i32 {
    EINVAL
}
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_open_larangan_o_nofollow_noerror() -> i32 {
    O_NOFOLLOW_NOERROR
}
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_open_larangan_o_unlink_internal() -> i32 {
    O_UNLINK_INTERNAL
}
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_open_larangan_mask_terlarang() -> i32 {
    MASK_TERLARANG
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_open_larangan_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_open_larangan_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosOpenLaranganReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosOpenLaranganReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_valid: u32,
    pub prod_invalid_nofollow: u32,
    pub prod_invalid_unlink: u32,
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

/// Matriks otoritas penolakan bendera terlarang `open`. Input SINTETIS.
fn run_matrix() -> KlatosOpenLaranganReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // 0 (tidak ada bendera) -> sah.
    m.catat(putuskan(0) == 0, Group::Peta);
    // Bit rendah umum (O_RDONLY=0, O_WRONLY=1, O_RDWR=2, O_CREAT=0x40,
    // O_EXCL=0x80, O_TRUNC=0x200, O_APPEND=0x400, O_NONBLOCK=0x800,
    // O_DIRECTORY=0x10000, O_NOFOLLOW=0x20000, O_CLOEXEC=0x80000) -> sah.
    m.catat(putuskan(0o0) == 0, Group::Peta);
    m.catat(putuskan(0o1) == 0, Group::Peta);
    m.catat(putuskan(0o2) == 0, Group::Peta);
    m.catat(putuskan(0x40) == 0, Group::Peta);
    m.catat(putuskan(0x80) == 0, Group::Peta);
    m.catat(putuskan(0x200) == 0, Group::Peta);
    m.catat(putuskan(0x10000) == 0, Group::Peta);
    m.catat(putuskan(0x20000) == 0, Group::Peta);
    m.catat(putuskan(0x80000) == 0, Group::Peta);
    // Bit-bit di luar MASK_TERLARANG -> sah.
    m.catat(putuskan(1 << 28) == 0, Group::Peta);
    // Bit tanda (1u32 << 31 == 0x80000000 == i32::MIN) bukan bit 29 atau 30,
    // jadi tetap sah. Pakai `as i32` agar tipe eksplisit cocok dengan `putuskan`.
    m.catat(putuskan((1u32 << 31) as i32) == 0, Group::Peta);
    // Bendera terlarang -> ditolak.
    m.catat(putuskan(O_NOFOLLOW_NOERROR) == -EINVAL, Group::Peta);
    m.catat(putuskan(O_UNLINK_INTERNAL) == -EINVAL, Group::Peta);
    m.catat(putuskan(MASK_TERLARANG) == -EINVAL, Group::Peta);
    // Kombinasi sah + terlarang -> ditolak.
    m.catat(putuskan(0o2 | O_NOFOLLOW_NOERROR) == -EINVAL, Group::Peta);
    m.catat(putuskan(0x40 | O_UNLINK_INTERNAL) == -EINVAL, Group::Peta);
    m.catat(putuskan(0o1 | MASK_TERLARANG) == -EINVAL, Group::Peta);
    // Urutan penolakan: NOFOLLOW lebih dulu.
    {
        let r = putuskan(MASK_TERLARANG);
        // Urutan sumber menentukan `prod_invalid_nofollow` yang naik.
        m.catat(r == -EINVAL, Group::Peta);
    }

    // --- kelompok 1: sapuan ekshaustif --------------------------------------
    // 32 bit -> terlalu besar. Sapuan pada 16 bit rendah (semua bit
    // 0..15 + dua bit terlarang pada 29, 30): fokus pada bit-bit yang
    // relevan.
    {
        let bit_terlarang = [(1u32 << 29), (1u32 << 30)];
        let bit_lain = [1u32 << 0, 1u32 << 1, 1u32 << 2, 1u32 << 6, 1u32 << 7,
                        1u32 << 9, 1u32 << 10, 1u32 << 11, 1u32 << 12,
                        1u32 << 13, 1u32 << 14, 1u32 << 15, 1u32 << 16,
                        1u32 << 17, 1u32 << 19, 1u32 << 23];
        // Setiap subset dari `bit_lain` digabung dengan setiap subset dari
        // `bit_terlarang` (termasuk kosong). Per submask_lain: 2 b_t × 2 opsi
        // (f, f|b_t) + 1 ekstra (f|MASK_TERLARANG) = 5 kasus. Total kasus =
        // 5 × 2^bit_lain.len() = 5 × 65536 = 327680.
        let mut n = 0u32;
        let mut total_sah = 0u32;
        let mut total_tolak = 0u32;
        let mut beda = 0u32;
        for submask_lain in 0u32..(1u32 << bit_lain.len() as u32) {
            let mut f = 0u32;
            for (i, b) in bit_lain.iter().enumerate() {
                if submask_lain & (1 << i) != 0 {
                    f |= *b;
                }
            }
            for &b_t in &bit_terlarang {
                // Pakai f saja, f | b_t, dan tanpa f (submask nol kasus).
                for options in [f, f | b_t] {
                    n += 1;
                    let r = putuskan(options as i32);
                    if r == 0 {
                        total_sah += 1;
                    } else if r == -EINVAL {
                        total_tolak += 1;
                    }
                    if r != putuskan(options as i32) {
                        beda += 1;
                    }
                }
            }
            // Juga kasus `f | kedua bit terlarang`.
            n += 1;
            let r = putuskan((f | MASK_TERLARANG as u32) as i32);
            if r == 0 {
                total_sah += 1;
            } else if r == -EINVAL {
                total_tolak += 1;
            }
            if r != putuskan((f | MASK_TERLARANG as u32) as i32) {
                beda += 1;
            }
        }
        let expected_n = 5 * (1u32 << bit_lain.len() as u32);
        m.catat(n == expected_n, Group::Peta);
        m.catat(beda == 0, Group::Peta);
        m.catat(total_tolak == 3 * (1u32 << bit_lain.len() as u32), Group::Peta);
        // total_sah = 2 per submask_lain (dua pemanggilan `f` di dalam loop b_t).
        m.catat(total_sah == 2 * (1u32 << bit_lain.len() as u32), Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(O_NOFOLLOW_NOERROR == 1 << 29, Group::Neg);
    m.catat(O_UNLINK_INTERNAL == 1 << 30, Group::Neg);
    m.catat(MASK_TERLARANG == (1 << 29) | (1 << 30), Group::Neg);
    m.catat(REPORT_TAG == 0x4B4C_4154_4F50_4E4C, Group::Neg);
    // Dua bit terlarang berdiri sendiri dan di atas bit 28.
    m.catat(O_NOFOLLOW_NOERROR != O_UNLINK_INTERNAL
         && (O_NOFOLLOW_NOERROR & O_UNLINK_INTERNAL) == 0
         && O_NOFOLLOW_NOERROR > (1 << 28)
         && O_UNLINK_INTERNAL > O_NOFOLLOW_NOERROR, Group::Neg);
    // Determinisme.
    {
        let mut last: Option<i32> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(MASK_TERLARANG);
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
        // 0 sah.
        if putuskan(0) != 0 {
            ok = false;
        }
        // O_NOFOLLOW_NOERROR ditolak.
        if putuskan(O_NOFOLLOW_NOERROR) != -EINVAL {
            ok = false;
        }
        // 1 << 28 bukan oksempadan digital.
        if putuskan(1 << 28) != 0 {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosOpenLaranganReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_validasi_bendera_open_larangan_counter(Counter::ProdCalls as u32),
        prod_valid: klatos_validasi_bendera_open_larangan_counter(Counter::ProdValid as u32),
        prod_invalid_nofollow: klatos_validasi_bendera_open_larangan_counter(Counter::ProdInvalidNofollow as u32),
        prod_invalid_unlink: klatos_validasi_bendera_open_larangan_counter(Counter::ProdInvalidUnlink as u32),
        status,
    }
}

/// Menjalankan matriks otoritas penolakan bendera terlarang `open`.
#[no_mangle]
pub extern "C" fn klatos_validasi_bendera_open_larangan_selftest() -> KlatosOpenLaranganReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EINVAL == 22);
        assert!(O_NOFOLLOW_NOERROR == 1 << 29);
        assert!(O_UNLINK_INTERNAL == 1 << 30);
        assert!(MASK_TERLARANG == (1 << 29) | (1 << 30));
        assert!(REPORT_TAG == 0x4B4C_4154_4F50_4E4C);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInvalidUnlink as usize == 3);

        assert!(putuskan(0) == 0);
        assert!(putuskan(0o2) == 0);
        assert!(putuskan(1 << 28) == 0);
        assert!(putuskan(O_NOFOLLOW_NOERROR) == -EINVAL);
        assert!(putuskan(O_UNLINK_INTERNAL) == -EINVAL);
        assert!(putuskan(MASK_TERLARANG) == -EINVAL);

        // 8 (tag) + 10*4 (u32) + 4 (status) = 52, padded ke kelipatan 8 = 56.
        assert!(core::mem::size_of::<KlatosOpenLaranganReport>() == 56);
        assert!(core::mem::align_of::<KlatosOpenLaranganReport>() == 8);
    };
}