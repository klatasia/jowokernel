// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.87 / WS4 — **OTORITAS BERKAS-SISTEM, BATCH 1** (Ext2FS + InodeFile).


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `ENOENT` — indeks 2.
pub const ENOENT: i32 = 2;
/// `EIO` — indeks 5.
pub const EIO: i32 = 5;
/// `EBUSY` — indeks 16.
pub const EBUSY: i32 = 16;
/// `EEXIST` — indeks 17.
pub const EEXIST: i32 = 17;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `ENOSPC` — indeks 28.
pub const ENOSPC: i32 = 28;
/// `EROFS` — indeks 30.
pub const EROFS: i32 = 30;
/// `EMLINK` — indeks 31.
pub const EMLINK: i32 = 31;
/// `ENAMETOOLONG` — indeks 34.
pub const ENAMETOOLONG: i32 = 34;

/// `EXT2_NAME_LEN` — cermin `KlatKernel/Efek/Berkas/Ext2FS/ext2_types.h`.
pub const EXT2_NAME_LEN: u32 = 255;
/// Batas `i_links_count` Ext2 — cermin `Ext2FS/Inode.cpp:911`.
pub const EXT2_MAX_LINKS: u32 = 65535;

/// Jenis ioctl `InodeFile`.
pub const IOCTL_FIBMAP: u32 = 0;
/// Jenis ioctl `InodeFile`.
pub const IOCTL_FIONREAD: u32 = 1;
/// Jenis ioctl `InodeFile` tak dikenal.
pub const IOCTL_LAIN: u32 = 2;

/// Tag laporan; cermin `KlatosAdmisiFs1Report` di `Jowo/Kontrak/KlatSeam88.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4653_4131; // "KLATFSA1"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_fs1_ext2_nama`.
    ProdExt2Nama = 0,
    /// `klatos_fs1_ext2_tambah_anak`.
    ProdExt2TambahAnak = 1,
    /// `klatos_fs1_ext2_cari`.
    ProdExt2Cari = 2,
    /// `klatos_fs1_ext2_tulis`.
    ProdExt2Tulis = 3,
    /// `klatos_fs1_ext2_waktu`.
    ProdExt2Waktu = 4,
    /// `klatos_fs1_ext2_link`.
    ProdExt2Link = 5,
    /// `klatos_fs1_ext2_resize`.
    ProdExt2Resize = 6,
    /// `klatos_fs1_ext2_induk`.
    ProdExt2Induk = 7,
    /// `klatos_fs1_ext2_superblok`.
    ProdExt2Superblok = 8,
    /// `klatos_fs1_ext2_grup`.
    ProdExt2Grup = 9,
    /// `klatos_fs1_ext2_blok_inode`.
    ProdExt2BlokInode = 10,
    /// `klatos_fs1_ext2_bitmap_indeks`.
    ProdExt2BitmapIndeks = 11,
    /// `klatos_fs1_ext2_bitmap_keadaan`.
    ProdExt2BitmapKeadaan = 12,
    /// `klatos_fs1_ext2_unmount`.
    ProdExt2Unmount = 13,
    /// `klatos_fs1_ext2_cache`.
    ProdExt2Cache = 14,
    /// `klatos_fs1_inodefile_ioctl`.
    ProdInodeFileIoctl = 15,
}

const N_COUNTER: usize = 16;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// `Ext2FSInode::add_child` — panjang nama. Transkripsi `:760-761`.
#[inline]
pub const fn ext2_nama_admisi(panjang: u32, maks: u32) -> i32 {
    if panjang > maks {
        return -ENAMETOOLONG;
    }
    0
}

/// `Ext2FSInode::add_child` — bentrok nama. Transkripsi `:768-769`.
#[inline]
pub const fn ext2_tambah_anak_admisi(nama_bentrok: u32) -> i32 {
    if nama_bentrok != 0 {
        return -EEXIST;
    }
    0
}

/// `Ext2FSInode::remove_child_impl`/`lookup` — nama tidak ditemukan.
/// Transkripsi `:797-798` dan `:864-866`.
#[inline]
pub const fn ext2_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENOENT;
    }
    0
}

/// `update_timestamps`/`increment_link_count`/`decrement_link_count` —
/// berkas-sistem read-only. Transkripsi `:877-878`, `:909-910`, `:922-923`.
#[inline]
pub const fn ext2_tulis_admisi(readonly: u32) -> i32 {
    if readonly != 0 {
        return -EROFS;
    }
    0
}

/// `update_timestamps` — detik melebihi `i32::MAX` padahal inode tidak punya
/// field `_extra`. Transkripsi `:881-886` (tiga cek, satu verdict).
#[inline]
pub const fn ext2_waktu_admisi(detik_besar: u32, muat_extra: u32) -> i32 {
    if detik_besar != 0 && muat_extra == 0 {
        return -EINVAL;
    }
    0
}

/// `increment_link_count` — read-only menang atas batas link.
/// Transkripsi `:909-913`.
#[inline]
pub const fn ext2_link_admisi(readonly: u32, links: u32, maks: u32) -> i32 {
    if readonly != 0 {
        return -EROFS;
    }
    if links == maks {
        return -EMLINK;
    }
    0
}

/// `Ext2FSInode::resize` — ukuran melebihi 32-bit tanpa fitur `FileSize64bits`.
/// Transkripsi `:544-546`.
#[inline]
pub const fn ext2_resize_admisi(ukuran_besar: u32, fitur_64bit: u32) -> i32 {
    if fitur_64bit == 0 && ukuran_besar != 0 {
        return -ENOSPC;
    }
    0
}

/// `Ext2FS::create_inode` — induk tanpa link. Transkripsi `:596-598`.
#[inline]
pub const fn ext2_induk_admisi(links_induk: u32) -> i32 {
    if links_induk == 0 {
        return -ENOENT;
    }
    0
}

/// `Ext2FS::initialize_while_locked` — magic superblok. Transkripsi `:151-154`.
#[inline]
pub const fn ext2_superblok_admisi(magic_ok: u32) -> i32 {
    if magic_ok == 0 {
        return -EINVAL;
    }
    0
}

/// `Ext2FS::initialize_while_locked` — nol block group. Transkripsi `:182-185`.
#[inline]
pub const fn ext2_grup_admisi(jumlah_grup: u32) -> i32 {
    if jumlah_grup == 0 {
        return -EINVAL;
    }
    0
}

/// `find_block_containing_inode` gagal. Transkripsi `:319-320`, `:819-820`,
/// `:852-853`.
#[inline]
pub const fn ext2_blok_inode_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -EINVAL;
    }
    0
}

/// `Ext2FS::get_inode_allocation_state` — indeks inode 0. Transkripsi `:485-486`.
#[inline]
pub const fn ext2_bitmap_indeks_admisi(indeks: u32) -> i32 {
    if indeks == 0 {
        return -EINVAL;
    }
    0
}

/// `Ext2FS::update_bitmap_block` — keadaan bitmap sudah seperti yang diminta.
/// Transkripsi `:498-501`.
#[inline]
pub const fn ext2_bitmap_keadaan_admisi(keadaan_sama: u32) -> i32 {
    if keadaan_sama != 0 {
        return -EIO;
    }
    0
}

/// `Ext2FS::prepare_to_unmount` — masih ada inode dipakai. Transkripsi `:692-693`.
#[inline]
pub const fn ext2_unmount_admisi(sibuk: u32) -> i32 {
    if sibuk != 0 {
        return -EBUSY;
    }
    0
}

/// `Ext2FS::get_inode` — entri cache ada tetapi bernilai null (inode kosong).
/// Transkripsi `:835-838`.
#[inline]
pub const fn ext2_cache_admisi(di_cache: u32, entri_null: u32) -> i32 {
    if di_cache != 0 && entri_null != 0 {
        return -ENOENT;
    }
    0
}

/// `InodeFile::ioctl` — verdict per jenis permintaan. Transkripsi `:51-84`:
///
/// ```cpp
/// case FIBMAP:
///     if (!is_superuser()) return EPERM;      // EPERM menang atas EINVAL
///     ...
///     if (block_number < 0) return EINVAL;
/// case FIONREAD: ...;
/// default: return EINVAL;
/// ```
#[inline]
pub const fn inodefile_ioctl_admisi(jenis: u32, superuser: u32, blok_negatif: u32) -> i32 {
    if jenis == IOCTL_FIBMAP {
        if superuser == 0 {
            return -EPERM;
        }
        if blok_negatif != 0 {
            return -EINVAL;
        }
        return 0;
    }
    if jenis == IOCTL_FIONREAD {
        return 0;
    }
    -EINVAL
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_ext2_nama_admisi(panjang: u32, maks: u32) -> i32 {
    if panjang <= maks {
        0
    } else {
        -ENAMETOOLONG
    }
}

fn naif_ext2_tambah_anak_admisi(nama_bentrok: u32) -> i32 {
    match nama_bentrok {
        0 => 0,
        _ => -EEXIST,
    }
}

fn naif_ext2_cari_admisi(ditemukan: u32) -> i32 {
    match ditemukan {
        1 => 0,
        _ => -ENOENT,
    }
}

fn naif_ext2_tulis_admisi(readonly: u32) -> i32 {
    match readonly {
        0 => 0,
        _ => -EROFS,
    }
}

fn naif_ext2_waktu_admisi(detik_besar: u32, muat_extra: u32) -> i32 {
    if detik_besar == 1 && muat_extra == 0 {
        -EINVAL
    } else {
        0
    }
}

fn naif_ext2_link_admisi(readonly: u32, links: u32, maks: u32) -> i32 {
    let mut e = 0;
    if readonly == 1 {
        e = -EROFS;
    } else if links == maks {
        e = -EMLINK;
    }
    e
}

fn naif_ext2_resize_admisi(ukuran_besar: u32, fitur_64bit: u32) -> i32 {
    if fitur_64bit == 0 && ukuran_besar == 1 {
        -ENOSPC
    } else {
        0
    }
}

fn naif_ext2_induk_admisi(links_induk: u32) -> i32 {
    match links_induk {
        0 => -ENOENT,
        _ => 0,
    }
}

fn naif_ext2_superblok_admisi(magic_ok: u32) -> i32 {
    match magic_ok {
        0 => -EINVAL,
        _ => 0,
    }
}

fn naif_ext2_grup_admisi(jumlah_grup: u32) -> i32 {
    if jumlah_grup == 0 {
        -EINVAL
    } else {
        0
    }
}

fn naif_ext2_blok_inode_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 1 {
        0
    } else {
        -EINVAL
    }
}

fn naif_ext2_bitmap_indeks_admisi(indeks: u32) -> i32 {
    if indeks != 0 {
        0
    } else {
        -EINVAL
    }
}

fn naif_ext2_bitmap_keadaan_admisi(keadaan_sama: u32) -> i32 {
    match keadaan_sama {
        1 => -EIO,
        _ => 0,
    }
}

fn naif_ext2_unmount_admisi(sibuk: u32) -> i32 {
    if sibuk == 1 {
        -EBUSY
    } else {
        0
    }
}

fn naif_ext2_cache_admisi(di_cache: u32, entri_null: u32) -> i32 {
    if di_cache == 1 && entri_null == 1 {
        -ENOENT
    } else {
        0
    }
}

fn naif_inodefile_ioctl_admisi(jenis: u32, superuser: u32, blok_negatif: u32) -> i32 {
    match jenis {
        IOCTL_FIBMAP => {
            if superuser != 1 {
                -EPERM
            } else if blok_negatif == 1 {
                -EINVAL
            } else {
                0
            }
        }
        IOCTL_FIONREAD => 0,
        _ => -EINVAL,
    }
}

/// Kontrol NEGATIF: `ext2_link_admisi` yang MENJATUHKAN presedensi `EROFS`.
/// Sengaja SALAH; berdaya pada `readonly == 1 && links == maks`.
fn ext2_link_salah(readonly: u32, links: u32, maks: u32) -> i32 {
    let _ = readonly;
    if links == maks {
        return -EMLINK;
    }
    0
}

/// Kontrol NEGATIF: `inodefile_ioctl_admisi` yang MENJATUHKAN presedensi
/// `EPERM` pada FIBMAP. Sengaja SALAH; berdaya pada `FIBMAP && !superuser`.
fn inodefile_ioctl_salah(jenis: u32, superuser: u32, blok_negatif: u32) -> i32 {
    if jenis == IOCTL_FIBMAP {
        if blok_negatif != 0 {
            return -EINVAL;
        }
        let _ = superuser;
        return 0;
    }
    if jenis == IOCTL_FIONREAD {
        return 0;
    }
    -EINVAL
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`ext2_nama_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_nama(panjang: u32, maks: u32) -> i32 {
    bump(Counter::ProdExt2Nama);
    ext2_nama_admisi(panjang, maks)
}

/// Lihat [`ext2_tambah_anak_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_tambah_anak(nama_bentrok: u32) -> i32 {
    bump(Counter::ProdExt2TambahAnak);
    ext2_tambah_anak_admisi(nama_bentrok)
}

/// Lihat [`ext2_cari_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_cari(ditemukan: u32) -> i32 {
    bump(Counter::ProdExt2Cari);
    ext2_cari_admisi(ditemukan)
}

/// Lihat [`ext2_tulis_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_tulis(readonly: u32) -> i32 {
    bump(Counter::ProdExt2Tulis);
    ext2_tulis_admisi(readonly)
}

/// Lihat [`ext2_waktu_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_waktu(detik_besar: u32, muat_extra: u32) -> i32 {
    bump(Counter::ProdExt2Waktu);
    ext2_waktu_admisi(detik_besar, muat_extra)
}

/// Lihat [`ext2_link_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_link(readonly: u32, links: u32, maks: u32) -> i32 {
    bump(Counter::ProdExt2Link);
    ext2_link_admisi(readonly, links, maks)
}

/// Lihat [`ext2_resize_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_resize(ukuran_besar: u32, fitur_64bit: u32) -> i32 {
    bump(Counter::ProdExt2Resize);
    ext2_resize_admisi(ukuran_besar, fitur_64bit)
}

/// Lihat [`ext2_induk_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_induk(links_induk: u32) -> i32 {
    bump(Counter::ProdExt2Induk);
    ext2_induk_admisi(links_induk)
}

/// Lihat [`ext2_superblok_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_superblok(magic_ok: u32) -> i32 {
    bump(Counter::ProdExt2Superblok);
    ext2_superblok_admisi(magic_ok)
}

/// Lihat [`ext2_grup_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_grup(jumlah_grup: u32) -> i32 {
    bump(Counter::ProdExt2Grup);
    ext2_grup_admisi(jumlah_grup)
}

/// Lihat [`ext2_blok_inode_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_blok_inode(ditemukan: u32) -> i32 {
    bump(Counter::ProdExt2BlokInode);
    ext2_blok_inode_admisi(ditemukan)
}

/// Lihat [`ext2_bitmap_indeks_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_bitmap_indeks(indeks: u32) -> i32 {
    bump(Counter::ProdExt2BitmapIndeks);
    ext2_bitmap_indeks_admisi(indeks)
}

/// Lihat [`ext2_bitmap_keadaan_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_bitmap_keadaan(keadaan_sama: u32) -> i32 {
    bump(Counter::ProdExt2BitmapKeadaan);
    ext2_bitmap_keadaan_admisi(keadaan_sama)
}

/// Lihat [`ext2_unmount_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_unmount(sibuk: u32) -> i32 {
    bump(Counter::ProdExt2Unmount);
    ext2_unmount_admisi(sibuk)
}

/// Lihat [`ext2_cache_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_ext2_cache(di_cache: u32, entri_null: u32) -> i32 {
    bump(Counter::ProdExt2Cache);
    ext2_cache_admisi(di_cache, entri_null)
}

/// Lihat [`inodefile_ioctl_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs1_inodefile_ioctl(jenis: u32, superuser: u32, blok_negatif: u32) -> i32 {
    bump(Counter::ProdInodeFileIoctl);
    inodefile_ioctl_admisi(jenis, superuser, blok_negatif)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_fs1_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_fs1_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiFs1Report` di `Jowo/Kontrak/KlatSeam88.h`.
#[repr(C)]
pub struct KlatosAdmisiFs1Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub ext2_checks: u32,
    pub ext2_passed: u32,
    pub file_checks: u32,
    pub file_passed: u32,
    pub prod_ext2_nama: u32,
    pub prod_ext2_tambah_anak: u32,
    pub prod_ext2_cari: u32,
    pub prod_ext2_tulis: u32,
    pub prod_ext2_waktu: u32,
    pub prod_ext2_link: u32,
    pub prod_ext2_resize: u32,
    pub prod_ext2_induk: u32,
    pub prod_ext2_superblok: u32,
    pub prod_ext2_grup: u32,
    pub prod_ext2_blok_inode: u32,
    pub prod_ext2_bitmap_indeks: u32,
    pub prod_ext2_bitmap_keadaan: u32,
    pub prod_ext2_unmount: u32,
    pub prod_ext2_cache: u32,
    pub prod_inodefile_ioctl: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Ext2,
    File,
}

struct Matrix {
    checks: u32,
    passed: u32,
    ext2_checks: u32,
    ext2_passed: u32,
    file_checks: u32,
    file_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            ext2_checks: 0,
            ext2_passed: 0,
            file_checks: 0,
            file_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Ext2 => {
                self.ext2_checks += 1;
                if lulus {
                    self.ext2_passed += 1;
                }
            }
            Group::File => {
                self.file_checks += 1;
                if lulus {
                    self.file_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **50**.
pub const DIFF_EXPECTED: u32 = 50;

fn run_matrix() -> KlatosAdmisiFs1Report {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4653_4131, Group::Ext2);
    m.catat(
        EPERM == 1 && ENOENT == 2 && EIO == 5 && EBUSY == 16 && EEXIST == 17
            && EINVAL == 22 && ENOSPC == 28 && EROFS == 30 && EMLINK == 31 && ENAMETOOLONG == 34,
        Group::Ext2,
    );
    m.catat(EXT2_NAME_LEN == 255 && EXT2_MAX_LINKS == 65535, Group::Ext2);

    // --- kasus utama ------------------------------------------------------
    m.catat(ext2_nama_admisi(255, EXT2_NAME_LEN) == 0, Group::Ext2);
    m.catat(ext2_nama_admisi(256, EXT2_NAME_LEN) == -ENAMETOOLONG, Group::Ext2);
    m.catat(ext2_tambah_anak_admisi(1) == -EEXIST, Group::Ext2);
    m.catat(ext2_cari_admisi(0) == -ENOENT, Group::Ext2);
    m.catat(ext2_tulis_admisi(1) == -EROFS, Group::Ext2);
    m.catat(ext2_waktu_admisi(1, 0) == -EINVAL, Group::Ext2);
    m.catat(ext2_waktu_admisi(1, 1) == 0, Group::Ext2);
    // EROFS menang atas EMLINK.
    m.catat(ext2_link_admisi(1, EXT2_MAX_LINKS, EXT2_MAX_LINKS) == -EROFS, Group::Ext2);
    m.catat(ext2_link_admisi(0, EXT2_MAX_LINKS, EXT2_MAX_LINKS) == -EMLINK, Group::Ext2);
    m.catat(ext2_resize_admisi(1, 0) == -ENOSPC, Group::Ext2);
    m.catat(ext2_induk_admisi(0) == -ENOENT, Group::Ext2);
    m.catat(ext2_superblok_admisi(0) == -EINVAL, Group::Ext2);
    m.catat(ext2_grup_admisi(0) == -EINVAL, Group::Ext2);
    m.catat(ext2_blok_inode_admisi(0) == -EINVAL, Group::Ext2);
    m.catat(ext2_bitmap_indeks_admisi(0) == -EINVAL, Group::Ext2);
    m.catat(ext2_bitmap_keadaan_admisi(1) == -EIO, Group::Ext2);
    m.catat(ext2_unmount_admisi(1) == -EBUSY, Group::Ext2);
    m.catat(ext2_cache_admisi(1, 1) == -ENOENT, Group::Ext2);
    m.catat(ext2_cache_admisi(1, 0) == 0, Group::Ext2);
    // ioctl: EPERM menang atas EINVAL pada FIBMAP.
    m.catat(inodefile_ioctl_admisi(IOCTL_FIBMAP, 0, 1) == -EPERM, Group::File);
    m.catat(inodefile_ioctl_admisi(IOCTL_FIBMAP, 1, 1) == -EINVAL, Group::File);
    m.catat(inodefile_ioctl_admisi(IOCTL_FIBMAP, 1, 0) == 0, Group::File);
    m.catat(inodefile_ioctl_admisi(IOCTL_FIONREAD, 0, 1) == 0, Group::File);
    m.catat(inodefile_ioctl_admisi(IOCTL_LAIN, 1, 0) == -EINVAL, Group::File);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // presedensi EROFS berdaya.
        m.catat(ext2_link_admisi(1, EXT2_MAX_LINKS, EXT2_MAX_LINKS) == -EROFS, Group::Ext2);
        m.catat(ext2_link_salah(1, EXT2_MAX_LINKS, EXT2_MAX_LINKS) == -EMLINK, Group::Ext2);
        m.catat(
            ext2_link_admisi(1, EXT2_MAX_LINKS, EXT2_MAX_LINKS) != ext2_link_salah(1, EXT2_MAX_LINKS, EXT2_MAX_LINKS),
            Group::Ext2,
        );
        m.catat(
            ext2_link_admisi(0, 0, EXT2_MAX_LINKS) == ext2_link_salah(0, 0, EXT2_MAX_LINKS),
            Group::Ext2,
        );

        // presedensi EPERM pada FIBMAP berdaya.
        m.catat(inodefile_ioctl_admisi(IOCTL_FIBMAP, 0, 0) == -EPERM, Group::File);
        m.catat(inodefile_ioctl_salah(IOCTL_FIBMAP, 0, 0) == 0, Group::File);
        m.catat(
            inodefile_ioctl_admisi(IOCTL_FIBMAP, 0, 0) != inodefile_ioctl_salah(IOCTL_FIBMAP, 0, 0),
            Group::File,
        );
        m.catat(
            inodefile_ioctl_admisi(IOCTL_FIBMAP, 1, 1) == inodefile_ioctl_salah(IOCTL_FIBMAP, 1, 1),
            Group::File,
        );

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if ext2_tulis_admisi(0) != 0 {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Ext2);
    }

    // --- diferensial produksi vs oracle (50 kasus) ------------------------
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
        ukur!(Group::Ext2, ext2_tambah_anak_admisi(v), naif_ext2_tambah_anak_admisi(v));
        ukur!(Group::Ext2, ext2_cari_admisi(v), naif_ext2_cari_admisi(v));
        ukur!(Group::Ext2, ext2_tulis_admisi(v), naif_ext2_tulis_admisi(v));
        ukur!(Group::Ext2, ext2_induk_admisi(v), naif_ext2_induk_admisi(v));
        ukur!(Group::Ext2, ext2_superblok_admisi(v), naif_ext2_superblok_admisi(v));
        ukur!(Group::Ext2, ext2_grup_admisi(v), naif_ext2_grup_admisi(v));
        ukur!(Group::Ext2, ext2_blok_inode_admisi(v), naif_ext2_blok_inode_admisi(v));
        ukur!(Group::Ext2, ext2_bitmap_indeks_admisi(v), naif_ext2_bitmap_indeks_admisi(v));
        ukur!(Group::Ext2, ext2_bitmap_keadaan_admisi(v), naif_ext2_bitmap_keadaan_admisi(v));
        ukur!(Group::Ext2, ext2_unmount_admisi(v), naif_ext2_unmount_admisi(v));
    }
    for v in 0..2u32 {
        ukur!(
            Group::Ext2,
            ext2_nama_admisi(v, EXT2_NAME_LEN),
            naif_ext2_nama_admisi(v, EXT2_NAME_LEN)
        );
    }
    for a in 0..2u32 {
        for b in 0..2u32 {
            ukur!(Group::Ext2, ext2_waktu_admisi(a, b), naif_ext2_waktu_admisi(a, b));
            ukur!(Group::Ext2, ext2_resize_admisi(a, b), naif_ext2_resize_admisi(a, b));
            ukur!(Group::Ext2, ext2_cache_admisi(a, b), naif_ext2_cache_admisi(a, b));
        }
    }
    for ro in 0..2u32 {
        for lk in 0..2u32 {
            let links = if lk == 1 { EXT2_MAX_LINKS } else { 0 };
            ukur!(
                Group::Ext2,
                ext2_link_admisi(ro, links, EXT2_MAX_LINKS),
                naif_ext2_link_admisi(ro, links, EXT2_MAX_LINKS)
            );
        }
    }
    for jenis in 0..3u32 {
        for su in 0..2u32 {
            for bn in 0..2u32 {
                ukur!(
                    Group::File,
                    inodefile_ioctl_admisi(jenis, su, bn),
                    naif_inodefile_ioctl_admisi(jenis, su, bn)
                );
            }
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Ext2);
    m.catat(beda == 0, Group::Ext2);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiFs1Report {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        ext2_checks: m.ext2_checks,
        ext2_passed: m.ext2_passed,
        file_checks: m.file_checks,
        file_passed: m.file_passed,
        prod_ext2_nama: klatos_fs1_counter(Counter::ProdExt2Nama as u32),
        prod_ext2_tambah_anak: klatos_fs1_counter(Counter::ProdExt2TambahAnak as u32),
        prod_ext2_cari: klatos_fs1_counter(Counter::ProdExt2Cari as u32),
        prod_ext2_tulis: klatos_fs1_counter(Counter::ProdExt2Tulis as u32),
        prod_ext2_waktu: klatos_fs1_counter(Counter::ProdExt2Waktu as u32),
        prod_ext2_link: klatos_fs1_counter(Counter::ProdExt2Link as u32),
        prod_ext2_resize: klatos_fs1_counter(Counter::ProdExt2Resize as u32),
        prod_ext2_induk: klatos_fs1_counter(Counter::ProdExt2Induk as u32),
        prod_ext2_superblok: klatos_fs1_counter(Counter::ProdExt2Superblok as u32),
        prod_ext2_grup: klatos_fs1_counter(Counter::ProdExt2Grup as u32),
        prod_ext2_blok_inode: klatos_fs1_counter(Counter::ProdExt2BlokInode as u32),
        prod_ext2_bitmap_indeks: klatos_fs1_counter(Counter::ProdExt2BitmapIndeks as u32),
        prod_ext2_bitmap_keadaan: klatos_fs1_counter(Counter::ProdExt2BitmapKeadaan as u32),
        prod_ext2_unmount: klatos_fs1_counter(Counter::ProdExt2Unmount as u32),
        prod_ext2_cache: klatos_fs1_counter(Counter::ProdExt2Cache as u32),
        prod_inodefile_ioctl: klatos_fs1_counter(Counter::ProdInodeFileIoctl as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas berkas-sistem batch 1.
#[no_mangle]
pub extern "C" fn klatos_fs1_selftest() -> KlatosAdmisiFs1Report {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4653_4131);
        assert!(EPERM == 1);
        assert!(ENOENT == 2);
        assert!(EIO == 5);
        assert!(EBUSY == 16);
        assert!(EEXIST == 17);
        assert!(EINVAL == 22);
        assert!(ENOSPC == 28);
        assert!(EROFS == 30);
        assert!(EMLINK == 31);
        assert!(ENAMETOOLONG == 34);
        assert!(EXT2_NAME_LEN == 255);
        assert!(EXT2_MAX_LINKS == 65535);
        assert!(IOCTL_FIBMAP == 0 && IOCTL_FIONREAD == 1 && IOCTL_LAIN == 2);
        assert!(Counter::ProdExt2Nama as usize == 0);
        assert!(Counter::ProdInodeFileIoctl as usize == 15);
        assert!(N_COUNTER == 16);
        assert!(DIFF_EXPECTED == 50);

        assert!(ext2_nama_admisi(256, EXT2_NAME_LEN) == -ENAMETOOLONG);
        assert!(ext2_link_admisi(1, EXT2_MAX_LINKS, EXT2_MAX_LINKS) == -EROFS);
        assert!(inodefile_ioctl_admisi(IOCTL_FIBMAP, 0, 1) == -EPERM);

        // 8 (tag) + 25*4 (u32) + 4 (status) = 112.
        assert!(core::mem::size_of::<KlatosAdmisiFs1Report>() == 112);
        assert!(core::mem::align_of::<KlatosAdmisiFs1Report>() == 8);
    };
}
