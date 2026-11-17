// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas TRANSISI KEADAAN `pledge()` Jowo (P1.15).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `<errno.h>`.
pub const EPERM: i32 = 1;

/// Bit `janji::no_error` (janji ke-28, indeks 27).
pub const NO_ERROR_BIT: u32 = 1 << 27;
/// Jumlah janji (cermin `ENUMERATE_KSM_PERMISSIONS`).
pub const N_PLEDGE: u32 = 28;
/// Bitmask sah tertinggi; 28 bit, jadi bit 31 SELALU 0.
pub const MASK_MAKS: u32 = (1u32 << N_PLEDGE) - 1;

/// Verdict: tolak dengan `EPERM` (negatif).
pub const KSM_TOLAK: i32 = -EPERM;

/// Tag laporan; cermin `KlatosKsmTransisiReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_504C_4447; // "KLATPLDG"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_ksm_transisi_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan "diterima tanpa penyempitan".
    ProdTerima = 1,
    /// Jumlah keputusan "diterima DENGAN penyempitan".
    ProdSempit = 2,
    /// Jumlah penolakan `EPERM`.
    ProdTolak = 3,
    /// Jumlah keputusan ketika belum ada janji sebelumnya (`has_prev == 0`).
    ProdTanpaPrev = 4,
}

const N_COUNTER: usize = 5;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan transisi keadaan janji.
///
/// * `has_prev` — apakah sudah ada janji sebelumnya (`has_promises` /
///   `has_execpromises`).
/// * `prev_mask` — bitmask janji sebelumnya.
/// * `new_mask` — bitmask janji yang diminta.
/// * `no_error_set` — apakah bit `no_error` menyala di **`protected_data.promises`**
///   (lihat catatan keanehan historis di dokumentasi modul).
///
/// Mengembalikan bitmask yang harus diterapkan (`>= 0`) atau `-EPERM`.
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(has_prev: bool, prev_mask: u32, new_mask: u32, no_error_set: bool) -> i32 {
    // Belum ada janji sebelumnya -> tidak ada penyempitan.
    if !has_prev {
        return new_mask as i32;
    }
    // Sudah himpunan bagian -> tidak ada yang perlu disempitkan.
    if (new_mask & !prev_mask) == 0 {
        return new_mask as i32;
    }
    // Meminta janji yang lebih luas.
    if !no_error_set {
        return KSM_TOLAK;
    }
    (new_mask & prev_mask) as i32
}

/// Cermin transkripsi dari kebijakan C++ historis (`pledge.cpp:63-96`), ditulis
/// ulang di Rust HANYA sebagai pembanding matriks.
#[inline]
pub const fn kebijakan_lama(has_prev: bool, prev_mask: u32, new_mask: u32, no_error_set: bool) -> i32 {
    let mut hasil = new_mask;
    if has_prev && (new_mask & !prev_mask) != 0 {
        if !no_error_set {
            return KSM_TOLAK;
        }
        hasil &= prev_mask;
    }
    hasil as i32
}

/// Apakah hasil ini "diterima dengan penyempitan"?
#[inline]
pub const fn menyempit(has_prev: bool, prev_mask: u32, new_mask: u32, hasil: i32) -> bool {
    hasil >= 0 && has_prev && (new_mask & !prev_mask) != 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan transisi keadaan janji kebijakan keamanan. Lihat [`putuskan`] untuk kontrak.
///
/// Menaikkan `prod_calls` dan penghitung yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_ksm_transisi_decide(
    has_prev: u32,
    prev_mask: u32,
    new_mask: u32,
    no_error_set: u32,
) -> i32 {
    bump(Counter::ProdCalls);
    let hp = has_prev != 0;
    let ne = no_error_set != 0;
    if !hp {
        bump(Counter::ProdTanpaPrev);
    }
    let hasil = putuskan(hp, prev_mask, new_mask, ne);
    if hasil < 0 {
        bump(Counter::ProdTolak);
    } else if menyempit(hp, prev_mask, new_mask, hasil) {
        bump(Counter::ProdSempit);
    } else {
        bump(Counter::ProdTerima);
    }
    hasil
}

/// Verdict tolak (negatif). C++ TIDAK boleh menyalin angkanya.
#[no_mangle]
pub extern "C" fn klatos_ksm_transisi_tolak() -> i32 {
    KSM_TOLAK
}

/// Bit `janji::no_error`.
#[no_mangle]
pub extern "C" fn klatos_ksm_transisi_no_error_bit() -> u32 {
    NO_ERROR_BIT
}

/// Jumlah janji.
#[no_mangle]
pub extern "C" fn klatos_ksm_transisi_n_izin() -> u32 {
    N_PLEDGE
}

/// Bitmask sah tertinggi.
#[no_mangle]
pub extern "C" fn klatos_ksm_transisi_mask_maks() -> u32 {
    MASK_MAKS
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_ksm_transisi_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_ksm_transisi_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosKsmTransisiReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosKsmTransisiReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_terima: u32,
    pub prod_sempit: u32,
    pub prod_tolak: u32,
    pub prod_tanpa_prev: u32,
    pub n_izin: u32,
    pub no_error_bit: u32,
    pub mask_maks: u32,
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

/// Matriks otoritas transisi kebijakan keamanan. Input SINTETIS.
fn run_matrix() -> KlatosKsmTransisiReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Tanpa janji sebelumnya: apa pun diterima apa adanya.
    m.catat(putuskan(false, 0, 0, false) == 0, Group::Peta);
    m.catat(putuskan(false, 0, MASK_MAKS, false) == MASK_MAKS as i32, Group::Peta);
    m.catat(putuskan(false, 0x1234, 0xFFFF, false) == 0xFFFF, Group::Peta);
    // Himpunan bagian: diterima apa adanya, tidak menyempit.
    m.catat(putuskan(true, 0xFF, 0x0F, false) == 0x0F, Group::Peta);
    m.catat(putuskan(true, 0xFF, 0xFF, false) == 0xFF, Group::Peta);
    m.catat(putuskan(true, MASK_MAKS, 0, false) == 0, Group::Peta);
    // Perluasan tanpa no_error: DITOLAK.
    m.catat(putuskan(true, 0x0F, 0xFF, false) == KSM_TOLAK, Group::Peta);
    m.catat(putuskan(true, 0, 1, false) == KSM_TOLAK, Group::Peta);
    // Perluasan dengan no_error: disempitkan ke irisan.
    m.catat(putuskan(true, 0x0F, 0xFF, true) == 0x0F, Group::Peta);
    m.catat(putuskan(true, 0x0F, 0xF3, true) == 0x03, Group::Peta);
    m.catat(putuskan(true, 0, MASK_MAKS, true) == 0, Group::Peta);
    // Perluasan sebagian dengan no_error: sisa di luar irisan dibuang.
    m.catat(putuskan(true, 0b1010, 0b0111, true) == 0b0010, Group::Peta);

    // --- kelompok 1: sapuan ekshaustif atas 4 bit rendah --------------------
    // Domain 4-bit (16 nilai) x 16 nilai x has_prev(2) x no_error(2) = 1024,
    // yang menutup SEMUA bentuk keputusan (subset, superset, campuran).
    {
        let mut n = 0u32;
        let mut beda = 0u32;
        let mut tak_dikenal = 0u32;
        let mut n_terima = 0u32;
        let mut n_sempit = 0u32;
        let mut n_tolak = 0u32;
        let mut hp = 0u32;
        while hp < 2 {
            let mut pv = 0u32;
            while pv < 16 {
                let mut nw = 0u32;
                while nw < 16 {
                    let mut ne = 0u32;
                    while ne < 2 {
                        let a = putuskan(hp != 0, pv, nw, ne != 0);
                        let b = kebijakan_lama(hp != 0, pv, nw, ne != 0);
                        n += 1;
                        if a != b {
                            beda += 1;
                        }
                        match a {
                            KSM_TOLAK => n_tolak += 1,
                            v if v >= 0 => {
                                if hp != 0 && (nw & !pv) != 0 {
                                    n_sempit += 1;
                                } else {
                                    n_terima += 1;
                                }
                            }
                            _ => tak_dikenal += 1,
                        }
                        ne += 1;
                    }
                    nw += 1;
                }
                pv += 1;
            }
            hp += 1;
        }
        m.catat(n == 1024, Group::Peta);
        m.catat(beda == 0, Group::Peta);
        m.catat(tak_dikenal == 0, Group::Peta);
        m.catat(n_terima > 0 && n_sempit > 0 && n_tolak > 0, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(EPERM == 1, Group::Neg);
    m.catat(KSM_TOLAK == -EPERM, Group::Neg);
    m.catat(N_PLEDGE == 28, Group::Neg);
    m.catat(NO_ERROR_BIT == (1u32 << 27), Group::Neg);
    m.catat(MASK_MAKS == (1u32 << 28) - 1, Group::Neg);
    // Bitmask SELALU muat sebagai i32 non-negatif -> tidak ada tabrakan dengan
    // penanda galat negatif (pelajaran F-P107-05).
    m.catat(MASK_MAKS < 0x8000_0000, Group::Neg);
    m.catat((MASK_MAKS as i32) >= 0, Group::Neg);
    m.catat(KSM_TOLAK < 0, Group::Neg);
    // Tanpa janji sebelumnya, TIDAK PERNAH menolak.
    {
        let mut ok = true;
        let mut nw = 0u32;
        while nw < 256 {
            if putuskan(false, 0, nw, false) < 0 {
                ok = false;
            }
            nw += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Himpunan bagian TIDAK PERNAH menolak.
    {
        let mut ok = true;
        let mut nw = 0u32;
        while nw < 256 {
            if putuskan(true, 0xFF, nw, false) < 0 {
                ok = false;
            }
            nw += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Perluasan TANPA no_error SELALU menolak.
    m.catat(putuskan(true, 0x00FF, 0x0100, false) == KSM_TOLAK, Group::Neg);
    m.catat(putuskan(true, 0x00FF, 0xFFFF, false) == KSM_TOLAK, Group::Neg);
    // Perluasan DENGAN no_error TIDAK PERNAH menolak, dan hasilnya ⊆ prev.
    {
        let mut ok = true;
        let mut nw = 0u32;
        while nw < 256 {
            let r = putuskan(true, 0x0F, nw, true);
            if r < 0 || (r as u32 & !0x0Fu32) != 0 {
                ok = false;
            }
            nw += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Determinisme.
    {
        let mut last: Option<i32> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(true, 0x0F, 0xFF, true);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: harapan yang SALAH harus terdeteksi.
    {
        let benar = putuskan(true, 0x0F, 0xFF, false);
        m.catat(benar == KSM_TOLAK && benar != 0 && benar != 0x0F, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosKsmTransisiReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_ksm_transisi_counter(Counter::ProdCalls as u32),
        prod_terima: klatos_ksm_transisi_counter(Counter::ProdTerima as u32),
        prod_sempit: klatos_ksm_transisi_counter(Counter::ProdSempit as u32),
        prod_tolak: klatos_ksm_transisi_counter(Counter::ProdTolak as u32),
        prod_tanpa_prev: klatos_ksm_transisi_counter(Counter::ProdTanpaPrev as u32),
        n_izin: N_PLEDGE,
        no_error_bit: NO_ERROR_BIT,
        mask_maks: MASK_MAKS,
        status,
    }
}

/// Menjalankan matriks otoritas transisi kebijakan keamanan.
#[no_mangle]
pub extern "C" fn klatos_ksm_transisi_selftest() -> KlatosKsmTransisiReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EPERM == 1);
        assert!(KSM_TOLAK == -1);
        assert!(N_PLEDGE == 28);
        assert!(NO_ERROR_BIT == (1u32 << 27));
        assert!(MASK_MAKS == (1u32 << 28) - 1);
        // Inti pembuktian pengkodean: bitmask tertinggi < 2^31.
        assert!(MASK_MAKS < 0x8000_0000);
        assert!(REPORT_TAG == 0x4B4C_4154_504C_4447);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdTanpaPrev as usize == 4);

        assert!(putuskan(false, 0, 0xFFFF, false) == 0xFFFF);
        assert!(putuskan(true, 0x0F, 0x0F, false) == 0x0F);
        assert!(putuskan(true, 0x0F, 0xFF, false) == KSM_TOLAK);
        assert!(putuskan(true, 0x0F, 0xFF, true) == 0x0F);
        assert!(putuskan(true, 0b1010, 0b0111, true) == 0b0010);

        // 8 (tag) + 14*4 (u32) + 4 (status) = 68 -> 72 dengan padding.
        assert!(core::mem::size_of::<KlatosKsmTransisiReport>() == 72);
        assert!(core::mem::align_of::<KlatosKsmTransisiReport>() == 8);
    };
}
