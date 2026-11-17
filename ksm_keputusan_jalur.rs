// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas ADMISI PERMINTAAN `unveil()` Jowo (P1.14).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `<errno.h>`.
pub const EPERM: i32 = 1;
/// `EINVAL` — cermin `<errno.h>`.
pub const EINVAL: i32 = 22;

/// Verdict: kunci veil, lalu `sys$unveil` mengembalikan 0.
pub const ADM_LOCK: i32 = 0;
/// Verdict: abaikan permintaan diam-diam, kembalikan 0.
pub const ADM_IGNORE: i32 = 1;
/// Verdict: lanjutkan ke penguraian izin + resolusi jalur.
pub const ADM_PROSES: i32 = 2;
/// Verdict: keputusan butuh informasi jalur; C++ harus menyalin jalur dan
/// bertanya lagi dengan `path_diketahui = true`.
pub const ADM_BUTUH_PATH: i32 = 3;
/// Verdict: tolak dengan `EPERM`.
pub const ADM_EPERM: i32 = -EPERM;
/// Verdict: tolak dengan `EINVAL`.
pub const ADM_EINVAL: i32 = -EINVAL;

/// Cermin `BenderaAksesJalur::CurrentProgram`.
pub const FLAG_CURRENT_PROGRAM: u32 = 1;
/// Cermin `BenderaAksesJalur::AfterExec`.
pub const FLAG_AFTER_EXEC: u32 = 2;

/// Cermin `VeilState::None`.
pub const VEIL_NONE: u32 = 0;
/// Cermin `VeilState::Dropped`.
pub const VEIL_DROPPED: u32 = 1;
/// Cermin `VeilState::Locked`.
pub const VEIL_LOCKED: u32 = 2;
/// Cermin `VeilState::LockedInherited`.
pub const VEIL_LOCKED_INHERITED: u32 = 3;

/// Tag laporan; cermin `KlatosKeputusanAksesJalurReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_554E_4144; // "KLATUNAD"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_ksm_keputusan_jalur_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah verdict `Lock`.
    ProdLock = 1,
    /// Jumlah verdict `Ignore`.
    ProdIgnore = 2,
    /// Jumlah verdict `EPERM`.
    ProdEperm = 3,
    /// Jumlah verdict `EINVAL`.
    ProdEinval = 4,
    /// Jumlah verdict `Proses`.
    ProdProses = 5,
    /// Jumlah verdict `ButuhPath` (fase 1 yang meminta fase 2).
    ProdButuhPath = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan admisi permintaan `unveil`.
///
/// Urutan dan isi keputusan SAMA PERSIS dengan blok C++ historis
/// (`Jowo/Keamanan/akses_jalur.cpp:86-110`).
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(
    has_path: bool,
    has_perms: bool,
    flags: u32,
    veil_state: u32,
    path_diketahui: bool,
    path_empty: bool,
    path_absolute: bool,
) -> i32 {
    // 1. Tanpa jalur DAN tanpa izin -> kunci veil, sukses.
    if !has_path && !has_perms {
        return ADM_LOCK;
    }

    let current_program = (flags & FLAG_CURRENT_PROGRAM) != 0;
    let after_exec = (flags & FLAG_AFTER_EXEC) != 0;

    // 2. Tidak ada flag yang dikenal -> EINVAL.
    if !current_program && !after_exec {
        return ADM_EINVAL;
    }

    // 3. Keadaan diwarisi `execve` -> abaikan diam-diam (program akan melihat
    //    ENOENT nanti bila memang butuh jalur itu).
    if current_program && veil_state == VEIL_LOCKED_INHERITED {
        return ADM_IGNORE;
    }

    // 4. Veil sudah dikunci -> EPERM.
    if current_program && veil_state == VEIL_LOCKED {
        return ADM_EPERM;
    }

    // 5. Hanya salah satu dari jalur/izin yang ada -> EINVAL.
    if !has_path || !has_perms {
        return ADM_EINVAL;
    }

    // 6. Kelayakan jalur. Fase 1 belum menyalin jalur, jadi minta fase 2.
    if !path_diketahui {
        return ADM_BUTUH_PATH;
    }
    if path_empty || !path_absolute {
        return ADM_EINVAL;
    }

    ADM_PROSES
}

/// Cermin transkripsi dari kebijakan C++ historis, ditulis ulang di Rust HANYA
/// sebagai pembanding matriks (lihat [`crate::ksm_keputusan_jalur`] docs §diferensial).
///
/// Ditulis sebagai rangkaian `if` terpisah dari [`putuskan`] supaya salah tulis
/// pada salah satu salinan terdeteksi oleh sapuan ekshaustif.
#[inline]
pub const fn kebijakan_lama(
    has_path: bool,
    has_perms: bool,
    flags: u32,
    veil_state: u32,
    path_diketahui: bool,
    path_empty: bool,
    path_absolute: bool,
) -> i32 {
    if !has_path && !has_perms {
        return ADM_LOCK;
    }
    let cp = (flags & FLAG_CURRENT_PROGRAM) != 0;
    let ae = (flags & FLAG_AFTER_EXEC) != 0;
    if !cp && !ae {
        return ADM_EINVAL;
    }
    if cp && veil_state == VEIL_LOCKED_INHERITED {
        return ADM_IGNORE;
    }
    if cp && veil_state == VEIL_LOCKED {
        return ADM_EPERM;
    }
    if !has_path || !has_perms {
        return ADM_EINVAL;
    }
    if !path_diketahui {
        return ADM_BUTUH_PATH;
    }
    if path_empty || !path_absolute {
        return ADM_EINVAL;
    }
    ADM_PROSES
}

/// Apakah verdict ini sebuah galat (errno-negatif)?
#[inline]
pub const fn verdict_galat(v: i32) -> bool {
    v < 0
}

/// Jumlah kombinasi domain: `has_path(2) x has_perms(2) x flags(4) x veil(4) x
/// path_diketahui(2) x path_empty(2) x path_absolute(2)`.
pub const DOMAIN_KASUS: u32 = 2 * 2 * 4 * 4 * 2 * 2 * 2;

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan admisi permintaan `unveil`. Lihat [`putuskan`] untuk kontrak.
///
/// Konvensi ABI bridge: **hanya integer** — parameter boolean diangkut sebagai
/// `u32` (`0` = salah, selain nol = benar), sama seperti 105 deklarasi
/// `extern "C"` lain di `KlatKernelBridge.h`.
///
/// Menaikkan `prod_calls` dan penghitung verdict yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_decide(
    has_path: u32,
    has_perms: u32,
    flags: u32,
    veil_state: u32,
    path_diketahui: u32,
    path_empty: u32,
    path_absolute: u32,
) -> i32 {
    bump(Counter::ProdCalls);
    let v = putuskan(
        has_path != 0,
        has_perms != 0,
        flags,
        veil_state,
        path_diketahui != 0,
        path_empty != 0,
        path_absolute != 0,
    );
    match v {
        ADM_LOCK => bump(Counter::ProdLock),
        ADM_IGNORE => bump(Counter::ProdIgnore),
        ADM_PROSES => bump(Counter::ProdProses),
        ADM_BUTUH_PATH => bump(Counter::ProdButuhPath),
        ADM_EPERM => bump(Counter::ProdEperm),
        _ => bump(Counter::ProdEinval),
    }
    v
}

/// Verdict `Lock`. C++ TIDAK boleh menyalin angkanya.
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_lock() -> i32 {
    ADM_LOCK
}

/// Verdict `Ignore`.
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_ignore() -> i32 {
    ADM_IGNORE
}

/// Verdict `Proses`.
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_proses() -> i32 {
    ADM_PROSES
}

/// Verdict `ButuhPath`.
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_butuh_path() -> i32 {
    ADM_BUTUH_PATH
}

/// Verdict `EPERM` (negatif).
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_epem() -> i32 {
    ADM_EPERM
}

/// Verdict `EINVAL` (negatif).
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_einval() -> i32 {
    ADM_EINVAL
}

/// Jumlah kombinasi domain.
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_domain() -> u32 {
    DOMAIN_KASUS
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosKeputusanAksesJalurReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosKeputusanAksesJalurReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_lock: u32,
    pub prod_ignore: u32,
    pub prod_eperm: u32,
    pub prod_einval: u32,
    pub prod_proses: u32,
    pub prod_butuh_path: u32,
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

/// Matriks otoritas admisi `unveil`. Input SINTETIS.
fn run_matrix() -> KlatosKeputusanAksesJalurReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // 1. Tanpa jalur & tanpa izin -> Lock (apa pun flag/keadaan veil-nya).
    m.catat(putuskan(false, false, 0, VEIL_NONE, false, false, false) == ADM_LOCK, Group::Peta);
    m.catat(putuskan(false, false, FLAG_CURRENT_PROGRAM, VEIL_LOCKED, false, false, false) == ADM_LOCK, Group::Peta);
    m.catat(putuskan(false, false, FLAG_AFTER_EXEC, VEIL_LOCKED_INHERITED, true, false, false) == ADM_LOCK, Group::Peta);
    // 2. Flag nol -> EINVAL (walau jalur ada).
    m.catat(putuskan(true, true, 0, VEIL_NONE, false, false, false) == ADM_EINVAL, Group::Peta);
    m.catat(putuskan(true, true, 0, VEIL_LOCKED, true, false, true) == ADM_EINVAL, Group::Peta);
    // 3. CurrentProgram pada LockedInherited -> Ignore.
    m.catat(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_LOCKED_INHERITED, false, false, false) == ADM_IGNORE, Group::Peta);
    // 4. CurrentProgram pada Locked -> EPERM.
    m.catat(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_LOCKED, false, false, false) == ADM_EPERM, Group::Peta);
    // AfterExec TIDAK dikenai pemeriksaan keadaan veil.
    m.catat(putuskan(true, true, FLAG_AFTER_EXEC, VEIL_LOCKED, true, false, true) == ADM_PROSES, Group::Peta);
    m.catat(putuskan(true, true, FLAG_AFTER_EXEC, VEIL_LOCKED_INHERITED, true, false, true) == ADM_PROSES, Group::Peta);
    // 5. Jalur XOR izin -> EINVAL.
    m.catat(putuskan(true, false, FLAG_CURRENT_PROGRAM, VEIL_NONE, false, false, false) == ADM_EINVAL, Group::Peta);
    m.catat(putuskan(false, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, false, false, false) == ADM_EINVAL, Group::Peta);
    // 6. Fase 1 dengan jalur+izin -> minta fase 2.
    m.catat(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, false, false, false) == ADM_BUTUH_PATH, Group::Peta);
    m.catat(putuskan(true, true, FLAG_AFTER_EXEC, VEIL_DROPPED, false, false, false) == ADM_BUTUH_PATH, Group::Peta);
    // 7. Fase 2: kelayakan jalur.
    m.catat(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, true, true, false) == ADM_EINVAL, Group::Peta);
    m.catat(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, true, false, false) == ADM_EINVAL, Group::Peta);
    m.catat(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, true, true, true) == ADM_EINVAL, Group::Peta);
    m.catat(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, true, false, true) == ADM_PROSES, Group::Peta);
    m.catat(putuskan(true, true, FLAG_AFTER_EXEC, VEIL_DROPPED, true, false, true) == ADM_PROSES, Group::Peta);

    // --- kelompok 1: sapuan ekshaustif 512 kombinasi ------------------------
    // Setiap kombinasi dijalankan terhadap `putuskan` DAN transkripsi
    // `kebijakan_lama`; keduanya harus sama, dan hasilnya harus salah satu dari
    // enam verdict yang dikenal.
    {
        let mut n = 0u32;
        let mut beda = 0u32;
        let mut tak_dikenal = 0u32;
        let mut n_lock = 0u32;
        let mut n_ignore = 0u32;
        let mut n_proses = 0u32;
        let mut n_butuh = 0u32;
        let mut n_eperm = 0u32;
        let mut n_einval = 0u32;
        let mut hp = 0u32;
        while hp < 2 {
            let mut hpm = 0u32;
            while hpm < 2 {
                let mut fl = 0u32;
                while fl < 4 {
                    let mut vs = 0u32;
                    while vs < 4 {
                        let mut pd = 0u32;
                        while pd < 2 {
                            let mut pe = 0u32;
                            while pe < 2 {
                                let mut pa = 0u32;
                                while pa < 2 {
                                    let a = putuskan(
                                        hp != 0, hpm != 0, fl, vs, pd != 0, pe != 0, pa != 0,
                                    );
                                    let b = kebijakan_lama(
                                        hp != 0, hpm != 0, fl, vs, pd != 0, pe != 0, pa != 0,
                                    );
                                    n += 1;
                                    if a != b {
                                        beda += 1;
                                    }
                                    match a {
                                        ADM_LOCK => n_lock += 1,
                                        ADM_IGNORE => n_ignore += 1,
                                        ADM_PROSES => n_proses += 1,
                                        ADM_BUTUH_PATH => n_butuh += 1,
                                        ADM_EPERM => n_eperm += 1,
                                        ADM_EINVAL => n_einval += 1,
                                        _ => tak_dikenal += 1,
                                    }
                                    pa += 1;
                                }
                                pe += 1;
                            }
                            pd += 1;
                        }
                        vs += 1;
                    }
                    fl += 1;
                }
                hpm += 1;
            }
            hp += 1;
        }
        m.catat(n == DOMAIN_KASUS, Group::Peta);
        m.catat(beda == 0, Group::Peta);
        m.catat(tak_dikenal == 0, Group::Peta);
        // Semua enam verdict muncul di domain -> tidak ada cabang mati.
        m.catat(
            n_lock > 0 && n_ignore > 0 && n_proses > 0 && n_butuh > 0 && n_eperm > 0 && n_einval > 0,
            Group::Peta,
        );
        // Tanpa jalur & tanpa izin, verdict SELALU Lock (independen sisa input).
        m.catat(n_lock == 4 * 4 * 2 * 2 * 2, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(EPERM == 1, Group::Neg);
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(FLAG_CURRENT_PROGRAM == 1, Group::Neg);
    m.catat(FLAG_AFTER_EXEC == 2, Group::Neg);
    m.catat(VEIL_NONE == 0, Group::Neg);
    m.catat(VEIL_DROPPED == 1, Group::Neg);
    m.catat(VEIL_LOCKED == 2, Group::Neg);
    m.catat(VEIL_LOCKED_INHERITED == 3, Group::Neg);
    m.catat(ADM_EPERM == -EPERM, Group::Neg);
    m.catat(ADM_EINVAL == -EINVAL, Group::Neg);
    // Verdict galat HARUS negatif; verdict non-galat HARUS >= 0.
    m.catat(verdict_galat(ADM_EPERM), Group::Neg);
    m.catat(verdict_galat(ADM_EINVAL), Group::Neg);
    m.catat(!verdict_galat(ADM_LOCK), Group::Neg);
    m.catat(!verdict_galat(ADM_IGNORE), Group::Neg);
    m.catat(!verdict_galat(ADM_PROSES), Group::Neg);
    m.catat(!verdict_galat(ADM_BUTUH_PATH), Group::Neg);
    // Verdict-verdict berbeda satu sama lain (tidak ada penggabungan tak sengaja).
    m.catat(ADM_LOCK != ADM_IGNORE, Group::Neg);
    m.catat(ADM_IGNORE != ADM_PROSES, Group::Neg);
    m.catat(ADM_PROSES != ADM_BUTUH_PATH, Group::Neg);
    m.catat(ADM_EPERM != ADM_EINVAL, Group::Neg);
    // Flag di luar bit 0/1 tidak mengaktifkan CurrentProgram/AfterExec.
    m.catat(putuskan(true, true, 0xFFFF_FFFC, VEIL_NONE, false, false, false) == ADM_EINVAL, Group::Neg);
    // Keadaan veil di luar 0..3 diperlakukan seperti bukan Locked/Inherited.
    m.catat(putuskan(true, true, FLAG_CURRENT_PROGRAM, 99, false, false, false) == ADM_BUTUH_PATH, Group::Neg);
    // Determinisme: 1000 panggilan input sama -> verdict sama.
    {
        let mut last: Option<i32> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, true, false, true);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: sebuah harapan yang SALAH harus terdeteksi. Kasus
    // `CurrentProgram` pada keadaan `Locked` bernilai `EPERM`; harapan `EINVAL`
    // (atau `Proses`) harus terbukti berbeda. Ini membuktikan matriks tidak
    // "selalu lulus".
    {
        let benar = putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_LOCKED, false, false, false);
        m.catat(
            benar == ADM_EPERM && benar != ADM_EINVAL && benar != ADM_PROSES,
            Group::Neg,
        );
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosKeputusanAksesJalurReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_ksm_keputusan_jalur_counter(Counter::ProdCalls as u32),
        prod_lock: klatos_ksm_keputusan_jalur_counter(Counter::ProdLock as u32),
        prod_ignore: klatos_ksm_keputusan_jalur_counter(Counter::ProdIgnore as u32),
        prod_eperm: klatos_ksm_keputusan_jalur_counter(Counter::ProdEperm as u32),
        prod_einval: klatos_ksm_keputusan_jalur_counter(Counter::ProdEinval as u32),
        prod_proses: klatos_ksm_keputusan_jalur_counter(Counter::ProdProses as u32),
        prod_butuh_path: klatos_ksm_keputusan_jalur_counter(Counter::ProdButuhPath as u32),
        status,
    }
}

/// Menjalankan matriks otoritas admisi `unveil`.
#[no_mangle]
pub extern "C" fn klatos_ksm_keputusan_jalur_selftest() -> KlatosKeputusanAksesJalurReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EPERM == 1);
        assert!(EINVAL == 22);
        assert!(ADM_LOCK == 0);
        assert!(ADM_IGNORE == 1);
        assert!(ADM_PROSES == 2);
        assert!(ADM_BUTUH_PATH == 3);
        assert!(ADM_EPERM == -1);
        assert!(ADM_EINVAL == -22);
        assert!(FLAG_CURRENT_PROGRAM == 1);
        assert!(FLAG_AFTER_EXEC == 2);
        assert!(VEIL_NONE == 0);
        assert!(VEIL_DROPPED == 1);
        assert!(VEIL_LOCKED == 2);
        assert!(VEIL_LOCKED_INHERITED == 3);
        assert!(DOMAIN_KASUS == 512);
        assert!(REPORT_TAG == 0x4B4C_4154_554E_4144);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdButuhPath as usize == 6);

        // Verdict inti.
        assert!(putuskan(false, false, 0, VEIL_NONE, false, false, false) == ADM_LOCK);
        assert!(putuskan(true, true, 0, VEIL_NONE, true, false, true) == ADM_EINVAL);
        assert!(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_LOCKED_INHERITED, false, false, false) == ADM_IGNORE);
        assert!(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_LOCKED, false, false, false) == ADM_EPERM);
        assert!(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, false, false, false) == ADM_BUTUH_PATH);
        assert!(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, true, false, true) == ADM_PROSES);
        assert!(putuskan(true, true, FLAG_CURRENT_PROGRAM, VEIL_NONE, true, true, true) == ADM_EINVAL);

        // 8 (tag) + 13*4 (u32) + 4 (status) = 64.
        assert!(core::mem::size_of::<KlatosKeputusanAksesJalurReport>() == 64);
        assert!(core::mem::align_of::<KlatosKeputusanAksesJalurReport>() == 8);
    };
}
