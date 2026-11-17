// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.80 / WS4 — **OTORITAS ADMISI VFS** (`VirtualFileSystem.cpp`).


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `ENOENT` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 2).
pub const ENOENT: i32 = 2;
/// `EACCES` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 13).
pub const EACCES: i32 = 13;
/// `EEXIST` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 17).
pub const EEXIST: i32 = 17;
/// `ENOTDIR` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 20).
pub const ENOTDIR: i32 = 20;
/// `EISDIR` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 21).
pub const EISDIR: i32 = 21;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `EROFS` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 30).
pub const EROFS: i32 = 30;

/// Tag laporan; cermin `KlatosAdmisiVfsReport` di `Jowo/Kontrak/KlatSeam80.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5646_5341; // "KLATVFSA"

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_vfs_buka_awal` + `_buka` dari produksi.
    ProdBuka = 0,
    /// `klatos_vfs_buat` dari produksi.
    ProdBuat = 1,
    /// `klatos_vfs_mknod` + `_mkdir` dari produksi.
    ProdNode = 2,
    /// `klatos_vfs_utimens` dari produksi.
    ProdWaktu = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Admisi AWAL `VFS::open`, SEBELUM `resolve_path`. Transkripsi
/// `open:391-392`:
///
/// ```cpp
/// if ((options & O_CREAT) && (options & O_DIRECTORY))
///     return EINVAL;
/// ```
///
/// Mengembalikan `0` (lanjut) atau `-EINVAL`.
#[inline]
pub const fn buka_awal_admisi(o_creat: u32, o_directory: u32) -> i32 {
    if o_creat != 0 && o_directory != 0 {
        return -EINVAL;
    }
    0
}

/// Admisi `VFS::open` SESUDAH `resolve_path` berhasil. Transkripsi
/// `open:404-475`. Urutan cek PERSIS seperti C++ historis:
///
/// ```cpp
/// if ((options & O_CREAT) && (options & O_EXCL)) return EEXIST;
/// if (metadata.is_regular_file() && (custody.mount_flags() & MS_NOREGULAR)) return EACCES;
/// if ((options & O_DIRECTORY) && !metadata.is_directory()) return ENOTDIR;
/// if ((options & O_RDONLY) && !metadata.may_read(credentials)) return EACCES;
/// if (options & O_WRONLY) {
///     if (!metadata.may_write(credentials)) return EACCES;
///     if (metadata.is_directory()) return EISDIR;
/// }
/// if (options & O_EXEC) {
///     if (!metadata.may_execute(credentials) || (custody.mount_flags() & MS_NOEXEC)) return EACCES;
/// }
/// if (metadata.is_fifo()) { if (!(options & (O_RDONLY | O_WRONLY))) return EINVAL; ... return; }
/// if (metadata.is_device()) { if (custody.mount_flags() & MS_NODEV) return EACCES; ... return; }
/// if ((options & O_WRONLY) && custody.is_readonly()) return EROFS;
/// ```
///
/// **Keteraturan penting:** `EROFS` hanya berlaku pada jalur NON-FIFO dan
/// NON-perangkat, karena kedua cabang itu `return` lebih dulu. Itu sebabnya
/// syaratnya `fifo == 0 && perangkat == 0`.
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
#[allow(clippy::too_many_arguments)]
pub const fn buka_admisi(
    o_creat: u32,
    o_excl: u32,
    o_directory: u32,
    o_rdonly: u32,
    o_wronly: u32,
    o_exec: u32,
    reguler: u32,
    direktori: u32,
    fifo: u32,
    perangkat: u32,
    norregular: u32,
    nodev: u32,
    noexec: u32,
    readonly: u32,
    boleh_baca: u32,
    boleh_tulis: u32,
    boleh_exec: u32,
) -> i32 {
    if o_creat != 0 && o_excl != 0 {
        return -EEXIST;
    }
    if reguler != 0 && norregular != 0 {
        return -EACCES;
    }
    if o_directory != 0 && direktori == 0 {
        return -ENOTDIR;
    }
    if o_rdonly != 0 && boleh_baca == 0 {
        return -EACCES;
    }
    if o_wronly != 0 {
        if boleh_tulis == 0 {
            return -EACCES;
        }
        if direktori != 0 {
            return -EISDIR;
        }
    }
    if o_exec != 0 {
        if boleh_exec == 0 || noexec != 0 {
            return -EACCES;
        }
    }
    if fifo != 0 && o_rdonly == 0 && o_wronly == 0 {
        return -EINVAL;
    }
    if perangkat != 0 && nodev != 0 {
        return -EACCES;
    }
    if fifo == 0 && perangkat == 0 && o_wronly != 0 && readonly != 0 {
        return -EROFS;
    }
    0
}

/// Admisi `VFS::create`. Transkripsi `create:530-535`:
///
/// ```cpp
/// if (!parent_inode.metadata().may_write(credentials)) return EACCES;
/// if (parent_custody.is_readonly()) return EROFS;
/// if (is_regular_file(mode) && (parent_custody.mount_flags() & MS_NOREGULAR)) return EACCES;
/// ```
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn buat_admisi(boleh_tulis: u32, readonly: u32, reguler: u32, norregular: u32) -> i32 {
    if boleh_tulis == 0 {
        return -EACCES;
    }
    if readonly != 0 {
        return -EROFS;
    }
    if reguler != 0 && norregular != 0 {
        return -EACCES;
    }
    0
}

/// Admisi `VFS::mknod`. Transkripsi `mknod:489-504`:
///
/// ```cpp
/// if (!is_regular_file(mode) && !is_block_device(mode) && !is_character_device(mode)
///     && !is_fifo(mode) && !is_socket(mode)) return EINVAL;
/// auto existing = resolve_path(..., &parent_custody);
/// if (!existing.is_error()) return EEXIST;
/// if (!parent_custody) return ENOENT;
/// if (existing.error().code() != ENOENT) return existing.error();
/// if (!parent_inode.metadata().may_write(credentials)) return EACCES;
/// if (parent_custody->is_readonly()) return EROFS;
/// ```
///
/// `resolve_errno` adalah errno NEGATIF yang dikembalikan `resolve_path`;
/// hanya bermakna bila `resolve_ok == 0`. Mengembalikan `0` (lanjut) atau
/// errno negatif.
#[inline]
pub const fn mknod_admisi(
    mode_valid: u32,
    resolve_ok: u32,
    parent_ada: u32,
    resolve_errno: i32,
    boleh_tulis: u32,
    readonly: u32,
) -> i32 {
    if mode_valid == 0 {
        return -EINVAL;
    }
    if resolve_ok != 0 {
        return -EEXIST;
    }
    if parent_ada == 0 {
        return -ENOENT;
    }
    if resolve_errno != -ENOENT {
        return resolve_errno;
    }
    if boleh_tulis == 0 {
        return -EACCES;
    }
    if readonly != 0 {
        return -EROFS;
    }
    0
}

/// Admisi `VFS::mkdir`. Transkripsi `mkdir:567-579`:
///
/// ```cpp
/// auto result = resolve_path_without_veil(..., &parent_custody);
/// if (!result.is_error()) return EEXIST;
/// else if (!parent_custody) return result.release_error();
/// VERIFY(result.error().code() == ENOENT);
/// ...
/// if (!parent_inode.metadata().may_write(credentials)) return EACCES;
/// if (parent_custody->is_readonly()) return EROFS;
/// ```
///
/// `resolve_errno` hanya bermakna bila `resolve_ok == 0`. Mengembalikan `0`
/// (lanjut) atau errno negatif.
#[inline]
pub const fn mkdir_admisi(
    resolve_ok: u32,
    parent_ada: u32,
    resolve_errno: i32,
    boleh_tulis: u32,
    readonly: u32,
) -> i32 {
    if resolve_ok != 0 {
        return -EEXIST;
    }
    if parent_ada == 0 {
        return resolve_errno;
    }
    if resolve_errno != -ENOENT {
        return resolve_errno;
    }
    if boleh_tulis == 0 {
        return -EACCES;
    }
    if readonly != 0 {
        return -EROFS;
    }
    0
}

/// Admisi `VFS::utime` / `VFS::do_utimens`. Transkripsi `utime:346-349` dan
/// `do_utimens:364-367`:
///
/// ```cpp
/// if (!credentials.is_superuser() && inode.metadata().uid != credentials.euid())
///     return EACCES;
/// if (custody.is_readonly()) return EROFS;
/// ```
///
/// `uid_sama` = `inode.metadata().uid == credentials.euid()`.
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn utimens_admisi(superuser: u32, uid_sama: u32, readonly: u32) -> i32 {
    if superuser == 0 && uid_sama == 0 {
        return -EACCES;
    }
    if readonly != 0 {
        return -EROFS;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_buka_awal_admisi(o_creat: u32, o_directory: u32) -> i32 {
    let tolak = o_creat == 1 && o_directory == 1;
    if tolak {
        -EINVAL
    } else {
        0
    }
}

#[allow(clippy::too_many_arguments)]
fn naif_buka_admisi(
    o_creat: u32,
    o_excl: u32,
    o_directory: u32,
    o_rdonly: u32,
    o_wronly: u32,
    o_exec: u32,
    reguler: u32,
    direktori: u32,
    fifo: u32,
    perangkat: u32,
    norregular: u32,
    nodev: u32,
    noexec: u32,
    readonly: u32,
    boleh_baca: u32,
    boleh_tulis: u32,
    boleh_exec: u32,
) -> i32 {
    let mut e = 0;
    if e == 0 && o_creat == 1 && o_excl == 1 {
        e = -EEXIST;
    }
    if e == 0 && reguler == 1 && norregular == 1 {
        e = -EACCES;
    }
    if e == 0 && o_directory == 1 && direktori == 0 {
        e = -ENOTDIR;
    }
    if e == 0 && o_rdonly == 1 && boleh_baca == 0 {
        e = -EACCES;
    }
    if e == 0 && o_wronly == 1 && boleh_tulis == 0 {
        e = -EACCES;
    }
    if e == 0 && o_wronly == 1 && direktori == 1 {
        e = -EISDIR;
    }
    if e == 0 && o_exec == 1 && (boleh_exec == 0 || noexec == 1) {
        e = -EACCES;
    }
    if e == 0 && fifo == 1 && o_rdonly == 0 && o_wronly == 0 {
        e = -EINVAL;
    }
    if e == 0 && perangkat == 1 && nodev == 1 {
        e = -EACCES;
    }
    if e == 0 && fifo == 0 && perangkat == 0 && o_wronly == 1 && readonly == 1 {
        e = -EROFS;
    }
    e
}

fn naif_buat_admisi(boleh_tulis: u32, readonly: u32, reguler: u32, norregular: u32) -> i32 {
    if boleh_tulis == 0 {
        return -EACCES;
    }
    if readonly == 1 {
        return -EROFS;
    }
    if reguler == 1 && norregular == 1 {
        return -EACCES;
    }
    0
}

fn naif_mknod_admisi(
    mode_valid: u32,
    resolve_ok: u32,
    parent_ada: u32,
    resolve_errno: i32,
    boleh_tulis: u32,
    readonly: u32,
) -> i32 {
    let mut e = 0;
    if mode_valid == 0 {
        e = -EINVAL;
    } else if resolve_ok == 1 {
        e = -EEXIST;
    } else if parent_ada == 0 {
        e = -ENOENT;
    } else if resolve_errno != -ENOENT {
        e = resolve_errno;
    } else if boleh_tulis == 0 {
        e = -EACCES;
    } else if readonly == 1 {
        e = -EROFS;
    }
    e
}

fn naif_mkdir_admisi(resolve_ok: u32, parent_ada: u32, resolve_errno: i32, boleh_tulis: u32, readonly: u32) -> i32 {
    if resolve_ok == 1 {
        return -EEXIST;
    }
    if parent_ada == 0 {
        return resolve_errno;
    }
    if resolve_errno != -ENOENT {
        return resolve_errno;
    }
    if boleh_tulis == 0 {
        return -EACCES;
    }
    if readonly == 1 {
        return -EROFS;
    }
    0
}

fn naif_utimens_admisi(superuser: u32, uid_sama: u32, readonly: u32) -> i32 {
    let boleh = superuser == 1 || uid_sama == 1;
    if !boleh {
        return -EACCES;
    }
    if readonly == 1 {
        return -EROFS;
    }
    0
}

/// Kontrol NEGATIF: `buka_admisi` yang memakai `&&` longgar pada cek `EROFS`
/// (lupa syarat NON-FIFO/NON-perangkat). Sengaja salah; berdaya pada
/// `fifo == 1 && o_wronly == 1 && readonly == 1` — produksi `0` (FIFO dibuka),
/// yang salah `-EROFS`.
#[allow(clippy::too_many_arguments)]
fn buka_admisi_salah_rofs(
    o_creat: u32,
    o_excl: u32,
    o_directory: u32,
    o_rdonly: u32,
    o_wronly: u32,
    o_exec: u32,
    reguler: u32,
    direktori: u32,
    fifo: u32,
    perangkat: u32,
    norregular: u32,
    nodev: u32,
    noexec: u32,
    readonly: u32,
    boleh_baca: u32,
    boleh_tulis: u32,
    boleh_exec: u32,
) -> i32 {
    let _ = (o_creat, o_excl, o_directory, o_rdonly, o_exec, reguler, direktori, fifo, perangkat, norregular, nodev, noexec, boleh_baca, boleh_tulis, boleh_exec);
    // Sengaja menjatuhkan syarat `fifo == 0 && perangkat == 0`.
    if o_wronly != 0 && readonly != 0 {
        return -EROFS;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Admisi awal `VFS::open`. Lihat [`buka_awal_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs_buka_awal(o_creat: u32, o_directory: u32) -> i32 {
    bump(Counter::ProdBuka);
    buka_awal_admisi(o_creat, o_directory)
}

/// Admisi `VFS::open` pasca-resolve. Lihat [`buka_admisi`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn klatos_vfs_buka(
    o_creat: u32,
    o_excl: u32,
    o_directory: u32,
    o_rdonly: u32,
    o_wronly: u32,
    o_exec: u32,
    reguler: u32,
    direktori: u32,
    fifo: u32,
    perangkat: u32,
    norregular: u32,
    nodev: u32,
    noexec: u32,
    readonly: u32,
    boleh_baca: u32,
    boleh_tulis: u32,
    boleh_exec: u32,
) -> i32 {
    buka_admisi(
        o_creat, o_excl, o_directory, o_rdonly, o_wronly, o_exec, reguler, direktori, fifo, perangkat,
        norregular, nodev, noexec, readonly, boleh_baca, boleh_tulis, boleh_exec,
    )
}

/// Admisi `VFS::create`. Lihat [`buat_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs_buat(boleh_tulis: u32, readonly: u32, reguler: u32, norregular: u32) -> i32 {
    bump(Counter::ProdBuat);
    buat_admisi(boleh_tulis, readonly, reguler, norregular)
}

/// Admisi `VFS::mknod`. Lihat [`mknod_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs_mknod(
    mode_valid: u32,
    resolve_ok: u32,
    parent_ada: u32,
    resolve_errno: i32,
    boleh_tulis: u32,
    readonly: u32,
) -> i32 {
    bump(Counter::ProdNode);
    mknod_admisi(mode_valid, resolve_ok, parent_ada, resolve_errno, boleh_tulis, readonly)
}

/// Admisi `VFS::mkdir`. Lihat [`mkdir_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs_mkdir(resolve_ok: u32, parent_ada: u32, resolve_errno: i32, boleh_tulis: u32, readonly: u32) -> i32 {
    mkdir_admisi(resolve_ok, parent_ada, resolve_errno, boleh_tulis, readonly)
}

/// Admisi `VFS::utime`/`utimensat`. Lihat [`utimens_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs_utimens(superuser: u32, uid_sama: u32, readonly: u32) -> i32 {
    bump(Counter::ProdWaktu);
    utimens_admisi(superuser, uid_sama, readonly)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_vfs_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_vfs_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiVfsReport` di `Jowo/Kontrak/KlatSeam80.h`.
#[repr(C)]
pub struct KlatosAdmisiVfsReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub buka_checks: u32,
    pub buka_passed: u32,
    pub buat_checks: u32,
    pub buat_passed: u32,
    pub node_checks: u32,
    pub node_passed: u32,
    pub waktu_checks: u32,
    pub waktu_passed: u32,
    pub prod_buka: u32,
    pub prod_buat: u32,
    pub prod_node: u32,
    pub prod_waktu: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Buka,
    Buat,
    Node,
    Waktu,
}

struct Matrix {
    checks: u32,
    passed: u32,
    buka_checks: u32,
    buka_passed: u32,
    buat_checks: u32,
    buat_passed: u32,
    node_checks: u32,
    node_passed: u32,
    waktu_checks: u32,
    waktu_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            buka_checks: 0,
            buka_passed: 0,
            buat_checks: 0,
            buat_passed: 0,
            node_checks: 0,
            node_passed: 0,
            waktu_checks: 0,
            waktu_passed: 0,
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
            Group::Buat => {
                self.buat_checks += 1;
                if lulus {
                    self.buat_passed += 1;
                }
            }
            Group::Node => {
                self.node_checks += 1;
                if lulus {
                    self.node_passed += 1;
                }
            }
            Group::Waktu => {
                self.waktu_checks += 1;
                if lulus {
                    self.waktu_passed += 1;
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
/// buka_awal `2 × 2` = 4; buka `3 profil × 4 variasi` = 12; buat `2 × 2 × 2`
/// = 8; mknod `2 × 2 × 2 × 3 (bt,ro)` = 24; mkdir `2 × 2 × 2 (bt,ro)` = 8;
/// utimens `2 × 2 × 2` = 8 ⇒ 4+12+8+24+8+8 = **64**.
pub const DIFF_EXPECTED: u32 = 64;

/// Profil fakta untuk sapuan diferensial `buka_admisi`.
/// `(o_creat, o_excl, o_directory, o_rdonly, o_wronly, o_exec, reguler,
///   direktori, fifo, perangkat, norregular, nodev, noexec, readonly,
///   boleh_baca, boleh_tulis, boleh_exec)`
const PROFIL_BUKA: [[u32; 17]; 3] = [
    // [0] berkas reguler biasa, baca+tulis, semuanya boleh, mount rw.
    [0, 0, 0, 1, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1],
    // [1] direktori, O_DIRECTORY, boleh, mount ro + MS_NOREGULAR.
    [0, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 1, 1, 1],
    // [2] FIFO O_WRONLY pada mount ro, izin lengkap.
    [0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1],
];

fn run_matrix() -> KlatosAdmisiVfsReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5646_5341, Group::Buka);
    m.catat(ENOENT == 2 && EACCES == 13 && EEXIST == 17, Group::Buka);
    m.catat(ENOTDIR == 20 && EISDIR == 21 && EINVAL == 22 && EROFS == 30, Group::Buka);

    // --- buka_awal --------------------------------------------------------
    m.catat(buka_awal_admisi(0, 0) == 0, Group::Buka);
    m.catat(buka_awal_admisi(1, 0) == 0, Group::Buka);
    m.catat(buka_awal_admisi(0, 1) == 0, Group::Buka);
    m.catat(buka_awal_admisi(1, 1) == -EINVAL, Group::Buka);

    // --- buka (kasus tangan) ---------------------------------------------
    {
        let p = PROFIL_BUKA[0];
        m.catat(buka_admisi(p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8], p[9], p[10], p[11], p[12], p[13], p[14], p[15], p[16]) == 0, Group::Buka);
    }
    // O_CREAT|O_EXCL pada berkas yang ADA -> EEXIST.
    m.catat(buka_admisi(1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1) == -EEXIST, Group::Buka);
    // O_DIRECTORY pada non-direktori -> ENOTDIR.
    m.catat(buka_admisi(0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1) == -ENOTDIR, Group::Buka);
    // O_WRONLY pada direktori -> EISDIR.
    m.catat(buka_admisi(0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1) == -EISDIR, Group::Buka);
    // O_EXEC tanpa izin -> EACCES.
    m.catat(buka_admisi(0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0) == -EACCES, Group::Buka);
    // MS_NOEXEC -> EACCES.
    m.catat(buka_admisi(0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1) == -EACCES, Group::Buka);
    // FIFO tanpa O_RDONLY/O_WRONLY -> EINVAL.
    m.catat(buka_admisi(0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 1) == -EINVAL, Group::Buka);
    // perangkat + MS_NODEV -> EACCES.
    m.catat(buka_admisi(0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1, 1, 1) == -EACCES, Group::Buka);
    // EROFS pada berkas reguler O_WRONLY.
    m.catat(buka_admisi(0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1) == -EROFS, Group::Buka);
    // Keteraturan: FIFO O_WRONLY pada mount ro TIDAK EROFS (cabang FIFO lebih dulu).
    m.catat(buka_admisi(0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1) == 0, Group::Buka);

    // --- buat -------------------------------------------------------------
    m.catat(buat_admisi(1, 0, 1, 0) == 0, Group::Buat);
    m.catat(buat_admisi(0, 0, 1, 0) == -EACCES, Group::Buat);
    m.catat(buat_admisi(1, 1, 1, 0) == -EROFS, Group::Buat);
    m.catat(buat_admisi(1, 0, 1, 1) == -EACCES, Group::Buat);
    m.catat(buat_admisi(1, 0, 0, 1) == 0, Group::Buat); // bukan reguler -> MS_NOREGULAR tak berlaku

    // --- mknod ------------------------------------------------------------
    m.catat(mknod_admisi(0, 0, 1, -ENOENT, 1, 0) == -EINVAL, Group::Node);
    m.catat(mknod_admisi(1, 1, 1, 0, 1, 0) == -EEXIST, Group::Node);
    m.catat(mknod_admisi(1, 0, 0, -ENOENT, 1, 0) == -ENOENT, Group::Node);
    m.catat(mknod_admisi(1, 0, 1, -EACCES, 1, 0) == -EACCES, Group::Node); // errno non-ENOENT diteruskan
    m.catat(mknod_admisi(1, 0, 1, -ENOENT, 0, 0) == -EACCES, Group::Node);
    m.catat(mknod_admisi(1, 0, 1, -ENOENT, 1, 1) == -EROFS, Group::Node);
    m.catat(mknod_admisi(1, 0, 1, -ENOENT, 1, 0) == 0, Group::Node);

    // --- mkdir ------------------------------------------------------------
    m.catat(mkdir_admisi(1, 1, 0, 1, 0) == -EEXIST, Group::Node);
    m.catat(mkdir_admisi(0, 0, -EACCES, 1, 0) == -EACCES, Group::Node);
    m.catat(mkdir_admisi(0, 1, -ENOENT, 0, 0) == -EACCES, Group::Node);
    m.catat(mkdir_admisi(0, 1, -ENOENT, 1, 1) == -EROFS, Group::Node);
    m.catat(mkdir_admisi(0, 1, -ENOENT, 1, 0) == 0, Group::Node);

    // --- utimens ----------------------------------------------------------
    m.catat(utimens_admisi(1, 0, 0) == 0, Group::Waktu); // superuser
    m.catat(utimens_admisi(0, 1, 0) == 0, Group::Waktu); // uid sama
    m.catat(utimens_admisi(0, 0, 0) == -EACCES, Group::Waktu);
    m.catat(utimens_admisi(0, 0, 1) == -EACCES, Group::Waktu); // EACCES menang atas EROFS
    m.catat(utimens_admisi(1, 0, 1) == -EROFS, Group::Waktu);
    m.catat(utimens_admisi(0, 1, 1) == -EROFS, Group::Waktu);

    // --- kontrol NEGATIF: EROFS FIFO berdaya ------------------------------
    {
        let p = PROFIL_BUKA[2]; // FIFO O_WRONLY, mount ro
        let benar = buka_admisi(p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8], p[9], p[10], p[11], p[12], p[13], p[14], p[15], p[16]);
        let salah = buka_admisi_salah_rofs(p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8], p[9], p[10], p[11], p[12], p[13], p[14], p[15], p[16]);
        m.catat(benar == 0, Group::Buka);
        m.catat(salah == -EROFS, Group::Buka);
        m.catat(benar != salah, Group::Buka);
        // Pada berkas NON-fifo, keduanya HARUS sama.
        let q = PROFIL_BUKA[0];
        m.catat(
            buka_admisi(q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7], q[8], q[9], q[10], q[11], q[12], q[13], q[14], q[15], q[16])
                == buka_admisi_salah_rofs(q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7], q[8], q[9], q[10], q[11], q[12], q[13], q[14], q[15], q[16]),
            Group::Buka,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // buka_awal: 4
    for c in 0..2u32 {
        for d in 0..2u32 {
            if buka_awal_admisi(c, d) != naif_buka_awal_admisi(c, d) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // buka: 3 profil x 4 variasi baca/tulis/exec/creat
    for p in PROFIL_BUKA.iter() {
        for v in 0..4u32 {
            let (a, b, e, rd, wr, ex) = match v {
                0 => (p[0], p[1], p[2], 1u32, 0u32, 0u32),
                1 => (p[0], p[1], p[2], 0u32, 1u32, 0u32),
                2 => (p[0], p[1], p[2], 0u32, 0u32, 1u32),
                _ => (1u32, 1u32, 0u32, 1u32, 1u32, 0u32),
            };
            let args = (a, b, e, rd, wr, ex, p[6], p[7], p[8], p[9], p[10], p[11], p[12], p[13], p[14], p[15], p[16]);
            if buka_admisi(args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9, args.10, args.11, args.12, args.13, args.14, args.15, args.16)
                != naif_buka_admisi(args.0, args.1, args.2, args.3, args.4, args.5, args.6, args.7, args.8, args.9, args.10, args.11, args.12, args.13, args.14, args.15, args.16)
            {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // buat: 2 x 2 x 2 = 8
    for t in 0..2u32 {
        for r in 0..2u32 {
            for nr in 0..2u32 {
                if buat_admisi(t, r, nr, nr) != naif_buat_admisi(t, r, nr, nr) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // mknod: 2 x 2 x 2 = 8
    for mv in 0..2u32 {
        for rok in 0..2u32 {
            for pa in 0..2u32 {
                for (bt, ro) in [(1u32, 0u32), (0, 0), (1, 1)] {
                    if mknod_admisi(mv, rok, pa, -ENOENT, bt, ro) != naif_mknod_admisi(mv, rok, pa, -ENOENT, bt, ro) {
                        beda += 1;
                    }
                    diukur += 1;
                }
            }
        }
    }
    // mkdir: 2 x 2 x 2 = 8
    for rok in 0..2u32 {
        for pa in 0..2u32 {
            for (bt, ro) in [(1u32, 0u32), (1, 1)] {
                if mkdir_admisi(rok, pa, -ENOENT, bt, ro) != naif_mkdir_admisi(rok, pa, -ENOENT, bt, ro) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // utimens: 2 x 2 x 2 = 8
    for su in 0..2u32 {
        for us in 0..2u32 {
            for ro in 0..2u32 {
                if utimens_admisi(su, us, ro) != naif_utimens_admisi(su, us, ro) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Buka);
    m.catat(beda == 0, Group::Buka);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiVfsReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        buka_checks: m.buka_checks,
        buka_passed: m.buka_passed,
        buat_checks: m.buat_checks,
        buat_passed: m.buat_passed,
        node_checks: m.node_checks,
        node_passed: m.node_passed,
        waktu_checks: m.waktu_checks,
        waktu_passed: m.waktu_passed,
        prod_buka: klatos_vfs_counter(Counter::ProdBuka as u32),
        prod_buat: klatos_vfs_counter(Counter::ProdBuat as u32),
        prod_node: klatos_vfs_counter(Counter::ProdNode as u32),
        prod_waktu: klatos_vfs_counter(Counter::ProdWaktu as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi VFS.
#[no_mangle]
pub extern "C" fn klatos_vfs_selftest() -> KlatosAdmisiVfsReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5646_5341);
        assert!(ENOENT == 2);
        assert!(EACCES == 13);
        assert!(EEXIST == 17);
        assert!(ENOTDIR == 20);
        assert!(EISDIR == 21);
        assert!(EINVAL == 22);
        assert!(EROFS == 30);
        assert!(Counter::ProdBuka as usize == 0);
        assert!(Counter::ProdBuat as usize == 1);
        assert!(Counter::ProdNode as usize == 2);
        assert!(Counter::ProdWaktu as usize == 3);
        assert!(N_COUNTER == 4);
        assert!(DIFF_EXPECTED == 64);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76 -> align 8 -> 80.
        assert!(core::mem::size_of::<KlatosAdmisiVfsReport>() == 80);
        assert!(core::mem::align_of::<KlatosAdmisiVfsReport>() == 8);
    };
}
