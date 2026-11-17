// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.101 / REHOME-01 — **OTORITAS AKSI BADAN HANDLER PANGGILAN SISTEM**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosPanggilanAksiReport` di
/// `Jowo/Kontrak/KlatSeam113.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5041_4B53; // "KLATPAKS"

// ===========================================================================
// 1. sysconf — nama `_SC_*` -> sumber nilai
// ===========================================================================
//
// Nilai `_SC_*` dari `Jowo/Kontrak/POSIX/unistd.h` (enum, urut 0..14):
//   0 _SC_MONOTONIC_CLOCK, 1 _SC_NPROCESSORS_CONF, 2 _SC_NPROCESSORS_ONLN,
//   3 _SC_OPEN_MAX, 4 _SC_HOST_NAME_MAX, 5 _SC_TTY_NAME_MAX, 6 _SC_PAGESIZE,
//   7 _SC_GETPW_R_SIZE_MAX, 8 _SC_GETGR_R_SIZE_MAX, 9 _SC_CLK_TCK,
//   10 _SC_SYMLOOP_MAX, 11 _SC_MAPPED_FILES, 12 _SC_ARG_MAX, 13 _SC_IOV_MAX,
//   14 _SC_PHYS_PAGES.
//
// Perhatikan: `_SC_MAPPED_FILES` (11) TIDAK ditangani C++ historis (jatuh ke
// `default` -> `EINVAL`); pemetaan Rust mempertahankannya.

/// Nama `sysconf` tidak dikenal -> `EINVAL`.
pub const SC_TAK_DIKENAL: u32 = 0;
/// Nilai tetap; pakai `nilai`.
pub const SC_TETAP: u32 = 1;
/// `Processor::count()`.
pub const SC_PROSESOR: u32 = 2;
/// `OpenFileDescriptions::max_open()`.
pub const SC_OPEN_MAX: u32 = 3;
/// `PAGE_SIZE`.
pub const SC_HALAMAN: u32 = 4;
/// `TimeManagement::the().ticks_per_second()`.
pub const SC_CLK_TCK: u32 = 5;
/// `VirtualFileSystem::symlink_recursion_limit`.
pub const SC_SYMLOOP: u32 = 6;
/// `Process::max_arguments_size`.
pub const SC_ARG_MAKS: u32 = 7;
/// `MM.get_system_memory_info().physical_pages`.
pub const SC_HALAMAN_FISIK: u32 = 8;
/// Jumlah sumber `sysconf`.
pub const N_SUMBER_SYSCONF: u32 = 9;

/// Verdict `sysconf`; cermin `KlatosSysconfVerdict`.
///
/// `nilai` hanya bermakna saat `sumber == SC_TETAP`; pada sumber lain ia nol.
/// Dua medan, 16 byte, align 8.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosSysconfVerdict {
    pub sumber: u32,
    pub _pad: u32,
    pub nilai: i64,
}

impl KlatosSysconfVerdict {
    #[inline]
    const fn tetap(n: i64) -> Self {
        KlatosSysconfVerdict {
            sumber: SC_TETAP,
            _pad: 0,
            nilai: n,
        }
    }
    #[inline]
    const fn sumber(s: u32) -> Self {
        KlatosSysconfVerdict {
            sumber: s,
            _pad: 0,
            nilai: 0,
        }
    }
}

/// Pemetaan nama `sysconf` -> sumber nilai. Transkripsi `sysconf.cpp:17-52`.
///
/// Konstanta literal (`HOST_NAME_MAX` = 64, `TTY_NAME_MAX` = 32, `IOV_MAX` =
/// 1024, ukuran `getpw`/`getgr` = 4096) adalah konstanta ABI POSIX; nilainya
/// disalin apa adanya dari C++ historis.
#[inline]
pub const fn sysconf(name: i32) -> KlatosSysconfVerdict {
    match name {
        0 => KlatosSysconfVerdict::tetap(1),           // _SC_MONOTONIC_CLOCK
        1 | 2 => KlatosSysconfVerdict::sumber(SC_PROSESOR),
        3 => KlatosSysconfVerdict::sumber(SC_OPEN_MAX),
        4 => KlatosSysconfVerdict::tetap(64),          // HOST_NAME_MAX
        5 => KlatosSysconfVerdict::tetap(32),          // TTY_NAME_MAX
        6 => KlatosSysconfVerdict::sumber(SC_HALAMAN),
        7 | 8 => KlatosSysconfVerdict::tetap(4096),    // GETPW_R / GETGR_R
        9 => KlatosSysconfVerdict::sumber(SC_CLK_TCK),
        10 => KlatosSysconfVerdict::sumber(SC_SYMLOOP),
        12 => KlatosSysconfVerdict::sumber(SC_ARG_MAKS),
        13 => KlatosSysconfVerdict::tetap(1024),       // IOV_MAX
        14 => KlatosSysconfVerdict::sumber(SC_HALAMAN_FISIK),
        _ => KlatosSysconfVerdict::sumber(SC_TAK_DIKENAL), // termasuk 11
    }
}

// ===========================================================================
// 2. ioctl — request -> kelas
// ===========================================================================

/// Bukan permintaan khusus; teruskan ke `File::ioctl`.
pub const IOCTL_LANJUT: u32 = 0;
/// `FIONBIO` — set blocking dari `int` ruang pengguna.
pub const IOCTL_FIONBIO: u32 = 1;
/// `FIOCLEX` — set `FD_CLOEXEC` pada fd.
pub const IOCTL_FIOCLEX: u32 = 2;
/// `FIONCLEX` — bersihkan `FD_CLOEXEC` pada fd.
pub const IOCTL_FIONCLEX: u32 = 3;
/// Jumlah kelas `ioctl`.
pub const N_KELAS_IOCTL: u32 = 4;

/// Klasifikasi `ioctl` — transkripsi `ioctl.cpp:16-34`.
///
/// Nilai `fionbio`/`fioclex`/`fionclex` DISUPLAI C++ (nomor ioctl adalah
/// konstanta `Jowo/Kontrak/Ioctl.h` milik C++). Presedensi: `FIONBIO` >
/// `FIOCLEX` > `FIONCLEX` > lanjut — persis urutan `if` C++ historis.
#[inline]
pub const fn ioctl_kelas(request: u32, fionbio: u32, fioclex: u32, fionclex: u32) -> u32 {
    if request == fionbio {
        return IOCTL_FIONBIO;
    }
    if request == fioclex {
        return IOCTL_FIOCLEX;
    }
    if request == fionclex {
        return IOCTL_FIONCLEX;
    }
    IOCTL_LANJUT
}

// ===========================================================================
// 3. dbgputstr — size -> kelas buffer
// ===========================================================================

/// `size == 0` — tidak ada yang ditulis.
pub const DBG_NOL: u32 = 0;
/// `0 < size <= 1024` — buffer tumpukan tetap.
pub const DBG_KECIL: u32 = 1;
/// `size > 1024` — alokasi `KString`.
pub const DBG_BESAR: u32 = 2;
/// Jumlah kelas `dbgputstr`.
pub const N_KELAS_DBG: u32 = 3;

/// Ambang buffer tumpukan `dbgputstr`; transkripsi `debug.cpp:20-35`.
pub const DBG_AMBANG: u64 = 1024;

/// Klasifikasi ukuran `dbgputstr`.
#[inline]
pub const fn dbgputstr_kelas(size: u64) -> u32 {
    if size == 0 {
        DBG_NOL
    } else if size <= DBG_AMBANG {
        DBG_KECIL
    } else {
        DBG_BESAR
    }
}

// ===========================================================================
// 4. unlink — flags -> operasi
// ===========================================================================

/// `unlink()` biasa.
pub const HAPUS_UNLINK: u32 = 0;
/// `unlink()` dengan `AT_REMOVEDIR` -> `rmdir()`.
pub const HAPUS_RMDIR: u32 = 1;
/// Bendera di luar `AT_REMOVEDIR` -> `EINVAL`.
pub const HAPUS_EINVAL: u32 = 2;
/// Jumlah operasi `unlink`.
pub const N_HAPUS: u32 = 3;

/// Operasi `unlink` — transkripsi `unlink.cpp:19-28`.
///
/// `at_removedir` DISUPLAI C++ (konstanta `AT_REMOVEDIR` = `0x200` dari
/// `Jowo/Kontrak/POSIX/fcntl.h`). Bendera di luar topeng `AT_REMOVEDIR`
/// ditolak lebih dulu.
#[inline]
pub const fn unlink_operasi(flags: u32, at_removedir: u32) -> u32 {
    if (flags & !at_removedir) != 0 {
        return HAPUS_EINVAL;
    }
    if (flags & at_removedir) != 0 {
        return HAPUS_RMDIR;
    }
    HAPUS_UNLINK
}

// ===========================================================================
// 5. umask — mask -> topeng baru
// ===========================================================================

/// Topeng bit izin `umask`; transkripsi `umask.cpp:14-21` (`mask & 0777`).
pub const UMASK_TOPENG: u32 = 0o777;

/// Topeng `umask` baru. `mode_t` (u16) diperlebar ke `u32` di batas FFI.
#[inline]
pub const fn umask_baru(mask: u32) -> u32 {
    mask & UMASK_TOPENG
}

// ===========================================================================
// Penghitung audit
// ===========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_panggilan_sysconf` dari produksi.
    ProdSysconf = 0,
    /// Jumlah `klatos_panggilan_ioctl_kelas` dari produksi.
    ProdIoctl = 1,
    /// Jumlah `klatos_panggilan_dbgputstr_kelas` dari produksi.
    ProdDbg = 2,
    /// Jumlah `klatos_panggilan_unlink_operasi` dari produksi.
    ProdUnlink = 3,
    /// Jumlah `klatos_panggilan_umask_baru` dari produksi.
    ProdUmask = 4,
    /// Jumlah `sysconf` yang ditolak (`SC_TAK_DIKENAL`).
    ProdSysconfTolak = 5,
    /// Jumlah `unlink` yang ditolak (`HAPUS_EINVAL`).
    ProdUnlinkTolak = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ===========================================================================
// Permukaan C ABI
// ===========================================================================

/// Pemetaan `sysconf`. Lihat [`sysconf`].
#[no_mangle]
pub extern "C" fn klatos_panggilan_sysconf(name: i32) -> KlatosSysconfVerdict {
    bump(Counter::ProdSysconf);
    let v = sysconf(name);
    if v.sumber == SC_TAK_DIKENAL {
        bump(Counter::ProdSysconfTolak);
    }
    v
}

/// Klasifikasi `ioctl`. Lihat [`ioctl_kelas`].
#[no_mangle]
pub extern "C" fn klatos_panggilan_ioctl_kelas(request: u32, fionbio: u32, fioclex: u32, fionclex: u32) -> u32 {
    bump(Counter::ProdIoctl);
    ioctl_kelas(request, fionbio, fioclex, fionclex)
}

/// Klasifikasi `dbgputstr`. Lihat [`dbgputstr_kelas`].
#[no_mangle]
pub extern "C" fn klatos_panggilan_dbgputstr_kelas(size: u64) -> u32 {
    bump(Counter::ProdDbg);
    dbgputstr_kelas(size)
}

/// Operasi `unlink`. Lihat [`unlink_operasi`].
#[no_mangle]
pub extern "C" fn klatos_panggilan_unlink_operasi(flags: u32, at_removedir: u32) -> u32 {
    bump(Counter::ProdUnlink);
    let o = unlink_operasi(flags, at_removedir);
    if o == HAPUS_EINVAL {
        bump(Counter::ProdUnlinkTolak);
    }
    o
}

/// Topeng `umask` baru. Lihat [`umask_baru`].
#[no_mangle]
pub extern "C" fn klatos_panggilan_umask_baru(mask: u32) -> u32 {
    bump(Counter::ProdUmask);
    umask_baru(mask)
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_panggilan_aksi_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_panggilan_aksi_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ===========================================================================
// Laporan + matriks
// ===========================================================================

/// Cermin `KlatosPanggilanAksiReport` di `Jowo/Kontrak/KlatSeam113.h`.
#[repr(C)]
pub struct KlatosPanggilanAksiReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub sysconf_checks: u32,
    pub sysconf_passed: u32,
    pub ioctl_checks: u32,
    pub ioctl_passed: u32,
    pub dbg_checks: u32,
    pub dbg_passed: u32,
    pub unlink_checks: u32,
    pub unlink_passed: u32,
    pub umask_checks: u32,
    pub umask_passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub prod_sysconf: u32,
    pub prod_ioctl: u32,
    pub prod_dbg: u32,
    pub prod_unlink: u32,
    pub prod_umask: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Sysconf,
    Ioctl,
    Dbg,
    Unlink,
    Umask,
}

struct Matrix {
    checks: u32,
    passed: u32,
    sysconf_checks: u32,
    sysconf_passed: u32,
    ioctl_checks: u32,
    ioctl_passed: u32,
    dbg_checks: u32,
    dbg_passed: u32,
    unlink_checks: u32,
    unlink_passed: u32,
    umask_checks: u32,
    umask_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            sysconf_checks: 0,
            sysconf_passed: 0,
            ioctl_checks: 0,
            ioctl_passed: 0,
            dbg_checks: 0,
            dbg_passed: 0,
            unlink_checks: 0,
            unlink_passed: 0,
            umask_checks: 0,
            umask_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Sysconf => {
                self.sysconf_checks += 1;
                if lulus {
                    self.sysconf_passed += 1;
                }
            }
            Group::Ioctl => {
                self.ioctl_checks += 1;
                if lulus {
                    self.ioctl_passed += 1;
                }
            }
            Group::Dbg => {
                self.dbg_checks += 1;
                if lulus {
                    self.dbg_passed += 1;
                }
            }
            Group::Unlink => {
                self.unlink_checks += 1;
                if lulus {
                    self.unlink_passed += 1;
                }
            }
            Group::Umask => {
                self.umask_checks += 1;
                if lulus {
                    self.umask_passed += 1;
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
/// sysconf 300 + ioctl 80 + dbg 200 + unlink 1024 + umask 1536 = 3140.
pub const DIFF_EXPECTED: u32 = 3140;

/// Oracle `sysconf` BERBEDA: tabel `(nama, sumber, nilai)` datar.
fn naif_sysconf(name: i32) -> KlatosSysconfVerdict {
    const TABEL: [(i32, u32, i64); 14] = [
        (0, SC_TETAP, 1),
        (1, SC_PROSESOR, 0),
        (2, SC_PROSESOR, 0),
        (3, SC_OPEN_MAX, 0),
        (4, SC_TETAP, 64),
        (5, SC_TETAP, 32),
        (6, SC_HALAMAN, 0),
        (7, SC_TETAP, 4096),
        (8, SC_TETAP, 4096),
        (9, SC_CLK_TCK, 0),
        (10, SC_SYMLOOP, 0),
        (12, SC_ARG_MAKS, 0),
        (13, SC_TETAP, 1024),
        (14, SC_HALAMAN_FISIK, 0),
    ];
    let mut i = 0usize;
    while i < TABEL.len() {
        if TABEL[i].0 == name {
            return KlatosSysconfVerdict {
                sumber: TABEL[i].1,
                _pad: 0,
                nilai: TABEL[i].2,
            };
        }
        i += 1;
    }
    KlatosSysconfVerdict::sumber(SC_TAK_DIKENAL)
}

/// Oracle `ioctl` BERBEDA: peta pasangan -> kelas.
fn naif_ioctl(request: u32, fionbio: u32, fioclex: u32, fionclex: u32) -> u32 {
    let peta: [(u32, u32); 3] = [
        (fionbio, IOCTL_FIONBIO),
        (fioclex, IOCTL_FIOCLEX),
        (fionclex, IOCTL_FIONCLEX),
    ];
    for (r, k) in peta {
        if request == r {
            return k;
        }
    }
    IOCTL_LANJUT
}

/// Kontrol NEGATIF `ioctl`: presedensi TERBALIK (fionclex diperiksa lebih
/// dulu). Berdaya hanya bila ketiga nomor berbeda.
fn ioctl_salah_urutan(request: u32, fionbio: u32, fioclex: u32, fionclex: u32) -> u32 {
    if request == fionclex {
        return IOCTL_FIONCLEX;
    }
    if request == fioclex {
        return IOCTL_FIOCLEX;
    }
    if request == fionbio {
        return IOCTL_FIONBIO;
    }
    IOCTL_LANJUT
}

fn run_matrix() -> KlatosPanggilanAksiReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5041_4B53, Group::Sysconf);
    m.catat(SC_TAK_DIKENAL == 0, Group::Sysconf);
    m.catat(SC_TETAP == 1, Group::Sysconf);
    m.catat(N_SUMBER_SYSCONF == 9, Group::Sysconf);
    m.catat(IOCTL_LANJUT == 0, Group::Ioctl);
    m.catat(IOCTL_FIONBIO == 1, Group::Ioctl);
    m.catat(IOCTL_FIOCLEX == 2, Group::Ioctl);
    m.catat(IOCTL_FIONCLEX == 3, Group::Ioctl);
    m.catat(N_KELAS_IOCTL == 4, Group::Ioctl);
    m.catat(DBG_NOL == 0, Group::Dbg);
    m.catat(DBG_KECIL == 1, Group::Dbg);
    m.catat(DBG_BESAR == 2, Group::Dbg);
    m.catat(DBG_AMBANG == 1024, Group::Dbg);
    m.catat(HAPUS_UNLINK == 0, Group::Unlink);
    m.catat(HAPUS_RMDIR == 1, Group::Unlink);
    m.catat(HAPUS_EINVAL == 2, Group::Unlink);
    m.catat(UMASK_TOPENG == 0o777, Group::Umask);
    m.catat(core::mem::size_of::<KlatosSysconfVerdict>() == 16, Group::Sysconf);

    // --- sysconf ----------------------------------------------------------
    m.catat(sysconf(0).sumber == SC_TETAP && sysconf(0).nilai == 1, Group::Sysconf);
    m.catat(sysconf(1).sumber == SC_PROSESOR && sysconf(2).sumber == SC_PROSESOR, Group::Sysconf);
    m.catat(sysconf(3).sumber == SC_OPEN_MAX, Group::Sysconf);
    m.catat(sysconf(4).sumber == SC_TETAP && sysconf(4).nilai == 64, Group::Sysconf);
    m.catat(sysconf(5).sumber == SC_TETAP && sysconf(5).nilai == 32, Group::Sysconf);
    m.catat(sysconf(6).sumber == SC_HALAMAN, Group::Sysconf);
    m.catat(sysconf(7).nilai == 4096 && sysconf(8).nilai == 4096, Group::Sysconf);
    m.catat(sysconf(9).sumber == SC_CLK_TCK, Group::Sysconf);
    m.catat(sysconf(10).sumber == SC_SYMLOOP, Group::Sysconf);
    m.catat(sysconf(12).sumber == SC_ARG_MAKS, Group::Sysconf);
    m.catat(sysconf(13).sumber == SC_TETAP && sysconf(13).nilai == 1024, Group::Sysconf);
    m.catat(sysconf(14).sumber == SC_HALAMAN_FISIK, Group::Sysconf);
    // _SC_MAPPED_FILES (11) TIDAK ditangani C++ historis -> EINVAL.
    m.catat(sysconf(11).sumber == SC_TAK_DIKENAL, Group::Sysconf);
    m.catat(sysconf(99).sumber == SC_TAK_DIKENAL, Group::Sysconf);
    m.catat(sysconf(-1).sumber == SC_TAK_DIKENAL, Group::Sysconf);

    // --- ioctl ------------------------------------------------------------
    m.catat(ioctl_kelas(10, 10, 11, 12) == IOCTL_FIONBIO, Group::Ioctl);
    m.catat(ioctl_kelas(11, 10, 11, 12) == IOCTL_FIOCLEX, Group::Ioctl);
    m.catat(ioctl_kelas(12, 10, 11, 12) == IOCTL_FIONCLEX, Group::Ioctl);
    m.catat(ioctl_kelas(13, 10, 11, 12) == IOCTL_LANJUT, Group::Ioctl);
    // Presedensi: FIONBIO MENANG bila nomornya bertabrakan.
    m.catat(ioctl_kelas(10, 10, 10, 10) == IOCTL_FIONBIO, Group::Ioctl);
    // Kontrol NEGATIF: urutan terbalik memberi jawaban berbeda saat tabrakan.
    {
        let benar = ioctl_kelas(10, 10, 10, 10);
        let salah = ioctl_salah_urutan(10, 10, 10, 10);
        m.catat(benar == IOCTL_FIONBIO, Group::Ioctl);
        m.catat(salah == IOCTL_FIONCLEX, Group::Ioctl);
        m.catat(benar != salah, Group::Ioctl);
    }

    // --- dbgputstr --------------------------------------------------------
    m.catat(dbgputstr_kelas(0) == DBG_NOL, Group::Dbg);
    m.catat(dbgputstr_kelas(1) == DBG_KECIL, Group::Dbg);
    m.catat(dbgputstr_kelas(1024) == DBG_KECIL, Group::Dbg);
    m.catat(dbgputstr_kelas(1025) == DBG_BESAR, Group::Dbg);
    m.catat(dbgputstr_kelas(u64::MAX) == DBG_BESAR, Group::Dbg);

    // --- unlink -----------------------------------------------------------
    m.catat(unlink_operasi(0, 0x200) == HAPUS_UNLINK, Group::Unlink);
    m.catat(unlink_operasi(0x200, 0x200) == HAPUS_RMDIR, Group::Unlink);
    m.catat(unlink_operasi(1, 0x200) == HAPUS_EINVAL, Group::Unlink);
    m.catat(unlink_operasi(0x200 | 1, 0x200) == HAPUS_EINVAL, Group::Unlink);
    m.catat(unlink_operasi(0xFFFFFFFF, 0x200) == HAPUS_EINVAL, Group::Unlink);
    // AT_REMOVEDIR = 0x200 (fcntl.h).
    m.catat(0x200 == 512, Group::Unlink);

    // --- umask ------------------------------------------------------------
    m.catat(umask_baru(0o022) == 0o022, Group::Umask);
    m.catat(umask_baru(0o7777) == 0o777, Group::Umask);
    m.catat(umask_baru(0o777777) == 0o777, Group::Umask);
    m.catat(umask_baru(0) == 0, Group::Umask);
    m.catat(umask_baru(0o644) == 0o644, Group::Umask);

    // --- diferensial produksi vs oracle ----------------------------------
    // sysconf: 300 nama
    for name in -100..200i32 {
        let r = sysconf(name);
        let o = naif_sysconf(name);
        if r != o {
            beda += 1;
        }
        diukur += 1;
    }
    // ioctl: 3 nomor x 40 request x 2 himpunan = 240
    for &(fb, fc, fnc) in &[(10u32, 11u32, 12u32), (0x541B, 0x5401, 0x5402)] {
        for req in 0..40u32 {
            let r = ioctl_kelas(req, fb, fc, fnc);
            let o = naif_ioctl(req, fb, fc, fnc);
            if r != o {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // dbgputstr: 200 ukuran
    for size in 0..200u64 {
        let r = dbgputstr_kelas(size);
        let o = if size == 0 {
            DBG_NOL
        } else if size <= 1024 {
            DBG_KECIL
        } else {
            DBG_BESAR
        };
        if r != o {
            beda += 1;
        }
        diukur += 1;
    }
    // unlink: 512 bendera x 2 topeng = 1024
    for flags in 0..512u32 {
        for &arm in &[0x200u32, 0x400] {
            let r = unlink_operasi(flags, arm);
            let o = if (flags & !arm) != 0 {
                HAPUS_EINVAL
            } else if (flags & arm) != 0 {
                HAPUS_RMDIR
            } else {
                HAPUS_UNLINK
            };
            if r != o {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // umask: 1536 topeng
    for mask in 0..1536u32 {
        if umask_baru(mask) != (mask & 0o777) {
            beda += 1;
        }
        diukur += 1;
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Sysconf);
    m.catat(beda == 0, Group::Sysconf);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosPanggilanAksiReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        sysconf_checks: m.sysconf_checks,
        sysconf_passed: m.sysconf_passed,
        ioctl_checks: m.ioctl_checks,
        ioctl_passed: m.ioctl_passed,
        dbg_checks: m.dbg_checks,
        dbg_passed: m.dbg_passed,
        unlink_checks: m.unlink_checks,
        unlink_passed: m.unlink_passed,
        umask_checks: m.umask_checks,
        umask_passed: m.umask_passed,
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        prod_sysconf: klatos_panggilan_aksi_counter(Counter::ProdSysconf as u32),
        prod_ioctl: klatos_panggilan_aksi_counter(Counter::ProdIoctl as u32),
        prod_dbg: klatos_panggilan_aksi_counter(Counter::ProdDbg as u32),
        prod_unlink: klatos_panggilan_aksi_counter(Counter::ProdUnlink as u32),
        prod_umask: klatos_panggilan_aksi_counter(Counter::ProdUmask as u32),
        status,
    }
}

/// Selftest aksi badan handler.
#[no_mangle]
pub extern "C" fn klatos_panggilan_aksi_selftest() -> KlatosPanggilanAksiReport {
    run_matrix()
}

// ===========================================================================
// Konformansi waktu-kompilasi
// ===========================================================================

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5041_4B53);
        assert!(SC_TAK_DIKENAL == 0);
        assert!(SC_TETAP == 1);
        assert!(SC_PROSESOR == 2);
        assert!(SC_OPEN_MAX == 3);
        assert!(SC_HALAMAN == 4);
        assert!(SC_CLK_TCK == 5);
        assert!(SC_SYMLOOP == 6);
        assert!(SC_ARG_MAKS == 7);
        assert!(SC_HALAMAN_FISIK == 8);
        assert!(N_SUMBER_SYSCONF == 9);
        assert!(IOCTL_LANJUT == 0);
        assert!(IOCTL_FIONBIO == 1);
        assert!(IOCTL_FIOCLEX == 2);
        assert!(IOCTL_FIONCLEX == 3);
        assert!(N_KELAS_IOCTL == 4);
        assert!(DBG_NOL == 0);
        assert!(DBG_KECIL == 1);
        assert!(DBG_BESAR == 2);
        assert!(DBG_AMBANG == 1024);
        assert!(HAPUS_UNLINK == 0);
        assert!(HAPUS_RMDIR == 1);
        assert!(HAPUS_EINVAL == 2);
        assert!(UMASK_TOPENG == 0o777);
        assert!(N_COUNTER == 7);
        assert!(DIFF_EXPECTED == 3140);

        // tag 8 + 19*4 (u32) + 4 (status) = 88, kelipatan align 8.
        assert!(core::mem::size_of::<KlatosPanggilanAksiReport>() == 88);
        assert!(core::mem::align_of::<KlatosPanggilanAksiReport>() == 8);
        assert!(core::mem::size_of::<KlatosSysconfVerdict>() == 16);
        assert!(core::mem::align_of::<KlatosSysconfVerdict>() == 8);
    };
}
