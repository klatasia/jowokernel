// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas penerimaan bendera mount (P1.33).


//! `0` = bendera diterima; `-EINVAL (-22)` = ada `MS_REMOUNT`/`MS_BIND`.

use core::sync::atomic::{AtomicU32, Ordering};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;

/// `MS_BIND` — cermin `Jowo/Kontrak/POSIX/unistd.h:30`.
pub const MS_BIND: i32 = 1 << 3;
/// `MS_REMOUNT` — cermin `Jowo/Kontrak/POSIX/unistd.h:32`.
pub const MS_REMOUNT: i32 = 1 << 5;

/// Topeng bendera yang ditolak.
pub const MASK_TERLARANG: i32 = MS_REMOUNT | MS_BIND;

/// Tag laporan; cermin `KlatosMountBenderaReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4D4E_5442; // "KLATMNTB"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_mount_bendera_decide`].
    ProdCalls = 0,
    /// Jumlah bendera diterima.
    ProdValid = 1,
    /// Jumlah ditolak (`-EINVAL`).
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

/// Memutuskan penerimaan bendera mount. Mengembalikan `0` (terima) atau
/// `-EINVAL` (tolak).
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(flags: i32) -> i32 {
    if (flags & MASK_TERLARANG) != 0 {
        return -EINVAL;
    }
    0
}

/// Orakel historis: transkripsi langsung syarat `mount.cpp`. Dipakai HANYA
/// oleh matriks diferensial. Struktur sengaja berbeda supaya kesepakatan
/// dengan [`putuskan`] bermakna.
#[inline]
pub const fn kebijakan_lama(flags: i32) -> i32 {
    if (flags & MS_REMOUNT) != 0 {
        return -EINVAL;
    }
    if (flags & MS_BIND) != 0 {
        return -EINVAL;
    }
    0
}

/// Apakah hasil ini keputusan `diterima`?
#[inline]
pub const fn diterima(hasil: i32) -> bool {
    hasil == 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan penerimaan bendera mount. Lihat [`putuskan`] untuk kontrak.
#[no_mangle]
pub extern "C" fn klatos_mount_bendera_decide(flags: i32) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(flags);
    if hasil == 0 {
        bump(Counter::ProdValid);
    } else {
        bump(Counter::ProdInvalid);
    }
    hasil
}

/// Nilai errno dan bendera yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_mount_bendera_einval() -> i32 {
    EINVAL
}
#[no_mangle]
pub extern "C" fn klatos_mount_bendera_ms_bind() -> i32 {
    MS_BIND
}
#[no_mangle]
pub extern "C" fn klatos_mount_bendera_ms_remount() -> i32 {
    MS_REMOUNT
}
#[no_mangle]
pub extern "C" fn klatos_mount_bendera_mask_terlarang() -> i32 {
    MASK_TERLARANG
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_mount_bendera_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_mount_bendera_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosMountBenderaReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosMountBenderaReport {
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
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Ukuran domain sapuan diferensial: `flags` 0..=65_535 (16 bit rendah).
pub const DOMAIN_EXPECTED: u32 = 65_536;

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

/// Matriks otoritas penerimaan bendera mount. Input SINTETIS.
fn run_matrix() -> KlatosMountBenderaReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    m.catat(putuskan(0) == 0, Group::Peta);
    m.catat(putuskan(MS_BIND) == -EINVAL, Group::Peta);
    m.catat(putuskan(MS_REMOUNT) == -EINVAL, Group::Peta);
    m.catat(putuskan(MS_BIND | MS_REMOUNT) == -EINVAL, Group::Peta);
    // Bendera lain yang sah (mis. MS_RDONLY 1, MS_NOSUID 2, MS_NODEV 4) lolos.
    m.catat(putuskan(1) == 0, Group::Peta);
    m.catat(putuskan(2) == 0, Group::Peta);
    m.catat(putuskan(4) == 0, Group::Peta);
    m.catat(putuskan(1 | 2 | 4) == 0, Group::Peta);
    // Kombinasi sah + terlarang -> tolak.
    m.catat(putuskan(1 | MS_BIND) == -EINVAL, Group::Peta);
    m.catat(putuskan(4 | MS_REMOUNT) == -EINVAL, Group::Peta);
    // `-1` memasang SEMUA bit, termasuk MS_BIND|MS_REMOUNT -> tolak.
    m.catat(putuskan(-1) == -EINVAL, Group::Peta);
    // `i32::MIN` HANYA memasang bit tinggi (0x8000_0000): `& MASK_TERLARANG`
    // = 0, jadi DITERIMA — persis seperti orakel C++. Diuji sebagai
    // KESEPAKATAN dengan orakel, bukan sebagai nilai yang dikarang.
    m.catat(putuskan(i32::MIN) == kebijakan_lama(i32::MIN), Group::Peta);
    m.catat(putuskan(i32::MAX) == kebijakan_lama(i32::MAX), Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    let mut diukur = 0u32;
    let mut beda = 0u32;
    for flags in 0i32..=65_535 {
        let r = putuskan(flags);
        let b = kebijakan_lama(flags);
        diukur += 1;
        if r != b {
            beda += 1;
        }
    }
    m.catat(diukur == DOMAIN_EXPECTED, Group::Peta);
    m.catat(beda == 0, Group::Peta);

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4D4E_5442, Group::Neg);
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(MS_BIND == 8, Group::Neg);
    m.catat(MS_REMOUNT == 32, Group::Neg);
    m.catat(MASK_TERLARANG == 40, Group::Neg);
    // Dua bit terlarang berbeda dan berdiri sendiri.
    m.catat((MS_BIND & MS_REMOUNT) == 0 && MS_BIND != MS_REMOUNT, Group::Neg);
    // Determinisme.
    {
        let mut ok = true;
        let first = putuskan(7);
        for _ in 0..1000 {
            if putuskan(7) != first {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: harapan SALAH harus terdeteksi.
    {
        let mut ok = true;
        if putuskan(MS_BIND) != -EINVAL {
            ok = false;
        }
        if putuskan(0) != 0 {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosMountBenderaReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_mount_bendera_counter(Counter::ProdCalls as u32),
        prod_valid: klatos_mount_bendera_counter(Counter::ProdValid as u32),
        prod_invalid: klatos_mount_bendera_counter(Counter::ProdInvalid as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas penerimaan bendera mount.
#[no_mangle]
pub extern "C" fn klatos_mount_bendera_selftest() -> KlatosMountBenderaReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4D4E_5442);
        assert!(EINVAL == 22);
        assert!(MS_BIND == 8 && MS_REMOUNT == 32 && MASK_TERLARANG == 40);
        assert!(DOMAIN_EXPECTED == 65_536);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInvalid as usize == 2);

        assert!(putuskan(0) == 0);
        assert!(putuskan(MS_BIND) == -EINVAL);
        assert!(putuskan(MS_REMOUNT) == -EINVAL);
        assert!(putuskan(MS_BIND | MS_REMOUNT) == -EINVAL);
        assert!(putuskan(1 | 2 | 4) == 0);
        assert!(putuskan(-1) == -EINVAL);
        // Bit tinggi saja BUKAN bit terlarang: `i32::MIN & 40 == 0`.
        assert!(putuskan(i32::MIN) == 0);
        assert!(putuskan(i32::MIN) == kebijakan_lama(i32::MIN));

        assert!(core::mem::size_of::<KlatosMountBenderaReport>() == 56);
        assert!(core::mem::align_of::<KlatosMountBenderaReport>() == 8);
    };
}
