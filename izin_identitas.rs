// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas izin transisi identitas/kredensial (P1.31).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-1).
pub const EPERM: i32 = 1;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;

/// Operasi transisi identitas.
pub const OP_SETEUID: i32 = 1;
pub const OP_SETEGID: i32 = 2;
pub const OP_SETUID: i32 = 3;
pub const OP_SETGID: i32 = 4;
pub const OP_SETREUID: i32 = 5;
pub const OP_SETRESUID: i32 = 6;
pub const OP_SETREGID: i32 = 7;
pub const OP_SETRESGID: i32 = 8;

/// Tag laporan; cermin `KlatosIzinIdentitasReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_495A_4944; // "KLATIZID"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_izin_identitas_decide`].
    ProdCalls = 0,
    /// Jumlah keputusan `diizinkan`.
    ProdAllow = 1,
    /// Jumlah keputusan `ditolak` (`-EPERM`).
    ProdDeny = 2,
    /// Jumlah `op` tak dikenal (`-EINVAL`).
    ProdInval = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

#[inline]
const fn ok_uid(x: u32, uid: u32, euid: u32, suid: u32) -> bool {
    x == uid || x == euid || x == suid
}

#[inline]
const fn ok_gid(x: u32, gid: u32, egid: u32, sgid: u32) -> bool {
    x == gid || x == egid || x == sgid
}

/// Memutuskan izin transisi identitas. Mengembalikan `0` (izin), `-EPERM`
/// (tolak), atau `-EINVAL` (`op` tak dikenal).
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
#[allow(clippy::too_many_arguments)]
pub const fn putuskan(
    op: i32,
    is_superuser: u32,
    uid: u32,
    euid: u32,
    suid: u32,
    gid: u32,
    egid: u32,
    sgid: u32,
    n1: u32,
    n2: u32,
    n3: u32,
) -> i32 {
    let su = is_superuser != 0;
    match op {
        OP_SETEUID => {
            if su || n1 == uid || n1 == suid {
                0
            } else {
                -EPERM
            }
        }
        OP_SETEGID => {
            if su || n1 == gid || n1 == sgid {
                0
            } else {
                -EPERM
            }
        }
        OP_SETUID => {
            if su || n1 == uid || n1 == euid {
                0
            } else {
                -EPERM
            }
        }
        OP_SETGID => {
            if su || n1 == gid || n1 == egid {
                0
            } else {
                -EPERM
            }
        }
        OP_SETREUID => {
            if ok_uid(n1, uid, euid, suid) && ok_uid(n2, uid, euid, suid) {
                0
            } else {
                -EPERM
            }
        }
        OP_SETRESUID => {
            if (ok_uid(n1, uid, euid, suid) && ok_uid(n2, uid, euid, suid) && ok_uid(n3, uid, euid, suid))
                || su
            {
                0
            } else {
                -EPERM
            }
        }
        OP_SETREGID => {
            if ok_gid(n1, gid, egid, sgid) && ok_gid(n2, gid, egid, sgid) {
                0
            } else {
                -EPERM
            }
        }
        OP_SETRESGID => {
            if (ok_gid(n1, gid, egid, sgid) && ok_gid(n2, gid, egid, sgid) && ok_gid(n3, gid, egid, sgid))
                || su
            {
                0
            } else {
                -EPERM
            }
        }
        _ => -EINVAL,
    }
}

/// Orakel historis: transkripsi langsung kedelapan aturan `setuid.cpp`.
/// Dipakai HANYA oleh matriks diferensial. Struktur sengaja berbeda
/// (rantai `if` eksplisit) supaya kesepakatan dengan [`putuskan`] bermakna.
#[inline]
#[allow(clippy::too_many_arguments)]
pub const fn kebijakan_lama(
    op: i32,
    is_superuser: u32,
    uid: u32,
    euid: u32,
    suid: u32,
    gid: u32,
    egid: u32,
    sgid: u32,
    n1: u32,
    n2: u32,
    n3: u32,
) -> i32 {
    match op {
        OP_SETEUID => {
            if n1 != uid && n1 != suid && is_superuser == 0 {
                -EPERM
            } else {
                0
            }
        }
        OP_SETEGID => {
            if n1 != gid && n1 != sgid && is_superuser == 0 {
                -EPERM
            } else {
                0
            }
        }
        OP_SETUID => {
            if n1 != uid && n1 != euid && is_superuser == 0 {
                -EPERM
            } else {
                0
            }
        }
        OP_SETGID => {
            if n1 != gid && n1 != egid && is_superuser == 0 {
                -EPERM
            } else {
                0
            }
        }
        OP_SETREUID => {
            let o1 = n1 == uid || n1 == euid || n1 == suid;
            let o2 = n2 == uid || n2 == euid || n2 == suid;
            if !o1 || !o2 {
                -EPERM
            } else {
                0
            }
        }
        OP_SETRESUID => {
            let o1 = n1 == uid || n1 == euid || n1 == suid;
            let o2 = n2 == uid || n2 == euid || n2 == suid;
            let o3 = n3 == uid || n3 == euid || n3 == suid;
            if (!o1 || !o2 || !o3) && is_superuser == 0 {
                -EPERM
            } else {
                0
            }
        }
        OP_SETREGID => {
            let o1 = n1 == gid || n1 == egid || n1 == sgid;
            let o2 = n2 == gid || n2 == egid || n2 == sgid;
            if !o1 || !o2 {
                -EPERM
            } else {
                0
            }
        }
        OP_SETRESGID => {
            let o1 = n1 == gid || n1 == egid || n1 == sgid;
            let o2 = n2 == gid || n2 == egid || n2 == sgid;
            let o3 = n3 == gid || n3 == egid || n3 == sgid;
            if (!o1 || !o2 || !o3) && is_superuser == 0 {
                -EPERM
            } else {
                0
            }
        }
        _ => -EINVAL,
    }
}

/// Apakah hasil ini keputusan `diizinkan`?
#[inline]
pub const fn diizinkan(hasil: i32) -> bool {
    hasil == 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan izin transisi identitas. Lihat [`putuskan`] untuk kontrak.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn klatos_izin_identitas_decide(
    op: i32,
    is_superuser: u32,
    uid: u32,
    euid: u32,
    suid: u32,
    gid: u32,
    egid: u32,
    sgid: u32,
    n1: u32,
    n2: u32,
    n3: u32,
) -> i32 {
    bump(Counter::ProdCalls);
    let hasil = putuskan(op, is_superuser, uid, euid, suid, gid, egid, sgid, n1, n2, n3);
    if hasil == 0 {
        bump(Counter::ProdAllow);
    } else if hasil == -EPERM {
        bump(Counter::ProdDeny);
    } else {
        bump(Counter::ProdInval);
    }
    hasil
}

/// Nilai op dan errno yang dipakai modul ini.
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_eperm() -> i32 {
    EPERM
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_einval() -> i32 {
    EINVAL
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_op_seteuid() -> i32 {
    OP_SETEUID
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_op_setegid() -> i32 {
    OP_SETEGID
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_op_setuid() -> i32 {
    OP_SETUID
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_op_setgid() -> i32 {
    OP_SETGID
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_op_setreuid() -> i32 {
    OP_SETREUID
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_op_setresuid() -> i32 {
    OP_SETRESUID
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_op_setregid() -> i32 {
    OP_SETREGID
}
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_op_setresgid() -> i32 {
    OP_SETRESGID
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosIzinIdentitasReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosIzinIdentitasReport {
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
    pub prod_inval: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    /// Padding eksplisit supaya `status` 4-selaras dan ukuran kelipatan 8.
    pub _pad: u32,
    pub status: i32,
}

/// Ukuran domain sapuan diferensial.
///
/// `op` (8) x `is_superuser` (2) x uid/euid/suid (2^3) x gid/egid/sgid (2^3) x
/// n1/n2/n3 (3^3) = 27_648.
pub const DOMAIN_EXPECTED: u32 = 8 * 2 * 8 * 8 * 27;

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

const IDS2: [u32; 2] = [0, 1000];
const N3: [u32; 3] = [0, 1000, u32::MAX];

/// Matriks otoritas izin transisi identitas. Input SINTETIS.
fn run_matrix() -> KlatosIzinIdentitasReport {
    let mut m = Matrix::new();
    let su = 0u32;
    // uid=1000 euid=1000 suid=1000, gid=100 egid=100 sgid=100 (identitas tunggal)
    let (uid, euid, suid, gid, egid, sgid) = (1000u32, 1000u32, 1000u32, 100u32, 100u32, 100u32);

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // seteuid: ke uid sendiri -> izin; ke id lain -> tolak.
    m.catat(putuskan(OP_SETEUID, su, uid, euid, suid, gid, egid, sgid, 1000, 0, 0) == 0, Group::Peta);
    m.catat(putuskan(OP_SETEUID, su, uid, euid, suid, gid, egid, sgid, 2000, 0, 0) == -EPERM, Group::Peta);
    m.catat(putuskan(OP_SETEUID, 1, uid, euid, suid, gid, egid, sgid, 2000, 0, 0) == 0, Group::Peta);
    // setuid: ke euid -> izin; ke id lain -> tolak.
    m.catat(putuskan(OP_SETUID, su, uid, euid, suid, gid, egid, sgid, 1000, 0, 0) == 0, Group::Peta);
    m.catat(putuskan(OP_SETUID, su, uid, euid, suid, gid, egid, sgid, 2000, 0, 0) == -EPERM, Group::Peta);
    // setgid / setegid.
    m.catat(putuskan(OP_SETGID, su, uid, euid, suid, gid, egid, sgid, 100, 0, 0) == 0, Group::Peta);
    m.catat(putuskan(OP_SETGID, su, uid, euid, suid, gid, egid, sgid, 200, 0, 0) == -EPERM, Group::Peta);
    m.catat(putuskan(OP_SETEGID, su, uid, euid, suid, gid, egid, sgid, 100, 0, 0) == 0, Group::Peta);
    m.catat(putuskan(OP_SETEGID, su, uid, euid, suid, gid, egid, sgid, 200, 0, 0) == -EPERM, Group::Peta);
    // setreuid: TANPA bypass root.
    m.catat(putuskan(OP_SETREUID, 1, uid, euid, suid, gid, egid, sgid, 1000, 1000, 0) == 0, Group::Peta);
    m.catat(putuskan(OP_SETREUID, 1, uid, euid, suid, gid, egid, sgid, 2000, 1000, 0) == -EPERM, Group::Peta);
    // setresuid: DENGAN bypass root.
    m.catat(putuskan(OP_SETRESUID, su, uid, euid, suid, gid, egid, sgid, 2000, 1000, 1000) == -EPERM, Group::Peta);
    m.catat(putuskan(OP_SETRESUID, 1, uid, euid, suid, gid, egid, sgid, 2000, 1000, 1000) == 0, Group::Peta);
    // setregid / setresgid.
    m.catat(putuskan(OP_SETREGID, 1, uid, euid, suid, gid, egid, sgid, 200, 100, 0) == -EPERM, Group::Peta);
    m.catat(putuskan(OP_SETRESGID, 1, uid, euid, suid, gid, egid, sgid, 200, 100, 100) == 0, Group::Peta);
    m.catat(putuskan(OP_SETRESGID, su, uid, euid, suid, gid, egid, sgid, 200, 100, 100) == -EPERM, Group::Peta);
    // op tak dikenal -> -EINVAL.
    m.catat(putuskan(0, su, uid, euid, suid, gid, egid, sgid, 0, 0, 0) == -EINVAL, Group::Peta);
    m.catat(putuskan(99, su, uid, euid, suid, gid, egid, sgid, 0, 0, 0) == -EINVAL, Group::Peta);

    // --- kelompok 1: sapuan diferensial ------------------------------------
    let ops = [OP_SETEUID, OP_SETEGID, OP_SETUID, OP_SETGID, OP_SETREUID, OP_SETRESUID, OP_SETREGID, OP_SETRESGID];
    let mut diukur = 0u32;
    let mut beda = 0u32;
    for &op in ops.iter() {
        for s in [0u32, 1] {
            for &u in IDS2.iter() {
                for &e in IDS2.iter() {
                    for &t in IDS2.iter() {
                        for &g in IDS2.iter() {
                            for &ge in IDS2.iter() {
                                for &gt in IDS2.iter() {
                                    for &n1 in N3.iter() {
                                        for &n2 in N3.iter() {
                                            for &n3 in N3.iter() {
                                                let r = putuskan(op, s, u, e, t, g, ge, gt, n1, n2, n3);
                                                let b = kebijakan_lama(op, s, u, e, t, g, ge, gt, n1, n2, n3);
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
                }
            }
        }
    }
    m.catat(diukur == DOMAIN_EXPECTED, Group::Peta);
    m.catat(beda == 0, Group::Peta);

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_495A_4944, Group::Neg);
    m.catat(EPERM == 1 && EINVAL == 22, Group::Neg);
    m.catat(OP_SETEUID == 1 && OP_SETRESGID == 8, Group::Neg);
    // Determinisme.
    {
        let mut ok = true;
        let first = putuskan(OP_SETREUID, 0, 1000, 1000, 1000, 100, 100, 100, 1000, 1000, 0);
        for _ in 0..1000 {
            if putuskan(OP_SETREUID, 0, 1000, 1000, 1000, 100, 100, 100, 1000, 1000, 0) != first {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: perbedaan setreuid (tanpa root) vs setresuid (dengan root).
    {
        let mut ok = true;
        if putuskan(OP_SETREUID, 1, 1000, 1000, 1000, 100, 100, 100, 2000, 1000, 0) != -EPERM {
            ok = false;
        }
        if putuskan(OP_SETRESUID, 1, 1000, 1000, 1000, 100, 100, 100, 2000, 1000, 1000) != 0 {
            ok = false;
        }
        m.catat(ok, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosIzinIdentitasReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_izin_identitas_counter(Counter::ProdCalls as u32),
        prod_allow: klatos_izin_identitas_counter(Counter::ProdAllow as u32),
        prod_deny: klatos_izin_identitas_counter(Counter::ProdDeny as u32),
        prod_inval: klatos_izin_identitas_counter(Counter::ProdInval as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        _pad: 0,
        status,
    }
}

/// Menjalankan matriks otoritas izin transisi identitas.
#[no_mangle]
pub extern "C" fn klatos_izin_identitas_selftest() -> KlatosIzinIdentitasReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_495A_4944);
        assert!(EPERM == 1 && EINVAL == 22);
        assert!(OP_SETEUID == 1 && OP_SETEGID == 2 && OP_SETUID == 3 && OP_SETGID == 4);
        assert!(OP_SETREUID == 5 && OP_SETRESUID == 6 && OP_SETREGID == 7 && OP_SETRESGID == 8);
        assert!(DOMAIN_EXPECTED == 27_648);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdInval as usize == 3);

        let (u, e, t, g, ge, gt) = (1000u32, 1000u32, 1000u32, 100u32, 100u32, 100u32);
        assert!(putuskan(OP_SETEUID, 0, u, e, t, g, ge, gt, 1000, 0, 0) == 0);
        assert!(putuskan(OP_SETEUID, 0, u, e, t, g, ge, gt, 2000, 0, 0) == -EPERM);
        assert!(putuskan(OP_SETREUID, 1, u, e, t, g, ge, gt, 2000, 1000, 0) == -EPERM);
        assert!(putuskan(OP_SETRESUID, 1, u, e, t, g, ge, gt, 2000, 1000, 1000) == 0);
        assert!(putuskan(99, 0, u, e, t, g, ge, gt, 0, 0, 0) == -EINVAL);

        assert!(core::mem::size_of::<KlatosIzinIdentitasReport>() == 64);
        assert!(core::mem::align_of::<KlatosIzinIdentitasReport>() == 8);
    };
}
