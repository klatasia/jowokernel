// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas **buku besar siklus hidup benang (thread)** Jowo (P1.45).


use core::sync::atomic::{AtomicU32, Ordering};

/// Jumlah nilai `Thread::State` (`KlatKernel/Efek/Eksekusi/Thread.h:115`):
/// Invalid=0, Runnable=1, Running=2, Dying=3, Dead=4, Stopped=5, Blocked=6.
pub const N_STATE: u32 = 7;

/// Tag laporan; cermin `KlatosBukuBesarBenangReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5448_524C; // "KLATTHRL"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_benang_catat_dibuat`] dari produksi.
    ProdDibuat = 0,
    /// Jumlah total pemanggilan [`klatos_benang_catat_dimusan`] dari produksi.
    ProdDimusan = 1,
    /// Jumlah total pemanggilan [`klatos_benang_catat_transisi`] dari produksi.
    ProdTransisi = 2,
    /// Jumlah transisi yang DITOLAK karena `dari == ke` (ilegal).
    ProdIlegal = 3,
    /// Jumlah nilai keadaan berbeda yang pernah dimasuki (0..=N_STATE).
    ProdStateAktif = 4,
}

const N_COUNTER: usize = 5;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Buku besar utama
// ---------------------------------------------------------------------------

/// Total benang yang pernah dibuat.
static DIBUAT: AtomicU32 = AtomicU32::new(0);

/// Total benang yang sudah dimusnahkan (`~Thread`).
static DIMUSAN: AtomicU32 = AtomicU32::new(0);

/// Benang yang hidup sekarang (`DIBUAT - DIMUSAN`, dijaga inkremental).
static HIDUP: AtomicU32 = AtomicU32::new(0);

/// Puncak benang hidup yang pernah teramati.
static PUNCAK: AtomicU32 = AtomicU32::new(0);

/// Total transisi keadaan yang DITERIMA.
static TRANSISI: AtomicU32 = AtomicU32::new(0);

/// Total transisi keadaan yang DITOLAK (`dari == ke`).
static ILEGAL: AtomicU32 = AtomicU32::new(0);

/// Berapa kali tiap keadaan DIMASUKI (indeks = nilai `Thread::State`).
static MASUK: [AtomicU32; N_STATE as usize] = [const { AtomicU32::new(0) }; N_STATE as usize];

/// Berapa kali tiap keadaan DITINGGALKAN.
static KELUAR: [AtomicU32; N_STATE as usize] = [const { AtomicU32::new(0) }; N_STATE as usize];

/// Total seluruh [`MASUK`] (agregat).
static TOTAL_MASUK: AtomicU32 = AtomicU32::new(0);

/// Total seluruh [`KELUAR`] (agregat).
static TOTAL_KELUAR: AtomicU32 = AtomicU32::new(0);

/// Mencatat kelahiran satu benang. Mengembalikan jumlah hidup setelahnya.
fn catat_dibuat_impl() -> u32 {
    bump(Counter::ProdDibuat);
    DIBUAT.fetch_add(1, Ordering::Relaxed);
    let hidup = HIDUP.fetch_add(1, Ordering::Relaxed) + 1;
    // Puncak diperbarui dengan CAS-loop agar tetap benar di bawah concurrency.
    let mut puncak = PUNCAK.load(Ordering::Relaxed);
    while hidup > puncak {
        match PUNCAK.compare_exchange_weak(puncak, hidup, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(now) => puncak = now,
        }
    }
    hidup
}

/// Mencatat pemusnahan satu benang. Mengembalikan jumlah hidup setelahnya.
fn catat_dimusan_impl() -> u32 {
    bump(Counter::ProdDimusan);
    DIMUSAN.fetch_add(1, Ordering::Relaxed);
    // `fetch_sub` jenuh pada 0: hanya kurangi bila masih ada yang hidup.
    let mut hidup = HIDUP.load(Ordering::Relaxed);
    loop {
        if hidup == 0 {
            return 0;
        }
        match HIDUP.compare_exchange_weak(hidup, hidup - 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return hidup - 1,
            Err(now) => hidup = now,
        }
    }
}

/// Mencatat transisi keadaan `dari` -> `ke`. Mengembalikan 1 bila diterima,
/// 0 bila ditolak (`dari == ke`, atau salah satu di luar 0..N_STATE).
fn catat_transisi_impl(dari: u32, ke: u32) -> u32 {
    bump(Counter::ProdTransisi);
    if dari >= N_STATE || ke >= N_STATE {
        bump(Counter::ProdIlegal);
        ILEGAL.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    if dari == ke {
        bump(Counter::ProdIlegal);
        ILEGAL.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    // Keluar dari keadaan lama.
    let prev_keluar = KELUAR[dari as usize].fetch_add(1, Ordering::Relaxed);
    let _ = prev_keluar;
    TOTAL_KELUAR.fetch_add(1, Ordering::Relaxed);
    // Masuk ke keadaan baru.
    let prev_masuk = MASUK[ke as usize].fetch_add(1, Ordering::Relaxed);
    if prev_masuk == 0 {
        bump(Counter::ProdStateAktif);
    }
    TOTAL_MASUK.fetch_add(1, Ordering::Relaxed);
    TRANSISI.fetch_add(1, Ordering::Relaxed);
    1
}

/// Reset seluruh buku besar ke 0. TIDAK dipanggil dari produksi.
fn reset_impl() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    for s in MASUK.iter() {
        s.store(0, Ordering::Release);
    }
    for s in KELUAR.iter() {
        s.store(0, Ordering::Release);
    }
    DIBUAT.store(0, Ordering::Release);
    DIMUSAN.store(0, Ordering::Release);
    HIDUP.store(0, Ordering::Release);
    PUNCAK.store(0, Ordering::Release);
    TRANSISI.store(0, Ordering::Release);
    ILEGAL.store(0, Ordering::Release);
    TOTAL_MASUK.store(0, Ordering::Release);
    TOTAL_KELUAR.store(0, Ordering::Release);
}

// ---------------------------------------------------------------------------
// Algoritma — oracle pembanding deterministik
// ---------------------------------------------------------------------------

/// Oracle: salinan deterministik buku besar dalam bentuk "naif" (array Rust
/// biasa). Dipakai matriks untuk pembanding.
struct NaifBukuBesar {
    dibuat: u32,
    dimusan: u32,
    hidup: u32,
    puncak: u32,
    transisi: u32,
    ilegal: u32,
    masuk: [u32; N_STATE as usize],
    keluar: [u32; N_STATE as usize],
}

impl NaifBukuBesar {
    const fn new() -> Self {
        NaifBukuBesar {
            dibuat: 0,
            dimusan: 0,
            hidup: 0,
            puncak: 0,
            transisi: 0,
            ilegal: 0,
            masuk: [0u32; N_STATE as usize],
            keluar: [0u32; N_STATE as usize],
        }
    }
    fn catat_dibuat(&mut self) -> u32 {
        self.dibuat += 1;
        self.hidup += 1;
        if self.hidup > self.puncak {
            self.puncak = self.hidup;
        }
        self.hidup
    }
    fn catat_dimusan(&mut self) -> u32 {
        self.dimusan += 1;
        if self.hidup > 0 {
            self.hidup -= 1;
        }
        self.hidup
    }
    fn catat_transisi(&mut self, dari: u32, ke: u32) -> u32 {
        if dari >= N_STATE || ke >= N_STATE || dari == ke {
            self.ilegal += 1;
            return 0;
        }
        self.keluar[dari as usize] += 1;
        self.masuk[ke as usize] += 1;
        self.transisi += 1;
        1
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mencatat kelahiran satu benang (dipanggil dari `Thread::Thread`).
#[no_mangle]
pub extern "C" fn klatos_benang_catat_dibuat() -> u32 {
    catat_dibuat_impl()
}

/// Mencatat pemusnahan satu benang (dipanggil dari `Thread::~Thread`).
#[no_mangle]
pub extern "C" fn klatos_benang_catat_dimusan() -> u32 {
    catat_dimusan_impl()
}

/// Mencatat transisi keadaan `dari` -> `ke` (dipanggil dari `Thread::set_state`).
/// Mengembalikan 1 bila diterima, 0 bila ditolak.
#[no_mangle]
pub extern "C" fn klatos_benang_catat_transisi(dari: u32, ke: u32) -> u32 {
    catat_transisi_impl(dari, ke)
}

/// Total benang yang pernah dibuat.
#[no_mangle]
pub extern "C" fn klatos_benang_dibuat() -> u32 {
    DIBUAT.load(Ordering::Relaxed)
}

/// Total benang yang sudah dimusnahkan.
#[no_mangle]
pub extern "C" fn klatos_benang_dimusan() -> u32 {
    DIMUSAN.load(Ordering::Relaxed)
}

/// Benang yang hidup sekarang.
#[no_mangle]
pub extern "C" fn klatos_benang_hidup() -> u32 {
    HIDUP.load(Ordering::Relaxed)
}

/// Puncak benang hidup yang pernah teramati.
#[no_mangle]
pub extern "C" fn klatos_benang_puncak() -> u32 {
    PUNCAK.load(Ordering::Relaxed)
}

/// Total transisi keadaan yang diterima.
#[no_mangle]
pub extern "C" fn klatos_benang_transisi_total() -> u32 {
    TRANSISI.load(Ordering::Relaxed)
}

/// Total transisi keadaan yang ditolak.
#[no_mangle]
pub extern "C" fn klatos_benang_transisi_ilegal() -> u32 {
    ILEGAL.load(Ordering::Relaxed)
}

/// Berapa kali keadaan `state` dimasuki. 0 bila di luar 0..N_STATE.
#[no_mangle]
pub extern "C" fn klatos_benang_masuk(state: u32) -> u32 {
    if state >= N_STATE {
        return 0;
    }
    MASUK[state as usize].load(Ordering::Relaxed)
}

/// Berapa kali keadaan `state` ditinggalkan. 0 bila di luar 0..N_STATE.
#[no_mangle]
pub extern "C" fn klatos_benang_keluar(state: u32) -> u32 {
    if state >= N_STATE {
        return 0;
    }
    KELUAR[state as usize].load(Ordering::Relaxed)
}

/// Total seluruh `MASUK` (agregat).
#[no_mangle]
pub extern "C" fn klatos_benang_total_masuk() -> u32 {
    TOTAL_MASUK.load(Ordering::Relaxed)
}

/// Total seluruh `KELUAR` (agregat).
#[no_mangle]
pub extern "C" fn klatos_benang_total_keluar() -> u32 {
    TOTAL_KELUAR.load(Ordering::Relaxed)
}

/// Konstanta `N_STATE`.
#[no_mangle]
pub extern "C" fn klatos_benang_n_state() -> u32 {
    N_STATE
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_benang_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset ke 0.
#[no_mangle]
pub extern "C" fn klatos_benang_reset() -> u32 {
    reset_impl();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosBukuBesarBenangReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosBukuBesarBenangReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_dibuat: u32,
    pub prod_dimusan: u32,
    pub prod_transisi: u32,
    pub prod_ilegal: u32,
    pub prod_state_aktif: u32,
    pub n_state: u32,
    pub hidup: u32,
    pub puncak: u32,
    pub total_masuk: u32,
    pub total_keluar: u32,
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

/// Matriks buku besar benang. Input SINTETIS.
fn run_matrix() -> KlatosBukuBesarBenangReport {
    let mut m = Matrix::new();

    // --- kelompok 0: konstanta -------------------------------------------
    m.catat(N_STATE == 7, Group::Peta);
    m.catat(REPORT_TAG == 0x4B4C_4154_5448_524C, Group::Peta);
    // Baca di luar jangkauan -> 0.
    m.catat(klatos_benang_masuk(N_STATE) == 0, Group::Peta);
    m.catat(klatos_benang_keluar(N_STATE) == 0, Group::Peta);
    m.catat(klatos_benang_masuk(0xFFFFFFFF) == 0, Group::Peta);
    m.catat(klatos_benang_keluar(0xFFFFFFFF) == 0, Group::Peta);

    // --- kelompok 1: oracle vs produksi untuk urutan deterministik --------
    reset_impl();
    let mut oracle = NaifBukuBesar::new();

    // 64 kelahiran, dengan puncak yang diketahui.
    for i in 0..64u32 {
        let r = catat_dibuat_impl();
        let o = oracle.catat_dibuat();
        m.catat(r == o, Group::Peta);
        let _ = i;
    }
    m.catat(klatos_benang_dibuat() == 64, Group::Peta);
    m.catat(klatos_benang_hidup() == 64, Group::Peta);
    m.catat(klatos_benang_puncak() == 64, Group::Peta);

    // 24 pemusnahan.
    for _ in 0..24u32 {
        let r = catat_dimusan_impl();
        let o = oracle.catat_dimusan();
        m.catat(r == o, Group::Peta);
    }
    m.catat(klatos_benang_dibuat() == 64, Group::Peta);
    m.catat(klatos_benang_dimusan() == 24, Group::Peta);
    m.catat(klatos_benang_hidup() == 40, Group::Peta);
    // Puncak tetap 64 walau hidup turun.
    m.catat(klatos_benang_puncak() == 64, Group::Peta);

    // 200 transisi deterministik (dari != ke) — siklus keadaan.
    for i in 0..200u32 {
        let dari = i % N_STATE;
        let ke = (i + 1) % N_STATE;
        let r = catat_transisi_impl(dari, ke);
        let o = oracle.catat_transisi(dari, ke);
        m.catat(r == o && r == 1, Group::Peta);
    }
    // 50 transisi ilegal (dari == ke).
    for i in 0..50u32 {
        let s = i % N_STATE;
        let r = catat_transisi_impl(s, s);
        let o = oracle.catat_transisi(s, s);
        m.catat(r == o && r == 0, Group::Peta);
    }
    // 20 transisi di luar jangkauan.
    for i in 0..20u32 {
        let r = catat_transisi_impl(N_STATE, i);
        let o = oracle.catat_transisi(N_STATE, i);
        m.catat(r == o && r == 0, Group::Peta);
    }

    // Bandingkan produksi vs oracle per-keadaan.
    for s in 0..N_STATE {
        m.catat(
            klatos_benang_masuk(s) == oracle.masuk[s as usize],
            Group::Peta,
        );
        m.catat(
            klatos_benang_keluar(s) == oracle.keluar[s as usize],
            Group::Peta,
        );
    }
    m.catat(klatos_benang_transisi_total() == oracle.transisi, Group::Peta);
    m.catat(klatos_benang_transisi_ilegal() == oracle.ilegal, Group::Peta);

    // Agregat = jumlah seluruh MASUK / KELUAR.
    let mut agg_masuk: u32 = 0;
    for &v in oracle.masuk.iter() {
        agg_masuk += v;
    }
    let mut agg_keluar: u32 = 0;
    for &v in oracle.keluar.iter() {
        agg_keluar += v;
    }
    m.catat(klatos_benang_total_masuk() == agg_masuk, Group::Peta);
    m.catat(klatos_benang_total_keluar() == agg_keluar, Group::Peta);
    // Setiap transisi menambah tepat satu MASUK dan satu KELUAR.
    m.catat(klatos_benang_total_masuk() == klatos_benang_transisi_total(), Group::Peta);
    m.catat(klatos_benang_total_keluar() == klatos_benang_transisi_total(), Group::Peta);

    // --- kelompok 2: determinisme (baca berulang jawaban sama) ------------
    {
        let prev_dibuat = klatos_benang_dibuat();
        let prev_hidup = klatos_benang_hidup();
        let prev_transisi = klatos_benang_transisi_total();
        let mut ok = true;
        for _ in 0..10_000 {
            if klatos_benang_dibuat() != prev_dibuat
                || klatos_benang_hidup() != prev_hidup
                || klatos_benang_transisi_total() != prev_transisi
            {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Peta);
    }

    // --- kelompok 3: negatif ----------------------------------------------
    // reset -> 0.
    reset_impl();
    m.catat(klatos_benang_dibuat() == 0, Group::Neg);
    m.catat(klatos_benang_dimusan() == 0, Group::Neg);
    m.catat(klatos_benang_hidup() == 0, Group::Neg);
    m.catat(klatos_benang_puncak() == 0, Group::Neg);
    m.catat(klatos_benang_transisi_total() == 0, Group::Neg);
    m.catat(klatos_benang_transisi_ilegal() == 0, Group::Neg);
    m.catat(klatos_benang_total_masuk() == 0, Group::Neg);
    m.catat(klatos_benang_total_keluar() == 0, Group::Neg);
    for s in 0..N_STATE {
        m.catat(klatos_benang_masuk(s) == 0, Group::Neg);
        m.catat(klatos_benang_keluar(s) == 0, Group::Neg);
    }
    // `dimusan` saat hidup 0 tidak boleh membuat underflow.
    m.catat(catat_dimusan_impl() == 0, Group::Neg);
    m.catat(klatos_benang_hidup() == 0, Group::Neg);
    // Transisi ilegal menaikkan penghitung ilegal, bukan transisi.
    let _ = catat_transisi_impl(3, 3);
    m.catat(klatos_benang_transisi_ilegal() == 1, Group::Neg);
    m.catat(klatos_benang_transisi_total() == 0, Group::Neg);
    // Penghitung audit di luar jangkauan -> 0.
    m.catat(klatos_benang_counter(99) == 0, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosBukuBesarBenangReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_dibuat: klatos_benang_counter(Counter::ProdDibuat as u32),
        prod_dimusan: klatos_benang_counter(Counter::ProdDimusan as u32),
        prod_transisi: klatos_benang_counter(Counter::ProdTransisi as u32),
        prod_ilegal: klatos_benang_counter(Counter::ProdIlegal as u32),
        prod_state_aktif: klatos_benang_counter(Counter::ProdStateAktif as u32),
        n_state: N_STATE,
        hidup: klatos_benang_hidup(),
        puncak: klatos_benang_puncak(),
        total_masuk: klatos_benang_total_masuk(),
        total_keluar: klatos_benang_total_keluar(),
        status,
    }
}

/// Menjalankan matriks buku besar benang.
#[no_mangle]
pub extern "C" fn klatos_benang_selftest() -> KlatosBukuBesarBenangReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(N_STATE == 7);
        assert!(REPORT_TAG == 0x4B4C_4154_5448_524C);
        assert!(Counter::ProdDibuat as usize == 0);
        assert!(Counter::ProdDimusan as usize == 1);
        assert!(Counter::ProdTransisi as usize == 2);
        assert!(Counter::ProdIlegal as usize == 3);
        assert!(Counter::ProdStateAktif as usize == 4);
        assert!(N_COUNTER == 5);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76, dibulatkan ke kelipatan
        // align 8 -> 80.
        assert!(core::mem::size_of::<KlatosBukuBesarBenangReport>() == 80);
        assert!(core::mem::align_of::<KlatosBukuBesarBenangReport>() == 8);
    };
}
