// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.92 / WS4 — **OTORITAS BERKAS-SISTEM, BATCH 6: SysFS**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `ENOENT` — indeks 2.
pub const ENOENT: i32 = 2;
/// `EIO` — indeks 5.
pub const EIO: i32 = 5;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `EROFS` — indeks 30.
pub const EROFS: i32 = 30;
/// `ELOOP` — indeks 35.
pub const ELOOP: i32 = 35;
/// `EOVERFLOW` — indeks 36.
pub const EOVERFLOW: i32 = 36;
/// `ENOTSUP` — indeks 74.
pub const ENOTSUP: i32 = 74;

/// Batas hop path relatif SysFS; cermin `Component.cpp` (`current_hop >= 128`).
pub const HOP_MAKS: u32 = 128;

/// Tag laporan; cermin `KlatosAdmisiSysfsReport` di `Jowo/Kontrak/KlatSeam93.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5359_5346; // "KLATSYSF"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_sysfs_ro`.
    ProdRo = 0,
    /// `klatos_sysfs_izin`.
    ProdIzin = 1,
    /// `klatos_sysfs_cari`.
    ProdCari = 2,
    /// `klatos_sysfs_loop`.
    ProdLoop = 3,
    /// `klatos_sysfs_jail`.
    ProdJail = 4,
    /// `klatos_sysfs_eio`.
    ProdEio = 5,
    /// `klatos_sysfs_offset`.
    ProdOffset = 6,
    /// `klatos_sysfs_truncate`.
    ProdTruncate = 7,
    /// `klatos_sysfs_boolean`.
    ProdBoolean = 8,
    /// `klatos_sysfs_coredump`.
    ProdCoredump = 9,
    /// `klatos_sysfs_power`.
    ProdPower = 10,
    /// `klatos_sysfs_rom`.
    ProdRom = 11,
    /// `klatos_sysfs_rom_pointer`.
    ProdRomPointer = 12,
    /// `klatos_sysfs_fisik`.
    ProdFisik = 13,
    /// `klatos_sysfs_panic`.
    ProdPanic = 14,
}

const N_COUNTER: usize = 15;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Operasi tulis SysFS read-only. Satu verdict, empat pemanggil.
/// Transkripsi `SysFS/FileSystem.cpp:44` dan `SysFS/Inode.cpp:83,88,93`.
#[inline]
pub const fn sysfs_ro_admisi() -> i32 {
    -EROFS
}

/// `chmod`/`chown` tak diizinkan. Transkripsi `SysFS/Inode.cpp:98,103`.
#[inline]
pub const fn sysfs_izin_admisi() -> i32 {
    -EPERM
}

/// Komponen SysFS tidak ditemukan.
/// Transkripsi `SysFS/DirectoryInode.cpp:51` dan `SysFS/Subsystems/Inti/Profile.cpp:29`.
#[inline]
pub const fn sysfs_cari_admisi(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ENOENT;
    }
    0
}

/// Hop path relatif melebihi batas. Transkripsi `SysFS/Component.cpp:51,61`.
#[inline]
pub const fn sysfs_loop_admisi(hop: u32) -> i32 {
    if hop >= HOP_MAKS {
        return -ELOOP;
    }
    0
}

/// Proses di-jail menulis (boleh=0) atau membaca node tak-publik (boleh=0).
/// Satu verdict, sembilan pemanggil.
/// Transkripsi `BooleanVariable.cpp:31`, `StringVariable.cpp:33`,
/// `PowerStateSwitch.cpp:45`, `RequestPanic.cpp:24,31,39`,
/// `ConstantInformation.cpp:43`, `GlobalInformation.cpp:59`.
#[inline]
pub const fn sysfs_jail_admisi(jailed: u32, boleh: u32) -> i32 {
    if jailed != 0 && boleh == 0 {
        return -EPERM;
    }
    0
}

/// Deskripsi / cache data hilang.
/// Transkripsi `GlobalInformation.cpp:23,29`, `DeviceInformation.cpp:126,132`.
#[inline]
pub const fn sysfs_eio_admisi(ada: u32) -> i32 {
    if ada == 0 {
        return -EIO;
    }
    0
}

/// Offset negatif. Transkripsi `ConstantInformation.cpp:39`,
/// `DeviceExpansionROM.cpp:39`.
#[inline]
pub const fn sysfs_offset_admisi(negatif: u32) -> i32 {
    if negatif != 0 {
        return -EINVAL;
    }
    0
}

/// `truncate` dengan ukuran bukan nol.
/// Transkripsi `BooleanVariable.cpp:48`, `StringVariable.cpp:41`,
/// `PowerStateSwitch.cpp:37`.
#[inline]
pub const fn sysfs_truncate_admisi(ukuran: u64) -> i32 {
    if ukuran != 0 {
        return -EPERM;
    }
    0
}

/// Tulis variabel boolean: panjang harus 1 dan nilai harus '0'/'1'.
/// Transkripsi `BooleanVariable.cpp:34,42`.
#[inline]
pub const fn sysfs_boolean_admisi(count: u64, nilai_valid: u32) -> i32 {
    if count != 1 {
        return -EINVAL;
    }
    if nilai_valid == 0 {
        return -EINVAL;
    }
    0
}

/// Path coredump harus absolut (atau kosong).
/// Transkripsi `CoredumpDirectory.cpp:38`.
#[inline]
pub const fn sysfs_coredump_admisi(panjang: u64, awal_slash: u32) -> i32 {
    if panjang > 0 && awal_slash == 0 {
        return -EINVAL;
    }
    0
}

/// Tulis power-state: overflow, offset, panjang, perintah.
/// Transkripsi `PowerStateSwitch.cpp:47,49,51,62`.
#[inline]
pub const fn sysfs_power_tulis_admisi(
    overflow: u32,
    offset: u64,
    count: u64,
    nilai_valid: u32,
) -> i32 {
    if overflow != 0 {
        return -EOVERFLOW;
    }
    if offset > 0 {
        return -EINVAL;
    }
    if count > 1 {
        return -EINVAL;
    }
    if nilai_valid == 0 {
        return -EINVAL;
    }
    0
}

/// Expansion ROM PCI tidak ada (ukuran 0).
/// Transkripsi `DeviceExpansionROM.cpp:35`.
#[inline]
pub const fn sysfs_rom_admisi(ukuran: u64) -> i32 {
    if ukuran == 0 {
        return -EIO;
    }
    0
}

/// Pointer fisik expansion ROM PCI nol.
/// Transkripsi `DeviceExpansionROM.cpp:66`.
#[inline]
pub const fn sysfs_rom_pointer_admisi(pointer: u32) -> i32 {
    if pointer == 0 {
        return -EIO;
    }
    0
}

/// Akses memori fisik expansion ROM ditolak.
/// Transkripsi `DeviceExpansionROM.cpp:74`.
#[inline]
pub const fn sysfs_fisik_admisi(diizinkan: u32) -> i32 {
    if diizinkan == 0 {
        return -EPERM;
    }
    0
}

/// Node request-panic tidak bisa dibaca.
/// Transkripsi `RequestPanic.cpp:25`.
#[inline]
pub const fn sysfs_panic_baca_admisi() -> i32 {
    -ENOTSUP
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_sysfs_cari_admisi(ditemukan: u32) -> i32 {
    match ditemukan {
        0 => -ENOENT,
        _ => 0,
    }
}
fn naif_sysfs_loop_admisi(hop: u32) -> i32 {
    if hop < HOP_MAKS {
        0
    } else {
        -ELOOP
    }
}
fn naif_sysfs_jail_admisi(jailed: u32, boleh: u32) -> i32 {
    match (jailed, boleh) {
        (0, _) => 0,
        (_, 1) => 0,
        _ => -EPERM,
    }
}
fn naif_sysfs_eio_admisi(ada: u32) -> i32 {
    match ada {
        1 => 0,
        _ => -EIO,
    }
}
fn naif_sysfs_offset_admisi(negatif: u32) -> i32 {
    match negatif {
        0 => 0,
        _ => -EINVAL,
    }
}
fn naif_sysfs_truncate_admisi(ukuran: u64) -> i32 {
    if ukuran == 0 {
        0
    } else {
        -EPERM
    }
}
fn naif_sysfs_boolean_admisi(count: u64, nilai_valid: u32) -> i32 {
    if count == 1 && nilai_valid == 1 {
        0
    } else {
        -EINVAL
    }
}
fn naif_sysfs_coredump_admisi(panjang: u64, awal_slash: u32) -> i32 {
    if panjang != 0 && awal_slash == 0 {
        -EINVAL
    } else {
        0
    }
}
fn naif_sysfs_power_tulis_admisi(overflow: u32, offset: u64, count: u64, nilai_valid: u32) -> i32 {
    if overflow == 1 {
        -EOVERFLOW
    } else if offset != 0 || count > 1 || nilai_valid == 0 {
        -EINVAL
    } else {
        0
    }
}
fn naif_sysfs_rom_admisi(ukuran: u64) -> i32 {
    match ukuran {
        0 => -EIO,
        _ => 0,
    }
}
fn naif_sysfs_rom_pointer_admisi(pointer: u32) -> i32 {
    match pointer {
        0 => -EIO,
        _ => 0,
    }
}
fn naif_sysfs_fisik_admisi(diizinkan: u32) -> i32 {
    match diizinkan {
        0 => -EPERM,
        _ => 0,
    }
}

/// Kontrol NEGATIF: `sysfs_jail_admisi` yang salah mengembalikan `EACCES`
/// (bukan `EPERM`). Sengaja SALAH.
fn sysfs_jail_salah(jailed: u32, boleh: u32) -> i32 {
    if jailed != 0 && boleh == 0 {
        return -13; // EACCES
    }
    0
}

/// Kontrol NEGATIF: `sysfs_loop_admisi` yang salah memakai `> HOP_MAKS`
/// (bukan `>=`). Sengaja SALAH; berdaya pada `hop == HOP_MAKS`.
fn sysfs_loop_salah(hop: u32) -> i32 {
    if hop > HOP_MAKS {
        return -ELOOP;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`sysfs_ro_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_ro() -> i32 {
    bump(Counter::ProdRo);
    sysfs_ro_admisi()
}

/// Lihat [`sysfs_izin_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_izin() -> i32 {
    bump(Counter::ProdIzin);
    sysfs_izin_admisi()
}

/// Lihat [`sysfs_cari_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_cari(ditemukan: u32) -> i32 {
    bump(Counter::ProdCari);
    sysfs_cari_admisi(ditemukan)
}

/// Lihat [`sysfs_loop_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_loop(hop: u32) -> i32 {
    bump(Counter::ProdLoop);
    sysfs_loop_admisi(hop)
}

/// Lihat [`sysfs_jail_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_jail(jailed: u32, boleh: u32) -> i32 {
    bump(Counter::ProdJail);
    sysfs_jail_admisi(jailed, boleh)
}

/// Lihat [`sysfs_eio_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_eio(ada: u32) -> i32 {
    bump(Counter::ProdEio);
    sysfs_eio_admisi(ada)
}

/// Lihat [`sysfs_offset_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_offset(negatif: u32) -> i32 {
    bump(Counter::ProdOffset);
    sysfs_offset_admisi(negatif)
}

/// Lihat [`sysfs_truncate_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_truncate(ukuran: u64) -> i32 {
    bump(Counter::ProdTruncate);
    sysfs_truncate_admisi(ukuran)
}

/// Lihat [`sysfs_boolean_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_boolean(count: u64, nilai_valid: u32) -> i32 {
    bump(Counter::ProdBoolean);
    sysfs_boolean_admisi(count, nilai_valid)
}

/// Lihat [`sysfs_coredump_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_coredump(panjang: u64, awal_slash: u32) -> i32 {
    bump(Counter::ProdCoredump);
    sysfs_coredump_admisi(panjang, awal_slash)
}

/// Lihat [`sysfs_power_tulis_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_power(overflow: u32, offset: u64, count: u64, nilai_valid: u32) -> i32 {
    bump(Counter::ProdPower);
    sysfs_power_tulis_admisi(overflow, offset, count, nilai_valid)
}

/// Lihat [`sysfs_rom_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_rom(ukuran: u64) -> i32 {
    bump(Counter::ProdRom);
    sysfs_rom_admisi(ukuran)
}

/// Lihat [`sysfs_rom_pointer_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_rom_pointer(pointer: u32) -> i32 {
    bump(Counter::ProdRomPointer);
    sysfs_rom_pointer_admisi(pointer)
}

/// Lihat [`sysfs_fisik_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_fisik(diizinkan: u32) -> i32 {
    bump(Counter::ProdFisik);
    sysfs_fisik_admisi(diizinkan)
}

/// Lihat [`sysfs_panic_baca_admisi`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_panic() -> i32 {
    bump(Counter::ProdPanic);
    sysfs_panic_baca_admisi()
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_sysfs_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_sysfs_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiSysfsReport` di `Jowo/Kontrak/KlatSeam93.h`.
#[repr(C)]
pub struct KlatosAdmisiSysfsReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub teras_checks: u32,
    pub teras_passed: u32,
    pub sub_checks: u32,
    pub sub_passed: u32,
    pub prod_ro: u32,
    pub prod_izin: u32,
    pub prod_cari: u32,
    pub prod_loop: u32,
    pub prod_jail: u32,
    pub prod_eio: u32,
    pub prod_offset: u32,
    pub prod_truncate: u32,
    pub prod_boolean: u32,
    pub prod_coredump: u32,
    pub prod_power: u32,
    pub prod_rom: u32,
    pub prod_rom_pointer: u32,
    pub prod_fisik: u32,
    pub prod_panic: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Teras,
    Sub,
}

struct Matrix {
    checks: u32,
    passed: u32,
    teras_checks: u32,
    teras_passed: u32,
    sub_checks: u32,
    sub_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            teras_checks: 0,
            teras_passed: 0,
            sub_checks: 0,
            sub_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Teras => {
                self.teras_checks += 1;
                if lulus {
                    self.teras_passed += 1;
                }
            }
            Group::Sub => {
                self.sub_checks += 1;
                if lulus {
                    self.sub_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **48**.
pub const DIFF_EXPECTED: u32 = 48;

fn run_matrix() -> KlatosAdmisiSysfsReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5359_5346, Group::Teras);
    m.catat(
        EPERM == 1 && ENOENT == 2 && EIO == 5 && EINVAL == 22 && EROFS == 30 && ELOOP == 35
            && EOVERFLOW == 36 && ENOTSUP == 74 && HOP_MAKS == 128,
        Group::Teras,
    );

    // --- inti (Teras) -----------------------------------------------------
    m.catat(sysfs_ro_admisi() == -EROFS, Group::Teras);
    m.catat(sysfs_izin_admisi() == -EPERM, Group::Teras);
    m.catat(sysfs_cari_admisi(0) == -ENOENT, Group::Teras);
    m.catat(sysfs_cari_admisi(1) == 0, Group::Teras);
    m.catat(sysfs_loop_admisi(127) == 0, Group::Teras);
    m.catat(sysfs_loop_admisi(128) == -ELOOP, Group::Teras);

    // --- subsistem (Sub) --------------------------------------------------
    m.catat(sysfs_jail_admisi(1, 0) == -EPERM, Group::Sub);
    m.catat(sysfs_jail_admisi(0, 0) == 0, Group::Sub);
    m.catat(sysfs_jail_admisi(1, 1) == 0, Group::Sub);
    m.catat(sysfs_jail_admisi(0, 1) == 0, Group::Sub);
    m.catat(sysfs_eio_admisi(0) == -EIO, Group::Sub);
    m.catat(sysfs_eio_admisi(1) == 0, Group::Sub);
    m.catat(sysfs_offset_admisi(1) == -EINVAL, Group::Sub);
    m.catat(sysfs_offset_admisi(0) == 0, Group::Sub);
    m.catat(sysfs_truncate_admisi(5) == -EPERM, Group::Sub);
    m.catat(sysfs_truncate_admisi(0) == 0, Group::Sub);
    m.catat(sysfs_boolean_admisi(1, 1) == 0, Group::Sub);
    m.catat(sysfs_boolean_admisi(0, 1) == -EINVAL, Group::Sub);
    m.catat(sysfs_boolean_admisi(1, 0) == -EINVAL, Group::Sub);
    m.catat(sysfs_coredump_admisi(5, 1) == 0, Group::Sub);
    m.catat(sysfs_coredump_admisi(5, 0) == -EINVAL, Group::Sub);
    m.catat(sysfs_coredump_admisi(0, 0) == 0, Group::Sub);
    m.catat(sysfs_power_tulis_admisi(1, 0, 0, 1) == -EOVERFLOW, Group::Sub);
    m.catat(sysfs_power_tulis_admisi(0, 1, 0, 1) == -EINVAL, Group::Sub);
    m.catat(sysfs_power_tulis_admisi(0, 0, 2, 1) == -EINVAL, Group::Sub);
    m.catat(sysfs_power_tulis_admisi(0, 0, 0, 0) == -EINVAL, Group::Sub);
    m.catat(sysfs_power_tulis_admisi(0, 0, 0, 1) == 0, Group::Sub);
    m.catat(sysfs_rom_admisi(0) == -EIO, Group::Sub);
    m.catat(sysfs_rom_admisi(1) == 0, Group::Sub);
    m.catat(sysfs_rom_pointer_admisi(0) == -EIO, Group::Sub);
    m.catat(sysfs_rom_pointer_admisi(1) == 0, Group::Sub);
    m.catat(sysfs_fisik_admisi(0) == -EPERM, Group::Sub);
    m.catat(sysfs_fisik_admisi(1) == 0, Group::Sub);
    m.catat(sysfs_panic_baca_admisi() == -ENOTSUP, Group::Sub);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // EPERM bukan EACCES pada proses di-jail.
        m.catat(sysfs_jail_admisi(1, 0) == -EPERM, Group::Sub);
        m.catat(sysfs_jail_salah(1, 0) == -13, Group::Sub);
        m.catat(sysfs_jail_admisi(1, 0) != sysfs_jail_salah(1, 0), Group::Sub);
        m.catat(sysfs_jail_admisi(0, 0) == sysfs_jail_salah(0, 0), Group::Sub);

        // ELOOP pada hop >= 128 (bukan > 128).
        m.catat(sysfs_loop_admisi(128) == -ELOOP, Group::Teras);
        m.catat(sysfs_loop_salah(128) == 0, Group::Teras);
        m.catat(sysfs_loop_admisi(128) != sysfs_loop_salah(128), Group::Teras);
        m.catat(sysfs_loop_admisi(200) == sysfs_loop_salah(200), Group::Teras);

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if sysfs_ro_admisi() != -EROFS {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Teras);
    }

    // --- diferensial produksi vs oracle (48 kasus) ------------------------
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
        ukur!(Group::Teras, sysfs_cari_admisi(v), naif_sysfs_cari_admisi(v));
        ukur!(Group::Sub, sysfs_eio_admisi(v), naif_sysfs_eio_admisi(v));
        ukur!(Group::Sub, sysfs_offset_admisi(v), naif_sysfs_offset_admisi(v));
        ukur!(Group::Sub, sysfs_rom_admisi(v as u64), naif_sysfs_rom_admisi(v as u64));
        ukur!(
            Group::Sub,
            sysfs_rom_pointer_admisi(v),
            naif_sysfs_rom_pointer_admisi(v)
        );
        ukur!(Group::Sub, sysfs_fisik_admisi(v), naif_sysfs_fisik_admisi(v));
        for b in 0..2u32 {
            ukur!(
                Group::Sub,
                sysfs_jail_admisi(v, b),
                naif_sysfs_jail_admisi(v, b)
            );
        }
    }
    for hop in [0u32, 127, 128] {
        ukur!(Group::Teras, sysfs_loop_admisi(hop), naif_sysfs_loop_admisi(hop));
    }
    for u in [0u64, 5] {
        ukur!(
            Group::Sub,
            sysfs_truncate_admisi(u),
            naif_sysfs_truncate_admisi(u)
        );
    }
    for c in 0..2u64 {
        for v in 0..2u32 {
            ukur!(
                Group::Sub,
                sysfs_boolean_admisi(c, v),
                naif_sysfs_boolean_admisi(c, v)
            );
            ukur!(
                Group::Sub,
                sysfs_coredump_admisi(c, v),
                naif_sysfs_coredump_admisi(c, v)
            );
        }
    }
    for ov in 0..2u32 {
        for of in 0..2u64 {
            for ct in 0..2u64 {
                for nv in 0..2u32 {
                    ukur!(
                        Group::Sub,
                        sysfs_power_tulis_admisi(ov, of, ct, nv),
                        naif_sysfs_power_tulis_admisi(ov, of, ct, nv)
                    );
                }
            }
        }
    }
    ukur!(Group::Teras, sysfs_ro_admisi(), -EROFS);
    ukur!(Group::Teras, sysfs_izin_admisi(), -EPERM);
    ukur!(Group::Sub, sysfs_panic_baca_admisi(), -ENOTSUP);

    m.catat(diukur == DIFF_EXPECTED, Group::Teras);
    m.catat(beda == 0, Group::Teras);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiSysfsReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        teras_checks: m.teras_checks,
        teras_passed: m.teras_passed,
        sub_checks: m.sub_checks,
        sub_passed: m.sub_passed,
        prod_ro: klatos_sysfs_counter(Counter::ProdRo as u32),
        prod_izin: klatos_sysfs_counter(Counter::ProdIzin as u32),
        prod_cari: klatos_sysfs_counter(Counter::ProdCari as u32),
        prod_loop: klatos_sysfs_counter(Counter::ProdLoop as u32),
        prod_jail: klatos_sysfs_counter(Counter::ProdJail as u32),
        prod_eio: klatos_sysfs_counter(Counter::ProdEio as u32),
        prod_offset: klatos_sysfs_counter(Counter::ProdOffset as u32),
        prod_truncate: klatos_sysfs_counter(Counter::ProdTruncate as u32),
        prod_boolean: klatos_sysfs_counter(Counter::ProdBoolean as u32),
        prod_coredump: klatos_sysfs_counter(Counter::ProdCoredump as u32),
        prod_power: klatos_sysfs_counter(Counter::ProdPower as u32),
        prod_rom: klatos_sysfs_counter(Counter::ProdRom as u32),
        prod_rom_pointer: klatos_sysfs_counter(Counter::ProdRomPointer as u32),
        prod_fisik: klatos_sysfs_counter(Counter::ProdFisik as u32),
        prod_panic: klatos_sysfs_counter(Counter::ProdPanic as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas SysFS.
#[no_mangle]
pub extern "C" fn klatos_sysfs_selftest() -> KlatosAdmisiSysfsReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5359_5346);
        assert!(EPERM == 1);
        assert!(ENOENT == 2);
        assert!(EIO == 5);
        assert!(EINVAL == 22);
        assert!(EROFS == 30);
        assert!(ELOOP == 35);
        assert!(EOVERFLOW == 36);
        assert!(ENOTSUP == 74);
        assert!(HOP_MAKS == 128);
        assert!(Counter::ProdRo as usize == 0);
        assert!(Counter::ProdPanic as usize == 14);
        assert!(N_COUNTER == 15);
        assert!(DIFF_EXPECTED == 48);

        assert!(sysfs_ro_admisi() == -EROFS);
        assert!(sysfs_jail_admisi(1, 0) == -EPERM);
        assert!(sysfs_loop_admisi(128) == -ELOOP);

        // 8 (tag) + 24*4 (u32) + 4 (status) = 104.
        assert!(core::mem::size_of::<KlatosAdmisiSysfsReport>() == 104);
        assert!(core::mem::align_of::<KlatosAdmisiSysfsReport>() == 8);
    };
}
