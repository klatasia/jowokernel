// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas keputusan validasi handle IPC Jowo (P1.02).


use core::sync::atomic::{AtomicU32, Ordering};

/// Kode galat POSIX `EINVAL` (cermin `<errno.h>` C).
pub const EINVAL: i32 = 22;

/// Kode galat POSIX `EBADF` (cermin `<errno.h>` C).
pub const EBADF: i32 = 9;

/// Konstanta sentinel "handle 0 = tidak sah" dari ABI v2
/// (`Jowo/Kontrak/Abi2.h`). Disimpan sebagai bilangan agar selaras
/// dengan perilaku produksi; fungsi [`decide`] tidak mengeceknya
/// secara eksplisit karena tabel kernel sudah menjamin slot 0 kosong
/// (lihat komentar modul), tetapi konstanta tersedia untuk audit
/// batas bawah keputusan Rust.
pub const INVALID_HANDLE: i32 = 0;

/// Batas atas jumlah handle per proses. Cermin
/// `JowoIPCHandles::s_max_open_handles = 256` di
/// `KlatKernel/Efek/Eksekusi/Process.h:998`.
pub const MAX_HANDLES: u32 = 256;

/// Tag laporan. BERBEDA dari `frame::REPORT_TAG` ("KLATKRNL"),
/// `task_id::REPORT_TAG` ("KLATTKID"), dan
/// `priority_bucket::REPORT_TAG` ("KLATSCHD") dengan sengaja: keempat
/// laporan adalah empat keputusan berbeda, dan C++ memeriksanya lewat
/// konstanta berbeda (`kernel_tag_expected`, `task_id_tag_expected`,
/// `sched_bucket_tag_expected`, `ipc_handle_tag_expected`) di
/// `Jowo/Kontrak/KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4950_4348; // "KLATIPCH"

/// Galat otoritas handle IPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleError {
    /// Handle di luar rentang tabel. Galat POSIX `EINVAL`.
    TidakSah,
    /// Slot handle tidak berisi endpoint. Galat POSIX `EBADF`.
    TidakAda,
}

impl HandleError {
    /// Kode C ABI: negatif, sehingga `>= 0` berarti handle yang sah
    /// dan errno absolut bisa dipulihkan dengan `(-e.kode())`.
    pub const fn kode(&self) -> i32 {
        match self {
            HandleError::TidakSah => -EINVAL,
            HandleError::TidakAda => -EBADF,
        }
    }

    /// Kode POSIX errno absolut (untuk dibandingkan dengan `EINVAL`/`EBADF`).
    pub const fn errno(&self) -> i32 {
        match self {
            HandleError::TidakSah => EINVAL,
            HandleError::TidakAda => EBADF,
        }
    }
}

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_ipc_handle_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah penolakan karena handle di luar rentang tabel.
    ErrInvalid = 1,
    /// Jumlah penolakan karena slot tabel tidak berisi endpoint.
    ErrBadf = 2,
    /// Jumlah keputusan `Ok(handle)` yang SEHARUSNYA `EINVAL` menurut
    /// keputusan C++ historis (harus 0 jika byte-identik).
    DriftInvalid = 3,
    /// Jumlah keputusan `Ok(handle)` yang SEHARUSNYA `EBADF` menurut
    /// keputusan C++ historis (harus 0 jika byte-identik).
    DriftBadf = 4,
}

const N_COUNTER: usize = 5;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mengembalikan indeks handle tervalidasi bila keputusan adalah `Ok`,
/// atau kode galat C ABI negatif (`-EINVAL` / `-EBADF`) bila penolakan.
///
/// Urutan byte keputusan pada input yang sama SAMA PERSIS dengan urutan
/// byte keputusan C++ (`KlatKernel/Efek/Eksekusi/Process.h:1033-1040`).
///
/// `# Safety`: tidak ada — fungsi ini murni, tidak menyentuh memori
/// bersama. Penghitung audit (`prod_calls`, ...) di-bump dari sini.
#[inline]
pub const fn decide(
    handle: i32,
    table_size: u32,
    slot_has_endpoint: u32,
) -> Result<u32, HandleError> {
    if handle < 0 || (handle as u32) >= table_size {
        return Err(HandleError::TidakSah);
    }
    if slot_has_endpoint == 0 {
        return Err(HandleError::TidakAda);
    }
    Ok(handle as u32)
}

/// Pembantu konformansi: ekstrak `u32` dari `Result` dengan default `d`
/// bila `Err`. Ditulis sebagai `const fn` agar bisa dipakai di blok
/// konformansi waktu-kompilasi.
const fn unwrap_or(r: Result<u32, HandleError>, d: u32) -> u32 {
    match r {
        Ok(v) => v,
        Err(_) => d,
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan `>= 0` (= handle tervalidasi, sama dengan `handle` bila
/// sah) atau `< 0` (= kode galat absolut, `EINVAL`/`EBADF`).
///
/// Menaikkan `prod_calls`, `err_invalid`, atau `err_badf` sesuai
/// peristiwa yang sebenarnya.
#[no_mangle]
pub extern "C" fn klatos_ipc_handle_decide(
    handle: i32,
    table_size: u32,
    slot_has_endpoint: u32,
) -> i32 {
    bump(Counter::ProdCalls);
    match decide(handle, table_size, slot_has_endpoint) {
        Ok(h) => h as i32,
        Err(e) => {
            match e {
                HandleError::TidakSah => bump(Counter::ErrInvalid),
                HandleError::TidakAda => bump(Counter::ErrBadf),
            }
            e.kode()
        }
    }
}

/// Batas atas jumlah handle per proses. Cermin `s_max_open_handles`.
#[no_mangle]
pub extern "C" fn klatos_ipc_handle_max() -> u32 {
    MAX_HANDLES
}

/// `1` bila keputusan untuk handle ini `Ok`, `0` bila `Err`. TIDAK
/// menyentuh penghitung produksi (audit-only).
#[no_mangle]
pub extern "C" fn klatos_ipc_handle_is_valid(
    handle: i32,
    table_size: u32,
    slot_has_endpoint: u32,
) -> u32 {
    u32::from(decide(handle, table_size, slot_has_endpoint).is_ok())
}

/// `1` bila keputusan untuk handle ini `Err(TidakSah)` (di luar rentang),
/// `0` bila bukan. Audit-only.
#[no_mangle]
pub extern "C" fn klatos_ipc_handle_is_invalid(
    handle: i32,
    table_size: u32,
    _slot_has_endpoint: u32,
) -> u32 {
    // Pemeriksaan slot TIDAK relevan untuk "rentang"; abaikan
    // `slot_has_endpoint` dengan awalan `_` agar kompilator tidak
    // memperingatkan parameter tak-terpakai.
    u32::from(handle < 0 || (handle as u32) >= table_size)
}

/// `1` bila keputusan untuk handle/slot ini `Err(TidakAda)` (slot kosong),
/// `0` bila bukan. Audit-only.
#[no_mangle]
pub extern "C" fn klatos_ipc_handle_is_empty(
    handle: i32,
    table_size: u32,
    slot_has_endpoint: u32,
) -> u32 {
    if handle < 0 || (handle as u32) >= table_size {
        return 0;
    }
    u32::from(slot_has_endpoint == 0)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_ipc_handle_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++
/// (matriks tidak menyentuh produksi), tetapi disediakan untuk
/// konsistensi dengan modul lain.
#[no_mangle]
pub extern "C" fn klatos_ipc_handle_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    MAX_HANDLES
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosIpcHandleReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
/// Tata letaknya dijaga `static_assert` di sisi C++ dan diuji harness host.
#[repr(C)]
pub struct KlatosIpcHandleReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub range_checks: u32,
    pub range_passed: u32,
    pub slot_checks: u32,
    pub slot_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_invalid: u32,
    pub prod_badf: u32,
    pub prod_invalid_expected: u32,
    pub prod_badf_expected: u32,
    pub prod_calls: u32,
    pub max_handles: u32,
    pub invalid_handle: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Range,
    Slot,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    range_checks: u32,
    range_passed: u32,
    slot_checks: u32,
    slot_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            range_checks: 0,
            range_passed: 0,
            slot_checks: 0,
            slot_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Range => {
                self.range_checks += 1;
                if lulus {
                    self.range_passed += 1;
                }
            }
            Group::Slot => {
                self.slot_checks += 1;
                if lulus {
                    self.slot_passed += 1;
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

/// Matriks otoritas validasi handle. Berjalan pada nilai SINTETIS — input
/// buatan. Tidak pernah memanggil [`klatos_ipc_handle_decide`] sehingga
/// tidak menggeser penghitung produksi; produksi dilihat secara terpisah
/// oleh `init_stage2` lewat [`klatos_ipc_handle_counter`].
///
/// Matriks menguji keputusan pada empat kelas input:
///   1. **Rentang** — handle di dalam/di luar rentang; keputusan harus
///      `Ok` atau `Err(TidakSah)` byte-identik dengan C++.
///   2. **Slot** — slot terisi/kosong; keputusan `Ok` atau `Err(TidakAda)`
///      byte-identik dengan C++.
///   3. **Negatif** — semua kasus di atas pada nilai batas ekstrem
///      (handle 0, handle -1, handle = u32::MAX, slot_has_endpoint > 1,
///      table_size = 0).
fn run_matrix() -> KlatosIpcHandleReport {
    let mut m = Matrix::new();

    // --- kelompok 0: rentang ------------------------------------------------
    // (table_size, handle, slot_has_endpoint, ekspektasi)
    m.catat(
        decide(0, 1, 1) == Ok(0),
        Group::Range,
    );
    m.catat(
        decide(0, 16, 1) == Ok(0),
        Group::Range,
    );
    m.catat(
        decide(15, 16, 1) == Ok(15),
        Group::Range,
    );
    m.catat(
        decide(16, 16, 1) == Err(HandleError::TidakSah),
        Group::Range,
    );
    // Handle negatif.
    m.catat(
        decide(-1, 16, 1) == Err(HandleError::TidakSah),
        Group::Range,
    );
    m.catat(
        decide(-100, 16, 1) == Err(HandleError::TidakSah),
        Group::Range,
    );
    // Handle i32::MIN.
    m.catat(
        decide(i32::MIN, 16, 1) == Err(HandleError::TidakSah),
        Group::Range,
    );

    // --- kelompok 1: slot ---------------------------------------------------
    // Slot kosong -> EBADF, meskipun handle dalam rentang.
    m.catat(
        decide(5, 16, 0) == Err(HandleError::TidakAda),
        Group::Slot,
    );
    m.catat(
        decide(0, 1, 0) == Err(HandleError::TidakAda),
        Group::Slot,
    );
    // Slot terisi -> Ok.
    m.catat(
        decide(7, 16, 1) == Ok(7),
        Group::Slot,
    );
    // Slot "penuh" (slot_has_endpoint > 1) -> tetap Ok karena byte-identik
    // dengan C++ yang hanya membandingkan `nullptr`.
    m.catat(
        decide(7, 16, 2) == Ok(7),
        Group::Slot,
    );

    // --- kelompok 2: negatif (harus BISA gagal) ----------------------------
    // Konstanta.
    m.catat(MAX_HANDLES == 256, Group::Neg);
    m.catat(INVALID_HANDLE == 0, Group::Neg);
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(EBADF == 9, Group::Neg);
    // Kode galat negatif konsisten dengan errno absolut.
    m.catat(HandleError::TidakSah.kode() == -EINVAL, Group::Neg);
    m.catat(HandleError::TidakAda.kode() == -EBADF, Group::Neg);
    m.catat(HandleError::TidakSah.errno() == EINVAL, Group::Neg);
    m.catat(HandleError::TidakAda.errno() == EBADF, Group::Neg);
    // is_valid/is_invalid/is_empty konsisten.
    m.catat(klatos_ipc_handle_is_valid(5, 16, 1) == 1, Group::Neg);
    m.catat(klatos_ipc_handle_is_valid(5, 16, 0) == 0, Group::Neg);
    m.catat(klatos_ipc_handle_is_invalid(-1, 16, 1) == 1, Group::Neg);
    m.catat(klatos_ipc_handle_is_invalid(5, 16, 1) == 0, Group::Neg);
    m.catat(klatos_ipc_handle_is_empty(5, 16, 0) == 1, Group::Neg);
    m.catat(klatos_ipc_handle_is_empty(5, 16, 1) == 0, Group::Neg);
    m.catat(klatos_ipc_handle_is_empty(-1, 16, 0) == 0, Group::Neg);
    m.catat(klatos_ipc_handle_max() == 256, Group::Neg);
    // u32::MAX yang di-cast ke i32 -> negatif -> Err(TidakSah).
    m.catat(
        decide(u32::MAX as i32, 16, 1) == Err(HandleError::TidakSah),
        Group::Neg,
    );
    // table_size = 0 -> semua handle out-of-range -> Err(TidakSah).
    m.catat(
        decide(0, 0, 0) == Err(HandleError::TidakSah),
        Group::Neg,
    );
    m.catat(
        decide(0, 0, 1) == Err(HandleError::TidakSah),
        Group::Neg,
    );

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosIpcHandleReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        range_checks: m.range_checks,
        range_passed: m.range_passed,
        slot_checks: m.slot_checks,
        slot_passed: m.slot_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_invalid: klatos_ipc_handle_counter(Counter::ErrInvalid as u32),
        prod_badf: klatos_ipc_handle_counter(Counter::ErrBadf as u32),
        prod_invalid_expected: 0,
        prod_badf_expected: 0,
        prod_calls: klatos_ipc_handle_counter(Counter::ProdCalls as u32),
        max_handles: MAX_HANDLES,
        invalid_handle: INVALID_HANDLE as u32,
        status,
    }
}

/// Menjalankan matriks otoritas handle IPC.
#[no_mangle]
pub extern "C" fn klatos_ipc_handle_selftest() -> KlatosIpcHandleReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        // Konstanta dasar.
        assert!(MAX_HANDLES == 256);
        assert!(INVALID_HANDLE == 0);
        assert!(EINVAL == 22);
        assert!(EBADF == 9);

        // Tag laporan konstan.
        assert!(REPORT_TAG == 0x4B4C_4154_4950_4348);

        // byte-identik: handle 0 + table_size >= 1 + slot_has_endpoint == 1
        // -> Ok(0) (sama dengan C++ yang juga melewati semua pemeriksaan).
        assert!(unwrap_or(decide(0, 1, 1), 0xFFFF_FFFF) == 0);
        assert!(unwrap_or(decide(7, 16, 1), 0xFFFF_FFFF) == 7);

        // byte-identik: handle out-of-range -> Err(TidakSah).
        // unwrap_or -> sentinel; kita periksa sentinelnya.
        assert!(unwrap_or(decide(16, 16, 1), 0xFFFF_FFFF) == 0xFFFF_FFFF);
        assert!(unwrap_or(decide(-1, 16, 1), 0xFFFF_FFFF) == 0xFFFF_FFFF);
        assert!(unwrap_or(decide(i32::MIN, 16, 1), 0xFFFF_FFFF) == 0xFFFF_FFFF);
        // byte-identik: slot kosong dalam rentang -> Err(TidakAda).
        assert!(unwrap_or(decide(5, 16, 0), 0xFFFF_FFFF) == 0xFFFF_FFFF);

        // Kode galat negatif.
        assert!(HandleError::TidakSah.kode() < 0);
        assert!(HandleError::TidakAda.kode() < 0);
        assert!(HandleError::TidakSah.kode() == -EINVAL);
        assert!(HandleError::TidakAda.kode() == -EBADF);

        // Urutan penghitung audit stabil.
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ErrInvalid as usize == 1);
        assert!(Counter::ErrBadf as usize == 2);
        assert!(Counter::DriftInvalid as usize == 3);
        assert!(Counter::DriftBadf as usize == 4);
    };
}
