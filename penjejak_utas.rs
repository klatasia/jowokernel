// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-20B — **PENJEJAK UTAS: pemilik keadaan `ThreadTracer` (Rust)**.



#![allow(unsafe_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering, AtomicU64};

/// Sentinel "tidak ada penjejak".
pub const TAK_SAH: u64 = u64::MAX;
/// Jumlah slot penjejak.
pub const N_SLOT: usize = 64;
/// Penanda laporan; hanya ada di Rust.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5055_4A54; // "KLATPUJT"

const N_COUNTER: usize = 4;

/// Indeks penghitung audit.
pub const COUNTER_BUAT: u32 = 0;
pub const COUNTER_LEPAS: u32 = 1;
pub const COUNTER_TOLAK: u32 = 2;
pub const COUNTER_SINYAL: u32 = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(c: u32) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

struct Slot {
    gen: AtomicU32,
    hidup: AtomicBool,
    tracer_pid: AtomicU32,
    pending_signals: AtomicU32,
    trace_syscalls: AtomicBool,
}

impl Slot {
    const fn baru() -> Self {
        Self {
            gen: AtomicU32::new(0),
            hidup: AtomicBool::new(false),
            tracer_pid: AtomicU32::new(0),
            pending_signals: AtomicU32::new(0),
            trace_syscalls: AtomicBool::new(false),
        }
    }
}

static SLOTS: [Slot; N_SLOT] = [const { Slot::baru() }; N_SLOT];
static KUNCI: AtomicBool = AtomicBool::new(false);
/// Status interupsi pemegang KUNCI. Hanya pemegang yang menulis dan
/// membacanya sebelum melepas, jadi tidak perlu atomik kuat.
static KUNCI_FLAGS: AtomicU64 = AtomicU64::new(0);

fn kunci() {
    // Matikan interupsi SEBELUM mengambil kunci. Pemegang kunci tidak boleh
    // di-preempt dan handler interupsi tidak boleh masuk lagi ke bagian kritis;
    // keduanya membuat penunggu berputar selamanya (P0-1).
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while KUNCI
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    KUNCI_FLAGS.store(flags, Ordering::Relaxed);
}

fn buka() {
    // Baca status milik sendiri SEBELUM melepas kunci, supaya pengambil
    // berikutnya tidak bisa menimpa nilainya.
    let flags = KUNCI_FLAGS.load(Ordering::Relaxed);
    KUNCI.store(false, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Dekode handle `(gen << 16) | index`. `None` bila bentuk/hidup tak sah.
/// HARUS dipanggil dengan KUNCI dipegang.
fn slot_untuk(h: u64) -> Option<usize> {
    if h == TAK_SAH {
        return None;
    }
    let index = (h & 0xFFFF) as usize;
    let gen = (h >> 16) as u32;
    if index >= N_SLOT || gen == 0 {
        return None;
    }
    if SLOTS[index].hidup.load(Ordering::Relaxed)
        && SLOTS[index].gen.load(Ordering::Relaxed) == gen
    {
        Some(index)
    } else {
        None
    }
}

fn baca_u32(a: &AtomicU32) -> u32 {
    a.load(Ordering::Relaxed)
}

/// Buat penjejak untuk `tracer_pid`; mengembalikan handle atau `TAK_SAH`.
pub fn buat(tracer_pid: u32) -> u64 {
    kunci();
    let mut dipilih: Option<usize> = None;
    let mut i = 0;
    while i < N_SLOT {
        if !SLOTS[i].hidup.load(Ordering::Relaxed) {
            dipilih = Some(i);
            break;
        }
        i += 1;
    }
    let i = match dipilih {
        Some(i) => i,
        None => {
            buka();
            bump(COUNTER_TOLAK);
            return TAK_SAH;
        }
    };
    let gen = SLOTS[i].gen.load(Ordering::Relaxed).wrapping_add(1);
    if gen == 0 {
        // lewati generasi 0 (tak sah)
        SLOTS[i].gen.store(1, Ordering::Relaxed);
    } else {
        SLOTS[i].gen.store(gen, Ordering::Relaxed);
    }
    let gen = SLOTS[i].gen.load(Ordering::Relaxed);
    SLOTS[i].tracer_pid.store(tracer_pid, Ordering::Relaxed);
    SLOTS[i].pending_signals.store(0, Ordering::Relaxed);
    SLOTS[i].trace_syscalls.store(false, Ordering::Relaxed);
    SLOTS[i].hidup.store(true, Ordering::Release);
    buka();
    bump(COUNTER_BUAT);
    ((gen as u64) << 16) | (i as u64)
}

/// Lepas penjejak. `0` atau `-EINVAL`.
pub fn lepas(h: u64) -> i32 {
    kunci();
    match slot_untuk(h) {
        Some(i) => {
            SLOTS[i].hidup.store(false, Ordering::Release);
            SLOTS[i].tracer_pid.store(0, Ordering::Relaxed);
            SLOTS[i].pending_signals.store(0, Ordering::Relaxed);
            SLOTS[i].trace_syscalls.store(false, Ordering::Relaxed);
            buka();
            bump(COUNTER_LEPAS);
            0
        }
        None => {
            buka();
            bump(COUNTER_TOLAK);
            -22 // -EINVAL
        }
    }
}

/// PID penjejak, atau `-1` bila handle tak sah.
pub fn tracer_pid(h: u64) -> i32 {
    kunci();
    let hasil = match slot_untuk(h) {
        Some(i) => baca_u32(&SLOTS[i].tracer_pid) as i32,
        None => -1,
    };
    buka();
    hasil
}

fn sinyal_bit(signal: u32) -> Option<u32> {
    if signal == 0 || signal > 32 {
        None
    } else {
        Some(1u32 << (signal - 1))
    }
}

/// 1/0 apakah sinyal tertunda; `-EINVAL` bila handle/signal tak sah.
pub fn sinyal_ada(h: u64, signal: u32) -> i32 {
    let bit = match sinyal_bit(signal) {
        Some(b) => b,
        None => return -22,
    };
    kunci();
    let hasil = match slot_untuk(h) {
        Some(i) => {
            if baca_u32(&SLOTS[i].pending_signals) & bit != 0 {
                1
            } else {
                0
            }
        }
        None => -22,
    };
    buka();
    hasil
}

/// Set sinyal tertunda. `0` atau `-EINVAL`.
pub fn set_sinyal(h: u64, signal: u32) -> i32 {
    let bit = match sinyal_bit(signal) {
        Some(b) => b,
        None => return -22,
    };
    kunci();
    let hasil = match slot_untuk(h) {
        Some(i) => {
            let v = baca_u32(&SLOTS[i].pending_signals) | bit;
            SLOTS[i].pending_signals.store(v, Ordering::Relaxed);
            0
        }
        None => -22,
    };
    buka();
    if hasil == 0 {
        bump(COUNTER_SINYAL);
    }
    hasil
}

/// Hapus sinyal tertunda. `0` atau `-EINVAL`.
pub fn unset_sinyal(h: u64, signal: u32) -> i32 {
    let bit = match sinyal_bit(signal) {
        Some(b) => b,
        None => return -22,
    };
    kunci();
    let hasil = match slot_untuk(h) {
        Some(i) => {
            let v = baca_u32(&SLOTS[i].pending_signals) & !bit;
            SLOTS[i].pending_signals.store(v, Ordering::Relaxed);
            0
        }
        None => -22,
    };
    buka();
    hasil
}

/// 1/0 apakah syscall tracing aktif; `-EINVAL` bila handle tak sah.
pub fn trace_syscall(h: u64) -> i32 {
    kunci();
    let hasil = match slot_untuk(h) {
        Some(i) => {
            if SLOTS[i].trace_syscalls.load(Ordering::Relaxed) {
                1
            } else {
                0
            }
        }
        None => -22,
    };
    buka();
    hasil
}

/// Set syscall tracing. `0` atau `-EINVAL`.
pub fn set_trace_syscall(h: u64, v: u32) -> i32 {
    kunci();
    let hasil = match slot_untuk(h) {
        Some(i) => {
            SLOTS[i].trace_syscalls.store(v != 0, Ordering::Relaxed);
            0
        }
        None => -22,
    };
    buka();
    hasil
}

/// Jumlah penjejak hidup.
pub fn hidup() -> u32 {
    kunci();
    let mut n = 0;
    let mut i = 0;
    while i < N_SLOT {
        if SLOTS[i].hidup.load(Ordering::Relaxed) {
            n += 1;
        }
        i += 1;
    }
    buka();
    n
}

/// Nilai satu penghitung audit.
pub fn counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Kosongkan tabel + penghitung. TIDAK dipanggil produksi.
pub fn reset() {
    kunci();
    let mut i = 0;
    while i < N_SLOT {
        SLOTS[i].hidup.store(false, Ordering::Relaxed);
        SLOTS[i].gen.store(0, Ordering::Relaxed);
        SLOTS[i].tracer_pid.store(0, Ordering::Relaxed);
        SLOTS[i].pending_signals.store(0, Ordering::Relaxed);
        SLOTS[i].trace_syscalls.store(false, Ordering::Relaxed);
        i += 1;
    }
    buka();
    let mut c = 0;
    while c < N_COUNTER {
        COUNTERS[c].store(0, Ordering::Release);
        c += 1;
    }
}

// ---------------------------------------------------------------------------
// Snapshot uji-sendiri (hermetis)
// ---------------------------------------------------------------------------
//
// run_matrix() mengosongkan tabel untuk menguji kapasitas, lalu MENGISI
// seluruh slot dan hanya melepas sebagian. Tanpa pemulihan, tabel tetap
// penuh setelah uji, sehingga pemakai produksi berikutnya gagal. Snapshot
// ini mengembalikan tabel DAN penghitung ke keadaan sebelum uji, sehingga
// uji boleh dijalankan berulang tanpa merusak keadaan produksi.
//
// Penyangga ini hanya disentuh dari jalur uji-sendiri di bawah KUNCI.

static SIMPAN_HIDUP: [AtomicBool; N_SLOT] = [const { AtomicBool::new(false) }; N_SLOT];
static SIMPAN_GEN: [AtomicU32; N_SLOT] = [const { AtomicU32::new(0) }; N_SLOT];
static SIMPAN_TRACER_PID: [AtomicU32; N_SLOT] = [const { AtomicU32::new(0) }; N_SLOT];
static SIMPAN_PENDING_SIGNALS: [AtomicU32; N_SLOT] = [const { AtomicU32::new(0) }; N_SLOT];
static SIMPAN_TRACE_SYSCALLS: [AtomicBool; N_SLOT] = [const { AtomicBool::new(false) }; N_SLOT];
static SIMPAN_COUNTER: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

/// Rekam keadaan tabel + penghitung sebelum uji merusaknya.
pub fn simpan() {
    kunci();
    let mut i = 0;
    while i < N_SLOT {
        SIMPAN_HIDUP[i].store(SLOTS[i].hidup.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_GEN[i].store(SLOTS[i].gen.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_TRACER_PID[i].store(SLOTS[i].tracer_pid.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_PENDING_SIGNALS[i].store(SLOTS[i].pending_signals.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_TRACE_SYSCALLS[i].store(SLOTS[i].trace_syscalls.load(Ordering::Relaxed), Ordering::Relaxed);
        i += 1;
    }
    buka();
    let mut c = 0;
    while c < N_COUNTER {
        SIMPAN_COUNTER[c].store(COUNTERS[c].load(Ordering::Relaxed), Ordering::Relaxed);
        c += 1;
    }
}

/// Kembalikan tabel + penghitung ke keadaan yang direkam oleh simpan().
pub fn pulihkan() {
    kunci();
    let mut i = 0;
    while i < N_SLOT {
        SLOTS[i].hidup.store(SIMPAN_HIDUP[i].load(Ordering::Relaxed), Ordering::Release);
        SLOTS[i].gen.store(SIMPAN_GEN[i].load(Ordering::Relaxed), Ordering::Relaxed);
        SLOTS[i].tracer_pid.store(SIMPAN_TRACER_PID[i].load(Ordering::Relaxed), Ordering::Relaxed);
        SLOTS[i].pending_signals.store(SIMPAN_PENDING_SIGNALS[i].load(Ordering::Relaxed), Ordering::Relaxed);
        SLOTS[i].trace_syscalls.store(SIMPAN_TRACE_SYSCALLS[i].load(Ordering::Relaxed), Ordering::Relaxed);
        i += 1;
    }
    buka();
    let mut c = 0;
    while c < N_COUNTER {
        COUNTERS[c].store(SIMPAN_COUNTER[c].load(Ordering::Relaxed), Ordering::Relaxed);
        c += 1;
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_buat(tracer_pid: u32) -> u64 {
    buat(tracer_pid)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_lepas(h: u64) -> i32 {
    lepas(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_tracer_pid(h: u64) -> i32 {
    tracer_pid(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_sinyal_ada(h: u64, signal: u32) -> i32 {
    sinyal_ada(h, signal)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_set_sinyal(h: u64, signal: u32) -> i32 {
    set_sinyal(h, signal)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_unset_sinyal(h: u64, signal: u32) -> i32 {
    unset_sinyal(h, signal)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_trace_syscall(h: u64) -> i32 {
    trace_syscall(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_set_trace_syscall(h: u64, v: u32) -> i32 {
    set_trace_syscall(h, v)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_hidup() -> u32 {
    hidup()
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_counter(which: u32) -> u32 {
    counter(which)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_reset() -> u32 {
    reset();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks bukti
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosPenjejakUtasReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub hidup: u32,
    pub prod_buat: u32,
    pub prod_lepas: u32,
    pub prod_tolak: u32,
    pub prod_sinyal: u32,
    pub status: i32,
    pub reserved: u32,
}

pub fn run_matrix() -> KlatosPenjejakUtasReport {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;
    macro_rules! catat {
        ($cond:expr) => {{
            checks += 1;
            if $cond {
                passed += 1;
            }
        }};
    }

    // Hermetis: rekam keadaan produksi sebelum uji mengosongkan tabel.
    simpan();
    reset();

    catat!(REPORT_TAG == 0x4B4C_4154_5055_4A54);
    catat!(N_SLOT == 64 && TAK_SAH == u64::MAX);
    catat!(COUNTER_BUAT == 0 && COUNTER_SINYAL == 3 && N_COUNTER == 4);

    // siklus nyata
    let a = buat(0x2A);
    catat!(a != TAK_SAH);
    catat!(tracer_pid(a) == 0x2A);
    catat!(hidup() == 1);
    catat!(sinyal_ada(a, 9) == 0);
    catat!(set_sinyal(a, 9) == 0);
    catat!(sinyal_ada(a, 9) == 1);
    catat!(unset_sinyal(a, 9) == 0);
    catat!(sinyal_ada(a, 9) == 0);
    catat!(trace_syscall(a) == 0);
    catat!(set_trace_syscall(a, 1) == 0);
    catat!(trace_syscall(a) == 1);
    catat!(set_trace_syscall(a, 0) == 0);
    catat!(lepas(a) == 0);
    catat!(hidup() == 0);
    catat!(tracer_pid(a) == -1); // stale handle
    catat!(lepas(a) == -22);

    // dua penjejak berdampingan (state terpisah)
    let b = buat(1);
    let c = buat(2);
    catat!(b != c && b != TAK_SAH && c != TAK_SAH);
    catat!(set_sinyal(b, 1) == 0);
    catat!(sinyal_ada(b, 1) == 1 && sinyal_ada(c, 1) == 0);
    catat!(tracer_pid(b) == 1 && tracer_pid(c) == 2);
    catat!(lepas(b) == 0 && lepas(c) == 0);

    // input tak sah
    catat!(sinyal_ada(a, 0) == -22);
    catat!(sinyal_ada(a, 33) == -22);
    catat!(set_sinyal(a, 0) == -22);
    catat!(set_trace_syscall(TAK_SAH, 1) == -22);
    catat!(sinyal_ada(TAK_SAH, 9) == -22);

    // kapasitas
    reset();
    let first = buat(0);
    catat!(first != TAK_SAH && (first & 0xFFFF) == 0);
    let mut ok = 1u32;
    let mut i = 1;
    while i < N_SLOT {
        if buat(i as u32) != TAK_SAH {
            ok += 1;
        }
        i += 1;
    }
    catat!(ok == N_SLOT as u32);
    catat!(hidup() == N_SLOT as u32);
    catat!(buat(999) == TAK_SAH);
    // lepas satu lalu pakai lagi (generasi maju -> handle berbeda)
    catat!(lepas(first) == 0);
    let d = buat(7);
    catat!(d != TAK_SAH && (d & 0xFFFF) == 0 && d != first);

    // penghitung bergerak (bebaskan satu slot dulu supaya buat() sukses)
    catat!(counter(COUNTER_BUAT) > 0);
    catat!(counter(COUNTER_TOLAK) > 0);
    catat!(lepas(d) == 0);
    let s = counter(COUNTER_BUAT);
    let _ = buat(1);
    catat!(counter(COUNTER_BUAT) == s + 1);

    let status = if checks == passed { 0 } else { -1 };
    let laporan = KlatosPenjejakUtasReport {
        tag: REPORT_TAG,
        checks,
        passed,
        hidup: hidup(),
        prod_buat: counter(COUNTER_BUAT),
        prod_lepas: counter(COUNTER_LEPAS),
        prod_tolak: counter(COUNTER_TOLAK),
        prod_sinyal: counter(COUNTER_SINYAL),
        status,
        reserved: 0,
    };
    // Hermetis: kembalikan tabel + penghitung ke keadaan sebelum uji.
    pulihkan();
    laporan
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_penjejak_utas_selftest() -> KlatosPenjejakUtasReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5055_4A54);
        assert!(N_SLOT == 64);
        assert!(N_COUNTER == 4);
        assert!(COUNTER_BUAT == 0 && COUNTER_SINYAL == 3);
        assert!(core::mem::size_of::<KlatosPenjejakUtasReport>() == 48);
        assert!(core::mem::align_of::<KlatosPenjejakUtasReport>() == 8);
    };
}
