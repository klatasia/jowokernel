// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS BERKAS-SISTEM, BATCH 2** (MountFile + InodeWatcher


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `ENOENT` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 2).
pub const ENOENT: i32 = 2;
/// `ENXIO` — indeks 6.
pub const ENXIO: i32 = 6;
/// `E2BIG` — indeks 7.
pub const E2BIG: i32 = 7;
/// `EAGAIN` — indeks 11.
pub const EAGAIN: i32 = 11;
/// `EFAULT` — indeks 14.
pub const EFAULT: i32 = 14;
/// `EEXIST` — indeks 17.
pub const EEXIST: i32 = 17;
/// `ENODEV` — indeks 19.
pub const ENODEV: i32 = 19;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `ENOTTY` — indeks 25.
pub const ENOTTY: i32 = 25;
/// `EPIPE` — indeks 32.
pub const EPIPE: i32 = 32;
/// `EDOM` — indeks 44.
pub const EDOM: i32 = 44;

/// Nilai `MountSpecificFlag::ValueType::Boolean` — cermin `MountSpecificFlags.h:20`.
pub const MT_BOOLEAN: u32 = 0;
/// Nilai `MountSpecificFlag::ValueType::UnsignedInteger`.
pub const MT_UNSIGNED: u32 = 1;
/// Nilai `MountSpecificFlag::ValueType::SignedInteger`.
pub const MT_SIGNED: u32 = 2;

/// Nilai `IOCtlNumber::MOUNT_IOCTL_SET_MOUNT_SPECIFIC_FLAG` — cermin `Ioctl.h:113`.
pub const MOUNT_SET: u32 = 27;
/// Nilai `IOCtlNumber::MOUNT_IOCTL_DELETE_MOUNT_SPECIFIC_FLAG` — cermin `Ioctl.h:114`.
pub const MOUNT_HAPUS: u32 = 28;

/// Hasil khusus [`fifo_baca_admisi`]: lanjut ke `DoubleBuffer::read`.
pub const FIFO_LANJUT: i32 = 0;
/// Hasil khusus [`fifo_baca_admisi`]: kembalikan 0 (EOF) tanpa menyentuh penyangga.
pub const FIFO_EOF: i32 = 1;

/// Tag laporan; cermin `KlatosAdmisiFs2Report` di `Jowo/Kontrak/KlatSeam89.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4653_4232; // "KLATFSB2"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_fs2_mount_data`.
    ProdMountData = 0,
    /// `klatos_fs2_mount_ioctl`.
    ProdMountIoctl = 1,
    /// `klatos_fs2_mount_bool`.
    ProdMountBool = 2,
    /// `klatos_fs2_watcher_baca`.
    ProdWatcherBaca = 3,
    /// `klatos_fs2_watcher_daftar`.
    ProdWatcherDaftar = 4,
    /// `klatos_fs2_watcher_lepas`.
    ProdWatcherLepas = 5,
    /// `klatos_fs2_fifo_buka`.
    ProdFifoBuka = 6,
    /// `klatos_fs2_fifo_baca`.
    ProdFifoBaca = 7,
    /// `klatos_fs2_fifo_tulis`.
    ProdFifoTulis = 8,
    /// `klatos_fs2_file_ioctl`.
    ProdFileIoctl = 9,
    /// `klatos_fs2_file_mmap`.
    ProdFileMmap = 10,
    /// `klatos_fs2_blok_baca`.
    ProdBlokBaca = 11,
}

const N_COUNTER: usize = 12;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// `verify_mount_specific_option_data` — validasi data mount-flag.
/// Transkripsi `MountFile.cpp:57-75`.
///
/// Urutan: nol → terlalu besar → alamat nol → jenis tak dikenal.
#[inline]
pub const fn mount_data_admisi(
    value_length: u32,
    value_max_size: u32,
    value_addr_null: u32,
    value_type: u32,
) -> i32 {
    if value_length == 0 {
        return -EINVAL;
    }
    if value_length > value_max_size {
        return -E2BIG;
    }
    if value_addr_null != 0 {
        return -EFAULT;
    }
    if value_type != MT_BOOLEAN && value_type != MT_UNSIGNED && value_type != MT_SIGNED {
        return -EINVAL;
    }
    0
}

/// `MountFile::ioctl` — verdict lengkap jalur `SET`/`DELETE`/tak dikenal.
/// Transkripsi `MountFile.cpp:78-151`.
///
/// `DELETE` selalu lolos (0) tanpa memeriksa data. `SET` memakai ULANG
/// [`mount_data_admisi`], lalu menolak kunci duplikat dengan `EEXIST`.
/// Permintaan lain `EINVAL`.
#[inline]
pub const fn mount_ioctl_admisi(
    request: u32,
    value_length: u32,
    value_max_size: u32,
    value_addr_null: u32,
    value_type: u32,
    kunci_ada: u32,
) -> i32 {
    if request == MOUNT_HAPUS {
        return 0;
    }
    if request != MOUNT_SET {
        return -EINVAL;
    }
    let data = mount_data_admisi(value_length, value_max_size, value_addr_null, value_type);
    if data != 0 {
        return data;
    }
    if kunci_ada != 0 {
        return -EEXIST;
    }
    0
}

/// `MountFile::ioctl` jalur `Boolean` — nilai harus 0 atau 1.
/// Transkripsi `MountFile.cpp:116-117`.
#[inline]
pub const fn mount_bool_admisi(value_integer: u64) -> i32 {
    if value_integer != 0 && value_integer != 1 {
        return -EDOM;
    }
    0
}

/// `InodeWatcher::read` — antrean kosong atau penyangga terlalu kecil.
/// Transkripsi `InodeWatcher.cpp:32-48`.
#[inline]
pub const fn inode_watcher_read_admisi(
    queue_kosong: u32,
    buffer_size: u32,
    bytes_to_write: u32,
) -> i32 {
    if queue_kosong != 0 {
        return -EAGAIN;
    }
    if buffer_size < bytes_to_write {
        return -EINVAL;
    }
    0
}

/// `InodeWatcher::register_inode` — inode sudah ditonton.
/// Transkripsi `InodeWatcher.cpp:118-119`.
#[inline]
pub const fn inode_watcher_register_admisi(sudah_ada: u32) -> i32 {
    if sudah_ada != 0 {
        return -EEXIST;
    }
    0
}

/// `InodeWatcher::unregister_by_wd` — wd tidak terdaftar.
/// Transkripsi `InodeWatcher.cpp:154-155`.
#[inline]
pub const fn inode_watcher_unregister_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENOENT;
    }
    0
}

/// `FIFO::open_direction` — penulis non-blocking tanpa pembaca.
/// Transkripsi `FIFO.cpp:37-39`.
#[inline]
pub const fn fifo_buka_admisi(block: u32, arah_writer: u32, pembaca: u32) -> i32 {
    if block == 0 && arah_writer != 0 && pembaca == 0 {
        return -ENXIO;
    }
    0
}

/// `FIFO::read` — keputusan pada penyangga kosong.
/// Transkripsi `FIFO.cpp:114-121`. Mengembalikan [`FIFO_LANJUT`], [`FIFO_EOF`],
/// atau `-EAGAIN`.
#[inline]
pub const fn fifo_baca_admisi(buffer_kosong: u32, penulis: u32, blocking: u32) -> i32 {
    if buffer_kosong != 0 {
        if penulis == 0 {
            return FIFO_EOF;
        }
        if blocking == 0 {
            return -EAGAIN;
        }
    }
    FIFO_LANJUT
}

/// `FIFO::write` — tanpa pembaca (`EPIPE`) menang atas penuh non-blocking
/// (`EAGAIN`). Transkripsi `FIFO.cpp:125-130`.
#[inline]
pub const fn fifo_tulis_admisi(pembaca: u32, blocking: u32, ruang: u32) -> i32 {
    if pembaca == 0 {
        return -EPIPE;
    }
    if blocking == 0 && ruang == 0 {
        return -EAGAIN;
    }
    0
}

/// `File::ioctl` bawaan. Transkripsi `File.cpp:36-38`.
#[inline]
pub const fn file_ioctl_admisi() -> i32 {
    -ENOTTY
}

/// `File::vmobject_and_memory_type_for_mmap` bawaan. Transkripsi `File.cpp:41-43`.
#[inline]
pub const fn file_mmap_admisi() -> i32 {
    -ENODEV
}

/// `BlockBasedFileSystem::read_blocks` — hitungan blok nol.
/// Transkripsi `BlockBasedFileSystem.cpp:254-256`.
#[inline]
pub const fn blok_baca_admisi(count: u32) -> i32 {
    if count == 0 {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_mount_data_admisi(vl: u32, vm: u32, an: u32, vt: u32) -> i32 {
    if vl == 0 {
        -EINVAL
    } else if vl > vm {
        -E2BIG
    } else if an != 0 {
        -EFAULT
    } else if vt > MT_SIGNED {
        -EINVAL
    } else {
        0
    }
}

fn naif_mount_ioctl_admisi(req: u32, vl: u32, vm: u32, an: u32, vt: u32, ka: u32) -> i32 {
    match req {
        MOUNT_HAPUS => 0,
        MOUNT_SET => {
            let d = naif_mount_data_admisi(vl, vm, an, vt);
            if d != 0 {
                d
            } else if ka == 1 {
                -EEXIST
            } else {
                0
            }
        }
        _ => -EINVAL,
    }
}

fn naif_mount_bool_admisi(v: u64) -> i32 {
    match v {
        0 | 1 => 0,
        _ => -EDOM,
    }
}

fn naif_inode_watcher_read_admisi(kosong: u32, bs: u32, btw: u32) -> i32 {
    if kosong == 1 {
        -EAGAIN
    } else if bs < btw {
        -EINVAL
    } else {
        0
    }
}

fn naif_inode_watcher_register_admisi(ada: u32) -> i32 {
    if ada == 1 {
        -EEXIST
    } else {
        0
    }
}

fn naif_inode_watcher_unregister_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        -ENOENT
    } else {
        0
    }
}

fn naif_fifo_buka_admisi(block: u32, writer: u32, pembaca: u32) -> i32 {
    if block == 0 && writer == 1 && pembaca == 0 {
        -ENXIO
    } else {
        0
    }
}

fn naif_fifo_baca_admisi(kosong: u32, penulis: u32, blocking: u32) -> i32 {
    if kosong == 1 {
        if penulis == 0 {
            FIFO_EOF
        } else if blocking == 0 {
            -EAGAIN
        } else {
            FIFO_LANJUT
        }
    } else {
        FIFO_LANJUT
    }
}

fn naif_fifo_tulis_admisi(pembaca: u32, blocking: u32, ruang: u32) -> i32 {
    if pembaca == 0 {
        -EPIPE
    } else if blocking == 0 && ruang == 0 {
        -EAGAIN
    } else {
        0
    }
}

fn naif_blok_baca_admisi(count: u32) -> i32 {
    if count == 0 {
        -EINVAL
    } else {
        0
    }
}

/// Kontrol NEGATIF: `mount_ioctl_admisi` yang MENJATUHKAN urutan validasi data
/// (memeriksa `EEXIST` sebelum `E2BIG`). Sengaja SALAH; berdaya pada
/// `value_length > max && kunci_ada == 1`.
fn mount_ioctl_salah(req: u32, vl: u32, vm: u32, an: u32, vt: u32, ka: u32) -> i32 {
    if req == MOUNT_HAPUS {
        return 0;
    }
    if req != MOUNT_SET {
        return -EINVAL;
    }
    if ka != 0 {
        return -EEXIST;
    }
    mount_data_admisi(vl, vm, an, vt)
}

/// Kontrol NEGATIF: `fifo_baca_admisi` yang MENJATUHKAN presedensi EOF atas
/// `EAGAIN`. Sengaja SALAH; berdaya pada `buffer_kosong && !penulis && !blocking`.
fn fifo_baca_salah(buffer_kosong: u32, penulis: u32, blocking: u32) -> i32 {
    if buffer_kosong != 0 && blocking == 0 {
        return -EAGAIN;
    }
    let _ = penulis;
    FIFO_LANJUT
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`mount_data_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_mount_data(
    value_length: u32,
    value_max_size: u32,
    value_addr_null: u32,
    value_type: u32,
) -> i32 {
    bump(Counter::ProdMountData);
    mount_data_admisi(value_length, value_max_size, value_addr_null, value_type)
}

/// Lihat [`mount_ioctl_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_mount_ioctl(
    request: u32,
    value_length: u32,
    value_max_size: u32,
    value_addr_null: u32,
    value_type: u32,
    kunci_ada: u32,
) -> i32 {
    bump(Counter::ProdMountIoctl);
    mount_ioctl_admisi(
        request,
        value_length,
        value_max_size,
        value_addr_null,
        value_type,
        kunci_ada,
    )
}

/// Lihat [`mount_bool_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_mount_bool(value_integer: u64) -> i32 {
    bump(Counter::ProdMountBool);
    mount_bool_admisi(value_integer)
}

/// Lihat [`inode_watcher_read_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_watcher_baca(
    queue_kosong: u32,
    buffer_size: u32,
    bytes_to_write: u32,
) -> i32 {
    bump(Counter::ProdWatcherBaca);
    inode_watcher_read_admisi(queue_kosong, buffer_size, bytes_to_write)
}

/// Lihat [`inode_watcher_register_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_watcher_daftar(sudah_ada: u32) -> i32 {
    bump(Counter::ProdWatcherDaftar);
    inode_watcher_register_admisi(sudah_ada)
}

/// Lihat [`inode_watcher_unregister_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_watcher_lepas(ditemukan: u32) -> i32 {
    bump(Counter::ProdWatcherLepas);
    inode_watcher_unregister_admisi(ditemukan)
}

/// Lihat [`fifo_buka_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_fifo_buka(block: u32, arah_writer: u32, pembaca: u32) -> i32 {
    bump(Counter::ProdFifoBuka);
    fifo_buka_admisi(block, arah_writer, pembaca)
}

/// Lihat [`fifo_baca_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_fifo_baca(buffer_kosong: u32, penulis: u32, blocking: u32) -> i32 {
    bump(Counter::ProdFifoBaca);
    fifo_baca_admisi(buffer_kosong, penulis, blocking)
}

/// Lihat [`fifo_tulis_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_fifo_tulis(pembaca: u32, blocking: u32, ruang: u32) -> i32 {
    bump(Counter::ProdFifoTulis);
    fifo_tulis_admisi(pembaca, blocking, ruang)
}

/// Lihat [`file_ioctl_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_file_ioctl() -> i32 {
    bump(Counter::ProdFileIoctl);
    file_ioctl_admisi()
}

/// Lihat [`file_mmap_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_file_mmap() -> i32 {
    bump(Counter::ProdFileMmap);
    file_mmap_admisi()
}

/// Lihat [`blok_baca_admisi`].
#[no_mangle]
pub extern "C" fn klatos_fs2_blok_baca(count: u32) -> i32 {
    bump(Counter::ProdBlokBaca);
    blok_baca_admisi(count)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_fs2_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_fs2_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiFs2Report` di `Jowo/Kontrak/KlatSeam89.h`.
#[repr(C)]
pub struct KlatosAdmisiFs2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub mount_checks: u32,
    pub mount_passed: u32,
    pub fifo_checks: u32,
    pub fifo_passed: u32,
    pub prod_mount_data: u32,
    pub prod_mount_ioctl: u32,
    pub prod_mount_bool: u32,
    pub prod_watcher_baca: u32,
    pub prod_watcher_daftar: u32,
    pub prod_watcher_lepas: u32,
    pub prod_fifo_buka: u32,
    pub prod_fifo_baca: u32,
    pub prod_fifo_tulis: u32,
    pub prod_file_ioctl: u32,
    pub prod_file_mmap: u32,
    pub prod_blok_baca: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Mount,
    Fifo,
}

struct Matrix {
    checks: u32,
    passed: u32,
    mount_checks: u32,
    mount_passed: u32,
    fifo_checks: u32,
    fifo_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            mount_checks: 0,
            mount_passed: 0,
            fifo_checks: 0,
            fifo_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Mount => {
                self.mount_checks += 1;
                if lulus {
                    self.mount_passed += 1;
                }
            }
            Group::Fifo => {
                self.fifo_checks += 1;
                if lulus {
                    self.fifo_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **109**
/// (48 mount_data + 18 mount_ioctl + 4 mount_bool + 8 watcher_read
///  + 4 watcher reg/lepas + 3 blok_baca + 8 fifo_buka + 8 fifo_baca + 8 fifo_tulis).
pub const DIFF_EXPECTED: u32 = 109;

fn run_matrix() -> KlatosAdmisiFs2Report {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4653_4232, Group::Mount);
    m.catat(
        ENOENT == 2 && ENXIO == 6 && E2BIG == 7 && EAGAIN == 11 && EFAULT == 14 && EEXIST == 17
            && ENODEV == 19 && EINVAL == 22 && ENOTTY == 25 && EPIPE == 32 && EDOM == 44,
        Group::Mount,
    );
    m.catat(
        MT_BOOLEAN == 0 && MT_UNSIGNED == 1 && MT_SIGNED == 2,
        Group::Mount,
    );
    m.catat(MOUNT_SET == 27 && MOUNT_HAPUS == 28, Group::Mount);
    m.catat(FIFO_LANJUT == 0 && FIFO_EOF == 1, Group::Mount);

    // --- kasus utama ------------------------------------------------------
    const VM: u32 = 8;
    // mount data: urutan nol > besar > alamat > jenis.
    m.catat(mount_data_admisi(0, VM, 0, MT_BOOLEAN) == -EINVAL, Group::Mount);
    m.catat(mount_data_admisi(9, VM, 0, MT_BOOLEAN) == -E2BIG, Group::Mount);
    m.catat(mount_data_admisi(8, VM, 1, MT_BOOLEAN) == -EFAULT, Group::Mount);
    m.catat(mount_data_admisi(8, VM, 0, 3) == -EINVAL, Group::Mount);
    m.catat(mount_data_admisi(8, VM, 0, MT_UNSIGNED) == 0, Group::Mount);
    // data cacat menang atas EEXIST.
    m.catat(mount_ioctl_admisi(MOUNT_SET, 9, VM, 0, MT_BOOLEAN, 1) == -E2BIG, Group::Mount);
    m.catat(mount_ioctl_admisi(MOUNT_SET, 8, VM, 0, MT_BOOLEAN, 1) == -EEXIST, Group::Mount);
    m.catat(mount_ioctl_admisi(MOUNT_SET, 8, VM, 0, MT_SIGNED, 0) == 0, Group::Mount);
    // DELETE selalu lolos, bahkan dengan data sampah.
    m.catat(mount_ioctl_admisi(MOUNT_HAPUS, 0, VM, 1, 99, 1) == 0, Group::Mount);
    // permintaan tak dikenal.
    m.catat(mount_ioctl_admisi(99, 8, VM, 0, MT_BOOLEAN, 0) == -EINVAL, Group::Mount);
    // boolean 0/1.
    m.catat(mount_bool_admisi(0) == 0, Group::Mount);
    m.catat(mount_bool_admisi(1) == 0, Group::Mount);
    m.catat(mount_bool_admisi(2) == -EDOM, Group::Mount);

    // InodeWatcher.
    m.catat(inode_watcher_read_admisi(1, 0, 0) == -EAGAIN, Group::Mount);
    m.catat(inode_watcher_read_admisi(0, 4, 8) == -EINVAL, Group::Mount);
    m.catat(inode_watcher_read_admisi(0, 8, 8) == 0, Group::Mount);
    m.catat(inode_watcher_register_admisi(1) == -EEXIST, Group::Mount);
    m.catat(inode_watcher_register_admisi(0) == 0, Group::Mount);
    m.catat(inode_watcher_unregister_admisi(0) == -ENOENT, Group::Mount);
    m.catat(inode_watcher_unregister_admisi(1) == 0, Group::Mount);

    // File bawaan.
    m.catat(file_ioctl_admisi() == -ENOTTY, Group::Mount);
    m.catat(file_mmap_admisi() == -ENODEV, Group::Mount);
    m.catat(blok_baca_admisi(0) == -EINVAL, Group::Mount);
    m.catat(blok_baca_admisi(1) == 0, Group::Mount);

    // FIFO.
    m.catat(fifo_buka_admisi(0, 1, 0) == -ENXIO, Group::Fifo);
    m.catat(fifo_buka_admisi(1, 1, 0) == 0, Group::Fifo);
    m.catat(fifo_buka_admisi(0, 0, 0) == 0, Group::Fifo);
    // EOF menang atas EAGAIN.
    m.catat(fifo_baca_admisi(1, 0, 0) == FIFO_EOF, Group::Fifo);
    m.catat(fifo_baca_admisi(1, 1, 0) == -EAGAIN, Group::Fifo);
    m.catat(fifo_baca_admisi(1, 1, 1) == FIFO_LANJUT, Group::Fifo);
    m.catat(fifo_baca_admisi(0, 0, 0) == FIFO_LANJUT, Group::Fifo);
    // EPIPE menang atas EAGAIN.
    m.catat(fifo_tulis_admisi(0, 0, 0) == -EPIPE, Group::Fifo);
    m.catat(fifo_tulis_admisi(1, 0, 0) == -EAGAIN, Group::Fifo);
    m.catat(fifo_tulis_admisi(1, 1, 0) == 0, Group::Fifo);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // urutan validasi data berdaya.
        m.catat(mount_ioctl_admisi(MOUNT_SET, 9, VM, 0, MT_BOOLEAN, 1) == -E2BIG, Group::Mount);
        m.catat(mount_ioctl_salah(MOUNT_SET, 9, VM, 0, MT_BOOLEAN, 1) == -EEXIST, Group::Mount);
        m.catat(
            mount_ioctl_admisi(MOUNT_SET, 9, VM, 0, MT_BOOLEAN, 1)
                != mount_ioctl_salah(MOUNT_SET, 9, VM, 0, MT_BOOLEAN, 1),
            Group::Mount,
        );
        m.catat(
            mount_ioctl_admisi(MOUNT_SET, 8, VM, 0, MT_BOOLEAN, 1)
                == mount_ioctl_salah(MOUNT_SET, 8, VM, 0, MT_BOOLEAN, 1),
            Group::Mount,
        );

        // presedensi EOF atas EAGAIN berdaya.
        m.catat(fifo_baca_admisi(1, 0, 0) == FIFO_EOF, Group::Fifo);
        m.catat(fifo_baca_salah(1, 0, 0) == -EAGAIN, Group::Fifo);
        m.catat(
            fifo_baca_admisi(1, 0, 0) != fifo_baca_salah(1, 0, 0),
            Group::Fifo,
        );
        m.catat(
            fifo_baca_admisi(1, 1, 0) == fifo_baca_salah(1, 1, 0),
            Group::Fifo,
        );

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if file_mmap_admisi() != -ENODEV {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Mount);
    }

    // --- diferensial produksi vs oracle (109 kasus) -----------------------
    macro_rules! ukur {
        ($g:expr, $p:expr, $n:expr) => {{
            if $p != $n {
                beda += 1;
            }
            diukur += 1;
            let _ = $g;
        }};
    }

    // mount_data: 2 (vl) x 3 (vm) x 2 (an) x 4 (vt) = 48
    for vl in [0u32, 8] {
        for vm in [4u32, 8, 16] {
            for an in 0..2u32 {
                for vt in 0..4u32 {
                    ukur!(
                        Group::Mount,
                        mount_data_admisi(vl, vm, an, vt),
                        naif_mount_data_admisi(vl, vm, an, vt)
                    );
                }
            }
        }
    }
    // mount_bool: 4
    for v in [0u64, 1, 2, u64::MAX] {
        ukur!(
            Group::Mount,
            mount_bool_admisi(v),
            naif_mount_bool_admisi(v)
        );
    }
    // mount_ioctl: 3 (req) x 3 (vl) x 2 (kunci_ada) = 18
    for req in [MOUNT_SET, MOUNT_HAPUS, 99u32] {
        for vl in [0u32, 8, 9] {
            for ka in 0..2u32 {
                ukur!(
                    Group::Mount,
                    mount_ioctl_admisi(req, vl, 8, 0, 0, ka),
                    naif_mount_ioctl_admisi(req, vl, 8, 0, 0, ka)
                );
            }
        }
    }
    // watcher_read: 2 x 2 x 2 = 8
    for a in 0..2u32 {
        for b in [0u32, 8] {
            for c in [0u32, 8] {
                ukur!(
                    Group::Mount,
                    inode_watcher_read_admisi(a, b, c),
                    naif_inode_watcher_read_admisi(a, b, c)
                );
            }
        }
    }
    // watcher register/unregister: 2 + 2
    for v in 0..2u32 {
        ukur!(
            Group::Mount,
            inode_watcher_register_admisi(v),
            naif_inode_watcher_register_admisi(v)
        );
        ukur!(
            Group::Mount,
            inode_watcher_unregister_admisi(v),
            naif_inode_watcher_unregister_admisi(v)
        );
    }
    // blok_baca: 3
    for v in [0u32, 1, 7] {
        ukur!(
            Group::Mount,
            blok_baca_admisi(v),
            naif_blok_baca_admisi(v)
        );
    }
    // fifo_buka: 2 x 2 x 2 = 8
    for a in 0..2u32 {
        for b in 0..2u32 {
            for c in 0..2u32 {
                ukur!(
                    Group::Fifo,
                    fifo_buka_admisi(a, b, c),
                    naif_fifo_buka_admisi(a, b, c)
                );
            }
        }
    }
    // fifo_baca: 2 x 2 x 2 = 8
    for a in 0..2u32 {
        for b in 0..2u32 {
            for c in 0..2u32 {
                ukur!(
                    Group::Fifo,
                    fifo_baca_admisi(a, b, c),
                    naif_fifo_baca_admisi(a, b, c)
                );
            }
        }
    }
    // fifo_tulis: 2 x 2 x 2 = 8
    for a in 0..2u32 {
        for b in 0..2u32 {
            for c in 0..2u32 {
                ukur!(
                    Group::Fifo,
                    fifo_tulis_admisi(a, b, c),
                    naif_fifo_tulis_admisi(a, b, c)
                );
            }
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Mount);
    m.catat(beda == 0, Group::Mount);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiFs2Report {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        mount_checks: m.mount_checks,
        mount_passed: m.mount_passed,
        fifo_checks: m.fifo_checks,
        fifo_passed: m.fifo_passed,
        prod_mount_data: klatos_fs2_counter(Counter::ProdMountData as u32),
        prod_mount_ioctl: klatos_fs2_counter(Counter::ProdMountIoctl as u32),
        prod_mount_bool: klatos_fs2_counter(Counter::ProdMountBool as u32),
        prod_watcher_baca: klatos_fs2_counter(Counter::ProdWatcherBaca as u32),
        prod_watcher_daftar: klatos_fs2_counter(Counter::ProdWatcherDaftar as u32),
        prod_watcher_lepas: klatos_fs2_counter(Counter::ProdWatcherLepas as u32),
        prod_fifo_buka: klatos_fs2_counter(Counter::ProdFifoBuka as u32),
        prod_fifo_baca: klatos_fs2_counter(Counter::ProdFifoBaca as u32),
        prod_fifo_tulis: klatos_fs2_counter(Counter::ProdFifoTulis as u32),
        prod_file_ioctl: klatos_fs2_counter(Counter::ProdFileIoctl as u32),
        prod_file_mmap: klatos_fs2_counter(Counter::ProdFileMmap as u32),
        prod_blok_baca: klatos_fs2_counter(Counter::ProdBlokBaca as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas berkas-sistem batch 2.
#[no_mangle]
pub extern "C" fn klatos_fs2_selftest() -> KlatosAdmisiFs2Report {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4653_4232);
        assert!(ENOENT == 2);
        assert!(ENXIO == 6);
        assert!(E2BIG == 7);
        assert!(EAGAIN == 11);
        assert!(EFAULT == 14);
        assert!(EEXIST == 17);
        assert!(ENODEV == 19);
        assert!(EINVAL == 22);
        assert!(ENOTTY == 25);
        assert!(EPIPE == 32);
        assert!(EDOM == 44);
        assert!(MT_BOOLEAN == 0 && MT_UNSIGNED == 1 && MT_SIGNED == 2);
        assert!(MOUNT_SET == 27 && MOUNT_HAPUS == 28);
        assert!(FIFO_LANJUT == 0 && FIFO_EOF == 1);
        assert!(Counter::ProdMountData as usize == 0);
        assert!(Counter::ProdBlokBaca as usize == 11);
        assert!(N_COUNTER == 12);
        assert!(DIFF_EXPECTED == 109);

        assert!(mount_data_admisi(0, 8, 0, 0) == -EINVAL);
        assert!(mount_ioctl_admisi(MOUNT_SET, 9, 8, 0, 0, 1) == -E2BIG);
        assert!(fifo_baca_admisi(1, 0, 0) == FIFO_EOF);

        // 8 (tag) + 20*4 (u32) + 4 (status) = 92 → 96 dengan padding 8-byte.
        assert!(core::mem::size_of::<KlatosAdmisiFs2Report>() == 96);
        assert!(core::mem::align_of::<KlatosAdmisiFs2Report>() == 8);
    };
}
