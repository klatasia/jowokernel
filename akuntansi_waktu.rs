// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.38 — OTORITAS AKUNTANSI WAKTU CPU (Rust).


use core::cell::Cell;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Tag laporan: "KLATAWKT".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4157_4B54;

const RENTANG_DIFERENSIAL: u64 = 65536;
const N_COUNTER: usize = 4;

// ---------------------------------------------------------------------------
// Oracle historis
// ---------------------------------------------------------------------------

/// Transisi C++ historis: `total += d; if (k) total_kernel += d;`
pub const fn kebijakan_lama(total: u64, total_kernel: u64, d: u64, is_kernel: bool) -> (u64, u64) {
    let t = total.wrapping_add(d);
    let tk = if is_kernel {
        total_kernel.wrapping_add(d)
    } else {
        total_kernel
    };
    (t, tk)
}

// ---------------------------------------------------------------------------
// Algoritma TUNGGAL
// ---------------------------------------------------------------------------

/// Penyimpanan akuntansi. Abstraksi ini ada supaya matriks memakai algoritma
/// produksi yang sama, bukan salinannya.
pub trait PenyimpananAkuntansi {
    fn muat_total(&self) -> u64;
    fn muat_kernel(&self) -> u64;
    fn tambah_total(&self, d: u64);
    fn tambah_kernel(&self, d: u64);
}

/// Satu langkah produksi.
pub fn tambah_in<S: PenyimpananAkuntansi>(s: &S, d: u64, is_kernel: bool) {
    s.tambah_total(d);
    if is_kernel {
        s.tambah_kernel(d);
    }
}

// ---------------------------------------------------------------------------
// Penyimpanan PRODUKSI
// ---------------------------------------------------------------------------

/// Total waktu CPU terjadwal. Menggantikan `g_total_time_scheduled.total`.
/// `.0` = `total`, `.1` = `total_kernel` — SATU penyimpanan, dibaca dan
/// ditulis jalur produksi yang sama.
static PROD: (AtomicU64, AtomicU64) = (AtomicU64::new(0), AtomicU64::new(0));

/// `0` = tambah, `1` = tambah_kernel, `2` = tambah_delta_nol, `3` = galat.
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

impl PenyimpananAkuntansi for (AtomicU64, AtomicU64) {
    fn muat_total(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
    fn muat_kernel(&self) -> u64 {
        self.1.load(Ordering::Relaxed)
    }
    fn tambah_total(&self, d: u64) {
        self.0.fetch_add(d, Ordering::Relaxed);
    }
    fn tambah_kernel(&self, d: u64) {
        self.1.fetch_add(d, Ordering::Relaxed);
    }
}

/// Penyimpanan SINTETIS untuk matriks.
pub struct PenyimpananUjiAkuntansi {
    total: Cell<u64>,
    kernel: Cell<u64>,
}

impl PenyimpananUjiAkuntansi {
    pub const fn baru(total: u64, kernel: u64) -> Self {
        PenyimpananUjiAkuntansi {
            total: Cell::new(total),
            kernel: Cell::new(kernel),
        }
    }
    pub fn intip(&self) -> (u64, u64) {
        (self.total.get(), self.kernel.get())
    }
}

impl PenyimpananAkuntansi for PenyimpananUjiAkuntansi {
    fn muat_total(&self) -> u64 {
        self.total.get()
    }
    fn muat_kernel(&self) -> u64 {
        self.kernel.get()
    }
    fn tambah_total(&self, d: u64) {
        self.total.set(self.total.get().wrapping_add(d));
    }
    fn tambah_kernel(&self, d: u64) {
        self.kernel.set(self.kernel.get().wrapping_add(d));
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — PRODUKSI
// ---------------------------------------------------------------------------

/// Menambahkan `delta` ke akuntansi. `is_kernel != 0` juga menambah kernel.
#[no_mangle]
pub extern "C" fn klatos_akuntansi_waktu_tambah(delta: u64, is_kernel: u32) {
    if delta == 0 {
        COUNTERS[2].fetch_add(1, Ordering::Relaxed);
    }
    tambah_in(&PROD, delta, is_kernel != 0);
    COUNTERS[0].fetch_add(1, Ordering::Relaxed);
    if is_kernel != 0 {
        COUNTERS[1].fetch_add(1, Ordering::Relaxed);
    }
}

/// Total waktu terjadwal. Dibaca SETELAH `..._total_kernel()` oleh pemanggil
/// agar `total_kernel <= total` selalu berlaku (lihat dokumentasi modul).
#[no_mangle]
pub extern "C" fn klatos_akuntansi_waktu_total() -> u64 {
    PROD.muat_total()
}

/// Total waktu mode kernel. Dibaca SEBELUM `..._total()`.
#[no_mangle]
pub extern "C" fn klatos_akuntansi_waktu_total_kernel() -> u64 {
    PROD.muat_kernel()
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_akuntansi_waktu_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAkuntansiReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosAkuntansiReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_a: u32,
    pub prod_b: u32,
    pub prod_c: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

struct Matrix {
    checks: u32,
    passed: u32,
    grup_checks: [u32; 3],
    grup_passed: [u32; 3],
}

impl Matrix {
    const fn baru() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            grup_checks: [0; 3],
            grup_passed: [0; 3],
        }
    }
    fn catat(&mut self, lulus: bool, grup: usize) {
        self.checks += 1;
        self.grup_checks[grup] += 1;
        if lulus {
            self.passed += 1;
            self.grup_passed[grup] += 1;
        }
    }
}

fn run_matrix() -> KlatosAkuntansiReport {
    let mut m = Matrix::baru();

    // RACE: penghitung PROD diperbarui Scheduler.cpp:458 setiap kali terjadi
    // pergantian thread, termasuk dari tick timer. Pemeriksaan KONTROL di
    // bawah menuntut keadaan produksi TIDAK bergeser, jadi matriks harus
    // berjalan tanpa preemption. Tanpa ini otoritas akuntansi_waktu kadang
    // ditolak P148 (terbukti: gagal_verifikasi=1 pada boot D-8-FIX).
    let flags = crate::arch::simpan_dan_matikan_interupsi();

    let prod_before = (
        PROD.muat_total(),
        PROD.muat_kernel(),
    );

    // --- kelompok 0: akumulasi dasar ---------------------------------------
    {
        let s = PenyimpananUjiAkuntansi::baru(0, 0);
        tambah_in(&s, 100, false);
        m.catat(s.intip() == (100, 0), 0);
        tambah_in(&s, 50, true);
        m.catat(s.intip() == (150, 50), 0);
        tambah_in(&s, 0, true);
        m.catat(s.intip() == (150, 50), 0); // delta 0 tidak mengubah apa pun
        tambah_in(&s, 25, false);
        m.catat(s.intip() == (175, 50), 0); // non-kernel tidak menyentuh kernel
        m.catat(s.muat_kernel() <= s.muat_total(), 0);

        // 1000 akumulasi: total = jumlah, kernel = jumlah subset.
        let t = PenyimpananUjiAkuntansi::baru(0, 0);
        let mut i = 0u64;
        let mut jml = 0u64;
        let mut jml_k = 0u64;
        while i < 1000 {
            let d = (i * 7) % 101;
            let k = (i % 3) == 0;
            tambah_in(&t, d, k);
            jml += d;
            if k {
                jml_k += d;
            }
            i += 1;
        }
        m.catat(t.intip() == (jml, jml_k), 0);
        m.catat(t.muat_kernel() <= t.muat_total(), 0);
    }

    // --- kelompok 1: diferensial terhadap oracle historis -------------------
    let mut diukur = 0u32;
    let mut beda = 0u32;
    let mut i = 0u64;
    while i < RENTANG_DIFERENSIAL {
        let total = i.wrapping_mul(1_000_003);
        let kernel = i.wrapping_mul(7919);
        let d = i.wrapping_mul(31).wrapping_add(7);
        let is_kernel = (i & 1) == 0;
        let s = PenyimpananUjiAkuntansi::baru(total, kernel);
        tambah_in(&s, d, is_kernel);
        diukur += 1;
        let (et, ek) = kebijakan_lama(total, kernel, d, is_kernel);
        if s.intip() != (et, ek) {
            beda += 1;
        }
        i += 1;
    }
    m.catat(beda == 0, 1);
    m.catat(diukur == RENTANG_DIFERENSIAL as u32, 1);

    // Diferensial untuk is_kernel=false juga (mengunci cabang `if`).
    {
        let mut b2 = 0u32;
        let mut k = 0u64;
        while k < 4096 {
            let s = PenyimpananUjiAkuntansi::baru(k, k / 2);
            tambah_in(&s, k + 1, false);
            diukur += 1;
            let (et, ek) = kebijakan_lama(k, k / 2, k + 1, false);
            if s.intip() != (et, ek) {
                b2 += 1;
            }
            k += 1;
        }
        m.catat(b2 == 0, 1);
    }
    m.catat(diukur == RENTANG_DIFERENSIAL as u32 + 4096, 1);

    // --- kelompok 2: negatif ------------------------------------------------
    {
        // Pembungkusan u64 sama seperti C++ (bukan penolakan).
        let s = PenyimpananUjiAkuntansi::baru(u64::MAX, 0);
        tambah_in(&s, 1, false);
        m.catat(s.intip() == (0, 0), 2);
        let s2 = PenyimpananUjiAkuntansi::baru(0, 0);
        tambah_in(&s2, u64::MAX, true);
        m.catat(s2.intip() == (u64::MAX, u64::MAX), 2);

        // `is_kernel = false` TIDAK menyentuh penghitung kernel.
        let s3 = PenyimpananUjiAkuntansi::baru(10, 7);
        tambah_in(&s3, 5, false);
        m.catat(s3.intip() == (15, 7), 2);

        // Urutan baca pembaca produksi menjaga `total_kernel <= total`.
        m.catat(PROD.muat_kernel() <= PROD.muat_total(), 2);

        // KONTROL: keadaan PRODUKSI tidak bergeser.
        m.catat(
            (PROD.muat_total(), PROD.muat_kernel()) == prod_before,
            2,
        );
    }

    let mut report = KlatosAkuntansiReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.grup_checks[1],
        peta_passed: m.grup_passed[1],
        neg_checks: m.grup_checks[2],
        neg_passed: m.grup_passed[2],
        prod_a: klatos_akuntansi_waktu_counter(0),
        prod_b: klatos_akuntansi_waktu_counter(1),
        prod_c: 0,
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status: 0,
    };
    if report.checks != report.passed {
        report.status = -1;
    }
    crate::arch::pulihkan_interupsi(flags);
    report
}

/// Menjalankan matriks otoritas akuntansi waktu CPU.
#[no_mangle]
pub extern "C" fn klatos_akuntansi_waktu_selftest() -> KlatosAkuntansiReport {
    run_matrix()
}

/// Konformansi waktu-kompilasi.
pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4157_4B54); // "KLATAWKT"
        // Oracle: cabang kernel dan non-kernel (per-bidang, karena `PartialEq`
        // tuple belum `const`).
        assert!(kebijakan_lama(10, 5, 3, true).0 == 13);
        assert!(kebijakan_lama(10, 5, 3, true).1 == 8);
        assert!(kebijakan_lama(10, 5, 3, false).0 == 13);
        assert!(kebijakan_lama(10, 5, 3, false).1 == 5);
        assert!(kebijakan_lama(0, 0, 0, true).0 == 0);
        assert!(kebijakan_lama(0, 0, 0, true).1 == 0);
        assert!(core::mem::size_of::<KlatosAkuntansiReport>() == 56);
        assert!(N_COUNTER == 4);
    };
}
