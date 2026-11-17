// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-15B — **IKATAN EFEK: capability KSM → pemilik byte Rust**.

#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, Ordering};

/// `EBADF` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 9).
pub const EBADF: i32 = 9;
/// `EEXIST` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 17).
pub const EEXIST: i32 = 17;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `EMFILE` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 24).
pub const EMFILE: i32 = 24;

/// Jumlah slot ikatan.
pub const N_SLOT: usize = 64;

/// Sentinel "tidak ada penyimpan".
pub const TAK_SAH: u64 = u64::MAX;

/// Kind ABI v2 yang boleh diikat. Cermin `abi2::object_kind`.
pub const KIND_MEMORY: u16 = 3;
/// Kind ABI v2 FileRef. Cermin `abi2::object_kind`.
pub const KIND_FILEREF: u16 = 6;

/// Penanda laporan; hanya ada di Rust.
pub const REPORT_TAG: u64 = 0x4B4C_4154_494B_4154; // "KLATIKAT"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Ikatan dibuat.
    Ikat = 0,
    /// Pencarian ikatan berhasil.
    Selesai = 1,
    /// Ikatan dilepas.
    Lepas = 2,
    /// Ditolak (kind/gen tak sah, duplikat, kapasitas, tak ditemukan).
    Tolak = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Tabel ikatan: N_SLOT slot, tiap slot satu asosiasi identitas→penyimpan
// ---------------------------------------------------------------------------

struct Slot {
    kind: AtomicU16,
    index: AtomicU16,
    gen: AtomicU32,
    store: AtomicU64,
    hidup: AtomicBool,
}

impl Slot {
    const fn baru() -> Self {
        Self {
            kind: AtomicU16::new(0),
            index: AtomicU16::new(0),
            gen: AtomicU32::new(0),
            store: AtomicU64::new(0),
            hidup: AtomicBool::new(false),
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

/// Kind yang boleh diikat.
#[inline]
const fn kind_sah(kind: u16) -> bool {
    kind == KIND_MEMORY || kind == KIND_FILEREF
}

/// Slot hidup yang cocok dengan identitas. HARUS dipanggil dengan KUNCI dipegang.
fn cari(kind: u16, index: u16, gen: u32) -> Option<usize> {
    let mut i = 0;
    while i < N_SLOT {
        if SLOTS[i].hidup.load(Ordering::Relaxed)
            && SLOTS[i].kind.load(Ordering::Relaxed) == kind
            && SLOTS[i].index.load(Ordering::Relaxed) == index
            && SLOTS[i].gen.load(Ordering::Relaxed) == gen
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Operasi keadaan — API aman
// ---------------------------------------------------------------------------

/// Ikat identitas objek KSM `(kind, index, generation)` ke penyimpan `store`.
///
/// Mengembalikan `0` atau `-errno`: `-EINVAL` (kind/gen/store tak sah),
/// `-EEXIST` (sudah terikat), `-EMFILE` (tabel penuh).
pub fn ikat(kind: u16, index: u16, gen: u32, store: u64) -> i32 {
    if !kind_sah(kind) || gen == 0 || store == TAK_SAH {
        bump(Counter::Tolak);
        return -EINVAL;
    }
    kunci();
    if cari(kind, index, gen).is_some() {
        buka();
        bump(Counter::Tolak);
        return -EEXIST;
    }
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
            bump(Counter::Tolak);
            return -EMFILE;
        }
    };
    SLOTS[i].kind.store(kind, Ordering::Relaxed);
    SLOTS[i].index.store(index, Ordering::Relaxed);
    SLOTS[i].gen.store(gen, Ordering::Relaxed);
    SLOTS[i].store.store(store, Ordering::Relaxed);
    SLOTS[i].hidup.store(true, Ordering::Release);
    buka();
    bump(Counter::Ikat);
    0
}

/// Selesaikan identitas objek KSM menjadi penyimpan Rust.
///
/// Mengembalikan handle penyimpan (>0) atau `-EBADF` bila tidak terikat.
pub fn selesaikan(kind: u16, index: u16, gen: u32) -> Result<u64, i32> {
    kunci();
    let hasil = match cari(kind, index, gen) {
        Some(i) => Ok(SLOTS[i].store.load(Ordering::Relaxed)),
        None => Err(-EBADF),
    };
    buka();
    if hasil.is_ok() {
        bump(Counter::Selesai);
    } else {
        bump(Counter::Tolak);
    }
    hasil
}

/// Lepas ikatan. Mengembalikan `0` atau `-EBADF`.
pub fn lepas(kind: u16, index: u16, gen: u32) -> i32 {
    kunci();
    let i = match cari(kind, index, gen) {
        Some(i) => i,
        None => {
            buka();
            bump(Counter::Tolak);
            return -EBADF;
        }
    };
    SLOTS[i].hidup.store(false, Ordering::Release);
    SLOTS[i].store.store(0, Ordering::Relaxed);
    buka();
    bump(Counter::Lepas);
    0
}

/// Jumlah ikatan hidup.
pub fn hidup() -> u32 {
    let mut n = 0;
    kunci();
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
pub fn counter(which: Counter) -> u32 {
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Kosongkan tabel + penghitung. TIDAK dipanggil produksi.
pub fn reset() {
    kunci();
    let mut i = 0;
    while i < N_SLOT {
        SLOTS[i].hidup.store(false, Ordering::Relaxed);
        SLOTS[i].kind.store(0, Ordering::Relaxed);
        SLOTS[i].index.store(0, Ordering::Relaxed);
        SLOTS[i].gen.store(0, Ordering::Relaxed);
        SLOTS[i].store.store(0, Ordering::Relaxed);
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
static SIMPAN_KIND: [AtomicU16; N_SLOT] = [const { AtomicU16::new(0) }; N_SLOT];
static SIMPAN_INDEX: [AtomicU16; N_SLOT] = [const { AtomicU16::new(0) }; N_SLOT];
static SIMPAN_GEN: [AtomicU32; N_SLOT] = [const { AtomicU32::new(0) }; N_SLOT];
static SIMPAN_STORE: [AtomicU64; N_SLOT] = [const { AtomicU64::new(0) }; N_SLOT];
static SIMPAN_COUNTER: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

/// Rekam keadaan tabel + penghitung sebelum uji merusaknya.
pub fn simpan() {
    kunci();
    let mut i = 0;
    while i < N_SLOT {
        SIMPAN_HIDUP[i].store(SLOTS[i].hidup.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_KIND[i].store(SLOTS[i].kind.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_INDEX[i].store(SLOTS[i].index.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_GEN[i].store(SLOTS[i].gen.load(Ordering::Relaxed), Ordering::Relaxed);
        SIMPAN_STORE[i].store(SLOTS[i].store.load(Ordering::Relaxed), Ordering::Relaxed);
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
        SLOTS[i].kind.store(SIMPAN_KIND[i].load(Ordering::Relaxed), Ordering::Relaxed);
        SLOTS[i].index.store(SIMPAN_INDEX[i].load(Ordering::Relaxed), Ordering::Relaxed);
        SLOTS[i].gen.store(SIMPAN_GEN[i].load(Ordering::Relaxed), Ordering::Relaxed);
        SLOTS[i].store.store(SIMPAN_STORE[i].load(Ordering::Relaxed), Ordering::Relaxed);
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

/// Ikat identitas objek KSM ke penyimpan. `0` = sukses.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ikatan_efek_ikat(kind: u16, index: u16, generation: u32, store: u64) -> i32 {
    ikat(kind, index, generation, store)
}

/// Selesaikan identitas objek KSM menjadi penyimpan; `TAK_SAH` bila tak terikat.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ikatan_efek_selesaikan(kind: u16, index: u16, generation: u32) -> u64 {
    match selesaikan(kind, index, generation) {
        Ok(s) => s,
        Err(_) => TAK_SAH,
    }
}

/// Lepas ikatan.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ikatan_efek_lepas(kind: u16, index: u16, generation: u32) -> i32 {
    lepas(kind, index, generation)
}

/// Jumlah ikatan hidup.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ikatan_efek_hidup() -> u32 {
    hidup()
}

/// Nilai satu penghitung audit.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ikatan_efek_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset tabel + penghitung. TIDAK dipanggil produksi.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ikatan_efek_reset() -> u32 {
    reset();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks bukti
// ---------------------------------------------------------------------------

/// Cermin `KlatosIkatanEfekReport`.
#[repr(C)]
pub struct KlatosIkatanEfekReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub hidup: u32,
    pub prod_ikat: u32,
    pub prod_selesai: u32,
    pub prod_lepas: u32,
    pub prod_tolak: u32,
    pub status: i32,
    pub reserved: u32,
}

/// Matriks bukti.
pub fn run_matrix() -> KlatosIkatanEfekReport {
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

    // --- konstanta --------------------------------------------------------
    catat!(REPORT_TAG == 0x4B4C_4154_494B_4154);
    catat!(EBADF == 9 && EEXIST == 17 && EINVAL == 22 && EMFILE == 24);
    catat!(N_SLOT == 64 && TAK_SAH == u64::MAX);
    catat!(KIND_MEMORY == 3 && KIND_FILEREF == 6);

    // --- siklus NYATA -----------------------------------------------------
    {
        catat!(ikat(KIND_MEMORY, 7, 3, 0xABCD) == 0);
        catat!(selesaikan(KIND_MEMORY, 7, 3) == Ok(0xABCD));
        catat!(hidup() == 1);
        // identitas berbeda (generasi) tidak cocok
        catat!(selesaikan(KIND_MEMORY, 7, 4) == Err(-EBADF));
        // kind berbeda tidak cocok
        catat!(selesaikan(KIND_FILEREF, 7, 3) == Err(-EBADF));
        // index berbeda tidak cocok
        catat!(selesaikan(KIND_MEMORY, 8, 3) == Err(-EBADF));
        // duplikat
        catat!(ikat(KIND_MEMORY, 7, 3, 0x1111) == -EEXIST);
        // lepas
        catat!(lepas(KIND_MEMORY, 7, 3) == 0);
        catat!(hidup() == 0);
        catat!(selesaikan(KIND_MEMORY, 7, 3) == Err(-EBADF));
        catat!(lepas(KIND_MEMORY, 7, 3) == -EBADF);
    }

    // --- dua kind berdampingan -------------------------------------------
    {
        catat!(ikat(KIND_MEMORY, 1, 1, 0x20) == 0);
        catat!(ikat(KIND_FILEREF, 1, 1, 0x30) == 0);
        catat!(hidup() == 2);
        catat!(selesaikan(KIND_MEMORY, 1, 1) == Ok(0x20));
        catat!(selesaikan(KIND_FILEREF, 1, 1) == Ok(0x30));
        catat!(lepas(KIND_MEMORY, 1, 1) == 0);
        catat!(selesaikan(KIND_FILEREF, 1, 1) == Ok(0x30));
        catat!(lepas(KIND_FILEREF, 1, 1) == 0);
    }

    // --- input tak sah ----------------------------------------------------
    {
        catat!(ikat(KIND_MEMORY, 0, 0, 0x1) == -EINVAL); // generasi 0
        catat!(ikat(0, 0, 1, 0x1) == -EINVAL); // kind NONE
        catat!(ikat(9, 0, 1, 0x1) == -EINVAL); // kind di luar himpunan
        catat!(ikat(KIND_MEMORY, 0, 1, TAK_SAH) == -EINVAL); // penyimpan sentinel
        catat!(selesaikan(0, 0, 1) == Err(-EBADF));
        catat!(lepas(0, 0, 1) == -EBADF);
    }

    // --- kapasitas: tabel penuh ------------------------------------------
    {
        reset();
        let mut i = 0u16;
        let mut ok = 0u32;
        while (i as usize) < N_SLOT {
            if ikat(KIND_MEMORY, i, 1, 0x100 + i as u64) == 0 {
                ok += 1;
            }
            i += 1;
        }
        catat!(ok == N_SLOT as u32);
        catat!(hidup() == N_SLOT as u32);
        catat!(ikat(KIND_MEMORY, N_SLOT as u16, 1, 0x1) == -EMFILE);
        // setelah lepas satu, slot dapat dipakai lagi
        catat!(lepas(KIND_MEMORY, 0, 1) == 0);
        catat!(ikat(KIND_MEMORY, 999, 1, 0x55) == 0);
        catat!(selesaikan(KIND_MEMORY, 999, 1) == Ok(0x55));
        catat!(lepas(KIND_MEMORY, 999, 1) == 0); // bebaskan slot untuk uji determinisme
    }

    // --- penghitung bergerak ---------------------------------------------
    catat!(counter(Counter::Ikat) > 0);
    catat!(counter(Counter::Selesai) > 0);
    catat!(counter(Counter::Lepas) > 0);
    catat!(counter(Counter::Tolak) > 0);

    let a = counter(Counter::Ikat);
    let _ = ikat(KIND_MEMORY, 0, 1, 0x7);
    catat!(counter(Counter::Ikat) == a + 1);

    let status = if checks == passed { 0 } else { -1 };
    let laporan = KlatosIkatanEfekReport {
        tag: REPORT_TAG,
        checks,
        passed,
        hidup: hidup(),
        prod_ikat: counter(Counter::Ikat),
        prod_selesai: counter(Counter::Selesai),
        prod_lepas: counter(Counter::Lepas),
        prod_tolak: counter(Counter::Tolak),
        status,
        reserved: 0,
    };
    // Hermetis: kembalikan tabel + penghitung ke keadaan sebelum uji.
    pulihkan();
    laporan
}

/// Titik masuk C ABI matriks bukti ikatan efek.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_ikatan_efek_selftest() -> KlatosIkatanEfekReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_494B_4154);
        assert!(EBADF == 9 && EEXIST == 17 && EINVAL == 22 && EMFILE == 24);
        assert!(N_SLOT == 64);
        assert!(KIND_MEMORY == 3 && KIND_FILEREF == 6);
        assert!(Counter::Ikat as usize == 0);
        assert!(Counter::Tolak as usize == 3);
        assert!(N_COUNTER == 4);
        // Laporan: 8 (tag) + 9*4 = 44 -> align 8 -> 48.
        assert!(core::mem::size_of::<KlatosIkatanEfekReport>() == 48);
        assert!(core::mem::align_of::<KlatosIkatanEfekReport>() == 8);
    };
}
