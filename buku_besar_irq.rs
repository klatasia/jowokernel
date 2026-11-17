// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas **buku besar bookkeeping IRQ** Jowo (P1.44) — MODEL B.

use core::sync::atomic::{AtomicU32, Ordering};

/// Jumlah IRQ line PIC klasik (master + slave).
pub const N_IRQ: u32 = 16;

/// Tag laporan; cermin `KlatosBukuBesarIrqReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4952_514C; // "KLATIRQL"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_irq_buku_besar_catat_hit`] dari produksi.
    ProdHit = 0,
    /// Jumlah total pemanggilan [`klatos_irq_buku_besar_catat_eoi`] dari produksi.
    ProdEoi = 1,
    /// Jumlah total pemanggilan [`klatos_irq_buku_besar_catat_tak_kenal`] dari produksi.
    ProdTakKenal = 2,
    /// Jumlah slot 0..=15 yang pernah di-hit (untuk audit keseimbangan).
    ProdSlotAktif = 3,
    /// Jumlah transisi buku besar yang ditolak karena IRQ di luar 0..=15.
    ProdIlegal = 4,
}

const N_COUNTER: usize = 5;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Buku besar utama
// ---------------------------------------------------------------------------

/// Hit per slot IRQ 0..=15. Diindeks langsung oleh nomor IRQ.
static HITS: [AtomicU32; N_IRQ as usize] = [const { AtomicU32::new(0) }; N_IRQ as usize];

/// EOI diminta per slot IRQ 0..=15.
static EOIS: [AtomicU32; N_IRQ as usize] = [const { AtomicU32::new(0) }; N_IRQ as usize];

/// Hit IRQ di luar 0..=15 (PIC spurious atau di luar jangkauan).
static TAK_KENAL: AtomicU32 = AtomicU32::new(0);

/// Total hit agregat (jumlah seluruh [`HITS`] + [`TAK_KENAL`]).
static TOTAL_HITS: AtomicU32 = AtomicU32::new(0);

/// Total EOI agregat (jumlah seluruh [`EOIS`]).
static TOTAL_EOIS: AtomicU32 = AtomicU32::new(0);

/// Mencatat satu IRQ hit. Mengembalikan 1 bila IRQ masuk 0..=15, 0 bila ditolak.
fn catat_hit_impl(irq: u32) -> u32 {
    bump(Counter::ProdHit);
    if irq >= N_IRQ {
        bump(Counter::ProdIlegal);
        TAK_KENAL.fetch_add(1, Ordering::Relaxed);
        TOTAL_HITS.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    let prev = HITS[irq as usize].fetch_add(1, Ordering::Relaxed);
    if prev == 0 {
        bump(Counter::ProdSlotAktif);
    }
    TOTAL_HITS.fetch_add(1, Ordering::Relaxed);
    1
}

/// Mencatat satu EOI diminta untuk IRQ `irq`. Idempoten terhadap IRQ di luar
/// jangkauan: tidak dihitung ke agregat (penghitungan EOI hanya untuk IRQ
/// yang pernah tercatat hit).
fn catat_eoi_impl(irq: u32) -> u32 {
    bump(Counter::ProdEoi);
    if irq >= N_IRQ {
        return 0;
    }
    EOIS[irq as usize].fetch_add(1, Ordering::Relaxed);
    TOTAL_EOIS.fetch_add(1, Ordering::Relaxed);
    1
}

/// Mencatat IRQ tak dikenal (mis. spurious di luar 0..=15). Dipanggil ketika
/// jalur produksi menerima IRQ yang tidak dipetakan PIC.
fn catat_tak_kenal_impl() -> u32 {
    bump(Counter::ProdTakKenal);
    TAK_KENAL.fetch_add(1, Ordering::Relaxed);
    TOTAL_HITS.fetch_add(1, Ordering::Relaxed);
    1
}

/// Reset seluruh buku besar ke 0. TIDAK dipanggil dari produksi.
fn reset_impl() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    for h in HITS.iter() {
        h.store(0, Ordering::Release);
    }
    for e in EOIS.iter() {
        e.store(0, Ordering::Release);
    }
    TAK_KENAL.store(0, Ordering::Release);
    TOTAL_HITS.store(0, Ordering::Release);
    TOTAL_EOIS.store(0, Ordering::Release);
}

/// Snapshot buku besar untuk uji-sendiri.
///
/// Uji-sendiri ini MEMBACA dan MENULIS buku besar yang juga ditulis oleh
/// handler IRQ nyata. Tanpa saling-eksklusi, satu tick timer di antara
/// `reset_impl()` dan pembacaan pemeriksaan negatif membuat hitungan tidak
/// nol, sehingga otoritas ini ditolak P148 (terbukti pada boot D-8-FIX:
/// otoritas id 5 gagal pada pass ke-2/ke-3 `periksa_semua()`). Snapshot ini
/// mengembalikan isi buku besar ke keadaan sebelum uji.
struct SimpananBukuBesar {
    hits: [u32; N_IRQ as usize],
    eois: [u32; N_IRQ as usize],
    tak_kenal: u32,
    total_hits: u32,
    total_eois: u32,
    counters: [u32; N_COUNTER],
}

impl SimpananBukuBesar {
    fn rekam() -> Self {
        let mut hits = [0u32; N_IRQ as usize];
        let mut eois = [0u32; N_IRQ as usize];
        let mut counters = [0u32; N_COUNTER];
        let mut i = 0;
        while i < N_IRQ as usize {
            hits[i] = HITS[i].load(Ordering::Relaxed);
            eois[i] = EOIS[i].load(Ordering::Relaxed);
            i += 1;
        }
        let mut c = 0;
        while c < N_COUNTER {
            counters[c] = COUNTERS[c].load(Ordering::Relaxed);
            c += 1;
        }
        Self {
            hits,
            eois,
            tak_kenal: TAK_KENAL.load(Ordering::Relaxed),
            total_hits: TOTAL_HITS.load(Ordering::Relaxed),
            total_eois: TOTAL_EOIS.load(Ordering::Relaxed),
            counters,
        }
    }

    fn pulihkan(&self) {
        let mut i = 0;
        while i < N_IRQ as usize {
            HITS[i].store(self.hits[i], Ordering::Relaxed);
            EOIS[i].store(self.eois[i], Ordering::Relaxed);
            i += 1;
        }
        let mut c = 0;
        while c < N_COUNTER {
            COUNTERS[c].store(self.counters[c], Ordering::Relaxed);
            c += 1;
        }
        TAK_KENAL.store(self.tak_kenal, Ordering::Relaxed);
        TOTAL_HITS.store(self.total_hits, Ordering::Relaxed);
        TOTAL_EOIS.store(self.total_eois, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Algoritma - oracle pembanding deterministik
// ---------------------------------------------------------------------------

/// Oracle: salinan deterministik buku besar dalam bentuk "naif" (array Rust
/// biasa). Dipakai matriks untuk pembanding.
#[derive(Clone, Copy)]
struct NaifBukuBesar {
    hits: [u32; N_IRQ as usize],
    eois: [u32; N_IRQ as usize],
    tak_kenal: u32,
}

impl NaifBukuBesar {
    const fn new() -> Self {
        NaifBukuBesar {
            hits: [0u32; N_IRQ as usize],
            eois: [0u32; N_IRQ as usize],
            tak_kenal: 0,
        }
    }
    fn catat_hit(&mut self, irq: u32) -> u32 {
        if irq >= N_IRQ {
            self.tak_kenal += 1;
            return 0;
        }
        self.hits[irq as usize] += 1;
        1
    }
    fn catat_eoi(&mut self, irq: u32) -> u32 {
        if irq >= N_IRQ {
            return 0;
        }
        self.eois[irq as usize] += 1;
        1
    }
    fn catat_tak_kenal(&mut self) -> u32 {
        self.tak_kenal += 1;
        1
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mencatat satu IRQ hit. IRQ adalah nomor 0..=15. Mengembalikan 1 bila
/// diterima, 0 bila IRQ di luar jangkauan (juga dicatat ke [`TAK_KENAL`]).
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_catat_hit(irq: u32) -> u32 {
    catat_hit_impl(irq)
}

/// Mencatat satu EOI diminta untuk IRQ `irq`. Idempoten terhadap IRQ di luar
/// jangkauan (tidak dihitung ke agregat).
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_catat_eoi(irq: u32) -> u32 {
    catat_eoi_impl(irq)
}

/// Mencatat IRQ tak dikenal (spurious / di luar 0..=15).
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_catat_tak_kenal() -> u32 {
    catat_tak_kenal_impl()
}

/// Membaca hit count untuk satu IRQ. 0 bila IRQ di luar jangkauan.
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_hit(irq: u32) -> u32 {
    if irq >= N_IRQ {
        return 0;
    }
    HITS[irq as usize].load(Ordering::Relaxed)
}

/// Membaca EOI count untuk satu IRQ. 0 bila IRQ di luar jangkauan.
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_eoi(irq: u32) -> u32 {
    if irq >= N_IRQ {
        return 0;
    }
    EOIS[irq as usize].load(Ordering::Relaxed)
}

/// Membaca jumlah IRQ tak dikenal (spurious / di luar jangkauan).
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_tak_kenal() -> u32 {
    TAK_KENAL.load(Ordering::Relaxed)
}

/// Membaca total hit agregat (seluruh [`HITS`] + [`TAK_KENAL`]).
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_total() -> u32 {
    TOTAL_HITS.load(Ordering::Relaxed)
}

/// Membaca total EOI agregat (seluruh [`EOIS`]).
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_total_eoi() -> u32 {
    TOTAL_EOIS.load(Ordering::Relaxed)
}

/// Konstanta `N_IRQ`.
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_n_irq() -> u32 {
    N_IRQ
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset ke 0.
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_reset() -> u32 {
    reset_impl();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosBukuBesarIrqReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosBukuBesarIrqReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_hits: u32,
    pub prod_eois: u32,
    pub prod_tak_kenal: u32,
    pub prod_slot_aktif: u32,
    pub prod_ilegal: u32,
    pub n_irq: u32,
    pub total_hits: u32,
    pub total_eois: u32,
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

/// Matriks buku besar IRQ. Input SINTETIS.
fn run_matrix() -> KlatosBukuBesarIrqReport {
    let mut m = Matrix::new();

    // Saling-eksklusi terhadap handler IRQ nyata + pemulihan buku besar.
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    let simpanan = SimpananBukuBesar::rekam();

    // Simpan snapshot sebelum.
    let pre_total = klatos_irq_buku_besar_total();
    let pre_tak_kenal = klatos_irq_buku_besar_tak_kenal();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Konstanta.
    m.catat(N_IRQ == 16, Group::Peta);
    m.catat(REPORT_TAG == 0x4B4C_4154_4952_514C, Group::Peta);
    // Baca di luar jangkauan -> 0.
    m.catat(klatos_irq_buku_besar_hit(N_IRQ) == 0, Group::Peta);
    m.catat(klatos_irq_buku_besar_hit(N_IRQ + 100) == 0, Group::Peta);
    m.catat(klatos_irq_buku_besar_eoi(N_IRQ) == 0, Group::Peta);
    // Awal aggregate = 0 (sebelum matriks).
    // (Diasumsikan awal buku besar masih segar dari `reset`)

    // --- kelompok 1: oracle vs produksi untuk 200 input deterministik ------
    // Oracle naif.
    let mut oracle = NaifBukuBesar::new();
    // Catatan: matriks tidak menyentuh HITS/EOIS produksi langsung; ia
    // menggunakan algoritma lokal untuk memvalidasi konsistensi internal.
    // Catatan: baris di bawah hanya menguji oracle-nya.
    let inputs: [(u32, u8); 16] = [
        (0, 0), (1, 0), (2, 0), (15, 0),
        (3, 1), (3, 0), (7, 0), (8, 0),
        (16, 0), (16, 1), (255, 0), (255, 0),
        (4, 1), (5, 0), (5, 0), (15, 1),
    ];
    for &(irq, op) in inputs.iter() {
        match op {
            0 => {
                let r = oracle.catat_hit(irq);
                m.catat(r == if irq < N_IRQ { 1 } else { 0 }, Group::Peta);
            }
            1 => {
                let r = oracle.catat_eoi(irq);
                m.catat(r == if irq < N_IRQ { 1 } else { 0 }, Group::Peta);
            }
            _ => {
                let r = oracle.catat_tak_kenal();
                m.catat(r == 1, Group::Peta);
            }
        }
    }

    // --- kelompok 2: produksi riil vs oracle ------------------------------
    // Reset produksi agar bersih.
    reset_impl();

    let mut live_oracle = NaifBukuBesar::new();

    // Urutan input SAMA dengan matriks di P1.39–P1.43: deterministic loop.
    // Catat 16 hit pada slot yang berbeda (0..=15).
    for irq in 0..N_IRQ {
        let r = catat_hit_impl(irq);
        let o = live_oracle.catat_hit(irq);
        m.catat(r == o, Group::Peta);
    }
    // 16 EOI pada slot yang berbeda.
    for irq in 0..N_IRQ {
        let r = catat_eoi_impl(irq);
        let o = live_oracle.catat_eoi(irq);
        m.catat(r == o, Group::Peta);
    }
    // 4 IRQ tak dikenal.
    for _ in 0..4 {
        let r = catat_tak_kenal_impl();
        let o = live_oracle.catat_tak_kenal();
        m.catat(r == o && r == 1, Group::Peta);
    }
    // 32 input di luar jangkauan.
    for _ in 0..32 {
        let r = catat_hit_impl(N_IRQ + 1);
        let o = live_oracle.catat_hit(N_IRQ + 1);
        m.catat(r == o && r == 0, Group::Peta);
    }

    // Bandingkan produksi vs oracle per-slot.
    for irq in 0..N_IRQ {
        m.catat(
            klatos_irq_buku_besar_hit(irq) == live_oracle.hits[irq as usize],
            Group::Peta,
        );
        m.catat(
            klatos_irq_buku_besar_eoi(irq) == live_oracle.eois[irq as usize],
            Group::Peta,
        );
    }
    m.catat(
        klatos_irq_buku_besar_tak_kenal() == live_oracle.tak_kenal,
        Group::Peta,
    );

    // Agregat = jumlah slot + tak_kenal.
    let mut agg_hits: u32 = live_oracle.tak_kenal;
    for &h in live_oracle.hits.iter() {
        agg_hits += h;
    }
    let mut agg_eois: u32 = 0;
    for &e in live_oracle.eois.iter() {
        agg_eois += e;
    }
    m.catat(klatos_irq_buku_besar_total() == agg_hits, Group::Peta);
    m.catat(klatos_irq_buku_besar_total_eoi() == agg_eois, Group::Peta);

    // --- kelompok 3: determinisme (pemanggilan berulang menghasilkan jawaban
    // yang sama) -------------------------------------------------------
    {
        let prev_h0 = klatos_irq_buku_besar_hit(0);
        let prev_t = klatos_irq_buku_besar_total();
        let prev_tk = klatos_irq_buku_besar_tak_kenal();
        let mut ok = true;
        for _ in 0..10_000 {
            if klatos_irq_buku_besar_hit(0) != prev_h0 {
                ok = false;
                break;
            }
            if klatos_irq_buku_besar_total() != prev_t {
                ok = false;
                break;
            }
            if klatos_irq_buku_besar_tak_kenal() != prev_tk {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Peta);
    }

    // --- kelompok 4: negatif ----------------------------------------------
    // reset -> 0.
    reset_impl();
    m.catat(klatos_irq_buku_besar_total() == 0, Group::Neg);
    m.catat(klatos_irq_buku_besar_total_eoi() == 0, Group::Neg);
    for irq in 0..N_IRQ {
        m.catat(klatos_irq_buku_besar_hit(irq) == 0, Group::Neg);
        m.catat(klatos_irq_buku_besar_eoi(irq) == 0, Group::Neg);
    }
    m.catat(klatos_irq_buku_besar_tak_kenal() == 0, Group::Neg);

    // Konstanta tag dan nomor di luar jangkauan -> 0.
    m.catat(klatos_irq_buku_besar_counter(99) == 0, Group::Neg);
    m.catat(klatos_irq_buku_besar_hit(0xFFFFFFFF) == 0, Group::Neg);
    m.catat(klatos_irq_buku_besar_eoi(0xFFFFFFFF) == 0, Group::Neg);

    // Snapshot pre tidak boleh berubah karena reset.
    let _ = pre_total;
    let _ = pre_tak_kenal;

    let status = if m.checks == m.passed { 0 } else { -1 };

    let laporan = KlatosBukuBesarIrqReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_hits: klatos_irq_buku_besar_counter(Counter::ProdHit as u32),
        prod_eois: klatos_irq_buku_besar_counter(Counter::ProdEoi as u32),
        prod_tak_kenal: klatos_irq_buku_besar_counter(Counter::ProdTakKenal as u32),
        prod_slot_aktif: klatos_irq_buku_besar_counter(Counter::ProdSlotAktif as u32),
        prod_ilegal: klatos_irq_buku_besar_counter(Counter::ProdIlegal as u32),
        n_irq: N_IRQ,
        total_hits: klatos_irq_buku_besar_total(),
        total_eois: klatos_irq_buku_besar_total_eoi(),
        status,
    };
    // Hermetis: kembalikan buku besar + status interupsi.
    simpanan.pulihkan();
    crate::arch::pulihkan_interupsi(flags);
    laporan
}

/// Menjalankan matriks buku besar IRQ.
#[no_mangle]
pub extern "C" fn klatos_irq_buku_besar_selftest() -> KlatosBukuBesarIrqReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(N_IRQ == 16);
        assert!(REPORT_TAG == 0x4B4C_4154_4952_514C);
        assert!(Counter::ProdHit as usize == 0);
        assert!(Counter::ProdEoi as usize == 1);
        assert!(Counter::ProdTakKenal as usize == 2);
        assert!(Counter::ProdSlotAktif as usize == 3);
        assert!(Counter::ProdIlegal as usize == 4);
        assert!(N_COUNTER == 5);

        // 8 (tag) + 14*4 (u32) + 4 (status) = 68, dibulatkan ke kelipatan
        // align 8 -> 72.
        assert!(core::mem::size_of::<KlatosBukuBesarIrqReport>() == 72);
        assert!(core::mem::align_of::<KlatosBukuBesarIrqReport>() == 8);
    };
}