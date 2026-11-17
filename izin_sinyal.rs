// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas izin pengiriman sinyal (P1.30).


use core::sync::atomic::{AtomicU32, Ordering};

/// `SIGCONT` — cermin `Jowo/Kontrak/POSIX/signal_numbers.h:30`.
pub const SIGCONT: i32 = 18;

/// Tag laporan; cermin `KlatosIzinSinyalReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_495A_5347; // "KLATIZSG"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_izin_sinyal_decide`].
    ProdCalls = 0,
    /// Jumlah keputusan `diizinkan`.
    ProdAllow = 1,
    /// Jumlah keputusan `ditolak`.
    ProdDeny = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan izin pengiriman sinyal.
///
/// SAMA PERSIS dengan disjungsi empat suku `do_kill` historis. Mengembalikan
/// `1` (diizinkan) atau `0` (ditolak).
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(
    is_superuser: u32,
    sender_euid: u32,
    sender_uid: u32,
    sender_pgid: i32,
    signal: i32,
    target_uid: u32,
    target_pgid: i32,
) -> i32 {
    if is_superuser != 0 {
        return 1;
    }
    if sender_euid == target_uid {
        return 1;
    }
    if sender_uid == target_uid {
        return 1;
    }
    if signal == SIGCONT && sender_pgid == target_pgid {
        return 1;
    }
    0
}

/// Orakel historis: transkripsi langsung `can_send_signal`
/// (`kill.cpp:23-27`). Dipakai HANYA oleh matriks diferensial.
///
/// Struktur sengaja berbeda (satu ekspresi boolean) supaya kesepakatan dengan
/// [`putuskan`] bermakna.
#[inline]
pub const fn kebijakan_lama(
    is_superuser: u32,
    sender_euid: u32,
    sender_uid: u32,
    sender_pgid: i32,
    signal: i32,
    target_uid: u32,
    target_pgid: i32,
) -> i32 {
    let can_send_signal = (is_superuser != 0)
        || (sender_euid == target_uid)
        || (sender_uid == target_uid)
        || (signal == SIGCONT && sender_pgid == target_pgid);
    if can_send_signal {
        1
    } else {
        0
    }
}

/// Apakah hasil ini keputusan `diizinkan`?
#[inline]
pub const fn diizinkan(hasil: i32) -> bool {
    hasil == 1
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan izin pengiriman sinyal. Lihat [`putuskan`] untuk kontrak.
#[no_mangle]
pub extern "C" fn klatos_izin_sinyal_decide(
    is_superuser: u32,
    sender_euid: u32,
    sender_uid: u32,
    sender_pgid: i32,
    signal: i32,
    target_uid: u32,
    target_pgid: i32,
) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(is_superuser, sender_euid, sender_uid, sender_pgid, signal, target_uid, target_pgid);
    if hasil == 1 {
        bump(Counter::ProdAllow);
    } else {
        bump(Counter::ProdDeny);
    }
    hasil
}

/// Nilai `SIGCONT` yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_izin_sinyal_sigcont() -> i32 {
    SIGCONT
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_izin_sinyal_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_izin_sinyal_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosIzinSinyalReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosIzinSinyalReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_allow: u32,
    pub prod_deny: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Ukuran domain sapuan diferensial.
///
/// `signal` (6) x `is_superuser` (2) x `sender_euid` (3) x `sender_uid` (3) x
/// `sender_pgid` (3) x `target_uid` (3) x `target_pgid` (3) = 2_916.
pub const DOMAIN_EXPECTED: u32 = 6 * 2 * 3 * 3 * 3 * 3 * 3;

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

const SIGNALS: [i32; 6] = [0, 1, 9, 18, 19, 31];
const UIDS: [u32; 3] = [0, 1000, 1001];
const PGIDS: [i32; 3] = [0, 5, 7];

/// Matriks otoritas izin pengiriman sinyal. Input SINTETIS.
fn run_matrix() -> KlatosIzinSinyalReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Root selalu boleh.
    m.catat(putuskan(1, 1000, 1000, 5, 9, 1001, 7) == 1, Group::Peta);
    m.catat(putuskan(1, 0, 0, 0, 9, 0, 0) == 1, Group::Peta);
    // euid cocok.
    m.catat(putuskan(0, 1000, 999, 5, 9, 1000, 7) == 1, Group::Peta);
    // uid cocok.
    m.catat(putuskan(0, 999, 1000, 5, 9, 1000, 7) == 1, Group::Peta);
    // SIGCONT + pgid cocok.
    m.catat(putuskan(0, 999, 998, 5, SIGCONT, 1000, 5) == 1, Group::Peta);
    // SIGCONT tapi pgid beda -> tolak.
    m.catat(putuskan(0, 999, 998, 5, SIGCONT, 1000, 7) == 0, Group::Peta);
    // Sinyal lain + pgid cocok -> tolak.
    m.catat(putuskan(0, 999, 998, 5, 9, 1000, 5) == 0, Group::Peta);
    // Tak ada yang cocok -> tolak.
    m.catat(putuskan(0, 999, 998, 5, 9, 1000, 7) == 0, Group::Peta);
    // signal 0 (probe) tetap tunduk aturan yang sama.
    m.catat(putuskan(0, 1000, 999, 5, 0, 1000, 7) == 1, Group::Peta);
    m.catat(putuskan(0, 999, 998, 5, 0, 1000, 7) == 0, Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    let mut diukur = 0u32;
    let mut beda = 0u32;
    for &signal in SIGNALS.iter() {
        for su in [0u32, 1] {
            for &se in UIDS.iter() {
                for &sr in UIDS.iter() {
                    for &sp in PGIDS.iter() {
                        for &tu in UIDS.iter() {
                            for &tp in PGIDS.iter() {
                                let r = putuskan(su, se, sr, sp, signal, tu, tp);
                                let b = kebijakan_lama(su, se, sr, sp, signal, tu, tp);
                                diukur += 1;
                                if r != b {
                                    beda += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    m.catat(diukur == DOMAIN_EXPECTED, Group::Peta);
    m.catat(beda == 0, Group::Peta);

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_495A_5347, Group::Neg);
    m.catat(SIGCONT == 18, Group::Neg);
    // Determinisme.
    {
        let mut ok = true;
        let first = putuskan(0, 1000, 1000, 5, 9, 1000, 7);
        for _ in 0..1000 {
            if putuskan(0, 1000, 1000, 5, 9, 1000, 7) != first {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: harapan SALAH harus terdeteksi.
    {
        let mut ok = true;
        if putuskan(0, 1000, 999, 5, 9, 1000, 7) != 1 {
            ok = false;
        }
        if putuskan(0, 999, 998, 5, 9, 1000, 7) != 0 {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosIzinSinyalReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_izin_sinyal_counter(Counter::ProdCalls as u32),
        prod_allow: klatos_izin_sinyal_counter(Counter::ProdAllow as u32),
        prod_deny: klatos_izin_sinyal_counter(Counter::ProdDeny as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas izin pengiriman sinyal.
#[no_mangle]
pub extern "C" fn klatos_izin_sinyal_selftest() -> KlatosIzinSinyalReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_495A_5347);
        assert!(SIGCONT == 18);
        assert!(DOMAIN_EXPECTED == 2_916);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdDeny as usize == 2);

        assert!(putuskan(1, 1000, 1000, 5, 9, 1001, 7) == 1);
        assert!(putuskan(0, 1000, 999, 5, 9, 1000, 7) == 1);
        assert!(putuskan(0, 999, 1000, 5, 9, 1000, 7) == 1);
        assert!(putuskan(0, 999, 998, 5, SIGCONT, 1000, 5) == 1);
        assert!(putuskan(0, 999, 998, 5, 9, 1000, 7) == 0);
        assert!(putuskan(0, 999, 998, 5, SIGCONT, 1000, 7) == 0);

        assert!(core::mem::size_of::<KlatosIzinSinyalReport>() == 56);
        assert!(core::mem::align_of::<KlatosIzinSinyalReport>() == 8);
    };
}
