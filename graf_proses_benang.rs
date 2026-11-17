// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//!**OTORITAS GRAF OBJEK PROSES / BENANG** (Rust).


use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Konstanta
// ---------------------------------------------------------------------------

/// Kapasitas tabel proses (berbatas).
pub const MAX_PROSES: usize = 4096;

/// Kapasitas tabel benang (berbatas).
pub const MAX_BENANG: usize = 4096;

/// Slot teratas dicadangkan untuk selftest.
pub const CADANGAN: usize = 64;

/// Batas atas alokasi produksi proses (eksklusif).
pub const BATAS_PROSES: usize = MAX_PROSES - CADANGAN;
/// Batas atas alokasi produksi benang (eksklusif).
pub const BATAS_BENANG: usize = MAX_BENANG - CADANGAN;

/// Tag laporan: "KLATGRPH".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4752_5048;

/// Sentinel "handle tidak sah".
pub const TAK_SAH: u32 = 0xFFFF_FFFF;

// Keadaan proses — cermin `Process::State`.
pub const PR_RUNNING: u32 = 0;
pub const PR_DYING: u32 = 1;
pub const PR_DEAD: u32 = 2;

// Keadaan benang — cermin `Thread::State` DAN `thread_transisi::ST_*`.
pub const TH_INVALID: u32 = 0;
pub const TH_RUNNABLE: u32 = 1;
pub const TH_RUNNING: u32 = 2;
pub const TH_DYING: u32 = 3;
pub const TH_DEAD: u32 = 4;
pub const TH_STOPPED: u32 = 5;
pub const TH_BLOCKED: u32 = 6;
pub const TH_N_STATE: u32 = 7;

// Bendera proses.
pub const PF_STOP: u32 = 1 << 0;
pub const PF_KERNEL: u32 = 1 << 1;

// FASE-4 BATCH 03 — kode `which` bidang JANJI per-proses (7..=10). Nilainya
// HARUS sama dengan `graf_pw_*` di `Jowo/Kontrak/KlatKernelBridge.h`.
pub const PW_HAS_PROMISES: u32 = 7;
pub const PW_PROMISES: u32 = 8;
pub const PW_HAS_EXECPROMISES: u32 = 9;
pub const PW_EXECPROMISES: u32 = 10;
/// `which` proses tertinggi yang sah.
pub const PW_MAKS: u32 = PW_EXECPROMISES;

// Bendera benang.
pub const TF_JOINABLE: u32 = 1 << 0;
pub const TF_ACTIVE: u32 = 1 << 1;
pub const TF_SHOULD_DIE: u32 = 1 << 2;
pub const TF_INITIALIZED: u32 = 1 << 3;
pub const TF_IDLE: u32 = 1 << 4;
pub const TF_CRASHING: u32 = 1 << 5;
pub const TF_IN_PROCESS: u32 = 1 << 6;
pub const TF_BLOCKING: u32 = 1 << 7;
pub const TF_INTR_DEATH: u32 = 1 << 8;
pub const TF_DID_TIMEOUT: u32 = 1 << 9;
pub const TF_SEMUA: u32 = 0x3FF;

// Pemilih `which` benang (lihat `recon/06-ffi-boundaries.md`).
pub const TW_PRIORITY: u32 = 2;
pub const TW_AFFINITY: u32 = 3;
pub const TW_PENDING_SIGNALS: u32 = 4;
pub const TW_SIGNAL_MASK: u32 = 5;
pub const TW_HANDLED_SIGNAL: u32 = 6;
pub const TW_INTR_SIGNAL: u32 = 9;

// Hasil blokir — cermin `Thread::BlockResult::Type`.
pub const BR_WOKE: u32 = 0;
pub const BR_NOT_BLOCKED: u32 = 1;
pub const BR_INTR_SIGNAL: u32 = 2;
pub const BR_INTR_DEATH: u32 = 3;
pub const BR_INTR_TIMEOUT: u32 = 4;

/// Nilai awal afinitas (`THREAD_AFFINITY_DEFAULT`).
pub const AFINITAS_BAWAAN: u32 = 0xFFFF_FFFF;

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_graf_proses_daftar` dari produksi.
    ProdDaftarProses = 0,
    /// `klatos_graf_proses_lepas` dari produksi.
    ProdLepasProses = 1,
    /// `klatos_graf_benang_daftar` dari produksi.
    ProdDaftarBenang = 2,
    /// `klatos_graf_benang_lepas` dari produksi.
    ProdLepasBenang = 3,
    /// Transisi `Running -> Dying` yang berhasil (CAS `die()`).
    ProdMati = 4,
    /// `set_keadaan` benang dari produksi.
    ProdTransisi = 5,
    /// `set_flag` benang dari produksi.
    ProdSetFlag = 6,
    /// Handle basi / di luar tabel.
    ProdHandleSalah = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// DIAGNOSTIK F-P149-05 — mengapa `ProdHandleSalah` naik?
// ---------------------------------------------------------------------------
//
// `ProdHandleSalah` menandai panggilan yang DITOLAK otoritas: handle basi ATAU
// argumen di luar rentang. Untuk memisahkan sebabnya, setiap titik penolakan
// melaporkan ID fungsinya ke histogram di bawah, dan penolakan PERTAMA pada
// setiap fungsi merekam (index, gen, argumen) yang ditolak.

/// Histogram penolakan per titik masuk. Indeks = ID fungsi (1..15).
static TOLAK: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
/// Salinan histogram pada saat `klatos_graf_tolak_snapshot()` dipanggil.
static TOLAK_BASE: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
/// Penanda "detail sudah direkam" per titik masuk (di-reset oleh snapshot).
static TOLAK_SEEN: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
/// Detail penolakan pertama per titik masuk: index, gen, argumen.
static TOLAK_IDX: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
static TOLAK_GEN: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
static TOLAK_ARG: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];

#[inline]
fn tolak(fn_id: u32, index: u32, gen: u32, arg: u32) {
    bump(Counter::ProdHandleSalah);
    let i = (fn_id & 0xF) as usize;
    TOLAK[i].fetch_add(1, Ordering::Relaxed);
    if TOLAK_SEEN[i].swap(1, Ordering::AcqRel) == 0 {
        TOLAK_IDX[i].store(index, Ordering::Relaxed);
        TOLAK_GEN[i].store(gen, Ordering::Relaxed);
        TOLAK_ARG[i].store(arg, Ordering::Relaxed);
    }
}

/// Diagnostik: jumlah penolakan pada titik masuk `fn_id` (1..15).
#[no_mangle]
pub extern "C" fn klatos_graf_tolak(fn_id: u32) -> u32 {
    TOLAK[(fn_id & 0xF) as usize].load(Ordering::Relaxed)
}

/// Diagnostik: jumlah penolakan SEJAK snapshot terakhir (produksi saja).
#[no_mangle]
pub extern "C" fn klatos_graf_tolak_prod(fn_id: u32) -> u32 {
    let i = (fn_id & 0xF) as usize;
    TOLAK[i]
        .load(Ordering::Relaxed)
        .saturating_sub(TOLAK_BASE[i].load(Ordering::Relaxed))
}

/// Diagnostik: ambil baseline dan mulai merekam detail lagi. Dipanggil
/// SESUDAH seluruh selftest selesai agar penolakan uji tidak tercampur.
#[no_mangle]
pub extern "C" fn klatos_graf_tolak_snapshot() -> u32 {
    let mut i = 0usize;
    while i < 16 {
        TOLAK_BASE[i].store(TOLAK[i].load(Ordering::Relaxed), Ordering::Relaxed);
        TOLAK_SEEN[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    1
}

/// Diagnostik: detail penolakan pertama titik masuk `fn_id`.
/// `sel`: 0=index 1=gen 2=arg.
#[no_mangle]
pub extern "C" fn klatos_graf_tolak_detail(fn_id: u32, sel: u32) -> u32 {
    let i = (fn_id & 0xF) as usize;
    match sel {
        0 => TOLAK_IDX[i].load(Ordering::Relaxed),
        1 => TOLAK_GEN[i].load(Ordering::Relaxed),
        _ => TOLAK_ARG[i].load(Ordering::Relaxed),
    }
}

// ---------------------------------------------------------------------------
// P1.57 — SAKSI PRODUKSI: TULIS BIDANG BENANG (prioritas + afinitas)
// ---------------------------------------------------------------------------
//
// MENGAPA ADA
//   Mandat §15 (P1.57) menuntut jalur produksi "perubahan prioritas" dan
//   "perubahan afinitas" terbukti benar-benar melewati otoritas Rust SESUDAH
//   `/init`, bukan hanya dari selftest. `klatos_graf_benang_taruh` adalah
//   SATU-SATUNYA pintu tulis bidang benang; `Thread::set_priority`
//   (`TW_PRIORITY`) dan `Thread::set_affinity` (`TW_AFFINITY`) memanggilnya.
//
// MENGAPA SNAPSHOT
//   Selftest graf juga memanggil `klatos_graf_benang_taruh`. Karena itu saksi
//   memakai pola F-P149-05 yang sama: `klatos_graf_tulis_snapshot()` menyalin
//   nilai sekarang sebagai DASAR, dan `klatos_graf_tulis(which)` mengembalikan
//   DELTA sesudah snapshot — jadi hanya aktivitas produksi yang terhitung.
//
// Indeks tabel = `which & 0xF` (2 = prioritas, 3 = afinitas, 4..9 = sinyal).
static TULIS: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
static TULIS_BASE: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];

#[inline]
fn bump_tulis(which: u32) {
    TULIS[(which & 0xF) as usize].fetch_add(1, Ordering::Relaxed);
}

/// Diagnostik: delta tulis bidang `which` sejak snapshot terakhir (produksi saja).
#[no_mangle]
pub extern "C" fn klatos_graf_tulis(which: u32) -> u32 {
    let i = (which & 0xF) as usize;
    TULIS[i]
        .load(Ordering::Relaxed)
        .saturating_sub(TULIS_BASE[i].load(Ordering::Relaxed))
}

/// Ambil baseline penghitung tulis bidang; dipanggil SESUDAH seluruh selftest.
#[no_mangle]
pub extern "C" fn klatos_graf_tulis_snapshot() -> u32 {
    let mut i = 0usize;
    while i < 16 {
        TULIS_BASE[i].store(TULIS[i].load(Ordering::Relaxed), Ordering::Relaxed);
        i += 1;
    }
    1
}

// ---------------------------------------------------------------------------
// Tabel proses — array paralel, atomik
// ---------------------------------------------------------------------------

static P_GEN: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_PID: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_STATE: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_FLAGS: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_TCOUNT: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_TERMST: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_TERMSG: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_HIDUP: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
// P1.73 — raw scheduler clock_t accounting. Units are coarse scheduler ticks,
// intentionally distinct from WS2 nanosecond elapsed scheduling counters.
static P_TICKS_USER: [AtomicU64; MAX_PROSES] = [const { AtomicU64::new(0) }; MAX_PROSES];
static P_TICKS_KERNEL: [AtomicU64; MAX_PROSES] = [const { AtomicU64::new(0) }; MAX_PROSES];
static P_TICKS_DEAD_USER: [AtomicU64; MAX_PROSES] = [const { AtomicU64::new(0) }; MAX_PROSES];
static P_TICKS_DEAD_KERNEL: [AtomicU64; MAX_PROSES] = [const { AtomicU64::new(0) }; MAX_PROSES];
// FASE-4 BATCH 03 — state JANJI (pledge) per-proses. Sebelumnya keempat bidang
// ini tinggal di `Process::ProtectedValues` (C++): C++ memegang state, Rust
// hanya memutuskan transisi. Kini Rust memiliki state-nya (array per-slot,
// sejajar dengan tabel proses), dan C++ hanya membaca/menulis lewat pintu
// `klatos_graf_proses_ambil`/`_taruh`/`_salin_janji`.
static P_HAS_PROMISES: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_PROMISES: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_HAS_EXECPROMISES: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];
static P_EXECPROMISES: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];

// ---------------------------------------------------------------------------
// Tabel benang — array paralel, atomik
// ---------------------------------------------------------------------------

static T_GEN: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_TID: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_PID: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_STATE: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_FLAGS: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_PRIO: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_AFF: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_PSIG: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_SMASK: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_HSIG: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_ISIG: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_HIDUP: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
// WS2 — per-thread scheduling bookkeeping. The C++ Thread object remains the
// mechanism/lifetime owner; Rust owns these counters and the running interval.
static T_LAST_TIME: [AtomicU64; MAX_BENANG] = [const { AtomicU64::new(0) }; MAX_BENANG];
static T_LAST_VALID: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];
static T_TIME_USER: [AtomicU64; MAX_BENANG] = [const { AtomicU64::new(0) }; MAX_BENANG];
static T_TIME_KERNEL: [AtomicU64; MAX_BENANG] = [const { AtomicU64::new(0) }; MAX_BENANG];
static T_TIMES_SCHEDULED: [AtomicU32; MAX_BENANG] = [const { AtomicU32::new(0) }; MAX_BENANG];

/// Kunci HANYA untuk alokasi/pelepasan slot (bukan untuk baca/tulis keadaan).
/// Jalur baca/tulis keadaan lock-free supaya aman dipanggil dari konteks
/// interupsi dan dari bawah `g_scheduler_lock`.
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

/// Jumlah proses/benang hidup PRODUKSI (dijaga inkremental).
static HIDUP_P: AtomicU32 = AtomicU32::new(0);
static HIDUP_B: AtomicU32 = AtomicU32::new(0);
static PUNCAK_P: AtomicU32 = AtomicU32::new(0);
static PUNCAK_B: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// Resolusi handle
// ---------------------------------------------------------------------------

#[inline]
fn p_sah(index: u32, gen: u32) -> bool {
    let i = index as usize;
    gen != 0 && i < MAX_PROSES && P_HIDUP[i].load(Ordering::Acquire) != 0
        && P_GEN[i].load(Ordering::Relaxed) == gen
}

#[inline]
fn t_sah(index: u32, gen: u32) -> bool {
    let i = index as usize;
    gen != 0 && i < MAX_BENANG && T_HIDUP[i].load(Ordering::Acquire) != 0
        && T_GEN[i].load(Ordering::Relaxed) == gen
}

// ---------------------------------------------------------------------------
// F-P149-07 — KEABSAHAN ADALAH KANAL TERSENDIRI
// ---------------------------------------------------------------------------
//
// Bug historis: pembungkus C-ABI memutuskan "ditolak" dengan
// `r == TAK_SAH`. Itu keliru karena NILAI bidang yang SAH dapat sama dengan
// `TAK_SAH` (0xFFFF_FFFF): afinitas bawaan benang memang bernilai
// `AFINITAS_BAWAAN == 0xFFFF_FFFF == TAK_SAH`. Akibatnya setiap pembacaan
// afinitas bawaan dari produksi tercatat sebagai penolakan handle palsu
// (`t10`/`t11`), dan telemetri penolakan berbohong.
//
// Perbaikan: keabsahan dievaluasi lewat `p_sah`/`t_sah` DAN validitas `which`
// (fungsi di bawah) — bukan lewat nilai yang dikembalikan. Kanal nilai
// (mengembalikan `TAK_SAH` untuk `which` tak dikenal) tetap ada, tetapi tidak
// pernah dipakai untuk memutuskan penolakan.

/// `which` sah untuk pembacaan bidang proses (0..=6, ditambah 7..=10 = janji).
#[inline]
fn p_which_sah(which: u32) -> bool {
    which <= PW_MAKS
}

/// `which` sah untuk pembacaan bidang benang.
#[inline]
fn t_baca_which_sah(which: u32) -> bool {
    which == 0
        || which == 1
        || which == TW_PRIORITY
        || which == TW_AFFINITY
        || which == TW_PENDING_SIGNALS
        || which == TW_SIGNAL_MASK
        || which == TW_HANDLED_SIGNAL
        || which == 7
        || which == 8
        || which == TW_INTR_SIGNAL
}

/// `which` sah untuk penulisan bidang benang.
#[inline]
fn t_tulis_which_sah(which: u32) -> bool {
    which == TW_PRIORITY
        || which == TW_AFFINITY
        || which == TW_PENDING_SIGNALS
        || which == TW_SIGNAL_MASK
        || which == TW_HANDLED_SIGNAL
        || which == TW_INTR_SIGNAL
        || which == 7
}

// ---------------------------------------------------------------------------
// Inti proses
// ---------------------------------------------------------------------------

fn p_daftar_impl(pid: u32, is_kernel: u32, lo: usize, hi: usize) -> u64 {
    lock();
    let mut slot: Option<usize> = None;
    let mut i = lo;
    while i < hi {
        if P_HIDUP[i].load(Ordering::Relaxed) == 0 {
            slot = Some(i);
            break;
        }
        i += 1;
    }
    let hasil = if let Some(i) = slot {
        let g = P_GEN[i].load(Ordering::Relaxed).wrapping_add(1);
        if g == 0 {
            P_HIDUP[i].store(0, Ordering::Release);
            0u64
        } else {
            P_PID[i].store(pid, Ordering::Relaxed);
            P_STATE[i].store(PR_RUNNING, Ordering::Relaxed);
            P_FLAGS[i].store(if is_kernel != 0 { PF_KERNEL } else { 0 }, Ordering::Relaxed);
            P_TCOUNT[i].store(0, Ordering::Relaxed);
            P_TERMST[i].store(0, Ordering::Relaxed);
            P_TERMSG[i].store(0, Ordering::Relaxed);
            P_TICKS_USER[i].store(0, Ordering::Relaxed);
            P_TICKS_KERNEL[i].store(0, Ordering::Relaxed);
            P_TICKS_DEAD_USER[i].store(0, Ordering::Relaxed);
            P_TICKS_DEAD_KERNEL[i].store(0, Ordering::Relaxed);
            // Slot dapat dipakai ulang: state janji WAJIB bersih, kalau tidak
            // proses baru mewarisi janji proses mati.
            P_HAS_PROMISES[i].store(0, Ordering::Relaxed);
            P_PROMISES[i].store(0, Ordering::Relaxed);
            P_HAS_EXECPROMISES[i].store(0, Ordering::Relaxed);
            P_EXECPROMISES[i].store(0, Ordering::Relaxed);
            P_GEN[i].store(g, Ordering::Relaxed);
            P_HIDUP[i].store(1, Ordering::Release);
            ((g as u64) << 32) | (i as u64)
        }
    } else {
        0u64
    };
    unlock();
    // Penghitung HANYA untuk rentang produksi.
    if hasil != 0 && ((hasil & 0xFFFF_FFFF) as usize) < BATAS_PROSES {
        let h = HIDUP_P.load(Ordering::Relaxed) + 1;
        HIDUP_P.store(h, Ordering::Relaxed);
        if h > PUNCAK_P.load(Ordering::Relaxed) {
            PUNCAK_P.store(h, Ordering::Relaxed);
        }
    }
    hasil
}

fn p_lepas_impl(index: u32, gen: u32) -> u32 {
    if !p_sah(index, gen) {
        return 0;
    }
    lock();
    if !p_sah(index, gen) {
        unlock();
        return 0;
    }
    let i = index as usize;
    P_HIDUP[i].store(0, Ordering::Release);
    P_FLAGS[i].store(0, Ordering::Relaxed);
    P_TCOUNT[i].store(0, Ordering::Relaxed);
    if i < BATAS_PROSES {
        let h = HIDUP_P.load(Ordering::Relaxed);
        if h > 0 {
            HIDUP_P.store(h - 1, Ordering::Relaxed);
        }
    }
    unlock();
    1
}

fn p_ambil_impl(index: u32, gen: u32, which: u32) -> u32 {
    if !p_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    match which {
        0 => P_STATE[i].load(Ordering::Relaxed),
        1 => P_FLAGS[i].load(Ordering::Relaxed),
        2 => P_TCOUNT[i].load(Ordering::Relaxed),
        3 => P_TERMST[i].load(Ordering::Relaxed),
        4 => P_TERMSG[i].load(Ordering::Relaxed),
        5 => P_PID[i].load(Ordering::Relaxed),
        6 => (P_FLAGS[i].load(Ordering::Relaxed) & PF_KERNEL) >> 1,
        PW_HAS_PROMISES => P_HAS_PROMISES[i].load(Ordering::Relaxed),
        PW_PROMISES => P_PROMISES[i].load(Ordering::Relaxed),
        PW_HAS_EXECPROMISES => P_HAS_EXECPROMISES[i].load(Ordering::Relaxed),
        PW_EXECPROMISES => P_EXECPROMISES[i].load(Ordering::Relaxed),
        _ => TAK_SAH,
    }
}

/// FASE-4 BATCH 03 — tulis SATU bidang janji proses. Mengembalikan nilai LAMA.
/// Hanya `which` 7..=10 yang diterima; selain itu `TAK_SAH` (tanpa efek).
fn p_taruh_impl(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    if !p_sah(index, gen) {
        return TAK_SAH;
    }
    if !(PW_HAS_PROMISES..=PW_EXECPROMISES).contains(&which) {
        return TAK_SAH;
    }
    let i = index as usize;
    let sel = match which {
        PW_HAS_PROMISES => &P_HAS_PROMISES[i],
        PW_PROMISES => &P_PROMISES[i],
        PW_HAS_EXECPROMISES => &P_HAS_EXECPROMISES[i],
        _ => &P_EXECPROMISES[i],
    };
    sel.swap(val, Ordering::AcqRel)
}

/// FASE-4 BATCH 03 — salin SELURUH state janji dari proses `from` ke proses `to`
/// (dipakai `fork`). Mengembalikan 1 bila kedua handle sah, 0 bila tidak.
fn p_salin_janji_impl(
    from_index: u32,
    from_gen: u32,
    to_index: u32,
    to_gen: u32,
) -> u32 {
    if !p_sah(from_index, from_gen) || !p_sah(to_index, to_gen) {
        return 0;
    }
    let a = from_index as usize;
    let b = to_index as usize;
    P_HAS_PROMISES[b].store(P_HAS_PROMISES[a].load(Ordering::Relaxed), Ordering::Relaxed);
    P_PROMISES[b].store(P_PROMISES[a].load(Ordering::Relaxed), Ordering::Relaxed);
    P_HAS_EXECPROMISES[b].store(P_HAS_EXECPROMISES[a].load(Ordering::Relaxed), Ordering::Relaxed);
    P_EXECPROMISES[b].store(P_EXECPROMISES[a].load(Ordering::Relaxed), Ordering::Relaxed);
    1
}

fn p_set_keadaan_impl(index: u32, gen: u32, state: u32) -> u32 {
    if !p_sah(index, gen) || state > PR_DEAD {
        return TAK_SAH;
    }
    let i = index as usize;
    P_STATE[i].swap(state, Ordering::AcqRel)
}

/// `die()`: CAS `Running -> Dying`. Mengembalikan 1 bila transisi terjadi.
fn p_mulai_mati_impl(index: u32, gen: u32) -> u32 {
    if !p_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    match P_STATE[i].compare_exchange(PR_RUNNING, PR_DYING, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// `set_stopped(x)`: menetapkan bit STOP ke `val`, mengembalikan nilai LAMA.
fn p_stop_impl(index: u32, gen: u32, val: u32) -> u32 {
    if !p_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    let baru = if val != 0 { PF_STOP } else { 0 };
    let mut lama = P_FLAGS[i].load(Ordering::Acquire);
    loop {
        let target = (lama & !PF_STOP) | baru;
        match P_FLAGS[i].compare_exchange_weak(
            lama,
            target,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(now) => lama = now,
        }
    }
    (lama & PF_STOP) >> 0
}

fn p_benang_tambah_impl(index: u32, gen: u32) -> u32 {
    if !p_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    P_TCOUNT[i].fetch_add(1, Ordering::AcqRel) + 1
}

fn p_benang_kurang_impl(index: u32, gen: u32) -> u32 {
    if !p_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    P_TCOUNT[i].fetch_sub(1, Ordering::AcqRel)
}

fn p_set_terminasi_impl(index: u32, gen: u32, status: u32, signal: u32) -> u32 {
    if !p_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    P_TERMST[i].store(status & 0xFF, Ordering::Relaxed);
    P_TERMSG[i].store(signal & 0xFF, Ordering::Relaxed);
    1
}

// ---------------------------------------------------------------------------
// Inti benang
// ---------------------------------------------------------------------------

fn t_daftar_impl(tid: u32, pid: u32, lo: usize, hi: usize) -> u64 {
    lock();
    let mut slot: Option<usize> = None;
    let mut i = lo;
    while i < hi {
        if T_HIDUP[i].load(Ordering::Relaxed) == 0 {
            slot = Some(i);
            break;
        }
        i += 1;
    }
    let hasil = if let Some(i) = slot {
        let g = T_GEN[i].load(Ordering::Relaxed).wrapping_add(1);
        if g == 0 {
            T_HIDUP[i].store(0, Ordering::Release);
            0u64
        } else {
            T_TID[i].store(tid, Ordering::Relaxed);
            T_PID[i].store(pid, Ordering::Relaxed);
            T_STATE[i].store(TH_INVALID, Ordering::Relaxed);
            T_FLAGS[i].store(TF_JOINABLE, Ordering::Relaxed);
            T_PRIO[i].store(0, Ordering::Relaxed);
            T_AFF[i].store(AFINITAS_BAWAAN, Ordering::Relaxed);
            T_PSIG[i].store(0, Ordering::Relaxed);
            T_SMASK[i].store(0, Ordering::Relaxed);
            T_HSIG[i].store(0, Ordering::Relaxed);
            T_ISIG[i].store(0, Ordering::Relaxed);
            T_LAST_TIME[i].store(0, Ordering::Relaxed);
            T_LAST_VALID[i].store(0, Ordering::Relaxed);
            T_TIME_USER[i].store(0, Ordering::Relaxed);
            T_TIME_KERNEL[i].store(0, Ordering::Relaxed);
            T_TIMES_SCHEDULED[i].store(0, Ordering::Relaxed);
            T_GEN[i].store(g, Ordering::Relaxed);
            T_HIDUP[i].store(1, Ordering::Release);
            ((g as u64) << 32) | (i as u64)
        }
    } else {
        0u64
    };
    unlock();
    if hasil != 0 && ((hasil & 0xFFFF_FFFF) as usize) < BATAS_BENANG {
        let h = HIDUP_B.load(Ordering::Relaxed) + 1;
        HIDUP_B.store(h, Ordering::Relaxed);
        if h > PUNCAK_B.load(Ordering::Relaxed) {
            PUNCAK_B.store(h, Ordering::Relaxed);
        }
    }
    hasil
}

fn t_lepas_impl(index: u32, gen: u32) -> u32 {
    if !t_sah(index, gen) {
        return 0;
    }
    lock();
    if !t_sah(index, gen) {
        unlock();
        return 0;
    }
    let i = index as usize;
    T_HIDUP[i].store(0, Ordering::Release);
    T_FLAGS[i].store(0, Ordering::Relaxed);
    if i < BATAS_BENANG {
        let h = HIDUP_B.load(Ordering::Relaxed);
        if h > 0 {
            HIDUP_B.store(h - 1, Ordering::Relaxed);
        }
    }
    unlock();
    1
}

fn t_ambil_impl(index: u32, gen: u32, which: u32) -> u32 {
    if !t_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    match which {
        0 => T_STATE[i].load(Ordering::Relaxed),
        1 => T_FLAGS[i].load(Ordering::Relaxed),
        TW_PRIORITY => T_PRIO[i].load(Ordering::Relaxed),
        TW_AFFINITY => T_AFF[i].load(Ordering::Relaxed),
        TW_PENDING_SIGNALS => T_PSIG[i].load(Ordering::Relaxed),
        TW_SIGNAL_MASK => T_SMASK[i].load(Ordering::Relaxed),
        TW_HANDLED_SIGNAL => T_HSIG[i].load(Ordering::Relaxed),
        7 => T_PID[i].load(Ordering::Relaxed),
        8 => T_TID[i].load(Ordering::Relaxed),
        TW_INTR_SIGNAL => T_ISIG[i].load(Ordering::Relaxed),
        _ => TAK_SAH,
    }
}

fn t_taruh_impl(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    if !t_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    match which {
        TW_PRIORITY => T_PRIO[i].swap(val, Ordering::AcqRel),
        TW_AFFINITY => T_AFF[i].swap(val, Ordering::AcqRel),
        TW_PENDING_SIGNALS => T_PSIG[i].swap(val, Ordering::AcqRel),
        TW_SIGNAL_MASK => T_SMASK[i].swap(val, Ordering::AcqRel),
        TW_HANDLED_SIGNAL => T_HSIG[i].swap(val & 0xFF, Ordering::AcqRel),
        TW_INTR_SIGNAL => T_ISIG[i].swap(val & 0xFF, Ordering::AcqRel),
        7 => T_PID[i].swap(val, Ordering::AcqRel),
        _ => TAK_SAH,
    }
}

fn t_set_keadaan_impl(index: u32, gen: u32, state: u32) -> u32 {
    if !t_sah(index, gen) || state >= TH_N_STATE {
        return TAK_SAH;
    }
    let i = index as usize;
    T_STATE[i].swap(state, Ordering::AcqRel)
}

fn t_set_flag_impl(index: u32, gen: u32, bit: u32, val: u32) -> u32 {
    if !t_sah(index, gen) || (bit & !TF_SEMUA) != 0 || bit == 0 {
        return TAK_SAH;
    }
    let i = index as usize;
    if val != 0 {
        T_FLAGS[i].fetch_or(bit, Ordering::AcqRel)
    } else {
        T_FLAGS[i].fetch_and(!bit, Ordering::AcqRel)
    }
}

/// Keputusan `block_result()` historis (invarian B2).
fn t_hasil_blokir_impl(index: u32, gen: u32) -> u32 {
    if !t_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    let f = T_FLAGS[i].load(Ordering::Relaxed);
    if (f & TF_INTR_DEATH) != 0 {
        return BR_INTR_DEATH;
    }
    if T_ISIG[i].load(Ordering::Relaxed) != 0 {
        return BR_INTR_SIGNAL;
    }
    if (f & TF_DID_TIMEOUT) != 0 {
        return BR_INTR_TIMEOUT;
    }
    BR_WOKE
}

// ---------------------------------------------------------------------------
// P1.56 — akses internal untuk otoritas antrean jadwal (`jadwal_antrean`)
// ---------------------------------------------------------------------------
//
// Modul antrean jadwal membutuhkan KELAYAKAN benang (`TF_ACTIVE`) dan
// AFINITAS-nya (`TW_AFFINITY`) saat memilih benang berikutnya. Supaya tidak ada
// otoritas ganda (mandat §21), nilai itu DIBACA dari sini — bukan disalin.
// Fungsi di bawah sengaja `pub(crate)` dan TIDAK menaikkan penghitung apa pun
// (sama seperti selftest internal), karena dipanggil dari jalur panas penjadwal
// dan dari matriks uji.

/// Keabsahan handle benang (tanpa penghitung).
#[inline]
pub(crate) fn benang_sah(index: u32, gen: u32) -> bool {
    t_sah(index, gen)
}

/// Baca satu bit bendera benang (tanpa penghitung); 0 bila handle tak sah.
#[inline]
pub(crate) fn benang_flag(index: u32, gen: u32, bit: u32) -> u32 {
    if !t_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    ((T_FLAGS[i].load(Ordering::Relaxed) & bit) != 0) as u32
}

/// Baca satu bidang benang (tanpa penghitung); 0 bila handle tak sah.
#[inline]
pub(crate) fn benang_ambil(index: u32, gen: u32, which: u32) -> u32 {
    if !t_sah(index, gen) {
        return 0;
    }
    t_ambil_impl(index, gen, which)
}

/// Rentang cadangan benang (untuk selftest lintas modul).
pub(crate) const LO_CADANGAN_BENANG: usize = BATAS_BENANG;

/// Kosongkan arena cadangan benang.
pub(crate) fn benang_cadangan_reset() {
    reset_cadangan();
}

/// Daftarkan benang di arena cadangan (tanpa penghitung).
pub(crate) fn benang_cadangan_daftar() -> u64 {
    t_daftar_impl(0, 0, LO_CADANGAN_BENANG, MAX_BENANG)
}

/// Lepaskan benang arena cadangan (tanpa penghitung).
pub(crate) fn benang_cadangan_lepas(index: u32, gen: u32) -> u32 {
    t_lepas_impl(index, gen)
}

/// Set/bersihkan bit bendera benang arena cadangan (tanpa penghitung).
pub(crate) fn benang_cadangan_set_flag(index: u32, gen: u32, bit: u32, val: u32) -> u32 {
    t_set_flag_impl(index, gen, bit, val)
}

/// Tulis bidang benang arena cadangan (tanpa penghitung).
pub(crate) fn benang_cadangan_taruh(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    t_taruh_impl(index, gen, which, val)
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — PROSES
// ---------------------------------------------------------------------------

/// Daftarkan proses. Mengembalikan `(generation << 32) | index`, atau 0 bila
/// tabel penuh / generasi membungkus.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_daftar(pid: u32, is_kernel: u32) -> u64 {
    bump(Counter::ProdDaftarProses);
    p_daftar_impl(pid, is_kernel, 0, BATAS_PROSES)
}

/// Lepaskan proses. 1 bila dihapus, 0 bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_lepas(index: u32, gen: u32) -> u32 {
    let r = p_lepas_impl(index, gen);
    if r == 0 {
        tolak(1, index, gen, 0);
    } else {
        bump(Counter::ProdLepasProses);
    }
    r
}

/// Baca satu bidang proses.
///
/// F-P149-07: keabsahan = kanal tersendiri (`p_sah` + `p_which_sah`), bukan
/// `r == TAK_SAH`. Nilai bidang yang sah tidak pernah dianggap penolakan.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_ambil(index: u32, gen: u32, which: u32) -> u32 {
    if !p_sah(index, gen) {
        tolak(2, index, gen, which);
        return TAK_SAH;
    }
    if !p_which_sah(which) {
        // Handle sah tetapi `which` di luar rentang: bukan penolakan handle.
        return TAK_SAH;
    }
    p_ambil_impl(index, gen, which)
}

/// FASE-4 BATCH 03 — tulis satu bidang proses (`which` 7..=10 = janji).
/// Mengembalikan nilai LAMA, atau `TAK_SAH` bila handle/`which` tidak sah.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_taruh(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    if !p_sah(index, gen) {
        tolak(23, index, gen, which);
        return TAK_SAH;
    }
    if !(PW_HAS_PROMISES..=PW_EXECPROMISES).contains(&which) {
        return TAK_SAH;
    }
    p_taruh_impl(index, gen, which, val)
}

/// FASE-4 BATCH 03 — salin state janji dari satu proses ke proses lain (`fork`).
/// 1 bila berhasil, 0 bila salah satu handle tidak sah.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_salin_janji(
    from_index: u32,
    from_gen: u32,
    to_index: u32,
    to_gen: u32,
) -> u32 {
    let r = p_salin_janji_impl(from_index, from_gen, to_index, to_gen);
    if r == 0 {
        tolak(24, from_index, from_gen, to_index);
    }
    r
}

/// Tulis keadaan proses; mengembalikan keadaan LAMA.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_set_keadaan(index: u32, gen: u32, state: u32) -> u32 {
    if !p_sah(index, gen) {
        tolak(3, index, gen, state);
        return TAK_SAH;
    }
    if state > PR_DEAD {
        return TAK_SAH;
    }
    p_set_keadaan_impl(index, gen, state)
}

/// CAS `Running -> Dying` (`Process::die()`). 1 bila transisi terjadi.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_mulai_mati(index: u32, gen: u32) -> u32 {
    let r = p_mulai_mati_impl(index, gen);
    if r == 1 {
        bump(Counter::ProdMati);
    } else if !p_sah(index, gen) {
        tolak(4, index, gen, 0);
    }
    r
}

/// `set_stopped(val)` — menetapkan bit STOP, mengembalikan nilai LAMA.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_stop(index: u32, gen: u32, val: u32) -> u32 {
    if !p_sah(index, gen) {
        tolak(5, index, gen, val);
        return TAK_SAH;
    }
    p_stop_impl(index, gen, val)
}

// ---------------------------------------------------------------------------
// P1.73 — akuntansi clock_t proses (satuan tick scheduler)
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_graf_proses_tick(index: u32, gen: u32, is_kernel: u32) -> u32 {
    if !p_sah(index, gen) {
        tolak(20, index, gen, is_kernel);
        return 0;
    }
    let i = index as usize;
    if is_kernel != 0 {
        P_TICKS_KERNEL[i].fetch_add(1, Ordering::Relaxed);
    } else {
        P_TICKS_USER[i].fetch_add(1, Ordering::Relaxed);
    }
    1
}

#[no_mangle]
pub extern "C" fn klatos_graf_proses_adopt_dead(index: u32, gen: u32, user: u64, kernel: u64) -> u32 {
    if !p_sah(index, gen) {
        tolak(21, index, gen, 0);
        return 0;
    }
    let i = index as usize;
    P_TICKS_DEAD_USER[i].fetch_add(user, Ordering::Relaxed);
    P_TICKS_DEAD_KERNEL[i].fetch_add(kernel, Ordering::Relaxed);
    1
}

#[no_mangle]
pub extern "C" fn klatos_graf_proses_ticks(index: u32, gen: u32, which: u32) -> u64 {
    if !p_sah(index, gen) || which > 3 {
        tolak(22, index, gen, which);
        return 0;
    }
    let i = index as usize;
    match which {
        0 => P_TICKS_USER[i].load(Ordering::Relaxed),
        1 => P_TICKS_KERNEL[i].load(Ordering::Relaxed),
        2 => P_TICKS_DEAD_USER[i].load(Ordering::Relaxed),
        _ => P_TICKS_DEAD_KERNEL[i].load(Ordering::Relaxed),
    }
}

/// Tambah penghitung benang; mengembalikan jumlah BARU.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_benang_tambah(index: u32, gen: u32) -> u32 {
    if !p_sah(index, gen) {
        tolak(6, index, gen, 0);
        return TAK_SAH;
    }
    p_benang_tambah_impl(index, gen)
}

/// Kurangi penghitung benang; mengembalikan jumlah LAMA.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_benang_kurang(index: u32, gen: u32) -> u32 {
    if !p_sah(index, gen) {
        tolak(7, index, gen, 0);
        return TAK_SAH;
    }
    p_benang_kurang_impl(index, gen)
}

/// Set status + sinyal terminasi.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_set_terminasi(
    index: u32,
    gen: u32,
    status: u32,
    signal: u32,
) -> u32 {
    let r = p_set_terminasi_impl(index, gen, status, signal);
    if r == 0 {
        tolak(8, index, gen, signal);
    }
    r
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — BENANG
// ---------------------------------------------------------------------------

/// Daftarkan benang. Mengembalikan `(generation << 32) | index`, atau 0.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_daftar(tid: u32, pid: u32) -> u64 {
    bump(Counter::ProdDaftarBenang);
    t_daftar_impl(tid, pid, 0, BATAS_BENANG)
}

/// Lepaskan benang. 1 bila dihapus, 0 bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_lepas(index: u32, gen: u32) -> u32 {
    let r = t_lepas_impl(index, gen);
    if r == 0 {
        tolak(9, index, gen, 0);
    } else {
        bump(Counter::ProdLepasBenang);
    }
    r
}

/// Baca satu bidang benang.
///
/// F-P149-07: keabsahan = kanal tersendiri (`t_sah` + `t_baca_which_sah`).
/// `TW_AFFINITY` bawaan bernilai `0xFFFF_FFFF == TAK_SAH`; pembacaan itu SAH
/// dan TIDAK boleh dihitung sebagai penolakan handle.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_ambil(index: u32, gen: u32, which: u32) -> u32 {
    if !t_sah(index, gen) {
        tolak(10, index, gen, which);
        return TAK_SAH;
    }
    if !t_baca_which_sah(which) {
        return TAK_SAH;
    }
    t_ambil_impl(index, gen, which)
}

/// Tulis satu bidang benang; mengembalikan nilai LAMA.
///
/// F-P149-07: keabsahan = kanal tersendiri; nilai LAMA yang kebetulan sama
/// dengan `TAK_SAH` (afinitas bawaan) bukan penolakan.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_taruh(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    if !t_sah(index, gen) {
        tolak(11, index, gen, which);
        return TAK_SAH;
    }
    if !t_tulis_which_sah(which) {
        return TAK_SAH;
    }
    // P1.57 — saksi produksi: hitung setiap tulis bidang benang yang SAH
    // (prioritas/afinitas/sinyal). Selftest memakai `t_taruh_impl` langsung,
    // jadi yang tercatat di sini adalah jalur C-ABI; saksi boot memakai
    // `klatos_graf_tulis_snapshot()` untuk memisahkan produksi dari selftest.
    let r = t_taruh_impl(index, gen, which, val);
    bump_tulis(which);
    r
}

/// Set keadaan benang; mengembalikan keadaan LAMA.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_set_keadaan(index: u32, gen: u32, state: u32) -> u32 {
    if !t_sah(index, gen) {
        tolak(12, index, gen, state);
        return TAK_SAH;
    }
    if state >= TH_N_STATE {
        return TAK_SAH;
    }
    let r = t_set_keadaan_impl(index, gen, state);
    if r == TAK_SAH {
        // Balapan: handle dilepas di tengah panggilan.
        tolak(12, index, gen, state);
    } else {
        bump(Counter::ProdTransisi);
    }
    r
}

/// Set/bersihkan satu bit bendera; mengembalikan bendera LAMA.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_set_flag(index: u32, gen: u32, bit: u32, val: u32) -> u32 {
    if !t_sah(index, gen) {
        tolak(13, index, gen, bit);
        return TAK_SAH;
    }
    if bit == 0 || (bit & !TF_SEMUA) != 0 {
        return TAK_SAH;
    }
    let r = t_set_flag_impl(index, gen, bit, val);
    if r == TAK_SAH {
        tolak(13, index, gen, bit);
    } else {
        bump(Counter::ProdSetFlag);
    }
    r
}

/// Baca satu bit bendera benang (0/1).
#[no_mangle]
pub extern "C" fn klatos_graf_benang_flag(index: u32, gen: u32, bit: u32) -> u32 {
    if !t_sah(index, gen) {
        tolak(14, index, gen, bit);
        return TAK_SAH;
    }
    let i = index as usize;
    ((T_FLAGS[i].load(Ordering::Relaxed) & bit) != 0) as u32
}

/// Keputusan `block_result()`.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_hasil_blokir(index: u32, gen: u32) -> u32 {
    if !t_sah(index, gen) {
        tolak(15, index, gen, 0);
        return TAK_SAH;
    }
    t_hasil_blokir_impl(index, gen)
}

// ---------------------------------------------------------------------------
// WS2 — akuntansi waktu benang
// ---------------------------------------------------------------------------

/// Update running interval and return the elapsed delta for the global clock.
/// Rust owns the per-thread interval/counters; C++ only feeds the scheduler
/// clock and applies the returned delta to the existing global aggregate.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_update_time(
    index: u32,
    gen: u32,
    now: u64,
    is_kernel: u32,
    no_longer_running: u32,
) -> u64 {
    if !t_sah(index, gen) {
        tolak(16, index, gen, 0);
        return 0;
    }
    let i = index as usize;
    let delta = if T_LAST_VALID[i].load(Ordering::Acquire) != 0 {
        let last = T_LAST_TIME[i].load(Ordering::Relaxed);
        now.abs_diff(last)
    } else {
        0
    };
    if delta != 0 {
        if is_kernel != 0 {
            T_TIME_KERNEL[i].fetch_add(delta, Ordering::Relaxed);
        } else {
            T_TIME_USER[i].fetch_add(delta, Ordering::Relaxed);
        }
    }
    if no_longer_running != 0 {
        T_LAST_VALID[i].store(0, Ordering::Release);
    } else {
        T_LAST_TIME[i].store(now, Ordering::Relaxed);
        T_LAST_VALID[i].store(1, Ordering::Release);
    }
    delta
}

/// Read Rust-owned per-thread scheduled time. `which=0` user, `which=1`
/// kernel; invalid handles/fields return `TAK_SAH`.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_time(index: u32, gen: u32, which: u32) -> u64 {
    if !t_sah(index, gen) || which > 1 {
        tolak(17, index, gen, which);
        return TAK_SAH as u64;
    }
    let i = index as usize;
    if which == 0 {
        T_TIME_USER[i].load(Ordering::Relaxed)
    } else {
        T_TIME_KERNEL[i].load(Ordering::Relaxed)
    }
}

/// Record a completed schedule. Returns the new count.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_did_schedule(index: u32, gen: u32) -> u32 {
    if !t_sah(index, gen) {
        tolak(18, index, gen, 0);
        return TAK_SAH;
    }
    T_TIMES_SCHEDULED[index as usize].fetch_add(1, Ordering::Relaxed) + 1
}

#[no_mangle]
pub extern "C" fn klatos_graf_benang_times_scheduled(index: u32, gen: u32) -> u32 {
    if !t_sah(index, gen) {
        tolak(19, index, gen, 0);
        return TAK_SAH;
    }
    T_TIMES_SCHEDULED[index as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — audit
// ---------------------------------------------------------------------------

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_graf_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Proses hidup sekarang (rentang produksi).
#[no_mangle]
pub extern "C" fn klatos_graf_proses_hidup() -> u32 {
    HIDUP_P.load(Ordering::Relaxed)
}

/// Benang hidup sekarang (rentang produksi).
#[no_mangle]
pub extern "C" fn klatos_graf_benang_hidup() -> u32 {
    HIDUP_B.load(Ordering::Relaxed)
}

/// Puncak proses hidup.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_puncak() -> u32 {
    PUNCAK_P.load(Ordering::Relaxed)
}

/// Puncak benang hidup.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_puncak() -> u32 {
    PUNCAK_B.load(Ordering::Relaxed)
}

/// Kapasitas produksi tabel proses.
#[no_mangle]
pub extern "C" fn klatos_graf_proses_kapasitas() -> u32 {
    BATAS_PROSES as u32
}

/// Kapasitas produksi tabel benang.
#[no_mangle]
pub extern "C" fn klatos_graf_benang_kapasitas() -> u32 {
    BATAS_BENANG as u32
}

/// Reset HANYA rentang produksi (dipakai audit; bukan produksi).
#[no_mangle]
pub extern "C" fn klatos_graf_reset() -> u32 {
    lock();
    let mut i = 0;
    while i < BATAS_PROSES {
        P_HIDUP[i].store(0, Ordering::Release);
        P_FLAGS[i].store(0, Ordering::Relaxed);
        P_TCOUNT[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    let mut j = 0;
    while j < BATAS_BENANG {
        T_HIDUP[j].store(0, Ordering::Release);
        T_FLAGS[j].store(0, Ordering::Relaxed);
        j += 1;
    }
    HIDUP_P.store(0, Ordering::Release);
    HIDUP_B.store(0, Ordering::Release);
    PUNCAK_P.store(0, Ordering::Release);
    PUNCAK_B.store(0, Ordering::Release);
    unlock();
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Oracle Naif — model independen semantik C++ historis
// ---------------------------------------------------------------------------

/// Model naif: keadaan proses + benang sebagai array biasa, tanpa atomik.
/// Implementasinya SENGAJA berbeda bentuk dari produksi supaya tidak
/// menyalin kesalahan yang sama.
struct Naif {
    p_state: [u32; 64],
    p_stop: [bool; 64],
    p_count: [u32; 64],
    t_state: [u32; 64],
    t_flags: [u32; 64],
    t_isig: [u32; 64],
}

impl Naif {
    const fn baru() -> Self {
        Naif {
            p_state: [0; 64],
            p_stop: [false; 64],
            p_count: [0; 64],
            t_state: [0; 64],
            t_flags: [0; 64],
            t_isig: [0; 64],
        }
    }
    /// `die()`: hanya transisi pertama yang berhasil.
    fn mati(&mut self, i: usize) -> bool {
        if self.p_state[i] == PR_RUNNING {
            self.p_state[i] = PR_DYING;
            true
        } else {
            false
        }
    }
    /// `set_stopped(x)`: kembalikan nilai LAMA.
    fn set_stop(&mut self, i: usize, v: bool) -> bool {
        let lama = self.p_stop[i];
        self.p_stop[i] = v;
        lama
    }
    /// `add_thread`: kembalikan `is_first`.
    fn tambah(&mut self, i: usize) -> bool {
        let lama = self.p_count[i];
        self.p_count[i] = lama.wrapping_add(1);
        lama == 0
    }
    /// `remove_thread`: kembalikan `was_last`. Membungkus seperti produksi.
    fn kurang(&mut self, i: usize) -> bool {
        let lama = self.p_count[i];
        self.p_count[i] = lama.wrapping_sub(1);
        lama == 1
    }
    /// `block_result()`.
    fn hasil_blokir(&self, i: usize) -> u32 {
        if (self.t_flags[i] & TF_INTR_DEATH) != 0 {
            return BR_INTR_DEATH;
        }
        if self.t_isig[i] != 0 {
            return BR_INTR_SIGNAL;
        }
        if (self.t_flags[i] & TF_DID_TIMEOUT) != 0 {
            return BR_INTR_TIMEOUT;
        }
        BR_WOKE
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

/// Cermin `KlatosGrafProsesBenangReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosGrafProsesBenangReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_daftar_proses: u32,
    pub prod_lepas_proses: u32,
    pub prod_daftar_benang: u32,
    pub prod_lepas_benang: u32,
    pub prod_mati: u32,
    pub prod_transisi: u32,
    pub hidup_proses: u32,
    pub hidup_benang: u32,
    pub puncak_proses: u32,
    pub puncak_benang: u32,
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

/// Rentang slot uji (cadangan) untuk kedua tabel.
const LO_P: usize = BATAS_PROSES;
const HI_P: usize = MAX_PROSES;
const LO_T: usize = BATAS_BENANG;
const HI_T: usize = MAX_BENANG;

/// Reset rentang CADANGAN saja (tidak pernah menyentuh produksi, dan tidak
/// menyentuh penghitung produksi).
/// Mengosongkan arena cadangan **beserta generasinya**, sehingga matriks dapat
/// dijalankan berulang kali dalam satu boot dengan hasil yang identik.
/// Rentang produksi TIDAK pernah disentuh, jadi invarian I8 (generasi tidak
/// pernah berulang untuk indeks produksi yang didaur ulang) tetap berlaku.
fn reset_cadangan() {
    lock();
    let mut i = LO_P;
    while i < HI_P {
        P_HIDUP[i].store(0, Ordering::Release);
        P_GEN[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    let mut j = LO_T;
    while j < HI_T {
        T_HIDUP[j].store(0, Ordering::Release);
        T_GEN[j].store(0, Ordering::Relaxed);
        j += 1;
    }
    unlock();
}

fn run_matrix() -> KlatosGrafProsesBenangReport {
    let mut m = Matrix::new();

    // --- kelompok 0: konstanta -------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4752_5048, Group::Peta);
    m.catat(MAX_PROSES == 4096 && MAX_BENANG == 4096, Group::Peta);
    m.catat(BATAS_PROSES == 4032 && BATAS_BENANG == 4032, Group::Peta);
    m.catat(CADANGAN == 64, Group::Peta);
    m.catat(klatos_graf_counter(99) == 0, Group::Peta);
    m.catat(TH_N_STATE == 7, Group::Peta);
    m.catat(PR_DEAD == 2, Group::Peta);
    m.catat(TF_SEMUA == 0x3FF, Group::Peta);
    m.catat(klatos_graf_proses_kapasitas() == 4032, Group::Peta);
    m.catat(klatos_graf_benang_kapasitas() == 4032, Group::Peta);
    m.catat(klatos_graf_proses_ambil(0, 0, 0) == TAK_SAH, Group::Peta);
    m.catat(klatos_graf_benang_ambil(0, 0, 0) == TAK_SAH, Group::Peta);

    // --- kelompok 1: siklus hidup proses (peta) ---------------------------
    reset_cadangan();
    {
        let h = p_daftar_impl(4242, 0, LO_P, HI_P);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        m.catat(h != 0, Group::Peta);
        m.catat(idx >= LO_P as u32 && (idx as usize) < HI_P, Group::Peta);
        m.catat(gen == 1, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 0) == PR_RUNNING, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 5) == 4242, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 6) == 0, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 2) == 0, Group::Peta);

        m.catat(p_mulai_mati_impl(idx, gen) == 1, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 0) == PR_DYING, Group::Peta);
        for _ in 0..5 {
            m.catat(p_mulai_mati_impl(idx, gen) == 0, Group::Peta);
        }
        m.catat(p_ambil_impl(idx, gen, 0) == PR_DYING, Group::Peta);
        m.catat(p_set_keadaan_impl(idx, gen, PR_DEAD) == PR_DYING, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 0) == PR_DEAD, Group::Peta);

        m.catat(p_stop_impl(idx, gen, 1) == 0, Group::Peta);
        m.catat(p_stop_impl(idx, gen, 1) == 1, Group::Peta);
        m.catat(p_stop_impl(idx, gen, 0) == 1, Group::Peta);
        m.catat(p_stop_impl(idx, gen, 0) == 0, Group::Peta);
        // Bit KERNEL tidak boleh tersentuh oleh STOP.
        m.catat(p_ambil_impl(idx, gen, 6) == 0, Group::Peta);

        m.catat(p_set_terminasi_impl(idx, gen, 7, 11) == 1, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 3) == 7, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 4) == 11, Group::Peta);

        m.catat(p_benang_tambah_impl(idx, gen) == 1, Group::Peta);
        m.catat(p_benang_tambah_impl(idx, gen) == 2, Group::Peta);
        m.catat(p_benang_kurang_impl(idx, gen) == 2, Group::Peta);
        m.catat(p_benang_kurang_impl(idx, gen) == 1, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 2) == 0, Group::Peta);

        m.catat(p_lepas_impl(idx, gen) == 1, Group::Peta);
        m.catat(p_ambil_impl(idx, gen, 0) == TAK_SAH, Group::Peta);
        let h2 = p_daftar_impl(4243, 1, LO_P, HI_P);
        m.catat((h2 & 0xFFFF_FFFF) as u32 == idx, Group::Peta);
        m.catat((h2 >> 32) as u32 == 2, Group::Peta);
        m.catat(p_ambil_impl(idx, 1, 0) == TAK_SAH, Group::Peta);
        m.catat(p_ambil_impl(idx, 2, 6) == 1, Group::Peta);
        m.catat(p_lepas_impl(idx, 2) == 1, Group::Peta);
    }

    // --- kelompok 2: siklus hidup benang (peta) ---------------------------
    {
        let h = t_daftar_impl(900, 4242, LO_T, HI_T);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        m.catat(h != 0, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, 0) == TH_INVALID, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, 7) == 4242, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, 8) == 900, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, 1) == TF_JOINABLE, Group::Peta);
        m.catat(klatos_graf_benang_flag(idx, gen, TF_JOINABLE) == 1, Group::Peta);
        m.catat(klatos_graf_benang_flag(idx, gen, TF_ACTIVE) == 0, Group::Peta);

        let mut prev = TH_INVALID;
        let urut = [
            TH_RUNNABLE,
            TH_RUNNING,
            TH_BLOCKED,
            TH_RUNNING,
            TH_STOPPED,
            TH_RUNNABLE,
            TH_RUNNING,
            TH_DYING,
            TH_DEAD,
        ];
        for s in urut {
            m.catat(t_set_keadaan_impl(idx, gen, s) == prev, Group::Peta);
            prev = s;
            m.catat(t_ambil_impl(idx, gen, 0) == s, Group::Peta);
        }

        let l1 = t_set_flag_impl(idx, gen, TF_ACTIVE, 1);
        m.catat(l1 & TF_ACTIVE == 0, Group::Peta);
        let l2 = t_set_flag_impl(idx, gen, TF_ACTIVE, 1);
        m.catat(l2 & TF_ACTIVE != 0, Group::Peta);
        let l3 = t_set_flag_impl(idx, gen, TF_ACTIVE, 0);
        m.catat(l3 & TF_ACTIVE != 0, Group::Peta);
        m.catat(klatos_graf_benang_flag(idx, gen, TF_ACTIVE) == 0, Group::Peta);
        t_set_flag_impl(idx, gen, TF_SHOULD_DIE, 1);
        t_set_flag_impl(idx, gen, TF_INITIALIZED, 1);
        let f = t_ambil_impl(idx, gen, 1);
        m.catat(f & TF_SHOULD_DIE != 0, Group::Peta);
        m.catat(f & TF_INITIALIZED != 0, Group::Peta);
        m.catat(f & TF_ACTIVE == 0, Group::Peta);

        m.catat(t_taruh_impl(idx, gen, TW_PRIORITY, 42) == 0, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, TW_PRIORITY) == 42, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, TW_AFFINITY) == AFINITAS_BAWAAN, Group::Peta);
        m.catat(t_taruh_impl(idx, gen, TW_AFFINITY, 3) == AFINITAS_BAWAAN, Group::Peta);
        m.catat(t_taruh_impl(idx, gen, TW_PENDING_SIGNALS, 0x10) == 0, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, TW_PENDING_SIGNALS) == 0x10, Group::Peta);
        m.catat(t_taruh_impl(idx, gen, TW_SIGNAL_MASK, 0xFF) == 0, Group::Peta);
        m.catat(t_taruh_impl(idx, gen, TW_HANDLED_SIGNAL, 9) == 0, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, TW_HANDLED_SIGNAL) == 9, Group::Peta);

        m.catat(t_hasil_blokir_impl(idx, gen) == BR_WOKE, Group::Peta);
        t_taruh_impl(idx, gen, TW_INTR_SIGNAL, 17);
        m.catat(t_hasil_blokir_impl(idx, gen) == BR_INTR_SIGNAL, Group::Peta);
        t_taruh_impl(idx, gen, TW_INTR_SIGNAL, 0);
        t_set_flag_impl(idx, gen, TF_DID_TIMEOUT, 1);
        m.catat(t_hasil_blokir_impl(idx, gen) == BR_INTR_TIMEOUT, Group::Peta);
        t_set_flag_impl(idx, gen, TF_INTR_DEATH, 1);
        m.catat(t_hasil_blokir_impl(idx, gen) == BR_INTR_DEATH, Group::Peta);
        t_set_flag_impl(idx, gen, TF_INTR_DEATH, 0);
        t_set_flag_impl(idx, gen, TF_DID_TIMEOUT, 0);
        m.catat(t_hasil_blokir_impl(idx, gen) == BR_WOKE, Group::Peta);

        m.catat(t_lepas_impl(idx, gen) == 1, Group::Peta);
        m.catat(t_ambil_impl(idx, gen, 0) == TAK_SAH, Group::Peta);
    }

    // --- kelompok 3: keanggotaan proses<->benang --------------------------
    {
        let hp = p_daftar_impl(7000, 0, LO_P, HI_P);
        let pi = (hp & 0xFFFF_FFFF) as u32;
        let pg = (hp >> 32) as u32;
        m.catat(p_benang_tambah_impl(pi, pg) == 1, Group::Peta);
        let mut tids = [0u32; 4];
        let mut gens = [0u32; 4];
        let mut k = 0;
        while k < 4 {
            let ht = t_daftar_impl(7001 + k as u32, 7000, LO_T, HI_T);
            tids[k] = (ht & 0xFFFF_FFFF) as u32;
            gens[k] = (ht >> 32) as u32;
            m.catat(t_taruh_impl(tids[k], gens[k], 7, 7000) == 7000, Group::Peta);
            // Benang ke-k menambah keanggotaan; `tambah` mengembalikan jumlah
            // BARU. Setelah benang proses sendiri (=1) lalu 4 benang: k+2.
            m.catat(p_benang_tambah_impl(pi, pg) == (k as u32) + 2, Group::Peta);
            k += 1;
        }
        m.catat(p_ambil_impl(pi, pg, 2) == 5, Group::Peta);
        let mut j = 0;
        while j < 4 {
            m.catat(t_ambil_impl(tids[j], gens[j], 7) == 7000, Group::Peta);
            j += 1;
        }
        let mut z = 0;
        while z < 4 {
            m.catat(t_lepas_impl(tids[z], gens[z]) == 1, Group::Peta);
            // `kurang` mengembalikan jumlah LAMA: 5, 4, 3, 2.
            let sisa = 5 - (z as u32);
            m.catat(p_benang_kurang_impl(pi, pg) == sisa, Group::Peta);
            z += 1;
        }
        m.catat(p_ambil_impl(pi, pg, 2) == 1, Group::Peta);
        m.catat(p_benang_kurang_impl(pi, pg) == 1, Group::Peta);
        m.catat(p_ambil_impl(pi, pg, 2) == 0, Group::Peta);
        m.catat(p_lepas_impl(pi, pg) == 1, Group::Peta);
    }

    // --- kelompok 4: diferensial vs oracle Naif (>= 100.000 kasus) --------
    let mut dif_ukur: u32 = 0;
    let mut dif_beda: u32 = 0;
    {
        reset_cadangan();
        let mut naif = Naif::baru();
        let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);

        let mut ph = [0u32; 32];
        let mut pg = [0u32; 32];
        let mut i = 0;
        while i < 32 {
            let h = p_daftar_impl(5000 + i as u32, (i % 2) as u32, LO_P, HI_P);
            ph[i] = (h & 0xFFFF_FFFF) as u32;
            pg[i] = (h >> 32) as u32;
            naif.p_state[i] = PR_RUNNING;
            i += 1;
        }
        let mut th = [0u32; 32];
        let mut tg = [0u32; 32];
        let mut j = 0;
        while j < 32 {
            let h = t_daftar_impl(6000 + j as u32, 5000 + j as u32, LO_T, HI_T);
            th[j] = (h & 0xFFFF_FFFF) as u32;
            tg[j] = (h >> 32) as u32;
            naif.t_state[j] = TH_INVALID;
            naif.t_flags[j] = TF_JOINABLE;
            j += 1;
        }

        let mut op = 0;
        while op < 40_000 {
            let r = rng.next();
            let slot = (r as usize) % 32;
            match r % 7 {
                0 => {
                    let want = match (r >> 8) % 3 {
                        0 => PR_RUNNING,
                        1 => PR_DYING,
                        _ => PR_DEAD,
                    };
                    let prod = p_set_keadaan_impl(ph[slot], pg[slot], want);
                    let lama = naif.p_state[slot];
                    naif.p_state[slot] = want;
                    dif_ukur += 3;
                    if prod != lama {
                        dif_beda += 1;
                    }
                    if p_ambil_impl(ph[slot], pg[slot], 0) != naif.p_state[slot] {
                        dif_beda += 1;
                    }
                    if p_ambil_impl(ph[slot], pg[slot], 0) != want {
                        dif_beda += 1;
                    }
                }
                1 => {
                    let prod = p_mulai_mati_impl(ph[slot], pg[slot]);
                    let o = naif.mati(slot) as u32;
                    dif_ukur += 3;
                    if prod != o {
                        dif_beda += 1;
                    }
                    if p_ambil_impl(ph[slot], pg[slot], 0) != naif.p_state[slot] {
                        dif_beda += 1;
                    }
                    if prod == 1 && p_ambil_impl(ph[slot], pg[slot], 0) != PR_DYING {
                        dif_beda += 1;
                    }
                }
                2 => {
                    let v = (r >> 8) & 1;
                    let prod = p_stop_impl(ph[slot], pg[slot], v);
                    let o = naif.set_stop(slot, v != 0) as u32;
                    dif_ukur += 3;
                    if prod != o {
                        dif_beda += 1;
                    }
                    let bit = p_ambil_impl(ph[slot], pg[slot], 1) & PF_STOP;
                    if (bit != 0) != naif.p_stop[slot] {
                        dif_beda += 1;
                    }
                    if (bit != 0) != (v != 0) {
                        dif_beda += 1;
                    }
                }
                3 => {
                    let prod = p_benang_tambah_impl(ph[slot], pg[slot]);
                    let o = naif.tambah(slot);
                    dif_ukur += 3;
                    if prod != naif.p_count[slot] {
                        dif_beda += 1;
                    }
                    if (prod == 1) != o {
                        dif_beda += 1;
                    }
                    if p_ambil_impl(ph[slot], pg[slot], 2) != naif.p_count[slot] {
                        dif_beda += 1;
                    }
                }
                4 => {
                    let prod = p_benang_kurang_impl(ph[slot], pg[slot]);
                    let o = naif.kurang(slot);
                    dif_ukur += 3;
                    if prod != naif.p_count[slot].wrapping_add(1) {
                        dif_beda += 1;
                    }
                    if (prod == 1) != o {
                        dif_beda += 1;
                    }
                    if p_ambil_impl(ph[slot], pg[slot], 2) != naif.p_count[slot] {
                        dif_beda += 1;
                    }
                }
                5 => {
                    let s = (r >> 8) % TH_N_STATE;
                    let prod = t_set_keadaan_impl(th[slot], tg[slot], s);
                    let lama = naif.t_state[slot];
                    naif.t_state[slot] = s;
                    dif_ukur += 3;
                    if prod != lama {
                        dif_beda += 1;
                    }
                    if t_ambil_impl(th[slot], tg[slot], 0) != naif.t_state[slot] {
                        dif_beda += 1;
                    }
                    if t_ambil_impl(th[slot], tg[slot], 0) != s {
                        dif_beda += 1;
                    }
                }
                _ => {
                    let bit = match (r >> 8) % 4 {
                        0 => TF_ACTIVE,
                        1 => TF_SHOULD_DIE,
                        2 => TF_DID_TIMEOUT,
                        _ => TF_INTR_DEATH,
                    };
                    let v = (r >> 16) & 1;
                    let prod = t_set_flag_impl(th[slot], tg[slot], bit, v);
                    let lama = naif.t_flags[slot];
                    if v != 0 {
                        naif.t_flags[slot] |= bit;
                    } else {
                        naif.t_flags[slot] &= !bit;
                    }
                    dif_ukur += 3;
                    if prod != lama {
                        dif_beda += 1;
                    }
                    if t_ambil_impl(th[slot], tg[slot], 1) != naif.t_flags[slot] {
                        dif_beda += 1;
                    }
                    if t_hasil_blokir_impl(th[slot], tg[slot]) != naif.hasil_blokir(slot) {
                        dif_beda += 1;
                    }
                }
            }
            op += 1;
        }

        let mut a = 0;
        while a < 32 {
            dif_ukur += 2;
            if p_ambil_impl(ph[a], pg[a], 0) != naif.p_state[a] {
                dif_beda += 1;
            }
            if t_ambil_impl(th[a], tg[a], 0) != naif.t_state[a] {
                dif_beda += 1;
            }
            a += 1;
        }

        // Lepaskan seluruh slot arena agar matriks dapat diulang (integrasi_inti
        // menjalankan selftest ini TIGA kali per boot: pas, determinisme, pulih).
        let mut r = 0;
        while r < 32 {
            p_lepas_impl(ph[r], pg[r]);
            t_lepas_impl(th[r], tg[r]);
            r += 1;
        }
    }
    m.catat(dif_ukur >= 100_000, Group::Peta);
    m.catat(dif_beda == 0, Group::Peta);

    // --- kelompok 5: determinisme -----------------------------------------
    {
        let a = klatos_graf_proses_hidup();
        let b = klatos_graf_benang_hidup();
        let mut ok = true;
        let mut i = 0;
        while i < 10_000 {
            if klatos_graf_proses_hidup() != a || klatos_graf_benang_hidup() != b {
                ok = false;
                break;
            }
            i += 1;
        }
        m.catat(ok, Group::Peta);
    }

    // --- kelompok 6: negatif ----------------------------------------------
    reset_cadangan();
    {
        m.catat(p_ambil_impl(0, 0, 0) == TAK_SAH, Group::Neg);
        m.catat(p_ambil_impl(LO_P as u32, 0, 0) == TAK_SAH, Group::Neg);
        m.catat(p_ambil_impl(MAX_PROSES as u32, 1, 0) == TAK_SAH, Group::Neg);
        m.catat(p_ambil_impl(LO_P as u32, 99, 0) == TAK_SAH, Group::Neg);
        m.catat(p_lepas_impl(LO_P as u32, 1) == 0, Group::Neg);
        m.catat(p_mulai_mati_impl(LO_P as u32, 1) == 0, Group::Neg);
        m.catat(p_stop_impl(LO_P as u32, 1, 1) == TAK_SAH, Group::Neg);
        m.catat(p_benang_tambah_impl(LO_P as u32, 1) == TAK_SAH, Group::Neg);
        m.catat(p_benang_kurang_impl(LO_P as u32, 1) == TAK_SAH, Group::Neg);
        m.catat(p_set_terminasi_impl(LO_P as u32, 1, 1, 1) == 0, Group::Neg);
        m.catat(p_set_keadaan_impl(LO_P as u32, 1, PR_DEAD) == TAK_SAH, Group::Neg);

        m.catat(t_ambil_impl(LO_T as u32, 1, 0) == TAK_SAH, Group::Neg);
        m.catat(t_lepas_impl(LO_T as u32, 1) == 0, Group::Neg);
        m.catat(t_set_keadaan_impl(LO_T as u32, 1, TH_RUNNING) == TAK_SAH, Group::Neg);
        m.catat(t_set_flag_impl(LO_T as u32, 1, TF_ACTIVE, 1) == TAK_SAH, Group::Neg);
        m.catat(klatos_graf_benang_flag(LO_T as u32, 1, TF_ACTIVE) == TAK_SAH, Group::Neg);
        m.catat(t_hasil_blokir_impl(LO_T as u32, 1) == TAK_SAH, Group::Neg);

        let h = p_daftar_impl(1, 0, LO_P, HI_P);
        let pi = (h & 0xFFFF_FFFF) as u32;
        let pg = (h >> 32) as u32;
        m.catat(p_set_keadaan_impl(pi, pg, 3) == TAK_SAH, Group::Neg);
        m.catat(p_ambil_impl(pi, pg, 0) == PR_RUNNING, Group::Neg);
        m.catat(p_lepas_impl(pi, pg) == 1, Group::Neg);

        let ht = t_daftar_impl(2, 1, LO_T, HI_T);
        let ti = (ht & 0xFFFF_FFFF) as u32;
        let tg = (ht >> 32) as u32;
        m.catat(t_set_keadaan_impl(ti, tg, TH_N_STATE) == TAK_SAH, Group::Neg);
        m.catat(t_set_flag_impl(ti, tg, 1 << 31, 1) == TAK_SAH, Group::Neg);
        m.catat(t_set_flag_impl(ti, tg, 0, 1) == TAK_SAH, Group::Neg);
        m.catat(t_ambil_impl(ti, tg, 99) == TAK_SAH, Group::Neg);
        m.catat(t_taruh_impl(ti, tg, 99, 1) == TAK_SAH, Group::Neg);
        m.catat(t_lepas_impl(ti, tg) == 1, Group::Neg);
        m.catat(t_ambil_impl(ti, tg, 0) == TAK_SAH, Group::Neg);

        // Matriks hanya memakai arena cadangan, dan arena itu bersih kembali
        // sesudahnya — bukti matriks tidak membocorkan slot (dan karena itu
        // dapat diulang). Rentang PRODUKSI sengaja tidak diperiksa "kosong":
        // selftest berjalan di `init_stage2`, ketika proses `init` sudah hidup.
        let mut i = LO_P;
        let mut bersih = true;
        while i < HI_P {
            if P_HIDUP[i].load(Ordering::Relaxed) != 0 {
                bersih = false;
                break;
            }
            i += 1;
        }
        m.catat(bersih, Group::Neg);
    }

    // --- kelompok 7: F-P149-07 — keabsahan vs nilai ------------------------
    //
    // Bug historis: pembungkus memutuskan "ditolak" dari `r == TAK_SAH`.
    // Afinitas bawaan SAH bernilai `AFINITAS_BAWAAN == TAK_SAH`, sehingga
    // setiap baca afinitas bawaan dari produksi dihitung sebagai penolakan
    // handle palsu (t10/t11). Di sini dibuktikan: kanal NILAI dan kanal
    // KEABSAHAN terpisah.
    {
        reset_cadangan();
        let h = t_daftar_impl(0x0A53, 0x0A53, LO_T, HI_T);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        m.catat(h != 0 && gen != 0, Group::Peta);

        // (a) NILAI afinitas bawaan == TAK_SAH, TETAPI bukan penolakan.
        let c0 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
        let v_awal = klatos_graf_benang_ambil(idx, gen, TW_AFFINITY);
        let c1 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
        m.catat(v_awal == AFINITAS_BAWAAN, Group::Peta);
        m.catat(v_awal == TAK_SAH, Group::Peta); // memang bentrok
        m.catat(c1 == c0, Group::Peta); // kanal keabsahan bersih

        // (b) menulis afinitas bawaan juga bukan penolakan.
        let c2 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
        let lama = klatos_graf_benang_taruh(idx, gen, TW_AFFINITY, AFINITAS_BAWAAN);
        let c3 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
        m.catat(lama == AFINITAS_BAWAAN, Group::Peta);
        m.catat(c3 == c2, Group::Peta);

        // (c) afinitas non-bawaan yang sah.
        m.catat(klatos_graf_benang_taruh(idx, gen, TW_AFFINITY, 5) == AFINITAS_BAWAAN, Group::Peta);
        m.catat(klatos_graf_benang_ambil(idx, gen, TW_AFFINITY) == 5, Group::Peta);
        klatos_graf_benang_taruh(idx, gen, TW_AFFINITY, AFINITAS_BAWAAN);

        // (d) handle SALAH harus TETAP dihitung — kanal keabsahan hidup.
        let salah = [
            (idx, gen.wrapping_add(1)), // generasi salah
            (idx, 0),                   // generasi 0 = tak terdaftar
            (MAX_BENANG as u32, gen),   // indeks di luar tabel
        ];
        for (bi, bg) in salah {
            let c4 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
            let r = klatos_graf_benang_ambil(bi, bg, TW_AFFINITY);
            let c5 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
            m.catat(r == TAK_SAH, Group::Peta);
            m.catat(c5 == c4 + 1, Group::Peta);
        }
        // Handle proses yang salah untuk benang (nomor proses) juga ditolak.
        {
            let c4 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
            let r = klatos_graf_benang_taruh(0x0A53, 0x0A53, TW_AFFINITY, 1);
            let c5 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
            m.catat(r == TAK_SAH, Group::Peta);
            m.catat(c5 == c4 + 1, Group::Peta);
        }

        // (e) SETIAP bidang TW_* dibaca/ditulis tanpa satu pun penolakan.
        let c6 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
        klatos_graf_benang_taruh(idx, gen, TW_PRIORITY, 37);
        m.catat(klatos_graf_benang_ambil(idx, gen, TW_PRIORITY) == 37, Group::Peta);
        klatos_graf_benang_taruh(idx, gen, TW_PENDING_SIGNALS, 0x20);
        m.catat(klatos_graf_benang_ambil(idx, gen, TW_PENDING_SIGNALS) == 0x20, Group::Peta);
        klatos_graf_benang_taruh(idx, gen, TW_SIGNAL_MASK, 0x0F);
        m.catat(klatos_graf_benang_ambil(idx, gen, TW_SIGNAL_MASK) == 0x0F, Group::Peta);
        klatos_graf_benang_taruh(idx, gen, TW_HANDLED_SIGNAL, 11);
        m.catat(klatos_graf_benang_ambil(idx, gen, TW_HANDLED_SIGNAL) == 11, Group::Peta);
        klatos_graf_benang_taruh(idx, gen, TW_INTR_SIGNAL, 13);
        m.catat(klatos_graf_benang_ambil(idx, gen, TW_INTR_SIGNAL) == 13, Group::Peta);
        m.catat(klatos_graf_benang_ambil(idx, gen, 0) == TH_INVALID, Group::Peta);
        m.catat(klatos_graf_benang_ambil(idx, gen, 1) == TF_JOINABLE, Group::Peta);
        m.catat(klatos_graf_benang_ambil(idx, gen, 7) == 0x0A53, Group::Peta);
        m.catat(klatos_graf_benang_ambil(idx, gen, 8) == 0x0A53, Group::Peta);
        let c7 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
        m.catat(c7 == c6, Group::Peta);

        // (f) generasi yang sudah DILEPAS -> ditolak.
        m.catat(t_lepas_impl(idx, gen) == 1, Group::Peta);
        let c8 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
        let r_lepas = klatos_graf_benang_ambil(idx, gen, TW_AFFINITY);
        let c9 = klatos_graf_counter(Counter::ProdHandleSalah as u32);
        m.catat(r_lepas == TAK_SAH, Group::Peta);
        m.catat(c9 == c8 + 1, Group::Peta);
    }

    // --- kelompok 8: F-P149-07 diferensial wrapper vs oracle (>= 100.000) --
    //
    // Membandingkan NILAI yang dikembalikan pembungkus C-ABI dengan oracle
    // bidang independen. Nilai `0xFFFF_FFFF` sengaja sering dialirkan agar
    // sentinel lama pasti terpicu.
    let mut ch_ukur: u32 = 0;
    let mut ch_beda: u32 = 0;
    {
        reset_cadangan();
        let mut th = [0u32; 16];
        let mut tg = [0u32; 16];
        // Kolom: 0=state 1=flags 2=prio 3=aff 4=psig 5=smask 6=hsig
        //        7=pid 8=tid 9=isig
        let mut nilai = [[0u32; 10]; 16];
        let mut i = 0;
        while i < 16 {
            let h = t_daftar_impl(0xB000 + i as u32, 0xB000 + i as u32, LO_T, HI_T);
            th[i] = (h & 0xFFFF_FFFF) as u32;
            tg[i] = (h >> 32) as u32;
            nilai[i] = [
                TH_INVALID,
                TF_JOINABLE,
                0,
                AFINITAS_BAWAAN,
                0,
                0,
                0,
                0xB000 + i as u32,
                0xB000 + i as u32,
                0,
            ];
            i += 1;
        }
        // (which, kolom, boleh_tulis)
        let field = [
            (0u32, 0usize, false),
            (1, 1, false),
            (TW_PRIORITY, 2, true),
            (TW_AFFINITY, 3, true),
            (TW_PENDING_SIGNALS, 4, true),
            (TW_SIGNAL_MASK, 5, true),
            (TW_HANDLED_SIGNAL, 6, true),
            (7, 7, true),
            (TW_INTR_SIGNAL, 9, true),
        ];
        let mut rng = Lcg(0xD1CE_5EED_0BAD_F00D);
        let mut op = 0;
        while op < 90_000 {
            let r = rng.next();
            let slot = (r as usize) % 16;
            let (which, col, bisa_tulis) = field[((r >> 8) as usize) % field.len()];
            if bisa_tulis && ((r >> 20) & 1) != 0 {
                // 1/16 tulisan memakai 0xFFFF_FFFF (sentinel lama).
                let v = if (r >> 28) & 1 != 0 { AFINITAS_BAWAAN } else { r >> 24 };
                let lama = klatos_graf_benang_taruh(th[slot], tg[slot], which, v);
                ch_ukur += 1;
                if lama != nilai[slot][col] {
                    ch_beda += 1;
                }
                nilai[slot][col] = if which == TW_HANDLED_SIGNAL || which == TW_INTR_SIGNAL {
                    v & 0xFF
                } else {
                    v
                };
            }
            let got = klatos_graf_benang_ambil(th[slot], tg[slot], which);
            ch_ukur += 1;
            if got != nilai[slot][col] {
                ch_beda += 1;
            }
            op += 1;
        }
        let mut z = 0;
        while z < 16 {
            t_lepas_impl(th[z], tg[z]);
            z += 1;
        }
    }
    m.catat(ch_ukur >= 100_000, Group::Peta);
    m.catat(ch_beda == 0, Group::Peta);

    // --- kelompok 9: P1.73 — akuntansi clock_t proses ---------------------
    // Sumber `times(2)`/`getrusage(2)`. Satuan = tick scheduler KASAR, sengaja
    // TERPISAH dari waktu-jadwal nanodetik P1.72 (jangan dicampur satuannya).
    reset_cadangan();
    {
        let h = p_daftar_impl(7301, 0, LO_P, HI_P);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        m.catat(h != 0, Group::Peta);

        // Keadaan awal: keempat penghitung NOL (slot bersih saat daftar ulang).
        m.catat(klatos_graf_proses_ticks(idx, gen, 0) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 1) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 2) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 3) == 0, Group::Peta);

        // Keabsahan = kanal tersendiri (F-P149-07): gen salah / which liar.
        m.catat(klatos_graf_proses_tick(idx, gen.wrapping_add(7), 0) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen.wrapping_add(7), 0) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 4) == 0, Group::Peta);
        m.catat(klatos_graf_proses_adopt_dead(idx, gen.wrapping_add(7), 1, 1) == 0, Group::Peta);

        // Diferensial: N tick user + M tick kernel terhadap oracle.
        let n_user = 37u64;
        let n_kernel = 23u64;
        let mut ok = true;
        let mut k = 0u64;
        while k < n_user {
            ok &= klatos_graf_proses_tick(idx, gen, 0) == 1;
            k += 1;
        }
        k = 0;
        while k < n_kernel {
            ok &= klatos_graf_proses_tick(idx, gen, 1) == 1;
            k += 1;
        }
        m.catat(ok, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 0) == n_user, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 1) == n_kernel, Group::Peta);
        // Tidak ada kebocoran antar-saluran.
        m.catat(klatos_graf_proses_ticks(idx, gen, 0) != klatos_graf_proses_ticks(idx, gen, 1), Group::Peta);

        // Penghitung anak-mati: terpisah dan AKUMULATIF.
        m.catat(klatos_graf_proses_adopt_dead(idx, gen, 11, 5) == 1, Group::Peta);
        m.catat(klatos_graf_proses_adopt_dead(idx, gen, 7, 3) == 1, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 2) == 18, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 3) == 8, Group::Peta);
        // Akumulasi anak-mati tidak menyentuh penghitung langsung.
        m.catat(klatos_graf_proses_ticks(idx, gen, 0) == n_user, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 1) == n_kernel, Group::Peta);

        // Rilis: slot tak lagi sah; baca/tick berikutnya ditolak.
        m.catat(p_lepas_impl(idx, gen) == 1, Group::Peta);
        m.catat(klatos_graf_proses_ticks(idx, gen, 0) == 0, Group::Peta);
        m.catat(klatos_graf_proses_tick(idx, gen, 0) == 0, Group::Peta);
    }

    // --- kelompok 10: FASE-4 BATCH 03 — state JANJI (pledge) per-proses ----
    // Otoritas state janji kini milik Rust (bukan `Process::ProtectedValues`).
    reset_cadangan();
    {
        let h1 = p_daftar_impl(7401, 0, LO_P, HI_P);
        let i1 = (h1 & 0xFFFF_FFFF) as u32;
        let g1 = (h1 >> 32) as u32;
        let h2 = p_daftar_impl(7402, 0, LO_P, HI_P);
        let i2 = (h2 & 0xFFFF_FFFF) as u32;
        let g2 = (h2 >> 32) as u32;
        m.catat(h1 != 0 && h2 != 0 && i1 != i2, Group::Peta);

        // Slot bersih saat daftar ulang.
        m.catat(klatos_graf_proses_ambil(i1, g1, PW_HAS_PROMISES) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i1, g1, PW_PROMISES) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i1, g1, PW_HAS_EXECPROMISES) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i1, g1, PW_EXECPROMISES) == 0, Group::Peta);

        // Tulis: mengembalikan nilai LAMA.
        m.catat(klatos_graf_proses_taruh(i1, g1, PW_PROMISES, 0x0BAD_F00D) == 0, Group::Peta);
        m.catat(klatos_graf_proses_taruh(i1, g1, PW_PROMISES, 0x0000_0004) == 0x0BAD_F00D, Group::Peta);
        m.catat(klatos_graf_proses_taruh(i1, g1, PW_HAS_PROMISES, 1) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i1, g1, PW_PROMISES) == 0x0000_0004, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i1, g1, PW_HAS_PROMISES) == 1, Group::Peta);

        // `which` di luar rentang janji ditolak, state tak tersentuh.
        m.catat(klatos_graf_proses_taruh(i1, g1, 6, 7) == TAK_SAH, Group::Neg);
        m.catat(klatos_graf_proses_taruh(i1, g1, 11, 7) == TAK_SAH, Group::Neg);
        m.catat(klatos_graf_proses_ambil(i1, g1, 11) == TAK_SAH, Group::Neg);
        m.catat(klatos_graf_proses_ambil(i1, g1, PW_PROMISES) == 0x0000_0004, Group::Peta);
        // Handle basi ditolak.
        m.catat(klatos_graf_proses_taruh(i1, g1.wrapping_add(9), PW_PROMISES, 0) == TAK_SAH, Group::Neg);
        m.catat(klatos_graf_proses_salin_janji(i1, g1, i2, g2.wrapping_add(9)) == 0, Group::Neg);

        // Salin (fork): anak mewarisi janji induk.
        m.catat(klatos_graf_proses_taruh(i1, g1, PW_EXECPROMISES, 0x0000_0010) == 0, Group::Peta);
        m.catat(klatos_graf_proses_taruh(i1, g1, PW_HAS_EXECPROMISES, 1) == 0, Group::Peta);
        m.catat(klatos_graf_proses_salin_janji(i1, g1, i2, g2) == 1, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i2, g2, PW_PROMISES) == 0x0000_0004, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i2, g2, PW_HAS_PROMISES) == 1, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i2, g2, PW_EXECPROMISES) == 0x0000_0010, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i2, g2, PW_HAS_EXECPROMISES) == 1, Group::Peta);

        // Daur ulang slot: state janji WAJIB bersih (tidak mewarisi proses mati).
        m.catat(p_lepas_impl(i1, g1) == 1, Group::Peta);
        let h3 = p_daftar_impl(7403, 0, LO_P, HI_P);
        let i3 = (h3 & 0xFFFF_FFFF) as u32;
        let g3 = (h3 >> 32) as u32;
        m.catat(i3 == i1, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i3, g3, PW_PROMISES) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i3, g3, PW_HAS_PROMISES) == 0, Group::Peta);
        m.catat(klatos_graf_proses_ambil(i3, g3, PW_EXECPROMISES) == 0, Group::Peta);
        m.catat(p_lepas_impl(i3, g3) == 1, Group::Peta);
        m.catat(p_lepas_impl(i2, g2) == 1, Group::Peta);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosGrafProsesBenangReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_daftar_proses: klatos_graf_counter(Counter::ProdDaftarProses as u32),
        prod_lepas_proses: klatos_graf_counter(Counter::ProdLepasProses as u32),
        prod_daftar_benang: klatos_graf_counter(Counter::ProdDaftarBenang as u32),
        prod_lepas_benang: klatos_graf_counter(Counter::ProdLepasBenang as u32),
        prod_mati: klatos_graf_counter(Counter::ProdMati as u32),
        prod_transisi: klatos_graf_counter(Counter::ProdTransisi as u32),
        hidup_proses: klatos_graf_proses_hidup(),
        hidup_benang: klatos_graf_benang_hidup(),
        puncak_proses: klatos_graf_proses_puncak(),
        puncak_benang: klatos_graf_benang_puncak(),
        kapasitas: BATAS_PROSES as u32,
        status,
    }
}

/// Selftest graf objek proses/benang.
#[no_mangle]
pub extern "C" fn klatos_graf_selftest() -> KlatosGrafProsesBenangReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4752_5048);
        assert!(MAX_PROSES == 4096);
        assert!(MAX_BENANG == 4096);
        assert!(CADANGAN == 64);
        assert!(BATAS_PROSES == 4032);
        assert!(BATAS_BENANG == 4032);
        assert!(PR_RUNNING == 0 && PR_DYING == 1 && PR_DEAD == 2);
        // FASE-4 BATCH 03 — kode `which` janji harus tetap sinkron dengan
        // `graf_pw_*` di `Jowo/Kontrak/KlatKernelBridge.h`.
        assert!(PW_HAS_PROMISES == 7 && PW_PROMISES == 8);
        assert!(PW_HAS_EXECPROMISES == 9 && PW_EXECPROMISES == 10 && PW_MAKS == 10);
        assert!(TH_INVALID == 0 && TH_RUNNABLE == 1 && TH_RUNNING == 2);
        assert!(TH_DYING == 3 && TH_DEAD == 4 && TH_STOPPED == 5 && TH_BLOCKED == 6);
        assert!(TH_N_STATE == 7);
        assert!(TF_SEMUA == 0x3FF);
        assert!(AFINITAS_BAWAAN == 0xFFFF_FFFF);
        assert!(Counter::ProdDaftarProses as usize == 0);
        assert!(Counter::ProdHandleSalah as usize == 7);
        assert!(N_COUNTER == 8);
        assert!(core::mem::size_of::<KlatosGrafProsesBenangReport>() == 80);
        assert!(core::mem::align_of::<KlatosGrafProsesBenangReport>() == 8);
    };
}
