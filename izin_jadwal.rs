// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas izin parameter penjadwal (P1.32).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-1).
pub const EPERM: i32 = 1;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;

/// `THREAD_PRIORITY_MIN` — cermin `Jowo/Kontrak/POSIX/sched.h:22`.
pub const THREAD_PRIORITY_MIN: i32 = 1;
/// `THREAD_PRIORITY_MAX` — cermin `Jowo/Kontrak/POSIX/sched.h:26`.
pub const THREAD_PRIORITY_MAX: i32 = 99;

/// Tahap 1: hanya cek rentang prioritas (dipanggil sebelum kunci penjadwal).
pub const OP_RENTANG: i32 = 1;
/// Tahap 2: hanya cek izin (dipanggil setelah peer diketahui; set DAN get).
pub const OP_IZIN: i32 = 2;

/// Tag laporan; cermin `KlatosIzinJadwalReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_495A_4A44; // "KLATIZJD"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_izin_jadwal_decide`].
    ProdCalls = 0,
    /// Jumlah keputusan `diizinkan`.
    ProdAllow = 1,
    /// Jumlah ditolak karena rentang (`-EINVAL`).
    ProdInval = 2,
    /// Jumlah ditolak karena izin (`-EPERM`).
    ProdPerm = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan satu tahap penerimaan parameter penjadwal. Mengembalikan `0`
/// (izin), `-EINVAL` (rentang, hanya `OP_RENTANG`), atau `-EPERM`
/// (bukan pemilik/root, hanya `OP_IZIN`).
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(
    op: i32,
    is_superuser: u32,
    euid: u32,
    uid: u32,
    peer_euid: u32,
    peer_uid: u32,
    priority: i32,
) -> i32 {
    if op == OP_RENTANG {
        if priority < THREAD_PRIORITY_MIN || priority > THREAD_PRIORITY_MAX {
            return -EINVAL;
        }
        return 0;
    }
    // OP_IZIN
    if is_superuser != 0 || euid == peer_euid || uid == peer_uid {
        return 0;
    }
    -EPERM
}

/// Orakel historis: transkripsi langsung `sched.cpp:74-79,105-107`. Dipakai
/// HANYA oleh matriks diferensial. Struktur sengaja berbeda supaya kesepakatan
/// dengan [`putuskan`] bermakna.
#[inline]
pub const fn kebijakan_lama(
    op: i32,
    is_superuser: u32,
    euid: u32,
    uid: u32,
    peer_euid: u32,
    peer_uid: u32,
    priority: i32,
) -> i32 {
    if op == OP_RENTANG {
        if priority < THREAD_PRIORITY_MIN {
            return -EINVAL;
        }
        if priority > THREAD_PRIORITY_MAX {
            return -EINVAL;
        }
        return 0;
    }
    if is_superuser != 0 {
        return 0;
    }
    if euid == peer_euid {
        return 0;
    }
    if uid == peer_uid {
        return 0;
    }
    -EPERM
}

/// Apakah hasil ini keputusan `diizinkan`?
#[inline]
pub const fn diizinkan(hasil: i32) -> bool {
    hasil == 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan penerimaan parameter penjadwal. Lihat [`putuskan`] untuk kontrak.
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_decide(
    op: i32,
    is_superuser: u32,
    euid: u32,
    uid: u32,
    peer_euid: u32,
    peer_uid: u32,
    priority: i32,
) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(op, is_superuser, euid, uid, peer_euid, peer_uid, priority);
    if hasil == 0 {
        bump(Counter::ProdAllow);
    } else if hasil == -EINVAL {
        bump(Counter::ProdInval);
    } else {
        bump(Counter::ProdPerm);
    }
    hasil
}

/// Nilai op, errno, dan batas prioritas yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_eperm() -> i32 {
    EPERM
}
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_einval() -> i32 {
    EINVAL
}
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_op_rentang() -> i32 {
    OP_RENTANG
}
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_op_izin() -> i32 {
    OP_IZIN
}
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_priority_min() -> i32 {
    THREAD_PRIORITY_MIN
}
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_priority_max() -> i32 {
    THREAD_PRIORITY_MAX
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosIzinJadwalReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosIzinJadwalReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_allow: u32,
    pub prod_inval: u32,
    pub prod_perm: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    /// Padding eksplisit supaya `status` 4-selaras dan ukuran kelipatan 8.
    pub _pad: u32,
    pub status: i32,
}

/// Ukuran domain sapuan diferensial.
///
/// `op` (2) x `is_superuser` (2) x euid/uid/peer_euid/peer_uid (3^4) x
/// `priority` (6) = 1_944.
pub const DOMAIN_EXPECTED: u32 = 2 * 2 * 81 * 6;

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

const UIDS: [u32; 3] = [0, 1000, 1001];
const PRIOS: [i32; 6] = [-1, 0, 1, 50, 99, 100];

/// Matriks otoritas izin parameter penjadwal. Input SINTETIS.
fn run_matrix() -> KlatosIzinJadwalReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // OP_RENTANG: prioritas tepat di tepi -> izin.
    m.catat(putuskan(OP_RENTANG, 0, 1000, 1000, 1000, 1000, 1) == 0, Group::Peta);
    m.catat(putuskan(OP_RENTANG, 0, 1000, 1000, 1000, 1000, 99) == 0, Group::Peta);
    // OP_RENTANG: di luar rentang -> EINVAL (identitas tidak relevan).
    m.catat(putuskan(OP_RENTANG, 1, 1000, 1000, 2000, 2000, 0) == -EINVAL, Group::Peta);
    m.catat(putuskan(OP_RENTANG, 1, 1000, 1000, 2000, 2000, 100) == -EINVAL, Group::Peta);
    m.catat(putuskan(OP_RENTANG, 1, 1000, 1000, 2000, 2000, -1) == -EINVAL, Group::Peta);
    // OP_IZIN: rentang TIDAK diperiksa -> prioritas apa pun tidak memicu EINVAL.
    m.catat(putuskan(OP_IZIN, 0, 1000, 1000, 1000, 1000, 0) == 0, Group::Peta);
    m.catat(putuskan(OP_IZIN, 0, 1000, 1000, 1000, 1000, 100) == 0, Group::Peta);
    // OP_IZIN: bukan pemilik -> EPERM.
    m.catat(putuskan(OP_IZIN, 0, 1000, 1000, 2000, 2000, 50) == -EPERM, Group::Peta);
    // OP_IZIN: root -> izin.
    m.catat(putuskan(OP_IZIN, 1, 1000, 1000, 2000, 2000, 50) == 0, Group::Peta);
    // OP_IZIN: euid cocok.
    m.catat(putuskan(OP_IZIN, 0, 2000, 1000, 2000, 3000, 50) == 0, Group::Peta);
    // OP_IZIN: uid cocok.
    m.catat(putuskan(OP_IZIN, 0, 2000, 1000, 3000, 1000, 50) == 0, Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    let mut diukur = 0u32;
    let mut beda = 0u32;
    for &op in [OP_RENTANG, OP_IZIN].iter() {
        for s in [0u32, 1] {
            for &e in UIDS.iter() {
                for &u in UIDS.iter() {
                    for &pe in UIDS.iter() {
                        for &pu in UIDS.iter() {
                            for &pr in PRIOS.iter() {
                                let r = putuskan(op, s, e, u, pe, pu, pr);
                                let b = kebijakan_lama(op, s, e, u, pe, pu, pr);
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
    m.catat(REPORT_TAG == 0x4B4C_4154_495A_4A44, Group::Neg);
    m.catat(EPERM == 1 && EINVAL == 22, Group::Neg);
    m.catat(THREAD_PRIORITY_MIN == 1 && THREAD_PRIORITY_MAX == 99, Group::Neg);
    m.catat(OP_RENTANG == 1 && OP_IZIN == 2, Group::Neg);
    // Determinisme.
    {
        let mut ok = true;
        let first = putuskan(OP_RENTANG, 0, 1000, 1000, 1000, 1000, 50);
        for _ in 0..1000 {
            if putuskan(OP_RENTANG, 0, 1000, 1000, 1000, 1000, 50) != first {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: OP_RENTANG memeriksa rentang, OP_IZIN tidak.
    {
        let mut ok = true;
        if putuskan(OP_RENTANG, 1, 0, 0, 0, 0, 100) != -EINVAL {
            ok = false;
        }
        if putuskan(OP_IZIN, 1, 0, 0, 0, 0, 100) != 0 {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosIzinJadwalReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_izin_jadwal_counter(Counter::ProdCalls as u32),
        prod_allow: klatos_izin_jadwal_counter(Counter::ProdAllow as u32),
        prod_inval: klatos_izin_jadwal_counter(Counter::ProdInval as u32),
        prod_perm: klatos_izin_jadwal_counter(Counter::ProdPerm as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        _pad: 0,
        status,
    }
}

/// Menjalankan matriks otoritas izin parameter penjadwal.
#[no_mangle]
pub extern "C" fn klatos_izin_jadwal_selftest() -> KlatosIzinJadwalReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_495A_4A44);
        assert!(EPERM == 1 && EINVAL == 22);
        assert!(THREAD_PRIORITY_MIN == 1 && THREAD_PRIORITY_MAX == 99);
        assert!(OP_RENTANG == 1 && OP_IZIN == 2);
        assert!(DOMAIN_EXPECTED == 1_944);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdPerm as usize == 3);

        assert!(putuskan(OP_RENTANG, 0, 1000, 1000, 1000, 1000, 1) == 0);
        assert!(putuskan(OP_RENTANG, 1, 1000, 1000, 2000, 2000, 0) == -EINVAL);
        assert!(putuskan(OP_IZIN, 0, 1000, 1000, 2000, 2000, 50) == -EPERM);
        assert!(putuskan(OP_IZIN, 1, 1000, 1000, 2000, 2000, 100) == 0);

        assert!(core::mem::size_of::<KlatosIzinJadwalReport>() == 64);
        assert!(core::mem::align_of::<KlatosIzinJadwalReport>() == 8);
    };
}
