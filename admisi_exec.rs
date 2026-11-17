// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.77 / WS7 — **OTORITAS ADMISI & KEBIJAKAN FORMAT `execve`**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `E2BIG` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 7).
pub const E2BIG: i32 = 7;
/// `ENOEXEC` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 8).
pub const ENOEXEC: i32 = 8;
/// `EACCES` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 13).
pub const EACCES: i32 = 13;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `ELOOP` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 35).
pub const ELOOP: i32 = 35;
/// `ENOBUFS` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 59).
pub const ENOBUFS: i32 = 59;

/// Tag laporan; cermin `KlatosAdmisiExecReport` di `Jowo/Kontrak/KlatSeam77.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5345_5845; // "KLATSEXE"

/// Batas rekursi `#!` (`Process::exec`): `recursion_depth > 2` → `ELOOP`.
pub const MAKS_REKURSI: u32 = 2;
/// Ukuran minimum berkas ELF (`metadata.size < 3` → `ENOEXEC`).
pub const MIN_UKURAN_BERKAS: u64 = 3;

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_exec_recursion` + `_metadata` + `_argc` dari produksi.
    ProdAdmisi = 0,
    /// `klatos_exec_elf` dari produksi.
    ProdElf = 1,
    /// `klatos_exec_shebang_awal` + `_shebang_putusan` dari produksi.
    ProdShebang = 2,
    /// `klatos_exec_stack_ok` dari produksi.
    ProdStack = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Kedalaman rekursi `#!`. Transkripsi `Process::exec:910-913`:
///
/// ```cpp
/// if (recursion_depth > 2) { dbgln("SHENANIGANS! ..."); return ELOOP; }
/// ```
///
/// Mengembalikan `0` (lanjut) atau `-ELOOP`.
#[inline]
pub const fn recursion_ok(depth: u32, maks: u32) -> i32 {
    if depth > maks {
        return -ELOOP;
    }
    0
}

/// Admisi metadata berkas. Transkripsi DUA situs:
///
/// ```cpp
/// // Process::exec:923-928  (program utama)
/// if (!metadata.is_regular_file()) return EACCES;
/// if (metadata.size < 3)           return ENOEXEC;
///
/// // find_elf_interpreter_for_executable:880-888  (interpreter)
/// if (!interp_metadata.is_regular_file())              return ENOEXEC;
/// if (interp_metadata.size < sizeof(Elf_Ehdr))         return ENOEXEC;
/// ```
///
/// Urutan PENTING: regular-file diperiksa LEBIH DULU. `errno_non_reguler`
/// DISUPLAI C++ (situs menentukan `EACCES` vs `ENOEXEC`). Mengembalikan `0`,
/// `errno_non_reguler`, atau `-ENOEXEC`.
#[inline]
pub const fn metadata_admisi(reguler: u32, ukuran: u64, min_ukuran: u64, errno_non_reguler: i32) -> i32 {
    if reguler == 0 {
        return errno_non_reguler;
    }
    if ukuran < min_ukuran {
        return -ENOEXEC;
    }
    0
}

/// Admisi header ELF. Transkripsi `Process::exec:951-965`:
///
/// ```cpp
/// if (nread < sizeof(Elf_Ehdr))                     return ENOEXEC;
/// if (main_program_header->e_machine != host_elf_machine()) return ENOEXEC;
/// if (!ELF::validate_elf_header(header, size))      return ENOEXEC;
/// ```
///
/// Mengembalikan `0` (lanjut) atau `-ENOEXEC`.
#[inline]
pub const fn elf_admisi(
    nread: u64,
    ukuran_header: u64,
    e_machine: u32,
    mesin_host: u32,
    header_valid: u32,
) -> i32 {
    if nread < ukuran_header {
        return -ENOEXEC;
    }
    if e_machine != mesin_host {
        return -ENOEXEC;
    }
    if header_valid == 0 {
        return -ENOEXEC;
    }
    0
}

/// Admisi argc/env. Transkripsi `Process::sys$execve:1000-1007`:
///
/// ```cpp
/// if (params.arguments.length > ARG_MAX || params.environment.length > ARG_MAX)
///     return E2BIG;
/// if (params.arguments.length == 0)
///     return EINVAL;
/// ```
///
/// Mengembalikan `0`, `-E2BIG`, atau `-EINVAL`.
#[inline]
pub const fn argc_admisi(n_args: u32, n_env: u32, arg_max: u32) -> i32 {
    if n_args > arg_max || n_env > arg_max {
        return -E2BIG;
    }
    if n_args == 0 {
        return -EINVAL;
    }
    0
}

/// Deteksi awalan `#!`. Transkripsi `is_executable_starting_with_shebang:736-739`:
///
/// ```cpp
/// return preliminary_buffer.size() > 2
///     && preliminary_buffer[0] == '#' && preliminary_buffer[1] == '!';
/// ```
///
/// `b0`/`b1` DISUPLAI C++ (byte pertama/kedua). Mengembalikan `1`/`0`.
#[inline]
pub const fn shebang_awal(size: u32, b0: u32, b1: u32) -> u32 {
    let pagar = b'#' as u32;
    let seru = b'!' as u32;
    if size > 2 && b0 == pagar && b1 == seru {
        1
    } else {
        0
    }
}

/// Verdict baris `#!`. Transkripsi `find_shebang_interpreter_for_executable:741-784`:
///
/// ```cpp
/// ...scan sampai '\n'...
/// if (!found_line_break) return ENOBUFS;
/// ...kumpulkan kata...
/// if (interpreter_words.is_empty()) return ENOEXEC;
/// return interpreter_words;
/// ```
///
/// `found_line_break`/`n_kata` DISUPLAI C++ setelah pemindaian token
/// (mekanisme). Mengembalikan `0` (lanjut), `-ENOBUFS`, atau `-ENOEXEC`.
#[inline]
pub const fn shebang_putusan(found_line_break: u32, n_kata: u32) -> i32 {
    if found_line_break == 0 {
        return -ENOBUFS;
    }
    if n_kata == 0 {
        return -ENOEXEC;
    }
    0
}

/// Validasi ukuran tumpukan. Transkripsi `validate_stack_size:47-79` (bagian
/// perbandingan akhir):
///
/// ```cpp
/// if (total_arguments_size   > Process::max_arguments_size)   return false;
/// if (total_environment_size > Process::max_environment_size) return false;
/// if (total_auxiliary_size   > Process::max_auxiliary_size)   return false;
/// return true;
/// ```
///
/// Total DISUPLAI C++ setelah penjumlahan (mekanisme). Mengembalikan `1`/`0`.
#[inline]
pub const fn stack_ukuran_ok(
    total_args: u64,
    total_env: u64,
    total_aux: u64,
    maks_args: u64,
    maks_env: u64,
    maks_aux: u64,
) -> u32 {
    if total_args > maks_args {
        return 0;
    }
    if total_env > maks_env {
        return 0;
    }
    if total_aux > maks_aux {
        return 0;
    }
    1
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_recursion_ok(depth: u32, maks: u32) -> i32 {
    match depth {
        d if d > maks => -ELOOP,
        _ => 0,
    }
}

fn naif_metadata_admisi(reguler: u32, ukuran: u64, min_ukuran: u64, errno_non_reguler: i32) -> i32 {
    if reguler != 0 && ukuran >= min_ukuran {
        0
    } else if reguler == 0 {
        errno_non_reguler
    } else {
        -ENOEXEC
    }
}

fn naif_elf_admisi(
    nread: u64,
    ukuran_header: u64,
    e_machine: u32,
    mesin_host: u32,
    header_valid: u32,
) -> i32 {
    let ok = nread >= ukuran_header && e_machine == mesin_host && header_valid != 0;
    if ok {
        0
    } else {
        -ENOEXEC
    }
}

fn naif_argc_admisi(n_args: u32, n_env: u32, arg_max: u32) -> i32 {
    let terlalu_banyak = n_args > arg_max || n_env > arg_max;
    if terlalu_banyak {
        -E2BIG
    } else if n_args == 0 {
        -EINVAL
    } else {
        0
    }
}

fn naif_shebang_awal(size: u32, b0: u32, b1: u32) -> u32 {
    u32::from(size > 2 && b0 == 0x23 && b1 == 0x21)
}

fn naif_shebang_putusan(found_line_break: u32, n_kata: u32) -> i32 {
    if found_line_break == 0 {
        -ENOBUFS
    } else if n_kata == 0 {
        -ENOEXEC
    } else {
        0
    }
}

fn naif_stack_ukuran_ok(
    total_args: u64,
    total_env: u64,
    total_aux: u64,
    maks_args: u64,
    maks_env: u64,
    maks_aux: u64,
) -> u32 {
    let dalam_batas =
        total_args <= maks_args && total_env <= maks_env && total_aux <= maks_aux;
    u32::from(dalam_batas)
}

/// Kontrol NEGATIF: `metadata_admisi` yang MEMERIKSA UKURAN LEBIH DULU
/// (urutan terbalik dari produksi). Sengaja salah; berdaya pada
/// `(reguler = 0, ukuran = 0)` — produksi `-EACCES`, yang salah `-ENOEXEC`.
fn metadata_admisi_salah_urutan(reguler: u32, ukuran: u64, min_ukuran: u64, errno_non_reguler: i32) -> i32 {
    if ukuran < min_ukuran {
        return -ENOEXEC;
    }
    if reguler == 0 {
        return errno_non_reguler;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Kedalaman rekursi. Lihat [`recursion_ok`].
#[no_mangle]
pub extern "C" fn klatos_exec_recursion(depth: u32, maks: u32) -> i32 {
    bump(Counter::ProdAdmisi);
    recursion_ok(depth, maks)
}

/// Admisi metadata. Lihat [`metadata_admisi`].
#[no_mangle]
pub extern "C" fn klatos_exec_metadata(reguler: u32, ukuran: u64, min_ukuran: u64, errno_non_reguler: i32) -> i32 {
    bump(Counter::ProdAdmisi);
    metadata_admisi(reguler, ukuran, min_ukuran, errno_non_reguler)
}

/// Admisi header ELF. Lihat [`elf_admisi`].
#[no_mangle]
pub extern "C" fn klatos_exec_elf(
    nread: u64,
    ukuran_header: u64,
    e_machine: u32,
    mesin_host: u32,
    header_valid: u32,
) -> i32 {
    bump(Counter::ProdElf);
    elf_admisi(nread, ukuran_header, e_machine, mesin_host, header_valid)
}

/// Admisi argc/env. Lihat [`argc_admisi`].
#[no_mangle]
pub extern "C" fn klatos_exec_argc(n_args: u32, n_env: u32, arg_max: u32) -> i32 {
    bump(Counter::ProdAdmisi);
    argc_admisi(n_args, n_env, arg_max)
}

/// Deteksi `#!`. Lihat [`shebang_awal`].
#[no_mangle]
pub extern "C" fn klatos_exec_shebang_awal(size: u32, b0: u32, b1: u32) -> u32 {
    bump(Counter::ProdShebang);
    shebang_awal(size, b0, b1)
}

/// Verdict baris `#!`. Lihat [`shebang_putusan`].
#[no_mangle]
pub extern "C" fn klatos_exec_shebang_putusan(found_line_break: u32, n_kata: u32) -> i32 {
    bump(Counter::ProdShebang);
    shebang_putusan(found_line_break, n_kata)
}

/// Validasi ukuran tumpukan. Lihat [`stack_ukuran_ok`].
#[no_mangle]
pub extern "C" fn klatos_exec_stack_ok(
    total_args: u64,
    total_env: u64,
    total_aux: u64,
    maks_args: u64,
    maks_env: u64,
    maks_aux: u64,
) -> u32 {
    bump(Counter::ProdStack);
    stack_ukuran_ok(total_args, total_env, total_aux, maks_args, maks_env, maks_aux)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_exec_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_exec_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiExecReport` di `Jowo/Kontrak/KlatSeam77.h`.
#[repr(C)]
pub struct KlatosAdmisiExecReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub adm_checks: u32,
    pub adm_passed: u32,
    pub elf_checks: u32,
    pub elf_passed: u32,
    pub shebang_checks: u32,
    pub shebang_passed: u32,
    pub stack_checks: u32,
    pub stack_passed: u32,
    pub prod_admisi: u32,
    pub prod_elf: u32,
    pub prod_shebang: u32,
    pub prod_stack: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Admisi,
    Elf,
    Shebang,
    Stack,
}

struct Matrix {
    checks: u32,
    passed: u32,
    adm_checks: u32,
    adm_passed: u32,
    elf_checks: u32,
    elf_passed: u32,
    shebang_checks: u32,
    shebang_passed: u32,
    stack_checks: u32,
    stack_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            adm_checks: 0,
            adm_passed: 0,
            elf_checks: 0,
            elf_passed: 0,
            shebang_checks: 0,
            shebang_passed: 0,
            stack_checks: 0,
            stack_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Admisi => {
                self.adm_checks += 1;
                if lulus {
                    self.adm_passed += 1;
                }
            }
            Group::Elf => {
                self.elf_checks += 1;
                if lulus {
                    self.elf_passed += 1;
                }
            }
            Group::Shebang => {
                self.shebang_checks += 1;
                if lulus {
                    self.shebang_passed += 1;
                }
            }
            Group::Stack => {
                self.stack_checks += 1;
                if lulus {
                    self.stack_passed += 1;
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
/// recursion `8 × 3` = 24; metadata `2 × 5 × 2` = 20; elf `2×2×2×2` = 16;
/// argc `6 × 3` = 18; shebang_awal `4×2×2` = 16; shebang_putusan `2 × 4` = 8;
/// stack `2×2×2` = 8 ⇒ 24+20+16+18+16+8+8 = **110**.
pub const DIFF_EXPECTED: u32 = 110;

/// Nilai ABI sintetis (nilai NYATA diambil dari C++ saat produksi).
const HOST_MACHINE_SINTETIS: u32 = 3;
const ARG_MAX_SINTETIS: u32 = 128;

fn run_matrix() -> KlatosAdmisiExecReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5345_5845, Group::Admisi);
    m.catat(E2BIG == 7 && ENOEXEC == 8, Group::Admisi);
    m.catat(EACCES == 13 && EINVAL == 22, Group::Admisi);
    m.catat(ELOOP == 35 && ENOBUFS == 59, Group::Admisi);
    m.catat(MAKS_REKURSI == 2 && MIN_UKURAN_BERKAS == 3, Group::Admisi);

    // --- rekursi ----------------------------------------------------------
    m.catat(recursion_ok(0, MAKS_REKURSI) == 0, Group::Admisi);
    m.catat(recursion_ok(2, MAKS_REKURSI) == 0, Group::Admisi);
    m.catat(recursion_ok(3, MAKS_REKURSI) == -ELOOP, Group::Admisi);
    m.catat(recursion_ok(99, MAKS_REKURSI) == -ELOOP, Group::Admisi);

    // --- metadata ---------------------------------------------------------
    // Situs program utama: non-reguler → EACCES.
    m.catat(metadata_admisi(1, 3, MIN_UKURAN_BERKAS, -EACCES) == 0, Group::Admisi);
    m.catat(metadata_admisi(1, 100, MIN_UKURAN_BERKAS, -EACCES) == 0, Group::Admisi);
    // regular-file diperiksa LEBIH DULU: non-reguler walau ukuran 0 → EACCES.
    m.catat(metadata_admisi(0, 0, MIN_UKURAN_BERKAS, -EACCES) == -EACCES, Group::Admisi);
    m.catat(metadata_admisi(0, 100, MIN_UKURAN_BERKAS, -EACCES) == -EACCES, Group::Admisi);
    m.catat(metadata_admisi(1, 2, MIN_UKURAN_BERKAS, -EACCES) == -ENOEXEC, Group::Admisi);
    m.catat(metadata_admisi(1, 0, MIN_UKURAN_BERKAS, -EACCES) == -ENOEXEC, Group::Admisi);
    // Situs interpreter: non-reguler → ENOEXEC (bukan EACCES).
    m.catat(metadata_admisi(0, 0, 64, -ENOEXEC) == -ENOEXEC, Group::Admisi);
    m.catat(metadata_admisi(1, 64, 64, -ENOEXEC) == 0, Group::Admisi);
    m.catat(metadata_admisi(1, 63, 64, -ENOEXEC) == -ENOEXEC, Group::Admisi);

    // --- ELF --------------------------------------------------------------
    m.catat(elf_admisi(64, 64, 3, 3, 1) == 0, Group::Elf);
    m.catat(elf_admisi(63, 64, 3, 3, 1) == -ENOEXEC, Group::Elf);
    m.catat(elf_admisi(64, 64, 9, 3, 1) == -ENOEXEC, Group::Elf);
    m.catat(elf_admisi(64, 64, 3, 3, 0) == -ENOEXEC, Group::Elf);
    m.catat(elf_admisi(0, 64, 9, 3, 0) == -ENOEXEC, Group::Elf);

    // --- argc -------------------------------------------------------------
    m.catat(argc_admisi(1, 0, ARG_MAX_SINTETIS) == 0, Group::Admisi);
    m.catat(argc_admisi(ARG_MAX_SINTETIS, ARG_MAX_SINTETIS, ARG_MAX_SINTETIS) == 0, Group::Admisi);
    m.catat(argc_admisi(ARG_MAX_SINTETIS + 1, 0, ARG_MAX_SINTETIS) == -E2BIG, Group::Admisi);
    m.catat(argc_admisi(1, ARG_MAX_SINTETIS + 1, ARG_MAX_SINTETIS) == -E2BIG, Group::Admisi);
    // argc 0 DAN env kebesaran: E2BIG menang (diperiksa lebih dulu).
    m.catat(argc_admisi(0, ARG_MAX_SINTETIS + 1, ARG_MAX_SINTETIS) == -E2BIG, Group::Admisi);
    m.catat(argc_admisi(0, 0, ARG_MAX_SINTETIS) == -EINVAL, Group::Admisi);

    // --- shebang ----------------------------------------------------------
    m.catat(shebang_awal(3, 0x23, 0x21) == 1, Group::Shebang);
    m.catat(shebang_awal(2, 0x23, 0x21) == 0, Group::Shebang); // terlalu pendek
    m.catat(shebang_awal(10, 0x23, 0x78) == 0, Group::Shebang); // bukan '!'
    m.catat(shebang_awal(10, 0x78, 0x21) == 0, Group::Shebang); // bukan '#'
    m.catat(shebang_putusan(1, 1) == 0, Group::Shebang);
    m.catat(shebang_putusan(0, 5) == -ENOBUFS, Group::Shebang);
    m.catat(shebang_putusan(1, 0) == -ENOEXEC, Group::Shebang);
    // tanpa line-break DAN tanpa kata: ENOBUFS menang.
    m.catat(shebang_putusan(0, 0) == -ENOBUFS, Group::Shebang);

    // --- tumpukan ---------------------------------------------------------
    m.catat(stack_ukuran_ok(1, 1, 1, 10, 10, 10) == 1, Group::Stack);
    m.catat(stack_ukuran_ok(10, 10, 10, 10, 10, 10) == 1, Group::Stack); // batas = OK
    m.catat(stack_ukuran_ok(11, 1, 1, 10, 10, 10) == 0, Group::Stack);
    m.catat(stack_ukuran_ok(1, 11, 1, 10, 10, 10) == 0, Group::Stack);
    m.catat(stack_ukuran_ok(1, 1, 11, 10, 10, 10) == 0, Group::Stack);

    // --- kontrol NEGATIF: urutan pemeriksaan metadata berdaya -------------
    {
        let benar = metadata_admisi(0, 0, MIN_UKURAN_BERKAS, -EACCES);
        let salah = metadata_admisi_salah_urutan(0, 0, MIN_UKURAN_BERKAS, -EACCES);
        m.catat(benar == -EACCES, Group::Admisi);
        m.catat(salah == -ENOEXEC, Group::Admisi);
        m.catat(benar != salah, Group::Admisi);
        // Pada berkas regular, keduanya HARUS sama.
        m.catat(
            metadata_admisi(1, 0, MIN_UKURAN_BERKAS, -EACCES)
                == metadata_admisi_salah_urutan(1, 0, MIN_UKURAN_BERKAS, -EACCES),
            Group::Admisi,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // recursion: 8 x 3 = 24
    for d in 0..8u32 {
        for &maks in &[MAKS_REKURSI, 3, 5] {
            if recursion_ok(d, maks) != naif_recursion_ok(d, maks) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // metadata: 2 x 5 x 2 = 20
    for r in 0..2u32 {
        for &u in &[0u64, 2, 3, 10, 100] {
            for &enr in &[-EACCES, -ENOEXEC] {
                if metadata_admisi(r, u, MIN_UKURAN_BERKAS, enr)
                    != naif_metadata_admisi(r, u, MIN_UKURAN_BERKAS, enr)
                {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // elf: 2 x 2 x 2 x 2 = 16
    for n in 0..2u64 {
        for h in 0..2u64 {
            for e in 0..2u32 {
                for v in 0..2u32 {
                    let nr = if n == 0 { 63 } else { 64 };
                    let hs = if h == 0 { 64 } else { 32 };
                    let em = if e == 0 { HOST_MACHINE_SINTETIS } else { 9 };
                    if elf_admisi(nr, hs, em, HOST_MACHINE_SINTETIS, v)
                        != naif_elf_admisi(nr, hs, em, HOST_MACHINE_SINTETIS, v)
                    {
                        beda += 1;
                    }
                    diukur += 1;
                }
            }
        }
    }
    // argc: 6 x 3 = 18
    for a in 0..6u32 {
        for &e in &[0u32, ARG_MAX_SINTETIS, ARG_MAX_SINTETIS + 1] {
            if argc_admisi(a, e, ARG_MAX_SINTETIS) != naif_argc_admisi(a, e, ARG_MAX_SINTETIS) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // shebang_awal: 4 x 2 x 2 = 16
    for s in 0..4u32 {
        for &b0 in &[0x23u32, 0x78] {
            for &b1 in &[0x21u32, 0x78] {
                if shebang_awal(s, b0, b1) != naif_shebang_awal(s, b0, b1) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // shebang_putusan: 2 x 4 = 8
    for f in 0..2u32 {
        for k in 0..4u32 {
            if shebang_putusan(f, k) != naif_shebang_putusan(f, k) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // stack: 2 x 2 x 2 = 8
    for a in 0..2u64 {
        for e in 0..2u64 {
            for x in 0..2u64 {
                let ta = if a == 0 { 1 } else { 11 };
                let te = if e == 0 { 1 } else { 11 };
                let tx = if x == 0 { 1 } else { 11 };
                if stack_ukuran_ok(ta, te, tx, 10, 10, 10) != naif_stack_ukuran_ok(ta, te, tx, 10, 10, 10) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Admisi);
    m.catat(beda == 0, Group::Admisi);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiExecReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        adm_checks: m.adm_checks,
        adm_passed: m.adm_passed,
        elf_checks: m.elf_checks,
        elf_passed: m.elf_passed,
        shebang_checks: m.shebang_checks,
        shebang_passed: m.shebang_passed,
        stack_checks: m.stack_checks,
        stack_passed: m.stack_passed,
        prod_admisi: klatos_exec_counter(Counter::ProdAdmisi as u32),
        prod_elf: klatos_exec_counter(Counter::ProdElf as u32),
        prod_shebang: klatos_exec_counter(Counter::ProdShebang as u32),
        prod_stack: klatos_exec_counter(Counter::ProdStack as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi & kebijakan format `execve`.
#[no_mangle]
pub extern "C" fn klatos_exec_selftest() -> KlatosAdmisiExecReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5345_5845);
        assert!(E2BIG == 7);
        assert!(ENOEXEC == 8);
        assert!(EACCES == 13);
        assert!(EINVAL == 22);
        assert!(ELOOP == 35);
        assert!(ENOBUFS == 59);
        assert!(MAKS_REKURSI == 2);
        assert!(MIN_UKURAN_BERKAS == 3);
        assert!(Counter::ProdAdmisi as usize == 0);
        assert!(Counter::ProdElf as usize == 1);
        assert!(Counter::ProdShebang as usize == 2);
        assert!(Counter::ProdStack as usize == 3);
        assert!(N_COUNTER == 4);
        assert!(DIFF_EXPECTED == 110);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76 -> align 8 -> 80.
        assert!(core::mem::size_of::<KlatosAdmisiExecReport>() == 80);
        assert!(core::mem::align_of::<KlatosAdmisiExecReport>() == 8);
    };
}
