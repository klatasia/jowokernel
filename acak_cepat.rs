// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! OTORITAS TRANSISI KEADAAN PRNG CEPAT (Rust).


use core::cell::Cell;
use core::sync::atomic::{AtomicU32, Ordering};

/// Pengali LCG historis (`Random.cpp:157`).
pub const LCG_A: u32 = 1_103_515_245;
/// Penambah LCG historis.
pub const LCG_C: u32 = 12345;
/// Nilai awal `s_next_random_value` (`Random.cpp:28`).
pub const LCG_BIBIT: u32 = 1;

/// Tag laporan: "KLATACPT".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4143_5054;

const RENTANG_DIFERENSIAL: u64 = 262144;
const N_COUNTER: usize = 3;

// ---------------------------------------------------------------------------
// Oracle historis
// ---------------------------------------------------------------------------

/// Satu langkah LCG — terjemahan harfiah dari rumus C++.
pub const fn kebijakan_lama(state: u32) -> u32 {
    state.wrapping_mul(LCG_A).wrapping_add(LCG_C)
}

// ---------------------------------------------------------------------------
// Algoritma TUNGGAL
// ---------------------------------------------------------------------------

/// Penyimpanan keadaan LCG. Abstraksi ini ada supaya matriks memakai
/// algoritma produksi yang sama, bukan salinannya.
pub trait PenyimpananAcak {
    fn muat(&self) -> u32;
    fn cas(&self, cur: u32, baru: u32) -> bool;
    fn simpan(&self, v: u32);
}

/// Satu langkah produksi: hitung `kebijakan_lama(cur)`, pasang dengan CAS.
/// Mengembalikan `(nilai_baru, jumlah_ulang_cas)`.
pub fn langkah_in<S: PenyimpananAcak>(s: &S) -> (u32, u32) {
    let mut ulang = 0u32;
    let mut cur = s.muat();
    loop {
        let baru = kebijakan_lama(cur);
        if s.cas(cur, baru) {
            return (baru, ulang);
        }
        ulang += 1;
        cur = s.muat();
    }
}

// ---------------------------------------------------------------------------
// Penyimpanan PRODUKSI
// ---------------------------------------------------------------------------

/// Keadaan PRNG cepat produksi. Menggantikan `s_next_random_value` C++.
static KEADAAN: AtomicU32 = AtomicU32::new(LCG_BIBIT);

/// `0` = langkah, `1` = ulang_cas, `2` = galat.
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

impl PenyimpananAcak for AtomicU32 {
    fn muat(&self) -> u32 {
        self.load(Ordering::Relaxed)
    }
    fn cas(&self, cur: u32, baru: u32) -> bool {
        self.compare_exchange(cur, baru, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    }
    fn simpan(&self, v: u32) {
        self.store(v, Ordering::Release);
    }
}

/// Keadaan SINTETIS untuk matriks (stack, tanpa atomik).
pub struct PenyimpananUjiAcak {
    nilai: Cell<u32>,
}

impl PenyimpananUjiAcak {
    pub const fn baru(v: u32) -> Self {
        PenyimpananUjiAcak {
            nilai: Cell::new(v),
        }
    }
    pub fn intip(&self) -> u32 {
        self.nilai.get()
    }
}

impl PenyimpananAcak for PenyimpananUjiAcak {
    fn muat(&self) -> u32 {
        self.nilai.get()
    }
    fn cas(&self, cur: u32, baru: u32) -> bool {
        if self.nilai.get() == cur {
            self.nilai.set(baru);
            true
        } else {
            false
        }
    }
    fn simpan(&self, v: u32) {
        self.nilai.set(v);
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — PRODUKSI
// ---------------------------------------------------------------------------

/// Satu langkah LCG produksi. Mengembalikan nilai baru generator.
#[no_mangle]
pub extern "C" fn klatos_acak_cepat_berikut() -> u32 {
    let (nilai, ulang) = langkah_in(&KEADAAN);
    COUNTERS[0].fetch_add(1, Ordering::Relaxed);
    if ulang > 0 {
        COUNTERS[1].fetch_add(ulang, Ordering::Relaxed);
    }
    nilai
}

/// Nilai satu penghitung audit. `0`=langkah, `1`=ulang_cas, `2`=galat.
#[no_mangle]
pub extern "C" fn klatos_acak_cepat_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Keadaan generator saat ini (audit; bukan nilai terakhir yang diberikan).
#[no_mangle]
pub extern "C" fn klatos_acak_cepat_intip() -> u32 {
    KEADAAN.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAcakReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosAcakReport {
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

fn run_matrix() -> KlatosAcakReport {
    let mut m = Matrix::baru();

    // Kontrol: matriks tidak boleh menggeser keadaan produksi.
    let prod_before = KEADAAN.load(Ordering::Relaxed);

    // --- kelompok 0: langkah dasar -----------------------------------------
    {
        // Nilai pertama dari bibit 1 (dihitung oracle, bukan konstanta hafalan).
        let s = PenyimpananUjiAcak::baru(LCG_BIBIT);
        let (v1, u1) = langkah_in(&s);
        m.catat(v1 == kebijakan_lama(LCG_BIBIT), 0);
        m.catat(u1 == 0, 0);
        m.catat(s.intip() == v1, 0);

        let (v2, _) = langkah_in(&s);
        m.catat(v2 == kebijakan_lama(v1), 0);
        m.catat(v2 != v1, 0);

        // 1000 langkah: keadaan selalu berubah, dan tidak ada pengulangan
        // dalam jendela pendek (LCG penuh periode 2^32).
        let t = PenyimpananUjiAcak::baru(LCG_BIBIT);
        let mut berubah = true;
        let mut beda_semua = true;
        let mut i = 0;
        let mut prev = t.intip();
        while i < 1000 {
            let (v, _) = langkah_in(&t);
            if v == prev {
                berubah = false;
            }
            // sampel: 256 langkah pertama harus semuanya berbeda
            if i < 256 {
                // bandingkan terhadap 256 nilai pertama secara kuadratik kecil
                let mut j = 0;
                let u = PenyimpananUjiAcak::baru(LCG_BIBIT);
                while j < i {
                    let (w, _) = langkah_in(&u);
                    if w == v {
                        beda_semua = false;
                    }
                    j += 1;
                }
            }
            prev = v;
            i += 1;
        }
        m.catat(berubah, 0);
        m.catat(beda_semua, 0);
    }

    // --- kelompok 1: diferensial terhadap oracle historis -------------------
    let mut diukur = 0u32;
    let mut beda = 0u32;
    let mut seed = 0u64;
    while seed < RENTANG_DIFERENSIAL {
        let s = PenyimpananUjiAcak::baru(seed as u32);
        let (v, _) = langkah_in(&s);
        diukur += 1;
        if v != kebijakan_lama(seed as u32) || s.intip() != v {
            beda += 1;
        }
        seed += 1;
    }
    m.catat(beda == 0, 1);
    m.catat(diukur == RENTANG_DIFERENSIAL as u32, 1);

    // Diferensial dua langkah berurutan atas 4096 bibit.
    {
        let mut b2 = 0u32;
        let mut k = 0u64;
        while k < 4096 {
            let s = PenyimpananUjiAcak::baru(k as u32);
            let (a, _) = langkah_in(&s);
            let (b, _) = langkah_in(&s);
            diukur += 1;
            if b != kebijakan_lama(a) {
                b2 += 1;
            }
            k += 1;
        }
        m.catat(b2 == 0, 1);
    }
    // Total kasus diferensial = 262144 + 4096.
    m.catat(diukur == RENTANG_DIFERENSIAL as u32 + 4096, 1);

    // --- kelompok 2: negatif ------------------------------------------------
    {
        // Pembungkusan u32 adalah PERILAKU yang disengaja di sini (ini PRNG,
        // bukan ruang id): tidak ada penolakan.
        let s = PenyimpananUjiAcak::baru(u32::MAX);
        let (v, _) = langkah_in(&s);
        m.catat(v == kebijakan_lama(u32::MAX), 2);
        m.catat(s.intip() == v, 2);

        // Bibit 0 tetap menghasilkan nilai bukan-nol.
        let s0 = PenyimpananUjiAcak::baru(0);
        let (v0, _) = langkah_in(&s0);
        m.catat(v0 == LCG_C, 2);
        m.catat(v0 != 0, 2);

        // Aritmetika benar-benar membungkus (bukan panik / saturasi).
        m.catat(LCG_A.wrapping_mul(2) == LCG_A.wrapping_add(LCG_A), 2);
        m.catat(kebijakan_lama(u32::MAX) != u32::MAX, 2);

        // Konstanta persis seperti C++.
        m.catat(LCG_A == 1_103_515_245, 2);
        m.catat(LCG_C == 12345, 2);
        m.catat(LCG_BIBIT == 1, 2);

        // KONTROL: keadaan PRODUKSI tidak bergeser.
        m.catat(KEADAAN.load(Ordering::Relaxed) == prod_before, 2);
    }

    let mut report = KlatosAcakReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.grup_checks[1],
        peta_passed: m.grup_passed[1],
        neg_checks: m.grup_checks[2],
        neg_passed: m.grup_passed[2],
        prod_a: klatos_acak_cepat_counter(0),
        prod_b: klatos_acak_cepat_counter(1),
        prod_c: 0,
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status: 0,
    };
    if report.checks != report.passed {
        report.status = -1;
    }
    report
}

/// Menjalankan matriks otoritas PRNG cepat.
#[no_mangle]
pub extern "C" fn klatos_acak_cepat_selftest() -> KlatosAcakReport {
    run_matrix()
}

/// Konformansi waktu-kompilasi.
pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4143_5054); // "KLATACPT"
        assert!(LCG_A == 1103515245);
        assert!(LCG_C == 12345);
        assert!(LCG_BIBIT == 1);
        // Langkah pertama dari bibit 1.
        assert!(kebijakan_lama(1) == 1103527590);
        assert!(kebijakan_lama(0) == 12345);
        assert!(core::mem::size_of::<KlatosAcakReport>() == 56);
        assert!(N_COUNTER == 3);
    };
}
