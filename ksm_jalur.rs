// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.86 / WS4 — **OTORITAS GERBANG IZIN VEIL PER-AKSES**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `ENOENT` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 2).
pub const ENOENT: i32 = 2;
/// `EACCES` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 13).
pub const EACCES: i32 = 13;

// --- bit `IzinAksesJalur` (cermin `Jowo/Kontrak/AksesJalur.h`) ----------------

/// `IzinAksesJalur::Read`.
pub const UA_READ: u32 = 1;
/// `IzinAksesJalur::Write`.
pub const UA_WRITE: u32 = 2;
/// `IzinAksesJalur::Execute`.
pub const UA_EXECUTE: u32 = 4;
/// `IzinAksesJalur::CreateOrRemove`.
pub const UA_CREATE_OR_REMOVE: u32 = 8;
/// `IzinAksesJalur::Browse`.
pub const UA_BROWSE: u32 = 16;

// --- bit flag `open` (cermin `Jowo/Kontrak/POSIX/fcntl.h` + VFS internal) ---

/// `O_RDONLY`.
pub const O_RDONLY: u32 = 1 << 0;
/// `O_WRONLY`.
pub const O_WRONLY: u32 = 1 << 1;
/// `O_EXEC`.
pub const O_EXEC: u32 = 1 << 2;
/// `O_CREAT`.
pub const O_CREAT: u32 = 1 << 3;
/// `O_DIRECTORY`.
pub const O_DIRECTORY: u32 = 1 << 9;
/// `O_UNLINK_INTERNAL` — cermin `KlatKernel/Efek/Berkas/VirtualFileSystem.h:36`.
pub const O_UNLINK_INTERNAL: u32 = 1 << 30;

// --- alasan (untuk pemilihan pesan `dbgln` di C++) -------------------------

/// Lolos.
pub const ALASAN_LOLOS: u32 = 0;
/// Belum di-unakses_jalur ⇒ `ENOENT`, tanpa akhiran izin.
pub const ALASAN_AKSES_TIDAK_DIBERIKAN: u32 = 1;
/// Ditolak karena butuh `c` (`"'c'"`).
pub const ALASAN_TOLAK_C: u32 = 2;
/// Ditolak karena butuh `r` atau `b` (`"'r' or 'b'"`).
pub const ALASAN_TOLAK_RB: u32 = 3;
/// Ditolak karena butuh `r` (`"'r'"`).
pub const ALASAN_TOLAK_R: u32 = 4;
/// Ditolak karena butuh `w` (`"'w'"`).
pub const ALASAN_TOLAK_W: u32 = 5;
/// Ditolak karena butuh `x` (`"'x'"`).
pub const ALASAN_TOLAK_X: u32 = 6;

/// Tag laporan; cermin `KlatosAdmisiAksesJalurReport` di `Jowo/Kontrak/KlatSeam86.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4741_5445; // "KLATGATE"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Seluruh panggilan produksi.
    ProdCalls = 0,
    /// Verdict lolos.
    ProdOk = 1,
    /// Verdict `ENOENT`.
    ProdEnone = 2,
    /// Verdict `EACCES`.
    ProdEacces = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi, matriks, dan alasan
// ---------------------------------------------------------------------------

/// Klasifikasi tunggal. Mengembalikan salah satu `ALASAN_*`.
///
/// Ini SATU-SATUNYA salinan logika; [`ksm_jalur_internal`] dan [`alasan_akses_jalur`]
/// hanyalah dua tampilan darinya.
#[inline]
pub const fn klasifikasi_akses_jalur(permissions: u32, options: u32) -> u32 {
    if permissions == 0 {
        return ALASAN_AKSES_TIDAK_DIBERIKAN;
    }
    if options & O_CREAT != 0 && permissions & UA_CREATE_OR_REMOVE == 0 {
        return ALASAN_TOLAK_C;
    }
    if options & O_UNLINK_INTERNAL != 0 {
        if permissions & UA_CREATE_OR_REMOVE == 0 {
            return ALASAN_TOLAK_C;
        }
        // KELUAR AWAL — persis C++ historis.
        return ALASAN_LOLOS;
    }
    if options & O_RDONLY != 0 {
        if options & O_DIRECTORY != 0 {
            if permissions & (UA_READ | UA_BROWSE) == 0 {
                return ALASAN_TOLAK_RB;
            }
        } else if permissions & UA_READ == 0 {
            return ALASAN_TOLAK_R;
        }
    }
    if options & O_WRONLY != 0 && permissions & UA_WRITE == 0 {
        return ALASAN_TOLAK_W;
    }
    if options & O_EXEC != 0 && permissions & UA_EXECUTE == 0 {
        return ALASAN_TOLAK_X;
    }
    ALASAN_LOLOS
}

/// Verdict produksi. Mengembalikan `0` (lolos), `-ENOENT`, atau `-EACCES`.
#[inline]
pub const fn ksm_jalur_internal(permissions: u32, options: u32) -> i32 {
    let alasan = klasifikasi_akses_jalur(permissions, options);
    if alasan == ALASAN_LOLOS {
        0
    } else if alasan == ALASAN_AKSES_TIDAK_DIBERIKAN {
        -ENOENT
    } else {
        -EACCES
    }
}

/// Alasan penolakan (atau [`ALASAN_LOLOS`]). Hanya untuk merender pesan.
#[inline]
pub const fn alasan_akses_jalur(permissions: u32, options: u32) -> u32 {
    klasifikasi_akses_jalur(permissions, options)
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

/// Oracle: rantai `if` bersarang yang mengembalikan errno langsung, tanpa
/// enum alasan. Sengaja ditulis ulang supaya kesalahan salin-tempel pada
/// [`klasifikasi_akses_jalur`] tidak ikut tersalin.
fn model_naif_ksm_jalur(permissions: u32, options: u32) -> i32 {
    if permissions == 0 {
        return -ENOENT;
    }
    let punya_c = permissions & UA_CREATE_OR_REMOVE != 0;
    if options & O_CREAT != 0 && !punya_c {
        return -EACCES;
    }
    if options & O_UNLINK_INTERNAL != 0 {
        if punya_c {
            return 0;
        }
        return -EACCES;
    }
    if options & O_RDONLY != 0 {
        if options & O_DIRECTORY != 0 {
            let r_atau_b = permissions & UA_READ != 0 || permissions & UA_BROWSE != 0;
            if !r_atau_b {
                return -EACCES;
            }
        } else if permissions & UA_READ == 0 {
            return -EACCES;
        }
    }
    if options & O_WRONLY != 0 && permissions & UA_WRITE == 0 {
        return -EACCES;
    }
    if options & O_EXEC != 0 && permissions & UA_EXECUTE == 0 {
        return -EACCES;
    }
    0
}

/// Kontrol NEGATIF: versi yang **MENJATUHKAN keluar-awal `O_UNLINK_INTERNAL`**.
/// Sengaja SALAH. Berdaya pada `O_UNLINK_INTERNAL | O_RDONLY` dengan bit `c`
/// ada tetapi bit `r` TIDAK ada: produksi `0` (unlink internal tidak membaca),
/// yang salah `-EACCES`.
fn ksm_jalur_kontrol_negatif(permissions: u32, options: u32) -> i32 {
    if permissions == 0 {
        return -ENOENT;
    }
    if options & O_CREAT != 0 && permissions & UA_CREATE_OR_REMOVE == 0 {
        return -EACCES;
    }
    if options & O_UNLINK_INTERNAL != 0 && permissions & UA_CREATE_OR_REMOVE == 0 {
        return -EACCES;
    }
    // (keluar-awal DIJATUHKAN)
    if options & O_RDONLY != 0 {
        if options & O_DIRECTORY != 0 {
            if permissions & (UA_READ | UA_BROWSE) == 0 {
                return -EACCES;
            }
        } else if permissions & UA_READ == 0 {
            return -EACCES;
        }
    }
    if options & O_WRONLY != 0 && permissions & UA_WRITE == 0 {
        return -EACCES;
    }
    if options & O_EXEC != 0 && permissions & UA_EXECUTE == 0 {
        return -EACCES;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Verdict gerbang akses_jalur. Lihat [`ksm_jalur_internal`].
#[no_mangle]
pub extern "C" fn klatos_ksm_jalur(permissions: u32, options: u32) -> i32 {
    bump(Counter::ProdCalls);
    let rc = ksm_jalur_internal(permissions, options);
    if rc == 0 {
        bump(Counter::ProdOk);
    } else if rc == -ENOENT {
        bump(Counter::ProdEnone);
    } else {
        bump(Counter::ProdEacces);
    }
    rc
}

/// Alasan penolakan (untuk pemilihan pesan). Lihat [`alasan_akses_jalur`].
#[no_mangle]
pub extern "C" fn klatos_ksm_jalur_alasan(permissions: u32, options: u32) -> u32 {
    alasan_akses_jalur(permissions, options)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_ksm_jalur_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_ksm_jalur_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiAksesJalurReport` di `Jowo/Kontrak/KlatSeam86.h`.
#[repr(C)]
pub struct KlatosAdmisiAksesJalurReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_ok: u32,
    pub prod_enoent: u32,
    pub prod_eacces: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub alasan_checks: u32,
    pub alasan_passed: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Peta,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    peta_checks: u32,
    peta_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            peta_checks: 0,
            peta_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Peta => {
                self.peta_checks += 1;
                if lulus {
                    self.peta_passed += 1;
                }
            }
            Group::Neg => {
                self.neg_checks += 1;
                if lulus {
                    self.neg_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Enam bit `options` yang relevan bagi gerbang akses_jalur.
const OPSI_BIT: [u32; 6] = [O_RDONLY, O_WRONLY, O_EXEC, O_CREAT, O_DIRECTORY, O_UNLINK_INTERNAL];

/// Jumlah kasus diferensial: 32 kombinasi izin × 64 kombinasi flag = **2048**.
pub const DIFF_EXPECTED: u32 = 2048;

fn run_matrix() -> KlatosAdmisiAksesJalurReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;
    let mut alasan_checks: u32 = 0;
    let mut alasan_passed: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4741_5445, Group::Peta);
    m.catat(ENOENT == 2 && EACCES == 13, Group::Peta);
    m.catat(
        UA_READ == 1
            && UA_WRITE == 2
            && UA_EXECUTE == 4
            && UA_CREATE_OR_REMOVE == 8
            && UA_BROWSE == 16,
        Group::Peta,
    );
    m.catat(
        O_RDONLY == 1
            && O_WRONLY == 2
            && O_EXEC == 4
            && O_CREAT == 8
            && O_DIRECTORY == 512
            && O_UNLINK_INTERNAL == (1 << 30),
        Group::Peta,
    );

    // --- kasus utama ------------------------------------------------------
    // Belum di-unakses_jalur menang atas segalanya.
    m.catat(ksm_jalur_internal(0, 0) == -ENOENT, Group::Peta);
    m.catat(ksm_jalur_internal(0, O_CREAT | O_RDONLY) == -ENOENT, Group::Peta);
    // Baca murni.
    m.catat(ksm_jalur_internal(UA_READ, O_RDONLY) == 0, Group::Peta);
    m.catat(ksm_jalur_internal(UA_WRITE, O_RDONLY) == -EACCES, Group::Peta);
    // Tulis.
    m.catat(ksm_jalur_internal(UA_WRITE, O_WRONLY) == 0, Group::Peta);
    m.catat(ksm_jalur_internal(UA_READ, O_WRONLY) == -EACCES, Group::Peta);
    // Direktori: `r` ATAU `b` cukup.
    m.catat(ksm_jalur_internal(UA_READ, O_RDONLY | O_DIRECTORY) == 0, Group::Peta);
    m.catat(ksm_jalur_internal(UA_BROWSE, O_RDONLY | O_DIRECTORY) == 0, Group::Peta);
    m.catat(ksm_jalur_internal(UA_WRITE, O_RDONLY | O_DIRECTORY) == -EACCES, Group::Peta);
    // `b` saja TIDAK cukup untuk baca non-direktori.
    m.catat(ksm_jalur_internal(UA_BROWSE, O_RDONLY) == -EACCES, Group::Peta);
    // Exec.
    m.catat(ksm_jalur_internal(UA_EXECUTE, O_EXEC) == 0, Group::Peta);
    m.catat(ksm_jalur_internal(UA_READ, O_EXEC) == -EACCES, Group::Peta);
    // Buat.
    m.catat(ksm_jalur_internal(UA_CREATE_OR_REMOVE, O_CREAT) == 0, Group::Peta);
    m.catat(ksm_jalur_internal(UA_READ, O_CREAT) == -EACCES, Group::Peta);
    // Unlink internal: keluar awal, `r` TIDAK diminta.
    m.catat(ksm_jalur_internal(UA_CREATE_OR_REMOVE, O_UNLINK_INTERNAL) == 0, Group::Peta);
    m.catat(
        ksm_jalur_internal(UA_CREATE_OR_REMOVE, O_UNLINK_INTERNAL | O_RDONLY | O_WRONLY) == 0,
        Group::Peta,
    );
    m.catat(ksm_jalur_internal(UA_READ, O_UNLINK_INTERNAL) == -EACCES, Group::Peta);
    // `O_CREAT` diperiksa lebih dulu tetapi tidak keluar awal.
    m.catat(ksm_jalur_internal(UA_CREATE_OR_REMOVE, O_CREAT | O_WRONLY) == -EACCES, Group::Peta);

    // --- alasan -----------------------------------------------------------
    {
        let kasus: [(u32, u32, u32); 7] = [
            (UA_READ, O_RDONLY, ALASAN_LOLOS),
            (0, O_RDONLY, ALASAN_AKSES_TIDAK_DIBERIKAN),
            (UA_READ, O_CREAT, ALASAN_TOLAK_C),
            (UA_WRITE, O_RDONLY | O_DIRECTORY, ALASAN_TOLAK_RB),
            (UA_WRITE, O_RDONLY, ALASAN_TOLAK_R),
            (UA_READ, O_WRONLY, ALASAN_TOLAK_W),
            (UA_READ, O_EXEC, ALASAN_TOLAK_X),
        ];
        for (perm, opts, harap) in kasus {
            alasan_checks += 1;
            if alasan_akses_jalur(perm, opts) == harap {
                alasan_passed += 1;
            }
        }
    }

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // keluar-awal `O_UNLINK_INTERNAL` berdaya.
        let perm = UA_CREATE_OR_REMOVE;
        let opts = O_UNLINK_INTERNAL | O_RDONLY;
        m.catat(ksm_jalur_internal(perm, opts) == 0, Group::Neg);
        m.catat(ksm_jalur_kontrol_negatif(perm, opts) == -EACCES, Group::Neg);
        m.catat(ksm_jalur_internal(perm, opts) != ksm_jalur_kontrol_negatif(perm, opts), Group::Neg);
        // Pada kasus tanpa `O_UNLINK_INTERNAL` keduanya HARUS sama.
        m.catat(
            ksm_jalur_internal(perm, O_RDONLY) == ksm_jalur_kontrol_negatif(perm, O_RDONLY),
            Group::Neg,
        );
        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if ksm_jalur_internal(UA_READ, O_RDONLY) != 0 {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Neg);
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // 32 kombinasi izin × 64 kombinasi flag = 2048.
    for perm_mask in 0..32u32 {
        let mut perm = 0u32;
        if perm_mask & 1 != 0 {
            perm |= UA_READ;
        }
        if perm_mask & 2 != 0 {
            perm |= UA_WRITE;
        }
        if perm_mask & 4 != 0 {
            perm |= UA_EXECUTE;
        }
        if perm_mask & 8 != 0 {
            perm |= UA_CREATE_OR_REMOVE;
        }
        if perm_mask & 16 != 0 {
            perm |= UA_BROWSE;
        }
        for opsi_mask in 0..64u32 {
            let mut opts = 0u32;
            let mut b = 0;
            while b < 6 {
                if opsi_mask & (1 << b) != 0 {
                    opts |= OPSI_BIT[b];
                }
                b += 1;
            }
            if ksm_jalur_internal(perm, opts) != model_naif_ksm_jalur(perm, opts) {
                beda += 1;
            }
            diukur += 1;
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Peta);
    m.catat(beda == 0, Group::Peta);
    m.catat(alasan_checks == 7 && alasan_passed == 7, Group::Peta);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiAksesJalurReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_ksm_jalur_counter(Counter::ProdCalls as u32),
        prod_ok: klatos_ksm_jalur_counter(Counter::ProdOk as u32),
        prod_enoent: klatos_ksm_jalur_counter(Counter::ProdEnone as u32),
        prod_eacces: klatos_ksm_jalur_counter(Counter::ProdEacces as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        alasan_checks,
        alasan_passed,
        status,
    }
}

/// Menjalankan matriks otoritas gerbang izin akses_jalur.
#[no_mangle]
pub extern "C" fn klatos_ksm_jalur_selftest() -> KlatosAdmisiAksesJalurReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(ENOENT == 2);
        assert!(EACCES == 13);
        assert!(UA_READ == 1);
        assert!(UA_WRITE == 2);
        assert!(UA_EXECUTE == 4);
        assert!(UA_CREATE_OR_REMOVE == 8);
        assert!(UA_BROWSE == 16);
        assert!(O_RDONLY == 1);
        assert!(O_WRONLY == 2);
        assert!(O_EXEC == 4);
        assert!(O_CREAT == 8);
        assert!(O_DIRECTORY == 512);
        assert!(O_UNLINK_INTERNAL == 1_073_741_824);
        assert!(ALASAN_LOLOS == 0);
        assert!(ALASAN_TOLAK_X == 6);
        assert!(REPORT_TAG == 0x4B4C_4154_4741_5445);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdEacces as usize == 3);
        assert!(N_COUNTER == 4);
        assert!(DIFF_EXPECTED == 2048);

        assert!(ksm_jalur_internal(0, 0) == -ENOENT);
        assert!(ksm_jalur_internal(UA_READ, O_RDONLY) == 0);
        assert!(ksm_jalur_internal(UA_CREATE_OR_REMOVE, O_UNLINK_INTERNAL | O_RDONLY) == 0);
        assert!(ksm_jalur_internal(UA_READ, O_WRONLY) == -EACCES);
        assert!(alasan_akses_jalur(UA_WRITE, O_RDONLY | O_DIRECTORY) == ALASAN_TOLAK_RB);

        // 8 (tag) + 15*4 (u32) + 4 (status) = 72.
        assert!(core::mem::size_of::<KlatosAdmisiAksesJalurReport>() == 72);
        assert!(core::mem::align_of::<KlatosAdmisiAksesJalurReport>() == 8);
    };
}
