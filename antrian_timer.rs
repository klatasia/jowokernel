// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8.5-B — **OTORITAS KEBIJAKAN TIMER TERURUT (generik)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, Ordering};

/// Tag laporan; cermin `KlatosTimerAntreReport` di `KlatSeam135.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5449_4D52; // "KLATTIMR"

/// Indeks/deskriptor tidak sah.
pub const INVALID: u32 = 0xFFFF_FFFF;

/// Jumlah timer maksimum (tetap, tanpa alokasi).
pub const MAX_TIMER: usize = 1024;

/// Kelas jam.
pub const K_MONO: u32 = 0;
/// Kelas jam.
pub const K_REAL: u32 = 1;
/// Kelas uji terisolasi — TIDAK pernah dipakai adapter C++ maupun `fire()`
/// nyata (`TimerQueue::the().fire()` hanya mengiterasi `K_MONO`/`K_REAL`), jadi
/// matriks otoritas dapat menguji urutan/`next`/`siap` tanpa dipengaruhi timer
/// konsumen nyata yang hidup di arena yang sama.
pub const K_UJI: u32 = 2;
/// Sentinel "semua kelas" untuk [`jumlah`].
pub const K_SEMUA: u32 = 0xFFFF_FFFF;

/// Status slot.
pub const T_FREE: u32 = 0;
/// Menunggu deadline.
pub const T_QUEUED: u32 = 1;
/// Dipilih untuk dipanggil; callback tertunda.
pub const T_EXEC: u32 = 2;

/// `-EBADF`.
pub const E_BADF: i32 = 9;
/// `-EINVAL`.
pub const E_INVAL: i32 = 22;

/// Nilai "tidak ada deadline".
pub const TIDAK_ADA: i64 = i64::MAX;

/// Penghitung audit.
#[derive(Clone, Copy)]
pub enum Counter {
    ProdDaftar = 0,
    ProdBatalQueued = 1,
    ProdBatalExec = 2,
    ProdSiap = 3,
    ProdSelesai = 4,
    ProdTolakStale = 5,
    ProdTolakPenuh = 6,
    ProdTolakState = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- arena ------------------------------------------------------------------

static AT_GEN: [AtomicU32; MAX_TIMER] = [const { AtomicU32::new(0) }; MAX_TIMER];
static AT_ST: [AtomicU32; MAX_TIMER] = [const { AtomicU32::new(T_FREE) }; MAX_TIMER];
static AT_CLASS: [AtomicU32; MAX_TIMER] = [const { AtomicU32::new(K_MONO) }; MAX_TIMER];
static AT_DEADLINE: [AtomicI64; MAX_TIMER] = [const { AtomicI64::new(0) }; MAX_TIMER];
static AT_SEQ: [AtomicU64; MAX_TIMER] = [const { AtomicU64::new(0) }; MAX_TIMER];
static AT_BATAL: [AtomicU32; MAX_TIMER] = [const { AtomicU32::new(0) }; MAX_TIMER];
static AT_NEXT: [AtomicU32; MAX_TIMER] = [const { AtomicU32::new(INVALID) }; MAX_TIMER];
static AT_FREE: AtomicU32 = AtomicU32::new(INVALID);
static AT_SEQN: AtomicU64 = AtomicU64::new(1);
static AT_INIT: AtomicU32 = AtomicU32::new(0);
static AT_LOCK: AtomicU32 = AtomicU32::new(0);

/// Ambil kunci arena; kembalikan `true` bila interupsi tadinya menyala.
///
/// Jalur IRQ nyata (`TimerQueue::fire()` lewat `TimeManagement`) memakai pintu
/// ini, jadi kunci HARUS IRQ-aman pada target kernel: `cli` menahan IRQ pada
/// CPU ini selama kunci dipegang, lalu keadaan semula dipulihkan. Harness host
/// (tanpa `crate::arch`) memakai jalur spinlock polos.
#[inline]
fn lock() -> bool {
    #[cfg(target_os = "klatos")]
    {
        let was = crate::arch::interrupts_enabled();
        crate::arch::disable_interrupts();
        while AT_LOCK
            .compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        was
    }
    #[cfg(not(target_os = "klatos"))]
    {
        while AT_LOCK
            .compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        false
    }
}
#[inline]
fn unlock(was_enabled: bool) {
    AT_LOCK.store(0, Ordering::Release);
    #[cfg(target_os = "klatos")]
    {
        if was_enabled {
            crate::arch::enable_interrupts();
        }
    }
    #[cfg(not(target_os = "klatos"))]
    {
        let _ = was_enabled;
    }
}

fn pastikan_init() {
    if AT_INIT.load(Ordering::Acquire) == 1 {
        return;
    }
    let _if = lock();
    if AT_INIT.load(Ordering::Relaxed) == 0 {
        let mut i = 0usize;
        while i < MAX_TIMER {
            AT_GEN[i].store(0, Ordering::Relaxed);
            AT_ST[i].store(T_FREE, Ordering::Relaxed);
            AT_BATAL[i].store(0, Ordering::Relaxed);
            AT_NEXT[i].store(
                if i + 1 < MAX_TIMER { (i + 1) as u32 } else { INVALID },
                Ordering::Relaxed,
            );
            i += 1;
        }
        AT_FREE.store(0, Ordering::Relaxed);
        AT_SEQN.store(1, Ordering::Relaxed);
        AT_INIT.store(1, Ordering::Release);
    }
    unlock(_if);
}

#[inline]
fn hidup(index: u32, generation: u32) -> bool {
    let i = index as usize;
    generation != 0
        && i < MAX_TIMER
        && AT_GEN[i].load(Ordering::Acquire) == generation
        && AT_ST[i].load(Ordering::Acquire) != T_FREE
}

#[inline]
fn handle_of(index: u32, generation: u32) -> u64 {
    ((generation as u64) << 32) | index as u64
}

/// `daftar` — registrasi timer baru; 0 bila kelas tak sah / arena penuh.
///
/// Kontrak publik tetap dua kelas (`K_MONO`/`K_REAL`); kelas uji `K_UJI`
/// hanya dapat dipakai lewat [`daftar_inti`] dari matriks otoritas.
pub fn daftar(class: u32, deadline: i64) -> u64 {
    if class > K_REAL {
        bump(Counter::ProdTolakState);
        return 0;
    }
    daftar_inti(class, deadline)
}

/// Inti `daftar` tanpa gerbang kelas publik (dipakai `K_UJI` oleh matriks).
fn daftar_inti(class: u32, deadline: i64) -> u64 {
    pastikan_init();
    let _if = lock();
    let free = AT_FREE.load(Ordering::Relaxed);
    let result = if class > K_UJI {
        bump(Counter::ProdTolakState);
        0
    } else if free == INVALID {
        bump(Counter::ProdTolakPenuh);
        0
    } else {
        let s = free as usize;
        AT_FREE.store(AT_NEXT[s].load(Ordering::Relaxed), Ordering::Relaxed);
        AT_NEXT[s].store(INVALID, Ordering::Relaxed);
        let generation = AT_GEN[s].load(Ordering::Relaxed).wrapping_add(1);
        if generation == 0 {
            AT_NEXT[s].store(AT_FREE.load(Ordering::Relaxed), Ordering::Relaxed);
            AT_FREE.store(s as u32, Ordering::Relaxed);
            0
        } else {
            AT_GEN[s].store(generation, Ordering::Relaxed);
            AT_CLASS[s].store(class, Ordering::Relaxed);
            AT_DEADLINE[s].store(deadline, Ordering::Relaxed);
            AT_SEQ[s].store(AT_SEQN.fetch_add(1, Ordering::Relaxed), Ordering::Relaxed);
            AT_BATAL[s].store(0, Ordering::Relaxed);
            AT_ST[s].store(T_QUEUED, Ordering::Release);
            bump(Counter::ProdDaftar);
            handle_of(s as u32, generation)
        }
    };
    unlock(_if);
    result
}

/// `batal` — batalkan timer. `1` = dibatalkan dari QUEUED; `2` = ditandai batal
/// saat EXEC; `-EBADF` = usang/ganda.
pub fn batal(handle: u64) -> i32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        bump(Counter::ProdTolakStale);
        -E_BADF
    } else {
        let i = index as usize;
        let st = AT_ST[i].load(Ordering::Relaxed);
        if st == T_QUEUED {
            AT_ST[i].store(T_FREE, Ordering::Release);
            AT_NEXT[i].store(AT_FREE.load(Ordering::Relaxed), Ordering::Relaxed);
            AT_FREE.store(i as u32, Ordering::Release);
            bump(Counter::ProdBatalQueued);
            1
        } else if st == T_EXEC && AT_BATAL[i].load(Ordering::Relaxed) == 0 {
            AT_BATAL[i].store(1, Ordering::Release);
            bump(Counter::ProdBatalExec);
            2
        } else {
            bump(Counter::ProdTolakState);
            -E_BADF
        }
    };
    unlock(_if);
    r
}

/// `status` — `T_*` atau `-EBADF`.
pub fn status(handle: u64) -> i32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        -E_BADF
    } else {
        AT_ST[index as usize].load(Ordering::Relaxed) as i32
    };
    unlock(_if);
    r
}

/// `batal_p` — `1` bila EXEC sudah ditandai batal.
pub fn batal_p(handle: u64) -> u32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        0
    } else {
        AT_BATAL[index as usize].load(Ordering::Relaxed)
    };
    unlock(_if);
    r
}

/// `next` — handle timer QUEUED paling awal pada kelas; 0 bila kosong.
pub fn next(class: u32) -> u64 {
    pastikan_init();
    let _if = lock();
    let mut best = INVALID;
    let mut best_d: i64 = TIDAK_ADA;
    let mut best_seq: u64 = u64::MAX;
    let mut i = 0usize;
    while i < MAX_TIMER {
        if AT_ST[i].load(Ordering::Relaxed) == T_QUEUED
            && AT_CLASS[i].load(Ordering::Relaxed) == class
        {
            let d = AT_DEADLINE[i].load(Ordering::Relaxed);
            let sq = AT_SEQ[i].load(Ordering::Relaxed);
            if d < best_d || (d == best_d && sq < best_seq) {
                best_d = d;
                best_seq = sq;
                best = i as u32;
            }
        }
        i += 1;
    }
    let r = if best == INVALID {
        0
    } else {
        handle_of(best, AT_GEN[best as usize].load(Ordering::Relaxed))
    };
    unlock(_if);
    r
}

/// `next_due` — deadline paling awal pada kelas, atau `TIDAK_ADA`.
pub fn next_due(class: u32) -> i64 {
    let h = next(class);
    if h == 0 {
        return TIDAK_ADA;
    }
    let index = h as u32;
    let generation = (h >> 32) as u32;
    let _if = lock();
    let r = if hidup(index, generation) {
        AT_DEADLINE[index as usize].load(Ordering::Relaxed)
    } else {
        TIDAK_ADA
    };
    unlock(_if);
    r
}

/// `siap` — pilih semua QUEUED dengan `deadline < now` (paling awal dulu),
/// tandai EXEC, tulis handle ke `out`. Mengembalikan jumlah (≥0) atau `-EINVAL`.
///
/// # Safety
/// `out` harus menunjuk `cap` `u64` yang dapat ditulis.
pub unsafe fn siap(class: u32, now: i64, out: *mut u64, cap: u32) -> i32 {
    pastikan_init();
    if out.is_null() {
        return -E_INVAL;
    }
    let _if = lock();
    let mut n = 0u32;
    loop {
        if n >= cap {
            break;
        }
        let mut best = INVALID;
        let mut best_d: i64 = TIDAK_ADA;
        let mut best_seq: u64 = u64::MAX;
        let mut i = 0usize;
        while i < MAX_TIMER {
            if AT_ST[i].load(Ordering::Relaxed) == T_QUEUED
                && AT_CLASS[i].load(Ordering::Relaxed) == class
            {
                let d = AT_DEADLINE[i].load(Ordering::Relaxed);
                if d < now {
                    let sq = AT_SEQ[i].load(Ordering::Relaxed);
                    if d < best_d || (d == best_d && sq < best_seq) {
                        best_d = d;
                        best_seq = sq;
                        best = i as u32;
                    }
                }
            }
            i += 1;
        }
        if best == INVALID {
            break;
        }
        let bi = best as usize;
        AT_ST[bi].store(T_EXEC, Ordering::Release);
        bump(Counter::ProdSiap);
        let g = AT_GEN[bi].load(Ordering::Relaxed);
        core::ptr::write(out.add(n as usize), handle_of(best, g));
        n += 1;
    }
    unlock(_if);
    n as i32
}

/// `eksekusi_selesai` — callback selesai; lepaskan slot. `1` atau `-EBADF`.
pub fn eksekusi_selesai(handle: u64) -> i32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        bump(Counter::ProdTolakStale);
        -E_BADF
    } else {
        let i = index as usize;
        if AT_ST[i].load(Ordering::Relaxed) != T_EXEC {
            bump(Counter::ProdTolakState);
            -E_BADF
        } else {
            AT_ST[i].store(T_FREE, Ordering::Release);
            AT_BATAL[i].store(0, Ordering::Relaxed);
            AT_NEXT[i].store(AT_FREE.load(Ordering::Relaxed), Ordering::Relaxed);
            AT_FREE.store(i as u32, Ordering::Release);
            bump(Counter::ProdSelesai);
            1
        }
    };
    unlock(_if);
    r
}

/// Jumlah timer QUEUED+EXEC pada kelas (atau semua bila `class>1`).
pub fn jumlah(class: u32) -> u32 {
    pastikan_init();
    let _if = lock();
    let mut n = 0u32;
    let mut i = 0usize;
    while i < MAX_TIMER {
        let st = AT_ST[i].load(Ordering::Relaxed);
        if st != T_FREE
            && (class > K_REAL || AT_CLASS[i].load(Ordering::Relaxed) == class)
        {
            n += 1;
        }
        i += 1;
    }
    unlock(_if);
    n
}

/// Kosongkan arena (uji).
pub fn reset() -> u32 {
    pastikan_init();
    let _if = lock();
    let mut i = 0usize;
    while i < MAX_TIMER {
        AT_GEN[i].store(0, Ordering::Relaxed);
        AT_ST[i].store(T_FREE, Ordering::Relaxed);
        AT_BATAL[i].store(0, Ordering::Relaxed);
        AT_NEXT[i].store(
            if i + 1 < MAX_TIMER { (i + 1) as u32 } else { INVALID },
            Ordering::Relaxed,
        );
        i += 1;
    }
    AT_FREE.store(0, Ordering::Relaxed);
    unlock(_if);
    0
}

// ---------------------------------------------------------------------------
// Pintu C ABI
// ---------------------------------------------------------------------------

/// Laporan matriks. Cermin `KlatSeam135.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosTimerAntreReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_daftar: u32,
    pub prod_batal: u32,
    pub prod_siap: u32,
    pub prod_selesai: u32,
    pub tolak_stale: u32,
    pub tolak_penuh: u32,
    pub jumlah_maks: u32,
    pub status: i32,
}

const _: [(); 64] = [(); core::mem::size_of::<KlatosTimerAntreReport>()];

#[no_mangle]
pub extern "C" fn klatos_timer_daftar(class: u32, deadline: i64) -> u64 {
    daftar(class, deadline)
}
#[no_mangle]
pub extern "C" fn klatos_timer_batal(handle: u64) -> i32 {
    batal(handle)
}
#[no_mangle]
pub extern "C" fn klatos_timer_batal_p(handle: u64) -> u32 {
    batal_p(handle)
}
#[no_mangle]
pub extern "C" fn klatos_timer_status(handle: u64) -> i32 {
    status(handle)
}
#[no_mangle]
pub extern "C" fn klatos_timer_next(class: u32) -> u64 {
    next(class)
}
#[no_mangle]
pub extern "C" fn klatos_timer_next_due(class: u32) -> i64 {
    next_due(class)
}
#[no_mangle]
pub unsafe extern "C" fn klatos_timer_siap(class: u32, now: i64, out: *mut u64, cap: u32) -> i32 {
    siap(class, now, out, cap)
}
#[no_mangle]
pub extern "C" fn klatos_timer_eksekusi_selesai(handle: u64) -> i32 {
    eksekusi_selesai(handle)
}
#[no_mangle]
pub extern "C" fn klatos_timer_jumlah(class: u32) -> u32 {
    jumlah(class)
}
#[no_mangle]
pub extern "C" fn klatos_timer_antre_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}
#[no_mangle]
pub extern "C" fn klatos_timer_antre_reset() -> u32 {
    reset()
}

// ---------------------------------------------------------------------------
// Selftest (RELATIF; tahan arena tak-kosong)
// ---------------------------------------------------------------------------

struct Matrix {
    checks: u32,
    passed: u32,
    neg_checks: u32,
    neg_passed: u32,
    dif_checks: u32,
    dif_beda: u32,
}
impl Matrix {
    const fn new() -> Self {
        Matrix { checks: 0, passed: 0, neg_checks: 0, neg_passed: 0, dif_checks: 0, dif_beda: 0 }
    }
    fn catat(&mut self, l: bool) {
        self.checks += 1;
        if l {
            self.passed += 1;
        }
    }
    fn neg(&mut self, l: bool) {
        self.neg_checks += 1;
        if l {
            self.neg_passed += 1;
        }
        self.catat(l);
    }
    fn dif(&mut self, l: bool) {
        self.dif_checks += 1;
        if !l {
            self.dif_beda += 1;
        }
        self.catat(l);
    }
}

/// Model independen untuk diferensial (URUT terdahulu, tie-break seq).
struct Model {
    st: [u32; 32],
    class: [u32; 32],
    dl: [i64; 32],
    seq: [u64; 32],
    bat: [u32; 32],
    gen: [u32; 32],
    nseq: u64,
}
impl Model {
    fn new() -> Self {
        Model { st: [0; 32], class: [0; 32], dl: [0; 32], seq: [0; 32], bat: [0; 32], gen: [0; 32], nseq: 1 }
    }
    fn alloc(&mut self, class: u32, dl: i64) -> usize {
        let mut i = 0usize;
        while i < 32 {
            if self.st[i] == T_FREE {
                self.gen[i] = self.gen[i].wrapping_add(1);
                self.st[i] = T_QUEUED;
                self.class[i] = class;
                self.dl[i] = dl;
                self.seq[i] = self.nseq;
                self.nseq += 1;
                self.bat[i] = 0;
                return i;
            }
            i += 1;
        }
        32
    }
    fn batal(&mut self, i: usize) -> i32 {
        if i >= 32 || self.st[i] == T_FREE {
            return -E_BADF;
        }
        if self.st[i] == T_QUEUED {
            self.st[i] = T_FREE;
            return 1;
        }
        if self.bat[i] == 0 {
            self.bat[i] = 1;
            return 2;
        }
        -E_BADF
    }
    fn selesai(&mut self, i: usize) -> i32 {
        if i >= 32 || self.st[i] != T_EXEC {
            return -E_BADF;
        }
        self.st[i] = T_FREE;
        self.bat[i] = 0;
        1
    }
    fn next(&self, class: u32) -> usize {
        let mut best = 32usize;
        let mut bd = TIDAK_ADA;
        let mut bs = u64::MAX;
        for i in 0..32 {
            if self.st[i] == T_QUEUED && self.class[i] == class {
                if self.dl[i] < bd || (self.dl[i] == bd && self.seq[i] < bs) {
                    bd = self.dl[i];
                    bs = self.seq[i];
                    best = i;
                }
            }
        }
        best
    }
    unsafe fn siap(&mut self, class: u32, now: i64, out: *mut u64, cap: u32) -> i32 {
        let mut n = 0u32;
        loop {
            if n >= cap {
                break;
            }
            let mut best = 32usize;
            let mut bd = TIDAK_ADA;
            let mut bs = u64::MAX;
            for i in 0..32 {
                if self.st[i] == T_QUEUED && self.class[i] == class && self.dl[i] < now {
                    if self.dl[i] < bd || (self.dl[i] == bd && self.seq[i] < bs) {
                        bd = self.dl[i];
                        bs = self.seq[i];
                        best = i;
                    }
                }
            }
            if best == 32 {
                break;
            }
            self.st[best] = T_EXEC;
            core::ptr::write(out.add(n as usize), best as u64);
            n += 1;
        }
        n as i32
    }
}

fn run_matrix() -> KlatosTimerAntreReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_5449_4D52);
    m.catat(MAX_TIMER == 1024);
    m.catat(K_MONO == 0 && K_REAL == 1);
    m.catat(T_FREE == 0 && T_QUEUED == 1 && T_EXEC == 2);
    m.catat(E_BADF == 9 && E_INVAL == 22);
    m.catat(TIDAK_ADA == i64::MAX);

    let jumlah_awal = jumlah(K_SEMUA);

    // Semua deadline uji digeser jauh (~11,5 hari) supaya `TimerQueue::fire()`
    // nyata — yang berbagi arena/lock ini dan dipanggil `TimeManagement` dari
    // IRQ — TIDAK memakan timer uji di tengah matriks (kalau termakan, slot
    // akan tersangkut T_EXEC karena `m_slots`-nya kosong).
    const JAUH: i64 = 1_000_000_000_000_000;

    // G2: daftar/next/next_due/siap/eksekusi
    let a = daftar_inti(K_UJI, JAUH + 100);
    let b = daftar_inti(K_UJI, JAUH + 300);
    let c = daftar_inti(K_UJI, JAUH + 200);
    m.catat(a != 0 && b != 0 && c != 0);
    m.catat(next(K_UJI) == a);
    m.catat(next_due(K_UJI) == JAUH + 100);
    let mut out = [0u64; 8];
    let n = unsafe { siap(K_UJI, JAUH + 150, out.as_mut_ptr(), 8) };
    m.catat(n == 1 && out[0] == a);
    m.catat(next(K_UJI) == c);
    m.catat(status(a) == T_EXEC as i32);
    m.catat(eksekusi_selesai(a) == 1);
    m.catat(status(a) == -E_BADF);
    let n2 = unsafe { siap(K_UJI, JAUH + 250, out.as_mut_ptr(), 8) };
    m.catat(n2 == 1 && out[0] == c);
    m.catat(eksekusi_selesai(c) == 1);
    // Bersihkan sisa QUEUED (`b`) DULU supaya uji FIFO berikutnya terisolasi.
    m.catat(batal(b) == 1);
    // equal deadlines → FIFO (seq)
    let e1 = daftar_inti(K_UJI, JAUH + 500);
    let e2 = daftar_inti(K_UJI, JAUH + 500);
    m.catat(e1 != 0 && e2 != 0);
    m.catat(next(K_UJI) == e1);
    let n3 = unsafe { siap(K_UJI, JAUH + 501, out.as_mut_ptr(), 8) };
    m.catat(n3 == 2 && out[0] == e1 && out[1] == e2);
    m.catat(eksekusi_selesai(e1) == 1 && eksekusi_selesai(e2) == 1);

    // G3: batal + batal exec
    let d = daftar_inti(K_UJI, JAUH + 1000);
    m.catat(batal(d) == 1);
    m.catat(status(d) == -E_BADF);
    let f = daftar_inti(K_UJI, JAUH + 1000);
    let mut o2 = [0u64; 8];
    let nn = unsafe { siap(K_UJI, JAUH + 2000, o2.as_mut_ptr(), 8) };
    m.catat(nn == 1 && o2[0] == f);
    m.catat(batal(f) == 2);
    m.catat(batal_p(f) == 1);
    m.catat(eksekusi_selesai(f) == 1);

    // G4: negatif
    m.neg(status(INVALID as u64) == -E_BADF);
    m.neg(batal(0xDEAD_BEEFu64) == -E_BADF);
    m.neg(batal(d) == -E_BADF); // ganda
    let g = daftar_inti(K_UJI, JAUH + 10);
    m.catat(batal(g) == 1);
    m.neg(batal(g) == -E_BADF); // ganda
    m.neg(eksekusi_selesai(g) == -E_BADF);
    m.neg(daftar(99, 1) == 0); // kelas tidak sah
    m.catat(jumlah(K_SEMUA) == jumlah_awal);

    // G5: slot reuse + anti-ABA (robust)
    let r1 = daftar_inti(K_UJI, JAUH + 1);
    let (r1i, r1g) = (r1 as u32, (r1 >> 32) as u32);
    m.catat(batal(r1) == 1);
    let r2 = daftar_inti(K_UJI, JAUH + 2);
    let (r2i, r2g) = (r2 as u32, (r2 >> 32) as u32);
    m.catat(hidup(r2i, r2g));
    m.neg(!hidup(r1i, r1g));
    m.catat(r2i != r1i || r2g != r1g);
    m.catat(batal(r2) == 1);

    // G6: dif  40k kasus
    let mut model = Model::new();
    let mut hs = [0u64; 32];
    let mut sl = [0usize; 32];
    let mut ns = 0usize;
    let mut probe: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut t = 0u32;
    while t < 40_000 {
        probe = probe
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let op = ((probe >> 59) % 6) as u32;
        let idx = ((probe >> 11) % 32) as usize;
        match op {
            0 => {
                if ns < 32 {
                    let class = K_UJI;
                    let d = JAUH + ((probe >> 20) % 500) as i64;
                    let mi = model.alloc(class, d);
                    let rh = daftar_inti(class, d);
                    m.dif((mi < 32) == (rh != 0));
                    if rh != 0 && mi < 32 && ns < 32 {
                        hs[ns] = rh;
                        sl[ns] = mi;
                        ns += 1;
                    } else if rh == 0 && mi < 32 {
                        // arena menolak: kembalikan slot model agar tetap sinkron
                        model.st[mi] = T_FREE;
                    } else if rh != 0 && mi >= 32 {
                        // model penuh tapi arena punya ruang: buang agar sinkron
                        let _ = batal(rh);
                    }
                }
            }
            1 => {
                if ns > 0 {
                    let s = idx % ns;
                    let want = if model.st[sl[s]] == T_FREE { -E_BADF } else { model.batal(sl[s]) };
                    let got = batal(hs[s]);
                    m.dif(got == want);
                    if got == 1 {
                        hs[s] = hs[ns - 1];
                        sl[s] = sl[ns - 1];
                        ns -= 1;
                    }
                }
            }
            2 => {
                let class = K_UJI;
                let want = model.next(class);
                let want_h = if want == 32 { 0 } else { 1 };
                let got = next(class);
                m.dif((got != 0) == (want_h != 0));
            }
            3 => {
                let class = K_UJI;
                let now = JAUH + ((probe >> 24) % 600) as i64;
                let mut mo = [0u64; 8];
                let mut ro = [0u64; 8];
                let mc = unsafe { model.siap(class, now, mo.as_mut_ptr(), 8) };
                let rc = unsafe { siap(class, now, ro.as_mut_ptr(), 8) };
                m.dif(mc == rc);
                if mc > 0 {
                    let mut pos = usize::MAX;
                    let mut p = 0usize;
                    while p < ns {
                        if sl[p] == mo[0] as usize {
                            pos = p;
                            break;
                        }
                        p += 1;
                    }
                    m.dif(pos != usize::MAX && ro[0] == hs[pos]);
                }
            }
            4 => {
                if ns > 0 {
                    let s = idx % ns;
                    let want = if model.st[sl[s]] == T_EXEC { model.selesai(sl[s]) } else { -E_BADF };
                    let got = eksekusi_selesai(hs[s]);
                    m.dif(got == want);
                    if got == 1 {
                        hs[s] = hs[ns - 1];
                        sl[s] = sl[ns - 1];
                        ns -= 1;
                    }
                }
            }
            _ => {
                // status: model vs arena (melalui handle aktif)
                if ns > 0 {
                    let s = idx % ns;
                    let want = model.st[sl[s]] as i32;
                    let got = status(hs[s]);
                    m.dif(got == want);
                }
            }
        }
        t += 1;
    }
    let mut k = 0usize;
    while k < ns {
        let h = hs[k];
        if status(h) == T_EXEC as i32 {
            let _ = batal(h);
            let _ = eksekusi_selesai(h);
        } else {
            let _ = batal(h);
        }
        k += 1;
    }
    m.catat(jumlah(K_SEMUA) == jumlah_awal);

    let status = if m.checks == m.passed && m.dif_beda == 0 { 0 } else { -1 };
    KlatosTimerAntreReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_daftar: klatos_timer_antre_counter(Counter::ProdDaftar as u32),
        prod_batal: klatos_timer_antre_counter(Counter::ProdBatalQueued as u32)
            + klatos_timer_antre_counter(Counter::ProdBatalExec as u32),
        prod_siap: klatos_timer_antre_counter(Counter::ProdSiap as u32),
        prod_selesai: klatos_timer_antre_counter(Counter::ProdSelesai as u32),
        tolak_stale: klatos_timer_antre_counter(Counter::ProdTolakStale as u32),
        tolak_penuh: klatos_timer_antre_counter(Counter::ProdTolakPenuh as u32),
        jumlah_maks: MAX_TIMER as u32,
        status,
    }
}

/// Matriks otoritas kebijakan timer.
#[no_mangle]
pub extern "C" fn klatos_timer_antre_selftest() -> KlatosTimerAntreReport {
    run_matrix()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5449_4D52);
        assert!(MAX_TIMER == 1024);
        assert!(K_MONO == 0 && K_REAL == 1);
        assert!(T_FREE == 0 && T_QUEUED == 1 && T_EXEC == 2);
        assert!(N_COUNTER == 8);
        assert!(core::mem::size_of::<KlatosTimerAntreReport>() == 64);
        assert!(core::mem::align_of::<KlatosTimerAntreReport>() == 8);
    };
}
