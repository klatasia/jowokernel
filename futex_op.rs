// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas DEKODE/NORMALISASI OPERASI `futex()` Jowo (P1.18).


use core::sync::atomic::{AtomicU32, Ordering};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-22).
pub const EINVAL: i32 = 22;
/// `ENOSYS` — indeks ke-38.
pub const ENOSYS: i32 = 38;

/// Cermin `FUTEX_WAIT`.
pub const FUTEX_WAIT: u32 = 1;
/// Cermin `FUTEX_WAKE`.
pub const FUTEX_WAKE: u32 = 2;
/// Cermin `FUTEX_REQUEUE`.
pub const FUTEX_REQUEUE: u32 = 3;
/// Cermin `FUTEX_CMP_REQUEUE`.
pub const FUTEX_CMP_REQUEUE: u32 = 4;
/// Cermin `FUTEX_WAKE_OP`.
pub const FUTEX_WAKE_OP: u32 = 5;
/// Cermin `FUTEX_WAIT_BITSET`.
pub const FUTEX_WAIT_BITSET: u32 = 9;
/// Cermin `FUTEX_WAKE_BITSET`.
pub const FUTEX_WAKE_BITSET: u32 = 10;

/// Cermin `FUTEX_CLOCK_REALTIME` (`1 << 8`).
pub const FUTEX_CLOCK_REALTIME: u32 = 1 << 8;
/// Cermin `FUTEX_PRIVATE_FLAG` (`1 << 9`).
pub const FUTEX_PRIVATE_FLAG: u32 = 1 << 9;
/// Cermin `FUTEX_CMD_MASK` = `~(CLOCK_REALTIME | PRIVATE_FLAG)`.
pub const FUTEX_CMD_MASK: u32 = !(FUTEX_CLOCK_REALTIME | FUTEX_PRIVATE_FLAG);
/// Cermin `FUTEX_BITSET_MATCH_ANY`.
pub const FUTEX_BITSET_MATCH_ANY: u32 = 0xFFFF_FFFF;

/// Bendera hasil: operasi di-share antar-proses (`FUTEX_PRIVATE_FLAG` tidak dipasang).
pub const BENDERA_SHARED: u32 = 1 << 0;
/// Bendera hasil: memakai jam realtime.
pub const BENDERA_REALTIME: u32 = 1 << 1;
/// Bendera hasil: waktu tunggu absolut (dihitung dari `cmd` SEBELUM normalisasi).
pub const BENDERA_ABSOLUT: u32 = 1 << 2;
/// Bendera hasil: `params.timeout` relevan untuk operasi ini.
pub const BENDERA_PERLU_TIMEOUT: u32 = 1 << 3;

/// Semua bit bendera yang dikenal.
pub const BENDERA_SEMUA: u32 = BENDERA_SHARED | BENDERA_REALTIME | BENDERA_ABSOLUT | BENDERA_PERLU_TIMEOUT;

/// Tag laporan; cermin `KlatosFutexOpReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4655_5458; // "KLATFUTX"

/// Hasil dekode operasi futex. Cermin `KlatosFutexOpHasil`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosFutexOpHasil {
    /// `0` = lanjut; `< 0` = `-errno`.
    pub verdict: i32,
    /// Operasi setelah normalisasi.
    pub cmd: u32,
    /// Bitmask `BENDERA_*`.
    pub bendera: u32,
}

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_futex_op_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah keputusan "lanjut".
    ProdLanjut = 1,
    /// Jumlah normalisasi `WAIT_BITSET -> WAIT` (val3 == MATCH_ANY).
    ProdNormalisasiWait = 2,
    /// Jumlah normalisasi `WAKE_BITSET -> WAKE` (val3 == MATCH_ANY).
    ProdNormalisasiWake = 3,
    /// Jumlah penolakan `ENOSYS` (realtime pada non-wait, atau operasi tak dikenal).
    ProdEnosys = 4,
    /// Jumlah penolakan `EINVAL` (bitset dengan `val3 == 0`).
    ProdEinval = 5,
    /// Jumlah keputusan dengan `BENDERA_SHARED`.
    ProdShared = 6,
    /// Jumlah keputusan dengan `BENDERA_REALTIME`.
    ProdRealtime = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Apakah `cmd` termasuk keluarga yang memakai `params.timeout`
/// (`futex.cpp:110-113`).
#[inline]
pub const fn perlu_timeout(cmd: u32) -> bool {
    cmd == FUTEX_WAIT || cmd == FUTEX_WAIT_BITSET || cmd == FUTEX_REQUEUE || cmd == FUTEX_CMP_REQUEUE
}

/// Apakah `cmd` salah satu operasi yang dikenal (`switch` di `futex.cpp:244`).
#[inline]
pub const fn cmd_dikenal(cmd: u32) -> bool {
    cmd == FUTEX_WAIT
        || cmd == FUTEX_WAKE
        || cmd == FUTEX_WAKE_OP
        || cmd == FUTEX_REQUEUE
        || cmd == FUTEX_CMP_REQUEUE
        || cmd == FUTEX_WAIT_BITSET
        || cmd == FUTEX_WAKE_BITSET
}

/// Mendekode dan menormalkan operasi futex.
///
/// Urutan keputusan SAMA PERSIS dengan `Process::sys$futex` historis.
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(futex_op: u32, val3: u32) -> KlatosFutexOpHasil {
    let cmd_awal = futex_op & FUTEX_CMD_MASK;
    let realtime = (futex_op & FUTEX_CLOCK_REALTIME) != 0;

    // 1. Realtime hanya masuk akal untuk operasi menunggu.
    if realtime && cmd_awal != FUTEX_WAIT && cmd_awal != FUTEX_WAIT_BITSET {
        return KlatosFutexOpHasil {
            verdict: -ENOSYS,
            cmd: cmd_awal,
            bendera: 0,
        };
    }

    let mut bendera = 0u32;
    // 2. `shared` = FUTEX_PRIVATE_FLAG TIDAK dipasang.
    if (futex_op & FUTEX_PRIVATE_FLAG) == 0 {
        bendera |= BENDERA_SHARED;
    }
    if realtime {
        bendera |= BENDERA_REALTIME;
    }
    // 3. `is_absolute` historis dihitung dari `cmd` SEBELUM normalisasi.
    if cmd_awal != FUTEX_WAIT {
        bendera |= BENDERA_ABSOLUT;
    }
    if perlu_timeout(cmd_awal) {
        bendera |= BENDERA_PERLU_TIMEOUT;
    }

    // 4. Normalisasi `*_BITSET` + `MATCH_ANY`.
    let mut cmd = cmd_awal;
    if cmd_awal == FUTEX_WAIT_BITSET && val3 == FUTEX_BITSET_MATCH_ANY {
        cmd = FUTEX_WAIT;
    } else if cmd_awal == FUTEX_WAKE_BITSET && val3 == FUTEX_BITSET_MATCH_ANY {
        cmd = FUTEX_WAKE;
    }

    // 5. Operasi tak dikenal -> ENOSYS (jatuh keluar `switch` historis).
    if !cmd_dikenal(cmd) {
        return KlatosFutexOpHasil {
            verdict: -ENOSYS,
            cmd,
            bendera,
        };
    }

    // 6. `*_BITSET` dengan bitset kosong -> EINVAL (diperiksa SESUDAH normalisasi).
    if (cmd == FUTEX_WAIT_BITSET || cmd == FUTEX_WAKE_BITSET) && val3 == 0 {
        return KlatosFutexOpHasil {
            verdict: -EINVAL,
            cmd,
            bendera,
        };
    }

    KlatosFutexOpHasil {
        verdict: 0,
        cmd,
        bendera,
    }
}

/// Cermin transkripsi dari kebijakan C++ historis, ditulis ulang di Rust HANYA
/// sebagai pembanding matriks.
#[inline]
pub const fn kebijakan_lama(futex_op: u32, val3: u32) -> KlatosFutexOpHasil {
    let mut cmd = futex_op & FUTEX_CMD_MASK;
    let realtime = (futex_op & FUTEX_CLOCK_REALTIME) != 0;
    if realtime && cmd != FUTEX_WAIT && cmd != FUTEX_WAIT_BITSET {
        return KlatosFutexOpHasil {
            verdict: -ENOSYS,
            cmd,
            bendera: 0,
        };
    }
    let mut bendera = 0u32;
    if (futex_op & FUTEX_PRIVATE_FLAG) == 0 {
        bendera |= BENDERA_SHARED;
    }
    if realtime {
        bendera |= BENDERA_REALTIME;
    }
    if cmd != FUTEX_WAIT {
        bendera |= BENDERA_ABSOLUT;
    }
    if perlu_timeout(cmd) {
        bendera |= BENDERA_PERLU_TIMEOUT;
    }
    let cmd_awal = cmd;
    match cmd_awal {
        FUTEX_WAIT | FUTEX_WAIT_BITSET | FUTEX_REQUEUE | FUTEX_CMP_REQUEUE => {
            if cmd_awal == FUTEX_WAIT_BITSET && val3 == FUTEX_BITSET_MATCH_ANY {
                cmd = FUTEX_WAIT;
            }
        }
        FUTEX_WAKE_BITSET => {
            if val3 == FUTEX_BITSET_MATCH_ANY {
                cmd = FUTEX_WAKE;
            }
        }
        _ => {}
    }
    if !cmd_dikenal(cmd) {
        return KlatosFutexOpHasil {
            verdict: -ENOSYS,
            cmd,
            bendera,
        };
    }
    if (cmd == FUTEX_WAIT_BITSET || cmd == FUTEX_WAKE_BITSET) && val3 == 0 {
        return KlatosFutexOpHasil {
            verdict: -EINVAL,
            cmd,
            bendera,
        };
    }
    KlatosFutexOpHasil {
        verdict: 0,
        cmd,
        bendera,
    }
}

/// Apakah hasil ini "lanjut"?
#[inline]
pub const fn lanjut(hasil: KlatosFutexOpHasil) -> bool {
    hasil.verdict == 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mendekode operasi futex. Mengembalikan `{ verdict, cmd, bendera }`.
#[no_mangle]
pub extern "C" fn klatos_futex_op_decide(futex_op: u32, val3: u32) -> KlatosFutexOpHasil {
    bump(Counter::ProdCalls);
    let h = putuskan(futex_op, val3);
    let cmd_awal = futex_op & FUTEX_CMD_MASK;
    if h.verdict == 0 {
        bump(Counter::ProdLanjut);
        if cmd_awal == FUTEX_WAIT_BITSET && h.cmd == FUTEX_WAIT {
            bump(Counter::ProdNormalisasiWait);
        }
        if cmd_awal == FUTEX_WAKE_BITSET && h.cmd == FUTEX_WAKE {
            bump(Counter::ProdNormalisasiWake);
        }
    } else if h.verdict == -ENOSYS {
        bump(Counter::ProdEnosys);
    } else if h.verdict == -EINVAL {
        bump(Counter::ProdEinval);
    }
    if h.bendera & BENDERA_SHARED != 0 {
        bump(Counter::ProdShared);
    }
    if h.bendera & BENDERA_REALTIME != 0 {
        bump(Counter::ProdRealtime);
    }
    h
}

/// Verdict "lanjut" (`0`). C++ TIDAK boleh menyalin angkanya.
#[no_mangle]
pub extern "C" fn klatos_futex_op_lanjut() -> i32 {
    0
}
/// Bit bendera `shared`.
#[no_mangle]
pub extern "C" fn klatos_futex_op_bendera_shared() -> u32 {
    BENDERA_SHARED
}
/// Bit bendera `realtime`.
#[no_mangle]
pub extern "C" fn klatos_futex_op_bendera_realtime() -> u32 {
    BENDERA_REALTIME
}
/// Bit bendera `absolut`.
#[no_mangle]
pub extern "C" fn klatos_futex_op_bendera_absolut() -> u32 {
    BENDERA_ABSOLUT
}
/// Bit bendera `perlu_timeout`.
#[no_mangle]
pub extern "C" fn klatos_futex_op_bendera_perlu_timeout() -> u32 {
    BENDERA_PERLU_TIMEOUT
}
/// Nilai `FUTEX_WAIT`/`FUTEX_WAKE` setelah normalisasi (untuk audit).
#[no_mangle]
pub extern "C" fn klatos_futex_op_cmd_wait() -> u32 {
    FUTEX_WAIT
}
#[no_mangle]
pub extern "C" fn klatos_futex_op_cmd_wake() -> u32 {
    FUTEX_WAKE
}
/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_futex_op_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}
/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_futex_op_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosFutexOpReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosFutexOpReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_lanjut: u32,
    pub prod_normalisasi_wait: u32,
    pub prod_normalisasi_wake: u32,
    pub prod_enosys: u32,
    pub prod_einval: u32,
    pub prod_shared: u32,
    pub prod_realtime: u32,
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

/// Matriks otoritas dekode operasi futex. Input SINTETIS.
fn run_matrix() -> KlatosFutexOpReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // WAIT privat.
    {
        let h = putuskan(FUTEX_WAIT, 0);
        m.catat(h.verdict == 0 && h.cmd == FUTEX_WAIT, Group::Peta);
        m.catat(h.bendera == (BENDERA_SHARED | BENDERA_PERLU_TIMEOUT), Group::Peta);
    }
    // WAKE privat (tidak absolut, tidak perlu timeout).
    {
        let h = putuskan(FUTEX_WAKE, 0);
        m.catat(h.verdict == 0 && h.cmd == FUTEX_WAKE, Group::Peta);
        m.catat(h.bendera == (BENDERA_SHARED | BENDERA_ABSOLUT), Group::Peta);
    }
    // WAIT shared (tanpa PRIVATE_FLAG).
    {
        let h = putuskan(FUTEX_WAIT | FUTEX_PRIVATE_FLAG, 0);
        m.catat(h.verdict == 0 && h.bendera == BENDERA_PERLU_TIMEOUT, Group::Peta);
    }
    // Normalisasi WAIT_BITSET + MATCH_ANY -> WAIT.
    {
        let h = putuskan(FUTEX_WAIT_BITSET, FUTEX_BITSET_MATCH_ANY);
        m.catat(h.verdict == 0 && h.cmd == FUTEX_WAIT, Group::Peta);
        // `is_absolute` tetap dihitung dari cmd SEBELUM normalisasi.
        m.catat(h.bendera & BENDERA_ABSOLUT != 0, Group::Peta);
    }
    // Normalisasi WAKE_BITSET + MATCH_ANY -> WAKE.
    {
        let h = putuskan(FUTEX_WAKE_BITSET, FUTEX_BITSET_MATCH_ANY);
        m.catat(h.verdict == 0 && h.cmd == FUTEX_WAKE, Group::Peta);
    }
    // WAIT_BITSET dengan bitset sebagian -> tetap WAIT_BITSET.
    {
        let h = putuskan(FUTEX_WAIT_BITSET, 0x0F);
        m.catat(h.verdict == 0 && h.cmd == FUTEX_WAIT_BITSET, Group::Peta);
    }
    // WAIT_BITSET dengan bitset KOSONG -> EINVAL.
    m.catat(putuskan(FUTEX_WAIT_BITSET, 0).verdict == -EINVAL, Group::Peta);
    m.catat(putuskan(FUTEX_WAKE_BITSET, 0).verdict == -EINVAL, Group::Peta);
    // WAIT_BITSET MATCH_ANY TIDAK kena EINVAL (dinormalisasi lebih dulu).
    m.catat(putuskan(FUTEX_WAIT_BITSET, FUTEX_BITSET_MATCH_ANY).verdict == 0, Group::Peta);
    // Realtime hanya untuk WAIT/WAIT_BITSET.
    m.catat(putuskan(FUTEX_WAIT | FUTEX_CLOCK_REALTIME, 0).verdict == 0, Group::Peta);
    m.catat(putuskan(FUTEX_WAIT_BITSET | FUTEX_CLOCK_REALTIME, 0x0F).verdict == 0, Group::Peta);
    m.catat(putuskan(FUTEX_WAKE | FUTEX_CLOCK_REALTIME, 0).verdict == -ENOSYS, Group::Peta);
    m.catat(putuskan(FUTEX_REQUEUE | FUTEX_CLOCK_REALTIME, 0).verdict == -ENOSYS, Group::Peta);
    // Operasi tak dikenal -> ENOSYS (BUKAN EINVAL).
    m.catat(putuskan(0, 0).verdict == -ENOSYS, Group::Peta);
    m.catat(putuskan(99, 0).verdict == -ENOSYS, Group::Peta);
    m.catat(putuskan(FUTEX_WAKE_OP + 100, 0).verdict == -ENOSYS, Group::Peta);
    // Operasi yang dikenal.
    m.catat(putuskan(FUTEX_REQUEUE, 0).verdict == 0, Group::Peta);
    m.catat(putuskan(FUTEX_CMP_REQUEUE, 0).verdict == 0, Group::Peta);
    m.catat(putuskan(FUTEX_WAKE_OP, 0).verdict == 0, Group::Peta);
    m.catat(putuskan(FUTEX_WAKE_OP, 0).bendera & BENDERA_PERLU_TIMEOUT == 0, Group::Peta);

    // --- kelompok 1: sapuan ekshaustif --------------------------------------
    // futex_op atas semua cmd 0..16 x kedua bit bendera x val3 (5 nilai) = 17*4*5 = 340
    {
        let vals: [u32; 5] = [0, 1, 0x0F, FUTEX_BITSET_MATCH_ANY, 0x8000_0000];
        let mut n = 0u32;
        let mut beda = 0u32;
        let mut luar = 0u32;
        let mut c = 0u32;
        while c < 17 {
            let mut rf = 0u32;
            while rf < 2 {
                let mut pf = 0u32;
                while pf < 2 {
                    for v in vals {
                        let op = c | (rf * FUTEX_CLOCK_REALTIME) | (pf * FUTEX_PRIVATE_FLAG);
                        let a = putuskan(op, v);
                        let b = kebijakan_lama(op, v);
                        n += 1;
                        if a != b {
                            beda += 1;
                        }
                        if a.bendera & !BENDERA_SEMUA != 0 {
                            luar += 1;
                        }
                        if a.verdict > 0 {
                            luar += 1;
                        }
                        if a.verdict < 0 && a.verdict != -EINVAL && a.verdict != -ENOSYS {
                            luar += 1;
                        }
                    }
                    pf += 1;
                }
                rf += 1;
            }
            c += 1;
        }
        m.catat(n == 340, Group::Peta);
        m.catat(beda == 0, Group::Peta);
        m.catat(luar == 0, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(ENOSYS == 38, Group::Neg);
    m.catat(FUTEX_WAIT == 1 && FUTEX_WAKE == 2, Group::Neg);
    m.catat(FUTEX_REQUEUE == 3 && FUTEX_CMP_REQUEUE == 4 && FUTEX_WAKE_OP == 5, Group::Neg);
    m.catat(FUTEX_WAIT_BITSET == 9 && FUTEX_WAKE_BITSET == 10, Group::Neg);
    m.catat(FUTEX_CLOCK_REALTIME == 256 && FUTEX_PRIVATE_FLAG == 512, Group::Neg);
    m.catat(FUTEX_CMD_MASK == !768u32, Group::Neg);
    m.catat(FUTEX_BITSET_MATCH_ANY == 0xFFFF_FFFF, Group::Neg);
    m.catat(EINVAL != ENOSYS, Group::Neg);
    m.catat(BENDERA_SEMUA == 15, Group::Neg);
    // `FUTEX_CMD_MASK` benar-benar membuang kedua bit bendera.
    {
        let mut ok = true;
        let mut c = 0u32;
        while c < 17 {
            let op = c | FUTEX_CLOCK_REALTIME | FUTEX_PRIVATE_FLAG;
            if (op & FUTEX_CMD_MASK) != c {
                ok = false;
            }
            c += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Operasi tak dikenal SELALU ENOSYS, tidak pernah EINVAL.
    {
        let mut ok = true;
        let mut c = 11u32;
        while c < 200 {
            if putuskan(c, 0).verdict != -ENOSYS {
                ok = false;
            }
            c += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // `BENDERA_PERLU_TIMEOUT` tepat pada empat operasi.
    {
        let mut n = 0u32;
        let mut c = 0u32;
        while c < 17 {
            if putuskan(c, 0).bendera & BENDERA_PERLU_TIMEOUT != 0 {
                n += 1;
            }
            c += 1;
        }
        m.catat(n == 4, Group::Neg);
    }
    // Determinisme.
    {
        let mut last: Option<KlatosFutexOpHasil> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(FUTEX_WAIT, 0);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: harapan yang SALAH harus terdeteksi. Operasi tak dikenal
    // TIDAK boleh dianggap `EINVAL` (jebakan nomor 1).
    {
        let benar = putuskan(0, 0).verdict;
        m.catat(benar == -ENOSYS && benar != -EINVAL && benar != 0, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosFutexOpReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_futex_op_counter(Counter::ProdCalls as u32),
        prod_lanjut: klatos_futex_op_counter(Counter::ProdLanjut as u32),
        prod_normalisasi_wait: klatos_futex_op_counter(Counter::ProdNormalisasiWait as u32),
        prod_normalisasi_wake: klatos_futex_op_counter(Counter::ProdNormalisasiWake as u32),
        prod_enosys: klatos_futex_op_counter(Counter::ProdEnosys as u32),
        prod_einval: klatos_futex_op_counter(Counter::ProdEinval as u32),
        prod_shared: klatos_futex_op_counter(Counter::ProdShared as u32),
        prod_realtime: klatos_futex_op_counter(Counter::ProdRealtime as u32),
        status,
    }
}

/// Menjalankan matriks otoritas dekode operasi futex.
#[no_mangle]
pub extern "C" fn klatos_futex_op_selftest() -> KlatosFutexOpReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(EINVAL == 22);
        assert!(ENOSYS == 38);
        assert!(FUTEX_WAIT == 1);
        assert!(FUTEX_WAKE == 2);
        assert!(FUTEX_REQUEUE == 3);
        assert!(FUTEX_CMP_REQUEUE == 4);
        assert!(FUTEX_WAKE_OP == 5);
        assert!(FUTEX_WAIT_BITSET == 9);
        assert!(FUTEX_WAKE_BITSET == 10);
        assert!(FUTEX_CLOCK_REALTIME == 256);
        assert!(FUTEX_PRIVATE_FLAG == 512);
        assert!(FUTEX_CMD_MASK == 0xFFFF_FCFF);
        assert!(FUTEX_BITSET_MATCH_ANY == 0xFFFF_FFFF);
        assert!(BENDERA_SEMUA == 15);
        assert!(REPORT_TAG == 0x4B4C_4154_4655_5458);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdRealtime as usize == 7);

        assert!(putuskan(FUTEX_WAIT, 0).verdict == 0);
        assert!(putuskan(FUTEX_WAIT, 0).cmd == FUTEX_WAIT);
        assert!(putuskan(FUTEX_WAIT_BITSET, FUTEX_BITSET_MATCH_ANY).cmd == FUTEX_WAIT);
        assert!(putuskan(FUTEX_WAKE_BITSET, FUTEX_BITSET_MATCH_ANY).cmd == FUTEX_WAKE);
        assert!(putuskan(FUTEX_WAIT_BITSET, 0).verdict == -EINVAL);
        assert!(putuskan(FUTEX_WAKE | FUTEX_CLOCK_REALTIME, 0).verdict == -ENOSYS);
        assert!(putuskan(0, 0).verdict == -ENOSYS);

        // 8 (tag) + 14*4 (u32) + 4 (status) = 68 -> 72.
        assert!(core::mem::size_of::<KlatosFutexOpReport>() == 72);
        assert!(core::mem::align_of::<KlatosFutexOpReport>() == 8);
        // Hasil: 3 * 4 = 12 byte.
        assert!(core::mem::size_of::<KlatosFutexOpHasil>() == 12);
        assert!(core::mem::align_of::<KlatosFutexOpHasil>() == 4);
    };
}
