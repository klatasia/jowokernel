// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS ADMISI PERANGKAT & TTY**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;

/// Tag laporan; cermin `KlatosAdmisiPerangkatReport` di `Jowo/Kontrak/KlatSeam79.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4445_5641; // "KLATDEVA"

/// Aksi `TCFLSH` — bitmask.
pub const AKSI_TIDAK: u32 = 0;
/// Operasi meminta pengurasan masukan (`TCIFLUSH`/`TCIOFLUSH`).
pub const AKSI_FLUSH: u32 = 1;
/// Operasi tidak dikenal — tolak `EINVAL`.
pub const AKSI_TOLAK: u32 = 2;

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_dev_buka` dari produksi (setiap buka perangkat).
    ProdBuka = 0,
    /// `klatos_dev_spgrp` dari produksi.
    ProdSpgrp = 1,
    /// `klatos_dev_tcflush` dari produksi.
    ProdFlush = 2,
    /// `klatos_dev_winsz` dari produksi.
    ProdWinsz = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Admisi buka perangkat. Transkripsi `Device::open:124-125`:
///
/// ```cpp
/// if (Process::current().is_jailed() && !is_openable_by_jailed_processes())
///     return Error::from_errno(EPERM);
/// ```
///
/// `jailed`/`boleh` DISUPLAI C++. Mengembalikan `0` (lanjut) atau `-EPERM`.
#[inline]
pub const fn buka_admisi(jailed: u32, boleh: u32) -> i32 {
    if jailed != 0 && boleh == 0 {
        return -EPERM;
    }
    0
}

/// Admisi `TIOCSPGRP`. Transkripsi `TTY::ioctl:494-508`:
///
/// ```cpp
/// if (pgid <= 0) return EINVAL;
/// if (!process_group) return EINVAL;
/// SessionID new_sid = process ? process->sid() : Process::get_sid_from_pgid(pgid);
/// if (!new_sid || new_sid != current_process.sid()) return EPERM;
/// if (process && pgid != process->pgid()) return EPERM;
/// ```
///
/// `grup_ada` = grup proses ada; `sid_cocok` = `new_sid` ada DAN sama dengan
/// sid pemanggil; `pgid_beda` = `process && pgid != process->pgid()`.
/// Mengembalikan `0`, `-EINVAL`, atau `-EPERM`. Urutan penting: `EINVAL`
/// diperiksa lebih dulu.
#[inline]
pub const fn spgrp_admisi(pgid: i32, grup_ada: u32, sid_cocok: u32, pgid_beda: u32) -> i32 {
    if pgid <= 0 {
        return -EINVAL;
    }
    if grup_ada == 0 {
        return -EINVAL;
    }
    if sid_cocok == 0 {
        return -EPERM;
    }
    if pgid_beda != 0 {
        return -EPERM;
    }
    0
}

/// Penerapan `TCFLSH`. Transkripsi `TTY::ioctl:537-542`:
///
/// ```cpp
/// auto operation = static_cast<u8>(arg.ptr());
/// if (operation == TCIFLUSH || operation == TCIOFLUSH) flush_input();
/// else if (operation != TCOFLUSH) return EINVAL;
/// ```
///
/// `tciflush`/`tcioflush`/`tcoflush` DISUPLAI C++ (kontrak ABI). Mengembalikan
/// bitmask [`AKSI_FLUSH`] (kuras masukan) dan/atau [`AKSI_TOLAK`] (tolak
/// `EINVAL`). Keduanya tidak pernah bersamaan.
#[inline]
pub const fn tcflush_terapkan(operasi: u32, tciflush: u32, tcioflush: u32, tcoflush: u32) -> u32 {
    if operasi == tciflush || operasi == tcioflush {
        return AKSI_FLUSH;
    }
    if operasi != tcoflush {
        return AKSI_TOLAK;
    }
    AKSI_TIDAK
}

/// Perubahan ukuran `TIOCSWINSZ`. Transkripsi `TTY::ioctl:557-558`:
///
/// ```cpp
/// if (ws.ws_col == m_columns && ws.ws_row == m_rows) return {};
/// m_rows = ws.ws_row; m_columns = ws.ws_col; generate_signal(SIGWINCH);
/// ```
///
/// Mengembalikan `1` bila ukuran BERUBAH (harus diterapkan + `SIGWINCH`),
/// `0` bila tidak berubah (no-op dini).
#[inline]
pub const fn winsz_berubah(kolom_baru: u32, baris_baru: u32, kolom_lama: u32, baris_lama: u32) -> u32 {
    if kolom_baru == kolom_lama && baris_baru == baris_lama {
        return 0;
    }
    1
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_buka_admisi(jailed: u32, boleh: u32) -> i32 {
    let tolak = jailed == 1 && boleh == 0;
    if tolak {
        -EPERM
    } else {
        0
    }
}

fn naif_spgrp_admisi(pgid: i32, grup_ada: u32, sid_cocok: u32, pgid_beda: u32) -> i32 {
    if pgid < 1 {
        return -EINVAL;
    }
    if grup_ada == 0 {
        return -EINVAL;
    }
    let boleh = sid_cocok == 1 && pgid_beda == 0;
    if boleh {
        0
    } else {
        -EPERM
    }
}

fn naif_tcflush_terapkan(operasi: u32, tciflush: u32, tcioflush: u32, tcoflush: u32) -> u32 {
    let flush = operasi == tciflush || operasi == tcioflush;
    let tolak = operasi != tciflush && operasi != tcioflush && operasi != tcoflush;
    u32::from(flush) * AKSI_FLUSH | u32::from(tolak) * AKSI_TOLAK
}

fn naif_winsz_berubah(kolom_baru: u32, baris_baru: u32, kolom_lama: u32, baris_lama: u32) -> u32 {
    let sama = kolom_baru == kolom_lama && baris_baru == baris_lama;
    u32::from(!sama)
}

/// Kontrol NEGATIF: `spgrp_admisi` yang memakai `pgid == 0` alih-alih
/// `pgid <= 0`. Sengaja salah; berdaya pada `pgid < 0` — produksi `-EINVAL`,
/// yang salah lanjut ke `-EPERM`.
fn spgrp_admisi_salah_batas(pgid: i32, grup_ada: u32, sid_cocok: u32, pgid_beda: u32) -> i32 {
    if pgid == 0 {
        return -EINVAL;
    }
    if grup_ada == 0 {
        return -EINVAL;
    }
    if sid_cocok == 0 {
        return -EPERM;
    }
    if pgid_beda != 0 {
        return -EPERM;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Admisi buka perangkat. Lihat [`buka_admisi`].
#[no_mangle]
pub extern "C" fn klatos_dev_buka(jailed: u32, boleh: u32) -> i32 {
    bump(Counter::ProdBuka);
    buka_admisi(jailed, boleh)
}

/// Admisi `TIOCSPGRP`. Lihat [`spgrp_admisi`].
#[no_mangle]
pub extern "C" fn klatos_dev_spgrp(pgid: i32, grup_ada: u32, sid_cocok: u32, pgid_beda: u32) -> i32 {
    bump(Counter::ProdSpgrp);
    spgrp_admisi(pgid, grup_ada, sid_cocok, pgid_beda)
}

/// Penerapan `TCFLSH`. Lihat [`tcflush_terapkan`].
#[no_mangle]
pub extern "C" fn klatos_dev_tcflush(operasi: u32, tciflush: u32, tcioflush: u32, tcoflush: u32) -> u32 {
    bump(Counter::ProdFlush);
    tcflush_terapkan(operasi, tciflush, tcioflush, tcoflush)
}

/// Perubahan ukuran `TIOCSWINSZ`. Lihat [`winsz_berubah`].
#[no_mangle]
pub extern "C" fn klatos_dev_winsz(kolom_baru: u32, baris_baru: u32, kolom_lama: u32, baris_lama: u32) -> u32 {
    bump(Counter::ProdWinsz);
    winsz_berubah(kolom_baru, baris_baru, kolom_lama, baris_lama)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_dev_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_dev_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiPerangkatReport` di `Jowo/Kontrak/KlatSeam79.h`.
#[repr(C)]
pub struct KlatosAdmisiPerangkatReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub buka_checks: u32,
    pub buka_passed: u32,
    pub spgrp_checks: u32,
    pub spgrp_passed: u32,
    pub flush_checks: u32,
    pub flush_passed: u32,
    pub winsz_checks: u32,
    pub winsz_passed: u32,
    pub prod_buka: u32,
    pub prod_spgrp: u32,
    pub prod_flush: u32,
    pub prod_winsz: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Buka,
    Spgrp,
    Flush,
    Winsz,
}

struct Matrix {
    checks: u32,
    passed: u32,
    buka_checks: u32,
    buka_passed: u32,
    spgrp_checks: u32,
    spgrp_passed: u32,
    flush_checks: u32,
    flush_passed: u32,
    winsz_checks: u32,
    winsz_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            buka_checks: 0,
            buka_passed: 0,
            spgrp_checks: 0,
            spgrp_passed: 0,
            flush_checks: 0,
            flush_passed: 0,
            winsz_checks: 0,
            winsz_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Buka => {
                self.buka_checks += 1;
                if lulus {
                    self.buka_passed += 1;
                }
            }
            Group::Spgrp => {
                self.spgrp_checks += 1;
                if lulus {
                    self.spgrp_passed += 1;
                }
            }
            Group::Flush => {
                self.flush_checks += 1;
                if lulus {
                    self.flush_passed += 1;
                }
            }
            Group::Winsz => {
                self.winsz_checks += 1;
                if lulus {
                    self.winsz_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan oleh `run_matrix`.
///
/// buka `2 × 2` = 4; spgrp `3 × 2 × 2 × 2` = 24; tcflush `5 × 2` = 10;
/// winsz `3 × 3` = 9 ⇒ 4+24+10+9 = **47**.
pub const DIFF_EXPECTED: u32 = 47;

/// Nilai ABI sintetis (nilai NYATA diambil dari C++ saat produksi).
const TCIFLUSH_SINTETIS: u32 = 0;
const TCOFLUSH_SINTETIS: u32 = 1;
const TCIOFLUSH_SINTETIS: u32 = 2;

fn run_matrix() -> KlatosAdmisiPerangkatReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4445_5641, Group::Buka);
    m.catat(EPERM == 1 && EINVAL == 22, Group::Buka);
    m.catat(AKSI_TIDAK == 0 && AKSI_FLUSH == 1 && AKSI_TOLAK == 2, Group::Flush);

    // --- buka perangkat ---------------------------------------------------
    m.catat(buka_admisi(0, 0) == 0, Group::Buka); // tidak terjail -> boleh
    m.catat(buka_admisi(0, 1) == 0, Group::Buka);
    m.catat(buka_admisi(1, 1) == 0, Group::Buka); // terjail tapi diizinkan
    m.catat(buka_admisi(1, 0) == -EPERM, Group::Buka); // terjail & dilarang

    // --- TIOCSPGRP --------------------------------------------------------
    m.catat(spgrp_admisi(0, 1, 1, 0) == -EINVAL, Group::Spgrp);
    m.catat(spgrp_admisi(-3, 1, 1, 0) == -EINVAL, Group::Spgrp);
    m.catat(spgrp_admisi(7, 0, 1, 0) == -EINVAL, Group::Spgrp);
    m.catat(spgrp_admisi(7, 1, 0, 0) == -EPERM, Group::Spgrp);
    m.catat(spgrp_admisi(7, 1, 1, 1) == -EPERM, Group::Spgrp);
    m.catat(spgrp_admisi(7, 1, 1, 0) == 0, Group::Spgrp);
    // Urutan: EINVAL menang atas EPERM (pgid<=0 meski sid tidak cocok).
    m.catat(spgrp_admisi(-1, 0, 0, 1) == -EINVAL, Group::Spgrp);

    // --- TCFLSH -----------------------------------------------------------
    m.catat(tcflush_terapkan(TCIFLUSH_SINTETIS, TCIFLUSH_SINTETIS, TCIOFLUSH_SINTETIS, TCOFLUSH_SINTETIS) == AKSI_FLUSH, Group::Flush);
    m.catat(tcflush_terapkan(TCIOFLUSH_SINTETIS, TCIFLUSH_SINTETIS, TCIOFLUSH_SINTETIS, TCOFLUSH_SINTETIS) == AKSI_FLUSH, Group::Flush);
    m.catat(tcflush_terapkan(TCOFLUSH_SINTETIS, TCIFLUSH_SINTETIS, TCIOFLUSH_SINTETIS, TCOFLUSH_SINTETIS) == AKSI_TIDAK, Group::Flush);
    m.catat(tcflush_terapkan(9, TCIFLUSH_SINTETIS, TCIOFLUSH_SINTETIS, TCOFLUSH_SINTETIS) == AKSI_TOLAK, Group::Flush);

    // --- TIOCSWINSZ -------------------------------------------------------
    m.catat(winsz_berubah(80, 24, 80, 24) == 0, Group::Winsz);
    m.catat(winsz_berubah(81, 24, 80, 24) == 1, Group::Winsz);
    m.catat(winsz_berubah(80, 25, 80, 24) == 1, Group::Winsz);

    // --- kontrol NEGATIF: batas pgid berdaya ------------------------------
    {
        let benar = spgrp_admisi(-3, 1, 1, 0);
        let salah = spgrp_admisi_salah_batas(-3, 1, 1, 0);
        m.catat(benar == -EINVAL, Group::Spgrp);
        // `pgid == 0` meloloskan pgid negatif -> 0 (izinkan), padahal produksi
        // menolaknya `-EINVAL`. Divergensi yang berdaya.
        m.catat(salah == 0, Group::Spgrp);
        m.catat(benar != salah, Group::Spgrp);
        // Di pgid > 0, keduanya HARUS sama.
        m.catat(
            spgrp_admisi(7, 1, 1, 0) == spgrp_admisi_salah_batas(7, 1, 1, 0),
            Group::Spgrp,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // buka: 2 x 2 = 4
    for jailed in 0..2u32 {
        for boleh in 0..2u32 {
            if buka_admisi(jailed, boleh) != naif_buka_admisi(jailed, boleh) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // spgrp: 3 x 2 x 2 x 2 = 24
    for &pgid in &[-1i32, 0, 5] {
        for grup in 0..2u32 {
            for sid in 0..2u32 {
                for beda_pgid in 0..2u32 {
                    if spgrp_admisi(pgid, grup, sid, beda_pgid)
                        != naif_spgrp_admisi(pgid, grup, sid, beda_pgid)
                    {
                        beda += 1;
                    }
                    diukur += 1;
                }
            }
        }
    }
    // tcflush: 5 x 2 = 10
    for op in 0..5u32 {
        for &tco in &[TCOFLUSH_SINTETIS, 7u32] {
            if tcflush_terapkan(op, TCIFLUSH_SINTETIS, TCIOFLUSH_SINTETIS, tco)
                != naif_tcflush_terapkan(op, TCIFLUSH_SINTETIS, TCIOFLUSH_SINTETIS, tco)
            {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // winsz: 3 x 3 = 9
    for kolom in 0..3u32 {
        for baris in 0..3u32 {
            if winsz_berubah(kolom, baris, 1, 1) != naif_winsz_berubah(kolom, baris, 1, 1) {
                beda += 1;
            }
            diukur += 1;
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Buka);
    m.catat(beda == 0, Group::Buka);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiPerangkatReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        buka_checks: m.buka_checks,
        buka_passed: m.buka_passed,
        spgrp_checks: m.spgrp_checks,
        spgrp_passed: m.spgrp_passed,
        flush_checks: m.flush_checks,
        flush_passed: m.flush_passed,
        winsz_checks: m.winsz_checks,
        winsz_passed: m.winsz_passed,
        prod_buka: klatos_dev_counter(Counter::ProdBuka as u32),
        prod_spgrp: klatos_dev_counter(Counter::ProdSpgrp as u32),
        prod_flush: klatos_dev_counter(Counter::ProdFlush as u32),
        prod_winsz: klatos_dev_counter(Counter::ProdWinsz as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi perangkat & TTY.
#[no_mangle]
pub extern "C" fn klatos_dev_selftest() -> KlatosAdmisiPerangkatReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4445_5641);
        assert!(EPERM == 1);
        assert!(EINVAL == 22);
        assert!(AKSI_TIDAK == 0);
        assert!(AKSI_FLUSH == 1);
        assert!(AKSI_TOLAK == 2);
        assert!(Counter::ProdBuka as usize == 0);
        assert!(Counter::ProdSpgrp as usize == 1);
        assert!(Counter::ProdFlush as usize == 2);
        assert!(Counter::ProdWinsz as usize == 3);
        assert!(N_COUNTER == 4);
        assert!(DIFF_EXPECTED == 47);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76 -> align 8 -> 80.
        assert!(core::mem::size_of::<KlatosAdmisiPerangkatReport>() == 80);
        assert!(core::mem::align_of::<KlatosAdmisiPerangkatReport>() == 8);
    };
}
