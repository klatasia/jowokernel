// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.58 — **OTORITAS KEADAAN LOGIS WILAYAH (`Region`)**.


use core::sync::atomic::{AtomicU32, Ordering, AtomicU64};

// ---------------------------------------------------------------------------
// Konstanta
// ---------------------------------------------------------------------------

/// Kapasitas tabel wilayah (berbatas).
pub const MAX_WILAYAH: usize = 16384;

/// Slot teratas dicadangkan untuk selftest.
pub const CADANGAN: usize = 64;

/// Batas atas alokasi produksi (eksklusif).
pub const BATAS_WILAYAH: usize = MAX_WILAYAH - CADANGAN;

/// Tag laporan: "KLATWSTA".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5753_5441;

/// Sentinel "handle tidak sah".
pub const TAK_SAH: u32 = 0xFFFF_FFFF;

// Pemilih `which`.
pub const WW_AKSES: u32 = 0;
pub const WW_BENDERA: u32 = 1;
pub const WW_MEMTYPE: u32 = 2;
pub const WW_RIWAYAT: u32 = 3;

// Bit akses — cermin `Region::Access`.
pub const WA_READ: u32 = 1 << 0;
pub const WA_WRITE: u32 = 1 << 1;
pub const WA_EXEC: u32 = 1 << 2;
pub const WA_SEMUA: u32 = 0x7;

// Bendera — cermin `m_shared`/`m_stack`/`m_mmap`/`m_syscall_region`/
// `m_mmapped_from_readable`/`m_mmapped_from_writable`.
pub const WF_SHARED: u32 = 1 << 0;
pub const WF_STACK: u32 = 1 << 1;
pub const WF_MMAP: u32 = 1 << 2;
pub const WF_SYSCALL: u32 = 1 << 3;
pub const WF_MMAP_R: u32 = 1 << 4;
pub const WF_MMAP_W: u32 = 1 << 5;
pub const WF_SEMUA: u32 = 0x3F;

// Riwayat SetOnce — cermin `m_immutable`/`m_initially_loaded_executable_segment`/
// `m_has_been_readable`/`m_has_been_writable`/`m_has_been_executable`.
pub const WH_IMMUTABLE: u32 = 1 << 0;
pub const WH_INIT_EXEC: u32 = 1 << 1;
pub const WH_BEEN_R: u32 = 1 << 2;
pub const WH_BEEN_W: u32 = 1 << 3;
pub const WH_BEEN_X: u32 = 1 << 4;
pub const WH_SEMUA: u32 = 0x1F;

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_wilayah_daftar` dari produksi.
    ProdDaftar = 0,
    /// `klatos_wilayah_lepas` dari produksi.
    ProdLepas = 1,
    /// `klatos_wilayah_set_akses` dari produksi.
    ProdSetAkses = 2,
    /// `klatos_wilayah_set_bendera` dari produksi.
    ProdSetBendera = 3,
    /// `klatos_wilayah_set_riwayat` dari produksi.
    ProdSetRiwayat = 4,
    /// `klatos_wilayah_taruh` dari produksi.
    ProdTaruh = 5,
    /// Handle basi / di luar tabel / argumen tak sah.
    ProdHandleSalah = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Tabel — array paralel, atomik
// ---------------------------------------------------------------------------

static W_GEN: [AtomicU32; MAX_WILAYAH] = [const { AtomicU32::new(0) }; MAX_WILAYAH];
static W_AKSES: [AtomicU32; MAX_WILAYAH] = [const { AtomicU32::new(0) }; MAX_WILAYAH];
static W_BENDERA: [AtomicU32; MAX_WILAYAH] = [const { AtomicU32::new(0) }; MAX_WILAYAH];
static W_MEMTYPE: [AtomicU32; MAX_WILAYAH] = [const { AtomicU32::new(0) }; MAX_WILAYAH];
static W_RIWAYAT: [AtomicU32; MAX_WILAYAH] = [const { AtomicU32::new(0) }; MAX_WILAYAH];
static W_HIDUP: [AtomicU32; MAX_WILAYAH] = [const { AtomicU32::new(0) }; MAX_WILAYAH];

/// Kunci HANYA untuk alokasi/pelepasan slot (bukan untuk baca/tulis keadaan).
/// Jalur baca/tulis keadaan lock-free supaya aman dipanggil dari konteks
/// interupsi dan dari bawah kunci penjadwal/pohon wilayah.
static KUNCI: AtomicU32 = AtomicU32::new(0);
/// Status interupsi pemegang KUNCI; hanya pemegang yang menulis.
static KUNCI_FLAGS: AtomicU64 = AtomicU64::new(0);

fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while KUNCI.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    KUNCI_FLAGS.store(flags, Ordering::Relaxed);
}

fn unlock() {
    let flags = KUNCI_FLAGS.load(Ordering::Relaxed);
    KUNCI.store(0, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Jumlah wilayah hidup PRODUKSI (dijaga inkremental).
static HIDUP: AtomicU32 = AtomicU32::new(0);
static PUNCAK: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// Resolusi handle
// ---------------------------------------------------------------------------

#[inline]
fn w_sah(index: u32, gen: u32) -> bool {
    let i = index as usize;
    gen != 0 && i < MAX_WILAYAH && W_HIDUP[i].load(Ordering::Acquire) != 0
        && W_GEN[i].load(Ordering::Relaxed) == gen
}

/// `which` sah untuk baca/tulis.
#[inline]
fn w_which_sah(which: u32) -> bool {
    which <= WW_RIWAYAT
}

// ---------------------------------------------------------------------------
// Inti
// ---------------------------------------------------------------------------

fn w_daftar_impl(akses: u32, memtype: u32, shared: u32, lo: usize, hi: usize) -> u64 {
    lock();
    let mut slot: Option<usize> = None;
    let mut i = lo;
    while i < hi {
        if W_HIDUP[i].load(Ordering::Relaxed) == 0 {
            slot = Some(i);
            break;
        }
        i += 1;
    }
    let hasil = if let Some(i) = slot {
        let g = W_GEN[i].load(Ordering::Relaxed).wrapping_add(1);
        if g == 0 {
            W_HIDUP[i].store(0, Ordering::Release);
            0u64
        } else {
            W_AKSES[i].store(akses & WA_SEMUA, Ordering::Relaxed);
            W_BENDERA[i].store(if shared != 0 { WF_SHARED } else { 0 }, Ordering::Relaxed);
            W_MEMTYPE[i].store(memtype & 0xFF, Ordering::Relaxed);
            W_RIWAYAT[i].store(0, Ordering::Relaxed);
            W_GEN[i].store(g, Ordering::Relaxed);
            W_HIDUP[i].store(1, Ordering::Release);
            ((g as u64) << 32) | (i as u64)
        }
    } else {
        0u64
    };
    unlock();
    if hasil != 0 && ((hasil & 0xFFFF_FFFF) as usize) < BATAS_WILAYAH {
        let h = HIDUP.load(Ordering::Relaxed) + 1;
        HIDUP.store(h, Ordering::Relaxed);
        if h > PUNCAK.load(Ordering::Relaxed) {
            PUNCAK.store(h, Ordering::Relaxed);
        }
    }
    hasil
}

fn w_lepas_impl(index: u32, gen: u32) -> u32 {
    if !w_sah(index, gen) {
        return 0;
    }
    lock();
    if !w_sah(index, gen) {
        unlock();
        return 0;
    }
    let i = index as usize;
    W_HIDUP[i].store(0, Ordering::Release);
    W_AKSES[i].store(0, Ordering::Relaxed);
    W_BENDERA[i].store(0, Ordering::Relaxed);
    W_RIWAYAT[i].store(0, Ordering::Relaxed);
    if i < BATAS_WILAYAH {
        let h = HIDUP.load(Ordering::Relaxed);
        if h > 0 {
            HIDUP.store(h - 1, Ordering::Relaxed);
        }
    }
    unlock();
    1
}

fn w_ambil_impl(index: u32, gen: u32, which: u32) -> u32 {
    if !w_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    match which {
        WW_AKSES => W_AKSES[i].load(Ordering::Relaxed),
        WW_BENDERA => W_BENDERA[i].load(Ordering::Relaxed),
        WW_MEMTYPE => W_MEMTYPE[i].load(Ordering::Relaxed),
        WW_RIWAYAT => W_RIWAYAT[i].load(Ordering::Relaxed),
        _ => TAK_SAH,
    }
}

fn w_taruh_impl(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    if !w_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    match which {
        WW_AKSES => W_AKSES[i].swap(val & WA_SEMUA, Ordering::AcqRel),
        WW_BENDERA => W_BENDERA[i].swap(val & WF_SEMUA, Ordering::AcqRel),
        WW_MEMTYPE => W_MEMTYPE[i].swap(val & 0xFF, Ordering::AcqRel),
        WW_RIWAYAT => W_RIWAYAT[i].swap(val & WH_SEMUA, Ordering::AcqRel),
        _ => TAK_SAH,
    }
}

/// Set/bersihkan satu bit akses; mengembalikan kata akses LAMA.
fn w_set_akses_impl(index: u32, gen: u32, bit: u32, val: u32) -> u32 {
    if !w_sah(index, gen) || bit == 0 || (bit & !WA_SEMUA) != 0 {
        return TAK_SAH;
    }
    let i = index as usize;
    if val != 0 {
        W_AKSES[i].fetch_or(bit, Ordering::AcqRel)
    } else {
        W_AKSES[i].fetch_and(!bit, Ordering::AcqRel)
    }
}

/// Set/bersihkan satu bit bendera; mengembalikan kata bendera LAMA.
fn w_set_bendera_impl(index: u32, gen: u32, bit: u32, val: u32) -> u32 {
    if !w_sah(index, gen) || bit == 0 || (bit & !WF_SEMUA) != 0 {
        return TAK_SAH;
    }
    let i = index as usize;
    if val != 0 {
        W_BENDERA[i].fetch_or(bit, Ordering::AcqRel)
    } else {
        W_BENDERA[i].fetch_and(!bit, Ordering::AcqRel)
    }
}

/// Set satu bit riwayat (SetOnce — tidak pernah dibersihkan); mengembalikan
/// kata riwayat LAMA.
fn w_set_riwayat_impl(index: u32, gen: u32, bit: u32) -> u32 {
    if !w_sah(index, gen) || bit == 0 || (bit & !WH_SEMUA) != 0 {
        return TAK_SAH;
    }
    let i = index as usize;
    W_RIWAYAT[i].fetch_or(bit, Ordering::AcqRel)
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Daftarkan wilayah. Mengembalikan `(generation << 32) | index`, atau 0 bila
/// tabel penuh / generasi membungkus.
#[no_mangle]
pub extern "C" fn klatos_wilayah_daftar(akses: u32, memtype: u32, shared: u32) -> u64 {
    let h = w_daftar_impl(akses, memtype, shared, 0, BATAS_WILAYAH);
    if h != 0 {
        bump(Counter::ProdDaftar);
    } else {
        bump(Counter::ProdHandleSalah);
    }
    h
}

/// Lepaskan wilayah. 1 bila dihapus, 0 bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_wilayah_lepas(index: u32, gen: u32) -> u32 {
    let r = w_lepas_impl(index, gen);
    if r == 0 {
        bump(Counter::ProdHandleSalah);
    } else {
        bump(Counter::ProdLepas);
    }
    r
}

/// Baca satu bidang wilayah.
///
/// Keabsahan = kanal tersendiri (`w_sah` + `w_which_sah`), bukan
/// `r == TAK_SAH`.
#[no_mangle]
pub extern "C" fn klatos_wilayah_ambil(index: u32, gen: u32, which: u32) -> u32 {
    if !w_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    if !w_which_sah(which) {
        return TAK_SAH;
    }
    w_ambil_impl(index, gen, which)
}

/// Tulis satu bidang wilayah; mengembalikan nilai LAMA.
#[no_mangle]
pub extern "C" fn klatos_wilayah_taruh(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    if !w_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    if !w_which_sah(which) {
        return TAK_SAH;
    }
    let r = w_taruh_impl(index, gen, which, val);
    if r == TAK_SAH {
        bump(Counter::ProdHandleSalah);
    } else {
        bump(Counter::ProdTaruh);
    }
    r
}

/// Set/bersihkan satu bit akses; mengembalikan kata akses LAMA.
#[no_mangle]
pub extern "C" fn klatos_wilayah_set_akses(index: u32, gen: u32, bit: u32, val: u32) -> u32 {
    if !w_sah(index, gen) || bit == 0 || (bit & !WA_SEMUA) != 0 {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    let r = w_set_akses_impl(index, gen, bit, val);
    if r == TAK_SAH {
        bump(Counter::ProdHandleSalah);
    } else {
        bump(Counter::ProdSetAkses);
    }
    r
}

/// Set/bersihkan satu bit bendera; mengembalikan kata bendera LAMA.
#[no_mangle]
pub extern "C" fn klatos_wilayah_set_bendera(index: u32, gen: u32, bit: u32, val: u32) -> u32 {
    if !w_sah(index, gen) || bit == 0 || (bit & !WF_SEMUA) != 0 {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    let r = w_set_bendera_impl(index, gen, bit, val);
    if r == TAK_SAH {
        bump(Counter::ProdHandleSalah);
    } else {
        bump(Counter::ProdSetBendera);
    }
    r
}

/// Set satu bit riwayat (SetOnce); mengembalikan kata riwayat LAMA.
#[no_mangle]
pub extern "C" fn klatos_wilayah_set_riwayat(index: u32, gen: u32, bit: u32) -> u32 {
    if !w_sah(index, gen) || bit == 0 || (bit & !WH_SEMUA) != 0 {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    let r = w_set_riwayat_impl(index, gen, bit);
    if r == TAK_SAH {
        bump(Counter::ProdHandleSalah);
    } else {
        bump(Counter::ProdSetRiwayat);
    }
    r
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_wilayah_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Wilayah hidup sekarang (rentang produksi).
#[no_mangle]
pub extern "C" fn klatos_wilayah_hidup() -> u32 {
    HIDUP.load(Ordering::Relaxed)
}

/// Puncak wilayah hidup.
#[no_mangle]
pub extern "C" fn klatos_wilayah_puncak() -> u32 {
    PUNCAK.load(Ordering::Relaxed)
}

/// Kapasitas produksi tabel.
#[no_mangle]
pub extern "C" fn klatos_wilayah_kapasitas() -> u32 {
    BATAS_WILAYAH as u32
}

/// Reset HANYA rentang produksi (dipakai audit; bukan produksi).
#[no_mangle]
pub extern "C" fn klatos_wilayah_reset() -> u32 {
    lock();
    let mut i = 0;
    while i < BATAS_WILAYAH {
        W_HIDUP[i].store(0, Ordering::Release);
        W_AKSES[i].store(0, Ordering::Relaxed);
        W_BENDERA[i].store(0, Ordering::Relaxed);
        W_RIWAYAT[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    HIDUP.store(0, Ordering::Release);
    PUNCAK.store(0, Ordering::Release);
    unlock();
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Oracle Naif — model independen semantik C++ historis
// ---------------------------------------------------------------------------

/// Model naif: kata akses/bendera/riwayat sebagai array biasa, tanpa atomik.
/// Implementasinya SENGAJA berbeda bentuk dari produksi.
struct Naif {
    akses: [u32; 64],
    bendera: [u32; 64],
    riwayat: [u32; 64],
    memtype: [u32; 64],
}

impl Naif {
    const fn baru() -> Self {
        Naif {
            akses: [0; 64],
            bendera: [0; 64],
            riwayat: [0; 64],
            memtype: [0; 64],
        }
    }
    /// `set_access_bit(bit, b)`: kembalikan kata akses LAMA.
    fn set_akses(&mut self, i: usize, bit: u32, b: bool) -> u32 {
        let lama = self.akses[i];
        if b {
            self.akses[i] |= bit;
        } else {
            self.akses[i] &= !bit;
        }
        lama
    }
    /// `set_shared/stack/mmap/syscall`: kembalikan kata bendera LAMA.
    fn set_bendera(&mut self, i: usize, bit: u32, b: bool) -> u32 {
        let lama = self.bendera[i];
        if b {
            self.bendera[i] |= bit;
        } else {
            self.bendera[i] &= !bit;
        }
        lama
    }
    /// SetOnce: kembalikan kata riwayat LAMA.
    fn set_riwayat(&mut self, i: usize, bit: u32) -> u32 {
        let lama = self.riwayat[i];
        self.riwayat[i] |= bit;
        lama
    }
    /// `update_shadow_permission_bits(access)`.
    fn update_shadow(&mut self, i: usize, access: u32) {
        if (access & WA_READ) != 0 {
            self.riwayat[i] |= WH_BEEN_R;
        }
        if (access & WA_WRITE) != 0 {
            self.riwayat[i] |= WH_BEEN_W;
        }
        if (access & WA_EXEC) != 0 {
            self.riwayat[i] |= WH_BEEN_X;
        }
    }
}

/// LCG deterministik (numerik saja, tidak ada alokasi).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosKeadaanWilayahReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosKeadaanWilayahReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_daftar: u32,
    pub prod_lepas: u32,
    pub prod_set_akses: u32,
    pub prod_set_bendera: u32,
    pub prod_set_riwayat: u32,
    pub prod_taruh: u32,
    pub prod_handle_salah: u32,
    pub hidup: u32,
    pub puncak: u32,
    pub kapasitas: u32,
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
    fn catat(&mut self, lulus: bool, g: Group) {
        self.checks += 1;
        match g {
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

/// Rentang slot uji (cadangan).
const LO_W: usize = BATAS_WILAYAH;
const HI_W: usize = MAX_WILAYAH;

/// Reset rentang CADANGAN saja, beserta generasinya, supaya matriks dapat
/// dijalankan berulang kali dengan hasil identik. Rentang produksi TIDAK
/// pernah disentuh.
fn reset_cadangan() {
    lock();
    let mut i = LO_W;
    while i < HI_W {
        W_HIDUP[i].store(0, Ordering::Release);
        W_GEN[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    unlock();
}

fn run_matrix() -> KlatosKeadaanWilayahReport {
    let mut m = Matrix::new();

    // --- kelompok 0: konstanta -------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5753_5441, Group::Peta);
    m.catat(MAX_WILAYAH == 16384 && BATAS_WILAYAH == 16320, Group::Peta);
    m.catat(CADANGAN == 64, Group::Peta);
    m.catat(klatos_wilayah_counter(99) == 0, Group::Peta);
    m.catat(WA_SEMUA == 7 && WF_SEMUA == 0x3F && WH_SEMUA == 0x1F, Group::Peta);
    m.catat(klatos_wilayah_kapasitas() == 16320, Group::Peta);
    m.catat(klatos_wilayah_ambil(0, 0, 0) == TAK_SAH, Group::Peta);
    m.catat(N_COUNTER == 7, Group::Peta);
    m.catat(Counter::ProdDaftar as usize == 0 && Counter::ProdHandleSalah as usize == 6, Group::Peta);

    // --- kelompok 1: siklus hidup + akses + bendera (peta) ----------------
    reset_cadangan();
    {
        let h = w_daftar_impl(WA_READ | WA_WRITE, 0, 0, LO_W, HI_W);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        m.catat(h != 0, Group::Peta);
        m.catat(idx >= LO_W as u32 && (idx as usize) < HI_W, Group::Peta);
        m.catat(gen == 1, Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_AKSES) == (WA_READ | WA_WRITE), Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_BENDERA) == 0, Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_MEMTYPE) == 0, Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_RIWAYAT) == 0, Group::Peta);

        // set_readable(true) -> akses |= Read, riwayat |= BEEN_R
        m.catat(w_set_akses_impl(idx, gen, WA_READ, 1) == (WA_READ | WA_WRITE), Group::Peta);
        m.catat(w_set_riwayat_impl(idx, gen, WH_BEEN_R) == 0, Group::Peta);
        // set_writable(false) -> akses &= ~Write
        m.catat(w_set_akses_impl(idx, gen, WA_WRITE, 0) == (WA_READ | WA_WRITE), Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_AKSES) == WA_READ, Group::Peta);
        // set_executable(true)
        w_set_akses_impl(idx, gen, WA_EXEC, 1);
        w_set_riwayat_impl(idx, gen, WH_BEEN_X);
        m.catat(w_ambil_impl(idx, gen, WW_AKSES) == (WA_READ | WA_EXEC), Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_RIWAYAT) == (WH_BEEN_R | WH_BEEN_X), Group::Peta);

        // unsafe_clear_access() -> taruh(WW_AKSES, 0)
        m.catat(w_taruh_impl(idx, gen, WW_AKSES, 0) == (WA_READ | WA_EXEC), Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_AKSES) == 0, Group::Peta);
        // riwayat SetOnce TIDAK dibersihkan oleh clear access.
        m.catat(w_ambil_impl(idx, gen, WW_RIWAYAT) == (WH_BEEN_R | WH_BEEN_X), Group::Peta);

        // bendera: shared/stack/mmap/syscall
        m.catat(w_set_bendera_impl(idx, gen, WF_SHARED, 1) == 0, Group::Peta);
        m.catat(w_set_bendera_impl(idx, gen, WF_STACK, 1) == WF_SHARED, Group::Peta);
        m.catat(w_set_bendera_impl(idx, gen, WF_MMAP, 1) == (WF_SHARED | WF_STACK), Group::Peta);
        m.catat(w_set_bendera_impl(idx, gen, WF_MMAP_R, 1) == (WF_SHARED | WF_STACK | WF_MMAP), Group::Peta);
        m.catat(w_set_bendera_impl(idx, gen, WF_MMAP_W, 1) == (WF_SHARED | WF_STACK | WF_MMAP | WF_MMAP_R), Group::Peta);
        m.catat(w_set_bendera_impl(idx, gen, WF_SYSCALL, 1) == (WF_SHARED | WF_STACK | WF_MMAP | WF_MMAP_R | WF_MMAP_W), Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_BENDERA) == WF_SEMUA, Group::Peta);
        m.catat(w_set_bendera_impl(idx, gen, WF_SHARED, 0) == WF_SEMUA, Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_BENDERA) == (WF_SEMUA & !WF_SHARED), Group::Peta);

        // riwayat: immutable + init_exec
        m.catat(w_set_riwayat_impl(idx, gen, WH_IMMUTABLE) == (WH_BEEN_R | WH_BEEN_X), Group::Peta);
        m.catat(w_set_riwayat_impl(idx, gen, WH_INIT_EXEC) == (WH_BEEN_R | WH_BEEN_X | WH_IMMUTABLE), Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_RIWAYAT) == (WH_BEEN_R | WH_BEEN_X | WH_IMMUTABLE | WH_INIT_EXEC), Group::Peta);

        // memtype
        m.catat(w_taruh_impl(idx, gen, WW_MEMTYPE, 2) == 0, Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_MEMTYPE) == 2, Group::Peta);

        m.catat(w_lepas_impl(idx, gen) == 1, Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_AKSES) == TAK_SAH, Group::Peta);
        // Slot didaur ulang: indeks sama, generasi naik, keadaan bersih.
        let h2 = w_daftar_impl(WA_READ, 0, 1, LO_W, HI_W);
        m.catat((h2 & 0xFFFF_FFFF) as u32 == idx, Group::Peta);
        m.catat((h2 >> 32) as u32 == 2, Group::Peta);
        m.catat(w_ambil_impl(idx, 1, WW_AKSES) == TAK_SAH, Group::Peta);
        m.catat(w_ambil_impl(idx, 2, WW_AKSES) == WA_READ, Group::Peta);
        m.catat(w_ambil_impl(idx, 2, WW_BENDERA) == WF_SHARED, Group::Peta);
        m.catat(w_ambil_impl(idx, 2, WW_RIWAYAT) == 0, Group::Peta);
        m.catat(w_lepas_impl(idx, 2) == 1, Group::Peta);
    }

    // --- kelompok 2: `update_shadow_permission_bits` (peta) ---------------
    {
        let h = w_daftar_impl(0, 0, 0, LO_W, HI_W);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        let a = WA_READ | WA_EXEC;
        m.catat(w_taruh_impl(idx, gen, WW_AKSES, a) == 0, Group::Peta);
        // C++ memanggil tiga kali (satu per bit).
        m.catat(w_set_riwayat_impl(idx, gen, WH_BEEN_R) == 0, Group::Peta);
        m.catat(w_set_riwayat_impl(idx, gen, WH_BEEN_X) == (WH_BEEN_R), Group::Peta);
        m.catat(w_ambil_impl(idx, gen, WW_RIWAYAT) == (WH_BEEN_R | WH_BEEN_X), Group::Peta);
        m.catat((w_ambil_impl(idx, gen, WW_RIWAYAT) & WH_BEEN_W) == 0, Group::Peta);
        m.catat(w_lepas_impl(idx, gen) == 1, Group::Peta);
    }

    // --- kelompok 3: diferensial vs oracle Naif (>= 100.000 kasus) --------
    let mut dif_ukur: u32 = 0;
    let mut dif_beda: u32 = 0;
    {
        reset_cadangan();
        let mut naif = Naif::baru();
        let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);
        let mut wh = [0u32; 32];
        let mut wg = [0u32; 32];
        let mut i = 0;
        while i < 32 {
            let a = (i as u32) & WA_SEMUA;
            let h = w_daftar_impl(a, (i as u32) % 3, (i % 2) as u32, LO_W, HI_W);
            wh[i] = (h & 0xFFFF_FFFF) as u32;
            wg[i] = (h >> 32) as u32;
            naif.akses[i] = a;
            naif.bendera[i] = if i % 2 == 1 { WF_SHARED } else { 0 };
            naif.memtype[i] = (i as u32) % 3;
            // C++ `Region` memanggil `update_shadow_permission_bits(access)`
            // SESUDAH `daftarkan_wilayah`, jadi produksi mencatat riwayat di
            // sini. Selftest meniru urutan itu supaya oracle sebanding.
            if (a & WA_READ) != 0 {
                w_set_riwayat_impl(wh[i], wg[i], WH_BEEN_R);
            }
            if (a & WA_WRITE) != 0 {
                w_set_riwayat_impl(wh[i], wg[i], WH_BEEN_W);
            }
            if (a & WA_EXEC) != 0 {
                w_set_riwayat_impl(wh[i], wg[i], WH_BEEN_X);
            }
            naif.update_shadow(i, a);
            i += 1;
        }

        let mut op = 0;
        while op < 40_000 {
            let r = rng.next();
            let slot = (r as usize) % 32;
            match r % 6 {
                0 => {
                    let bit = match (r >> 8) % 3 {
                        0 => WA_READ,
                        1 => WA_WRITE,
                        _ => WA_EXEC,
                    };
                    let b = ((r >> 12) & 1) != 0;
                    let prod = w_set_akses_impl(wh[slot], wg[slot], bit, b as u32);
                    let o = naif.set_akses(slot, bit, b);
                    dif_ukur += 3;
                    if prod != o {
                        dif_beda += 1;
                    }
                    if w_ambil_impl(wh[slot], wg[slot], WW_AKSES) != naif.akses[slot] {
                        dif_beda += 1;
                    }
                    if (b && (bit == WA_READ || bit == WA_WRITE || bit == WA_EXEC)) && b {
                        // set_readable/set_writable/set_executable juga mencatat riwayat.
                        let rb = match bit {
                            WA_READ => WH_BEEN_R,
                            WA_WRITE => WH_BEEN_W,
                            _ => WH_BEEN_X,
                        };
                        w_set_riwayat_impl(wh[slot], wg[slot], rb);
                        naif.set_riwayat(slot, rb);
                    }
                }
                1 => {
                    let bit = match (r >> 8) % 5 {
                        0 => WF_SHARED,
                        1 => WF_STACK,
                        2 => WF_MMAP,
                        3 => WF_SYSCALL,
                        _ => WF_MMAP_R,
                    };
                    let b = ((r >> 12) & 1) != 0;
                    let prod = w_set_bendera_impl(wh[slot], wg[slot], bit, b as u32);
                    let o = naif.set_bendera(slot, bit, b);
                    dif_ukur += 3;
                    if prod != o {
                        dif_beda += 1;
                    }
                    if w_ambil_impl(wh[slot], wg[slot], WW_BENDERA) != naif.bendera[slot] {
                        dif_beda += 1;
                    }
                }
                2 => {
                    let bit = match (r >> 8) % 5 {
                        0 => WH_IMMUTABLE,
                        1 => WH_INIT_EXEC,
                        2 => WH_BEEN_R,
                        3 => WH_BEEN_W,
                        _ => WH_BEEN_X,
                    };
                    let prod = w_set_riwayat_impl(wh[slot], wg[slot], bit);
                    let o = naif.set_riwayat(slot, bit);
                    dif_ukur += 3;
                    if prod != o {
                        dif_beda += 1;
                    }
                    if w_ambil_impl(wh[slot], wg[slot], WW_RIWAYAT) != naif.riwayat[slot] {
                        dif_beda += 1;
                    }
                }
                3 => {
                    let v = (r >> 8) & WA_SEMUA;
                    let prod = w_taruh_impl(wh[slot], wg[slot], WW_AKSES, v);
                    let lama = naif.akses[slot];
                    naif.akses[slot] = v;
                    dif_ukur += 3;
                    if prod != lama {
                        dif_beda += 1;
                    }
                    if w_ambil_impl(wh[slot], wg[slot], WW_AKSES) != naif.akses[slot] {
                        dif_beda += 1;
                    }
                }
                4 => {
                    let v = (r >> 8) % 3;
                    let prod = w_taruh_impl(wh[slot], wg[slot], WW_MEMTYPE, v);
                    let lama = naif.memtype[slot];
                    naif.memtype[slot] = v;
                    dif_ukur += 3;
                    if prod != lama {
                        dif_beda += 1;
                    }
                    if w_ambil_impl(wh[slot], wg[slot], WW_MEMTYPE) != naif.memtype[slot] {
                        dif_beda += 1;
                    }
                }
                _ => {
                    // taruh bendera penuh
                    let v = (r >> 8) & WF_SEMUA;
                    let prod = w_taruh_impl(wh[slot], wg[slot], WW_BENDERA, v);
                    let lama = naif.bendera[slot];
                    naif.bendera[slot] = v;
                    dif_ukur += 3;
                    if prod != lama {
                        dif_beda += 1;
                    }
                    if w_ambil_impl(wh[slot], wg[slot], WW_BENDERA) != naif.bendera[slot] {
                        dif_beda += 1;
                    }
                }
            }
            op += 1;
        }

        let mut a = 0;
        while a < 32 {
            dif_ukur += 3;
            if w_ambil_impl(wh[a], wg[a], WW_AKSES) != naif.akses[a] {
                dif_beda += 1;
            }
            if w_ambil_impl(wh[a], wg[a], WW_BENDERA) != naif.bendera[a] {
                dif_beda += 1;
            }
            if w_ambil_impl(wh[a], wg[a], WW_RIWAYAT) != naif.riwayat[a] {
                dif_beda += 1;
            }
            a += 1;
        }

        let mut r = 0;
        while r < 32 {
            w_lepas_impl(wh[r], wg[r]);
            r += 1;
        }
    }
    m.catat(dif_ukur >= 100_000, Group::Peta);
    m.catat(dif_beda == 0, Group::Peta);

    // --- kelompok 4: determinisme -----------------------------------------
    {
        let a = klatos_wilayah_hidup();
        let mut ok = true;
        let mut i = 0;
        while i < 10_000 {
            if klatos_wilayah_hidup() != a {
                ok = false;
                break;
            }
            i += 1;
        }
        m.catat(ok, Group::Peta);
    }

    // --- kelompok 5: negatif ----------------------------------------------
    reset_cadangan();
    {
        m.catat(w_ambil_impl(0, 0, 0) == TAK_SAH, Group::Neg);
        m.catat(w_ambil_impl(LO_W as u32, 0, 0) == TAK_SAH, Group::Neg);
        m.catat(w_ambil_impl(MAX_WILAYAH as u32, 1, 0) == TAK_SAH, Group::Neg);
        m.catat(w_ambil_impl(LO_W as u32, 99, 0) == TAK_SAH, Group::Neg);
        m.catat(w_lepas_impl(LO_W as u32, 1) == 0, Group::Neg);
        m.catat(w_set_akses_impl(LO_W as u32, 1, WA_READ, 1) == TAK_SAH, Group::Neg);
        m.catat(w_set_bendera_impl(LO_W as u32, 1, WF_SHARED, 1) == TAK_SAH, Group::Neg);
        m.catat(w_set_riwayat_impl(LO_W as u32, 1, WH_IMMUTABLE) == TAK_SAH, Group::Neg);
        m.catat(w_taruh_impl(LO_W as u32, 1, WW_AKSES, 0) == TAK_SAH, Group::Neg);

        let h = w_daftar_impl(0, 0, 0, LO_W, HI_W);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        // `which` tak dikenal -> nilai TAK_SAH, tetapi handle SAH.
        m.catat(w_ambil_impl(idx, gen, 99) == TAK_SAH, Group::Neg);
        // bit di luar rentang -> ditolak.
        m.catat(w_set_akses_impl(idx, gen, 1 << 7, 1) == TAK_SAH, Group::Neg);
        m.catat(w_set_akses_impl(idx, gen, 0, 1) == TAK_SAH, Group::Neg);
        m.catat(w_set_bendera_impl(idx, gen, 1 << 7, 1) == TAK_SAH, Group::Neg);
        m.catat(w_set_riwayat_impl(idx, gen, 1 << 6) == TAK_SAH, Group::Neg);
        // keadaan tidak berubah oleh penolakan.
        m.catat(w_ambil_impl(idx, gen, WW_AKSES) == 0, Group::Neg);
        m.catat(w_ambil_impl(idx, gen, WW_BENDERA) == 0, Group::Neg);
        m.catat(w_ambil_impl(idx, gen, WW_RIWAYAT) == 0, Group::Neg);
        m.catat(w_lepas_impl(idx, gen) == 1, Group::Neg);
        m.catat(w_ambil_impl(idx, gen, WW_AKSES) == TAK_SAH, Group::Neg);

        // Arena cadangan bersih kembali -> matriks dapat diulang.
        let mut i = LO_W;
        let mut bersih = true;
        while i < HI_W {
            if W_HIDUP[i].load(Ordering::Relaxed) != 0 {
                bersih = false;
                break;
            }
            i += 1;
        }
        m.catat(bersih, Group::Neg);
    }

    // --- kelompok 6: kanal keabsahan vs kanal nilai -----------------------
    //
    // Handle SALAH harus dihitung penolakan; handle SAH dengan nilai sah
    // (termasuk 0 / TAK_SAH-looking) TIDAK boleh.
    {
        reset_cadangan();
        let h = w_daftar_impl(0, 0, 0, LO_W, HI_W);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        m.catat(h != 0, Group::Peta);

        // Nilai sah 0 dibaca dari handle sah: bukan penolakan.
        let c0 = klatos_wilayah_counter(Counter::ProdHandleSalah as u32);
        let v = klatos_wilayah_ambil(idx, gen, WW_AKSES);
        let c1 = klatos_wilayah_counter(Counter::ProdHandleSalah as u32);
        m.catat(v == 0, Group::Peta);
        m.catat(c1 == c0, Group::Peta);

        // Handle salah: tetap dihitung.
        let salah = [
            (idx, gen.wrapping_add(1)),
            (idx, 0),
            (MAX_WILAYAH as u32, gen),
        ];
        for (bi, bg) in salah {
            let c4 = klatos_wilayah_counter(Counter::ProdHandleSalah as u32);
            let r = klatos_wilayah_ambil(bi, bg, WW_AKSES);
            let c5 = klatos_wilayah_counter(Counter::ProdHandleSalah as u32);
            m.catat(r == TAK_SAH, Group::Peta);
            m.catat(c5 == c4 + 1, Group::Peta);
        }
        w_lepas_impl(idx, gen);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosKeadaanWilayahReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_daftar: klatos_wilayah_counter(Counter::ProdDaftar as u32),
        prod_lepas: klatos_wilayah_counter(Counter::ProdLepas as u32),
        prod_set_akses: klatos_wilayah_counter(Counter::ProdSetAkses as u32),
        prod_set_bendera: klatos_wilayah_counter(Counter::ProdSetBendera as u32),
        prod_set_riwayat: klatos_wilayah_counter(Counter::ProdSetRiwayat as u32),
        prod_taruh: klatos_wilayah_counter(Counter::ProdTaruh as u32),
        prod_handle_salah: klatos_wilayah_counter(Counter::ProdHandleSalah as u32),
        hidup: klatos_wilayah_hidup(),
        puncak: klatos_wilayah_puncak(),
        kapasitas: BATAS_WILAYAH as u32,
        status,
    }
}

/// Selftest keadaan logis wilayah.
#[no_mangle]
pub extern "C" fn klatos_wilayah_selftest() -> KlatosKeadaanWilayahReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5753_5441);
        assert!(MAX_WILAYAH == 16384);
        assert!(CADANGAN == 64);
        assert!(BATAS_WILAYAH == 16320);
        assert!(WA_READ == 1 && WA_WRITE == 2 && WA_EXEC == 4 && WA_SEMUA == 7);
        assert!(WF_SEMUA == 0x3F);
        assert!(WH_SEMUA == 0x1F);
        assert!(WW_AKSES == 0 && WW_BENDERA == 1 && WW_MEMTYPE == 2 && WW_RIWAYAT == 3);
        assert!(Counter::ProdDaftar as usize == 0);
        assert!(Counter::ProdHandleSalah as usize == 6);
        assert!(N_COUNTER == 7);
        assert!(core::mem::size_of::<KlatosKeadaanWilayahReport>() == 80);
        assert!(core::mem::align_of::<KlatosKeadaanWilayahReport>() == 8);
    };
}
