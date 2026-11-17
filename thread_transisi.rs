// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas LEGALITAS + KLASIFIKASI AKSI transisi keadaan Thread Jowo (P1.16).


use core::sync::atomic::{AtomicU32, Ordering};

/// Cermin `Thread::State`.
pub const ST_INVALID: u32 = 0;
pub const ST_RUNNABLE: u32 = 1;
pub const ST_RUNNING: u32 = 2;
pub const ST_DYING: u32 = 3;
pub const ST_DEAD: u32 = 4;
pub const ST_STOPPED: u32 = 5;
pub const ST_BLOCKED: u32 = 6;
/// Jumlah keadaan.
pub const N_STATE: u32 = 7;

/// Verdict: transisi tidak sah karena keadaan tujuan SAMA dengan keadaan kini
/// (`PANIC("Thread … is already …")`).
pub const TRANSISI_ILEGAL_SAMA: i32 = 0;
/// Verdict: `Stopped` dari keadaan selain `Running`
/// (`PANIC("Attempting to stop with invalid thread state - …")`).
pub const TRANSISI_ILEGAL_STOP: i32 = 1;
/// Verdict: `Dying` dari `Blocked` (kegagalan `VERIFY` historis).
pub const TRANSISI_ILEGAL_DYING: i32 = 2;
/// Verdict: transisi sah.
pub const TRANSISI_SAH: i32 = 3;

/// Aksi: keadaan sebelumnya `Runnable` -> `Scheduler::dequeue_runnable_thread`.
pub const AKSI_DEQUEUE: u32 = 1 << 0;
/// Aksi: keadaan sebelumnya `Stopped` -> bangunkan peer + `unblock_waiters` +
/// `SIGCHLD` ke induk.
pub const AKSI_RESUME_PEERS: u32 = 1 << 1;
/// Aksi: keadaan baru `Runnable` -> `Scheduler::enqueue_runnable_thread` +
/// `Processor::smp_wake_n_idle_processors(1)`.
pub const AKSI_ENQUEUE_WAKE: u32 = 1 << 2;
/// Aksi: keadaan baru `Stopped` -> `Process::set_stopped(true)` + `unblock_waiters`
/// + `SIGCHLD` ke induk.
pub const AKSI_STOP_PROSES: u32 = 1 << 3;
/// Aksi: keadaan baru `Dying` -> `Scheduler::notify_finalizer()` (C++ masih
/// memasang syarat `this != Thread::current() && is_finalizable()`).
pub const AKSI_NOTIFY_FINALIZER: u32 = 1 << 4;

/// Semua bit aksi yang dikenal.
pub const AKSI_SEMUA: u32 = AKSI_DEQUEUE | AKSI_RESUME_PEERS | AKSI_ENQUEUE_WAKE | AKSI_STOP_PROSES | AKSI_NOTIFY_FINALIZER;

/// Tag laporan; cermin `KlatosThreadTransisiReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5448_5244; // "KLATTHRD"

/// Hasil keputusan transisi. Cermin `KlatosThreadTransisiHasil`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosThreadTransisiHasil {
    /// Salah satu `TRANSISI_*`.
    pub verdict: i32,
    /// Bitmask `AKSI_*`.
    pub aksi: u32,
}

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_thread_transisi_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah transisi SAH.
    ProdSah = 1,
    /// Jumlah klasifikasi dengan aksi `dequeue`.
    ProdDequeue = 2,
    /// Jumlah klasifikasi dengan aksi `resume_peers`.
    ProdResumePeers = 3,
    /// Jumlah klasifikasi dengan aksi `enqueue_wake`.
    ProdEnqueueWake = 4,
    /// Jumlah klasifikasi dengan aksi `stop_proses`.
    ProdStopProses = 5,
    /// Jumlah klasifikasi dengan aksi `notify_finalizer`.
    ProdNotifyFinalizer = 6,
    /// Jumlah transisi TIDAK SAH (semua sebab).
    ProdIlegal = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Memutuskan legalitas dan aksi yang diperlukan untuk transisi
/// `previous_state -> new_state`.
///
/// Urutan pemeriksaan SAMA PERSIS dengan `Thread::set_state` historis.
///
/// `# Safety`: tidak ada — fungsi ini murni, tanpa pointer.
#[inline]
pub const fn putuskan(previous: u32, next: u32) -> KlatosThreadTransisiHasil {
    // 1. Keadaan tujuan sama dengan keadaan kini -> PANIC "already".
    if next == previous {
        return KlatosThreadTransisiHasil {
            verdict: TRANSISI_ILEGAL_SAMA,
            aksi: 0,
        };
    }

    let mut aksi = 0u32;
    // 2. Aksi yang berasal dari keadaan SEBELUMNYA.
    if previous == ST_RUNNABLE {
        aksi |= AKSI_DEQUEUE;
    } else if previous == ST_STOPPED {
        aksi |= AKSI_RESUME_PEERS;
    }
    // 3. Aksi yang berasal dari keadaan BARU.
    if next == ST_RUNNABLE {
        aksi |= AKSI_ENQUEUE_WAKE;
    } else if next == ST_STOPPED {
        aksi |= AKSI_STOP_PROSES;
    } else if next == ST_DYING {
        aksi |= AKSI_NOTIFY_FINALIZER;
    }

    // 4. Legalitas lanjutan, pada titik yang sama dengan C++ historis.
    let verdict = if next == ST_STOPPED {
        if previous != ST_RUNNING {
            TRANSISI_ILEGAL_STOP
        } else {
            TRANSISI_SAH
        }
    } else if next == ST_DYING {
        if previous == ST_BLOCKED {
            TRANSISI_ILEGAL_DYING
        } else {
            TRANSISI_SAH
        }
    } else {
        TRANSISI_SAH
    };

    KlatosThreadTransisiHasil { verdict, aksi }
}

/// Cermin transkripsi dari `Thread::set_state` historis, ditulis ulang di Rust
/// HANYA sebagai pembanding matriks.
#[inline]
pub const fn kebijakan_lama(previous: u32, next: u32) -> KlatosThreadTransisiHasil {
    if next == previous {
        return KlatosThreadTransisiHasil {
            verdict: TRANSISI_ILEGAL_SAMA,
            aksi: 0,
        };
    }
    let mut aksi = 0u32;
    if previous == ST_RUNNABLE {
        aksi |= AKSI_DEQUEUE;
    } else if previous == ST_STOPPED {
        aksi |= AKSI_RESUME_PEERS;
    }
    let mut verdict = TRANSISI_SAH;
    if next == ST_RUNNABLE {
        aksi |= AKSI_ENQUEUE_WAKE;
    } else if next == ST_STOPPED {
        aksi |= AKSI_STOP_PROSES;
        if previous != ST_RUNNING {
            verdict = TRANSISI_ILEGAL_STOP;
        }
    } else if next == ST_DYING {
        aksi |= AKSI_NOTIFY_FINALIZER;
        if previous == ST_BLOCKED {
            verdict = TRANSISI_ILEGAL_DYING;
        }
    }
    KlatosThreadTransisiHasil { verdict, aksi }
}

/// Apakah verdict ini sah?
#[inline]
pub const fn sah(verdict: i32) -> bool {
    verdict == TRANSISI_SAH
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memutuskan transisi keadaan thread. Mengembalikan `{ verdict, aksi }`.
///
/// Menaikkan `prod_calls` dan penghitung yang sesuai.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_decide(previous: u32, next: u32) -> KlatosThreadTransisiHasil {
    bump(Counter::ProdCalls);
    let h = putuskan(previous, next);
    if h.verdict == TRANSISI_SAH {
        bump(Counter::ProdSah);
    } else {
        bump(Counter::ProdIlegal);
    }
    if h.aksi & AKSI_DEQUEUE != 0 {
        bump(Counter::ProdDequeue);
    }
    if h.aksi & AKSI_RESUME_PEERS != 0 {
        bump(Counter::ProdResumePeers);
    }
    if h.aksi & AKSI_ENQUEUE_WAKE != 0 {
        bump(Counter::ProdEnqueueWake);
    }
    if h.aksi & AKSI_STOP_PROSES != 0 {
        bump(Counter::ProdStopProses);
    }
    if h.aksi & AKSI_NOTIFY_FINALIZER != 0 {
        bump(Counter::ProdNotifyFinalizer);
    }
    h
}

/// Verdict sah. C++ TIDAK boleh menyalin angkanya.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_sah() -> i32 {
    TRANSISI_SAH
}
/// Verdict `ilegal_sama`.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_ilegal_sama() -> i32 {
    TRANSISI_ILEGAL_SAMA
}
/// Verdict `ilegal_stop`.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_ilegal_stop() -> i32 {
    TRANSISI_ILEGAL_STOP
}
/// Verdict `ilegal_dying`.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_ilegal_dying() -> i32 {
    TRANSISI_ILEGAL_DYING
}
/// Bit aksi `dequeue`.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_aksi_dequeue() -> u32 {
    AKSI_DEQUEUE
}
/// Bit aksi `resume_peers`.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_aksi_resume_peers() -> u32 {
    AKSI_RESUME_PEERS
}
/// Bit aksi `enqueue_wake`.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_aksi_enqueue_wake() -> u32 {
    AKSI_ENQUEUE_WAKE
}
/// Bit aksi `stop_proses`.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_aksi_stop_proses() -> u32 {
    AKSI_STOP_PROSES
}
/// Bit aksi `notify_finalizer`.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_aksi_notify_finalizer() -> u32 {
    AKSI_NOTIFY_FINALIZER
}
/// Jumlah keadaan thread.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_n_state() -> u32 {
    N_STATE
}
/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}
/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosThreadTransisiReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosThreadTransisiReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_calls: u32,
    pub prod_sah: u32,
    pub prod_dequeue: u32,
    pub prod_resume_peers: u32,
    pub prod_enqueue_wake: u32,
    pub prod_stop_proses: u32,
    pub prod_notify_finalizer: u32,
    pub prod_ilegal: u32,
    pub n_state: u32,
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

/// Matriks otoritas transisi thread. Input SINTETIS.
fn run_matrix() -> KlatosThreadTransisiReport {
    let mut m = Matrix::new();

    // --- kelompok 0: kasus batas eksplisit ---------------------------------
    // Sama -> ilegal_sama, tanpa aksi.
    m.catat(putuskan(ST_RUNNING, ST_RUNNING).verdict == TRANSISI_ILEGAL_SAMA, Group::Peta);
    m.catat(putuskan(ST_BLOCKED, ST_BLOCKED).aksi == 0, Group::Peta);
    m.catat(putuskan(ST_RUNNING, ST_RUNNING).aksi == 0, Group::Peta);
    // Running -> Runnable: enqueue + wake.
    m.catat(putuskan(ST_RUNNING, ST_RUNNABLE).verdict == TRANSISI_SAH, Group::Peta);
    m.catat(putuskan(ST_RUNNING, ST_RUNNABLE).aksi == AKSI_ENQUEUE_WAKE, Group::Peta);
    // Runnable -> Running: dequeue.
    m.catat(putuskan(ST_RUNNABLE, ST_RUNNING).aksi == AKSI_DEQUEUE, Group::Peta);
    // Running -> Stopped: stop proses.
    m.catat(putuskan(ST_RUNNING, ST_STOPPED).verdict == TRANSISI_SAH, Group::Peta);
    m.catat(putuskan(ST_RUNNING, ST_STOPPED).aksi == AKSI_STOP_PROSES, Group::Peta);
    // Stopped -> Running: resume peers.
    m.catat(putuskan(ST_STOPPED, ST_RUNNING).verdict == TRANSISI_SAH, Group::Peta);
    m.catat(putuskan(ST_STOPPED, ST_RUNNING).aksi == AKSI_RESUME_PEERS, Group::Peta);
    // Runnable -> Stopped: dequeue + stop, TAPI ilegal (bukan dari Running).
    m.catat(putuskan(ST_RUNNABLE, ST_STOPPED).verdict == TRANSISI_ILEGAL_STOP, Group::Peta);
    m.catat(putuskan(ST_RUNNABLE, ST_STOPPED).aksi == (AKSI_DEQUEUE | AKSI_STOP_PROSES), Group::Peta);
    // Running -> Dying: notify finalizer.
    m.catat(putuskan(ST_RUNNING, ST_DYING).verdict == TRANSISI_SAH, Group::Peta);
    m.catat(putuskan(ST_RUNNING, ST_DYING).aksi == AKSI_NOTIFY_FINALIZER, Group::Peta);
    // Blocked -> Dying: ilegal (VERIFY historis).
    m.catat(putuskan(ST_BLOCKED, ST_DYING).verdict == TRANSISI_ILEGAL_DYING, Group::Peta);
    m.catat(putuskan(ST_BLOCKED, ST_DYING).aksi == AKSI_NOTIFY_FINALIZER, Group::Peta);
    // Running -> Dead: tidak ada aksi.
    m.catat(putuskan(ST_RUNNING, ST_DEAD).verdict == TRANSISI_SAH, Group::Peta);
    m.catat(putuskan(ST_RUNNING, ST_DEAD).aksi == 0, Group::Peta);

    // --- kelompok 1: sapuan ekshaustif 7x7 = 49 -----------------------------
    {
        let mut n = 0u32;
        let mut beda = 0u32;
        let mut n_sama = 0u32;
        let mut n_stop = 0u32;
        let mut n_dying = 0u32;
        let mut n_sah = 0u32;
        let mut aksi_luar = 0u32;
        let mut p = 0u32;
        while p < N_STATE {
            let mut q = 0u32;
            while q < N_STATE {
                let a = putuskan(p, q);
                let b = kebijakan_lama(p, q);
                n += 1;
                if a != b {
                    beda += 1;
                }
                if a.aksi & !AKSI_SEMUA != 0 {
                    aksi_luar += 1;
                }
                match a.verdict {
                    TRANSISI_ILEGAL_SAMA => n_sama += 1,
                    TRANSISI_ILEGAL_STOP => n_stop += 1,
                    TRANSISI_ILEGAL_DYING => n_dying += 1,
                    TRANSISI_SAH => n_sah += 1,
                    _ => aksi_luar += 1,
                }
                q += 1;
            }
            p += 1;
        }
        m.catat(n == 49, Group::Peta);
        m.catat(beda == 0, Group::Peta);
        m.catat(aksi_luar == 0, Group::Peta);
        m.catat(n_sama == 7, Group::Peta);
        m.catat(n_sah == 49 - 7 - 5 - 1, Group::Peta);
        m.catat(n_stop == 5, Group::Peta);
        m.catat(n_dying == 1, Group::Peta);
    }

    // --- kelompok 2: negatif ------------------------------------------------
    m.catat(N_STATE == 7, Group::Neg);
    m.catat(ST_INVALID == 0 && ST_RUNNABLE == 1 && ST_RUNNING == 2 && ST_DYING == 3, Group::Neg);
    m.catat(ST_DEAD == 4 && ST_STOPPED == 5 && ST_BLOCKED == 6, Group::Neg);
    m.catat(TRANSISI_ILEGAL_SAMA == 0 && TRANSISI_ILEGAL_STOP == 1, Group::Neg);
    m.catat(TRANSISI_ILEGAL_DYING == 2 && TRANSISI_SAH == 3, Group::Neg);
    m.catat(sah(TRANSISI_SAH) && !sah(TRANSISI_ILEGAL_SAMA), Group::Neg);
    // Aksi selalu ⊆ AKSI_SEMUA.
    {
        let mut ok = true;
        let mut p = 0u32;
        while p < N_STATE {
            let mut q = 0u32;
            while q < N_STATE {
                if putuskan(p, q).aksi & !AKSI_SEMUA != 0 {
                    ok = false;
                }
                q += 1;
            }
            p += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Sama -> TIDAK PERNAH sah.
    {
        let mut ok = true;
        let mut p = 0u32;
        while p < N_STATE {
            if sah(putuskan(p, p).verdict) {
                ok = false;
            }
            p += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // `Stopped` HANYA sah dari `Running`.
    {
        let mut ok = true;
        let mut p = 0u32;
        while p < N_STATE {
            let s = sah(putuskan(p, ST_STOPPED).verdict);
            if s != (p == ST_RUNNING) {
                ok = false;
            }
            p += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // `Dying` TIDAK sah HANYA dari `Blocked`.
    {
        let mut ok = true;
        let mut p = 0u32;
        while p < N_STATE {
            let s = sah(putuskan(p, ST_DYING).verdict);
            if s != (p != ST_BLOCKED && p != ST_DYING) {
                ok = false;
            }
            p += 1;
        }
        m.catat(ok, Group::Neg);
    }
    // Determinisme.
    {
        let mut last: Option<KlatosThreadTransisiHasil> = None;
        let mut ok = true;
        for _ in 0..1000 {
            let r = putuskan(ST_RUNNING, ST_RUNNABLE);
            if last.is_none() {
                last = Some(r);
            } else if last != Some(r) {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Neg);
    }
    // Kontrol negatif: harapan yang SALAH harus terdeteksi.
    {
        let benar = putuskan(ST_BLOCKED, ST_DYING).verdict;
        m.catat(benar == TRANSISI_ILEGAL_DYING && benar != TRANSISI_SAH, Group::Neg);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosThreadTransisiReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_calls: klatos_thread_transisi_counter(Counter::ProdCalls as u32),
        prod_sah: klatos_thread_transisi_counter(Counter::ProdSah as u32),
        prod_dequeue: klatos_thread_transisi_counter(Counter::ProdDequeue as u32),
        prod_resume_peers: klatos_thread_transisi_counter(Counter::ProdResumePeers as u32),
        prod_enqueue_wake: klatos_thread_transisi_counter(Counter::ProdEnqueueWake as u32),
        prod_stop_proses: klatos_thread_transisi_counter(Counter::ProdStopProses as u32),
        prod_notify_finalizer: klatos_thread_transisi_counter(Counter::ProdNotifyFinalizer as u32),
        prod_ilegal: klatos_thread_transisi_counter(Counter::ProdIlegal as u32),
        n_state: N_STATE,
        status,
    }
}

/// Menjalankan matriks otoritas transisi thread.
#[no_mangle]
pub extern "C" fn klatos_thread_transisi_selftest() -> KlatosThreadTransisiReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(ST_INVALID == 0);
        assert!(ST_RUNNABLE == 1);
        assert!(ST_RUNNING == 2);
        assert!(ST_DYING == 3);
        assert!(ST_DEAD == 4);
        assert!(ST_STOPPED == 5);
        assert!(ST_BLOCKED == 6);
        assert!(N_STATE == 7);
        assert!(TRANSISI_ILEGAL_SAMA == 0);
        assert!(TRANSISI_ILEGAL_STOP == 1);
        assert!(TRANSISI_ILEGAL_DYING == 2);
        assert!(TRANSISI_SAH == 3);
        assert!(AKSI_DEQUEUE == 1);
        assert!(AKSI_RESUME_PEERS == 2);
        assert!(AKSI_ENQUEUE_WAKE == 4);
        assert!(AKSI_STOP_PROSES == 8);
        assert!(AKSI_NOTIFY_FINALIZER == 16);
        assert!(AKSI_SEMUA == 31);
        assert!(REPORT_TAG == 0x4B4C_4154_5448_5244);
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdIlegal as usize == 7);

        assert!(putuskan(ST_RUNNING, ST_RUNNING).verdict == TRANSISI_ILEGAL_SAMA);
        assert!(putuskan(ST_RUNNING, ST_RUNNABLE).aksi == AKSI_ENQUEUE_WAKE);
        assert!(putuskan(ST_RUNNABLE, ST_RUNNING).aksi == AKSI_DEQUEUE);
        assert!(putuskan(ST_RUNNABLE, ST_STOPPED).verdict == TRANSISI_ILEGAL_STOP);
        assert!(putuskan(ST_BLOCKED, ST_DYING).verdict == TRANSISI_ILEGAL_DYING);
        assert!(putuskan(ST_STOPPED, ST_RUNNING).aksi == AKSI_RESUME_PEERS);

        // 8 (tag) + 15*4 (u32) + 4 (status) = 72 -> 72.
        assert!(core::mem::size_of::<KlatosThreadTransisiReport>() == 72);
        assert!(core::mem::align_of::<KlatosThreadTransisiReport>() == 8);
        // Hasil: 4 + 4 = 8 byte.
        assert!(core::mem::size_of::<KlatosThreadTransisiHasil>() == 8);
        assert!(core::mem::align_of::<KlatosThreadTransisiHasil>() == 4);
    };
}
