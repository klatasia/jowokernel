// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-19D — **KEADAAN RUANG: pemilik keadaan `AddressSpace` (Rust)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering, AtomicU64};

/// Sentinel "tidak ada ruang".
pub const TAK_SAH: u64 = u64::MAX;
/// Jumlah slot ruang. Batas atas jumlah `AddressSpace` hidup (satu per proses).
pub const N_SLOT: usize = 4096;
/// Penanda laporan; hanya ada di Rust.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5255_414E; // "KLATRUAN"

/// Indeks bendera: "ruang ini menegakkan region syscall".
pub const BENDERA_SYSCALL: u32 = 0;
/// Jumlah bendera yang dikenal.
pub const N_BENDERA: u32 = 1;

const N_COUNTER: usize = 4;

/// Indeks penghitung audit.
pub const COUNTER_BUAT: u32 = 0;
pub const COUNTER_LEPAS: u32 = 1;
pub const COUNTER_TOLAK: u32 = 2;
pub const COUNTER_SET: u32 = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(c: u32) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

struct Slot {
    gen: AtomicU32,
    hidup: AtomicBool,
    bendera: AtomicU32,
}

impl Slot {
    const fn baru() -> Self {
        Self {
            gen: AtomicU32::new(0),
            hidup: AtomicBool::new(false),
            bendera: AtomicU32::new(0),
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

fn bit_bendera(which: u32) -> Option<u32> {
    if which < N_BENDERA {
        Some(1u32 << which)
    } else {
        None
    }
}

/// Buat slot ruang; mengembalikan handle atau `TAK_SAH`.
pub fn buat() -> u64 {
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
        SLOTS[i].gen.store(1, Ordering::Relaxed);
    } else {
        SLOTS[i].gen.store(gen, Ordering::Relaxed);
    }
    let gen = SLOTS[i].gen.load(Ordering::Relaxed);
    SLOTS[i].bendera.store(0, Ordering::Relaxed);
    SLOTS[i].hidup.store(true, Ordering::Release);
    buka();
    bump(COUNTER_BUAT);
    ((gen as u64) << 16) | (i as u64)
}

/// Lepas slot ruang. `0` atau `-EINVAL`.
pub fn lepas(h: u64) -> i32 {
    kunci();
    match slot_untuk(h) {
        Some(i) => {
            SLOTS[i].hidup.store(false, Ordering::Release);
            SLOTS[i].bendera.store(0, Ordering::Relaxed);
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

/// Ambil satu bendera (0/1); `0xFFFF_FFFF` bila handle/bendera tak sah.
pub fn ambil_bendera(h: u64, which: u32) -> u32 {
    let bit = match bit_bendera(which) {
        Some(b) => b,
        None => return 0xFFFF_FFFF,
    };
    kunci();
    let hasil = match slot_untuk(h) {
        Some(i) => {
            if SLOTS[i].bendera.load(Ordering::Relaxed) & bit != 0 {
                1
            } else {
                0
            }
        }
        None => 0xFFFF_FFFF,
    };
    buka();
    hasil
}

/// Set satu bendera ke `val != 0`. `0` atau `-EINVAL`.
pub fn set_bendera(h: u64, which: u32, val: u32) -> i32 {
    let bit = match bit_bendera(which) {
        Some(b) => b,
        None => return -22,
    };
    kunci();
    let hasil = match slot_untuk(h) {
        Some(i) => {
            let cur = SLOTS[i].bendera.load(Ordering::Relaxed);
            let baru = if val != 0 { cur | bit } else { cur & !bit };
            SLOTS[i].bendera.store(baru, Ordering::Relaxed);
            0
        }
        None => -22,
    };
    buka();
    if hasil == 0 {
        bump(COUNTER_SET);
    }
    hasil
}

/// Jumlah slot ruang hidup.
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
        SLOTS[i].bendera.store(0, Ordering::Relaxed);
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
// seluruh 4096 slot dan hanya melepas satu. Tanpa pemulihan, tabel tetap
// penuh setelah uji, sehingga setiap AddressSpace::try_create() berikutnya
// gagal ENOMEM - itulah yang membuat WorkQueue::initialize() mem-panic
// ketika matriks integrasi P148 dijalankan pada boot. Snapshot ini
// mengembalikan tabel DAN penghitung ke keadaan sebelum uji, sehingga uji
// boleh dijalankan berulang tanpa merusak keadaan produksi.
//
// Penyangga ini hanya disentuh dari jalur uji-sendiri di bawah KUNCI.

static SIMPAN_HIDUP: [AtomicBool; N_SLOT] = [const { AtomicBool::new(false) }; N_SLOT];
static SIMPAN_GEN: [AtomicU32; N_SLOT] = [const { AtomicU32::new(0) }; N_SLOT];
static SIMPAN_BENDERA: [AtomicU32; N_SLOT] = [const { AtomicU32::new(0) }; N_SLOT];
static SIMPAN_COUNTER: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

/// Rekam keadaan tabel + penghitung sebelum uji merusaknya.
pub fn simpan() {
    kunci();
    let mut i = 0;
    while i < N_SLOT {
        SIMPAN_HIDUP[i].store(SLOTS[i].hidup.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_GEN[i].store(SLOTS[i].gen.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_BENDERA[i].store(SLOTS[i].bendera.load(Ordering::Relaxed), Ordering::Relaxed);
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
        SLOTS[i].gen.store(SIMPAN_GEN[i].load(Ordering::Relaxed), Ordering::Relaxed);
        SLOTS[i].bendera.store(SIMPAN_BENDERA[i].load(Ordering::Relaxed), Ordering::Relaxed);
        SLOTS[i]
            .hidup
            .store(SIMPAN_HIDUP[i].load(Ordering::Relaxed), Ordering::Release);
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
pub extern "C" fn klatos_ruang_buat() -> u64 {
    buat()
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ruang_lepas(h: u64) -> i32 {
    lepas(h)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ruang_ambil_bendera(h: u64, which: u32) -> u32 {
    ambil_bendera(h, which)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ruang_set_bendera(h: u64, which: u32, val: u32) -> i32 {
    set_bendera(h, which, val)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ruang_hidup() -> u32 {
    hidup()
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ruang_counter(which: u32) -> u32 {
    counter(which)
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ruang_reset() -> u32 {
    reset();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks bukti
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosRuangStateReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub hidup: u32,
    pub prod_buat: u32,
    pub prod_lepas: u32,
    pub prod_tolak: u32,
    pub prod_set: u32,
    pub status: i32,
    pub reserved: u32,
}

pub fn run_matrix() -> KlatosRuangStateReport {
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

    catat!(REPORT_TAG == 0x4B4C_4154_5255_414E);
    catat!(N_SLOT == 4096 && TAK_SAH == u64::MAX);
    catat!(BENDERA_SYSCALL == 0 && N_BENDERA == 1);
    catat!(COUNTER_BUAT == 0 && COUNTER_SET == 3 && N_COUNTER == 4);

    // siklus nyata: default tidak menegakkan, lalu di-set, lalu dilepas
    let a = buat();
    catat!(a != TAK_SAH);
    catat!(hidup() == 1);
    catat!(ambil_bendera(a, BENDERA_SYSCALL) == 0);
    catat!(set_bendera(a, BENDERA_SYSCALL, 1) == 0);
    catat!(ambil_bendera(a, BENDERA_SYSCALL) == 1);
    // idempoten: set lagi tetap 1
    catat!(set_bendera(a, BENDERA_SYSCALL, 1) == 0);
    catat!(ambil_bendera(a, BENDERA_SYSCALL) == 1);
    catat!(lepas(a) == 0);
    catat!(hidup() == 0);
    catat!(ambil_bendera(a, BENDERA_SYSCALL) == 0xFFFF_FFFF); // stale
    catat!(lepas(a) == -22);

    // dua ruang berdampingan: state terpisah
    let b = buat();
    let c = buat();
    catat!(b != c && b != TAK_SAH && c != TAK_SAH);
    catat!(set_bendera(b, BENDERA_SYSCALL, 1) == 0);
    catat!(ambil_bendera(b, BENDERA_SYSCALL) == 1);
    catat!(ambil_bendera(c, BENDERA_SYSCALL) == 0);
    // clear
    catat!(set_bendera(b, BENDERA_SYSCALL, 0) == 0);
    catat!(ambil_bendera(b, BENDERA_SYSCALL) == 0);
    catat!(lepas(b) == 0 && lepas(c) == 0);

    // input tak sah
    catat!(ambil_bendera(TAK_SAH, BENDERA_SYSCALL) == 0xFFFF_FFFF);
    catat!(set_bendera(TAK_SAH, BENDERA_SYSCALL, 1) == -22);
    catat!(ambil_bendera(0, BENDERA_SYSCALL) == 0xFFFF_FFFF);
    catat!(set_bendera(0, BENDERA_SYSCALL, 1) == -22);
    // bendera tak dikenal
    let d = buat();
    catat!(d != TAK_SAH);
    catat!(ambil_bendera(d, 99) == 0xFFFF_FFFF);
    catat!(set_bendera(d, 99, 1) == -22);
    catat!(lepas(d) == 0);

    // kapasitas + generasi
    reset();
    let first = buat();
    catat!(first != TAK_SAH && (first & 0xFFFF) == 0);
    let mut ok = 1u32;
    let mut i = 1;
    while i < N_SLOT {
        if buat() != TAK_SAH {
            ok += 1;
        }
        i += 1;
    }
    catat!(ok == N_SLOT as u32);
    catat!(hidup() == N_SLOT as u32);
    catat!(buat() == TAK_SAH);
    catat!(lepas(first) == 0);
    let e = buat();
    catat!(e != TAK_SAH && (e & 0xFFFF) == 0 && e != first);
    catat!(set_bendera(e, BENDERA_SYSCALL, 1) == 0);

    // penghitung bergerak (setelah `reset()` di atas)
    catat!(counter(COUNTER_BUAT) > 0);
    catat!(counter(COUNTER_TOLAK) > 0);
    catat!(counter(COUNTER_SET) > 0);
    catat!(lepas(e) == 0);
    let s = counter(COUNTER_LEPAS);
    let _ = buat();
    catat!(counter(COUNTER_LEPAS) == s); // buat tidak menambah lepas
    catat!(counter(COUNTER_LEPAS) >= 1);

    let status = if checks == passed { 0 } else { -1 };
    let laporan = KlatosRuangStateReport {
        tag: REPORT_TAG,
        checks,
        passed,
        hidup: hidup(),
        prod_buat: counter(COUNTER_BUAT),
        prod_lepas: counter(COUNTER_LEPAS),
        prod_tolak: counter(COUNTER_TOLAK),
        prod_set: counter(COUNTER_SET),
        status,
        reserved: 0,
    };
    // Hermetis: kembalikan tabel + penghitung ke keadaan sebelum uji.
    pulihkan();
    laporan
}

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ruang_selftest() -> KlatosRuangStateReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5255_414E);
        assert!(N_SLOT == 4096);
        assert!(N_COUNTER == 4);
        assert!(BENDERA_SYSCALL == 0 && N_BENDERA == 1);
        assert!(core::mem::size_of::<KlatosRuangStateReport>() == 48);
        assert!(core::mem::align_of::<KlatosRuangStateReport>() == 8);
    };
}
