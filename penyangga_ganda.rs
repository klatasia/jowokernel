// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.40 — OTORITAS METADATA PENYANGGA GANDA (Rust).


use core::sync::atomic::{AtomicBool, AtomicU32, Ordering, AtomicU64};

/// Tag laporan: "KLATPGDA".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5047_4441;

/// Jumlah slot handle maksimum (`DoubleBuffer` serentak).
///
/// P1.40 memperkenalkan batas ini; C++ pra-P1.40 tidak punya batas (ia memakai
/// `NonnullOwnPtr`/`new`). Tanpa allocator (§10), tabel tetap adalah satu-satunya
/// bentuk yang sah. Arm A mengamati puncak **80 hidup** (lihat `P140-PROD`);
/// 512 memberi ruang untuk arm G (setiap PTY + setiap FIFO + setiap soket
/// menambah satu). Bila tabel penuh, `buat` mengembalikan `0` dan
/// `DoubleBuffer::try_create` mengembalikan `ENOMEM` — dicatat sebagai
/// F-P140-01.
pub const MAX_PENYANGGA: usize = 512;

/// Jumlah penghitung audit.
pub const N_COUNTER: usize = 8;

const RENTANG_DIFERENSIAL: u32 = 100_000;
const KAPASITAS_DIFERENSIAL: u32 = 64;

/// Cermin `KlatKursor` di `Jowo/Kontrak/KlatKernelBridge.h` (`#[repr(C)]`,
/// 16 byte): paruh (0/1) + offset byte di dalam paruh itu.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KlatKursor {
    pub paruh: u64,
    pub offset: u64,
}

/// Kursor netral (dipakai untuk handle tidak sah).
pub const KURSOR_KOSONG: KlatKursor = KlatKursor {
    paruh: 0,
    offset: 0,
};

// ---------------------------------------------------------------------------
// Oracle historis — transkripsi independen `DoubleBuffer` C++
// ---------------------------------------------------------------------------

/// Model `DoubleBuffer` pra-P1.40, tanpa atomik. Dipakai sebagai oracle
/// diferensial; ia adalah transkripsi KEDUA (independen) dari C++ asli.
pub struct Model {
    pub kapasitas: u32,
    pub paruh_tulis: u32,
    pub ukuran: [u32; 2],
    pub offset_baca: u32,
    pub ruang_tulis: u32,
    pub kosong: u32,
    pub segera: u32,
}

impl Model {
    pub const fn baru(kapasitas: u32) -> Self {
        Model {
            kapasitas,
            paruh_tulis: 0,
            ukuran: [0, 0],
            offset_baca: 0,
            ruang_tulis: kapasitas,
            kosong: 1,
            segera: 0,
        }
    }

    /// `compute_lockfree_metadata()` + `immediately_readable()`.
    fn hitung(&mut self) {
        let pt = self.paruh_tulis as usize;
        let pb = 1 - pt;
        let ukr_t = self.ukuran[pt];
        let ukr_b = self.ukuran[pb];
        let off = self.offset_baca;
        self.kosong = if off >= ukr_b && ukr_t == 0 { 1 } else { 0 };
        self.ruang_tulis = self.kapasitas.saturating_sub(ukr_t);
        self.segera = ukr_b.saturating_sub(off).saturating_add(ukr_t);
    }

    pub fn kosong(&self) -> u32 {
        self.kosong
    }

    pub fn ruang_tulis(&self) -> u64 {
        self.ruang_tulis as u64
    }

    pub fn segera_baca(&self) -> u64 {
        self.segera as u64
    }

    pub fn kursor_tulis(&self) -> KlatKursor {
        let pt = self.paruh_tulis as usize;
        KlatKursor {
            paruh: self.paruh_tulis as u64,
            offset: self.ukuran[pt] as u64,
        }
    }

    pub fn kursor_baca(&self) -> KlatKursor {
        KlatKursor {
            paruh: (1 - self.paruh_tulis) as u64,
            offset: self.offset_baca as u64,
        }
    }

    /// `write()` fase 1: berapa byte yang boleh ditulis.
    pub fn tulis(&self, diminta: u64) -> u64 {
        let r = self.ruang_tulis as u64;
        if diminta < r {
            diminta
        } else {
            r
        }
    }

    /// `write()` fase 2: `m_write_buffer->size += ditulis; compute`.
    pub fn selesai_tulis(&mut self, ditulis: u64) {
        let pt = self.paruh_tulis as usize;
        let d = if ditulis > u32::MAX as u64 {
            u32::MAX
        } else {
            ditulis as u32
        };
        self.ukuran[pt] = self.ukuran[pt].saturating_add(d);
        self.hitung();
    }

    /// `flip()`; mengembalikan apakah `VERIFY(read_index == read_size)` benar.
    pub fn flip(&mut self) -> bool {
        let pb = 1 - self.paruh_tulis;
        let ok = self.offset_baca == self.ukuran[pb as usize];
        self.paruh_tulis = pb;
        self.ukuran[pb as usize] = 0;
        self.offset_baca = 0;
        self.hitung();
        ok
    }

    /// `read_impl()` fase 1; mengembalikan `(nread, flip_terjadi, verify_ok)`.
    pub fn baca(&mut self, diminta: u64) -> (u64, bool, bool) {
        let pt = self.paruh_tulis as usize;
        let mut flip = false;
        let mut verify_ok = true;
        if self.offset_baca >= self.ukuran[1 - pt] && self.ukuran[pt] != 0 {
            verify_ok = self.flip();
            flip = true;
        }
        let pb = 1 - self.paruh_tulis as usize;
        let off = self.offset_baca;
        let ukr = self.ukuran[pb];
        let n = if off >= ukr {
            0
        } else {
            let t = (ukr - off) as u64;
            if diminta < t {
                diminta
            } else {
                t
            }
        };
        (n, flip, verify_ok)
    }

    /// `read_impl()` fase 2: (opsional) `read_index += dibaca; compute`.
    pub fn selesai_baca(&mut self, dibaca: u64, maju: u32) {
        if maju != 0 {
            let d = if dibaca > u32::MAX as u64 {
                u32::MAX
            } else {
                dibaca as u32
            };
            self.offset_baca = self.offset_baca.saturating_add(d);
        }
        self.hitung();
    }
}

/// Oracle historis untuk SATU langkah acak (lihat [`langkah_baru`]).
/// Transkripsi independen; dipakai sebagai pembanding diferensial.
pub fn kebijakan_lama(m: &mut Model, x: u32) -> (u64, bool, bool) {
    let kind = x % 4;
    let arg = ((x >> 8) % 200) as u64;
    if kind <= 1 {
        let n = m.tulis(arg);
        let k = ((x >> 16) as u64) % (n + 1);
        m.selesai_tulis(k);
        (n, false, true)
    } else {
        let (n, f, v) = m.baca(arg);
        if n != 0 {
            let k = ((x >> 16) as u64) % (n + 1);
            let maju = (x >> 20) & 1;
            m.selesai_baca(k, maju);
        }
        (n, f, v)
    }
}

// ---------------------------------------------------------------------------
// Algoritma TUNGGAL — penyangga produksi
// ---------------------------------------------------------------------------

/// Metadata satu penyangga ganda. Satu algoritma dipakai produksi DAN matriks;
/// perbedaannya hanya pada instans (`TABEL` vs instans lokal).
pub struct Penyangga {
    kunci: AtomicBool,
    /// Status interupsi pemegang kunci; hanya pemegang yang menulis.
    kunci_flags: AtomicU64,
    kapasitas: AtomicU32,
    paruh_tulis: AtomicU32,
    ukuran0: AtomicU32,
    ukuran1: AtomicU32,
    offset_baca: AtomicU32,
    ruang_tulis: AtomicU32,
    kosong: AtomicU32,
    segera: AtomicU32,
}

impl Penyangga {
    pub const fn kosong_statis() -> Self {
        Penyangga {
            kunci: AtomicBool::new(false),
            kunci_flags: AtomicU64::new(0),
            kapasitas: AtomicU32::new(0),
            paruh_tulis: AtomicU32::new(0),
            ukuran0: AtomicU32::new(0),
            ukuran1: AtomicU32::new(0),
            offset_baca: AtomicU32::new(0),
            ruang_tulis: AtomicU32::new(0),
            kosong: AtomicU32::new(1),
            segera: AtomicU32::new(0),
        }
    }

    fn ambil_kunci(&self) {
        // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
        let flags = crate::arch::simpan_dan_matikan_interupsi();
        while self
            .kunci
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        self.kunci_flags.store(flags, Ordering::Relaxed);
    }

    fn lepas_kunci(&self) {
        let flags = self.kunci_flags.load(Ordering::Relaxed);
        self.kunci.store(false, Ordering::Release);
        crate::arch::pulihkan_interupsi(flags);
    }

    fn ukuran(&self, i: u32) -> u32 {
        if i == 0 {
            self.ukuran0.load(Ordering::Relaxed)
        } else {
            self.ukuran1.load(Ordering::Relaxed)
        }
    }

    fn set_ukuran(&self, i: u32, v: u32) {
        if i == 0 {
            self.ukuran0.store(v, Ordering::Relaxed);
        } else {
            self.ukuran1.store(v, Ordering::Relaxed);
        }
    }

    /// `compute_lockfree_metadata()` + turunan `segera`. Dipanggil dengan kunci
    /// dipegang.
    fn hitung(&self) {
        let pt = self.paruh_tulis.load(Ordering::Relaxed);
        let pb = 1 - pt;
        let ukr_t = self.ukuran(pt);
        let ukr_b = self.ukuran(pb);
        let off = self.offset_baca.load(Ordering::Relaxed);
        let kap = self.kapasitas.load(Ordering::Relaxed);
        let kosong = off >= ukr_b && ukr_t == 0;
        self.kosong.store(kosong as u32, Ordering::Release);
        self.ruang_tulis
            .store(kap.saturating_sub(ukr_t), Ordering::Release);
        self.segera
            .store(ukr_b.saturating_sub(off).saturating_add(ukr_t), Ordering::Release);
    }

    /// Inisialisasi slot (konstruktor `DoubleBuffer`).
    pub fn init(&self, kapasitas: u32) {
        self.ambil_kunci();
        self.kapasitas.store(kapasitas, Ordering::Relaxed);
        self.paruh_tulis.store(0, Ordering::Relaxed);
        self.ukuran0.store(0, Ordering::Relaxed);
        self.ukuran1.store(0, Ordering::Relaxed);
        self.offset_baca.store(0, Ordering::Relaxed);
        self.hitung();
        self.lepas_kunci();
    }

    pub fn kosong(&self) -> u32 {
        self.kosong.load(Ordering::Acquire)
    }

    pub fn ruang_tulis(&self) -> u64 {
        self.ruang_tulis.load(Ordering::Acquire) as u64
    }

    pub fn segera_baca(&self) -> u64 {
        self.segera.load(Ordering::Acquire) as u64
    }

    pub fn kursor_tulis(&self) -> KlatKursor {
        let pt = self.paruh_tulis.load(Ordering::Acquire);
        KlatKursor {
            paruh: pt as u64,
            offset: self.ukuran(pt) as u64,
        }
    }

    pub fn kursor_baca(&self) -> KlatKursor {
        let pt = self.paruh_tulis.load(Ordering::Acquire);
        KlatKursor {
            paruh: (1 - pt) as u64,
            offset: self.offset_baca.load(Ordering::Acquire) as u64,
        }
    }

    /// `write()` fase 1: berapa byte yang boleh ditulis.
    pub fn tulis(&self, diminta: u64) -> u64 {
        let r = self.ruang_tulis.load(Ordering::Acquire) as u64;
        if diminta < r {
            diminta
        } else {
            r
        }
    }

    /// `write()` fase 2.
    pub fn selesai_tulis(&self, ditulis: u64) {
        let d = if ditulis > u32::MAX as u64 {
            u32::MAX
        } else {
            ditulis as u32
        };
        self.ambil_kunci();
        let pt = self.paruh_tulis.load(Ordering::Relaxed);
        let baru = self.ukuran(pt).saturating_add(d);
        self.set_ukuran(pt, baru);
        self.hitung();
        self.lepas_kunci();
    }

    /// `flip()`; mengembalikan apakah `VERIFY(read_index == read_size)` benar.
    fn flip(&self) -> bool {
        let pt = self.paruh_tulis.load(Ordering::Relaxed);
        let pb = 1 - pt;
        let ok = self.offset_baca.load(Ordering::Relaxed) == self.ukuran(pb);
        self.paruh_tulis.store(pb, Ordering::Relaxed);
        self.set_ukuran(pb, 0);
        self.offset_baca.store(0, Ordering::Relaxed);
        self.hitung();
        ok
    }

    /// `read_impl()` fase 1; mengembalikan `(nread, flip_terjadi, verify_ok)`.
    pub fn baca(&self, diminta: u64) -> (u64, bool, bool) {
        self.ambil_kunci();
        let pt = self.paruh_tulis.load(Ordering::Relaxed);
        let mut flip = false;
        let mut verify_ok = true;
        if self.offset_baca.load(Ordering::Relaxed) >= self.ukuran(1 - pt)
            && self.ukuran(pt) != 0
        {
            verify_ok = self.flip();
            flip = true;
        }
        let pb = 1 - self.paruh_tulis.load(Ordering::Relaxed);
        let off = self.offset_baca.load(Ordering::Relaxed);
        let ukr = self.ukuran(pb);
        let n = if off >= ukr {
            0
        } else {
            let t = (ukr - off) as u64;
            if diminta < t {
                diminta
            } else {
                t
            }
        };
        self.lepas_kunci();
        (n, flip, verify_ok)
    }

    /// `read_impl()` fase 2.
    pub fn selesai_baca(&self, dibaca: u64, maju: u32) {
        let d = if dibaca > u32::MAX as u64 {
            u32::MAX
        } else {
            dibaca as u32
        };
        self.ambil_kunci();
        if maju != 0 {
            let off = self.offset_baca.load(Ordering::Relaxed);
            self.offset_baca.store(off.saturating_add(d), Ordering::Relaxed);
        }
        self.hitung();
        self.lepas_kunci();
    }
}

/// Satu langkah acak pada mekanisme produksi (lihat [`kebijakan_lama`]).
fn langkah_baru(p: &Penyangga, x: u32) -> (u64, bool, bool) {
    let kind = x % 4;
    let arg = ((x >> 8) % 200) as u64;
    if kind <= 1 {
        let n = p.tulis(arg);
        let k = ((x >> 16) as u64) % (n + 1);
        p.selesai_tulis(k);
        (n, false, true)
    } else {
        let (n, f, v) = p.baca(arg);
        if n != 0 {
            let k = ((x >> 16) as u64) % (n + 1);
            let maju = (x >> 20) & 1;
            p.selesai_baca(k, maju);
        }
        (n, f, v)
    }
}

// ---------------------------------------------------------------------------
// Penyimpanan PRODUKSI — tabel handle
// ---------------------------------------------------------------------------

static TABEL: [Penyangga; MAX_PENYANGGA] =
    [const { Penyangga::kosong_statis() }; MAX_PENYANGGA];
static DIPAKAI: [AtomicU32; MAX_PENYANGGA] = [const { AtomicU32::new(0) }; MAX_PENYANGGA];

/// `0` buat · `1` lepas · `2` tulis · `3` baca · `4` flip ·
/// `5` pelanggaran-verifikasi · `6` tulis-penuh · `7` baca-kosong.
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

/// Jumlah penyangga hidup.
fn hidup() -> u32 {
    let mut n = 0u32;
    let mut i = 0usize;
    while i < MAX_PENYANGGA {
        if DIPAKAI[i].load(Ordering::Acquire) != 0 {
            n += 1;
        }
        i += 1;
    }
    n
}

/// Slot handle (1..=MAX). `None` bila handle 0, di luar rentang, atau bebas.
fn slot(handle: u32) -> Option<&'static Penyangga> {
    if handle == 0 || (handle as usize) > MAX_PENYANGGA {
        return None;
    }
    let i = (handle - 1) as usize;
    if DIPAKAI[i].load(Ordering::Acquire) == 0 {
        return None;
    }
    Some(&TABEL[i])
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — PRODUKSI
// ---------------------------------------------------------------------------

/// Membuat penyangga ganda. Mengembalikan handle `1..=MAX`, atau `0` bila gagal.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_buat(kapasitas: u64) -> u32 {
    if kapasitas > u32::MAX as u64 {
        return 0;
    }
    let mut i = 0usize;
    while i < MAX_PENYANGGA {
        if DIPAKAI[i]
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            TABEL[i].init(kapasitas as u32);
            COUNTERS[0].fetch_add(1, Ordering::Relaxed);
            return (i as u32) + 1;
        }
        i += 1;
    }
    0
}

/// Melepas penyangga ganda. `1` bila ada, `0` bila handle tidak sah / sudah bebas.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_lepas(handle: u32) -> u32 {
    if handle == 0 || (handle as usize) > MAX_PENYANGGA {
        return 0;
    }
    let i = (handle - 1) as usize;
    if DIPAKAI[i].swap(0, Ordering::AcqRel) == 0 {
        return 0;
    }
    COUNTERS[1].fetch_add(1, Ordering::Relaxed);
    1
}

/// Jumlah penyangga hidup.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_hidup() -> u32 {
    hidup()
}

/// `write()` fase 1: berapa byte yang boleh ditulis.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_tulis(handle: u32, diminta: u64) -> u64 {
    match slot(handle) {
        Some(p) => {
            COUNTERS[2].fetch_add(1, Ordering::Relaxed);
            let n = p.tulis(diminta);
            if n == 0 {
                COUNTERS[6].fetch_add(1, Ordering::Relaxed);
            }
            n
        }
        None => 0,
    }
}

/// `write()` fase 2: commit.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_selesai_tulis(handle: u32, ditulis: u64) -> u32 {
    match slot(handle) {
        Some(p) => {
            p.selesai_tulis(ditulis);
            1
        }
        None => 0,
    }
}

/// Kursor tulis: `(paruh, offset)`.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_kursor_tulis(handle: u32) -> KlatKursor {
    match slot(handle) {
        Some(p) => p.kursor_tulis(),
        None => KURSOR_KOSONG,
    }
}

/// `read_impl()` fase 1: berapa byte yang boleh dibaca (`0` = tidak ada).
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_baca(handle: u32, diminta: u64) -> u64 {
    match slot(handle) {
        Some(p) => {
            COUNTERS[3].fetch_add(1, Ordering::Relaxed);
            let (n, flip, verify_ok) = p.baca(diminta);
            if flip {
                COUNTERS[4].fetch_add(1, Ordering::Relaxed);
            }
            if !verify_ok {
                COUNTERS[5].fetch_add(1, Ordering::Relaxed);
            }
            if n == 0 {
                COUNTERS[7].fetch_add(1, Ordering::Relaxed);
            }
            n
        }
        None => 0,
    }
}

/// `read_impl()` fase 2: `maju != 0` memajukan indeks baca.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_selesai_baca(
    handle: u32,
    dibaca: u64,
    maju: u32,
) -> u32 {
    match slot(handle) {
        Some(p) => {
            p.selesai_baca(dibaca, maju);
            1
        }
        None => 0,
    }
}

/// Kursor baca: `(paruh, offset)`.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_kursor_baca(handle: u32) -> KlatKursor {
    match slot(handle) {
        Some(p) => p.kursor_baca(),
        None => KURSOR_KOSONG,
    }
}

/// `1` bila kosong, `1` juga untuk handle tidak sah (aman untuk `can_read`).
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_kosong(handle: u32) -> u32 {
    match slot(handle) {
        Some(p) => p.kosong(),
        None => 1,
    }
}

/// `space_for_writing`.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_ruang_tulis(handle: u32) -> u64 {
    match slot(handle) {
        Some(p) => p.ruang_tulis(),
        None => 0,
    }
}

/// `immediately_readable`.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_segera_baca(handle: u32) -> u64 {
    match slot(handle) {
        Some(p) => p.segera_baca(),
        None => 0,
    }
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosPenyanggaReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosPenyanggaReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_a: u32,
    pub prod_b: u32,
    pub prod_c: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

struct Matrix {
    checks: u32,
    passed: u32,
    grup_checks: [u32; 3],
    grup_passed: [u32; 3],
}

impl Matrix {
    const fn baru() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            grup_checks: [0; 3],
            grup_passed: [0; 3],
        }
    }
    fn catat(&mut self, lulus: bool, grup: usize) {
        self.checks += 1;
        self.grup_checks[grup] += 1;
        if lulus {
            self.passed += 1;
            self.grup_passed[grup] += 1;
        }
    }
}

fn run_matrix() -> KlatosPenyanggaReport {
    let mut m = Matrix::baru();

    let hidup_awal = hidup();
    let counter_awal = [
        klatos_penyangga_ganda_counter(0),
        klatos_penyangga_ganda_counter(1),
        klatos_penyangga_ganda_counter(2),
        klatos_penyangga_ganda_counter(3),
        klatos_penyangga_ganda_counter(4),
        klatos_penyangga_ganda_counter(5),
        klatos_penyangga_ganda_counter(6),
        klatos_penyangga_ganda_counter(7),
    ];

    // --- kelompok 0: dasar + flip + peek + batas kapasitas ------------------
    {
        let p = Penyangga::kosong_statis();
        p.init(8);
        m.catat(p.kosong() == 1, 0);
        m.catat(p.ruang_tulis() == 8, 0);
        m.catat(p.segera_baca() == 0, 0);
        m.catat(
            p.kursor_tulis() == KlatKursor { paruh: 0, offset: 0 },
            0,
        );
        m.catat(p.kursor_baca() == KlatKursor { paruh: 1, offset: 0 }, 0);

        // tulis 3, commit
        m.catat(p.tulis(3) == 3, 0);
        m.catat(p.kursor_tulis() == KlatKursor { paruh: 0, offset: 0 }, 0);
        p.selesai_tulis(3);
        m.catat(p.kosong() == 0 && p.ruang_tulis() == 5 && p.segera_baca() == 3, 0);
        m.catat(p.kursor_tulis() == KlatKursor { paruh: 0, offset: 3 }, 0);

        // tulis 10 -> hanya 5
        m.catat(p.tulis(10) == 5, 0);
        p.selesai_tulis(5);
        m.catat(p.ruang_tulis() == 0 && p.segera_baca() == 8, 0);
        m.catat(p.tulis(1) == 0, 0);

        // baca 2 -> flip (paruh tulis 0 -> 1)
        let (n, f, _) = p.baca(2);
        m.catat(n == 2 && f, 0);
        m.catat(p.kursor_baca() == KlatKursor { paruh: 0, offset: 0 }, 0);
        p.selesai_baca(2, 1);
        m.catat(p.kosong() == 0 && p.ruang_tulis() == 8 && p.segera_baca() == 6, 0);
        m.catat(p.kursor_baca() == KlatKursor { paruh: 0, offset: 2 }, 0);

        // baca 100 -> 6 (tanpa flip)
        let (n, f, _) = p.baca(100);
        m.catat(n == 6 && !f, 0);
        p.selesai_baca(6, 1);
        m.catat(p.kosong() == 1 && p.ruang_tulis() == 8 && p.segera_baca() == 0, 0);

        // baca saat kosong -> 0, tanpa flip
        let (n, f, _) = p.baca(1);
        m.catat(n == 0 && !f, 0);

        // peek (maju=0) idempoten
        m.catat(p.tulis(4) == 4, 0);
        p.selesai_tulis(4);
        m.catat(p.segera_baca() == 4 && p.kosong() == 0, 0);
        let (n1, f1, _) = p.baca(2);
        m.catat(n1 == 2 && f1, 0);
        p.selesai_baca(2, 0);
        let (n2, f2, _) = p.baca(2);
        m.catat(n2 == 2 && !f2, 0);
        p.selesai_baca(2, 0);
        m.catat(p.segera_baca() == 4, 0);
        // maju sungguhan
        p.selesai_baca(2, 1);
        m.catat(p.segera_baca() == 2, 0);

        // kapasitas 1 (batas minimum)
        let q = Penyangga::kosong_statis();
        q.init(1);
        m.catat(q.ruang_tulis() == 1 && q.kosong() == 1, 0);
        m.catat(q.tulis(1) == 1, 0);
        q.selesai_tulis(1);
        m.catat(q.ruang_tulis() == 0 && q.segera_baca() == 1, 0);
        m.catat(q.tulis(1) == 0, 0);
        let (n, f, _) = q.baca(1);
        m.catat(n == 1 && f, 0);
        q.selesai_baca(1, 1);
        m.catat(q.kosong() == 1 && q.segera_baca() == 0, 0);
        let (n, f, _) = q.baca(1);
        m.catat(n == 0 && !f, 0);

        // kapasitas 0 (degenerat, harus aman)
        let z = Penyangga::kosong_statis();
        z.init(0);
        m.catat(z.ruang_tulis() == 0 && z.kosong() == 1, 0);
        m.catat(z.tulis(5) == 0, 0);
        let (n, f, _) = z.baca(5);
        m.catat(n == 0 && !f, 0);

        // banyak flip berturut-turut (bungkus paruh)
        let w = Penyangga::kosong_statis();
        w.init(4);
        let mut putaran = 0u32;
        let mut flip_ok = true;
        while putaran < 32 {
            w.selesai_tulis(w.tulis(4));
            let (n, f, _) = w.baca(4);
            if n != 4 || !f {
                flip_ok = false;
            }
            w.selesai_baca(4, 1);
            putaran += 1;
        }
        m.catat(flip_ok && w.kosong() == 1, 0);
    }

    // --- kelompok 1: diferensial terhadap oracle historis -------------------
    let mut diukur = 0u32;
    let mut beda = 0u32;
    {
        let uji = Penyangga::kosong_statis();
        uji.init(KAPASITAS_DIFERENSIAL);
        let mut model = Model::baru(KAPASITAS_DIFERENSIAL);
        let mut x = 0x1234_5678u32;
        let mut i = 0u32;
        while i < RENTANG_DIFERENSIAL {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let (n1, f1, v1) = langkah_baru(&uji, x);
            let (n2, f2, v2) = kebijakan_lama(&mut model, x);
            if (n1, f1, v1) != (n2, f2, v2) {
                beda += 1;
            }
            if uji.kosong() != model.kosong() {
                beda += 1;
            }
            if uji.ruang_tulis() != model.ruang_tulis() {
                beda += 1;
            }
            if uji.segera_baca() != model.segera_baca() {
                beda += 1;
            }
            if uji.kursor_tulis() != model.kursor_tulis() {
                beda += 1;
            }
            if uji.kursor_baca() != model.kursor_baca() {
                beda += 1;
            }
            diukur += 1;
            i += 1;
        }
        m.catat(beda == 0, 1);
        m.catat(diukur == RENTANG_DIFERENSIAL, 1);
    }

    // --- kelompok 2: negatif + kontrol produksi -----------------------------
    {
        // KONTROL: kelompok 0/1 (instans lokal) TIDAK menyentuh tabel produksi.
        m.catat(hidup() == hidup_awal, 2);
        let mut c = 0usize;
        let mut sama = true;
        while c < N_COUNTER {
            if klatos_penyangga_ganda_counter(c as u32) != counter_awal[c] {
                sama = false;
            }
            c += 1;
        }
        m.catat(sama, 2);

        // negatif: handle tidak sah (0, di luar rentang, sudah bebas)
        m.catat(klatos_penyangga_ganda_tulis(0, 10) == 0, 2);
        m.catat(klatos_penyangga_ganda_baca(0, 10) == 0, 2);
        m.catat(klatos_penyangga_ganda_lepas(0) == 0, 2);
        m.catat(klatos_penyangga_ganda_lepas(0xFFFF_FFFF) == 0, 2);
        m.catat(klatos_penyangga_ganda_kosong(0) == 1, 2);
        m.catat(klatos_penyangga_ganda_ruang_tulis(0) == 0, 2);
        m.catat(klatos_penyangga_ganda_segera_baca(0) == 0, 2);
        m.catat(klatos_penyangga_ganda_kursor_tulis(0) == KURSOR_KOSONG, 2);
        m.catat(klatos_penyangga_ganda_kursor_baca(0) == KURSOR_KOSONG, 2);
        m.catat(
            klatos_penyangga_ganda_selesai_tulis(0xFFFF_FFFF, 4) == 0,
            2,
        );

        // siklus hidup buat/lepas lewat permukaan FFI NYATA (satu pasang)
        let h = klatos_penyangga_ganda_buat(16);
        m.catat(h >= 1, 2);
        if h >= 1 {
            m.catat(hidup() == hidup_awal + 1, 2);
            m.catat(klatos_penyangga_ganda_kosong(h) == 1, 2);
            m.catat(klatos_penyangga_ganda_ruang_tulis(h) == 16, 2);
            m.catat(klatos_penyangga_ganda_segera_baca(h) == 0, 2);
            m.catat(klatos_penyangga_ganda_tulis(h, 4) == 4, 2);
            let k = klatos_penyangga_ganda_kursor_tulis(h);
            m.catat(k == KlatKursor { paruh: 0, offset: 0 }, 2);
            m.catat(klatos_penyangga_ganda_selesai_tulis(h, 4) == 1, 2);
            m.catat(klatos_penyangga_ganda_kosong(h) == 0, 2);
            m.catat(klatos_penyangga_ganda_segera_baca(h) == 4, 2);
            m.catat(klatos_penyangga_ganda_baca(h, 4) == 4, 2);
            let kb = klatos_penyangga_ganda_kursor_baca(h);
            m.catat(kb == KlatKursor { paruh: 0, offset: 0 }, 2);
            m.catat(klatos_penyangga_ganda_selesai_baca(h, 4, 1) == 1, 2);
            m.catat(klatos_penyangga_ganda_kosong(h) == 1, 2);
            m.catat(klatos_penyangga_ganda_lepas(h) == 1, 2);
            // lepas ganda: yang kedua harus ditolak
            m.catat(klatos_penyangga_ganda_lepas(h) == 0, 2);
            m.catat(hidup() == hidup_awal, 2);
            // handle bebas diperlakukan seperti handle tidak sah
            m.catat(klatos_penyangga_ganda_tulis(h, 4) == 0, 2);
        }
        // tepat satu buat + satu lepas lewat FFI (dari blok di atas)
        m.catat(klatos_penyangga_ganda_counter(0) == counter_awal[0] + 1, 2);
        m.catat(klatos_penyangga_ganda_counter(1) == counter_awal[1] + 1, 2);

        // struktur ABI
        m.catat(core::mem::size_of::<KlatKursor>() == 16, 2);
    }

    let mut report = KlatosPenyanggaReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.grup_checks[1],
        peta_passed: m.grup_passed[1],
        neg_checks: m.grup_checks[2],
        neg_passed: m.grup_passed[2],
        prod_a: klatos_penyangga_ganda_counter(0),
        prod_b: klatos_penyangga_ganda_counter(1),
        prod_c: klatos_penyangga_ganda_counter(4),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status: 0,
    };
    if report.checks != report.passed {
        report.status = -1;
    }
    report
}

/// Menjalankan matriks otoritas metadata penyangga ganda.
#[no_mangle]
pub extern "C" fn klatos_penyangga_ganda_selftest() -> KlatosPenyanggaReport {
    run_matrix()
}

/// Konformansi waktu-kompilasi.
pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5047_4441); // "KLATPGDA"
        assert!(N_COUNTER == 8);
        assert!(MAX_PENYANGGA == 512);
        assert!(core::mem::size_of::<KlatKursor>() == 16);
        assert!(core::mem::size_of::<KlatosPenyanggaReport>() == 56);
        assert!(KURSOR_KOSONG.paruh == 0 && KURSOR_KOSONG.offset == 0);
        // Oracle: 7 medan u32 + `ukuran[2]` = 8 u32 = 32 byte.
        assert!(core::mem::size_of::<Model>() == 32);
    };
}
