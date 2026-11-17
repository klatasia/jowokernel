// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.81 / WS4 — **OTORITAS ADMISI VFS, BATCH 2** (`VirtualFileSystem.cpp`).


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
/// `EROFS` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 30).
pub const EROFS: i32 = 30;

/// Tag laporan; cermin `KlatosAdmisiVfs2Report` di `Jowo/Kontrak/KlatSeam81.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5646_5332; // "KLATVFS2"

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_vfs2_akses` dari produksi.
    ProdAkses = 0,
    /// `klatos_vfs2_dir` dari produksi.
    ProdDir = 1,
    /// `klatos_vfs2_symlink` dari produksi.
    ProdSymlink = 2,
    /// `klatos_vfs2_simlink` dari produksi (jalur panas).
    ProdSimlink = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Admisi `VFS::access`. Transkripsi `access:642-658`:
///
/// ```cpp
/// if (mode & R_OK) { if (!metadata.may_read(credentials, use_effective_ids)) return EACCES; }
/// if (mode & W_OK) {
///     if (!metadata.may_write(credentials, use_effective_ids)) return EACCES;
///     if (custody->is_readonly()) return EROFS;
/// }
/// if (mode & X_OK) { if (!metadata.may_execute(credentials, use_effective_ids)) return EACCES; }
/// ```
///
/// **Urutan penting:** `EROFS` hanya muncul di blok `W_OK`, dan hanya SESUDAH
/// `may_write` lolos. `X_OK` diperiksa paling akhir.
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn akses_admisi(
    r_ok: u32,
    w_ok: u32,
    x_ok: u32,
    boleh_baca: u32,
    boleh_tulis: u32,
    boleh_exec: u32,
    readonly: u32,
) -> i32 {
    if r_ok != 0 && boleh_baca == 0 {
        return -EACCES;
    }
    if w_ok != 0 {
        if boleh_tulis == 0 {
            return -EACCES;
        }
        if readonly != 0 {
            return -EROFS;
        }
    }
    if x_ok != 0 && boleh_exec == 0 {
        return -EACCES;
    }
    0
}

/// Admisi `VFS::open_directory`. Transkripsi `open_directory:666-669`:
///
/// ```cpp
/// if (!inode.is_directory()) return ENOTDIR;
/// if (!inode.metadata().may_execute(credentials)) return EACCES;
/// ```
///
/// Mengembalikan `0` (lanjut) atau errno negatif.
#[inline]
pub const fn buka_direktori_admisi(adalah_direktori: u32, boleh_exec: u32) -> i32 {
    if adalah_direktori == 0 {
        return -ENOTDIR;
    }
    if boleh_exec == 0 {
        return -EACCES;
    }
    0
}

/// Admisi `VFS::symlink` — BAGIAN 1, SEBELUM cek veil. Transkripsi
/// `symlink:975-979`:
///
/// ```cpp
/// auto existing_custody_or_error = resolve_path(..., linkpath, base, &parent_custody, O_RDWR);
/// if (!existing_custody_or_error.is_error()) return EEXIST;
/// if (!parent_custody) return ENOENT;
/// ```
///
/// Mengembalikan `0` (lanjut ke cek veil) atau errno negatif.
#[inline]
pub const fn symlink_awal_admisi(resolve_ok: u32, parent_ada: u32) -> i32 {
    if resolve_ok != 0 {
        return -EEXIST;
    }
    if parent_ada == 0 {
        return -ENOENT;
    }
    0
}

/// Admisi `VFS::symlink` — BAGIAN 2, SESUDAH cek veil. Transkripsi
/// `symlink:987-993`:
///
/// ```cpp
/// TRY(validate_path_against_process_veil(*parent_custody, O_RDWR));   // di antara 1 dan 2
/// if (existing_custody_or_error.error().code() != ENOENT)
///     return existing_custody_or_error.release_error();
/// if (!parent_inode.metadata().may_write(credentials)) return EACCES;
/// if (parent_custody->is_readonly()) return EROFS;
/// ```
///
/// `resolve_errno` hanya bermakna bila bagian 1 mengembalikan `0` (yaitu
/// resolve gagal DENGAN induk ada). Mengembalikan `0` (lanjut) atau errno.
#[inline]
pub const fn symlink_akhir_admisi(resolve_errno: i32, boleh_tulis: u32, readonly: u32) -> i32 {
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

/// Komposisi PENUH `VFS::symlink` — dipakai matriks untuk membuktikan bahwa
/// pemecahan dua panggilan (karena cek veil berada di tengah urutan errno)
/// TIDAK mengubah verdict.
#[inline]
pub const fn symlink_admisi(
    resolve_ok: u32,
    parent_ada: u32,
    resolve_errno: i32,
    boleh_tulis: u32,
    readonly: u32,
) -> i32 {
    let awal = symlink_awal_admisi(resolve_ok, parent_ada);
    if awal != 0 {
        return awal;
    }
    symlink_akhir_admisi(resolve_errno, boleh_tulis, readonly)
}

/// `safe_to_follow_symlink` (`VirtualFileSystem.cpp:1177`). Transkripsi:
///
/// ```cpp
/// if (credentials.euid() == metadata.uid) return true;
/// if (!(parent_metadata.is_sticky() && parent_metadata.mode & S_IWOTH)) return true;
/// if (metadata.uid == parent_metadata.uid) return true;
/// return false;
/// ```
///
/// `induk_sticky_dan_wother` adalah fakta `parent_metadata.is_sticky() &&
/// (parent_metadata.mode & S_IWOTH) != 0` — dihitung C++ karena
/// `parent_metadata.mode` adalah MEKANISME.
///
/// Mengembalikan `1` (aman diikuti) atau `0` (TOLAK).
#[inline]
pub const fn ikuti_simlink_aman(
    euid_sama_uid_inode: u32,
    induk_sticky_dan_wother: u32,
    uid_inode_sama_uid_induk: u32,
) -> u32 {
    if euid_sama_uid_inode != 0 {
        return 1;
    }
    if induk_sticky_dan_wother == 0 {
        return 1;
    }
    if uid_inode_sama_uid_induk != 0 {
        return 1;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_akses_admisi(r_ok: u32, w_ok: u32, x_ok: u32, bb: u32, bt: u32, be: u32, ro: u32) -> i32 {
    let mut e = 0;
    if r_ok == 1 && bb == 0 {
        e = -EACCES;
    }
    if e == 0 && w_ok == 1 {
        if bt == 0 {
            e = -EACCES;
        } else if ro == 1 {
            e = -EROFS;
        }
    }
    if e == 0 && x_ok == 1 && be == 0 {
        e = -EACCES;
    }
    e
}

fn naif_buka_direktori_admisi(dir: u32, be: u32) -> i32 {
    match (dir, be) {
        (0, _) => -ENOTDIR,
        (_, 0) => -EACCES,
        _ => 0,
    }
}

fn naif_symlink_admisi(rok: u32, pa: u32, rerr: i32, bt: u32, ro: u32) -> i32 {
    let mut e = 0;
    if rok == 1 {
        e = -EEXIST;
    } else if pa == 0 {
        e = -ENOENT;
    } else if rerr != -ENOENT {
        e = rerr;
    } else if bt == 0 {
        e = -EACCES;
    } else if ro == 1 {
        e = -EROFS;
    }
    e
}

fn naif_ikuti_simlink_aman(a: u32, b: u32, c: u32) -> u32 {
    let tolak = a == 0 && b == 1 && c == 0;
    if tolak {
        0
    } else {
        1
    }
}

/// Kontrol NEGATIF: `ikuti_simlink_aman` yang MENJATUHKAN penjaga
/// `induk_sticky_dan_wother == 0 → aman`. Sengaja TIDAK AMAN; berdaya pada
/// `induk_sticky_dan_wother == 0 && uid_inode_sama_uid_induk == 0`:
/// produksi `1` (aman), yang salah `0` (menolak) — dan sebaliknya pada
/// `sticky == 1`, keduanya HARUS sama.
fn ikuti_simlink_aman_salah(a: u32, _b: u32, c: u32) -> u32 {
    if a != 0 {
        return 1;
    }
    if c != 0 {
        return 1;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Admisi `access()`. Lihat [`akses_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs2_akses(r_ok: u32, w_ok: u32, x_ok: u32, boleh_baca: u32, boleh_tulis: u32, boleh_exec: u32, readonly: u32) -> i32 {
    bump(Counter::ProdAkses);
    akses_admisi(r_ok, w_ok, x_ok, boleh_baca, boleh_tulis, boleh_exec, readonly)
}

/// Admisi `open_directory()`. Lihat [`buka_direktori_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs2_dir(adalah_direktori: u32, boleh_exec: u32) -> i32 {
    bump(Counter::ProdDir);
    buka_direktori_admisi(adalah_direktori, boleh_exec)
}

/// Admisi `symlink()` bagian 1 (sebelum veil). Lihat [`symlink_awal_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs2_symlink_awal(resolve_ok: u32, parent_ada: u32) -> i32 {
    bump(Counter::ProdSymlink);
    symlink_awal_admisi(resolve_ok, parent_ada)
}

/// Admisi `symlink()` bagian 2 (sesudah veil). Lihat [`symlink_akhir_admisi`].
#[no_mangle]
pub extern "C" fn klatos_vfs2_symlink_akhir(resolve_errno: i32, boleh_tulis: u32, readonly: u32) -> i32 {
    symlink_akhir_admisi(resolve_errno, boleh_tulis, readonly)
}

/// Keputusan `safe_to_follow_symlink`. Lihat [`ikuti_simlink_aman`].
#[no_mangle]
pub extern "C" fn klatos_vfs2_simlink(euid_sama_uid_inode: u32, induk_sticky_dan_wother: u32, uid_inode_sama_uid_induk: u32) -> u32 {
    bump(Counter::ProdSimlink);
    ikuti_simlink_aman(euid_sama_uid_inode, induk_sticky_dan_wother, uid_inode_sama_uid_induk)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_vfs2_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_vfs2_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiVfs2Report` di `Jowo/Kontrak/KlatSeam81.h`.
#[repr(C)]
pub struct KlatosAdmisiVfs2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub akses_checks: u32,
    pub akses_passed: u32,
    pub dir_checks: u32,
    pub dir_passed: u32,
    pub symlink_checks: u32,
    pub symlink_passed: u32,
    pub simlink_checks: u32,
    pub simlink_passed: u32,
    pub prod_akses: u32,
    pub prod_dir: u32,
    pub prod_symlink: u32,
    pub prod_simlink: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Akses,
    Dir,
    Symlink,
    Simlink,
}

struct Matrix {
    checks: u32,
    passed: u32,
    akses_checks: u32,
    akses_passed: u32,
    dir_checks: u32,
    dir_passed: u32,
    symlink_checks: u32,
    symlink_passed: u32,
    simlink_checks: u32,
    simlink_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            akses_checks: 0,
            akses_passed: 0,
            dir_checks: 0,
            dir_passed: 0,
            symlink_checks: 0,
            symlink_passed: 0,
            simlink_checks: 0,
            simlink_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Akses => {
                self.akses_checks += 1;
                if lulus {
                    self.akses_passed += 1;
                }
            }
            Group::Dir => {
                self.dir_checks += 1;
                if lulus {
                    self.dir_passed += 1;
                }
            }
            Group::Symlink => {
                self.symlink_checks += 1;
                if lulus {
                    self.symlink_passed += 1;
                }
            }
            Group::Simlink => {
                self.simlink_checks += 1;
                if lulus {
                    self.simlink_passed += 1;
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
/// akses `2 profil × 2×2×2 (r,w,x) × 2 (ro)` = 32; dir `2×2` = 4;
/// symlink `2×2×2×2` = 16; simlink `2×2×2` = 8 ⇒ 32+4+16+8 = **60**.
pub const DIFF_EXPECTED: u32 = 60;

/// Profil FAKTA untuk sapuan diferensial `akses_admisi`.
/// `(boleh_baca, boleh_tulis, boleh_exec)`
const PROFIL_AKSES: [[u32; 3]; 2] = [
    [1, 1, 1], // semuanya boleh
    [0, 0, 0], // semuanya ditolak
];

fn run_matrix() -> KlatosAdmisiVfs2Report {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5646_5332, Group::Akses);
    m.catat(ENOENT == 2 && EACCES == 13 && EEXIST == 17 && ENOTDIR == 20 && EROFS == 30, Group::Akses);

    // --- akses ------------------------------------------------------------
    m.catat(akses_admisi(0, 0, 0, 1, 1, 1, 0) == 0, Group::Akses);
    m.catat(akses_admisi(1, 0, 0, 1, 1, 1, 0) == 0, Group::Akses);
    m.catat(akses_admisi(1, 0, 0, 0, 1, 1, 0) == -EACCES, Group::Akses);
    m.catat(akses_admisi(0, 1, 0, 1, 1, 1, 0) == 0, Group::Akses);
    m.catat(akses_admisi(0, 1, 0, 1, 0, 1, 0) == -EACCES, Group::Akses);
    m.catat(akses_admisi(0, 1, 0, 1, 1, 1, 1) == -EROFS, Group::Akses);
    m.catat(akses_admisi(0, 0, 1, 1, 1, 1, 0) == 0, Group::Akses);
    m.catat(akses_admisi(0, 0, 1, 1, 1, 0, 0) == -EACCES, Group::Akses);
    // EACCES W menang atas EROFS; EROFS menang atas X.
    m.catat(akses_admisi(0, 1, 1, 1, 0, 0, 1) == -EACCES, Group::Akses);
    m.catat(akses_admisi(0, 1, 1, 1, 1, 0, 1) == -EROFS, Group::Akses);
    // R menang atas W.
    m.catat(akses_admisi(1, 1, 0, 0, 0, 1, 1) == -EACCES, Group::Akses);

    // --- buka_direktori ---------------------------------------------------
    m.catat(buka_direktori_admisi(1, 1) == 0, Group::Dir);
    m.catat(buka_direktori_admisi(0, 1) == -ENOTDIR, Group::Dir);
    m.catat(buka_direktori_admisi(1, 0) == -EACCES, Group::Dir);
    m.catat(buka_direktori_admisi(0, 0) == -ENOTDIR, Group::Dir); // ENOTDIR menang atas EACCES

    // --- symlink ----------------------------------------------------------
    m.catat(symlink_admisi(0, 1, -ENOENT, 1, 0) == 0, Group::Symlink);
    m.catat(symlink_admisi(1, 1, 0, 1, 0) == -EEXIST, Group::Symlink);
    m.catat(symlink_admisi(0, 0, -ENOENT, 1, 0) == -ENOENT, Group::Symlink);
    m.catat(symlink_admisi(0, 1, -EACCES, 1, 0) == -EACCES, Group::Symlink); // errno non-ENOENT diteruskan
    m.catat(symlink_admisi(0, 1, -ENOENT, 0, 0) == -EACCES, Group::Symlink);
    m.catat(symlink_admisi(0, 1, -ENOENT, 1, 1) == -EROFS, Group::Symlink);
    // EEXIST menang atas EACCES.
    m.catat(symlink_admisi(1, 1, 0, 0, 1) == -EEXIST, Group::Symlink);

    // --- ikuti_simlink_aman -----------------------------------------------
    m.catat(ikuti_simlink_aman(1, 1, 0) == 1, Group::Simlink); // euid == uid
    m.catat(ikuti_simlink_aman(0, 0, 0) == 1, Group::Simlink); // induk tidak sticky+wother
    m.catat(ikuti_simlink_aman(0, 1, 1) == 1, Group::Simlink); // uid inode == uid induk
    m.catat(ikuti_simlink_aman(0, 1, 0) == 0, Group::Simlink); // TOLAK

    // --- kontrol NEGATIF: penjaga sticky berdaya --------------------------
    {
        // (0,0,0): produksi AMAN (1); yang salah MENOLAK (0).
        m.catat(ikuti_simlink_aman(0, 0, 0) == 1, Group::Simlink);
        m.catat(ikuti_simlink_aman_salah(0, 0, 0) == 0, Group::Simlink);
        m.catat(ikuti_simlink_aman(0, 0, 0) != ikuti_simlink_aman_salah(0, 0, 0), Group::Simlink);
        // Pada sticky==1 keduanya HARUS sama.
        m.catat(
            ikuti_simlink_aman(0, 1, 0) == ikuti_simlink_aman_salah(0, 1, 0),
            Group::Simlink,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // akses: 2 profil × 8 (r,w,x) × 2 (ro) = 32
    for p in PROFIL_AKSES.iter() {
        for ro in 0..2u32 {
            for r in 0..2u32 {
                for w in 0..2u32 {
                    for x in 0..2u32 {
                        if akses_admisi(r, w, x, p[0], p[1], p[2], ro)
                            != naif_akses_admisi(r, w, x, p[0], p[1], p[2], ro)
                        {
                            beda += 1;
                        }
                        diukur += 1;
                    }
                }
            }
        }
    }
    // dir: 2 x 2 = 4
    for dir in 0..2u32 {
        for be in 0..2u32 {
            if buka_direktori_admisi(dir, be) != naif_buka_direktori_admisi(dir, be) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // symlink: 2 x 2 x 2 x 2 = 16 — diferensial dijalankan pada KOMPOSISI
    // dua panggilan (awal lalu akhir), bukan pada fungsi gabungan, supaya
    // pemecahan yang dipakai produksi benar-benar terbukti setara.
    for rok in 0..2u32 {
        for pa in 0..2u32 {
            for bt in 0..2u32 {
                for ro in 0..2u32 {
                    let awal = symlink_awal_admisi(rok, pa);
                    let hasil = if awal != 0 { awal } else { symlink_akhir_admisi(-ENOENT, bt, ro) };
                    if hasil != naif_symlink_admisi(rok, pa, -ENOENT, bt, ro) {
                        beda += 1;
                    }
                    diukur += 1;
                }
            }
        }
    }
    // simlink: 2 x 2 x 2 = 8
    for a in 0..2u32 {
        for b in 0..2u32 {
            for c in 0..2u32 {
                if ikuti_simlink_aman(a, b, c) != naif_ikuti_simlink_aman(a, b, c) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Akses);
    m.catat(beda == 0, Group::Akses);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiVfs2Report {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        akses_checks: m.akses_checks,
        akses_passed: m.akses_passed,
        dir_checks: m.dir_checks,
        dir_passed: m.dir_passed,
        symlink_checks: m.symlink_checks,
        symlink_passed: m.symlink_passed,
        simlink_checks: m.simlink_checks,
        simlink_passed: m.simlink_passed,
        prod_akses: klatos_vfs2_counter(Counter::ProdAkses as u32),
        prod_dir: klatos_vfs2_counter(Counter::ProdDir as u32),
        prod_symlink: klatos_vfs2_counter(Counter::ProdSymlink as u32),
        prod_simlink: klatos_vfs2_counter(Counter::ProdSimlink as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi VFS batch 2.
#[no_mangle]
pub extern "C" fn klatos_vfs2_selftest() -> KlatosAdmisiVfs2Report {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5646_5332);
        assert!(ENOENT == 2);
        assert!(EACCES == 13);
        assert!(EEXIST == 17);
        assert!(ENOTDIR == 20);
        assert!(EROFS == 30);
        assert!(Counter::ProdAkses as usize == 0);
        assert!(Counter::ProdDir as usize == 1);
        assert!(Counter::ProdSymlink as usize == 2);
        assert!(Counter::ProdSimlink as usize == 3);
        assert!(N_COUNTER == 4);
        assert!(DIFF_EXPECTED == 60);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76 -> align 8 -> 80.
        assert!(core::mem::size_of::<KlatosAdmisiVfs2Report>() == 80);
        assert!(core::mem::align_of::<KlatosAdmisiVfs2Report>() == 8);
    };
}
