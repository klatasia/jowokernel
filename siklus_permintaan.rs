// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8.4-B — **OTORITAS SIKLUS HIDUP PERMINTAAN PERANGKAT (generik)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosPermintaanReport` di `KlatSeam134.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_504D_4E54; // "KLATPMNT"

/// Indeks/deskriptor tidak sah.
pub const INVALID: u32 = 0xFFFF_FFFF;

/// Jumlah slot permintaan maksimum (tetap, tanpa alokasi).
pub const MAX_PERMINTAAN: usize = 4096;

// --- status siklus hidup (byte-identik dengan RequestResult C++) ------------

/// `RequestResult::Pending`.
pub const M_PENDING: u32 = 0;
/// `RequestResult::Started`.
pub const M_MULAI: u32 = 1;
/// `RequestResult::Success`.
pub const M_SUKSES: u32 = 2;
/// `RequestResult::Failure`.
pub const M_GAGAL: u32 = 3;
/// `RequestResult::MemoryFault`.
pub const M_FAULT: u32 = 4;
/// `RequestResult::OutOfMemory` (dianggap selesai).
pub const M_OOM: u32 = 5;
/// `RequestResult::Cancelled` (dianggap selesai).
pub const M_BATAL: u32 = 6;
/// Slot bebas (bukan state `RequestResult` yang sah).
pub const M_LEPAS: u32 = 7;

// --- errno mirror (Jowo/Kontrak/POSIX/errno.h) ------------------------------

/// `EBADF` — handle usang/aspal.
pub const E_BADF: i32 = 9;
/// `EINVAL` — transisi/argumen tidak sah.
pub const E_INVAL: i32 = 22;
/// `ENOMEM` — arena habis.
pub const E_NOMEM: i32 = 12;

/// Penghitung untuk audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `buat` berhasil.
    ProdBuat = 0,
    /// Transisi ke `Started`.
    ProdMulai = 1,
    /// Penyelesaian eksplisit (`selesai`).
    ProdSelesai = 2,
    /// Penyelesaian agregat (`agregat`).
    ProdAgregat = 3,
    /// Pelepasan slot.
    ProdLepas = 4,
    /// Penolakan handle usang/aspal.
    ProdTolakStale = 5,
    /// Penolakan transisi state.
    ProdTolakState = 6,
    /// Penolakan hasil di luar kontrak.
    ProdTolakHasil = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

/// `status > Started` — byte-identik dengan `is_completed_result` C++.
#[inline]
pub fn selesai_dari(result: u32) -> bool {
    result >= M_SUKSES && result <= M_BATAL
}

/// Hasil penyelesaian eksplisit yang sah (`Success`/`Failure`/`MemoryFault`).
#[inline]
pub fn hasil_sah(result: u32) -> bool {
    result == M_SUKSES || result == M_GAGAL || result == M_FAULT
}

// ---------------------------------------------------------------------------
// Arena permintaan — satu-satunya pemilik identitas + status
// ---------------------------------------------------------------------------

static PM_GEN: [AtomicU32; MAX_PERMINTAAN] = [const { AtomicU32::new(0) }; MAX_PERMINTAAN];
static PM_ST: [AtomicU32; MAX_PERMINTAAN] = [const { AtomicU32::new(M_LEPAS) }; MAX_PERMINTAAN];
static PM_NEXT: [AtomicU32; MAX_PERMINTAAN] = [const { AtomicU32::new(INVALID) }; MAX_PERMINTAAN];
static PM_FREE: AtomicU32 = AtomicU32::new(INVALID);
static PM_LIVE: AtomicU32 = AtomicU32::new(0);
static INIT: AtomicU32 = AtomicU32::new(0);
static LOCK: AtomicU32 = AtomicU32::new(0);

/// Ambil kunci arena; kembalikan `true` bila interupsi tadinya menyala.
///
/// Jalur IRQ nyata memakai pintu ini: `AsyncDeviceRequest::complete()` dipanggil
/// dari handler perangkat (`Processor::current_in_irq()`) dan memanggil
/// `klatos_minta_selesai`, sementara thread biasa memanggil `status`/`buat`
/// dengan interupsi MENYALA.  Kunci HARUS IRQ-aman pada target kernel, jika
/// tidak, sebuah IRQ yang tiba di CPU yang sedang memegang kunci akan
/// memutar-selamanya menunggu kunci yang dipegang oleh thread yang justru ia
/// preempt.  Pola ini sama dengan `antrian_timer.rs`: `cli` menahan IRQ pada
/// CPU ini selama kunci dipegang, lalu keadaan semula dipulihkan.  Harness host
/// (tanpa `crate::arch`) memakai spinlock polos.
#[inline]
fn lock() -> bool {
    #[cfg(target_os = "klatos")]
    {
        let was = crate::arch::interrupts_enabled();
        crate::arch::disable_interrupts();
        while LOCK
            .compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        was
    }
    #[cfg(not(target_os = "klatos"))]
    {
        while LOCK
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
    LOCK.store(0, Ordering::Release);
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

/// Sekali: bangun rantai bebas slot.
fn pastikan_init() {
    if INIT.load(Ordering::Acquire) == 1 {
        return;
    }
    let _if = lock();
    if INIT.load(Ordering::Relaxed) == 0 {
        let mut i = 0usize;
        while i < MAX_PERMINTAAN {
            PM_GEN[i].store(0, Ordering::Relaxed);
            PM_ST[i].store(M_LEPAS, Ordering::Relaxed);
            PM_NEXT[i].store(
                if i + 1 < MAX_PERMINTAAN { (i + 1) as u32 } else { INVALID },
                Ordering::Relaxed,
            );
            i += 1;
        }
        PM_FREE.store(0, Ordering::Relaxed);
        PM_LIVE.store(0, Ordering::Relaxed);
        INIT.store(1, Ordering::Release);
    }
    unlock(_if);
}

#[inline]
fn hidup(index: u32, generation: u32) -> bool {
    let i = index as usize;
    generation != 0
        && i < MAX_PERMINTAAN
        && PM_GEN[i].load(Ordering::Acquire) == generation
        && PM_ST[i].load(Ordering::Acquire) != M_LEPAS
}

#[inline]
fn handle_of(index: u32, generation: u32) -> u64 {
    ((generation as u64) << 32) | index as u64
}

/// `buat` — ambil slot baru; 0 bila arena penuh.
pub fn buat() -> u64 {
    pastikan_init();
    let _if = lock();
    let free = PM_FREE.load(Ordering::Relaxed);
    let result = if free == INVALID {
        bump(Counter::ProdTolakStale); // tidak ada slot (dihitung sebagai tolak)
        0
    } else {
        let s = free as usize;
        PM_FREE.store(PM_NEXT[s].load(Ordering::Relaxed), Ordering::Relaxed);
        PM_NEXT[s].store(INVALID, Ordering::Relaxed);
        let generation = PM_GEN[s].load(Ordering::Relaxed).wrapping_add(1);
        if generation == 0 {
            // Wrap (praktis tidak mungkin): kembalikan slot ke rantai bebas.
            PM_NEXT[s].store(PM_FREE.load(Ordering::Relaxed), Ordering::Relaxed);
            PM_FREE.store(s as u32, Ordering::Relaxed);
            0
        } else {
            PM_GEN[s].store(generation, Ordering::Relaxed);
            PM_ST[s].store(M_PENDING, Ordering::Release);
            PM_LIVE.fetch_add(1, Ordering::Relaxed);
            bump(Counter::ProdBuat);
            handle_of(s as u32, generation)
        }
    };
    unlock(_if);
    result
}

/// `mulai` — `Pending → Started` (idempoten pada `Started`).
///
/// Kembalian: `0` = panggil `start()`; `1` = sudah selesai (jangan panggil);
/// `-EBADF` = handle usang/aspal.
pub fn mulai(handle: u64) -> i32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        bump(Counter::ProdTolakStale);
        -E_BADF
    } else {
        let i = index as usize;
        let st = PM_ST[i].load(Ordering::Relaxed);
        if selesai_dari(st) {
            1
        } else {
            PM_ST[i].store(M_MULAI, Ordering::Release);
            bump(Counter::ProdMulai);
            0
        }
    };
    unlock(_if);
    r
}

/// `selesai` — `Started → result` (hanya `Success`/`Failure`/`MemoryFault`).
pub fn selesai(handle: u64, result: u32) -> i32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        bump(Counter::ProdTolakStale);
        -E_BADF
    } else if !hasil_sah(result) {
        bump(Counter::ProdTolakHasil);
        -E_INVAL
    } else {
        let i = index as usize;
        if PM_ST[i].load(Ordering::Relaxed) != M_MULAI {
            bump(Counter::ProdTolakState);
            -E_INVAL
        } else {
            PM_ST[i].store(result, Ordering::Release);
            bump(Counter::ProdSelesai);
            0
        }
    };
    unlock(_if);
    r
}

/// `agregat` — `Started → {Failure|MemoryFault|Success}` dari dua fakta.
pub fn agregat(handle: u64, ada_gagal: u32, ada_fault: u32) -> i32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        bump(Counter::ProdTolakStale);
        -E_BADF
    } else {
        let i = index as usize;
        if PM_ST[i].load(Ordering::Relaxed) != M_MULAI {
            bump(Counter::ProdTolakState);
            -E_INVAL
        } else {
            let final_ = if ada_gagal != 0 {
                M_GAGAL
            } else if ada_fault != 0 {
                M_FAULT
            } else {
                M_SUKSES
            };
            PM_ST[i].store(final_, Ordering::Release);
            bump(Counter::ProdAgregat);
            0
        }
    };
    unlock(_if);
    r
}

/// `status` — state saat ini, atau `-EBADF` bila handle usang/aspal.
pub fn status(handle: u64) -> i32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        -E_BADF
    } else {
        PM_ST[index as usize].load(Ordering::Relaxed) as i32
    };
    unlock(_if);
    r
}

/// `selesai_p` — `1` bila permintaan sudah selesai, `0` bila belum/usang.
pub fn selesai_p(handle: u64) -> u32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        0
    } else if selesai_dari(PM_ST[index as usize].load(Ordering::Relaxed)) {
        1
    } else {
        0
    };
    unlock(_if);
    r
}

/// `lepas` — lepaskan slot (idempoten TIDAK diizinkan: lepas ganda = `-EBADF`).
pub fn lepas(handle: u64) -> i32 {
    pastikan_init();
    let index = handle as u32;
    let generation = (handle >> 32) as u32;
    let _if = lock();
    let r = if !hidup(index, generation) {
        bump(Counter::ProdTolakStale);
        -E_BADF
    } else {
        let i = index as usize;
        PM_ST[i].store(M_LEPAS, Ordering::Release);
        PM_NEXT[i].store(PM_FREE.load(Ordering::Relaxed), Ordering::Relaxed);
        PM_FREE.store(i as u32, Ordering::Release);
        PM_LIVE.fetch_sub(1, Ordering::Relaxed);
        bump(Counter::ProdLepas);
        1
    };
    unlock(_if);
    r
}

/// Jumlah slot hidup.
pub fn jumlah() -> u32 {
    pastikan_init();
    PM_LIVE.load(Ordering::Relaxed)
}

/// Kembalikan seluruh arena ke keadaan awal (hanya untuk uji).
pub fn reset() -> u32 {
    pastikan_init();
    let _if = lock();
    let mut i = 0usize;
    while i < MAX_PERMINTAAN {
        PM_GEN[i].store(0, Ordering::Relaxed);
        PM_ST[i].store(M_LEPAS, Ordering::Relaxed);
        PM_NEXT[i].store(
            if i + 1 < MAX_PERMINTAAN { (i + 1) as u32 } else { INVALID },
            Ordering::Relaxed,
        );
        i += 1;
    }
    PM_FREE.store(0, Ordering::Relaxed);
    PM_LIVE.store(0, Ordering::Relaxed);
    unlock(_if);
    0
}

// ---------------------------------------------------------------------------
// Pintu C ABI
// ---------------------------------------------------------------------------

/// Laporan matriks otoritas siklus permintaan. Cermin `KlatSeam134.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosPermintaanReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_mulai: u32,
    pub prod_selesai: u32,
    pub prod_agregat: u32,
    pub prod_lepas: u32,
    pub tolak_stale: u32,
    pub tolak_state: u32,
    pub jumlah_maks: u32,
    pub status: i32,
}

const _: [(); 72] = [(); core::mem::size_of::<KlatosPermintaanReport>()];

#[no_mangle]
pub extern "C" fn klatos_minta_buat() -> u64 {
    buat()
}

#[no_mangle]
pub extern "C" fn klatos_minta_mulai(handle: u64) -> i32 {
    mulai(handle)
}

#[no_mangle]
pub extern "C" fn klatos_minta_selesai(handle: u64, result: u32) -> i32 {
    selesai(handle, result)
}

#[no_mangle]
pub extern "C" fn klatos_minta_agregat(handle: u64, ada_gagal: u32, ada_fault: u32) -> i32 {
    agregat(handle, ada_gagal, ada_fault)
}

#[no_mangle]
pub extern "C" fn klatos_minta_status(handle: u64) -> i32 {
    status(handle)
}

#[no_mangle]
pub extern "C" fn klatos_minta_selesai_p(handle: u64) -> u32 {
    selesai_p(handle)
}

#[no_mangle]
pub extern "C" fn klatos_minta_selesai_dari(result: u32) -> u32 {
    if selesai_dari(result) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_minta_lepas(handle: u64) -> i32 {
    lepas(handle)
}

#[no_mangle]
pub extern "C" fn klatos_minta_jumlah() -> u32 {
    jumlah()
}

#[no_mangle]
pub extern "C" fn klatos_minta_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn klatos_minta_reset() -> u32 {
    reset()
}

// ---------------------------------------------------------------------------
// Selftest (host + runtime; RELATIF agar tahan arena tak-kosong)
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
    fn catat(&mut self, lulus: bool) {
        self.checks += 1;
        if lulus {
            self.passed += 1;
        }
    }
    fn neg(&mut self, lulus: bool) {
        self.neg_checks += 1;
        if lulus {
            self.neg_passed += 1;
        }
        self.catat(lulus);
    }
    fn dif(&mut self, lulus: bool) {
        self.dif_checks += 1;
        if !lulus {
            self.dif_beda += 1;
        }
        self.catat(lulus);
    }
}

/// Model siklus permintaan independen (untuk diferensial).
struct Model {
    state: [u32; 64],
    live: [bool; 64],
    gen: [u32; 64],
}

impl Model {
    fn new() -> Self {
        Model { state: [M_LEPAS; 64], live: [false; 64], gen: [0; 64] }
    }
    fn alloc(&mut self) -> usize {
        let mut i = 0usize;
        while i < 64 {
            if !self.live[i] {
                self.live[i] = true;
                self.gen[i] = self.gen[i].wrapping_add(1);
                self.state[i] = M_PENDING;
                return i;
            }
            i += 1;
        }
        0
    }
    fn free(&mut self, i: usize) {
        self.live[i] = false;
        self.state[i] = M_LEPAS;
    }
    fn mulai(&mut self, i: usize) -> i32 {
        if !self.live[i] {
            return -E_BADF;
        }
        if selesai_dari(self.state[i]) {
            return 1;
        }
        self.state[i] = M_MULAI;
        0
    }
    fn selesai(&mut self, i: usize, result: u32) -> i32 {
        if !self.live[i] {
            return -E_BADF;
        }
        if !hasil_sah(result) {
            return -E_INVAL;
        }
        if self.state[i] != M_MULAI {
            return -E_INVAL;
        }
        self.state[i] = result;
        0
    }
    fn agregat(&mut self, i: usize, ada_gagal: u32, ada_fault: u32) -> i32 {
        if !self.live[i] {
            return -E_BADF;
        }
        if self.state[i] != M_MULAI {
            return -E_INVAL;
        }
        self.state[i] = if ada_gagal != 0 {
            M_GAGAL
        } else if ada_fault != 0 {
            M_FAULT
        } else {
            M_SUKSES
        };
        0
    }
}

fn run_matrix() -> KlatosPermintaanReport {
    let mut m = Matrix::new();

    // --- G1: konstanta ------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_504D_4E54);
    m.catat(MAX_PERMINTAAN == 4096);
    m.catat(M_PENDING == 0 && M_MULAI == 1 && M_SUKSES == 2 && M_GAGAL == 3 && M_FAULT == 4);
    m.catat(M_OOM == 5 && M_BATAL == 6 && M_LEPAS == 7);
    m.catat(E_BADF == 9 && E_INVAL == 22 && E_NOMEM == 12);
    m.catat(selesai_dari(M_SUKSES) && selesai_dari(M_GAGAL) && selesai_dari(M_FAULT));
    m.catat(selesai_dari(M_OOM) && selesai_dari(M_BATAL));
    m.catat(!selesai_dari(M_PENDING) && !selesai_dari(M_MULAI) && !selesai_dari(M_LEPAS));
    m.catat(hasil_sah(M_SUKSES) && hasil_sah(M_GAGAL) && hasil_sah(M_FAULT));
    m.catat(!hasil_sah(M_PENDING) && !hasil_sah(M_OOM) && !hasil_sah(M_BATAL));

    // Jumlah hidup SEBELUM matriks: runtime kernel dapat sudah memiliki
    // permintaan hidup; karena itu matriks bersifat RELATIF.
    let jumlah_awal = jumlah();

    // --- G2: siklus hidup utama --------------------------------------------
    let a = buat();
    m.catat(a != 0);
    let (ai, ag) = (a as u32, (a >> 32) as u32);
    m.catat(status(a) == M_PENDING as i32);
    m.catat(selesai_p(a) == 0);
    m.catat(mulai(a) == 0);
    m.catat(status(a) == M_MULAI as i32);
    m.catat(selesai_p(a) == 0);
    m.catat(mulai(a) == 0); // idempoten pada Started (sesuai do_start C++)
    m.catat(selesai(a, M_SUKSES) == 0);
    m.catat(status(a) == M_SUKSES as i32);
    m.catat(selesai_p(a) == 1);
    m.catat(mulai(a) == 1); // sudah selesai → jangan start()
    m.catat(selesai(a, M_GAGAL) == -E_INVAL); // selesai ganda
    m.catat(lepas(a) == 1);
    m.catat(status(a) == -E_BADF);
    m.catat(lepas(a) == -E_BADF); // lepas ganda
    m.catat(mulai(a) == -E_BADF); // handle usang

    // --- G3: kebijakan agregasi --------------------------------------------
    let f = buat();
    m.catat(f != 0 && mulai(f) == 0);
    m.catat(agregat(f, 1, 1) == 0);
    m.catat(status(f) == M_GAGAL as i32); // Failure menang
    m.catat(lepas(f) == 1);
    let g = buat();
    m.catat(g != 0 && mulai(g) == 0);
    m.catat(agregat(g, 0, 1) == 0);
    m.catat(status(g) == M_FAULT as i32); // MemoryFault menang atas Success
    m.catat(lepas(g) == 1);
    let h = buat();
    m.catat(h != 0 && mulai(h) == 0);
    m.catat(agregat(h, 0, 0) == 0);
    m.catat(status(h) == M_SUKSES as i32);
    m.catat(lepas(h) == 1);

    // --- G4: negatif wajib --------------------------------------------------
    m.neg(status(INVALID as u64) == -E_BADF); // index aspal
    m.neg(status(handle_of(MAX_PERMINTAAN as u32, 1)) == -E_BADF); // index di luar
    m.neg(status(handle_of(ai, 0)) == -E_BADF); // generasi 0
    m.neg(mulai(handle_of(ai, ag.wrapping_add(1))) == -E_BADF); // generasi salah
    let p = buat();
    m.catat(p != 0);
    m.neg(selesai(p, M_SUKSES) == -E_INVAL); // selesai pada Pending ditolak
    m.neg(agregat(p, 0, 0) == -E_INVAL);
    m.catat(mulai(p) == 0);
    m.neg(selesai(p, M_PENDING) == -E_INVAL); // hasil di luar kontrak
    m.neg(selesai(p, M_OOM) == -E_INVAL);
    m.neg(selesai(p, 99) == -E_INVAL);
    m.catat(lepas(p) == 1);
    m.catat(jumlah() == jumlah_awal);

    // --- G5: pemakaian ulang slot + anti-ABA (robust, tanpa asumsi slot) ----
    let r1 = buat();
    let r2 = buat();
    m.catat(r1 != 0 && r2 != 0);
    let (r1i, r1g) = (r1 as u32, (r1 >> 32) as u32);
    m.catat(mulai(r1) == 0 && selesai(r1, M_SUKSES) == 0);
    m.catat(lepas(r1) == 1);
    let r3 = buat();
    let (r3i, r3g) = (r3 as u32, (r3 >> 32) as u32);
    m.catat(r3 != 0);
    m.catat(hidup(r3i, r3g));
    m.neg(!hidup(r1i, r1g)); // handle lama mati
    m.catat(r3i != r1i || r3g != r1g); // handle baru tidak identik
    m.catat(status(r3) == M_PENDING as i32);
    m.catat(lepas(r3) == 1);
    m.catat(lepas(r2) == 1);
    m.catat(jumlah() == jumlah_awal);

    // --- G6: kehabisan slot (ekshaustif) -----------------------------------
    let mut disimpan = [0u64; 4096];
    let mut n = 0usize;
    loop {
        let v = buat();
        if v == 0 {
            break;
        }
        disimpan[n] = v;
        n += 1;
        if n >= MAX_PERMINTAAN {
            break;
        }
    }
    // RELATIF: bila arena runtime sudah berisi permintaan hidup, yang dapat
    // dialokasikan hanyalah sisa ruang.
    m.catat(n as u32 == MAX_PERMINTAAN as u32 - jumlah_awal);
    m.catat(jumlah() == MAX_PERMINTAAN as u32);
    m.catat(buat() == 0); // penuh → 0
    let mut k = 0usize;
    while k < n {
        m.catat(lepas(disimpan[k]) == 1);
        k += 1;
    }
    m.catat(jumlah() == jumlah_awal);

    // --- G7: diferensial state machine (40k kasus, model independen) --------
    let mut model = Model::new();
    let mut handles = [0u64; 64];
    let mut slot = [0usize; 64];
    let mut nslots = 0usize;
    let mut probe: u64 = 0x51ED_2701_9E37_79B9;
    let mut t = 0u32;
    while t < 40_000 {
        probe = probe
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let op = ((probe >> 59) % 7) as u32;
        let idx = ((probe >> 13) % 64) as usize;
        match op {
            0 => {
                // buat: model vs arena
                if nslots < 64 {
                    let mh = model.alloc();
                    let rh = buat();
                    m.dif((mh < 64) == (rh != 0));
                    if rh != 0 && nslots < 64 {
                        handles[nslots] = rh;
                        slot[nslots] = mh;
                        nslots += 1;
                    }
                }
            }
            1 => {
                if nslots > 0 {
                    let s = idx % nslots;
                    let want = model.mulai(slot[s]);
                    let got = mulai(handles[s]);
                    m.dif(got == want);
                }
            }
            2 => {
                if nslots > 0 {
                    let s = idx % nslots;
                    let res = [M_SUKSES, M_GAGAL, M_FAULT, M_PENDING, M_OOM, 99][(idx % 6) as usize];
                    let want = model.selesai(slot[s], res);
                    let got = selesai(handles[s], res);
                    m.dif(got == want);
                }
            }
            3 => {
                if nslots > 0 {
                    let s = idx % nslots;
                    let ag = (idx & 1) as u32;
                    let af = ((idx >> 1) & 1) as u32;
                    let want = model.agregat(slot[s], ag, af);
                    let got = agregat(handles[s], ag, af);
                    m.dif(got == want);
                }
            }
            4 => {
                if nslots > 0 {
                    let s = idx % nslots;
                    let want = if model.live[slot[s]] {
                        model.free(slot[s]);
                        1
                    } else {
                        -E_BADF
                    };
                    let got = lepas(handles[s]);
                    m.dif(got == want);
                    if got == 1 {
                        // buang entri dari daftar aktif
                        handles[s] = handles[nslots - 1];
                        slot[s] = slot[nslots - 1];
                        nslots -= 1;
                    }
                }
            }
            5 => {
                if nslots > 0 {
                    let s = idx % nslots;
                    let want = if !model.live[slot[s]] {
                        -E_BADF
                    } else {
                        model.state[slot[s]] as i32
                    };
                    let got = status(handles[s]);
                    m.dif(got == want);
                }
            }
            _ => {
                if nslots > 0 {
                    let s = idx % nslots;
                    let want = if !model.live[slot[s]] {
                        0
                    } else if selesai_dari(model.state[slot[s]]) {
                        1
                    } else {
                        0
                    };
                    let got = selesai_p(handles[s]);
                    m.dif(got == want);
                }
            }
        }
        t += 1;
    }
    // Bersihkan semua sisa.
    let mut k = 0usize;
    while k < nslots {
        let _ = lepas(handles[k]);
        k += 1;
    }
    m.catat(jumlah() == jumlah_awal);

    let status = if m.checks == m.passed && m.dif_beda == 0 { 0 } else { -1 };

    KlatosPermintaanReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_buat: klatos_minta_counter(Counter::ProdBuat as u32),
        prod_mulai: klatos_minta_counter(Counter::ProdMulai as u32),
        prod_selesai: klatos_minta_counter(Counter::ProdSelesai as u32),
        prod_agregat: klatos_minta_counter(Counter::ProdAgregat as u32),
        prod_lepas: klatos_minta_counter(Counter::ProdLepas as u32),
        tolak_stale: klatos_minta_counter(Counter::ProdTolakStale as u32),
        tolak_state: klatos_minta_counter(Counter::ProdTolakState as u32),
        jumlah_maks: MAX_PERMINTAAN as u32,
        status,
    }
}

/// Matriks otoritas siklus permintaan perangkat.
#[no_mangle]
pub extern "C" fn klatos_minta_selftest() -> KlatosPermintaanReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_504D_4E54);
        assert!(MAX_PERMINTAAN == 4096);
        assert!(M_PENDING == 0 && M_MULAI == 1 && M_SUKSES == 2 && M_GAGAL == 3);
        assert!(M_FAULT == 4 && M_OOM == 5 && M_BATAL == 6 && M_LEPAS == 7);
        assert!(E_BADF == 9 && E_INVAL == 22 && E_NOMEM == 12);
        assert!(Counter::ProdBuat as usize == 0);
        assert!(Counter::ProdTolakHasil as usize == 7);
        assert!(N_COUNTER == 8);
        assert!(core::mem::size_of::<KlatosPermintaanReport>() == 72);
        assert!(core::mem::align_of::<KlatosPermintaanReport>() == 8);
    };
}
