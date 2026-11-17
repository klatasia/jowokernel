// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.75 / WS3 — **OTORITAS KEBIJAKAN DISPATCH PANGGILAN SISTEM**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosKebijakanPanggilanReport` di
/// `Jowo/Kontrak/KlatSeam75.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5359_5350; // "KLATSYSP"

// --- verdict admisi ---------------------------------------------------------

/// Dispatch diterima; lanjut ke pemilihan jalur + pemanggilan handler.
pub const ADMISI_LOLOS: u32 = 0;
/// Sistem sedang dimatikan; tolak dengan `ENOSYS` (tanpa pesan).
pub const ADMISI_MATI: u32 = 1;
/// Nomor fungsi `>= n_fungsi` (`Function::__Count`); tolak `ENOSYS`
/// ("Unknown syscall").
pub const ADMISI_TAK_DIKENAL: u32 = 2;
/// Slot tabel ada tapi `handler == nullptr`; tolak `ENOSYS`
/// ("Null syscall").
pub const ADMISI_HANDLER_NOL: u32 = 3;

/// Jumlah verdict admisi.
pub const N_VERDICT: u32 = 4;

// --- rute jalur -------------------------------------------------------------

/// Handler biasa: `(arg1, arg2, arg3, arg4) -> ErrorOr<FlatPtr>`.
pub const JALUR_BIASA: u32 = 0;
/// `HandlerWithRegisterState`: handler menerima `RegisterState&` (fork, sigreturn).
pub const JALUR_REGSTATE: u32 = 1;
/// Handler tidak pernah kembali ke pemanggil (exit, exit_thread).
pub const JALUR_TIDAK_KEMBALI: u32 = 2;

/// Jumlah rute.
pub const N_JALUR: u32 = 3;

// --- keputusan crash --------------------------------------------------------

/// Tidak menjatuhkan proses.
pub const CRASH_TIDAK: u32 = 0;
/// Jatuhkan proses dengan `SIGABRT` (pelanggaran janji).
pub const CRASH_YA: u32 = 1;

// --- aksi dispatch (P1.101) -------------------------------------------------
//
// Rencana dispatch mengembalikan SATU aksi yang harus dijalankan C++.
// Sebelum P1.101 C++ memiliki rantai `if (verdict != lolos)` +
// `if (jalur == ...)`; kini seluruh pemilihan aksi adalah milik Rust dan C++
// hanya mengeksekusi nilai yang dikembalikan.

/// Tolak dispatch: kembalikan `kode_error` (opsional mencetak `pesan`).
pub const AKSI_TOLAK: u32 = 0;
/// Handler tidak pernah kembali ke pemanggil (`exit`/`exit_thread`).
pub const AKSI_KEMBALI: u32 = 1;
/// Panggil `HandlerWithRegisterState` (`fork`/`sigreturn`).
pub const AKSI_REGSTATE: u32 = 2;
/// Panggil handler biasa `(arg1..arg4)`.
pub const AKSI_BIASA: u32 = 3;
/// Jumlah aksi.
pub const N_AKSI: u32 = 4;

// --- kelas pesan (P1.101) ---------------------------------------------------

/// Tidak ada pesan.
pub const PESAN_NONE: u32 = 0;
/// `"Unknown syscall ..."`.
pub const PESAN_TAK_DIKENAL: u32 = 1;
/// `"Null syscall ... you probably need to rebuild this program!"`.
pub const PESAN_HANDLER_NOL: u32 = 2;
/// Jumlah kelas pesan.
pub const N_PESAN: u32 = 3;

/// Penghitung audit (§21).
///
/// P1.101 TIDAK menambah penghitung: [`klatos_kebijakan_rencana`] menaikkan
/// [`Counter::ProdAdmisi`] (setiap rencana adalah evaluasi admisi), dan menaikkan
/// [`Counter::ProdJalur`] saat dispatch diterima atau [`Counter::ProdTolak`]
/// saat ditolak. Dengan begitu saksi produksi yang sudah ada (`P175-PROD`)
/// tetap bermakna dan tata letak laporan TIDAK berubah.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah evaluasi admisi dispatch dari produksi.
    ProdAdmisi = 0,
    /// Jumlah rute handler yang diputuskan dari produksi.
    ProdJalur = 1,
    /// Jumlah `klatos_kebijakan_harus_crash` dari produksi.
    ProdCrash = 2,
    /// Jumlah admisi yang DITOLAK dari produksi (verdict != LOLOS).
    ProdTolak = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Keputusan admisi dispatch. Transkripsi `SyscallHandler.cpp:64-80`:
///
/// ```cpp
/// if (g_in_system_shutdown)                     return ENOSYS;  // MATI
/// if (function >= Function::__Count)            return ENOSYS;  // TAK_DIKENAL
/// if (syscall_metadata.handler == nullptr)      return ENOSYS;  // HANDLER_NOL
/// // lanjut
/// ```
///
/// Presedensi: [`ADMISI_MATI`] > [`ADMISI_TAK_DIKENAL`] > [`ADMISI_HANDLER_NOL`].
///
/// * `mati` — `g_in_system_shutdown` (0/1).
/// * `fungsi` — nomor panggilan sistem (lebar `u64` supaya `FlatPtr` besar
///   TIDAK terpotong sebelum dibandingkan).
/// * `n_fungsi` — `Function::__Count`.
/// * `handler_nol` — "slot tabel `fungsi` ada dan `handler == nullptr`?" (0/1).
///   Pemanggil WAJIB menghitungnya hanya saat `fungsi < n_fungsi` (di luar
///   rentang tidak boleh mengindeks tabel).
#[inline]
pub const fn admisi(mati: u32, fungsi: u64, n_fungsi: u32, handler_nol: u32) -> u32 {
    if mati != 0 {
        return ADMISI_MATI;
    }
    if fungsi >= n_fungsi as u64 {
        return ADMISI_TAK_DIKENAL;
    }
    if handler_nol != 0 {
        return ADMISI_HANDLER_NOL;
    }
    ADMISI_LOLOS
}

/// Keputusan rute handler. Transkripsi `SyscallHandler.cpp:88-115`:
///
/// ```cpp
/// if (function == SC_exit || function == SC_exit_thread) { ...; VERIFY_NOT_REACHED(); } // TIDAK_KEMBALI
/// if (function == SC_fork || function == SC_sigreturn) regs_handler(regs);             // REGSTATE
/// else                                                 normal_handler(a1,a2,a3,a4);   // BIASA
/// ```
///
/// Presedensi: [`JALUR_TIDAK_KEMBALI`] > [`JALUR_REGSTATE`] > [`JALUR_BIASA`].
/// Nilai `sc_*` DISUPLAI C++ (nomor adalah kontrak ABI milik C++).
#[inline]
pub const fn jalur(
    fungsi: u64,
    sc_exit: u64,
    sc_exit_thread: u64,
    sc_fork: u64,
    sc_sigreturn: u64,
) -> u32 {
    if fungsi == sc_exit || fungsi == sc_exit_thread {
        return JALUR_TIDAK_KEMBALI;
    }
    if fungsi == sc_fork || fungsi == sc_sigreturn {
        return JALUR_REGSTATE;
    }
    JALUR_BIASA
}

/// Keputusan menjatuhkan proses karena pelanggaran janji. Transkripsi
/// `SyscallHandler.cpp:206-212`:
///
/// ```cpp
/// if (result.is_error() && result.error().code() == EPROMISEVIOLATION) { ...; crash(SIGABRT); }
/// ```
///
/// * `ada_error` — `result.is_error()` (0/1).
/// * `kode_error` — `result.error().code()` (diabaikan saat `ada_error == 0`).
/// * `epelanggaran` — konstanta ABI `EPROMISEVIOLATION` (milik C++).
#[inline]
pub const fn harus_crash(ada_error: u32, kode_error: i32, epelanggaran: i32) -> u32 {
    if ada_error == 0 {
        return CRASH_TIDAK;
    }
    if kode_error == epelanggaran {
        return CRASH_YA;
    }
    CRASH_TIDAK
}

/// Rencana dispatch lengkap (P1.101); cermin `KlatosRencanaDispatch` di
/// `Jowo/Kontrak/KlatSeam75.h`.
///
/// Empat medan, seluruhnya `u32`, sehingga bentuknya deterministik di seluruh
/// ABI (16 byte, align 4).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosRencanaDispatch {
    /// [`ADMISI_*`].
    pub verdict: u32,
    /// [`JALUR_*`] (bermakna hanya bila `aksi` bukan [`AKSI_TOLAK`]).
    pub jalur: u32,
    /// [`AKSI_*`] — satu-satunya hal yang dieksekusi C++.
    pub aksi: u32,
    /// [`PESAN_*`] — pesan diagnostik yang harus dicetak C++.
    pub pesan: u32,
}

impl KlatosRencanaDispatch {
    /// Rencana "tolak" dengan pesan `pesan`.
    #[inline]
    const fn tolak(verdict: u32, pesan: u32) -> Self {
        KlatosRencanaDispatch {
            verdict,
            jalur: JALUR_BIASA,
            aksi: AKSI_TOLAK,
            pesan,
        }
    }
}

/// Keputusan **rencana dispatch** (P1.101). Menggantikan pemanggilan terpisah
/// [`admisi`] + [`jalur`] dari `SyscallHandler.cpp` dengan SATU keputusan yang
/// mengembalikan aksi lengkap. C++ tidak lagi memiliki rantai `if`.
///
/// Presedensi tetap: [`ADMISI_MATI`] > [`ADMISI_TAK_DIKENAL`] >
/// [`ADMISI_HANDLER_NOL`]; lalu [`JALUR_TIDAK_KEMBALI`] > [`JALUR_REGSTATE`] >
/// [`JALUR_BIASA`].
#[inline]
pub const fn rencana(
    mati: u32,
    fungsi: u64,
    n_fungsi: u32,
    handler_nol: u32,
    sc_exit: u64,
    sc_exit_thread: u64,
    sc_fork: u64,
    sc_sigreturn: u64,
) -> KlatosRencanaDispatch {
    let v = admisi(mati, fungsi, n_fungsi, handler_nol);
    if v != ADMISI_LOLOS {
        let pesan = if v == ADMISI_TAK_DIKENAL {
            PESAN_TAK_DIKENAL
        } else if v == ADMISI_HANDLER_NOL {
            PESAN_HANDLER_NOL
        } else {
            PESAN_NONE
        };
        return KlatosRencanaDispatch::tolak(v, pesan);
    }
    let j = jalur(fungsi, sc_exit, sc_exit_thread, sc_fork, sc_sigreturn);
    let aksi = if j == JALUR_TIDAK_KEMBALI {
        AKSI_KEMBALI
    } else if j == JALUR_REGSTATE {
        AKSI_REGSTATE
    } else {
        AKSI_BIASA
    };
    KlatosRencanaDispatch {
        verdict: v,
        jalur: j,
        aksi,
        pesan: PESAN_NONE,
    }
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

/// Oracle admisi: tabel keputusan terurut, bukan rantai `if`.
fn naif_admisi(mati: u32, fungsi: u64, n_fungsi: u32, handler_nol: u32) -> u32 {
    // (predikat, verdict) dalam urutan presedensi.
    let tabel: [(bool, u32); 4] = [
        (mati != 0, ADMISI_MATI),
        (fungsi >= n_fungsi as u64, ADMISI_TAK_DIKENAL),
        (handler_nol != 0, ADMISI_HANDLER_NOL),
        (true, ADMISI_LOLOS),
    ];
    let mut hasil = ADMISI_LOLOS;
    for (pred, verdict) in tabel {
        if pred {
            hasil = verdict;
            break;
        }
    }
    hasil
}

/// Oracle jalur: `match` pada keanggotaan himpunan, bukan rantai `if`.
fn naif_jalur(fungsi: u64, sc_exit: u64, sc_exit_thread: u64, sc_fork: u64, sc_sigreturn: u64) -> u32 {
    let tidak_kembali = [sc_exit, sc_exit_thread];
    let regstate = [sc_fork, sc_sigreturn];
    if tidak_kembali.contains(&fungsi) {
        JALUR_TIDAK_KEMBALI
    } else if regstate.contains(&fungsi) {
        JALUR_REGSTATE
    } else {
        JALUR_BIASA
    }
}

/// Oracle crash: `match` pada tuple, bukan rantai `if`.
fn naif_harus_crash(ada_error: u32, kode_error: i32, epelanggaran: i32) -> u32 {
    match (ada_error != 0, kode_error) {
        (true, k) if k == epelanggaran => CRASH_YA,
        _ => CRASH_TIDAK,
    }
}

/// Kontrol NEGATIF: admisi dengan presedensi SALAH (handler-nol diperiksa
/// sebelum rentang). Dipakai matriks untuk membuktikan presedensi itu
/// berdaya — bila ia menghasilkan jawaban yang sama pada semua kasus,
/// presedensi tidak pernah diuji.
fn admisi_salah_urutan(mati: u32, fungsi: u64, n_fungsi: u32, handler_nol: u32) -> u32 {
    if mati != 0 {
        return ADMISI_MATI;
    }
    if handler_nol != 0 {
        return ADMISI_HANDLER_NOL;
    }
    if fungsi >= n_fungsi as u64 {
        return ADMISI_TAK_DIKENAL;
    }
    ADMISI_LOLOS
}

/// Oracle rencana: dibangun dari peta aksi eksplisit, bukan dari rantai `if`.
fn naif_rencana(
    mati: u32,
    fungsi: u64,
    n_fungsi: u32,
    handler_nol: u32,
    sc_exit: u64,
    sc_exit_thread: u64,
    sc_fork: u64,
    sc_sigreturn: u64,
) -> KlatosRencanaDispatch {
    let v = naif_admisi(mati, fungsi, n_fungsi, handler_nol);
    if v != ADMISI_LOLOS {
        let pesan = match v {
            ADMISI_TAK_DIKENAL => PESAN_TAK_DIKENAL,
            ADMISI_HANDLER_NOL => PESAN_HANDLER_NOL,
            _ => PESAN_NONE,
        };
        return KlatosRencanaDispatch::tolak(v, pesan);
    }
    let j = naif_jalur(fungsi, sc_exit, sc_exit_thread, sc_fork, sc_sigreturn);
    let aksi = match j {
        JALUR_TIDAK_KEMBALI => AKSI_KEMBALI,
        JALUR_REGSTATE => AKSI_REGSTATE,
        _ => AKSI_BIASA,
    };
    KlatosRencanaDispatch {
        verdict: v,
        jalur: j,
        aksi,
        pesan: PESAN_NONE,
    }
}

/// Kontrol NEGATIF rencana: aksi dihitung dengan urutan jalur TERBALIK
/// (regstate diperiksa sebelum tidak-kembali). Bila jawabannya identik pada
/// semua kasus, presedensi jalur tidak pernah diuji.
fn rencana_salah_urutan(
    mati: u32,
    fungsi: u64,
    n_fungsi: u32,
    handler_nol: u32,
    sc_exit: u64,
    sc_exit_thread: u64,
    sc_fork: u64,
    sc_sigreturn: u64,
) -> KlatosRencanaDispatch {
    let v = admisi(mati, fungsi, n_fungsi, handler_nol);
    if v != ADMISI_LOLOS {
        return KlatosRencanaDispatch::tolak(v, PESAN_NONE);
    }
    let aksi = if fungsi == sc_fork || fungsi == sc_sigreturn {
        AKSI_REGSTATE
    } else if fungsi == sc_exit || fungsi == sc_exit_thread {
        AKSI_KEMBALI
    } else {
        AKSI_BIASA
    };
    KlatosRencanaDispatch {
        verdict: v,
        jalur: JALUR_BIASA,
        aksi,
        pesan: PESAN_NONE,
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Keputusan **rencana dispatch** produksi (P1.101). Lihat [`rencana`].
///
/// Menggantikan dua pintu lama (`klatos_kebijakan_admisi` +
/// `klatos_kebijakan_jalur`) dengan SATU keputusan yang mengembalikan aksi
/// lengkap. `SyscallHandler.cpp` tidak lagi memiliki rantai `if` admisi/rute.
///
/// Penghitung: `ProdAdmisi` selalu; `ProdJalur` saat diterima; `ProdTolak`
/// saat ditolak. Saksi `P175-PROD` tetap bermakna tanpa perubahan.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_rencana(
    mati: u32,
    fungsi: u64,
    n_fungsi: u32,
    handler_nol: u32,
    sc_exit: u64,
    sc_exit_thread: u64,
    sc_fork: u64,
    sc_sigreturn: u64,
) -> KlatosRencanaDispatch {
    bump(Counter::ProdAdmisi);
    let r = rencana(
        mati,
        fungsi,
        n_fungsi,
        handler_nol,
        sc_exit,
        sc_exit_thread,
        sc_fork,
        sc_sigreturn,
    );
    if r.aksi == AKSI_TOLAK {
        bump(Counter::ProdTolak);
    } else {
        bump(Counter::ProdJalur);
    }
    r
}

// P1.101 — pintu C ABI `klatos_kebijakan_jalur` **DIHAPUS**: rute kini
// dikembalikan di dalam [`klatos_kebijakan_rencana`]. Fungsi murni [`jalur`]
// tetap ada (dipakai [`rencana`] dan matriks); tidak ada pintu kedua.

/// Keputusan menjatuhkan proses. Lihat [`harus_crash`].
#[no_mangle]
pub extern "C" fn klatos_kebijakan_harus_crash(ada_error: u32, kode_error: i32, epelanggaran: i32) -> u32 {
    bump(Counter::ProdCrash);
    harus_crash(ada_error, kode_error, epelanggaran)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_kebijakan_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosKebijakanPanggilanReport` di `Jowo/Kontrak/KlatSeam75.h`.
#[repr(C)]
pub struct KlatosKebijakanPanggilanReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub adm_checks: u32,
    pub adm_passed: u32,
    pub jalur_checks: u32,
    pub jalur_passed: u32,
    pub crash_checks: u32,
    pub crash_passed: u32,
    pub prod_admisi: u32,
    pub prod_jalur: u32,
    pub prod_crash: u32,
    pub prod_tolak: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub n_verdict: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Admisi,
    Jalur,
    Crash,
}

struct Matrix {
    checks: u32,
    passed: u32,
    adm_checks: u32,
    adm_passed: u32,
    jalur_checks: u32,
    jalur_passed: u32,
    crash_checks: u32,
    crash_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            adm_checks: 0,
            adm_passed: 0,
            jalur_checks: 0,
            jalur_passed: 0,
            crash_checks: 0,
            crash_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Admisi => {
                self.adm_checks += 1;
                if lulus {
                    self.adm_passed += 1;
                }
            }
            Group::Jalur => {
                self.jalur_checks += 1;
                if lulus {
                    self.jalur_passed += 1;
                }
            }
            Group::Crash => {
                self.crash_checks += 1;
                if lulus {
                    self.crash_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan oleh `run_matrix`.
pub const DIFF_EXPECTED: u32 = 10300;

/// Nilai ABI sintetis untuk matriks (nilai NYATA diambil dari C++ saat
/// produksi; matriks memakai nilai tetap agar deterministik).
const N_FUNGSI_SINTETIS: u32 = 128;
const SC_EXIT_SINTETIS: u64 = 26;
const SC_EXIT_THREAD_SINTETIS: u64 = 27;
const SC_FORK_SINTETIS: u64 = 33;
const SC_SIGRETURN_SINTETIS: u64 = 133;
const EPELANGGARAN_SINTETIS: i32 = 60;

fn run_matrix() -> KlatosKebijakanPanggilanReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5359_5350, Group::Admisi);
    m.catat(ADMISI_LOLOS == 0, Group::Admisi);
    m.catat(ADMISI_MATI == 1, Group::Admisi);
    m.catat(ADMISI_TAK_DIKENAL == 2, Group::Admisi);
    m.catat(ADMISI_HANDLER_NOL == 3, Group::Admisi);
    m.catat(N_VERDICT == 4, Group::Admisi);
    m.catat(JALUR_BIASA == 0, Group::Jalur);
    m.catat(JALUR_REGSTATE == 1, Group::Jalur);
    m.catat(JALUR_TIDAK_KEMBALI == 2, Group::Jalur);
    m.catat(N_JALUR == 3, Group::Jalur);
    m.catat(CRASH_TIDAK == 0, Group::Crash);
    m.catat(CRASH_YA == 1, Group::Crash);

    // --- P1.101: konstanta rencana ---------------------------------------
    m.catat(AKSI_TOLAK == 0, Group::Jalur);
    m.catat(AKSI_KEMBALI == 1, Group::Jalur);
    m.catat(AKSI_REGSTATE == 2, Group::Jalur);
    m.catat(AKSI_BIASA == 3, Group::Jalur);
    m.catat(N_AKSI == 4, Group::Jalur);
    m.catat(PESAN_NONE == 0, Group::Jalur);
    m.catat(PESAN_TAK_DIKENAL == 1, Group::Jalur);
    m.catat(PESAN_HANDLER_NOL == 2, Group::Jalur);
    m.catat(N_PESAN == 3, Group::Jalur);
    m.catat(core::mem::size_of::<KlatosRencanaDispatch>() == 16, Group::Jalur);
    m.catat(core::mem::align_of::<KlatosRencanaDispatch>() == 4, Group::Jalur);

    // --- admisi: presedensi & batas --------------------------------------
    // mati MENANG atas segalanya.
    m.catat(admisi(1, 0, N_FUNGSI_SINTETIS, 0) == ADMISI_MATI, Group::Admisi);
    m.catat(admisi(1, 9999, N_FUNGSI_SINTETIS, 1) == ADMISI_MATI, Group::Admisi);
    // di luar __Count MENANG atas handler nol.
    m.catat(admisi(0, 128, N_FUNGSI_SINTETIS, 1) == ADMISI_TAK_DIKENAL, Group::Admisi);
    m.catat(admisi(0, 200, N_FUNGSI_SINTETIS, 1) == ADMISI_TAK_DIKENAL, Group::Admisi);
    // tepat di batas n_fungsi -> TAK_DIKENAL (bukan LOLOS).
    m.catat(admisi(0, 128, 128, 0) == ADMISI_TAK_DIKENAL, Group::Admisi);
    m.catat(admisi(0, 127, 128, 0) == ADMISI_LOLOS, Group::Admisi);
    // handler nol.
    m.catat(admisi(0, 5, N_FUNGSI_SINTETIS, 1) == ADMISI_HANDLER_NOL, Group::Admisi);
    m.catat(admisi(0, 5, N_FUNGSI_SINTETIS, 0) == ADMISI_LOLOS, Group::Admisi);
    // n_fungsi 0 -> semua TAK_DIKENAL.
    m.catat(admisi(0, 0, 0, 0) == ADMISI_TAK_DIKENAL, Group::Admisi);
    // nomor sangat besar TIDAK terpotong (u64).
    m.catat(admisi(0, u64::MAX, N_FUNGSI_SINTETIS, 0) == ADMISI_TAK_DIKENAL, Group::Admisi);

    // --- jalur: presedensi & batas ---------------------------------------
    m.catat(jalur(SC_EXIT_SINTETIS, 26, 27, 33, 133) == JALUR_TIDAK_KEMBALI, Group::Jalur);
    m.catat(jalur(SC_EXIT_THREAD_SINTETIS, 26, 27, 33, 133) == JALUR_TIDAK_KEMBALI, Group::Jalur);
    m.catat(jalur(SC_FORK_SINTETIS, 26, 27, 33, 133) == JALUR_REGSTATE, Group::Jalur);
    m.catat(jalur(SC_SIGRETURN_SINTETIS, 26, 27, 33, 133) == JALUR_REGSTATE, Group::Jalur);
    m.catat(jalur(0, 26, 27, 33, 133) == JALUR_BIASA, Group::Jalur);
    m.catat(jalur(25, 26, 27, 33, 133) == JALUR_BIASA, Group::Jalur);
    m.catat(jalur(28, 26, 27, 33, 133) == JALUR_BIASA, Group::Jalur);
    // exit/exit_thread MENANG bila (secara patologis) nomornya bertabrakan
    // dengan fork/sigreturn: di sini sc_exit == sc_fork == fungsi.
    m.catat(jalur(33, 33, 27, 33, 133) == JALUR_TIDAK_KEMBALI, Group::Jalur);
    m.catat(jalur(33, 26, 27, 33, 133) == JALUR_REGSTATE, Group::Jalur);

    // --- P1.101: rencana — aksi + pesan -----------------------------------
    // Tolak: mati-sistem -> AKSI_TOLAK tanpa pesan.
    {
        let r = rencana(1, 0, N_FUNGSI_SINTETIS, 0, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r.verdict == ADMISI_MATI && r.aksi == AKSI_TOLAK && r.pesan == PESAN_NONE, Group::Jalur);
    }
    // Tolak: di luar __Count -> AKSI_TOLAK + PESAN_TAK_DIKENAL.
    {
        let r = rencana(0, 200, N_FUNGSI_SINTETIS, 1, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r.verdict == ADMISI_TAK_DIKENAL && r.aksi == AKSI_TOLAK && r.pesan == PESAN_TAK_DIKENAL, Group::Jalur);
    }
    // Tolak: handler nol -> AKSI_TOLAK + PESAN_HANDLER_NOL.
    {
        let r = rencana(0, 5, N_FUNGSI_SINTETIS, 1, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r.verdict == ADMISI_HANDLER_NOL && r.aksi == AKSI_TOLAK && r.pesan == PESAN_HANDLER_NOL, Group::Jalur);
    }
    // Terima: exit -> AKSI_KEMBALI; exit_thread -> AKSI_KEMBALI.
    {
        let r = rencana(0, SC_EXIT_SINTETIS, N_FUNGSI_SINTETIS, 0, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r.verdict == ADMISI_LOLOS && r.jalur == JALUR_TIDAK_KEMBALI && r.aksi == AKSI_KEMBALI, Group::Jalur);
        let r2 = rencana(0, SC_EXIT_THREAD_SINTETIS, N_FUNGSI_SINTETIS, 0, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r2.aksi == AKSI_KEMBALI, Group::Jalur);
    }
    // Terima: fork -> AKSI_REGSTATE; sigreturn -> AKSI_REGSTATE.
    // CATATAN: `SC_SIGRETURN_SINTETIS` (133) > `N_FUNGSI_SINTETIS` (128), jadi
    // lengan sigreturn memakai `n_fungsi` sintetis 256 supaya nomornya MASUK
    // rentang — kalau tidak, admisi benar-benar menolaknya sebagai TAK_DIKENAL.
    {
        let r = rencana(0, SC_FORK_SINTETIS, N_FUNGSI_SINTETIS, 0, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r.jalur == JALUR_REGSTATE && r.aksi == AKSI_REGSTATE, Group::Jalur);
        let r2 = rencana(0, SC_SIGRETURN_SINTETIS, 256, 0, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r2.jalur == JALUR_REGSTATE && r2.aksi == AKSI_REGSTATE, Group::Jalur);
    }
    // Terima: biasa -> AKSI_BIASA, tanpa pesan.
    {
        let r = rencana(0, 0, N_FUNGSI_SINTETIS, 0, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r.jalur == JALUR_BIASA && r.aksi == AKSI_BIASA && r.pesan == PESAN_NONE, Group::Jalur);
    }
    // Presedensi: mati MENANG atas di-luar-rentang -> pesan NONE (bukan
    // TAK_DIKENAL). Bila urutan salah, pesannya akan TAK_DIKENAL.
    {
        let r = rencana(1, 9999, N_FUNGSI_SINTETIS, 1, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        m.catat(r.verdict == ADMISI_MATI && r.pesan == PESAN_NONE, Group::Jalur);
    }
    // Kontrol NEGATIF: presedensi jalur berdaya. `sc_exit == sc_fork == fungsi`
    // -> urutan benar memberi AKSI_KEMBALI, urutan terbalik AKSI_REGSTATE.
    {
        let benar = rencana(0, 33, N_FUNGSI_SINTETIS, 0, 33, 27, 33, 133);
        let salah = rencana_salah_urutan(0, 33, N_FUNGSI_SINTETIS, 0, 33, 27, 33, 133);
        m.catat(benar.aksi == AKSI_KEMBALI, Group::Jalur);
        m.catat(salah.aksi == AKSI_REGSTATE, Group::Jalur);
        m.catat(benar.aksi != salah.aksi, Group::Jalur);
    }

    // --- crash ------------------------------------------------------------
    m.catat(harus_crash(1, EPELANGGARAN_SINTETIS, EPELANGGARAN_SINTETIS) == CRASH_YA, Group::Crash);
    m.catat(harus_crash(1, 0, EPELANGGARAN_SINTETIS) == CRASH_TIDAK, Group::Crash);
    m.catat(harus_crash(1, 22, EPELANGGARAN_SINTETIS) == CRASH_TIDAK, Group::Crash);
    m.catat(harus_crash(0, EPELANGGARAN_SINTETIS, EPELANGGARAN_SINTETIS) == CRASH_TIDAK, Group::Crash);
    m.catat(harus_crash(0, 0, EPELANGGARAN_SINTETIS) == CRASH_TIDAK, Group::Crash);

    // --- kontrol NEGATIF: presedensi admisi berdaya ----------------------
    // Kasus di mana salah-urutan memberi jawaban BERBEDA (handler_nol=1 dan
    // fungsi di luar rentang): benar = TAK_DIKENAL, salah = HANDLER_NOL.
    {
        let benar = admisi(0, 200, N_FUNGSI_SINTETIS, 1);
        let salah = admisi_salah_urutan(0, 200, N_FUNGSI_SINTETIS, 1);
        m.catat(benar == ADMISI_TAK_DIKENAL, Group::Admisi);
        m.catat(salah == ADMISI_HANDLER_NOL, Group::Admisi);
        m.catat(benar != salah, Group::Admisi);
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // admisi: 2 x 300 x 4 x 2 = 4800
    for mati in 0..2u32 {
        for fungsi in 0..300u64 {
            for &nf in &[0u32, 128, 200, 256] {
                for hn in 0..2u32 {
                    let r = admisi(mati, fungsi, nf, hn);
                    let o = naif_admisi(mati, fungsi, nf, hn);
                    if r != o {
                        beda += 1;
                    }
                    diukur += 1;
                }
            }
        }
    }
    // jalur: 300
    for fungsi in 0..300u64 {
        let r = jalur(fungsi, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        let o = naif_jalur(fungsi, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
        if r != o {
            beda += 1;
        }
        diukur += 1;
    }
    // crash: 2 x 200 = 400
    for ae in 0..2u32 {
        for kode in 0..200i32 {
            let r = harus_crash(ae, kode, EPELANGGARAN_SINTETIS);
            let o = naif_harus_crash(ae, kode, EPELANGGARAN_SINTETIS);
            if r != o {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // rencana (P1.101): 2 x 300 x 4 x 2 = 4800
    for mati in 0..2u32 {
        for fungsi in 0..300u64 {
            for &nf in &[0u32, 128, 200, 256] {
                for hn in 0..2u32 {
                    let r = rencana(mati, fungsi, nf, hn, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
                    let o = naif_rencana(mati, fungsi, nf, hn, SC_EXIT_SINTETIS, SC_EXIT_THREAD_SINTETIS, SC_FORK_SINTETIS, SC_SIGRETURN_SINTETIS);
                    if r != o {
                        beda += 1;
                    }
                    diukur += 1;
                }
            }
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Admisi);
    m.catat(beda == 0, Group::Admisi);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosKebijakanPanggilanReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        adm_checks: m.adm_checks,
        adm_passed: m.adm_passed,
        jalur_checks: m.jalur_checks,
        jalur_passed: m.jalur_passed,
        crash_checks: m.crash_checks,
        crash_passed: m.crash_passed,
        prod_admisi: klatos_kebijakan_counter(Counter::ProdAdmisi as u32),
        prod_jalur: klatos_kebijakan_counter(Counter::ProdJalur as u32),
        prod_crash: klatos_kebijakan_counter(Counter::ProdCrash as u32),
        prod_tolak: klatos_kebijakan_counter(Counter::ProdTolak as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        n_verdict: N_VERDICT,
        status,
    }
}

/// Menjalankan matriks kebijakan dispatch panggilan sistem.
#[no_mangle]
pub extern "C" fn klatos_kebijakan_selftest() -> KlatosKebijakanPanggilanReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5359_5350);
        assert!(ADMISI_LOLOS == 0);
        assert!(ADMISI_MATI == 1);
        assert!(ADMISI_TAK_DIKENAL == 2);
        assert!(ADMISI_HANDLER_NOL == 3);
        assert!(N_VERDICT == 4);
        assert!(JALUR_BIASA == 0);
        assert!(JALUR_REGSTATE == 1);
        assert!(JALUR_TIDAK_KEMBALI == 2);
        assert!(N_JALUR == 3);
        assert!(CRASH_TIDAK == 0);
        assert!(CRASH_YA == 1);
        assert!(AKSI_TOLAK == 0);
        assert!(AKSI_KEMBALI == 1);
        assert!(AKSI_REGSTATE == 2);
        assert!(AKSI_BIASA == 3);
        assert!(N_AKSI == 4);
        assert!(PESAN_NONE == 0);
        assert!(PESAN_TAK_DIKENAL == 1);
        assert!(PESAN_HANDLER_NOL == 2);
        assert!(N_PESAN == 3);
        assert!(Counter::ProdAdmisi as usize == 0);
        assert!(Counter::ProdJalur as usize == 1);
        assert!(Counter::ProdCrash as usize == 2);
        assert!(Counter::ProdTolak as usize == 3);
        assert!(N_COUNTER == 4);
        assert!(DIFF_EXPECTED == 10300);

        // 8 (tag) + 15*4 (u32) + 4 (status) = 72, sudah kelipatan align 8.
        assert!(core::mem::size_of::<KlatosKebijakanPanggilanReport>() == 72);
        assert!(core::mem::align_of::<KlatosKebijakanPanggilanReport>() == 8);
        // P1.101 — rencana: 4 u32 = 16 byte, align 4.
        assert!(core::mem::size_of::<KlatosRencanaDispatch>() == 16);
        assert!(core::mem::align_of::<KlatosRencanaDispatch>() == 4);
    };
}
