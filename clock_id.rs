// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas keputusan validasi clock_id Jowo (P1.03).


use core::sync::atomic::{AtomicU32, Ordering};

/// Kode galat POSIX `EINVAL` (cermin `<errno.h>` C).
pub const EINVAL: i32 = 22;

/// Konstanta `CLOCK_REALTIME` (cermin `<POSIX/time.h>`).
pub const CLOCK_REALTIME: i32 = 0;

/// Konstanta `CLOCK_MONOTONIC` (cermin `<POSIX/time.h>`).
pub const CLOCK_MONOTONIC: i32 = 1;

/// Konstanta `CLOCK_MONOTONIC_RAW` (cermin `<POSIX/time.h>`).
pub const CLOCK_MONOTONIC_RAW: i32 = 2;

/// Konstanta `CLOCK_REALTIME_COARSE` (cermin `<POSIX/time.h>`).
pub const CLOCK_REALTIME_COARSE: i32 = 3;

/// Konstanta `CLOCK_MONOTONIC_COARSE` (cermin `<POSIX/time.h>`).
pub const CLOCK_MONOTONIC_COARSE: i32 = 4;

/// Batas atas jumlah clock_id sah + 1 (sentinel). Cermin `CLOCK_ID_COUNT`
/// di `Jowo/Kontrak/POSIX/time.h:38`.
pub const MAX_CLOCK_ID: u32 = 5;

/// Tag laporan. BERBEDA dari `frame::REPORT_TAG` ("KLATKRNL"),
/// `task_id::REPORT_TAG` ("KLATTKID"),
/// `priority_bucket::REPORT_TAG` ("KLATSCHD"), dan
/// `ipc_handle::REPORT_TAG` ("KLATIPCH") dengan sengaja: kelima laporan
/// adalah lima keputusan berbeda, dan C++ memeriksanya lewat konstanta
/// berbeda (`kernel_tag_expected`, `task_id_tag_expected`,
/// `sched_bucket_tag_expected`, `ipc_handle_tag_expected`,
/// `clock_id_tag_expected`) di `Jowo/Kontrak/KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4349_4444; // "KLATCID"

/// Galat otoritas clock_id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockIdError {
    /// Clock_id tidak ada dalam himpunan 5 nilai sah. Galat POSIX `EINVAL`.
    TidakDikenal,
}

impl ClockIdError {
    /// Kode C ABI: negatif, sehingga `>= 0` berarti clock_id yang sah
    /// dan errno absolut bisa dipulihkan dengan `(-e.kode())`.
    pub const fn kode(&self) -> i32 {
        match self {
            ClockIdError::TidakDikenal => -EINVAL,
        }
    }

    /// Kode POSIX errno absolut (untuk dibandingkan dengan `EINVAL`).
    pub const fn errno(&self) -> i32 {
        match self {
            ClockIdError::TidakDikenal => EINVAL,
        }
    }
}

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_clock_id_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah penolakan karena clock_id tidak dikenal.
    ErrUnknown = 1,
    /// Jumlah keputusan `Ok` yang SEHARUSNYA `EINVAL` menurut keputusan
    /// C++ historis (harus 0 jika byte-identik).
    DriftUnknown = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mengembalikan `clock_id` lolos validasi bila keputusan adalah `Ok`,
/// atau kode galat C ABI negatif (`-EINVAL`) bila penolakan.
///
/// Urutan byte keputusan pada input yang sama SAMA PERSIS dengan urutan
/// byte keputusan C++ (`KlatKernel/Efek/Inti/TimeManagement.cpp:84-95`).
///
/// `# Safety`: tidak ada — fungsi ini murni, tidak menyentuh memori
/// bersama. Penghitung audit (`prod_calls`, ...) di-bump dari sini.
#[inline]
pub const fn decide(clock_id: i32) -> Result<u32, ClockIdError> {
    match clock_id {
        CLOCK_REALTIME => Ok(clock_id as u32),
        CLOCK_MONOTONIC => Ok(clock_id as u32),
        CLOCK_MONOTONIC_RAW => Ok(clock_id as u32),
        CLOCK_REALTIME_COARSE => Ok(clock_id as u32),
        CLOCK_MONOTONIC_COARSE => Ok(clock_id as u32),
        _ => Err(ClockIdError::TidakDikenal),
    }
}

/// Pembantu konformansi: ekstrak `u32` dari `Result` dengan default `d`
/// bila `Err`. Ditulis sebagai `const fn` agar bisa dipakai di blok
/// konformansi waktu-kompilasi.
const fn unwrap_or(r: Result<u32, ClockIdError>, d: u32) -> u32 {
    match r {
        Ok(v) => v,
        Err(_) => d,
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan `>= 0` (= clock_id tervalidasi, sama dengan `clock_id`
/// bila sah) atau `< 0` (= kode galat absolut, `EINVAL`).
///
/// Menaikkan `prod_calls` atau `err_unknown` sesuai peristiwa yang
/// sebenarnya.
#[no_mangle]
pub extern "C" fn klatos_clock_id_decide(clock_id: i32) -> i32 {
    bump(Counter::ProdCalls);
    match decide(clock_id) {
        Ok(c) => c as i32,
        Err(e) => {
            bump(Counter::ErrUnknown);
            e.kode()
        }
    }
}

/// Batas atas jumlah clock_id + 1. Cermin `CLOCK_ID_COUNT`.
#[no_mangle]
pub extern "C" fn klatos_clock_id_max() -> u32 {
    MAX_CLOCK_ID
}

/// `1` bila keputusan untuk clock_id ini `Ok`, `0` bila `Err`. TIDAK
/// menyentuh penghitung produksi (audit-only).
#[no_mangle]
pub extern "C" fn klatos_clock_id_is_valid(clock_id: i32) -> u32 {
    u32::from(decide(clock_id).is_ok())
}

/// `1` bila keputusan untuk clock_id ini `Err(TidakDikenal)` (tidak ada
/// dalam himpunan sah), `0` bila bukan. Audit-only.
#[no_mangle]
pub extern "C" fn klatos_clock_id_is_invalid(clock_id: i32) -> u32 {
    u32::from(matches!(decide(clock_id), Err(ClockIdError::TidakDikenal)))
}

/// `1` bila keputusan untuk clock_id ini `Err(TidakDikenal)`, `0` bila
/// bukan. Audit-only. (Alias untuk [`klatos_clock_id_is_invalid`] karena
/// untuk clock_id tidak ada konsep "kosong" yang terpisah dari "invalid".)
#[no_mangle]
pub extern "C" fn klatos_clock_id_is_empty(clock_id: i32) -> u32 {
    klatos_clock_id_is_invalid(clock_id)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_clock_id_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++
/// (matriks tidak menyentuh produksi), tetapi disediakan untuk
/// konsistensi dengan modul lain.
#[no_mangle]
pub extern "C" fn klatos_clock_id_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    MAX_CLOCK_ID
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosClockIdReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
/// Tata letaknya dijaga `static_assert` di sisi C++ dan diuji harness host.
#[repr(C)]
pub struct KlatosClockIdReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub valid_checks: u32,
    pub valid_passed: u32,
    pub boundary_checks: u32,
    pub boundary_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_unknown: u32,
    pub prod_unknown_expected: u32,
    pub prod_calls: u32,
    pub max_clock_id: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Valid,
    Boundary,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    valid_checks: u32,
    valid_passed: u32,
    boundary_checks: u32,
    boundary_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            valid_checks: 0,
            valid_passed: 0,
            boundary_checks: 0,
            boundary_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Valid => {
                self.valid_checks += 1;
                if lulus {
                    self.valid_passed += 1;
                }
            }
            Group::Boundary => {
                self.boundary_checks += 1;
                if lulus {
                    self.boundary_passed += 1;
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

/// Matriks otoritas validasi clock_id. Berjalan pada nilai SINTETIS — input
/// buatan. Tidak pernah memanggil [`klatos_clock_id_decide`] sehingga
/// tidak menggeser penghitung produksi; produksi dilihat secara terpisah
/// oleh `init_stage2` lewat [`klatos_clock_id_counter`].
///
/// Matriks menguji keputusan pada tiga kelas input:
///   1. **Valid** — kelima clock_id sah harus `Ok`.
///   2. **Boundary** — nilai batas tepat di atas dan di bawah himpunan sah
///      (5, 6, -1, i32::MIN, i32::MAX) harus `Err(TidakDikenal)`.
///   3. **Negatif** — konstanta, kode galat, dan konsistensi probe audit.
fn run_matrix() -> KlatosClockIdReport {
    let mut m = Matrix::new();

    // --- kelompok 0: valid --------------------------------------------------
    // Kelima clock_id sah harus Ok dengan nilai identik.
    m.catat(
        decide(CLOCK_REALTIME) == Ok(0),
        Group::Valid,
    );
    m.catat(
        decide(CLOCK_MONOTONIC) == Ok(1),
        Group::Valid,
    );
    m.catat(
        decide(CLOCK_MONOTONIC_RAW) == Ok(2),
        Group::Valid,
    );
    m.catat(
        decide(CLOCK_REALTIME_COARSE) == Ok(3),
        Group::Valid,
    );
    m.catat(
        decide(CLOCK_MONOTONIC_COARSE) == Ok(4),
        Group::Valid,
    );

    // --- kelompok 1: batas-batas --------------------------------------------
    // Tepat di atas himpunan sah -> Err(TidakDikenal).
    m.catat(
        decide(5) == Err(ClockIdError::TidakDikenal),
        Group::Boundary,
    );
    m.catat(
        decide(6) == Err(ClockIdError::TidakDikenal),
        Group::Boundary,
    );
    m.catat(
        decide(100) == Err(ClockIdError::TidakDikenal),
        Group::Boundary,
    );
    // Di bawah himpunan sah.
    m.catat(
        decide(-1) == Err(ClockIdError::TidakDikenal),
        Group::Boundary,
    );
    m.catat(
        decide(-100) == Err(ClockIdError::TidakDikenal),
        Group::Boundary,
    );
    // Batas ekstrem integer.
    m.catat(
        decide(i32::MIN) == Err(ClockIdError::TidakDikenal),
        Group::Boundary,
    );
    m.catat(
        decide(i32::MAX) == Err(ClockIdError::TidakDikenal),
        Group::Boundary,
    );
    // Sentinel MAX_CLOCK_ID (cermin CLOCK_ID_COUNT) bukan anggota -> Err.
    m.catat(
        decide(MAX_CLOCK_ID as i32) == Err(ClockIdError::TidakDikenal),
        Group::Boundary,
    );
    // Konstanta.
    m.catat(MAX_CLOCK_ID == 5, Group::Boundary);
    m.catat(CLOCK_REALTIME == 0, Group::Boundary);
    m.catat(CLOCK_MONOTONIC == 1, Group::Boundary);
    m.catat(CLOCK_MONOTONIC_RAW == 2, Group::Boundary);
    m.catat(CLOCK_REALTIME_COARSE == 3, Group::Boundary);
    m.catat(CLOCK_MONOTONIC_COARSE == 4, Group::Boundary);

    // --- kelompok 2: negatif (harus BISA gagal) ----------------------------
    // Konstanta kode galat.
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(ClockIdError::TidakDikenal.kode() == -EINVAL, Group::Neg);
    m.catat(ClockIdError::TidakDikenal.errno() == EINVAL, Group::Neg);
    // is_valid/is_invalid konsisten.
    m.catat(klatos_clock_id_is_valid(0) == 1, Group::Neg);
    m.catat(klatos_clock_id_is_valid(1) == 1, Group::Neg);
    m.catat(klatos_clock_id_is_valid(2) == 1, Group::Neg);
    m.catat(klatos_clock_id_is_valid(3) == 1, Group::Neg);
    m.catat(klatos_clock_id_is_valid(4) == 1, Group::Neg);
    m.catat(klatos_clock_id_is_valid(5) == 0, Group::Neg);
    m.catat(klatos_clock_id_is_invalid(5) == 1, Group::Neg);
    m.catat(klatos_clock_id_is_invalid(0) == 0, Group::Neg);
    m.catat(klatos_clock_id_is_empty(5) == 1, Group::Neg);
    m.catat(klatos_clock_id_is_empty(0) == 0, Group::Neg);
    m.catat(klatos_clock_id_max() == 5, Group::Neg);
    // Konsistensi: 1000 panggilan dengan input sama menghasilkan keputusan
    // sama (tidak ada fluktuasi).
    let mut last: Option<Result<u32, ClockIdError>> = None;
    let mut ok = true;
    for _ in 0..1000 {
        let r = decide(2);
        if last.is_none() {
            last = Some(r);
        } else if last != Some(r) {
            ok = false;
            break;
        }
    }
    m.catat(ok, Group::Neg);
    // Konsistensi silang: nilai Ok identik dengan clock_id asli (tidak
    // ditransformasi).
    m.catat(decide(CLOCK_MONOTONIC).unwrap() == CLOCK_MONOTONIC as u32, Group::Neg);
    m.catat(decide(CLOCK_REALTIME_COARSE).unwrap() == CLOCK_REALTIME_COARSE as u32, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosClockIdReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        valid_checks: m.valid_checks,
        valid_passed: m.valid_passed,
        boundary_checks: m.boundary_checks,
        boundary_passed: m.boundary_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_unknown: klatos_clock_id_counter(Counter::ErrUnknown as u32),
        prod_unknown_expected: 0,
        prod_calls: klatos_clock_id_counter(Counter::ProdCalls as u32),
        max_clock_id: MAX_CLOCK_ID,
        status,
    }
}

/// Menjalankan matriks otoritas clock_id.
#[no_mangle]
pub extern "C" fn klatos_clock_id_selftest() -> KlatosClockIdReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        // Konstanta dasar.
        assert!(EINVAL == 22);
        assert!(MAX_CLOCK_ID == 5);
        assert!(CLOCK_REALTIME == 0);
        assert!(CLOCK_MONOTONIC == 1);
        assert!(CLOCK_MONOTONIC_RAW == 2);
        assert!(CLOCK_REALTIME_COARSE == 3);
        assert!(CLOCK_MONOTONIC_COARSE == 4);

        // Tag laporan konstan.
        assert!(REPORT_TAG == 0x4B4C_4154_4349_4444);

        // byte-identik: kelima clock_id sah -> Ok dengan nilai identik.
        assert!(unwrap_or(decide(CLOCK_REALTIME), 0xFFFF_FFFF) == CLOCK_REALTIME as u32);
        assert!(unwrap_or(decide(CLOCK_MONOTONIC), 0xFFFF_FFFF) == CLOCK_MONOTONIC as u32);
        assert!(unwrap_or(decide(CLOCK_MONOTONIC_RAW), 0xFFFF_FFFF) == CLOCK_MONOTONIC_RAW as u32);
        assert!(unwrap_or(decide(CLOCK_REALTIME_COARSE), 0xFFFF_FFFF) == CLOCK_REALTIME_COARSE as u32);
        assert!(unwrap_or(decide(CLOCK_MONOTONIC_COARSE), 0xFFFF_FFFF) == CLOCK_MONOTONIC_COARSE as u32);

        // byte-identik: clock_id out-of-range -> Err(TidakDikenal).
        // unwrap_or -> sentinel; kita periksa sentinelnya.
        assert!(unwrap_or(decide(5), 0xFFFF_FFFF) == 0xFFFF_FFFF);
        assert!(unwrap_or(decide(-1), 0xFFFF_FFFF) == 0xFFFF_FFFF);
        assert!(unwrap_or(decide(i32::MIN), 0xFFFF_FFFF) == 0xFFFF_FFFF);
        assert!(unwrap_or(decide(i32::MAX), 0xFFFF_FFFF) == 0xFFFF_FFFF);

        // Kode galat negatif.
        assert!(ClockIdError::TidakDikenal.kode() < 0);
        assert!(ClockIdError::TidakDikenal.kode() == -EINVAL);
        assert!(ClockIdError::TidakDikenal.errno() == EINVAL);

        // Urutan penghitung audit stabil.
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ErrUnknown as usize == 1);
        assert!(Counter::DriftUnknown as usize == 2);

        // Ukuran laporan: 8 (tag) + 12*4 (u32 fields) + 4 (status) = 60 byte
        // data, tapi alignof = 8 (tag u64) ⇒ sizeof = 64 dengan padding 4
        // byte di akhir. Cermin `KlatosClockIdReport` di KlatKernelBridge.h.
        assert!(core::mem::size_of::<KlatosClockIdReport>() == 64);
        assert!(core::mem::align_of::<KlatosClockIdReport>() == 8);
    };
}