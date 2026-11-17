// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! *OTORITAS PEMASANGAN + VALIDASI PERANGKAT**.



#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `ENODEV` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 19).
pub const ENODEV: i32 = 19;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `EOVERFLOW` — indeks 36.
pub const EOVERFLOW: i32 = 36;

/// Tag laporan; cermin `KlatosAdmisiPasangReport` di `Jowo/Kontrak/KlatSeam94.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5044_4556; // "KLATPDEV"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_pdev_bendera`.
    ProdBendera = 0,
    /// `klatos_devctl_cari`.
    ProdCari = 1,
    /// `klatos_devctl_offset`.
    ProdOffset = 2,
    /// `klatos_devctl_ukuran`.
    ProdUkuran = 3,
    /// `klatos_devctl_versi`.
    ProdVersi = 4,
    /// `klatos_devctl_operasi`.
    ProdOperasi = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Berkas-sistem tidak menerima jenis bendera mount ini. Satu verdict, empat
/// pemanggil. Transkripsi `VirtualFileSystem.cpp:97,102,107,112`.
#[inline]
pub const fn mount_bendera_admisi() -> i32 {
    -EINVAL
}

/// Perangkat loop tidak ditemukan. Transkripsi `DeviceControlDevice.cpp:94`.
#[inline]
pub const fn devctl_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENODEV;
    }
    0
}

/// `read` pada offset bukan nol. Transkripsi `DeviceControlDevice.cpp:129`.
#[inline]
pub const fn devctl_offset_admisi(offset: u64) -> i32 {
    if offset != 0 {
        return -EINVAL;
    }
    0
}

/// Ukuran `read` bukan kelipatan `DeviceEvent`. `sisa` adalah
/// `size % sizeof(DeviceEvent)`. Transkripsi `DeviceControlDevice.cpp:131`.
#[inline]
pub const fn devctl_ukuran_admisi(sisa: u64) -> i32 {
    if sisa != 0 {
        return -EOVERFLOW;
    }
    0
}

/// Versi protokol `devctl` tidak cocok. Satu verdict, empat pemanggil.
/// Transkripsi `DeviceControlDevice.cpp:180,197,220,235`.
#[inline]
pub const fn devctl_versi_admisi(cocok: u32) -> i32 {
    if cocok == 0 {
        return -EINVAL;
    }
    0
}

/// Operasi/ioctl `devctl` tak dikenal. Transkripsi
/// `DeviceControlDevice.cpp:267,299`.
#[inline]
pub const fn devctl_operasi_admisi(dikenal: u32) -> i32 {
    if dikenal == 0 {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_devctl_cari_admisi(ditemukan: u32) -> i32 {
    match ditemukan {
        0 => -ENODEV,
        _ => 0,
    }
}
fn naif_devctl_offset_admisi(offset: u64) -> i32 {
    if offset == 0 {
        0
    } else {
        -EINVAL
    }
}
fn naif_devctl_ukuran_admisi(sisa: u64) -> i32 {
    match sisa {
        0 => 0,
        _ => -EOVERFLOW,
    }
}
fn naif_devctl_versi_admisi(cocok: u32) -> i32 {
    match cocok {
        1 => 0,
        _ => -EINVAL,
    }
}
fn naif_devctl_operasi_admisi(dikenal: u32) -> i32 {
    match dikenal {
        0 => -EINVAL,
        _ => 0,
    }
}

/// Kontrol NEGATIF: `devctl_ukuran_admisi` yang salah mengembalikan `EINVAL`
/// (bukan `EOVERFLOW`). Sengaja SALAH.
fn devctl_ukuran_salah(sisa: u64) -> i32 {
    if sisa != 0 {
        return -EINVAL;
    }
    0
}

/// Kontrol NEGATIF: `devctl_cari_admisi` yang salah mengembalikan `ENOENT`
/// (bukan `ENODEV`). Sengaja SALAH.
fn devctl_cari_salah(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -2; // ENOENT
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`mount_bendera_admisi`].
#[no_mangle]
pub extern "C" fn klatos_pdev_bendera() -> i32 {
    bump(Counter::ProdBendera);
    mount_bendera_admisi()
}

/// Lihat [`devctl_cari_admisi`].
#[no_mangle]
pub extern "C" fn klatos_devctl_cari(ditemukan: u32) -> i32 {
    bump(Counter::ProdCari);
    devctl_cari_admisi(ditemukan)
}

/// Lihat [`devctl_offset_admisi`].
#[no_mangle]
pub extern "C" fn klatos_devctl_offset(offset: u64) -> i32 {
    bump(Counter::ProdOffset);
    devctl_offset_admisi(offset)
}

/// Lihat [`devctl_ukuran_admisi`].
#[no_mangle]
pub extern "C" fn klatos_devctl_ukuran(sisa: u64) -> i32 {
    bump(Counter::ProdUkuran);
    devctl_ukuran_admisi(sisa)
}

/// Lihat [`devctl_versi_admisi`].
#[no_mangle]
pub extern "C" fn klatos_devctl_versi(cocok: u32) -> i32 {
    bump(Counter::ProdVersi);
    devctl_versi_admisi(cocok)
}

/// Lihat [`devctl_operasi_admisi`].
#[no_mangle]
pub extern "C" fn klatos_devctl_operasi(dikenal: u32) -> i32 {
    bump(Counter::ProdOperasi);
    devctl_operasi_admisi(dikenal)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_pdev_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_pdev_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiPasangReport` di `Jowo/Kontrak/KlatSeam94.h`.
#[repr(C)]
pub struct KlatosAdmisiPasangReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub pasang_checks: u32,
    pub pasang_passed: u32,
    pub perangkat_checks: u32,
    pub perangkat_passed: u32,
    pub prod_bendera: u32,
    pub prod_cari: u32,
    pub prod_offset: u32,
    pub prod_ukuran: u32,
    pub prod_versi: u32,
    pub prod_operasi: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Pasang,
    Perangkat,
}

struct Matrix {
    checks: u32,
    passed: u32,
    pasang_checks: u32,
    pasang_passed: u32,
    perangkat_checks: u32,
    perangkat_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            pasang_checks: 0,
            pasang_passed: 0,
            perangkat_checks: 0,
            perangkat_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Pasang => {
                self.pasang_checks += 1;
                if lulus {
                    self.pasang_passed += 1;
                }
            }
            Group::Perangkat => {
                self.perangkat_checks += 1;
                if lulus {
                    self.perangkat_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **12**.
pub const DIFF_EXPECTED: u32 = 12;

fn run_matrix() -> KlatosAdmisiPasangReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5044_4556, Group::Pasang);
    m.catat(ENODEV == 19 && EINVAL == 22 && EOVERFLOW == 36, Group::Pasang);

    // --- kasus utama ------------------------------------------------------
    m.catat(mount_bendera_admisi() == -EINVAL, Group::Pasang);

    m.catat(devctl_cari_admisi(0) == -ENODEV, Group::Perangkat);
    m.catat(devctl_cari_admisi(1) == 0, Group::Perangkat);
    m.catat(devctl_offset_admisi(0) == 0, Group::Perangkat);
    m.catat(devctl_offset_admisi(8) == -EINVAL, Group::Perangkat);
    m.catat(devctl_ukuran_admisi(0) == 0, Group::Perangkat);
    m.catat(devctl_ukuran_admisi(3) == -EOVERFLOW, Group::Perangkat);
    m.catat(devctl_versi_admisi(0) == -EINVAL, Group::Perangkat);
    m.catat(devctl_versi_admisi(1) == 0, Group::Perangkat);
    m.catat(devctl_operasi_admisi(0) == -EINVAL, Group::Perangkat);
    m.catat(devctl_operasi_admisi(1) == 0, Group::Perangkat);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // EOVERFLOW bukan EINVAL pada ukuran tak selaras.
        m.catat(devctl_ukuran_admisi(3) == -EOVERFLOW, Group::Perangkat);
        m.catat(devctl_ukuran_salah(3) == -EINVAL, Group::Perangkat);
        m.catat(devctl_ukuran_admisi(3) != devctl_ukuran_salah(3), Group::Perangkat);
        m.catat(devctl_ukuran_admisi(0) == devctl_ukuran_salah(0), Group::Perangkat);

        // ENODEV bukan ENOENT pada perangkat hilang.
        m.catat(devctl_cari_admisi(0) == -ENODEV, Group::Perangkat);
        m.catat(devctl_cari_salah(0) == -2, Group::Perangkat);
        m.catat(devctl_cari_admisi(0) != devctl_cari_salah(0), Group::Perangkat);
        m.catat(devctl_cari_admisi(1) == devctl_cari_salah(1), Group::Perangkat);

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if mount_bendera_admisi() != -EINVAL {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Pasang);
    }

    // --- diferensial produksi vs oracle (13 kasus) ------------------------
    macro_rules! ukur {
        ($g:expr, $p:expr, $n:expr) => {{
            if $p != $n {
                beda += 1;
            }
            diukur += 1;
            let _ = $g;
        }};
    }

    ukur!(Group::Pasang, mount_bendera_admisi(), -EINVAL);
    for v in 0..2u32 {
        ukur!(Group::Perangkat, devctl_cari_admisi(v), naif_devctl_cari_admisi(v));
        ukur!(Group::Perangkat, devctl_versi_admisi(v), naif_devctl_versi_admisi(v));
        ukur!(
            Group::Perangkat,
            devctl_operasi_admisi(v),
            naif_devctl_operasi_admisi(v)
        );
        ukur!(
            Group::Perangkat,
            devctl_offset_admisi(v as u64),
            naif_devctl_offset_admisi(v as u64)
        );
        ukur!(
            Group::Perangkat,
            devctl_ukuran_admisi(v as u64),
            naif_devctl_ukuran_admisi(v as u64)
        );
    }
    ukur!(
        Group::Perangkat,
        devctl_ukuran_admisi(3),
        naif_devctl_ukuran_admisi(3)
    );

    m.catat(diukur == DIFF_EXPECTED, Group::Pasang);
    m.catat(beda == 0, Group::Pasang);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiPasangReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        pasang_checks: m.pasang_checks,
        pasang_passed: m.pasang_passed,
        perangkat_checks: m.perangkat_checks,
        perangkat_passed: m.perangkat_passed,
        prod_bendera: klatos_pdev_counter(Counter::ProdBendera as u32),
        prod_cari: klatos_pdev_counter(Counter::ProdCari as u32),
        prod_offset: klatos_pdev_counter(Counter::ProdOffset as u32),
        prod_ukuran: klatos_pdev_counter(Counter::ProdUkuran as u32),
        prod_versi: klatos_pdev_counter(Counter::ProdVersi as u32),
        prod_operasi: klatos_pdev_counter(Counter::ProdOperasi as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas pemasangan + validasi perangkat.
#[no_mangle]
pub extern "C" fn klatos_pdev_selftest() -> KlatosAdmisiPasangReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5044_4556);
        assert!(ENODEV == 19);
        assert!(EINVAL == 22);
        assert!(EOVERFLOW == 36);
        assert!(Counter::ProdBendera as usize == 0);
        assert!(Counter::ProdOperasi as usize == 5);
        assert!(N_COUNTER == 6);
        assert!(DIFF_EXPECTED == 12);

        assert!(mount_bendera_admisi() == -EINVAL);
        assert!(devctl_cari_admisi(0) == -ENODEV);
        assert!(devctl_ukuran_admisi(3) == -EOVERFLOW);

        // 8 (tag) + 15*4 (u32) + 4 (status) = 68 → 72 dengan padding 8-byte.
        assert!(core::mem::size_of::<KlatosAdmisiPasangReport>() == 72);
        assert!(core::mem::align_of::<KlatosAdmisiPasangReport>() == 8);
    };
}
