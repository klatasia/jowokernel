// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


use core::sync::atomic::{AtomicU32, Ordering, AtomicU64};

// ---------------------------------------------------------------------------
// Konstanta
// ---------------------------------------------------------------------------

/// Kapasitas tabel penghalang (berbatas).
pub const MAX_PENGHALANG: usize = 4096;

/// Slot teratas dicadangkan untuk selftest.
pub const CADANGAN: usize = 64;

/// Batas atas alokasi produksi (eksklusif).
pub const BATAS_PENGHALANG: usize = MAX_PENGHALANG - CADANGAN;

/// Tag laporan: "KLATPHBL".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5048_424C;

/// Sentinel "handle tidak sah".
pub const TAK_SAH: u32 = 0xFFFF_FFFF;

// Jenis penghalang — cermin `Thread::Blocker::Type` VERBATIM.
pub const PK_UNKNOWN: u32 = 0;
pub const PK_FILE: u32 = 1;
pub const PK_FUTEX: u32 = 2;
pub const PK_PLAN9FS: u32 = 3;
pub const PK_JOIN: u32 = 4;
pub const PK_QUEUE: u32 = 5;
pub const PK_ROUTING: u32 = 6;
pub const PK_SLEEP: u32 = 7;
pub const PK_SIGNAL: u32 = 8;
pub const PK_WAIT: u32 = 9;
pub const PK_FLOCK: u32 = 10;
pub const PK_N_KIND: u32 = 11;

// Bendera logis.
pub const PB_BLOCKING: u32 = 1 << 0;
pub const PB_INTR_DEATH: u32 = 1 << 1;
pub const PB_DID_TIMEOUT: u32 = 1 << 2;
pub const PB_SEMUA: u32 = 0x7;

// Pemilih `which` untuk `klatos_pembendung_baca`.
pub const PW_FLAGS: u32 = 0;
pub const PW_ISIG: u32 = 1;
pub const PW_KIND: u32 = 2;
pub const PW_WAS_INTR: u32 = 3;
pub const PW_N_WHICH: u32 = 4;

// ---------------------------------------------------------------------------
// P1.55 — MESIN KEADAAN TRANSISI (peristiwa yang diklasifikasikan)
// ---------------------------------------------------------------------------
//
// Keadaan penghalang adalah pasangan `(flags, isig)`:
//
//   flags bit 0 = BLOCKING     bit 1 = INTR_DEATH     bit 2 = DID_TIMEOUT
//   isig        = sinyal penginterupsi (0 = tidak ada)
//
// Peristiwa yang mengubah keadaan (semua berasal dari sumber, tidak ada yang
// dikarang), beserta transisinya:
//
//   MULAI          (BLOCKING=0) -> BLOCKING=1                       ret 1
//   MULAI          (BLOCKING=1) -> tidak berubah                    ret 0   B1
//   AKHIR          (BLOCKING=1) -> BLOCKING=0                       ret 1
//   AKHIR          (BLOCKING=0) -> tidak berubah                    ret 0   B5
//   INTR_MATI      -> INTR_DEATH=1 (isig TIDAK disentuh)            ret 1   B7
//   INTR_SINYAL(s) s=0 -> tidak berubah                             ret 0   B6
//   INTR_SINYAL(s) s!=0 -> isig=s                                   ret 1
//   HAPUS_INTR     -> isig=0                                        ret 1   B11
//   LEWAT(0)       -> tidak berubah                                 ret 1
//   LEWAT(1)       -> DID_TIMEOUT=1                                 ret 1   B8
//   HASIL(k)       -> keadaan tidak berubah; hasil = B2 lalu B12     BR_*
//
// Inilah SATU-SATUNYA tempat mesin keadaan didefinisikan. Implementasi
// produksinya adalah primitif atomik di bawah (supaya balapan `end_blocking`
// vs `unblock_from_blocker` tetap terserialisasi); selftest kelompok 8
// membandingkan keduanya secara menyeluruh.

/// `begin_blocking()` menghasilkan BLOCKING (B1 sukses).
pub const EV_MULAI: u32 = 0;
/// `begin_blocking()` saat sudah memblokir (pelanggaran B1).
pub const EV_MULAI_DITOLAK: u32 = 1;
/// `end_blocking()` saat memang memblokir.
pub const EV_AKHIR_BLOKIR: u32 = 2;
/// `end_blocking()`/`unblock_from_blocker()` saat sudah bersih (B5 no-op).
pub const EV_AKHIR_BERSIH: u32 = 3;
/// `set_interrupted_by_death()` (B7).
pub const EV_INTR_MATI: u32 = 4;
/// `set_interrupted_by_signal(s != 0)`.
pub const EV_INTR_SINYAL: u32 = 5;
/// `set_interrupted_by_signal(0)` — ditolak (B6).
pub const EV_INTR_SINYAL_NOL: u32 = 6;
/// `do_clear_interrupted_by_signal()` (B11).
pub const EV_HAPUS_INTR: u32 = 7;
/// `was_unblocked(true)` — timeout terpasang (B8).
pub const EV_TIMEOUT: u32 = 8;
/// `was_unblocked(false)` — tidak ada timeout (B8).
pub const EV_TIMEOUT_NOOP: u32 = 9;
/// `block_result()` pulang normal.
pub const EV_HASIL_WOKE: u32 = 10;
/// `block_result()` terinterupsi sinyal.
pub const EV_HASIL_SINYAL: u32 = 11;
/// `block_result()` terinterupsi kematian.
pub const EV_HASIL_DEATH: u32 = 12;
/// `block_result()` timeout pada jenis NON-Sleep.
pub const EV_HASIL_TIMEOUT: u32 = 13;
/// `block_result()` timeout pada Sleep → dipetakan ke pulang normal (B12).
pub const EV_HASIL_SLEEP_REMAP: u32 = 14;
/// Jumlah peristiwa.
pub const EV_N: u32 = 15;

// Peristiwa (MASUKAN mesin keadaan) — ruang terpisah dari kelas di atas,
// supaya `spek_transisi` tidak dapat tertukar dengan kelas keluaran.
pub const PER_MULAI: u32 = 0;
pub const PER_AKHIR: u32 = 1;
pub const PER_INTR_MATI: u32 = 2;
pub const PER_INTR_SINYAL: u32 = 3;
pub const PER_HAPUS_INTR: u32 = 4;
pub const PER_LEWAT: u32 = 5;
pub const PER_HASIL: u32 = 6;
pub const PER_N: u32 = 7;

/// Penghitung peristiwa transisi (indeks = `EV_*`).
static EV: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
/// Salinan `EV` pada saat `klatos_pembendung_ev_snapshot()`.
static EV_BASE: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];

#[inline]
fn bump_ev(ev: u32) {
    EV[(ev & 0xF) as usize].fetch_add(1, Ordering::Relaxed);
}

/// Klasifikasi peristiwa `mulai` dari keadaan BLOCKING lama.
#[inline]
fn ev_mulai(lama: u32) -> u32 {
    if (lama & PB_BLOCKING) == 0 {
        EV_MULAI
    } else {
        EV_MULAI_DITOLAK
    }
}

/// Klasifikasi peristiwa `akhir` dari keadaan BLOCKING lama.
#[inline]
fn ev_akhir(lama: u32) -> u32 {
    if (lama & PB_BLOCKING) != 0 {
        EV_AKHIR_BLOKIR
    } else {
        EV_AKHIR_BERSIH
    }
}

/// Klasifikasi peristiwa `lewat_waktu`.
#[inline]
fn ev_lewat(did_timeout: u32) -> u32 {
    if did_timeout != 0 {
        EV_TIMEOUT
    } else {
        EV_TIMEOUT_NOOP
    }
}

/// Klasifikasi peristiwa `hasil` dari hasil MENTAH (sebelum remap B12).
#[inline]
fn ev_hasil(kind: u32, mentah: u32) -> u32 {
    match mentah {
        BR_WOKE => EV_HASIL_WOKE,
        BR_INTR_SIGNAL => EV_HASIL_SINYAL,
        BR_INTR_DEATH => EV_HASIL_DEATH,
        _ => {
            if kind == PK_SLEEP {
                EV_HASIL_SLEEP_REMAP
            } else {
                EV_HASIL_TIMEOUT
            }
        }
    }
}

/// Spesifikasi MURNI mesin keadaan: `(flags, isig, peristiwa, arg)` →
/// `(flags_baru, isig_baru, nilai_kembali)`. Dipakai HANYA oleh selftest
/// sebagai oracle independen; produksi memakai primitif atomik.
fn spek_transisi(flags: u32, isig: u32, per: u32, arg: u32) -> (u32, u32, u32) {
    match per {
        PER_MULAI => (
            flags | PB_BLOCKING,
            isig,
            ((flags & PB_BLOCKING) == 0) as u32,
        ),
        PER_AKHIR => (
            flags & !PB_BLOCKING,
            isig,
            ((flags & PB_BLOCKING) != 0) as u32,
        ),
        PER_INTR_MATI => (flags | PB_INTR_DEATH, isig, 1),
        PER_INTR_SINYAL => {
            if arg == 0 {
                (flags, isig, 0)
            } else {
                (flags, arg & 0xFF, 1)
            }
        }
        PER_HAPUS_INTR => (flags, 0, 1),
        PER_LEWAT => (
            if arg != 0 { flags | PB_DID_TIMEOUT } else { flags },
            isig,
            1,
        ),
        PER_HASIL => {
            let mentah = if (flags & PB_INTR_DEATH) != 0 {
                BR_INTR_DEATH
            } else if isig != 0 {
                BR_INTR_SIGNAL
            } else if (flags & PB_DID_TIMEOUT) != 0 {
                BR_INTR_TIMEOUT
            } else {
                BR_WOKE
            };
            let r = if arg == PK_SLEEP && mentah == BR_INTR_TIMEOUT {
                BR_WOKE
            } else {
                mentah
            };
            (flags, isig, r)
        }
        _ => (flags, isig, TAK_SAH),
    }
}

// Hasil blokir — cermin `Thread::BlockResult::Type` VERBATIM.
pub const BR_WOKE: u32 = 0;
pub const BR_NOT_BLOCKED: u32 = 1;
pub const BR_INTR_SIGNAL: u32 = 2;
pub const BR_INTR_DEATH: u32 = 3;
pub const BR_INTR_TIMEOUT: u32 = 4;

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_pembendung_daftar`.
    ProdDaftar = 0,
    /// `klatos_pembendung_lepas` yang berhasil.
    ProdLepas = 1,
    /// `klatos_pembendung_mulai` yang berhasil (B1).
    ProdMulai = 2,
    /// `klatos_pembendung_akhir` yang benar-benar memblokir.
    ProdAkhir = 3,
    /// `klatos_pembendung_intr_mati`.
    ProdIntrMati = 4,
    /// `klatos_pembendung_intr_sinyal`.
    ProdIntrSinyal = 5,
    /// `klatos_pembendung_hasil`.
    ProdHasil = 6,
    /// Handle basi / di luar tabel.
    ProdHandleSalah = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Diagnostik penolakan — mengapa `ProdHandleSalah` naik?
// ---------------------------------------------------------------------------
//
// Setiap titik penolakan melaporkan ID fungsinya (1..9) ke histogram di bawah,
// dan penolakan PERTAMA pada setiap fungsi merekam (index, gen, argumen).

/// Histogram penolakan per titik masuk. Indeks = ID fungsi (1..9).
static TOLAK: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
/// Salinan histogram pada saat `klatos_pembendung_tolak_snapshot()` dipanggil.
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

/// Diagnostik: jumlah penolakan pada titik masuk `fn_id` (1..9).
#[no_mangle]
pub extern "C" fn klatos_pembendung_tolak(fn_id: u32) -> u32 {
    TOLAK[(fn_id & 0xF) as usize].load(Ordering::Relaxed)
}

/// Diagnostik: jumlah penolakan SEJAK snapshot terakhir (produksi saja).
#[no_mangle]
pub extern "C" fn klatos_pembendung_tolak_prod(fn_id: u32) -> u32 {
    let i = (fn_id & 0xF) as usize;
    TOLAK[i]
        .load(Ordering::Relaxed)
        .saturating_sub(TOLAK_BASE[i].load(Ordering::Relaxed))
}

/// Diagnostik: ambil baseline dan mulai merekam detail lagi. Dipanggil
/// SESUDAH seluruh selftest selesai agar penolakan uji tidak tercampur.
#[no_mangle]
pub extern "C" fn klatos_pembendung_tolak_snapshot() -> u32 {
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
pub extern "C" fn klatos_pembendung_tolak_detail(fn_id: u32, sel: u32) -> u32 {
    let i = (fn_id & 0xF) as usize;
    match sel {
        0 => TOLAK_IDX[i].load(Ordering::Relaxed),
        1 => TOLAK_GEN[i].load(Ordering::Relaxed),
        _ => TOLAK_ARG[i].load(Ordering::Relaxed),
    }
}

// ---------------------------------------------------------------------------
// Tabel penghalang — array paralel, atomik
// ---------------------------------------------------------------------------

static PB_GEN: [AtomicU32; MAX_PENGHALANG] = [const { AtomicU32::new(0) }; MAX_PENGHALANG];
static PB_KIND: [AtomicU32; MAX_PENGHALANG] = [const { AtomicU32::new(0) }; MAX_PENGHALANG];
static PB_FLAGS: [AtomicU32; MAX_PENGHALANG] = [const { AtomicU32::new(0) }; MAX_PENGHALANG];
static PB_ISIG: [AtomicU32; MAX_PENGHALANG] = [const { AtomicU32::new(0) }; MAX_PENGHALANG];
static PB_HIDUP: [AtomicU32; MAX_PENGHALANG] = [const { AtomicU32::new(0) }; MAX_PENGHALANG];

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

/// Jumlah penghalang hidup PRODUKSI (dijaga inkremental).
static HIDUP: AtomicU32 = AtomicU32::new(0);
static PUNCAK: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// Resolusi handle
// ---------------------------------------------------------------------------

#[inline]
fn ph_sah(index: u32, gen: u32) -> bool {
    let i = index as usize;
    gen != 0 && i < MAX_PENGHALANG && PB_HIDUP[i].load(Ordering::Acquire) != 0
        && PB_GEN[i].load(Ordering::Relaxed) == gen
}

// ---------------------------------------------------------------------------
// Inti penghalang
// ---------------------------------------------------------------------------

fn ph_daftar_impl(lo: usize, hi: usize) -> u64 {
    lock();
    let mut slot: Option<usize> = None;
    let mut i = lo;
    while i < hi {
        if PB_HIDUP[i].load(Ordering::Relaxed) == 0 {
            slot = Some(i);
            break;
        }
        i += 1;
    }
    let hasil = if let Some(i) = slot {
        let g = PB_GEN[i].load(Ordering::Relaxed).wrapping_add(1);
        if g == 0 {
            PB_HIDUP[i].store(0, Ordering::Release);
            0u64
        } else {
            PB_KIND[i].store(PK_UNKNOWN, Ordering::Relaxed);
            PB_FLAGS[i].store(0, Ordering::Relaxed);
            PB_ISIG[i].store(0, Ordering::Relaxed);
            PB_GEN[i].store(g, Ordering::Relaxed);
            PB_HIDUP[i].store(1, Ordering::Release);
            ((g as u64) << 32) | (i as u64)
        }
    } else {
        0u64
    };
    unlock();
    if hasil != 0 && ((hasil & 0xFFFF_FFFF) as usize) < BATAS_PENGHALANG {
        let h = HIDUP.load(Ordering::Relaxed) + 1;
        HIDUP.store(h, Ordering::Relaxed);
        if h > PUNCAK.load(Ordering::Relaxed) {
            PUNCAK.store(h, Ordering::Relaxed);
        }
    }
    hasil
}

fn ph_lepas_impl(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        return 0;
    }
    lock();
    if !ph_sah(index, gen) {
        unlock();
        return 0;
    }
    let i = index as usize;
    PB_HIDUP[i].store(0, Ordering::Release);
    PB_FLAGS[i].store(0, Ordering::Relaxed);
    PB_ISIG[i].store(0, Ordering::Relaxed);
    PB_KIND[i].store(PK_UNKNOWN, Ordering::Relaxed);
    if i < BATAS_PENGHALANG {
        let h = HIDUP.load(Ordering::Relaxed);
        if h > 0 {
            HIDUP.store(h - 1, Ordering::Relaxed);
        }
    }
    unlock();
    1
}

/// B1 — `begin_blocking()`: uji-dan-set BLOCKING. Mengembalikan 1 bila tadinya
/// belum memblokir (sukses), 0 bila sudah memblokir (pelanggaran B1).
fn ph_mulai_impl(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    let lama = PB_FLAGS[i].fetch_or(PB_BLOCKING, Ordering::AcqRel);
    bump_ev(ev_mulai(lama));
    ((lama & PB_BLOCKING) == 0) as u32
}

/// Uji-dan-bersih BLOCKING. Mengembalikan bit BLOCKING **LAMA**:
/// 1 = tadinya memblokir, 0 = tadinya sudah bersih (B5).
///
/// Dipakai oleh `end_blocking()` (mengabaikan nilai kembali) DAN oleh
/// `unblock_from_blocker()` (yang early-return bila 0).
fn ph_akhir_impl(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    let lama = PB_FLAGS[i].fetch_and(!PB_BLOCKING, Ordering::AcqRel);
    bump_ev(ev_akhir(lama));
    ((lama & PB_BLOCKING) != 0) as u32
}

/// B7 — kematian tidak pernah menghapus sinyal.
fn ph_intr_mati_impl(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    PB_FLAGS[i].fetch_or(PB_INTR_DEATH, Ordering::AcqRel);
    bump_ev(EV_INTR_MATI);
    1
}

/// B6 — `set_interrupted_by_signal(s)`: `s != 0` dituntut.
fn ph_intr_sinyal_impl(index: u32, gen: u32, sig: u32) -> u32 {
    if !ph_sah(index, gen) {
        return 0;
    }
    if sig == 0 {
        bump_ev(EV_INTR_SINYAL_NOL);
        return 0;
    }
    let i = index as usize;
    PB_ISIG[i].store(sig & 0xFF, Ordering::Relaxed);
    bump_ev(EV_INTR_SINYAL);
    1
}

/// B11 — hanya jalur ini yang membersihkan sinyal penginterupsi.
fn ph_hapus_intr_sinyal_impl(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    PB_ISIG[i].store(0, Ordering::Relaxed);
    bump_ev(EV_HAPUS_INTR);
    1
}

/// B8 — `was_unblocked(did_timeout)`: menetapkan timeout hanya bila diminta.
/// B9 — `DID_TIMEOUT` lengket (tidak pernah dibersihkan di sini).
fn ph_lewat_waktu_impl(index: u32, gen: u32, did_timeout: u32) -> u32 {
    if !ph_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    if did_timeout != 0 {
        PB_FLAGS[i].fetch_or(PB_DID_TIMEOUT, Ordering::AcqRel);
    }
    bump_ev(ev_lewat(did_timeout));
    1
}

/// B2 + B12 — keputusan `block_result()`. `kind` diberikan oleh pemanggil
/// (virtual `blocker_type()`), dan dicatat sebagai otoritas jenis terakhir.
fn ph_hasil_impl(index: u32, gen: u32, kind: u32) -> u32 {
    if !ph_sah(index, gen) || kind >= PK_N_KIND {
        return TAK_SAH;
    }
    let i = index as usize;
    PB_KIND[i].store(kind, Ordering::Relaxed);
    let f = PB_FLAGS[i].load(Ordering::Relaxed);
    let isig = PB_ISIG[i].load(Ordering::Relaxed);
    // B2 — urutan death > signal > timeout > woke (hasil MENTAH).
    let mentah = if (f & PB_INTR_DEATH) != 0 {
        BR_INTR_DEATH
    } else if isig != 0 {
        BR_INTR_SIGNAL
    } else if (f & PB_DID_TIMEOUT) != 0 {
        BR_INTR_TIMEOUT
    } else {
        BR_WOKE
    };
    // B12 — timeout pada Sleep bukan interupsi.
    let hasil = if kind == PK_SLEEP && mentah == BR_INTR_TIMEOUT {
        BR_WOKE
    } else {
        mentah
    };
    bump_ev(ev_hasil(kind, mentah));
    hasil
}

/// Keanggotaan: 1 bila handle sah (baca-saja, tanpa penghitung).
fn ph_anggota_impl(index: u32, gen: u32) -> u32 {
    ph_sah(index, gen) as u32
}

/// Baca satu bidang logis. `which`: 0=flags 1=isig 2=kind 3=was_interrupted.
fn ph_baca_impl(index: u32, gen: u32, which: u32) -> u32 {
    if !ph_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    match which {
        PW_FLAGS => PB_FLAGS[i].load(Ordering::Relaxed),
        PW_ISIG => PB_ISIG[i].load(Ordering::Relaxed),
        PW_KIND => PB_KIND[i].load(Ordering::Relaxed),
        PW_WAS_INTR => {
            // B14 — death || signal != 0.
            let f = PB_FLAGS[i].load(Ordering::Relaxed);
            let isig = PB_ISIG[i].load(Ordering::Relaxed);
            (((f & PB_INTR_DEATH) != 0) || isig != 0) as u32
        }
        _ => TAK_SAH,
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Daftarkan penghalang. Mengembalikan `(generation << 32) | index`, atau 0
/// bila tabel penuh / generasi membungkus.
#[no_mangle]
pub extern "C" fn klatos_pembendung_daftar() -> u64 {
    bump(Counter::ProdDaftar);
    ph_daftar_impl(0, BATAS_PENGHALANG)
}

/// Lepaskan penghalang. 1 bila dihapus, 0 bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_pembendung_lepas(index: u32, gen: u32) -> u32 {
    let r = ph_lepas_impl(index, gen);
    if r == 0 {
        tolak(1, index, gen, 0);
    } else {
        bump(Counter::ProdLepas);
    }
    r
}

/// B1 — `begin_blocking()`: uji-dan-set BLOCKING. 1 = sukses.
#[no_mangle]
pub extern "C" fn klatos_pembendung_mulai(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        tolak(2, index, gen, 0);
        return 0;
    }
    let r = ph_mulai_impl(index, gen);
    if r == 1 {
        bump(Counter::ProdMulai);
    } else {
        tolak(2, index, gen, 0);
    }
    r
}

/// Uji-dan-bersih BLOCKING. Mengembalikan bit LAMA (1 = tadinya memblokir).
#[no_mangle]
pub extern "C" fn klatos_pembendung_akhir(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        tolak(3, index, gen, 0);
        return 0;
    }
    let r = ph_akhir_impl(index, gen);
    if r == 1 {
        bump(Counter::ProdAkhir);
    }
    r
}

/// B7 — set interupsi kematian (lengket).
#[no_mangle]
pub extern "C" fn klatos_pembendung_intr_mati(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        tolak(4, index, gen, 0);
        return 0;
    }
    bump(Counter::ProdIntrMati);
    ph_intr_mati_impl(index, gen)
}

/// B6 — set sinyal penginterupsi; `sig == 0` ditolak (bukan penolakan handle).
#[no_mangle]
pub extern "C" fn klatos_pembendung_intr_sinyal(index: u32, gen: u32, sig: u32) -> u32 {
    if !ph_sah(index, gen) {
        tolak(5, index, gen, sig);
        return 0;
    }
    let r = ph_intr_sinyal_impl(index, gen, sig);
    if r == 1 {
        bump(Counter::ProdIntrSinyal);
    }
    r
}

/// B11 — bersihkan sinyal penginterupsi (hanya WaitBlocker SIGCHLD).
#[no_mangle]
pub extern "C" fn klatos_pembendung_hapus_intr_sinyal(index: u32, gen: u32) -> u32 {
    if !ph_sah(index, gen) {
        tolak(6, index, gen, 0);
        return 0;
    }
    ph_hapus_intr_sinyal_impl(index, gen)
}

/// B8 — `was_unblocked(did_timeout)`.
#[no_mangle]
pub extern "C" fn klatos_pembendung_lewat_waktu(index: u32, gen: u32, did_timeout: u32) -> u32 {
    if !ph_sah(index, gen) {
        tolak(7, index, gen, did_timeout);
        return 0;
    }
    ph_lewat_waktu_impl(index, gen, did_timeout)
}

/// B2 + B12 — keputusan `block_result()`. `kind` dari `blocker_type()`.
#[no_mangle]
pub extern "C" fn klatos_pembendung_hasil(index: u32, gen: u32, kind: u32) -> u32 {
    if !ph_sah(index, gen) {
        tolak(8, index, gen, kind);
        return TAK_SAH;
    }
    if kind >= PK_N_KIND {
        // Handle sah, argumen di luar rentang: bukan penolakan handle.
        return TAK_SAH;
    }
    let r = ph_hasil_impl(index, gen, kind);
    if r == TAK_SAH {
        tolak(8, index, gen, kind);
    } else {
        bump(Counter::ProdHasil);
    }
    r
}

/// Keanggotaan tabel (1/0). Tanpa penghitung.
#[no_mangle]
pub extern "C" fn klatos_pembendung_anggota(index: u32, gen: u32) -> u32 {
    ph_anggota_impl(index, gen)
}

/// Baca satu bidang logis. `which`: 0=flags 1=isig 2=kind 3=was_interrupted.
#[no_mangle]
pub extern "C" fn klatos_pembendung_baca(index: u32, gen: u32, which: u32) -> u32 {
    if !ph_sah(index, gen) {
        tolak(9, index, gen, which);
        return TAK_SAH;
    }
    if which >= PW_N_WHICH {
        return TAK_SAH;
    }
    ph_baca_impl(index, gen, which)
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — audit
// ---------------------------------------------------------------------------

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_pembendung_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// P1.55 — permukaan audit peristiwa transisi
// ---------------------------------------------------------------------------

/// Nilai satu penghitung peristiwa (`EV_*`, 0..14).
#[no_mangle]
pub extern "C" fn klatos_pembendung_ev(which: u32) -> u32 {
    if which >= EV_N {
        return 0;
    }
    EV[which as usize].load(Ordering::Relaxed)
}

/// Nilai peristiwa SEJAK snapshot terakhir (produksi saja).
#[no_mangle]
pub extern "C" fn klatos_pembendung_ev_prod(which: u32) -> u32 {
    if which >= EV_N {
        return 0;
    }
    EV[which as usize]
        .load(Ordering::Relaxed)
        .saturating_sub(EV_BASE[which as usize].load(Ordering::Relaxed))
}

/// Ambil baseline peristiwa. Dipanggil SESUDAH seluruh selftest selesai,
/// supaya witness produksi tidak tercampur transisi uji.
#[no_mangle]
pub extern "C" fn klatos_pembendung_ev_snapshot() -> u32 {
    let mut i = 0usize;
    while i < 16 {
        EV_BASE[i].store(EV[i].load(Ordering::Relaxed), Ordering::Relaxed);
        i += 1;
    }
    EV_N
}

/// Penghalang hidup sekarang (rentang produksi).
#[no_mangle]
pub extern "C" fn klatos_pembendung_hidup() -> u32 {
    HIDUP.load(Ordering::Relaxed)
}

/// Puncak penghalang hidup.
#[no_mangle]
pub extern "C" fn klatos_pembendung_puncak() -> u32 {
    PUNCAK.load(Ordering::Relaxed)
}

/// Kapasitas produksi tabel.
#[no_mangle]
pub extern "C" fn klatos_pembendung_kapasitas() -> u32 {
    BATAS_PENGHALANG as u32
}

/// Reset HANYA rentang produksi (dipakai audit; bukan produksi).
#[no_mangle]
pub extern "C" fn klatos_pembendung_reset() -> u32 {
    lock();
    let mut i = 0;
    while i < BATAS_PENGHALANG {
        PB_HIDUP[i].store(0, Ordering::Release);
        PB_FLAGS[i].store(0, Ordering::Relaxed);
        PB_ISIG[i].store(0, Ordering::Relaxed);
        PB_KIND[i].store(PK_UNKNOWN, Ordering::Relaxed);
        i += 1;
    }
    HIDUP.store(0, Ordering::Release);
    PUNCAK.store(0, Ordering::Release);
    unlock();
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    for e in EV.iter() {
        e.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Oracle Naif — model independen semantik C++ historis
// ---------------------------------------------------------------------------

/// Model naif: keadaan logis sebagai array biasa, tanpa atomik. Bentuknya
/// SENGAJA berbeda dari produksi supaya tidak menyalin kesalahan yang sama.
struct Naif {
    flags: [u32; 32],
    isig: [u32; 32],
    kind: [u32; 32],
}

impl Naif {
    const fn baru() -> Self {
        Naif {
            flags: [0; 32],
            isig: [0; 32],
            kind: [0; 32],
        }
    }
    /// B1 — uji-dan-set. 1 = sukses.
    fn mulai(&mut self, i: usize) -> u32 {
        let lama = self.flags[i];
        self.flags[i] |= PB_BLOCKING;
        ((lama & PB_BLOCKING) == 0) as u32
    }
    /// Uji-dan-bersih. 1 = tadinya memblokir.
    fn akhir(&mut self, i: usize) -> u32 {
        let lama = self.flags[i];
        self.flags[i] &= !PB_BLOCKING;
        ((lama & PB_BLOCKING) != 0) as u32
    }
    /// B7.
    fn intr_mati(&mut self, i: usize) -> u32 {
        self.flags[i] |= PB_INTR_DEATH;
        1
    }
    /// B6.
    fn intr_sinyal(&mut self, i: usize, sig: u32) -> u32 {
        if sig == 0 {
            0
        } else {
            self.isig[i] = sig & 0xFF;
            1
        }
    }
    /// B11.
    fn hapus_intr(&mut self, i: usize) -> u32 {
        self.isig[i] = 0;
        1
    }
    /// B8 + B9.
    fn lewat(&mut self, i: usize, did_timeout: u32) -> u32 {
        if did_timeout != 0 {
            self.flags[i] |= PB_DID_TIMEOUT;
        }
        1
    }
    /// B2 + B12, sekaligus mencatat jenis. Jenis tak sah ditolak.
    fn hasil(&mut self, i: usize, kind: u32) -> u32 {
        if kind >= PK_N_KIND {
            return TAK_SAH;
        }
        self.kind[i] = kind;
        hasil_murni(kind, self.flags[i], self.isig[i])
    }
    /// Baca satu bidang.
    fn baca(&self, i: usize, which: u32) -> u32 {
        match which {
            PW_FLAGS => self.flags[i],
            PW_ISIG => self.isig[i],
            PW_KIND => self.kind[i],
            PW_WAS_INTR => {
                (((self.flags[i] & PB_INTR_DEATH) != 0) || self.isig[i] != 0) as u32
            }
            _ => TAK_SAH,
        }
    }
}

/// Keputusan murni B2 + B12 — dipakai sapuan menyeluruh.
fn hasil_murni(kind: u32, flags: u32, isig: u32) -> u32 {
    let r = if (flags & PB_INTR_DEATH) != 0 {
        BR_INTR_DEATH
    } else if isig != 0 {
        BR_INTR_SIGNAL
    } else if (flags & PB_DID_TIMEOUT) != 0 {
        BR_INTR_TIMEOUT
    } else {
        BR_WOKE
    };
    if kind == PK_SLEEP && r == BR_INTR_TIMEOUT {
        BR_WOKE
    } else {
        r
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

/// Cermin `KlatosPembendungReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosPembendungReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_daftar: u32,
    pub prod_lepas: u32,
    pub prod_mulai: u32,
    pub prod_akhir: u32,
    pub prod_intr_mati: u32,
    pub prod_intr_sinyal: u32,
    pub prod_hasil: u32,
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
const LO: usize = BATAS_PENGHALANG;
const HI: usize = MAX_PENGHALANG;

/// Reset rentang CADANGAN saja (tidak pernah menyentuh produksi, dan tidak
/// menyentuh penghitung produksi). Mengosongkan arena cadangan **beserta
/// generasinya**, sehingga matriks dapat dijalankan berulang kali dalam satu
/// boot dengan hasil yang identik.
fn reset_cadangan() {
    lock();
    let mut i = LO;
    while i < HI {
        PB_HIDUP[i].store(0, Ordering::Release);
        PB_GEN[i].store(0, Ordering::Relaxed);
        PB_FLAGS[i].store(0, Ordering::Relaxed);
        PB_ISIG[i].store(0, Ordering::Relaxed);
        PB_KIND[i].store(PK_UNKNOWN, Ordering::Relaxed);
        i += 1;
    }
    unlock();
}

fn run_matrix() -> KlatosPembendungReport {
    let mut m = Matrix::new();

    // --- kelompok 0: konstanta -------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5048_424C, Group::Peta);
    m.catat(MAX_PENGHALANG == 4096, Group::Peta);
    m.catat(BATAS_PENGHALANG == 4032, Group::Peta);
    m.catat(CADANGAN == 64, Group::Peta);
    m.catat(PK_N_KIND == 11, Group::Peta);
    m.catat(PB_SEMUA == 0x7, Group::Peta);
    m.catat(BR_WOKE == 0 && BR_NOT_BLOCKED == 1 && BR_INTR_SIGNAL == 2, Group::Peta);
    m.catat(BR_INTR_DEATH == 3 && BR_INTR_TIMEOUT == 4, Group::Peta);
    m.catat(klatos_pembendung_counter(99) == 0, Group::Peta);
    m.catat(klatos_pembendung_kapasitas() == 4032, Group::Peta);
    m.catat(klatos_pembendung_anggota(0, 0) == 0, Group::Peta);
    m.catat(klatos_pembendung_baca(0, 0, PW_FLAGS) == TAK_SAH, Group::Peta);
    m.catat(klatos_pembendung_hasil(0, 0, PK_FILE) == TAK_SAH, Group::Peta);

    // --- kelompok 1: siklus hidup penghalang (peta) -----------------------
    reset_cadangan();
    {
        let h = ph_daftar_impl(LO, HI);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        m.catat(h != 0, Group::Peta);
        m.catat(idx >= LO as u32 && (idx as usize) < HI, Group::Peta);
        m.catat(gen == 1, Group::Peta);
        m.catat(klatos_pembendung_anggota(idx, gen) == 1, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_FLAGS) == 0, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_ISIG) == 0, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_KIND) == PK_UNKNOWN, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_WAS_INTR) == 0, Group::Peta);

        // B1: mulai pertama sukses, kedua gagal.
        m.catat(ph_mulai_impl(idx, gen) == 1, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_FLAGS) == PB_BLOCKING, Group::Peta);
        m.catat(ph_mulai_impl(idx, gen) == 0, Group::Peta);
        // akhir pertama benar-benar memblokir, kedua sudah bersih.
        m.catat(ph_akhir_impl(idx, gen) == 1, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_FLAGS) == 0, Group::Peta);
        m.catat(ph_akhir_impl(idx, gen) == 0, Group::Peta);

        // B6: sig=0 tidak mengubah apa pun.
        m.catat(ph_intr_sinyal_impl(idx, gen, 0) == 0, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_ISIG) == 0, Group::Peta);
        m.catat(ph_intr_sinyal_impl(idx, gen, 17) == 1, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_ISIG) == 17, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_WAS_INTR) == 1, Group::Peta);
        // B11: hanya hapus yang membersihkan.
        m.catat(ph_intr_mati_impl(idx, gen) == 1, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_ISIG) == 17, Group::Peta);
        m.catat(ph_hapus_intr_sinyal_impl(idx, gen) == 1, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_ISIG) == 0, Group::Peta);
        // B7: kematian tetap ada meski sinyal dibersihkan.
        m.catat(ph_baca_impl(idx, gen, PW_WAS_INTR) == 1, Group::Peta);

        // B8: lewat_waktu(true) menetapkan; lewat_waktu(false) tidak.
        m.catat(ph_lewat_waktu_impl(idx, gen, 0) == 1, Group::Peta);
        m.catat((ph_baca_impl(idx, gen, PW_FLAGS) & PB_DID_TIMEOUT) == 0, Group::Peta);
        m.catat(ph_lewat_waktu_impl(idx, gen, 1) == 1, Group::Peta);
        m.catat((ph_baca_impl(idx, gen, PW_FLAGS) & PB_DID_TIMEOUT) != 0, Group::Peta);

        // Keputusan: death menang atas semuanya.
        m.catat(ph_hasil_impl(idx, gen, PK_FILE) == BR_INTR_DEATH, Group::Peta);
        m.catat(ph_baca_impl(idx, gen, PW_KIND) == PK_FILE, Group::Peta);
        // Bersihkan kematian secara manual (tidak ada jalur produksi, hanya
        // lewat reset slot) untuk menguji urutan berikutnya.
        let _ = ph_lepas_impl(idx, gen);
        m.catat(ph_baca_impl(idx, gen, PW_FLAGS) == TAK_SAH, Group::Peta);
    }

    // --- kelompok 2: setiap jenis, setiap kombinasi hasil (peta) ----------
    //
    // Sapuan menyeluruh: 11 jenis x 8 kombinasi bendera. Ketiga bit bendera
    // adalah `INTR_DEATH`, kehadiran sinyal (`ISIG != 0`), dan `DID_TIMEOUT`
    // — jadi kedelapan kombinasi menutup SELURUH ruang keputusan B2. Setiap
    // kombinasi memakai slot BARU agar keadaan bersih, dan hasilnya
    // dibandingkan dengan oracle murni `hasil_murni`.
    {
        let mut kind = 0u32;
        while kind < PK_N_KIND {
            let mut f = 0u32;
            while f < 8 {
                let isig = if (f & 2) != 0 { 17u32 } else { 0u32 };
                let h = ph_daftar_impl(LO, HI);
                let ii = (h & 0xFFFF_FFFF) as u32;
                let gg = (h >> 32) as u32;
                let mut flags = 0u32;
                if (f & 1) != 0 {
                    ph_intr_mati_impl(ii, gg);
                    flags |= PB_INTR_DEATH;
                }
                if (f & 2) != 0 {
                    ph_intr_sinyal_impl(ii, gg, isig);
                }
                if (f & 4) != 0 {
                    ph_lewat_waktu_impl(ii, gg, 1);
                    flags |= PB_DID_TIMEOUT;
                }
                let got = ph_hasil_impl(ii, gg, kind);
                let harap = hasil_murni(kind, flags, isig);
                m.catat(got == harap, Group::Peta);
                // jenis tercatat
                m.catat(ph_baca_impl(ii, gg, PW_KIND) == kind, Group::Peta);
                ph_lepas_impl(ii, gg);
                f += 1;
            }
            kind += 1;
        }
    }

    // --- kelompok 3: diferensial vs oracle Naif (>= 100.000 kasus) --------
    let mut dif_ukur: u32 = 0;
    let mut dif_beda: u32 = 0;
    {
        reset_cadangan();
        let mut naif = Naif::baru();
        let mut hh = [0u32; 16];
        let mut gg = [0u32; 16];
        let mut i = 0;
        while i < 16 {
            let h = ph_daftar_impl(LO, HI);
            hh[i] = (h & 0xFFFF_FFFF) as u32;
            gg[i] = (h >> 32) as u32;
            i += 1;
        }

        let mut rng = Lcg(0x0BAD_F00D_D1CE_5EED);
        let mut op = 0u32;
        while op < 120_000 {
            let r = rng.next();
            let slot = (r as usize) & 15;
            let aksi = (r >> 4) % 7;
            let idx = hh[slot];
            let gen = gg[slot];
            let (got, harap) = match aksi {
                0 => (ph_mulai_impl(idx, gen), naif.mulai(slot)),
                1 => (ph_akhir_impl(idx, gen), naif.akhir(slot)),
                2 => (ph_intr_mati_impl(idx, gen), naif.intr_mati(slot)),
                3 => {
                    let sig = if ((r >> 8) & 1) == 0 {
                        0u32
                    } else {
                        ((r >> 12) & 0x3F) + 1
                    };
                    (ph_intr_sinyal_impl(idx, gen, sig), naif.intr_sinyal(slot, sig))
                }
                4 => (
                    ph_hapus_intr_sinyal_impl(idx, gen),
                    naif.hapus_intr(slot),
                ),
                5 => {
                    let dt = (r >> 9) & 1;
                    (ph_lewat_waktu_impl(idx, gen, dt), naif.lewat(slot, dt))
                }
                _ => {
                    let kind = (r >> 10) % PK_N_KIND;
                    (ph_hasil_impl(idx, gen, kind), naif.hasil(slot, kind))
                }
            };
            dif_ukur += 1;
            if got != harap {
                dif_beda += 1;
            }
            let mut w = 0u32;
            while w < PW_N_WHICH {
                let a = ph_baca_impl(idx, gen, w);
                let b = naif.baca(slot, w);
                dif_ukur += 1;
                if a != b {
                    dif_beda += 1;
                }
                w += 1;
            }
            op += 1;
        }

        // Siklus pemakaian ulang slot: lepas lalu daftar lagi, generasi naik,
        // oracle ikut di-reset (slot baru = keadaan bersih).
        let mut z = 0usize;
        while z < 16 {
            m.catat(ph_lepas_impl(hh[z], gg[z]) == 1, Group::Peta);
            let h = ph_daftar_impl(LO, HI);
            hh[z] = (h & 0xFFFF_FFFF) as u32;
            gg[z] = (h >> 32) as u32;
            m.catat(gg[z] == 2, Group::Peta);
            naif.flags[z] = 0;
            naif.isig[z] = 0;
            naif.kind[z] = PK_UNKNOWN;
            m.catat(ph_baca_impl(hh[z], gg[z], PW_FLAGS) == 0, Group::Peta);
            z += 1;
        }
        // Handle generasi lama harus ditolak.
        m.catat(ph_anggota_impl(hh[0], 1) == 0, Group::Peta);

        let mut y = 0usize;
        while y < 16 {
            ph_lepas_impl(hh[y], gg[y]);
            y += 1;
        }
    }
    m.catat(dif_ukur >= 100_000, Group::Peta);
    m.catat(dif_beda == 0, Group::Peta);

    // --- kelompok 4: determinisme -----------------------------------------
    {
        reset_cadangan();
        let mut a = [0u32; 5];
        let mut b = [0u32; 5];
        let mut pass = 0;
        while pass < 2 {
            let h = ph_daftar_impl(LO, HI);
            let idx = (h & 0xFFFF_FFFF) as u32;
            let gen = (h >> 32) as u32;
            let mut r = [0u32; 5];
            r[0] = ph_mulai_impl(idx, gen);
            r[1] = ph_intr_sinyal_impl(idx, gen, 9);
            r[2] = ph_lewat_waktu_impl(idx, gen, 1);
            r[3] = ph_hasil_impl(idx, gen, PK_FUTEX);
            r[4] = ph_akhir_impl(idx, gen);
            ph_lepas_impl(idx, gen);
            if pass == 0 {
                a = r;
            } else {
                b = r;
            }
            pass += 1;
        }
        m.catat(a == b, Group::Peta);
        m.catat(a[0] == 1 && a[1] == 1 && a[2] == 1, Group::Peta);
        m.catat(a[3] == BR_INTR_SIGNAL, Group::Peta);
        m.catat(a[4] == 1, Group::Peta);
    }

    // --- kelompok 5: negatif ----------------------------------------------
    {
        // Handle tidak sah (belum pernah didaftarkan).
        m.catat(ph_mulai_impl(0, 0) == 0, Group::Neg);
        m.catat(ph_akhir_impl(0, 0) == 0, Group::Neg);
        m.catat(ph_anggota_impl(0, 0) == 0, Group::Neg);
        m.catat(ph_baca_impl(0, 0, PW_FLAGS) == TAK_SAH, Group::Neg);
        m.catat(ph_hasil_impl(0, 0, PK_FILE) == TAK_SAH, Group::Neg);
        m.catat(klatos_pembendung_lepas(0, 0) == 0, Group::Neg);
        m.catat(klatos_pembendung_mulai(0, 0) == 0, Group::Neg);
        m.catat(klatos_pembendung_baca(0, 0, PW_FLAGS) == TAK_SAH, Group::Neg);
        // Indeks di luar tabel.
        m.catat(ph_anggota_impl(MAX_PENGHALANG as u32, 1) == 0, Group::Neg);
        m.catat(ph_anggota_impl(0xFFFF_FFFF, 1) == 0, Group::Neg);
        // `which` di luar rentang pada handle SAH bukan penolakan handle.
        {
            reset_cadangan();
            let h = ph_daftar_impl(LO, HI);
            let idx = (h & 0xFFFF_FFFF) as u32;
            let gen = (h >> 32) as u32;
            m.catat(ph_baca_impl(idx, gen, PW_N_WHICH) == TAK_SAH, Group::Neg);
            m.catat(ph_baca_impl(idx, gen, 99) == TAK_SAH, Group::Neg);
            // Jenis di luar rentang pada handle SAH.
            m.catat(ph_hasil_impl(idx, gen, PK_N_KIND) == TAK_SAH, Group::Neg);
            m.catat(ph_baca_impl(idx, gen, PW_KIND) == PK_UNKNOWN, Group::Neg);
            ph_lepas_impl(idx, gen);
        }
        // B9/B10 lengket: tidak ada jalur yang membersihkan.
        {
            let h = ph_daftar_impl(LO, HI);
            let idx = (h & 0xFFFF_FFFF) as u32;
            let gen = (h >> 32) as u32;
            ph_intr_mati_impl(idx, gen);
            ph_lewat_waktu_impl(idx, gen, 1);
            let f1 = ph_baca_impl(idx, gen, PW_FLAGS);
            ph_mulai_impl(idx, gen);
            ph_akhir_impl(idx, gen);
            ph_lewat_waktu_impl(idx, gen, 0);
            ph_hapus_intr_sinyal_impl(idx, gen);
            let f2 = ph_baca_impl(idx, gen, PW_FLAGS);
            m.catat(f1 == f2, Group::Neg);
            m.catat((f2 & PB_INTR_DEATH) != 0, Group::Neg);
            m.catat((f2 & PB_DID_TIMEOUT) != 0, Group::Neg);
            ph_lepas_impl(idx, gen);
        }
    }

    // --- kelompok 6: kanal keabsahan vs nilai (pelajaran F-P149-07) --------
    //
    // Nilai bidang yang SAH tidak boleh pernah dihitung sebagai penolakan.
    {
        reset_cadangan();
        let h = ph_daftar_impl(LO, HI);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        let t0 = TOLAK[8].load(Ordering::Relaxed);
        // hasil dengan jenis SAH (Sleep) — keputusan sah, bukan penolakan.
        m.catat(klatos_pembendung_hasil(idx, gen, PK_SLEEP) == BR_WOKE, Group::Neg);
        m.catat(TOLAK[8].load(Ordering::Relaxed) == t0, Group::Neg);
        // baca bidang sah — bukan penolakan.
        let t1 = TOLAK[9].load(Ordering::Relaxed);
        let _ = klatos_pembendung_baca(idx, gen, PW_FLAGS);
        let _ = klatos_pembendung_baca(idx, gen, PW_ISIG);
        let _ = klatos_pembendung_baca(idx, gen, PW_KIND);
        let _ = klatos_pembendung_baca(idx, gen, PW_WAS_INTR);
        m.catat(TOLAK[9].load(Ordering::Relaxed) == t1, Group::Neg);
        ph_lepas_impl(idx, gen);
        // Dan penolakan handle tetap tercatat.
        let t2 = TOLAK[9].load(Ordering::Relaxed);
        m.catat(klatos_pembendung_baca(0, 0, PW_FLAGS) == TAK_SAH, Group::Neg);
        m.catat(TOLAK[9].load(Ordering::Relaxed) == t2 + 1, Group::Neg);
    }

    // --- kelompok 7: pembungkus C-ABI (peta) ------------------------------
    //
    // Membuktikan setiap pembungkus mengembalikan nilai yang benar. Slot
    // diambil lewat `ph_daftar_impl` (rentang CADANGAN) supaya matriks tidak
    // pernah menyentuh produksi — pembungkus `klatos_pembendung_daftar`
    // (rentang produksi) dibuktikan oleh boot, bukan oleh matriks.
    {
        reset_cadangan();
        let h = ph_daftar_impl(LO, HI);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        m.catat(gen != 0, Group::Peta);
        m.catat(klatos_pembendung_anggota(idx, gen) == 1, Group::Peta);
        m.catat(klatos_pembendung_mulai(idx, gen) == 1, Group::Peta);
        m.catat(klatos_pembendung_intr_sinyal(idx, gen, 17) == 1, Group::Peta);
        m.catat(klatos_pembendung_lewat_waktu(idx, gen, 1) == 1, Group::Peta);
        m.catat(klatos_pembendung_hasil(idx, gen, PK_SLEEP) == BR_INTR_SIGNAL, Group::Peta);
        m.catat(klatos_pembendung_intr_mati(idx, gen) == 1, Group::Peta);
        m.catat(klatos_pembendung_hasil(idx, gen, PK_FILE) == BR_INTR_DEATH, Group::Peta);
        m.catat(klatos_pembendung_hapus_intr_sinyal(idx, gen) == 1, Group::Peta);
        m.catat(klatos_pembendung_baca(idx, gen, PW_WAS_INTR) == 1, Group::Peta);
        m.catat(klatos_pembendung_akhir(idx, gen) == 1, Group::Peta);
        m.catat(klatos_pembendung_akhir(idx, gen) == 0, Group::Peta);
        m.catat(klatos_pembendung_lepas(idx, gen) == 1, Group::Peta);
        m.catat(klatos_pembendung_anggota(idx, gen) == 0, Group::Peta);
        m.catat(klatos_pembendung_baca(idx, gen, PW_FLAGS) == TAK_SAH, Group::Peta);
        // Jenis tak sah pada handle sah: nilai TAK_SAH, TIDAK menaikkan
        // `ProdHandleSalah`.
        let hs = ph_daftar_impl(LO, HI);
        let is = (hs & 0xFFFF_FFFF) as u32;
        let gs = (hs >> 32) as u32;
        let salah0 = klatos_pembendung_counter(Counter::ProdHandleSalah as u32);
        m.catat(klatos_pembendung_hasil(is, gs, 99) == TAK_SAH, Group::Peta);
        m.catat(
            klatos_pembendung_counter(Counter::ProdHandleSalah as u32) == salah0,
            Group::Peta,
        );
        // `which` tak sah pada handle sah juga bukan penolakan handle.
        let salah1 = klatos_pembendung_counter(Counter::ProdHandleSalah as u32);
        m.catat(klatos_pembendung_baca(is, gs, 99) == TAK_SAH, Group::Peta);
        m.catat(
            klatos_pembendung_counter(Counter::ProdHandleSalah as u32) == salah1,
            Group::Peta,
        );
        m.catat(klatos_pembendung_lepas(is, gs) == 1, Group::Peta);
    }

    // --- kelompok 8: MESIN KEADAAN TRANSISI (P1.55) -----------------------
    //
    // Untuk SETIAP keadaan (8 kombinasi 3 bit bendera x 2 nilai sinyal) dan
    // SETIAP peristiwa (7 peristiwa, dengan HASIL diulang untuk 11 jenis),
    // terapkan peristiwa ke slot yang disiapkan lalu bandingkan
    // (flags, isig, nilai kembali) dengan spesifikasi murni `spek_transisi`.
    // Sekaligus buktikan penghitung peristiwa naik TEPAT SATU, pada kelas
    // yang benar.
    {
        let isig_pil = [0u32, 17u32];
        let mut f = 0u32;
        while f < 8 {
            let mut si = 0usize;
            while si < 2 {
                let isig0 = isig_pil[si];
                let mut per = 0u32;
                while per < PER_N {
                    // HASIL diulang 11x (satu per jenis); INTR_SINYAL/LEWAT 2x
                    // (argumen 0 dan bukan-0); sisanya 1x.
                    let ulang: u32 = match per {
                        PER_HASIL => PK_N_KIND,
                        PER_INTR_SINYAL | PER_LEWAT => 2,
                        _ => 1,
                    };
                    let mut u = 0u32;
                    while u < ulang {
                        let arg: u32 = match per {
                            PER_INTR_SINYAL => {
                                if u == 0 {
                                    0
                                } else {
                                    17
                                }
                            }
                            PER_LEWAT => u,
                            PER_HASIL => u,
                            _ => 0,
                        };
                        reset_cadangan();
                        let h = ph_daftar_impl(LO, HI);
                        let ii = (h & 0xFFFF_FFFF) as u32;
                        let gg = (h >> 32) as u32;
                        if (f & 1) != 0 {
                            ph_mulai_impl(ii, gg);
                        }
                        if (f & 2) != 0 {
                            ph_intr_mati_impl(ii, gg);
                        }
                        if (f & 4) != 0 {
                            ph_lewat_waktu_impl(ii, gg, 1);
                        }
                        if isig0 != 0 {
                            ph_intr_sinyal_impl(ii, gg, isig0);
                        }
                        let flags0 = ph_baca_impl(ii, gg, PW_FLAGS);
                        let isig_b = ph_baca_impl(ii, gg, PW_ISIG);
                        m.catat(flags0 == f, Group::Peta);
                        m.catat(isig_b == isig0, Group::Peta);

                        let ev_kelas = match per {
                            PER_MULAI => ev_mulai(flags0),
                            PER_AKHIR => ev_akhir(flags0),
                            PER_INTR_MATI => EV_INTR_MATI,
                            PER_INTR_SINYAL => {
                                if arg == 0 {
                                    EV_INTR_SINYAL_NOL
                                } else {
                                    EV_INTR_SINYAL
                                }
                            }
                            PER_HAPUS_INTR => EV_HAPUS_INTR,
                            PER_LEWAT => ev_lewat(arg),
                            _ => {
                                let mentah = if (flags0 & PB_INTR_DEATH) != 0 {
                                    BR_INTR_DEATH
                                } else if isig0 != 0 {
                                    BR_INTR_SIGNAL
                                } else if (flags0 & PB_DID_TIMEOUT) != 0 {
                                    BR_INTR_TIMEOUT
                                } else {
                                    BR_WOKE
                                };
                                ev_hasil(arg, mentah)
                            }
                        };

                        let mut sebelum = [0u32; 16];
                        let mut k = 0usize;
                        while k < 16 {
                            sebelum[k] = EV[k].load(Ordering::Relaxed);
                            k += 1;
                        }

                        let dapat = match per {
                            PER_MULAI => ph_mulai_impl(ii, gg),
                            PER_AKHIR => ph_akhir_impl(ii, gg),
                            PER_INTR_MATI => ph_intr_mati_impl(ii, gg),
                            PER_INTR_SINYAL => ph_intr_sinyal_impl(ii, gg, arg),
                            PER_HAPUS_INTR => ph_hapus_intr_sinyal_impl(ii, gg),
                            PER_LEWAT => ph_lewat_waktu_impl(ii, gg, arg),
                            _ => ph_hasil_impl(ii, gg, arg),
                        };
                        let (hf, hi, hr) = spek_transisi(flags0, isig0, per, arg);
                        m.catat(dapat == hr, Group::Peta);
                        m.catat(ph_baca_impl(ii, gg, PW_FLAGS) == hf, Group::Peta);
                        m.catat(ph_baca_impl(ii, gg, PW_ISIG) == hi, Group::Peta);

                        let mut naik = 0u32;
                        let mut kk = 0usize;
                        while kk < 16 {
                            if EV[kk].load(Ordering::Relaxed) != sebelum[kk] {
                                naik += 1;
                            }
                            kk += 1;
                        }
                        m.catat(naik == 1, Group::Peta);
                        let ei = (ev_kelas & 0xF) as usize;
                        m.catat(
                            EV[ei].load(Ordering::Relaxed) == sebelum[ei].wrapping_add(1),
                            Group::Peta,
                        );

                        ph_lepas_impl(ii, gg);
                        u += 1;
                    }
                    per += 1;
                }
                si += 1;
            }
            f += 1;
        }
    }

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosPembendungReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_daftar: klatos_pembendung_counter(Counter::ProdDaftar as u32),
        prod_lepas: klatos_pembendung_counter(Counter::ProdLepas as u32),
        prod_mulai: klatos_pembendung_counter(Counter::ProdMulai as u32),
        prod_akhir: klatos_pembendung_counter(Counter::ProdAkhir as u32),
        prod_intr_mati: klatos_pembendung_counter(Counter::ProdIntrMati as u32),
        prod_intr_sinyal: klatos_pembendung_counter(Counter::ProdIntrSinyal as u32),
        prod_hasil: klatos_pembendung_counter(Counter::ProdHasil as u32),
        prod_handle_salah: klatos_pembendung_counter(Counter::ProdHandleSalah as u32),
        hidup: klatos_pembendung_hidup(),
        puncak: klatos_pembendung_puncak(),
        kapasitas: BATAS_PENGHALANG as u32,
        status,
    }
}

/// Selftest otoritas logis penghalang benang.
#[no_mangle]
pub extern "C" fn klatos_pembendung_selftest() -> KlatosPembendungReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5048_424C);
        assert!(MAX_PENGHALANG == 4096);
        assert!(CADANGAN == 64);
        assert!(BATAS_PENGHALANG == 4032);
        assert!(PK_UNKNOWN == 0 && PK_FILE == 1 && PK_FUTEX == 2);
        assert!(PK_PLAN9FS == 3 && PK_JOIN == 4 && PK_QUEUE == 5);
        assert!(PK_ROUTING == 6 && PK_SLEEP == 7 && PK_SIGNAL == 8);
        assert!(PK_WAIT == 9 && PK_FLOCK == 10 && PK_N_KIND == 11);
        assert!(PB_BLOCKING == 1 && PB_INTR_DEATH == 2 && PB_DID_TIMEOUT == 4);
        assert!(PB_SEMUA == 7);
        assert!(BR_WOKE == 0 && BR_NOT_BLOCKED == 1 && BR_INTR_SIGNAL == 2);
        assert!(BR_INTR_DEATH == 3 && BR_INTR_TIMEOUT == 4);
        assert!(PW_FLAGS == 0 && PW_ISIG == 1 && PW_KIND == 2 && PW_WAS_INTR == 3);
        assert!(PW_N_WHICH == 4);
        assert!(EV_MULAI == 0 && EV_MULAI_DITOLAK == 1);
        assert!(EV_AKHIR_BLOKIR == 2 && EV_AKHIR_BERSIH == 3);
        assert!(EV_INTR_MATI == 4 && EV_INTR_SINYAL == 5 && EV_INTR_SINYAL_NOL == 6);
        assert!(EV_HAPUS_INTR == 7 && EV_TIMEOUT == 8 && EV_TIMEOUT_NOOP == 9);
        assert!(EV_HASIL_WOKE == 10 && EV_HASIL_SINYAL == 11 && EV_HASIL_DEATH == 12);
        assert!(EV_HASIL_TIMEOUT == 13 && EV_HASIL_SLEEP_REMAP == 14);
        assert!(EV_N == 15);
        assert!(PER_MULAI == 0 && PER_AKHIR == 1 && PER_INTR_MATI == 2);
        assert!(PER_INTR_SINYAL == 3 && PER_HAPUS_INTR == 4 && PER_LEWAT == 5);
        assert!(PER_HASIL == 6 && PER_N == 7);
        assert!(Counter::ProdDaftar as usize == 0);
        assert!(Counter::ProdHandleSalah as usize == 7);
        assert!(N_COUNTER == 8);
        assert!(core::mem::size_of::<KlatosPembendungReport>() == 80);
        assert!(core::mem::align_of::<KlatosPembendungReport>() == 8);
    };
}
