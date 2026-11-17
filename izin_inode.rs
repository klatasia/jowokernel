// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas izin akses berkas DAC Unix (P1.29).


use core::sync::atomic::{AtomicU32, Ordering};

/// Operasi: baca.
pub const OP_BACA: i32 = 1;
/// Operasi: tulis.
pub const OP_TULIS: i32 = 2;
/// Operasi: eksekusi.
pub const OP_EKSEKUSI: i32 = 3;

/// `S_IRUSR` (0400) — cermin `Jowo/Kontrak/POSIX/sys/stat.h`.
pub const S_IRUSR: u32 = 0o400;
/// `S_IWUSR` (0200).
pub const S_IWUSR: u32 = 0o200;
/// `S_IXUSR` (0100).
pub const S_IXUSR: u32 = 0o100;
/// `S_IRGRP` (0040).
pub const S_IRGRP: u32 = 0o040;
/// `S_IWGRP` (0020).
pub const S_IWGRP: u32 = 0o020;
/// `S_IXGRP` (0010).
pub const S_IXGRP: u32 = 0o010;
/// `S_IROTH` (0004).
pub const S_IROTH: u32 = 0o004;
/// `S_IWOTH` (0002).
pub const S_IWOTH: u32 = 0o002;
/// `S_IXOTH` (0001).
pub const S_IXOTH: u32 = 0o001;

/// Topeng izin Unix klasik (9 bit).
pub const MASK_IZIN: u32 = 0o777;

/// Tag laporan; cermin `KlatosIzinInodeReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_495A_494E; // "KLATIZIN"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_izin_inode_decide`].
    ProdCalls = 0,
    /// Jumlah keputusan `diizinkan`.
    ProdAllow = 1,
    /// Jumlah keputusan `ditolak`.
    ProdDeny = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Bit izin yang berlaku untuk `op` pada kelas `kelas` (`0`=pemilik,
/// `1`=grup, `2`=lainnya).
#[inline]
pub const fn bit_izin(op: i32, kelas: u32) -> u32 {
    // Tabel 3x3: baris = op, kolom = kelas.
    match (op, kelas) {
        (OP_BACA, 0) => S_IRUSR,
        (OP_TULIS, 0) => S_IWUSR,
        (OP_EKSEKUSI, 0) => S_IXUSR,
        (OP_BACA, 1) => S_IRGRP,
        (OP_TULIS, 1) => S_IWGRP,
        (OP_EKSEKUSI, 1) => S_IXGRP,
        (OP_BACA, 2) => S_IROTH,
        (OP_TULIS, 2) => S_IWOTH,
        (OP_EKSEKUSI, 2) => S_IXOTH,
        _ => 0,
    }
}

/// Memutuskan izin akses berkas DAC Unix.
///
/// SAMA PERSIS dengan `InodeMetadata::may_*` historis. Mengembalikan `1`
/// (diizinkan) atau `0` (ditolak).
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(
    op: i32,
    caller_uid: u32,
    caller_gid: u32,
    egid_match: u32,
    file_uid: u32,
    file_gid: u32,
    mode: u32,
) -> i32 {
    // Pintasan root.
    if caller_uid == 0 {
        return 1;
    }
    let kelas = if file_uid == caller_uid {
        0u32
    } else if file_gid == caller_gid || egid_match != 0 {
        1u32
    } else {
        2u32
    };
    let bit = bit_izin(op, kelas);
    // Operasi tidak dikenal -> bit 0 -> tolak (kecuali root, sudah ditangani).
    if bit == 0 {
        return 0;
    }
    if (mode & bit) == bit {
        1
    } else {
        0
    }
}

/// Orakel historis: transkripsi langsung formula C++ `may_read`/`may_write`/
/// `may_execute` (`InodeMetadata.h:54-85`). Dipakai HANYA oleh matriks
/// diferensial, bukan oleh produksi.
///
/// Struktur sengaja berbeda dari [`putuskan`] (rantai `if` eksplisit per kelas,
/// tanpa tabel) supaya kesepakatan keduanya bermakna.
#[inline]
pub const fn kebijakan_lama(
    op: i32,
    caller_uid: u32,
    caller_gid: u32,
    egid_match: u32,
    file_uid: u32,
    file_gid: u32,
    mode: u32,
) -> i32 {
    if caller_uid == 0 {
        return 1;
    }
    if file_uid == caller_uid {
        let (baca, tulis, eksekusi) = (S_IRUSR, S_IWUSR, S_IXUSR);
        return match op {
            OP_BACA => ((mode & baca) == baca) as i32,
            OP_TULIS => ((mode & tulis) == tulis) as i32,
            OP_EKSEKUSI => ((mode & eksekusi) == eksekusi) as i32,
            _ => 0,
        };
    }
    if file_gid == caller_gid || egid_match != 0 {
        let (baca, tulis, eksekusi) = (S_IRGRP, S_IWGRP, S_IXGRP);
        return match op {
            OP_BACA => ((mode & baca) == baca) as i32,
            OP_TULIS => ((mode & tulis) == tulis) as i32,
            OP_EKSEKUSI => ((mode & eksekusi) == eksekusi) as i32,
            _ => 0,
        };
    }
    let (baca, tulis, eksekusi) = (S_IROTH, S_IWOTH, S_IXOTH);
    match op {
        OP_BACA => ((mode & baca) == baca) as i32,
        OP_TULIS => ((mode & tulis) == tulis) as i32,
        OP_EKSEKUSI => ((mode & eksekusi) == eksekusi) as i32,
        _ => 0,
    }
}

/// Apakah hasil ini keputusan `diizinkan`?
#[inline]
pub const fn diizinkan(hasil: i32) -> bool {
    hasil == 1
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan izin akses berkas DAC Unix. Lihat [`putuskan`] untuk kontrak.
#[no_mangle]
pub extern "C" fn klatos_izin_inode_decide(
    op: i32,
    caller_uid: u32,
    caller_gid: u32,
    egid_match: u32,
    file_uid: u32,
    file_gid: u32,
    mode: u32,
) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(op, caller_uid, caller_gid, egid_match, file_uid, file_gid, mode);
    if hasil == 1 {
        bump(Counter::ProdAllow);
    } else {
        bump(Counter::ProdDeny);
    }
    hasil
}

/// Nilai op dan topeng yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_izin_inode_op_baca() -> i32 {
    OP_BACA
}
#[no_mangle]
pub extern "C" fn klatos_izin_inode_op_tulis() -> i32 {
    OP_TULIS
}
#[no_mangle]
pub extern "C" fn klatos_izin_inode_op_eksekusi() -> i32 {
    OP_EKSEKUSI
}
#[no_mangle]
pub extern "C" fn klatos_izin_inode_mask_izin() -> u32 {
    MASK_IZIN
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_izin_inode_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_izin_inode_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosIzinInodeReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosIzinInodeReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_allow: u32,
    pub prod_deny: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Ukuran domain sapuan diferensial.
///
/// `caller_uid` (3) x `caller_gid` (2) x `egid_match` (2) x `file_uid` (2) x
/// `file_gid` (2) x `mode` (512) x `op` (3) = 73_728.
pub const DOMAIN_EXPECTED: u32 = 3 * 2 * 2 * 2 * 2 * 512 * 3;

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

const CALLER_UIDS: [u32; 3] = [0, 1000, 1001];
const CALLER_GIDS: [u32; 2] = [0, 100];
const FILE_UIDS: [u32; 2] = [0, 1000];
const FILE_GIDS: [u32; 2] = [0, 100];

/// Matriks otoritas izin akses berkas DAC Unix. Input SINTETIS.
fn run_matrix() -> KlatosIzinInodeReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Root selalu diizinkan, apa pun mode/op.
    m.catat(putuskan(OP_BACA, 0, 0, 0, 1000, 100, 0) == 1, Group::Peta);
    m.catat(putuskan(OP_TULIS, 0, 0, 0, 1000, 100, 0) == 1, Group::Peta);
    m.catat(putuskan(OP_EKSEKUSI, 0, 0, 0, 1000, 100, 0) == 1, Group::Peta);
    m.catat(putuskan(OP_BACA, 0, 0, 0, 0, 0, 0) == 1, Group::Peta);
    // Pemilik: mode 0400 -> baca saja.
    m.catat(putuskan(OP_BACA, 1000, 100, 0, 1000, 100, S_IRUSR) == 1, Group::Peta);
    m.catat(putuskan(OP_TULIS, 1000, 100, 0, 1000, 100, S_IRUSR) == 0, Group::Peta);
    m.catat(putuskan(OP_EKSEKUSI, 1000, 100, 0, 1000, 100, S_IRUSR) == 0, Group::Peta);
    // Pemilik: mode 0700 -> semua.
    m.catat(putuskan(OP_BACA, 1000, 100, 0, 1000, 100, 0o700) == 1, Group::Peta);
    m.catat(putuskan(OP_TULIS, 1000, 100, 0, 1000, 100, 0o700) == 1, Group::Peta);
    m.catat(putuskan(OP_EKSEKUSI, 1000, 100, 0, 1000, 100, 0o700) == 1, Group::Peta);
    // Grup (gid sama): mode 0040 -> baca grup.
    m.catat(putuskan(OP_BACA, 1001, 100, 0, 1000, 100, S_IRGRP) == 1, Group::Peta);
    m.catat(putuskan(OP_TULIS, 1001, 100, 0, 1000, 100, S_IRGRP) == 0, Group::Peta);
    // Grup (egid_match, gid beda): mode 0020 -> tulis grup.
    m.catat(putuskan(OP_TULIS, 1001, 200, 1, 1000, 100, S_IWGRP) == 1, Group::Peta);
    m.catat(putuskan(OP_BACA, 1001, 200, 1, 1000, 100, S_IWGRP) == 0, Group::Peta);
    // Lainnya: mode 0004 -> baca lainnya.
    m.catat(putuskan(OP_BACA, 1001, 200, 0, 1000, 100, S_IROTH) == 1, Group::Peta);
    m.catat(putuskan(OP_EKSEKUSI, 1001, 200, 0, 1000, 100, S_IROTH) == 0, Group::Peta);
    // mode 0 -> selalu ditolak (non-root).
    m.catat(putuskan(OP_BACA, 1000, 100, 0, 1000, 100, 0) == 0, Group::Peta);
    m.catat(putuskan(OP_BACA, 1001, 100, 0, 1000, 100, 0) == 0, Group::Peta);
    m.catat(putuskan(OP_BACA, 1001, 200, 0, 1000, 100, 0) == 0, Group::Peta);
    // Prioritas kelas: pemilik menang atas grup.
    m.catat(putuskan(OP_BACA, 1000, 100, 0, 1000, 100, S_IRGRP) == 0, Group::Peta);
    // Prioritas kelas: grup menang atas lainnya.
    m.catat(putuskan(OP_BACA, 1001, 100, 0, 1000, 100, S_IROTH) == 0, Group::Peta);
    // op tidak dikenal -> bit 0 -> selalu ditolak (kecuali root).
    m.catat(putuskan(0, 1000, 100, 0, 1000, 100, 0o777) == 0, Group::Peta);
    m.catat(putuskan(99, 1000, 100, 0, 1000, 100, 0o777) == 0, Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    let mut diukur = 0u32;
    let mut beda = 0u32;
    for op in [OP_BACA, OP_TULIS, OP_EKSEKUSI] {
        for &cu in CALLER_UIDS.iter() {
            for &cg in CALLER_GIDS.iter() {
                for eg in [0u32, 1] {
                    for &fu in FILE_UIDS.iter() {
                        for &fg in FILE_GIDS.iter() {
                            for mode in 0u32..512 {
                                let r = putuskan(op, cu, cg, eg, fu, fg, mode);
                                let b = kebijakan_lama(op, cu, cg, eg, fu, fg, mode);
                                diukur += 1;
                                if r != b {
                                    beda += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    m.catat(diukur == DOMAIN_EXPECTED, Group::Peta);
    m.catat(beda == 0, Group::Peta);

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_495A_494E, Group::Neg);
    m.catat(OP_BACA == 1 && OP_TULIS == 2 && OP_EKSEKUSI == 3, Group::Neg);
    m.catat(MASK_IZIN == 0o777, Group::Neg);
    // Bit-bit izin berdiri sendiri dan lengkap.
    m.catat(
        (S_IRUSR | S_IWUSR | S_IXUSR | S_IRGRP | S_IWGRP | S_IXGRP | S_IROTH | S_IWOTH | S_IXOTH)
            == MASK_IZIN,
        Group::Neg,
    );
    // Tabel bit berbeda antar kelas.
    m.catat(bit_izin(OP_BACA, 0) != bit_izin(OP_BACA, 1), Group::Neg);
    m.catat(bit_izin(OP_BACA, 1) != bit_izin(OP_BACA, 2), Group::Neg);
    // Determinisme.
    {
        let mut ok = true;
        let first = putuskan(OP_BACA, 1001, 200, 1, 1000, 100, 0o644);
        for _ in 0..1000 {
            if putuskan(OP_BACA, 1001, 200, 1, 1000, 100, 0o644) != first {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: harapan SALAH harus terdeteksi.
    {
        let mut ok = true;
        // 0o600 pada pemilik -> baca diizinkan.
        if putuskan(OP_BACA, 1000, 100, 0, 1000, 100, 0o600) != 1 {
            ok = false;
        }
        // 0o600 pada "lainnya" -> baca ditolak.
        if putuskan(OP_BACA, 1001, 200, 0, 1000, 100, 0o600) != 0 {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosIzinInodeReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_izin_inode_counter(Counter::ProdCalls as u32),
        prod_allow: klatos_izin_inode_counter(Counter::ProdAllow as u32),
        prod_deny: klatos_izin_inode_counter(Counter::ProdDeny as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas izin akses berkas DAC Unix.
#[no_mangle]
pub extern "C" fn klatos_izin_inode_selftest() -> KlatosIzinInodeReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_495A_494E);
        assert!(OP_BACA == 1 && OP_TULIS == 2 && OP_EKSEKUSI == 3);
        assert!(S_IRUSR == 256 && S_IWUSR == 128 && S_IXUSR == 64);
        assert!(S_IRGRP == 32 && S_IWGRP == 16 && S_IXGRP == 8);
        assert!(S_IROTH == 4 && S_IWOTH == 2 && S_IXOTH == 1);
        assert!(MASK_IZIN == 511);
        assert!(DOMAIN_EXPECTED == 73_728);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdDeny as usize == 2);

        // Root selalu diizinkan.
        assert!(putuskan(OP_BACA, 0, 0, 0, 1000, 100, 0) == 1);
        // Pemilik 0700.
        assert!(putuskan(OP_TULIS, 1000, 100, 0, 1000, 100, 0o700) == 1);
        // Grup.
        assert!(putuskan(OP_BACA, 1001, 100, 0, 1000, 100, S_IRGRP) == 1);
        // Lainnya.
        assert!(putuskan(OP_BACA, 1001, 200, 0, 1000, 100, S_IROTH) == 1);
        // Ditolak.
        assert!(putuskan(OP_TULIS, 1001, 200, 0, 1000, 100, 0o755) == 0);
        // Operasi tak dikenal -> tolak.
        assert!(putuskan(0, 1000, 100, 0, 1000, 100, 0o777) == 0);
        assert!(putuskan(99, 1000, 100, 0, 1000, 100, 0o777) == 0);
        // Diferensial setuju pada satu titik.
        assert!(putuskan(OP_EKSEKUSI, 1001, 100, 0, 1000, 100, 0o755)
            == kebijakan_lama(OP_EKSEKUSI, 1001, 100, 0, 1000, 100, 0o755));

        // 8 (tag) + 11*4 (u32) + 4 (status) = 56.
        assert!(core::mem::size_of::<KlatosIzinInodeReport>() == 56);
        assert!(core::mem::align_of::<KlatosIzinInodeReport>() == 8);
    };
}
