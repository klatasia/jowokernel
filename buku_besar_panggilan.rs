// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas **buku besar bookkeeping dispatch panggilan sistem** Jowo (P1.47).

use core::sync::atomic::{AtomicU32, Ordering};

/// Batas atas nomor panggilan sistem yang dilacak (tabel berbatas).
pub const MAX_FUNGSI: u32 = 256;

/// Jumlah kelas hasil dispatch.
pub const N_KELAS: u32 = 5;

/// Tag laporan; cermin `KlatosBukuBesarPanggilanReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5359_5343; // "KLATSYSC"

/// Kelas hasil dispatch.
pub const KELAS_OK: u32 = 0;
/// Nomor fungsi >= `n_fungsi` (di luar `Function::__Count`).
pub const KELAS_TAK_DIKENAL: u32 = 1;
/// `kode_error == enosys`.
pub const KELAS_ENOSYS: u32 = 2;
/// `kode_error == epelanggaran` (`EPROMISEVIOLATION`).
pub const KELAS_PELANGGARAN: u32 = 3;
/// Error lain (kode apa pun yang bukan 0/enosys/epelanggaran).
pub const KELAS_ERROR_LAIN: u32 = 4;

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_panggilan_catat`] dari produksi.
    ProdDicatat = 0,
    /// Jumlah kelas OK yang dicatat.
    ProdOk = 1,
    /// Jumlah kelas TAK_DIKENAL yang dicatat.
    ProdTakDikenal = 2,
    /// Jumlah kelas ENOSYS yang dicatat.
    ProdEnosys = 3,
    /// Jumlah kelas PELANGGARAN yang dicatat.
    ProdPelanggaran = 4,
    /// Jumlah kelas ERROR_LAIN yang dicatat.
    ProdErrorLain = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Buku besar utama
// ---------------------------------------------------------------------------

/// Banyak dispatch per kelas hasil. Diindeks oleh `KELAS_*`.
static KELAS: [AtomicU32; N_KELAS as usize] = [const { AtomicU32::new(0) }; N_KELAS as usize];

/// Banyak dispatch per nomor fungsi. Diindeks oleh nomor fungsi (0..MAX_FUNGSI).
static FUNGSI: [AtomicU32; MAX_FUNGSI as usize] = [const { AtomicU32::new(0) }; MAX_FUNGSI as usize];

/// Total dispatch yang dicatat (agregat seluruh [`KELAS`]).
static TOTAL: AtomicU32 = AtomicU32::new(0);

/// Banyak nomor fungsi berbeda yang pernah dipakai.
static FUNGSI_AKTIF: AtomicU32 = AtomicU32::new(0);

/// Klasifikasi + pencatatan. Mengembalikan kelas (0..N_KELAS) bila dicatat,
/// atau `N_KELAS` bila nomor fungsi di luar tabel berbatas (`>= MAX_FUNGSI`).
///
/// `fungsi` bertipe `u64` agar nomor `FlatPtr` yang sangat besar TIDAK
/// terpotong sebelum diklasifikasikan.
fn catat_impl(fungsi: u64, n_fungsi: u32, kode_error: i32, enosys: i32, epelanggaran: i32) -> u32 {
    bump(Counter::ProdDicatat);

    // Nomor di luar tabel berbatas: tetap dihitung ke TOTAL, tidak ke FUNGSI.
    if fungsi >= MAX_FUNGSI as u64 {
        TOTAL.fetch_add(1, Ordering::Relaxed);
        return N_KELAS;
    }

    let kelas = if fungsi >= n_fungsi as u64 {
        KELAS_TAK_DIKENAL
    } else if kode_error == 0 {
        KELAS_OK
    } else if kode_error == enosys {
        KELAS_ENOSYS
    } else if kode_error == epelanggaran {
        KELAS_PELANGGARAN
    } else {
        KELAS_ERROR_LAIN
    };

    let prev = FUNGSI[fungsi as usize].fetch_add(1, Ordering::Relaxed);
    if prev == 0 {
        FUNGSI_AKTIF.fetch_add(1, Ordering::Relaxed);
    }
    KELAS[kelas as usize].fetch_add(1, Ordering::Relaxed);
    TOTAL.fetch_add(1, Ordering::Relaxed);

    match kelas {
        KELAS_OK => bump(Counter::ProdOk),
        KELAS_TAK_DIKENAL => bump(Counter::ProdTakDikenal),
        KELAS_ENOSYS => bump(Counter::ProdEnosys),
        KELAS_PELANGGARAN => bump(Counter::ProdPelanggaran),
        _ => bump(Counter::ProdErrorLain),
    }

    kelas
}

/// Reset seluruh buku besar ke 0. TIDAK dipanggil dari produksi.
fn reset_impl() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    for k in KELAS.iter() {
        k.store(0, Ordering::Release);
    }
    for f in FUNGSI.iter() {
        f.store(0, Ordering::Release);
    }
    TOTAL.store(0, Ordering::Release);
    FUNGSI_AKTIF.store(0, Ordering::Release);
}

// ---------------------------------------------------------------------------
// Algoritma — oracle pembanding deterministik
// ---------------------------------------------------------------------------

/// Oracle: salinan deterministik buku besar (array Rust biasa).
struct NaifBukuBesar {
    kelas: [u32; N_KELAS as usize],
    fungsi: [u32; MAX_FUNGSI as usize],
    total: u32,
    fungsi_aktif: u32,
}

impl NaifBukuBesar {
    const fn new() -> Self {
        NaifBukuBesar {
            kelas: [0u32; N_KELAS as usize],
            fungsi: [0u32; MAX_FUNGSI as usize],
            total: 0,
            fungsi_aktif: 0,
        }
    }
    fn catat(&mut self, fungsi: u64, n_fungsi: u32, kode_error: i32, enosys: i32, epelanggaran: i32) -> u32 {
        if fungsi >= MAX_FUNGSI as u64 {
            self.total += 1;
            return N_KELAS;
        }
        let kelas = if fungsi >= n_fungsi as u64 {
            KELAS_TAK_DIKENAL
        } else if kode_error == 0 {
            KELAS_OK
        } else if kode_error == enosys {
            KELAS_ENOSYS
        } else if kode_error == epelanggaran {
            KELAS_PELANGGARAN
        } else {
            KELAS_ERROR_LAIN
        };
        if self.fungsi[fungsi as usize] == 0 {
            self.fungsi_aktif += 1;
        }
        self.fungsi[fungsi as usize] += 1;
        self.kelas[kelas as usize] += 1;
        self.total += 1;
        kelas
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengklasifikasikan + mencatat satu hasil dispatch.
///
/// * `fungsi` — nomor panggilan sistem (`Function`).
/// * `n_fungsi` — `Function::__Count` (jumlah nomor sah).
/// * `kode_error` — 0 bila sukses, selain itu `Error::code()`.
/// * `enosys` — konstanta ABI `ENOSYS` (milik C++).
/// * `epelanggaran` — konstanta ABI `EPROMISEVIOLATION` (milik C++).
///
/// Mengembalikan kelas `0..N_KELAS`, atau `N_KELAS` bila `fungsi >= MAX_FUNGSI`.
#[no_mangle]
pub extern "C" fn klatos_panggilan_catat(
    fungsi: u64,
    n_fungsi: u32,
    kode_error: i32,
    enosys: i32,
    epelanggaran: i32,
) -> u32 {
    catat_impl(fungsi, n_fungsi, kode_error, enosys, epelanggaran)
}

/// Banyak dispatch pada kelas `kelas`. 0 bila di luar `0..N_KELAS`.
#[no_mangle]
pub extern "C" fn klatos_panggilan_kelas(kelas: u32) -> u32 {
    if kelas >= N_KELAS {
        return 0;
    }
    KELAS[kelas as usize].load(Ordering::Relaxed)
}

/// Banyak dispatch pada nomor fungsi `fungsi`. 0 bila di luar `0..MAX_FUNGSI`.
#[no_mangle]
pub extern "C" fn klatos_panggilan_fungsi(fungsi: u32) -> u32 {
    if fungsi >= MAX_FUNGSI {
        return 0;
    }
    FUNGSI[fungsi as usize].load(Ordering::Relaxed)
}

/// Total dispatch yang dicatat.
#[no_mangle]
pub extern "C" fn klatos_panggilan_total() -> u32 {
    TOTAL.load(Ordering::Relaxed)
}

/// Banyak nomor fungsi berbeda yang pernah dipakai.
#[no_mangle]
pub extern "C" fn klatos_panggilan_fungsi_aktif() -> u32 {
    FUNGSI_AKTIF.load(Ordering::Relaxed)
}

/// Konstanta `N_KELAS`.
#[no_mangle]
pub extern "C" fn klatos_panggilan_n_kelas() -> u32 {
    N_KELAS
}

/// Konstanta `MAX_FUNGSI`.
#[no_mangle]
pub extern "C" fn klatos_panggilan_max_fungsi() -> u32 {
    MAX_FUNGSI
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_panggilan_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset ke 0.
#[no_mangle]
pub extern "C" fn klatos_panggilan_reset() -> u32 {
    reset_impl();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosBukuBesarPanggilanReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosBukuBesarPanggilanReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_dicatat: u32,
    pub prod_ok: u32,
    pub prod_tak_dikenal: u32,
    pub prod_enosys: u32,
    pub prod_pelanggaran: u32,
    pub prod_error_lain: u32,
    pub n_kelas: u32,
    pub max_fungsi: u32,
    pub total: u32,
    pub fungsi_aktif: u32,
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

/// Matriks buku besar dispatch. Input SINTETIS.
///
/// `n_fungsi` sintetis = 128 (nomor sah 0..128), `enosys` = 38,
/// `epelanggaran` = 60. Nilai ABI NYATA diambil dari C++ saat produksi; matriks
/// memakai nilai tetap agar deterministik.
fn run_matrix() -> KlatosBukuBesarPanggilanReport {
    const N_FUNGSI_SINTETIS: u32 = 128;
    const ENOSYS_SINTETIS: i32 = 38;
    const EPELANGGARAN_SINTETIS: i32 = 60;

    let mut m = Matrix::new();

    // --- kelompok 0: konstanta -------------------------------------------
    m.catat(MAX_FUNGSI == 256, Group::Peta);
    m.catat(N_KELAS == 5, Group::Peta);
    m.catat(REPORT_TAG == 0x4B4C_4154_5359_5343, Group::Peta);
    m.catat(KELAS_OK == 0, Group::Peta);
    m.catat(KELAS_TAK_DIKENAL == 1, Group::Peta);
    m.catat(KELAS_ENOSYS == 2, Group::Peta);
    m.catat(KELAS_PELANGGARAN == 3, Group::Peta);
    m.catat(KELAS_ERROR_LAIN == 4, Group::Peta);
    m.catat(klatos_panggilan_n_kelas() == 5, Group::Peta);
    m.catat(klatos_panggilan_max_fungsi() == 256, Group::Peta);
    m.catat(klatos_panggilan_kelas(99) == 0, Group::Peta);
    m.catat(klatos_panggilan_fungsi(999) == 0, Group::Peta);
    m.catat(klatos_panggilan_counter(99) == 0, Group::Peta);

    // --- kelompok 1: oracle vs produksi untuk urutan deterministik --------
    reset_impl();
    let mut oracle = NaifBukuBesar::new();

    // 200 sukses pada nomor 0..40.
    for i in 0..200u32 {
        let f = i % 40;
        let r = catat_impl(f as u64, N_FUNGSI_SINTETIS, 0, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        let o = oracle.catat(f as u64, N_FUNGSI_SINTETIS, 0, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        m.catat(r == o && r == KELAS_OK, Group::Peta);
    }
    // 80 ENOSYS pada nomor 0..20.
    for i in 0..80u32 {
        let f = i % 20;
        let r = catat_impl(f as u64, N_FUNGSI_SINTETIS, ENOSYS_SINTETIS, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        let o = oracle.catat(f as u64, N_FUNGSI_SINTETIS, ENOSYS_SINTETIS, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        m.catat(r == o && r == KELAS_ENOSYS, Group::Peta);
    }
    // 40 pelanggaran pada nomor 5..15.
    for i in 0..40u32 {
        let f = 5 + (i % 10);
        let r = catat_impl(f as u64, N_FUNGSI_SINTETIS, EPELANGGARAN_SINTETIS, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        let o = oracle.catat(f as u64, N_FUNGSI_SINTETIS, EPELANGGARAN_SINTETIS, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        m.catat(r == o && r == KELAS_PELANGGARAN, Group::Peta);
    }
    // 60 error lain (EINVAL=22) pada nomor 0..30.
    for i in 0..60u32 {
        let f = i % 30;
        let r = catat_impl(f as u64, N_FUNGSI_SINTETIS, 22, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        let o = oracle.catat(f as u64, N_FUNGSI_SINTETIS, 22, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        m.catat(r == o && r == KELAS_ERROR_LAIN, Group::Peta);
    }
    // 30 tak dikenal (nomor 200..229 >= n_fungsi 128, < MAX_FUNGSI 256).
    for i in 0..30u32 {
        let f = 200 + i;
        let r = catat_impl(f as u64, N_FUNGSI_SINTETIS, 0, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        let o = oracle.catat(f as u64, N_FUNGSI_SINTETIS, 0, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        m.catat(r == o && r == KELAS_TAK_DIKENAL, Group::Peta);
    }

    // Bandingkan produksi vs oracle per-kelas dan per-fungsi.
    for k in 0..N_KELAS {
        m.catat(klatos_panggilan_kelas(k) == oracle.kelas[k as usize], Group::Peta);
    }
    for f in 0..MAX_FUNGSI {
        m.catat(klatos_panggilan_fungsi(f) == oracle.fungsi[f as usize], Group::Peta);
    }
    m.catat(klatos_panggilan_total() == oracle.total, Group::Peta);
    m.catat(klatos_panggilan_fungsi_aktif() == oracle.fungsi_aktif, Group::Peta);
    // Agregat kelas == total.
    let mut agg: u32 = 0;
    for &v in oracle.kelas.iter() {
        agg += v;
    }
    m.catat(agg == oracle.total, Group::Peta);
    m.catat(klatos_panggilan_total() == agg, Group::Peta);

    // --- kelompok 2: batas tabel berbatas --------------------------------
    // Nomor >= MAX_FUNGSI: dihitung ke TOTAL tapi TIDAK ke FUNGSI/KELAS.
    let total_sebelum = klatos_panggilan_total();
    let kelas_sebelum: [u32; N_KELAS as usize] = [
        klatos_panggilan_kelas(0),
        klatos_panggilan_kelas(1),
        klatos_panggilan_kelas(2),
        klatos_panggilan_kelas(3),
        klatos_panggilan_kelas(4),
    ];
    for _ in 0..7u32 {
        let r = catat_impl(MAX_FUNGSI as u64, N_FUNGSI_SINTETIS, 0, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS);
        m.catat(r == N_KELAS, Group::Peta);
    }
    m.catat(klatos_panggilan_total() == total_sebelum + 7, Group::Peta);
    for k in 0..N_KELAS {
        m.catat(klatos_panggilan_kelas(k) == kelas_sebelum[k as usize], Group::Peta);
    }

    // --- kelompok 3: determinisme (baca berulang jawaban sama) ------------
    {
        let prev_total = klatos_panggilan_total();
        let prev_aktif = klatos_panggilan_fungsi_aktif();
        let mut ok = true;
        for _ in 0..10_000 {
            if klatos_panggilan_total() != prev_total
                || klatos_panggilan_fungsi_aktif() != prev_aktif
            {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Peta);
    }

    // --- kelompok 4: negatif ----------------------------------------------
    reset_impl();
    m.catat(klatos_panggilan_total() == 0, Group::Neg);
    m.catat(klatos_panggilan_fungsi_aktif() == 0, Group::Neg);
    for k in 0..N_KELAS {
        m.catat(klatos_panggilan_kelas(k) == 0, Group::Neg);
    }
    for f in 0..MAX_FUNGSI {
        m.catat(klatos_panggilan_fungsi(f) == 0, Group::Neg);
    }
    // Kelas di luar jangkauan -> 0.
    m.catat(klatos_panggilan_kelas(N_KELAS) == 0, Group::Neg);
    m.catat(klatos_panggilan_kelas(0xFFFFFFFF) == 0, Group::Neg);
    m.catat(klatos_panggilan_fungsi(0xFFFFFFFF) == 0, Group::Neg);
    // Nomor tepat di batas n_fungsi -> TAK_DIKENAL (bukan OK).
    m.catat(
        catat_impl(N_FUNGSI_SINTETIS as u64, N_FUNGSI_SINTETIS, 0, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS)
            == KELAS_TAK_DIKENAL,
        Group::Neg,
    );
    // kode_error 0 pada nomor sah -> OK, bukan ERROR_LAIN.
    m.catat(
        catat_impl(3u64, N_FUNGSI_SINTETIS, 0, ENOSYS_SINTETIS, EPELANGGARAN_SINTETIS) == KELAS_OK,
        Group::Neg,
    );

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosBukuBesarPanggilanReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_dicatat: klatos_panggilan_counter(Counter::ProdDicatat as u32),
        prod_ok: klatos_panggilan_counter(Counter::ProdOk as u32),
        prod_tak_dikenal: klatos_panggilan_counter(Counter::ProdTakDikenal as u32),
        prod_enosys: klatos_panggilan_counter(Counter::ProdEnosys as u32),
        prod_pelanggaran: klatos_panggilan_counter(Counter::ProdPelanggaran as u32),
        prod_error_lain: klatos_panggilan_counter(Counter::ProdErrorLain as u32),
        n_kelas: N_KELAS,
        max_fungsi: MAX_FUNGSI,
        total: klatos_panggilan_total(),
        fungsi_aktif: klatos_panggilan_fungsi_aktif(),
        status,
    }
}

/// Menjalankan matriks buku besar dispatch.
#[no_mangle]
pub extern "C" fn klatos_panggilan_selftest() -> KlatosBukuBesarPanggilanReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(MAX_FUNGSI == 256);
        assert!(N_KELAS == 5);
        assert!(REPORT_TAG == 0x4B4C_4154_5359_5343);
        assert!(KELAS_OK == 0);
        assert!(KELAS_TAK_DIKENAL == 1);
        assert!(KELAS_ENOSYS == 2);
        assert!(KELAS_PELANGGARAN == 3);
        assert!(KELAS_ERROR_LAIN == 4);
        assert!(Counter::ProdDicatat as usize == 0);
        assert!(Counter::ProdOk as usize == 1);
        assert!(Counter::ProdTakDikenal as usize == 2);
        assert!(Counter::ProdEnosys as usize == 3);
        assert!(Counter::ProdPelanggaran as usize == 4);
        assert!(Counter::ProdErrorLain as usize == 5);
        assert!(N_COUNTER == 6);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76, dibulatkan ke kelipatan
        // align 8 -> 80.
        assert!(core::mem::size_of::<KlatosBukuBesarPanggilanReport>() == 80);
        assert!(core::mem::align_of::<KlatosBukuBesarPanggilanReport>() == 8);
    };
}
