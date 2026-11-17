// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas keputusan ember prioritas Scheduler (P1.01).


use core::sync::atomic::{AtomicU32, Ordering};

/// Prioritas minimum. Cermin `THREAD_PRIORITY_MIN` di
/// `Jowo/Kontrak/POSIX/klatos.h:43`.
pub const PRIORITY_MIN: u32 = 1;

/// Prioritas maksimum. Cermin `THREAD_PRIORITY_MAX` di
/// `Jowo/Kontrak/POSIX/klatos.h:47`.
pub const PRIORITY_MAX: u32 = 99;

/// Jumlah tingkat prioritas yang sah. Cermin
/// `THREAD_PRIORITY_MAX - THREAD_PRIORITY_MIN + 1` di C++.
pub const PRIORITY_COUNT: u32 = PRIORITY_MAX - PRIORITY_MIN + 1;

/// Jumlah ember siap. Cermin `ThreadReadyQueues::count = sizeof(mask) * 8`
/// dengan `mask: u32` di `Scheduler.cpp:47-51`.
pub const BUCKET_COUNT: u32 = 32;

/// Ember minimum yang dapat dipilih pada prioritas sah. Saat ini `0` (dicapai
/// oleh prioritas apa pun kecuali `PRIORITY_MIN`).
pub const BUCKET_MIN: u32 = 0;

/// Ember maksimum yang dapat dipilih pada prioritas sah. Saat ini `31`
/// (dicapai oleh `PRIORITY_MIN`).
pub const BUCKET_MAX: u32 = BUCKET_COUNT - 1;

/// Tag laporan. BERBEDA dari `frame::REPORT_TAG` ("KLATKRNL") dan
/// `task_id::REPORT_TAG` ("KLATTKID") dengan sengaja: ketiga laporan adalah
/// tiga keputusan berbeda, dan C++ memeriksanya lewat konstanta berbeda
/// (`kernel_tag_expected`, `task_id_tag_expected`,
/// `sched_bucket_tag_expected`) di `Jowo/Kontrak/KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5343_4844; // "KLATSCHD"

/// Galat otoritas ember.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BucketError {
    /// Prioritas di bawah [`PRIORITY_MIN`] atau di atas [`PRIORITY_MAX`].
    TidakSah,
}

impl BucketError {
    /// Kode C ABI: negatif, sehingga `>= 0` berarti ember yang sah.
    pub const fn kode(&self) -> i32 {
        match self {
            BucketError::TidakSah => -1,
        }
    }
}

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_sched_bucket_index`] dari produksi.
    ProdCalls = 0,
    /// Jumlah penolakan input di luar rentang.
    Err = 1,
}

const N_COUNTER: usize = 2;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Menghitung ember siap untuk sebuah prioritas.
///
/// Catatan jujur tentang presisi: rumus C++ memakai `/` integer.
/// `(count - 1)` di belakang juga dikalikan dengan hasil bagi integer.
/// Untuk `PRIORITY_COUNT = 99` dan `BUCKET_COUNT = 32`:
///
/// - prioritas tertinggi (`PRIORITY_MIN = 1`): `(99 - 0) / 99 = 1` → ember 31.
/// - prioritas terendah (`PRIORITY_MAX = 99`): `(99 - 98) / 99 = 0` → ember 0.
///
/// Karena `PRIORITY_COUNT > 31`, hasil bagi integer SELALU `0` atau `1`,
/// sehingga keputusan jatuh ke ember `0` atau `31`. Ember `1..=30` tidak
/// pernah dipilih oleh prioritas integer — itu bukan keputusan modul ini,
/// melainkan konsekuensi aritmetika rumus. Karena itu matriks menguji
/// titik-titik penting (terendah, tertinggi, validasi rentang) dan BUKAN
/// menguji setiap prioritas — `enqueue_runnable_thread` hanya menaruh
/// thread ke ember `0` atau `31` pada prioritas integer.
#[inline]
pub const fn index_in(priority: u32) -> Result<u32, BucketError> {
    if priority < PRIORITY_MIN || priority > PRIORITY_MAX {
        return Err(BucketError::TidakSah);
    }
    let priority_count: u32 = PRIORITY_MAX - PRIORITY_MIN + 1;
    // C++: ((priority_count - (priority - PRIORITY_MIN)) / priority_count) * (count - 1)
    let bucket: u32 = ((priority_count - (priority - PRIORITY_MIN)) / priority_count)
        * (BUCKET_COUNT - 1);
    debug_assert!(bucket < BUCKET_COUNT, "bucket out of range");
    Ok(bucket)
}

/// Pembantu konformansi: ekstrak nilai `u32` dari `Result` dengan default
/// `d` bila `Err`. Ditulis sebagai `const fn` agar bisa dipakai di blok
/// konformansi.
const fn unwrap_or(r: Result<u32, BucketError>, d: u32) -> u32 {
    match r {
        Ok(v) => v,
        Err(_) => d,
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan ember siap untuk sebuah prioritas. Hasil `>= 0` adalah
/// indeks ember (0..=31). Hasil `< 0` adalah kode galat.
///
/// Menaikkan `prod_calls` dan `err` sesuai peristiwa yang sebenarnya.
#[no_mangle]
pub extern "C" fn klatos_sched_bucket_index(priority: u32) -> i32 {
    bump(Counter::ProdCalls);
    match index_in(priority) {
        Ok(b) => b as i32,
        Err(e) => {
            bump(Counter::Err);
            e.kode()
        }
    }
}

/// Jumlah ember siap. Cermin `ThreadReadyQueues::count`.
#[no_mangle]
pub extern "C" fn klatos_sched_bucket_count() -> u32 {
    BUCKET_COUNT
}

/// `1` bila prioritas ini diizinkan, `0` bila tidak. TIDAK menyentuh
/// penghitung produksi (audit-only).
#[no_mangle]
pub extern "C" fn klatos_sched_bucket_is_valid(priority: u32) -> u32 {
    u32::from(index_in(priority).is_ok())
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_sched_bucket_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. Saat ini TIDAK dipanggil dari C++
/// (matriks tidak menyentuh produksi), tetapi disediakan untuk konsistensi
/// dengan modul lain.
#[no_mangle]
pub extern "C" fn klatos_sched_bucket_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    BUCKET_COUNT
}

/// Ember minimum yang akan dipilih untuk SEBUAH prioritas dalam rentang
/// produksi. Cermin `BUCKET_MIN` (saat ini `0`).
#[no_mangle]
pub extern "C" fn klatos_sched_bucket_min() -> u32 {
    BUCKET_MIN
}

/// Ember maksimum yang akan dipilih untuk SEBUAH prioritas dalam rentang
/// produksi. Cermin `BUCKET_MAX` (saat ini `BUCKET_COUNT - 1`).
#[no_mangle]
pub extern "C" fn klatos_sched_bucket_max() -> u32 {
    BUCKET_MAX
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosSchedBucketReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
/// Tata letaknya dijaga `static_assert` di sisi C++ dan diuji harness host.
#[repr(C)]
pub struct KlatosSchedBucketReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub bucket_checks: u32,
    pub bucket_passed: u32,
    pub boundary_checks: u32,
    pub boundary_passed: u32,
    pub monotonic_checks: u32,
    pub monotonic_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_min: u32,
    pub prod_max: u32,
    pub prod_count: u32,
    pub prod_calls: u32,
    pub err: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Bucket,
    Boundary,
    Monotonic,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    bucket_checks: u32,
    bucket_passed: u32,
    boundary_checks: u32,
    boundary_passed: u32,
    monotonic_checks: u32,
    monotonic_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            bucket_checks: 0,
            bucket_passed: 0,
            boundary_checks: 0,
            boundary_passed: 0,
            monotonic_checks: 0,
            monotonic_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Bucket => {
                self.bucket_checks += 1;
                if lulus {
                    self.bucket_passed += 1;
                }
            }
            Group::Boundary => {
                self.boundary_checks += 1;
                if lulus {
                    self.boundary_passed += 1;
                }
            }
            Group::Monotonic => {
                self.monotonic_checks += 1;
                if lulus {
                    self.monotonic_passed += 1;
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

/// Matriks otoritas ember. Berjalan pada nilai SINTETIS — input prioritas
/// buatan. Tidak pernah memanggil [`klatos_sched_bucket_index`] sehingga
/// tidak menggeser penghitung produksi; produksi dilihat secara terpisah
/// oleh `init_stage2` lewat [`klatos_sched_bucket_counter`].
///
/// Mengapa TIDAK menguji setiap prioritas 1..=99: dengan rumus saat ini
/// prioritas maximal menghasilkan ember `31` dan prioritas minimal
/// menghasilkan ember `0`. Menguji titik-titik tersebut ditambah sebuah
/// titik tengah sudah cukup untuk membuktikan rumus Rust identik dengan
/// rumus C++.
fn run_matrix() -> KlatosSchedBucketReport {
    let mut m = Matrix::new();

    // --- kelompok 0: bucket nilai -------------------------------------------
    // Rumus identik dengan C++, jadi hasilnya HARUS identik dengan yang
    // akan dihasilkan C++ pada input yang sama.
    m.catat(index_in(PRIORITY_MIN) == Ok(BUCKET_COUNT - 1), Group::Bucket); // 1 -> 31
    m.catat(index_in(PRIORITY_MAX) == Ok(0), Group::Bucket); // 99 -> 0
    // Titik tengah: prioritas 50 -> ((99 - 49) / 99) * 31 = (50/99)*31 = 0*31 = 0
    m.catat(index_in(50) == Ok(0), Group::Bucket);
    // Bukti bahwa hasilnya ada di rentang ember yang valid.
    for p in [PRIORITY_MIN, 2u32, 25, 50, 75, 98, PRIORITY_MAX] {
        let r = index_in(p);
        m.catat(r.is_ok() && r.unwrap() < BUCKET_COUNT, Group::Bucket);
    }

    // --- kelompok 1: batas-batas --------------------------------------------
    m.catat(index_in(PRIORITY_MIN) == Ok(BUCKET_COUNT - 1), Group::Boundary);
    // PRIORITY_MIN + 1 sudah jatuh ke ember 0: `99 / 99 == 1` hanya untuk
    // prioritas = PRIORITY_MIN. Prioritas lain menghasilkan ember 0.
    m.catat(index_in(PRIORITY_MIN + 1) == Ok(0), Group::Boundary);
    m.catat(index_in(PRIORITY_MAX) == Ok(0), Group::Boundary);
    m.catat(index_in(PRIORITY_MAX - 1) == Ok(0), Group::Boundary);
    // Konstanta.
    m.catat(BUCKET_COUNT == 32, Group::Boundary);
    m.catat(PRIORITY_COUNT == 99, Group::Boundary);
    m.catat(PRIORITY_MIN == 1, Group::Boundary);
    m.catat(PRIORITY_MAX == 99, Group::Boundary);

    // --- kelompok 2: konsistensi --------------------------------------------
    // Prioritas yang sama -> ember yang sama. Dipanggil 1000 kali.
    let mut last: Option<Result<u32, BucketError>> = None;
    let mut ok = true;
    for _ in 0..1000 {
        let r = index_in(42);
        if last.is_none() {
            last = Some(r);
        } else if last != Some(r) {
            ok = false;
            break;
        }
    }
    m.catat(ok, Group::Monotonic);

    // Dua prioritas berbeda menghasilkan dua hasil yang BUKAN keduanya 31
    // (karena 31 hanya muncul pada prioritas tertinggi).
    let r_low = index_in(PRIORITY_MAX).unwrap();
    let r_high = index_in(PRIORITY_MIN).unwrap();
    m.catat(r_low == 0 && r_high == BUCKET_COUNT - 1, Group::Monotonic);

    // --- kelompok 3: negatif (harus BISA gagal) ----------------------------
    // Di bawah PRIORITY_MIN.
    m.catat(index_in(0) == Err(BucketError::TidakSah), Group::Neg);
    m.catat(
        index_in(PRIORITY_MIN - 1) == Err(BucketError::TidakSah),
        Group::Neg,
    );
    // Di atas PRIORITY_MAX.
    m.catat(
        index_in(PRIORITY_MAX + 1) == Err(BucketError::TidakSah),
        Group::Neg,
    );
    m.catat(index_in(u32::MAX) == Err(BucketError::TidakSah), Group::Neg);
    // Kode galat negatif.
    m.catat(BucketError::TidakSah.kode() < 0, Group::Neg);
    // is_valid konsisten.
    m.catat(klatos_sched_bucket_is_valid(50) == 1, Group::Neg);
    m.catat(klatos_sched_bucket_is_valid(0) == 0, Group::Neg);
    m.catat(klatos_sched_bucket_is_valid(u32::MAX) == 0, Group::Neg);
    m.catat(klatos_sched_bucket_count() == 32, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosSchedBucketReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        bucket_checks: m.bucket_checks,
        bucket_passed: m.bucket_passed,
        boundary_checks: m.boundary_checks,
        boundary_passed: m.boundary_passed,
        monotonic_checks: m.monotonic_checks,
        monotonic_passed: m.monotonic_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_min: BUCKET_MIN,
        prod_max: BUCKET_MAX,
        prod_count: 0, // matriks tidak memanggil produksi
        prod_calls: klatos_sched_bucket_counter(Counter::ProdCalls as u32),
        err: klatos_sched_bucket_counter(Counter::Err as u32),
        status,
    }
}

/// Menjalankan matriks otoritas ember.
#[no_mangle]
pub extern "C" fn klatos_sched_bucket_selftest() -> KlatosSchedBucketReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        // Konstanta dasar.
        assert!(PRIORITY_MIN == 1);
        assert!(PRIORITY_MAX == 99);
        assert!(PRIORITY_COUNT == 99);
        assert!(BUCKET_COUNT == 32);
        assert!(BUCKET_MIN == 0);
        assert!(BUCKET_MAX == 31);

        // Rumus: prioritas tertinggi -> ember tertinggi, dan sebaliknya.
        assert!(unwrap_or(index_in(PRIORITY_MIN), 0xFFFF_FFFF) == BUCKET_COUNT - 1);
        assert!(unwrap_or(index_in(PRIORITY_MAX), 0xFFFF_FFFF) == 0);

        // Penolakan: ember ekstraksi gagal -> `unwrap_or` mengembalikan
        // sentinel 0xFFFF_FFFF; kita periksa sentinelnya.
        assert!(unwrap_or(index_in(0), 0xFFFF_FFFF) == 0xFFFF_FFFF);
        assert!(unwrap_or(index_in(PRIORITY_MAX + 1), 0xFFFF_FFFF) == 0xFFFF_FFFF);

        // Ukuran laporan: 8 + 16*4 = 72 byte. Sama dengan cermin C++.
        assert!(core::mem::size_of::<KlatosSchedBucketReport>() == 72);

        // Kode galat negatif.
        assert!(BucketError::TidakSah.kode() < 0);

        // Urutan penghitung audit stabil.
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::Err as usize == 1);

        // Tag laporan konstan.
        assert!(REPORT_TAG == 0x4B4C_4154_5343_4844);
    };
}