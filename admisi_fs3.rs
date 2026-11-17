// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS BERKAS-SISTEM, BATCH 3** (FATFS + ISO9660FS).


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `ENOENT` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 2).
pub const ENOENT: i32 = 2;
/// `EIO` — indeks 5.
pub const EIO: i32 = 5;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `ENOSPC` — indeks 28.
pub const ENOSPC: i32 = 28;
/// `EROFS` — indeks 30.
pub const EROFS: i32 = 30;
/// `ENOTSUP` — indeks 74.
pub const ENOTSUP: i32 = 74;

/// Tag laporan; cermin `KlatosAdmisiFs3Report` di `Jowo/Kontrak/KlatSeam90.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4653_4233; // "KLATFSB3"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_fs3_fat_superblok`.
    ProdFatSuperblok = 0,
    /// `klatos_fs3_fat_bpb32`.
    ProdFatBpb32 = 1,
    /// `klatos_fs3_fat_fsinfo`.
    ProdFatFsinfo = 2,
    /// `klatos_fs3_fat_klaster_habis`.
    ProdFatKlasterHabis = 3,
    /// `klatos_fs3_fat_klaster`.
    ProdFatKlaster = 4,
    /// `klatos_fs3_fat_traverse`.
    ProdFatTraverse = 5,
    /// `klatos_fs3_fat_cari`.
    ProdFatCari = 6,
    /// `klatos_fs3_fat_hapus`.
    ProdFatHapus = 7,
    /// `klatos_fs3_fat_buat_anak`.
    ProdFatBuatAnak = 8,
    /// `klatos_fs3_fat_tak_didukung`.
    ProdFatTakDidukung = 9,
    /// `klatos_fs3_iso_ro`.
    ProdIsoRo = 10,
    /// `klatos_fs3_iso_struktur`.
    ProdIsoStruktur = 11,
    /// `klatos_fs3_iso_cari`.
    ProdIsoCari = 12,
}

const N_COUNTER: usize = 13;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// `FATFS::initialize_while_locked` — signature boot sector.
/// Transkripsi `FATFS/FileSystem.cpp:169-172`.
#[inline]
pub const fn fat_superblok_admisi(signature_ok: u32) -> i32 {
    if signature_ok == 0 {
        return -EINVAL;
    }
    0
}

/// `FATFS::initialize_while_locked` — FAT32 wajib punya BPB DOS7.
/// Transkripsi `FATFS/FileSystem.cpp:226-229`.
#[inline]
pub const fn fat_bpb32_admisi(ada_dos7: u32) -> i32 {
    if ada_dos7 == 0 {
        return -EINVAL;
    }
    0
}

/// `FATFS::initialize_while_locked` — signature struktur FSInfo.
/// Transkripsi `FATFS/FileSystem.cpp:254-257`.
#[inline]
pub const fn fat_fsinfo_admisi(signature_ok: u32) -> i32 {
    if signature_ok == 0 {
        return -EINVAL;
    }
    0
}

/// `FATFS::allocate_cluster` — seluruh klaster habis. Transkripsi `:453`.
#[inline]
pub const fn fat_klaster_habis_admisi() -> i32 {
    -ENOSPC
}

/// `FATInode::traverse` — klaster entri tidak valid (0/1 dengan ukuran > 0).
/// Transkripsi `FATFS/Inode.cpp:277-280`.
#[inline]
pub const fn fat_klaster_admisi(klaster_valid: u32) -> i32 {
    if klaster_valid == 0 {
        return -EINVAL;
    }
    0
}

/// `FATInode::traverse` — tidak ada entri yang diterima callback.
/// Transkripsi `FATFS/Inode.cpp:298`.
#[inline]
pub const fn fat_traverse_admisi(ketemu: u32) -> i32 {
    if ketemu == 0 {
        return -EINVAL;
    }
    0
}

/// `FATInode::lookup` — nama tidak ditemukan. Transkripsi `:580-582`.
#[inline]
pub const fn fat_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENOENT;
    }
    0
}

/// `FATInode::remove_child_impl` — tidak ada entri yang cocok.
/// Transkripsi `:822-825`.
#[inline]
pub const fn fat_hapus_admisi(ketemu: u32) -> i32 {
    if ketemu == 0 {
        return -EINVAL;
    }
    0
}

/// `FATInode::create_child` — mode bukan direktori/berkas biasa.
/// Transkripsi `:650-652`.
#[inline]
pub const fn fat_buat_anak_admisi(mode_ok: u32) -> i32 {
    if mode_ok == 0 {
        return -ENOTSUP;
    }
    0
}

/// Operasi FAT yang tidak didukung: `add_child`, `chmod`, `chown`.
/// Satu verdict, tiga pemanggil. Transkripsi `:745`, `:836`, `:841`.
#[inline]
pub const fn fat_tak_didukung_admisi() -> i32 {
    -ENOTSUP
}

/// `ISO9660FS::rename` + delapan metode tulis `ISO9660Inode` — berkas-sistem
/// read-only. Satu verdict, sembilan pemanggil.
/// Transkripsi `ISO9660FS/FileSystem.cpp:61` dan `ISO9660FS/Inode.cpp:113-148`.
#[inline]
pub const fn iso9660_ro_admisi() -> i32 {
    -EROFS
}

/// Struktur ISO9660 tidak valid (magic descriptor, tipe descriptor tak
/// dikenal, primary volume hilang, panjang direktori tidak selaras blok).
/// Satu verdict, enam pemanggil. Transkripsi
/// `ISO9660FS/FileSystem.cpp:115,134,143,154,165,254`.
#[inline]
pub const fn iso9660_struktur_admisi(valid: u32) -> i32 {
    if valid == 0 {
        return -EIO;
    }
    0
}

/// `ISO9660Inode::lookup` — nama tidak ditemukan. Transkripsi `:101-103`.
#[inline]
pub const fn iso9660_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENOENT;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_fat_superblok_admisi(ok: u32) -> i32 {
    match ok {
        1 => 0,
        _ => -EINVAL,
    }
}
fn naif_fat_bpb32_admisi(ok: u32) -> i32 {
    match ok {
        0 => -EINVAL,
        _ => 0,
    }
}
fn naif_fat_fsinfo_admisi(ok: u32) -> i32 {
    match ok {
        1 => 0,
        _ => -EINVAL,
    }
}
fn naif_fat_klaster_admisi(ok: u32) -> i32 {
    match ok {
        1 => 0,
        _ => -EINVAL,
    }
}
fn naif_fat_traverse_admisi(ketemu: u32) -> i32 {
    match ketemu {
        0 => -EINVAL,
        _ => 0,
    }
}
fn naif_fat_cari_admisi(ditemukan: u32) -> i32 {
    match ditemukan {
        0 => -ENOENT,
        _ => 0,
    }
}
fn naif_fat_hapus_admisi(ketemu: u32) -> i32 {
    match ketemu {
        1 => 0,
        _ => -EINVAL,
    }
}
fn naif_fat_buat_anak_admisi(ok: u32) -> i32 {
    match ok {
        1 => 0,
        _ => -ENOTSUP,
    }
}
fn naif_iso9660_struktur_admisi(valid: u32) -> i32 {
    match valid {
        1 => 0,
        _ => -EIO,
    }
}
fn naif_iso9660_cari_admisi(ditemukan: u32) -> i32 {
    match ditemukan {
        0 => -ENOENT,
        _ => 0,
    }
}

/// Kontrol NEGATIF: `fat_buat_anak_admisi` yang salah mengembalikan `EINVAL`
/// (bukan `ENOTSUP`). Sengaja SALAH; berdaya pada `mode_ok == 0`.
fn fat_buat_anak_salah(mode_ok: u32) -> i32 {
    if mode_ok == 0 {
        return -EINVAL;
    }
    0
}

/// Kontrol NEGATIF: `iso9660_struktur_admisi` yang salah mengembalikan `EINVAL`
/// (bukan `EIO`). Sengaja SALAH; berdaya pada `valid == 0`.
fn iso9660_struktur_salah(valid: u32) -> i32 {
    if valid == 0 {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`fat_superblok_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_superblok(signature_ok: u32) -> i32 {
    bump(Counter::ProdFatSuperblok);
    fat_superblok_admisi(signature_ok)
}

/// Lihat [`fat_bpb32_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_bpb32(ada_dos7: u32) -> i32 {
    bump(Counter::ProdFatBpb32);
    fat_bpb32_admisi(ada_dos7)
}

/// Lihat [`fat_fsinfo_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_fsinfo(signature_ok: u32) -> i32 {
    bump(Counter::ProdFatFsinfo);
    fat_fsinfo_admisi(signature_ok)
}

/// Lihat [`fat_klaster_habis_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_klaster_habis() -> i32 {
    bump(Counter::ProdFatKlasterHabis);
    fat_klaster_habis_admisi()
}

/// Lihat [`fat_klaster_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_klaster(klaster_valid: u32) -> i32 {
    bump(Counter::ProdFatKlaster);
    fat_klaster_admisi(klaster_valid)
}

/// Lihat [`fat_traverse_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_traverse(ketemu: u32) -> i32 {
    bump(Counter::ProdFatTraverse);
    fat_traverse_admisi(ketemu)
}

/// Lihat [`fat_cari_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_cari(ditemukan: u32) -> i32 {
    bump(Counter::ProdFatCari);
    fat_cari_admisi(ditemukan)
}

/// Lihat [`fat_hapus_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_hapus(ketemu: u32) -> i32 {
    bump(Counter::ProdFatHapus);
    fat_hapus_admisi(ketemu)
}

/// Lihat [`fat_buat_anak_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_buat_anak(mode_ok: u32) -> i32 {
    bump(Counter::ProdFatBuatAnak);
    fat_buat_anak_admisi(mode_ok)
}

/// Lihat [`fat_tak_didukung_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_fat_tak_didukung() -> i32 {
    bump(Counter::ProdFatTakDidukung);
    fat_tak_didukung_admisi()
}

/// Lihat [`iso9660_ro_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_iso_ro() -> i32 {
    bump(Counter::ProdIsoRo);
    iso9660_ro_admisi()
}

/// Lihat [`iso9660_struktur_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_iso_struktur(valid: u32) -> i32 {
    bump(Counter::ProdIsoStruktur);
    iso9660_struktur_admisi(valid)
}

/// Lihat [`iso9660_cari_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs3_iso_cari(ditemukan: u32) -> i32 {
    bump(Counter::ProdIsoCari);
    iso9660_cari_admisi(ditemukan)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_fs3_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_fs3_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiFs3Report` di `Jowo/Kontrak/KlatSeam90.h`.
#[repr(C)]
pub struct KlatosAdmisiFs3Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub fat_checks: u32,
    pub fat_passed: u32,
    pub iso_checks: u32,
    pub iso_passed: u32,
    pub prod_fat_superblok: u32,
    pub prod_fat_bpb32: u32,
    pub prod_fat_fsinfo: u32,
    pub prod_fat_klaster_habis: u32,
    pub prod_fat_klaster: u32,
    pub prod_fat_traverse: u32,
    pub prod_fat_cari: u32,
    pub prod_fat_hapus: u32,
    pub prod_fat_buat_anak: u32,
    pub prod_fat_tak_didukung: u32,
    pub prod_iso_ro: u32,
    pub prod_iso_struktur: u32,
    pub prod_iso_cari: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Fat,
    Iso,
}

struct Matrix {
    checks: u32,
    passed: u32,
    fat_checks: u32,
    fat_passed: u32,
    iso_checks: u32,
    iso_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            fat_checks: 0,
            fat_passed: 0,
            iso_checks: 0,
            iso_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Fat => {
                self.fat_checks += 1;
                if lulus {
                    self.fat_passed += 1;
                }
            }
            Group::Iso => {
                self.iso_checks += 1;
                if lulus {
                    self.iso_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **23**.
pub const DIFF_EXPECTED: u32 = 23;

fn run_matrix() -> KlatosAdmisiFs3Report {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4653_4233, Group::Fat);
    m.catat(
        ENOENT == 2 && EIO == 5 && EINVAL == 22 && ENOSPC == 28 && EROFS == 30 && ENOTSUP == 74,
        Group::Fat,
    );

    // --- kasus utama ------------------------------------------------------
    m.catat(fat_superblok_admisi(0) == -EINVAL, Group::Fat);
    m.catat(fat_superblok_admisi(1) == 0, Group::Fat);
    m.catat(fat_bpb32_admisi(0) == -EINVAL, Group::Fat);
    m.catat(fat_bpb32_admisi(1) == 0, Group::Fat);
    m.catat(fat_fsinfo_admisi(0) == -EINVAL, Group::Fat);
    m.catat(fat_klaster_habis_admisi() == -ENOSPC, Group::Fat);
    m.catat(fat_klaster_admisi(0) == -EINVAL, Group::Fat);
    m.catat(fat_traverse_admisi(0) == -EINVAL, Group::Fat);
    m.catat(fat_cari_admisi(0) == -ENOENT, Group::Fat);
    m.catat(fat_hapus_admisi(0) == -EINVAL, Group::Fat);
    m.catat(fat_buat_anak_admisi(0) == -ENOTSUP, Group::Fat);
    m.catat(fat_buat_anak_admisi(1) == 0, Group::Fat);
    m.catat(fat_tak_didukung_admisi() == -ENOTSUP, Group::Fat);

    m.catat(iso9660_ro_admisi() == -EROFS, Group::Iso);
    m.catat(iso9660_struktur_admisi(0) == -EIO, Group::Iso);
    m.catat(iso9660_struktur_admisi(1) == 0, Group::Iso);
    m.catat(iso9660_cari_admisi(0) == -ENOENT, Group::Iso);
    m.catat(iso9660_cari_admisi(1) == 0, Group::Iso);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // ENOTSUP bukan EINVAL pada create_child mode buruk.
        m.catat(fat_buat_anak_admisi(0) == -ENOTSUP, Group::Fat);
        m.catat(fat_buat_anak_salah(0) == -EINVAL, Group::Fat);
        m.catat(fat_buat_anak_admisi(0) != fat_buat_anak_salah(0), Group::Fat);
        m.catat(fat_buat_anak_admisi(1) == fat_buat_anak_salah(1), Group::Fat);

        // EIO bukan EINVAL pada struktur ISO rusak.
        m.catat(iso9660_struktur_admisi(0) == -EIO, Group::Iso);
        m.catat(iso9660_struktur_salah(0) == -EINVAL, Group::Iso);
        m.catat(iso9660_struktur_admisi(0) != iso9660_struktur_salah(0), Group::Iso);
        m.catat(iso9660_struktur_admisi(1) == iso9660_struktur_salah(1), Group::Iso);

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if iso9660_ro_admisi() != -EROFS {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Iso);
    }

    // --- diferensial produksi vs oracle (23 kasus) ------------------------
    macro_rules! ukur {
        ($g:expr, $p:expr, $n:expr) => {{
            if $p != $n {
                beda += 1;
            }
            diukur += 1;
            let _ = $g;
        }};
    }

    for v in 0..2u32 {
        ukur!(Group::Fat, fat_superblok_admisi(v), naif_fat_superblok_admisi(v));
        ukur!(Group::Fat, fat_bpb32_admisi(v), naif_fat_bpb32_admisi(v));
        ukur!(Group::Fat, fat_fsinfo_admisi(v), naif_fat_fsinfo_admisi(v));
        ukur!(Group::Fat, fat_klaster_admisi(v), naif_fat_klaster_admisi(v));
        ukur!(Group::Fat, fat_traverse_admisi(v), naif_fat_traverse_admisi(v));
        ukur!(Group::Fat, fat_cari_admisi(v), naif_fat_cari_admisi(v));
        ukur!(Group::Fat, fat_hapus_admisi(v), naif_fat_hapus_admisi(v));
        ukur!(Group::Fat, fat_buat_anak_admisi(v), naif_fat_buat_anak_admisi(v));
        ukur!(Group::Iso, iso9660_struktur_admisi(v), naif_iso9660_struktur_admisi(v));
        ukur!(Group::Iso, iso9660_cari_admisi(v), naif_iso9660_cari_admisi(v));
    }
    // Konstanta: dipanggil sekali.
    ukur!(Group::Fat, fat_klaster_habis_admisi(), -ENOSPC);
    ukur!(Group::Fat, fat_tak_didukung_admisi(), -ENOTSUP);
    ukur!(Group::Iso, iso9660_ro_admisi(), -EROFS);

    m.catat(diukur == DIFF_EXPECTED, Group::Fat);
    m.catat(beda == 0, Group::Fat);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiFs3Report {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        fat_checks: m.fat_checks,
        fat_passed: m.fat_passed,
        iso_checks: m.iso_checks,
        iso_passed: m.iso_passed,
        prod_fat_superblok: klatos_fs3_counter(Counter::ProdFatSuperblok as u32),
        prod_fat_bpb32: klatos_fs3_counter(Counter::ProdFatBpb32 as u32),
        prod_fat_fsinfo: klatos_fs3_counter(Counter::ProdFatFsinfo as u32),
        prod_fat_klaster_habis: klatos_fs3_counter(Counter::ProdFatKlasterHabis as u32),
        prod_fat_klaster: klatos_fs3_counter(Counter::ProdFatKlaster as u32),
        prod_fat_traverse: klatos_fs3_counter(Counter::ProdFatTraverse as u32),
        prod_fat_cari: klatos_fs3_counter(Counter::ProdFatCari as u32),
        prod_fat_hapus: klatos_fs3_counter(Counter::ProdFatHapus as u32),
        prod_fat_buat_anak: klatos_fs3_counter(Counter::ProdFatBuatAnak as u32),
        prod_fat_tak_didukung: klatos_fs3_counter(Counter::ProdFatTakDidukung as u32),
        prod_iso_ro: klatos_fs3_counter(Counter::ProdIsoRo as u32),
        prod_iso_struktur: klatos_fs3_counter(Counter::ProdIsoStruktur as u32),
        prod_iso_cari: klatos_fs3_counter(Counter::ProdIsoCari as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas berkas-sistem batch 3.
#[no_mangle]
pub extern "C" fn klatos_fs3_selftest() -> KlatosAdmisiFs3Report {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4653_4233);
        assert!(ENOENT == 2);
        assert!(EIO == 5);
        assert!(EINVAL == 22);
        assert!(ENOSPC == 28);
        assert!(EROFS == 30);
        assert!(ENOTSUP == 74);
        assert!(Counter::ProdFatSuperblok as usize == 0);
        assert!(Counter::ProdIsoCari as usize == 12);
        assert!(N_COUNTER == 13);
        assert!(DIFF_EXPECTED == 23);

        assert!(fat_buat_anak_admisi(0) == -ENOTSUP);
        assert!(iso9660_ro_admisi() == -EROFS);
        assert!(iso9660_struktur_admisi(0) == -EIO);

        // 8 (tag) + 20*4 (u32) + 4 (status) = 92 → 96 dengan padding 8-byte.
        assert!(core::mem::size_of::<KlatosAdmisiFs3Report>() == 96);
        assert!(core::mem::align_of::<KlatosAdmisiFs3Report>() == 8);
    };
}
