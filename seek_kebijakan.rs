// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas KEBIJAKAN `seek()` deskriptor berkas Jowo (P1.17).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EIO` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-5).
pub const EIO: i32 = 5;
/// `EINVAL` — indeks ke-22.
pub const EINVAL: i32 = 22;
/// `ESPIPE` — indeks ke-29.
pub const ESPIPE: i32 = 29;
/// `EOVERFLOW` — indeks ke-36.
pub const EOVERFLOW: i32 = 36;

/// Cermin `SEEK_SET`.
pub const SEEK_SET: i32 = 0;
/// Cermin `SEEK_CUR`.
pub const SEEK_CUR: i32 = 1;
/// Cermin `SEEK_END`.
pub const SEEK_END: i32 = 2;

/// Tag laporan; cermin `KlatosSeekReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5345_454B; // "KLATSEEK"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_seek_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah `SEEK_SET`.
    ProdSet = 1,
    /// Jumlah `SEEK_CUR`.
    ProdCur = 2,
    /// Jumlah `SEEK_END`.
    ProdEnd = 3,
    /// Jumlah penolakan `ESPIPE` (tidak dapat di-seek).
    ProdEspipe = 4,
    /// Jumlah penolakan `EINVAL`.
    ProdEinval = 5,
    /// Jumlah penolakan `EOVERFLOW`.
    ProdOverflow = 6,
    /// Jumlah penolakan `EIO` (metadata tidak sah pada `SEEK_END`).
    ProdEio = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan offset baru `seek`.
///
/// Urutan dan isi keputusan SAMA PERSIS dengan `OpenFileDescription::seek`
/// historis.
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(
    whence: i32,
    offset: i64,
    current_offset: i64,
    size: i64,
    seekable: bool,
    metadata_valid: bool,
) -> i64 {
    if !seekable {
        return -(ESPIPE as i64);
    }
    let baru: i64 = match whence {
        SEEK_SET => offset,
        SEEK_CUR => match current_offset.checked_add(offset) {
            Some(v) => v,
            None => return -(EOVERFLOW as i64),
        },
        SEEK_END => {
            if !metadata_valid {
                return -(EIO as i64);
            }
            match size.checked_add(offset) {
                Some(v) => v,
                None => return -(EOVERFLOW as i64),
            }
        }
        _ => return -(EINVAL as i64),
    };
    if baru < 0 {
        return -(EINVAL as i64);
    }
    baru
}

/// Cermin transkripsi dari kebijakan C++ historis, ditulis ulang di Rust HANYA
/// sebagai pembanding matriks.
#[inline]
pub const fn kebijakan_lama(
    whence: i32,
    offset: i64,
    current_offset: i64,
    size: i64,
    seekable: bool,
    metadata_valid: bool,
) -> i64 {
    if !seekable {
        return -(ESPIPE as i64);
    }
    let baru: i64;
    if whence == SEEK_SET {
        baru = offset;
    } else if whence == SEEK_CUR {
        match current_offset.checked_add(offset) {
            Some(v) => baru = v,
            None => return -(EOVERFLOW as i64),
        }
    } else if whence == SEEK_END {
        if !metadata_valid {
            return -(EIO as i64);
        }
        match size.checked_add(offset) {
            Some(v) => baru = v,
            None => return -(EOVERFLOW as i64),
        }
    } else {
        return -(EINVAL as i64);
    }
    if baru < 0 {
        return -(EINVAL as i64);
    }
    baru
}

/// Apakah hasil ini offset yang sah?
#[inline]
pub const fn berhasil(hasil: i64) -> bool {
    hasil >= 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan offset baru `seek`. Lihat [`putuskan`] untuk kontrak.
///
/// Menaikkan `prod_calls` dan penghitung yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_seek_decide(
    whence: i32,
    offset: i64,
    current_offset: i64,
    size: i64,
    seekable: u32,
    metadata_valid: u32,
) -> i64 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(
        whence,
        offset,
        current_offset,
        size,
        seekable != 0,
        metadata_valid != 0,
    );
    if hasil >= 0 {
        match whence {
            SEEK_SET => bump(Counter::ProdSet),
            SEEK_CUR => bump(Counter::ProdCur),
            SEEK_END => bump(Counter::ProdEnd),
            _ => {}
        }
    } else {
        match -hasil {
            v if v == EINVAL as i64 => bump(Counter::ProdEinval),
            v if v == ESPIPE as i64 => bump(Counter::ProdEspipe),
            v if v == EOVERFLOW as i64 => bump(Counter::ProdOverflow),
            v if v == EIO as i64 => bump(Counter::ProdEio),
            _ => {}
        }
    }
    hasil
}

/// Nilai errno yang dipakai modul ini. C++ TIDAK boleh menyalin angkanya.
#[no_mangle]
pub extern "C" fn klatos_seek_eio() -> i32 {
    EIO
}
#[no_mangle]
pub extern "C" fn klatos_seek_einval() -> i32 {
    EINVAL
}
#[no_mangle]
pub extern "C" fn klatos_seek_espipe() -> i32 {
    ESPIPE
}
#[no_mangle]
pub extern "C" fn klatos_seek_eoverflow() -> i32 {
    EOVERFLOW
}

/// Nilai `SEEK_*`.
#[no_mangle]
pub extern "C" fn klatos_seek_set() -> i32 {
    SEEK_SET
}
#[no_mangle]
pub extern "C" fn klatos_seek_cur() -> i32 {
    SEEK_CUR
}
#[no_mangle]
pub extern "C" fn klatos_seek_end() -> i32 {
    SEEK_END
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_seek_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_seek_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosSeekReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosSeekReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_set: u32,
    pub prod_cur: u32,
    pub prod_end: u32,
    pub prod_espipe: u32,
    pub prod_einval: u32,
    pub prod_overflow: u32,
    pub prod_eio: u32,
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

/// Matriks otoritas kebijakan `seek`. Input SINTETIS.
fn run_matrix() -> KlatosSeekReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Tidak dapat di-seek -> ESPIPE, apa pun sisanya.
    m.catat(putuskan(SEEK_SET, 0, 0, 0, false, true) == -(ESPIPE as i64), Group::Peta);
    m.catat(putuskan(SEEK_END, 0, 0, 0, false, false) == -(ESPIPE as i64), Group::Peta);
    // SEEK_SET: offset apa adanya.
    m.catat(putuskan(SEEK_SET, 100, 7, 999, true, true) == 100, Group::Peta);
    m.catat(putuskan(SEEK_SET, 0, 7, 999, true, true) == 0, Group::Peta);
    // SEEK_SET negatif -> EINVAL.
    m.catat(putuskan(SEEK_SET, -1, 0, 0, true, true) == -(EINVAL as i64), Group::Peta);
    // SEEK_CUR: relatif.
    m.catat(putuskan(SEEK_CUR, 5, 10, 999, true, true) == 15, Group::Peta);
    m.catat(putuskan(SEEK_CUR, -10, 10, 999, true, true) == 0, Group::Peta);
    m.catat(putuskan(SEEK_CUR, -11, 10, 999, true, true) == -(EINVAL as i64), Group::Peta);
    // SEEK_CUR luapan -> EOVERFLOW.
    m.catat(putuskan(SEEK_CUR, i64::MAX, 1, 0, true, true) == -(EOVERFLOW as i64), Group::Peta);
    m.catat(putuskan(SEEK_CUR, i64::MIN, -1, 0, true, true) == -(EOVERFLOW as i64), Group::Peta);
    // SEEK_END: dari ukuran berkas.
    m.catat(putuskan(SEEK_END, 0, 3, 4096, true, true) == 4096, Group::Peta);
    m.catat(putuskan(SEEK_END, -4096, 3, 4096, true, true) == 0, Group::Peta);
    m.catat(putuskan(SEEK_END, -4097, 3, 4096, true, true) == -(EINVAL as i64), Group::Peta);
    // SEEK_END dengan metadata tidak sah -> EIO.
    m.catat(putuskan(SEEK_END, 0, 3, 4096, true, false) == -(EIO as i64), Group::Peta);
    // SEEK_END luapan -> EOVERFLOW.
    m.catat(putuskan(SEEK_END, i64::MAX, 0, 1, true, true) == -(EOVERFLOW as i64), Group::Peta);
    // whence tak dikenal -> EINVAL (termasuk bila metadata tidak sah).
    m.catat(putuskan(3, 0, 0, 0, true, true) == -(EINVAL as i64), Group::Peta);
    m.catat(putuskan(-1, 0, 0, 0, true, false) == -(EINVAL as i64), Group::Peta);
    // Tidak dapat di-seek MENANG atas whence tak dikenal (urutan historis).
    m.catat(putuskan(99, 0, 0, 0, false, false) == -(ESPIPE as i64), Group::Peta);
    // SEEK_END tidak sah menang atas overflow.
    m.catat(putuskan(SEEK_END, i64::MAX, 0, 1, true, false) == -(EIO as i64), Group::Peta);

    // --- kelompok 1: sapuan ekshaustif --------------------------------------
    // whence(5) x offset(8) x current(4) x size(4) x seekable(2) x valid(2) = 2560
    {
        let offsets: [i64; 8] = [i64::MIN, -4097, -1, 0, 1, 100, i64::MAX - 1, i64::MAX];
        let currents: [i64; 4] = [i64::MIN, -1, 0, 10];
        let sizes: [i64; 4] = [i64::MIN, 0, 4096, i64::MAX];
        let whences: [i32; 5] = [-1, SEEK_SET, SEEK_CUR, SEEK_END, 3];
        let mut n = 0u32;
        let mut beda = 0u32;
        let mut negatif_sah = 0u32;
        for w in whences {
            for o in offsets {
                for c in currents {
                    for s in sizes {
                        for sk in 0..2u32 {
                            for mv in 0..2u32 {
                                let a = putuskan(w, o, c, s, sk != 0, mv != 0);
                                let b = kebijakan_lama(w, o, c, s, sk != 0, mv != 0);
                                n += 1;
                                if a != b {
                                    beda += 1;
                                }
                                // Hasil negatif HARUS salah satu errno yang dikenal.
                                if a < 0 {
                                    let e = -a;
                                    if e != EINVAL as i64 && e != ESPIPE as i64 && e != EOVERFLOW as i64 && e != EIO as i64 {
                                        negatif_sah += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        m.catat(n == 2560, Group::Peta);
        m.catat(beda == 0, Group::Peta);
        m.catat(negatif_sah == 0, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(EIO == 5, Group::Neg);
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(ESPIPE == 29, Group::Neg);
    m.catat(EOVERFLOW == 36, Group::Neg);
    m.catat(SEEK_SET == 0 && SEEK_CUR == 1 && SEEK_END == 2, Group::Neg);
    // Empat errno berbeda satu sama lain -> tidak ada penggabungan tak sengaja.
    m.catat(EIO != EINVAL && EINVAL != ESPIPE && ESPIPE != EOVERFLOW, Group::Neg);
    // Tidak ada offset sah yang negatif -> penanda galat tidak bertabrakan.
    {
        let mut ok = true;
        let mut o = -300i64;
        while o < 0 {
            if berhasil(putuskan(SEEK_SET, o, 0, 0, true, true)) {
                ok = false;
            }
            o += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // `seekable == false` SELALU `ESPIPE`.
    {
        let mut ok = true;
        let mut w = -2i32;
        while w < 5 {
            if putuskan(w, 0, 0, 0, false, true) != -(ESPIPE as i64) {
                ok = false;
            }
            w += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // `SEEK_SET` TIDAK PERNAH `EOVERFLOW`/`EIO`.
    {
        let mut ok = true;
        let mut o = -50i64;
        while o <= 50 {
            let r = putuskan(SEEK_SET, o, i64::MAX, i64::MAX, true, true);
            if r == -(EOVERFLOW as i64) || r == -(EIO as i64) {
                ok = false;
            }
            o += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Determinisme.
    {
        let mut last: Option<i64> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(SEEK_CUR, 5, 10, 999, true, true);
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
        let benar = putuskan(SEEK_END, 0, 0, 100, true, false);
        m.catat(benar == -(EIO as i64) && benar != 100 && benar != -(EINVAL as i64), Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosSeekReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_seek_counter(Counter::ProdCalls as u32),
        prod_set: klatos_seek_counter(Counter::ProdSet as u32),
        prod_cur: klatos_seek_counter(Counter::ProdCur as u32),
        prod_end: klatos_seek_counter(Counter::ProdEnd as u32),
        prod_espipe: klatos_seek_counter(Counter::ProdEspipe as u32),
        prod_einval: klatos_seek_counter(Counter::ProdEinval as u32),
        prod_overflow: klatos_seek_counter(Counter::ProdOverflow as u32),
        prod_eio: klatos_seek_counter(Counter::ProdEio as u32),
        status,
    }
}

/// Menjalankan matriks otoritas kebijakan `seek`.
#[no_mangle]
pub extern "C" fn klatos_seek_selftest() -> KlatosSeekReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EIO == 5);
        assert!(EINVAL == 22);
        assert!(ESPIPE == 29);
        assert!(EOVERFLOW == 36);
        assert!(SEEK_SET == 0);
        assert!(SEEK_CUR == 1);
        assert!(SEEK_END == 2);
        assert!(REPORT_TAG == 0x4B4C_4154_5345_454B);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdEio as usize == 7);

        assert!(putuskan(SEEK_SET, 100, 0, 0, true, true) == 100);
        assert!(putuskan(SEEK_CUR, 5, 10, 0, true, true) == 15);
        assert!(putuskan(SEEK_END, 0, 0, 4096, true, true) == 4096);
        assert!(putuskan(SEEK_SET, -1, 0, 0, true, true) == -22);
        assert!(putuskan(SEEK_CUR, i64::MAX, 1, 0, true, true) == -36);
        assert!(putuskan(SEEK_END, 0, 0, 0, true, false) == -5);
        assert!(putuskan(SEEK_SET, 0, 0, 0, false, true) == -29);
        assert!(putuskan(3, 0, 0, 0, true, true) == -22);

        // 8 (tag) + 14*4 (u32) + 4 (status) = 68 -> 72.
        assert!(core::mem::size_of::<KlatosSeekReport>() == 72);
        assert!(core::mem::align_of::<KlatosSeekReport>() == 8);
    };
}
