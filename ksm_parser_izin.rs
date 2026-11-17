// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas penguraian string izin `unveil()` Jowo (P1.13).


use core::sync::atomic::{AtomicU32, Ordering};

/// Kode galat POSIX `EINVAL` (cermin `<errno.h>` C).
pub const EINVAL: i32 = 22;

/// Cermin `IzinAksesJalur::Read`.
pub const IZIN_READ: u32 = 1;
/// Cermin `IzinAksesJalur::Write`.
pub const IZIN_WRITE: u32 = 2;
/// Cermin `IzinAksesJalur::Execute`.
pub const IZIN_EXECUTE: u32 = 4;
/// Cermin `IzinAksesJalur::CreateOrRemove`.
pub const IZIN_CREATE_OR_REMOVE: u32 = 8;
/// Cermin `IzinAksesJalur::Browse`.
pub const IZIN_BROWSE: u32 = 16;

/// Batas atas panjang string izin (cermin `get_syscall_string_fixed_buffer<5>`).
pub const MAX_PANJANG: u32 = 5;

/// Tag laporan; cermin `KlatosParserIzinJalurReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_554E564C; // "KLATUNVL"

/// Galat otoritas penguraian izin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IzinError {
    /// Karakter di luar `r/w/x/c/b`. Galat POSIX `EINVAL`.
    KarakterTidakDikenal,
    /// Panjang melebihi [`MAX_PANJANG`]. Galat POSIX `EINVAL`.
    TerlaluPanjang,
}

impl IzinError {
    /// Kode C ABI: negatif.
    pub const fn kode(&self) -> i32 {
        match self {
            IzinError::KarakterTidakDikenal => -EINVAL,
            IzinError::TerlaluPanjang => -EINVAL,
        }
    }

    /// Kode POSIX errno absolut.
    pub const fn errno(&self) -> i32 {
        match self {
            IzinError::KarakterTidakDikenal => EINVAL,
            IzinError::TerlaluPanjang => EINVAL,
        }
    }
}

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_ksm_parser_izin_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan dengan bit `Read`.
    ProdRead = 1,
    /// Jumlah keputusan dengan bit `Write`.
    ProdWrite = 2,
    /// Jumlah keputusan dengan bit `Execute`.
    ProdExecute = 3,
    /// Jumlah keputusan dengan bit `CreateOrRemove`.
    ProdCreate = 4,
    /// Jumlah keputusan dengan bit `Browse`.
    ProdBrowse = 5,
    /// Jumlah penolakan (karakter tak dikenal / terlalu panjang).
    ProdInvalid = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mengembalikan bitmask izin untuk `len` byte pertama dari `packed` (byte
/// ke-`i` ada di geser `8*i`), atau `Err` bila ada karakter tak dikenal.
///
/// Urutan dan isi keputusan SAMA PERSIS dengan loop C++ historis
/// (`Jowo/Keamanan/akses_jalur.cpp:80-99`).
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(packed: u64, len: u32) -> Result<u32, IzinError> {
    if len > MAX_PANJANG {
        return Err(IzinError::TerlaluPanjang);
    }

    let mut izin = 0u32;
    let mut i = 0u32;
    while i < len {
        let c = ((packed >> (8 * i)) & 0xFF) as u8;
        match c {
            b'r' => izin |= IZIN_READ,
            b'w' => izin |= IZIN_WRITE,
            b'x' => izin |= IZIN_EXECUTE,
            b'c' => izin |= IZIN_CREATE_OR_REMOVE,
            b'b' => izin |= IZIN_BROWSE,
            _ => return Err(IzinError::KarakterTidakDikenal),
        }
        i += 1;
    }
    Ok(izin)
}

/// Pembantu konformansi: `true` bila keputusan `Ok`.
const fn diterima(r: Result<u32, IzinError>) -> bool {
    matches!(r, Ok(_))
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan bitmask izin `>= 0`, atau `< 0` (= `-EINVAL`).
///
/// Menaikkan `prod_calls`, penghitung per-bit, atau `prod_invalid`.
#[no_mangle]
pub extern "C" fn klatos_ksm_parser_izin_decide(packed: u64, len: u32) -> i32 {
    bump(Counter::ProdCalls);
    match putuskan(packed, len) {
        Ok(mask) => {
            if mask & IZIN_READ != 0 {
                bump(Counter::ProdRead);
            }
            if mask & IZIN_WRITE != 0 {
                bump(Counter::ProdWrite);
            }
            if mask & IZIN_EXECUTE != 0 {
                bump(Counter::ProdExecute);
            }
            if mask & IZIN_CREATE_OR_REMOVE != 0 {
                bump(Counter::ProdCreate);
            }
            if mask & IZIN_BROWSE != 0 {
                bump(Counter::ProdBrowse);
            }
            mask as i32
        }
        Err(e) => {
            bump(Counter::ProdInvalid);
            e.kode()
        }
    }
}

/// Batas atas panjang string izin; cermin `get_syscall_string_fixed_buffer<5>`.
#[no_mangle]
pub extern "C" fn klatos_ksm_parser_izin_max_panjang() -> u32 {
    MAX_PANJANG
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_ksm_parser_izin_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_ksm_parser_izin_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosParserIzinJalurReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosParserIzinJalurReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_read: u32,
    pub prod_write: u32,
    pub prod_execute: u32,
    pub prod_create: u32,
    pub prod_browse: u32,
    pub prod_invalid: u32,
    pub max_panjang: u32,
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

/// Memadatkan string byte kecil ke `u64` (byte ke-`i` di geser `8*i`).
/// Hanya untuk matriks — produksi memakai pemadatan sisi C++.
const fn padat(s: &[u8]) -> u64 {
    let mut packed = 0u64;
    let mut i = 0usize;
    while i < s.len() {
        packed |= (s[i] as u64) << (8 * i);
        i += 1;
    }
    packed
}

/// Matriks otoritas penguraian izin `unveil`. Input SINTETIS.
fn run_matrix() -> KlatosParserIzinJalurReport {
    let mut m = Matrix::new();

    // --- kelompok 0: pemetaan karakter -------------------------------------
    m.catat(putuskan(padat(b""), 0) == Ok(0), Group::Peta);
    m.catat(putuskan(padat(b"r"), 1) == Ok(IZIN_READ), Group::Peta);
    m.catat(putuskan(padat(b"w"), 1) == Ok(IZIN_WRITE), Group::Peta);
    m.catat(putuskan(padat(b"x"), 1) == Ok(IZIN_EXECUTE), Group::Peta);
    m.catat(putuskan(padat(b"c"), 1) == Ok(IZIN_CREATE_OR_REMOVE), Group::Peta);
    m.catat(putuskan(padat(b"b"), 1) == Ok(IZIN_BROWSE), Group::Peta);
    m.catat(putuskan(padat(b"rw"), 2) == Ok(IZIN_READ | IZIN_WRITE), Group::Peta);
    m.catat(putuskan(padat(b"rwc"), 3) == Ok(IZIN_READ | IZIN_WRITE | IZIN_CREATE_OR_REMOVE), Group::Peta);
    m.catat(putuskan(padat(b"rwx"), 3) == Ok(IZIN_READ | IZIN_WRITE | IZIN_EXECUTE), Group::Peta);
    m.catat(putuskan(padat(b"rwxb"), 4) == Ok(IZIN_READ | IZIN_WRITE | IZIN_EXECUTE | IZIN_BROWSE), Group::Peta);
    m.catat(putuskan(padat(b"rwxcb"), 5) == Ok(31), Group::Peta);
    // Duplikat idempoten (OR).
    m.catat(putuskan(padat(b"rrr"), 3) == Ok(IZIN_READ), Group::Peta);
    // Urutan tidak berpengaruh.
    m.catat(putuskan(padat(b"wr"), 2) == putuskan(padat(b"rw"), 2), Group::Peta);
    // Hanya `len` byte pertama yang dibaca; byte di luar `len` diabaikan.
    m.catat(putuskan(padat(b"rz"), 1) == Ok(IZIN_READ), Group::Peta);
    m.catat(putuskan(padat(b"rz"), 2) == Err(IzinError::KarakterTidakDikenal), Group::Peta);

    // --- kelompok 1: negatif ------------------------------------------------
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(IZIN_READ == 1, Group::Neg);
    m.catat(IZIN_WRITE == 2, Group::Neg);
    m.catat(IZIN_EXECUTE == 4, Group::Neg);
    m.catat(IZIN_CREATE_OR_REMOVE == 8, Group::Neg);
    m.catat(IZIN_BROWSE == 16, Group::Neg);
    m.catat(MAX_PANJANG == 5, Group::Neg);
    m.catat(!diterima(putuskan(padat(b"z"), 1)), Group::Neg);
    m.catat(!diterima(putuskan(padat(b"R"), 1)), Group::Neg); // huruf besar ditolak
    m.catat(!diterima(putuskan(padat(b" "), 1)), Group::Neg);
    m.catat(!diterima(putuskan(0, 6)), Group::Neg); // terlalu panjang
    m.catat(IzinError::KarakterTidakDikenal.kode() == -EINVAL, Group::Neg);
    m.catat(IzinError::TerlaluPanjang.errno() == EINVAL, Group::Neg);
    // Sapuan penuh 1..=255 pada satu byte: HANYA lima karakter yang diterima.
    let mut n_terima = 0u32;
    let mut b = 1u32;
    while b <= 255 {
        if diterima(putuskan(b as u64, 1)) {
            n_terima += 1;
        }
        b += 1;
    }
    m.catat(n_terima == 5, Group::Neg);
    // Konsistensi: 1000 panggilan input sama -> keputusan sama.
    let mut last: Option<Result<u32, IzinError>> = None;
    let mut ok = true;
    for _ in 0..1000 {
        let r = putuskan(padat(b"rw"), 2);
        if last.is_none() {
            last = Some(r);
        } else if last != Some(r) {
            ok = false;
            break;
        }
    }
    m.catat(ok, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosParserIzinJalurReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_ksm_parser_izin_counter(Counter::ProdCalls as u32),
        prod_read: klatos_ksm_parser_izin_counter(Counter::ProdRead as u32),
        prod_write: klatos_ksm_parser_izin_counter(Counter::ProdWrite as u32),
        prod_execute: klatos_ksm_parser_izin_counter(Counter::ProdExecute as u32),
        prod_create: klatos_ksm_parser_izin_counter(Counter::ProdCreate as u32),
        prod_browse: klatos_ksm_parser_izin_counter(Counter::ProdBrowse as u32),
        prod_invalid: klatos_ksm_parser_izin_counter(Counter::ProdInvalid as u32),
        max_panjang: MAX_PANJANG,
        status,
    }
}

/// Menjalankan matriks otoritas penguraian izin `unveil`.
#[no_mangle]
pub extern "C" fn klatos_ksm_parser_izin_selftest() -> KlatosParserIzinJalurReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EINVAL == 22);
        assert!(IZIN_READ == 1);
        assert!(IZIN_WRITE == 2);
        assert!(IZIN_EXECUTE == 4);
        assert!(IZIN_CREATE_OR_REMOVE == 8);
        assert!(IZIN_BROWSE == 16);
        assert!(MAX_PANJANG == 5);
        assert!(REPORT_TAG == 0x4B4C_4154_554E564C);

        assert!(matches!(putuskan(padat(b"r"), 1), Ok(v) if v == IZIN_READ));
        assert!(matches!(putuskan(padat(b"rw"), 2), Ok(v) if v == IZIN_READ | IZIN_WRITE));
        assert!(matches!(putuskan(padat(b"rwxcb"), 5), Ok(v) if v == 31));
        assert!(matches!(putuskan(padat(b""), 0), Ok(v) if v == 0));
        assert!(matches!(putuskan(padat(b"z"), 1), Err(IzinError::KarakterTidakDikenal)));
        assert!(matches!(putuskan(0, 6), Err(IzinError::TerlaluPanjang)));

        assert!(IzinError::KarakterTidakDikenal.kode() == -EINVAL);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInvalid as usize == 6);

        // 8 (tag) + 14*4 (u32) + 4 (status) = 68 -> 72 dengan padding.
        assert!(core::mem::size_of::<KlatosParserIzinJalurReport>() == 72);
        assert!(core::mem::align_of::<KlatosParserIzinJalurReport>() == 8);
    };
}
