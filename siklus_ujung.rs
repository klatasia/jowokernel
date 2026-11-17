// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8.3-B — **OTORITAS SIKLUS HIDUP UJUNG + MEKANISME ANTREAN (KlatIPC)**.


#![allow(unsafe_code)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Tag laporan; cermin `KlatosUjungReport` di `KlatSeam132.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_554A_4E47; // "KLATUJNG"

/// Indeks/deskriptor tidak sah.
pub const INVALID: u32 = 0xFFFF_FFFF;

/// Jumlah slot ujung maksimum (tetap, tanpa alokasi).
pub const MAX_UJUNG: usize = 1024;
/// Jumlah deskriptor pesan global maksimum.
pub const MAX_PESAN: usize = 8192;
/// Jumlah slot payload global (masing-masing `SLOT_BYTES`).
pub const POOL_SLOT: usize = 2048;
/// Ukuran satu slot payload; sama dengan `MAX_PESAN_UKURAN`.
pub const SLOT_BYTES: usize = 4096;

const POOL_BYTES: usize = POOL_SLOT * SLOT_BYTES; // 8 MiB bss

// --- batas kanal (satu-satunya otoritas mekanisme) --------------------------

/// Ukuran pesan maksimum satu pesan.
pub const MAX_PESAN_UKURAN: u32 = 4096;
/// Byte antrean maksimum per ujung.
pub const MAX_ANTREAN_BYTE: u64 = 64 * 1024;
/// Jumlah pesan maksimum per ujung.
pub const MAX_ANTREAN_PESAN: u32 = 1024;

// --- errno mirror (Jowo/Kontrak/POSIX/errno.h) ------------------------------

/// `EBADF` — handle usang/aspal.
pub const E_BADF: i32 = 9;
/// `EAGAIN` — belum dapat dilanjutkan (non-blok).
pub const E_AGAIN: i32 = 11;
/// `ENOMEM` — arena/pool habis.
pub const E_NOMEM: i32 = 12;
/// `EINVAL` — transisi/argumen tidak sah.
pub const E_INVAL: i32 = 22;
/// `EPIPE` — ujung sendiri sudah tertutup.
pub const E_PIPE: i32 = 32;
/// `EMSGSIZE` — pesan melebihi batas / buffer penerima terlalu kecil.
pub const E_MSGSIZE: i32 = 55;

/// Sentinel `terima`: antrean kosong DAN peer tidak tersedia (EOF).
pub const TERIMA_EOF: i32 = -1;

// --- status siklus hidup ----------------------------------------------------

/// `CREATE` — dibuat, belum ditautkan.
pub const ST_BIAR: u32 = 0;
/// `ACTIVE` — ditautkan/beroperasi.
pub const ST_AKTIF: u32 = 1;
/// `CLOSED` — ditutup eksplisit (idempoten).
pub const ST_TUTUP: u32 = 2;
/// `RELEASED` — slot bebas (objek musnah).
pub const ST_LEPAS: u32 = 3;

/// Penghitung untuk audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_ujung_buat` dari produksi.
    ProdBuat = 0,
    /// Aktivasi berhasil.
    ProdAktif = 1,
    /// Penautan pasangan berhasil.
    ProdTaut = 2,
    /// Penutupan pertama berhasil.
    ProdTutup = 3,
    /// Pelepasan berhasil.
    ProdLepas = 4,
    /// Kirim berhasil.
    ProdKirimOk = 5,
    /// Kirim ditolak.
    ProdKirimTolak = 6,
    /// Terima berhasil.
    ProdTerimaOk = 7,
    /// Terima ditolak.
    ProdTerimaTolak = 8,
    /// Penolakan handle usang/aspal.
    ProdTolakStale = 9,
    /// Penolakan ukuran (`EMSGSIZE`).
    ProdTolakBesar = 10,
    /// Penolakan ruang/habis (`EAGAIN`/`ENOMEM`).
    ProdTolakPenuh = 11,
    /// Penolakan kosong tak-siap (`EAGAIN`).
    ProdTolakKosong = 12,
}

const N_COUNTER: usize = 13;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Arena ujung — satu-satunya pemilik identitas + antrean
// ---------------------------------------------------------------------------

static UJ_GEN: [AtomicU32; MAX_UJUNG] = [const { AtomicU32::new(0) }; MAX_UJUNG];
static UJ_ST: [AtomicU32; MAX_UJUNG] = [const { AtomicU32::new(ST_LEPAS) }; MAX_UJUNG];
static UJ_PEER: [AtomicU32; MAX_UJUNG] = [const { AtomicU32::new(INVALID) }; MAX_UJUNG];
static UJ_QHEAD: [AtomicU32; MAX_UJUNG] = [const { AtomicU32::new(INVALID) }; MAX_UJUNG];
static UJ_QTAIL: [AtomicU32; MAX_UJUNG] = [const { AtomicU32::new(INVALID) }; MAX_UJUNG];
static UJ_QBYTES: [AtomicU64; MAX_UJUNG] = [const { AtomicU64::new(0) }; MAX_UJUNG];
static UJ_QCOUNT: [AtomicU32; MAX_UJUNG] = [const { AtomicU32::new(0) }; MAX_UJUNG];

static PES_SLOT: [AtomicU32; MAX_PESAN] = [const { AtomicU32::new(INVALID) }; MAX_PESAN];
static PES_NEXT: [AtomicU32; MAX_PESAN] = [const { AtomicU32::new(INVALID) }; MAX_PESAN];
static PES_LEN: [AtomicU32; MAX_PESAN] = [const { AtomicU32::new(0) }; MAX_PESAN];

static SLOT_NEXT: [AtomicU32; POOL_SLOT] = [const { AtomicU32::new(INVALID) }; POOL_SLOT];
static PES_FREE: AtomicU32 = AtomicU32::new(INVALID);
static SLOT_FREE: AtomicU32 = AtomicU32::new(INVALID);
static INIT: AtomicU32 = AtomicU32::new(0);
static LOCK: AtomicU32 = AtomicU32::new(0);
/// Status interupsi pemegang LOCK; hanya pemegang yang menulis.
static LOCK_FLAGS: AtomicU64 = AtomicU64::new(0);

/// Pool payload global. Diakses HANYA di bawah `LOCK`.
struct Pool(UnsafeCell<[u8; POOL_BYTES]>);
unsafe impl Sync for Pool {}
static POOL: Pool = Pool(UnsafeCell::new([0u8; POOL_BYTES]));

fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while LOCK.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    LOCK_FLAGS.store(flags, Ordering::Relaxed);
}

fn unlock() {
    let flags = LOCK_FLAGS.load(Ordering::Relaxed);
    LOCK.store(0, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Sekali: bangun rantai bebas deskriptor pesan.
fn pastikan_init() {
    if INIT.load(Ordering::Acquire) == 1 {
        return;
    }
    lock();
    if INIT.load(Ordering::Relaxed) == 0 {
        let mut i = 0usize;
        while i < MAX_PESAN {
            PES_SLOT[i].store(INVALID, Ordering::Relaxed);
            PES_LEN[i].store(0, Ordering::Relaxed);
            PES_NEXT[i].store(
                if i + 1 < MAX_PESAN { (i + 1) as u32 } else { INVALID },
                Ordering::Relaxed,
            );
            i += 1;
        }
        let mut s = 0usize;
        while s < POOL_SLOT {
            SLOT_NEXT[s].store(
                if s + 1 < POOL_SLOT { (s + 1) as u32 } else { INVALID },
                Ordering::Relaxed,
            );
            s += 1;
        }
        PES_FREE.store(0, Ordering::Relaxed);
        SLOT_FREE.store(0, Ordering::Relaxed);
        INIT.store(1, Ordering::Release);
    }
    unlock();
}

/// Apakah `(index, generation)` menunjuk objek ujung hidup (termasuk CLOSED)?
#[inline]
fn hidup(index: u32, generation: u32) -> bool {
    let i = index as usize;
    generation != 0
        && i < MAX_UJUNG
        && UJ_GEN[i].load(Ordering::Acquire) == generation
        && UJ_ST[i].load(Ordering::Acquire) != ST_LEPAS
}

/// Bongkar handle `u64` → `(index, generation)`.
#[inline]
fn pecah(h: u64) -> (u32, u32) {
    (h as u32, (h >> 32) as u32)
}

/// `(index, generation)` → handle `u64`.
#[inline]
fn handle_of(index: u32, generation: u32) -> u64 {
    ((generation as u64) << 32) | index as u64
}

/// Peer tersedia (dipanggil di bawah `LOCK`).
#[inline]
fn peer_tersedia_terkunci(i: usize) -> bool {
    let p = UJ_PEER[i].load(Ordering::Relaxed);
    if p == INVALID {
        return false;
    }
    let pi = p as usize;
    if pi >= MAX_UJUNG {
        return false;
    }
    let st = UJ_ST[pi].load(Ordering::Relaxed);
    st < ST_TUTUP && st != ST_LEPAS
}

fn alloc_pesan_terkunci() -> u32 {
    let d = PES_FREE.load(Ordering::Relaxed);
    if d == INVALID {
        return INVALID;
    }
    PES_FREE.store(PES_NEXT[d as usize].load(Ordering::Relaxed), Ordering::Relaxed);
    d
}

fn free_pesan_terkunci(d: u32) {
    PES_NEXT[d as usize].store(PES_FREE.load(Ordering::Relaxed), Ordering::Relaxed);
    PES_FREE.store(d, Ordering::Relaxed);
}

fn alloc_slot_terkunci() -> u32 {
    let s = SLOT_FREE.load(Ordering::Relaxed);
    if s == INVALID || s as usize >= POOL_SLOT {
        return INVALID;
    }
    SLOT_FREE.store(SLOT_NEXT[s as usize].load(Ordering::Relaxed), Ordering::Relaxed);
    s
}

fn free_slot_terkunci(s: u32) {
    if s == INVALID || s as usize >= POOL_SLOT {
        return;
    }
    SLOT_NEXT[s as usize].store(SLOT_FREE.load(Ordering::Relaxed), Ordering::Relaxed);
    SLOT_FREE.store(s, Ordering::Relaxed);
}

fn slot_ptr_terkunci(slot: u32) -> *mut u8 {
    let base = POOL.0.get() as *mut u8;
    unsafe { base.add(slot as usize * SLOT_BYTES) }
}

// ---------------------------------------------------------------------------
// Algoritma inti
// ---------------------------------------------------------------------------

/// `buat` — `CREATE`. Handle atau 0 bila arena penuh.
pub fn buat() -> u64 {
    pastikan_init();
    lock();
    let mut slot = INVALID;
    let mut i = 0usize;
    while i < MAX_UJUNG {
        if UJ_ST[i].load(Ordering::Relaxed) == ST_LEPAS {
            slot = i as u32;
            break;
        }
        i += 1;
    }
    let result = if slot == INVALID {
        0
    } else {
        let si = slot as usize;
        let generation = UJ_GEN[si].load(Ordering::Relaxed).wrapping_add(1);
        if generation == 0 {
            0
        } else {
            UJ_GEN[si].store(generation, Ordering::Relaxed);
            UJ_PEER[si].store(INVALID, Ordering::Relaxed);
            UJ_QHEAD[si].store(INVALID, Ordering::Relaxed);
            UJ_QTAIL[si].store(INVALID, Ordering::Relaxed);
            UJ_QBYTES[si].store(0, Ordering::Relaxed);
            UJ_QCOUNT[si].store(0, Ordering::Relaxed);
            UJ_ST[si].store(ST_BIAR, Ordering::Release);
            handle_of(slot, generation)
        }
    };
    unlock();
    result
}

/// `aktif` — `CREATE→ACTIVE` (idempoten pada `ACTIVE`).
pub fn aktif(index: u32, generation: u32) -> i32 {
    pastikan_init();
    lock();
    let r = if !hidup(index, generation) {
        -E_BADF
    } else {
        let i = index as usize;
        let st = UJ_ST[i].load(Ordering::Relaxed);
        if st == ST_BIAR {
            UJ_ST[i].store(ST_AKTIF, Ordering::Release);
            0
        } else if st == ST_AKTIF {
            0
        } else {
            -E_INVAL
        }
    };
    unlock();
    if r == 0 {
        bump(Counter::ProdAktif);
    } else if r == -E_BADF {
        bump(Counter::ProdTolakStale);
    }
    r
}

/// `taut` — tautkan dua ujung (peer dua arah) dan aktifkan keduanya.
pub fn taut(i1: u32, g1: u32, i2: u32, g2: u32) -> i32 {
    pastikan_init();
    let mut r = 0i32;
    lock();
    if !hidup(i1, g1) || !hidup(i2, g2) {
        r = -E_BADF;
    } else if i1 == i2 {
        r = -E_INVAL;
    } else {
        let (a, b) = (i1 as usize, i2 as usize);
        let (sa, sb) = (
            UJ_ST[a].load(Ordering::Relaxed),
            UJ_ST[b].load(Ordering::Relaxed),
        );
        if sa >= ST_TUTUP || sb >= ST_TUTUP {
            r = -E_INVAL;
        } else {
            let pa = UJ_PEER[a].load(Ordering::Relaxed);
            let pb = UJ_PEER[b].load(Ordering::Relaxed);
            if (pa != INVALID && pa != i2) || (pb != INVALID && pb != i1) {
                r = -E_INVAL;
            } else {
                UJ_PEER[a].store(i2, Ordering::Relaxed);
                UJ_PEER[b].store(i1, Ordering::Relaxed);
                if sa == ST_BIAR {
                    UJ_ST[a].store(ST_AKTIF, Ordering::Relaxed);
                }
                if sb == ST_BIAR {
                    UJ_ST[b].store(ST_AKTIF, Ordering::Relaxed);
                }
            }
        }
    }
    unlock();
    if r == 0 {
        bump(Counter::ProdTaut);
    } else if r == -E_BADF {
        bump(Counter::ProdTolakStale);
    }
    r
}

/// `tutup` — `→CLOSED` idempoten. `1` transisi pertama, `0` sudah/aspal.
pub fn tutup(index: u32, generation: u32) -> i32 {
    pastikan_init();
    lock();
    let first = if !hidup(index, generation) {
        0
    } else {
        let i = index as usize;
        if UJ_ST[i].load(Ordering::Relaxed) >= ST_TUTUP {
            0
        } else {
            UJ_ST[i].store(ST_TUTUP, Ordering::Release);
            1
        }
    };
    unlock();
    if first == 1 {
        bump(Counter::ProdTutup);
    } else {
        bump(Counter::ProdTolakStale);
    }
    first
}

/// `lepas` — `→RELEASED`, kuras antrean, lepas tautan peer. `1`/`0`.
pub fn lepas(index: u32, generation: u32) -> i32 {
    pastikan_init();
    lock();
    let r = if !hidup(index, generation) {
        0
    } else {
        let i = index as usize;
        let mut d = UJ_QHEAD[i].load(Ordering::Relaxed);
        while d != INVALID {
            let next = PES_NEXT[d as usize].load(Ordering::Relaxed);
            let slot = PES_SLOT[d as usize].load(Ordering::Relaxed);
            PES_SLOT[d as usize].store(INVALID, Ordering::Relaxed);
            PES_LEN[d as usize].store(0, Ordering::Relaxed);
            free_slot_terkunci(slot);
            free_pesan_terkunci(d);
            d = next;
        }
        UJ_QHEAD[i].store(INVALID, Ordering::Relaxed);
        UJ_QTAIL[i].store(INVALID, Ordering::Relaxed);
        UJ_QBYTES[i].store(0, Ordering::Relaxed);
        UJ_QCOUNT[i].store(0, Ordering::Relaxed);
        let p = UJ_PEER[i].load(Ordering::Relaxed);
        if p != INVALID
            && (p as usize) < MAX_UJUNG
            && UJ_PEER[p as usize].load(Ordering::Relaxed) == index
        {
            UJ_PEER[p as usize].store(INVALID, Ordering::Relaxed);
        }
        UJ_PEER[i].store(INVALID, Ordering::Relaxed);
        UJ_ST[i].store(ST_LEPAS, Ordering::Release);
        1
    };
    unlock();
    if r == 1 {
        bump(Counter::ProdLepas);
    } else {
        bump(Counter::ProdTolakStale);
    }
    r
}

/// `siap_kirim` — predikat penunggu: `1` = boleh lanjut, `0` = tunggu,
/// `<0` = `-errno` (tertutup/aspal).
pub fn siap_kirim(index: u32, generation: u32) -> i32 {
    pastikan_init();
    lock();
    let r = if !hidup(index, generation) {
        -E_BADF
    } else {
        let i = index as usize;
        if UJ_ST[i].load(Ordering::Relaxed) >= ST_TUTUP {
            -E_PIPE
        } else {
            let bytes = UJ_QBYTES[i].load(Ordering::Relaxed);
            let count = UJ_QCOUNT[i].load(Ordering::Relaxed);
            if count < MAX_ANTREAN_PESAN && bytes < MAX_ANTREAN_BYTE {
                1
            } else {
                0
            }
        }
    };
    unlock();
    r
}

/// `siap_terima` — `1` = boleh lanjut (ada pesan ATAU peer tidak tersedia),
/// `0` = tunggu, `<0` = `-errno`.
pub fn siap_terima(index: u32, generation: u32) -> i32 {
    pastikan_init();
    lock();
    let r = if !hidup(index, generation) {
        -E_BADF
    } else {
        let i = index as usize;
        if UJ_QHEAD[i].load(Ordering::Relaxed) != INVALID || !peer_tersedia_terkunci(i) {
            1
        } else {
            0
        }
    };
    unlock();
    r
}

/// `kirim` — `SEND → ENQUEUE`. `0` sukses, selainnya `-errno`.
///
/// Urutan verdict (persis perilaku `Endpoint::push` C++ lama):
/// ukuran → tertutup (`EPIPE`) → penuh (`EAGAIN`) → habis (`ENOMEM`).
#[allow(unused_assignments)]
pub fn kirim(index: u32, generation: u32, data: *const u8, len: u32) -> i32 {
    pastikan_init();
    let mut r = 0i32;
    lock();
    if !hidup(index, generation) {
        r = -E_BADF;
    } else {
        let i = index as usize;
        if len > MAX_PESAN_UKURAN {
            r = -E_MSGSIZE;
        } else if UJ_ST[i].load(Ordering::Relaxed) >= ST_TUTUP {
            r = -E_PIPE;
        } else {
            let bytes = UJ_QBYTES[i].load(Ordering::Relaxed);
            let count = UJ_QCOUNT[i].load(Ordering::Relaxed);
            if count >= MAX_ANTREAN_PESAN || bytes + len as u64 > MAX_ANTREAN_BYTE {
                r = -E_AGAIN;
            } else {
                let slot = alloc_slot_terkunci();
                if slot == INVALID {
                    r = -E_NOMEM;
                } else {
                    let d = alloc_pesan_terkunci();
                    if d == INVALID {
                        free_slot_terkunci(slot);
                        r = -E_NOMEM;
                    } else {
                        if len > 0 {
                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    data,
                                    slot_ptr_terkunci(slot),
                                    len as usize,
                                );
                            }
                        }
                        PES_SLOT[d as usize].store(slot, Ordering::Relaxed);
                        PES_LEN[d as usize].store(len, Ordering::Relaxed);
                        PES_NEXT[d as usize].store(INVALID, Ordering::Relaxed);
                        let tail = UJ_QTAIL[i].load(Ordering::Relaxed);
                        if tail == INVALID {
                            UJ_QHEAD[i].store(d, Ordering::Relaxed);
                        } else {
                            PES_NEXT[tail as usize].store(d, Ordering::Relaxed);
                        }
                        UJ_QTAIL[i].store(d, Ordering::Relaxed);
                        UJ_QBYTES[i].store(bytes + len as u64, Ordering::Relaxed);
                        UJ_QCOUNT[i].store(count + 1, Ordering::Relaxed);
                    }
                }
            }
        }
    }
    unlock();
    if r == 0 {
        bump(Counter::ProdKirimOk);
    } else if r == -E_MSGSIZE {
        bump(Counter::ProdKirimTolak);
        bump(Counter::ProdTolakBesar);
    } else if r == -E_BADF {
        bump(Counter::ProdKirimTolak);
        bump(Counter::ProdTolakStale);
    } else {
        bump(Counter::ProdKirimTolak);
        bump(Counter::ProdTolakPenuh);
    }
    r
}

/// `terima` — `RECEIVE → DEQUEUE`. `>=0` = panjang (dikonsumsi),
/// `TERIMA_EOF` (`-1`) = kosong + peer tidak tersedia, selainnya `-errno`.
#[allow(unused_assignments)]
pub fn terima(index: u32, generation: u32, out: *mut u8, cap: u32) -> i32 {
    pastikan_init();
    let mut r = 0i32;
    lock();
    if !hidup(index, generation) {
        r = -E_BADF;
    } else {
        let i = index as usize;
        let head = UJ_QHEAD[i].load(Ordering::Relaxed);
        if head == INVALID {
            r = if peer_tersedia_terkunci(i) { -E_AGAIN } else { TERIMA_EOF };
        } else {
            let len = PES_LEN[head as usize].load(Ordering::Relaxed);
            if len > cap {
                r = -E_MSGSIZE;
            } else {
                if len > 0 {
                    let slot = PES_SLOT[head as usize].load(Ordering::Relaxed);
                    unsafe {
                        core::ptr::copy_nonoverlapping(slot_ptr_terkunci(slot), out, len as usize);
                    }
                }
                let next = PES_NEXT[head as usize].load(Ordering::Relaxed);
                UJ_QHEAD[i].store(next, Ordering::Relaxed);
                if next == INVALID {
                    UJ_QTAIL[i].store(INVALID, Ordering::Relaxed);
                }
                let bytes = UJ_QBYTES[i].load(Ordering::Relaxed);
                let count = UJ_QCOUNT[i].load(Ordering::Relaxed);
                UJ_QBYTES[i].store(bytes - len as u64, Ordering::Relaxed);
                UJ_QCOUNT[i].store(count - 1, Ordering::Relaxed);
                let slot = PES_SLOT[head as usize].load(Ordering::Relaxed);
                PES_SLOT[head as usize].store(INVALID, Ordering::Relaxed);
                PES_LEN[head as usize].store(0, Ordering::Relaxed);
                free_slot_terkunci(slot);
                free_pesan_terkunci(head);
                r = len as i32;
            }
        }
    }
    unlock();
    if r >= 0 {
        bump(Counter::ProdTerimaOk);
    } else {
        bump(Counter::ProdTerimaTolak);
        if r == -E_AGAIN {
            bump(Counter::ProdTolakKosong);
        }
    }
    r
}

/// `peek` — panjang pesan terdepan tanpa mengonsumsinya.
pub fn peek(index: u32, generation: u32) -> i32 {
    pastikan_init();
    lock();
    let r = if !hidup(index, generation) {
        -E_BADF
    } else {
        let i = index as usize;
        let head = UJ_QHEAD[i].load(Ordering::Relaxed);
        if head == INVALID {
            if peer_tersedia_terkunci(i) { -E_AGAIN } else { TERIMA_EOF }
        } else {
            PES_LEN[head as usize].load(Ordering::Relaxed) as i32
        }
    };
    unlock();
    r
}

/// Status siklus hidup, atau [`INVALID`] bila usang/aspal.
pub fn status(index: u32, generation: u32) -> u32 {
    pastikan_init();
    if !hidup(index, generation) {
        return INVALID;
    }
    UJ_ST[index as usize].load(Ordering::Acquire)
}

/// `1` bila peer ada dan belum tertutup.
pub fn peer_tersedia(index: u32, generation: u32) -> i32 {
    pastikan_init();
    lock();
    let r = if hidup(index, generation) && peer_tersedia_terkunci(index as usize) {
        1
    } else {
        0
    };
    unlock();
    r
}

/// Jumlah pesan terantre.
pub fn qcount(index: u32, generation: u32) -> u32 {
    pastikan_init();
    if !hidup(index, generation) {
        return 0;
    }
    UJ_QCOUNT[index as usize].load(Ordering::Acquire)
}

/// Jumlah byte terantre.
pub fn qbytes(index: u32, generation: u32) -> u64 {
    pastikan_init();
    if !hidup(index, generation) {
        return 0;
    }
    UJ_QBYTES[index as usize].load(Ordering::Acquire)
}

/// Jumlah ujung hidup saat ini.
pub fn jumlah() -> u32 {
    pastikan_init();
    let mut n = 0u32;
    let mut i = 0usize;
    while i < MAX_UJUNG {
        if UJ_ST[i].load(Ordering::Relaxed) != ST_LEPAS {
            n += 1;
        }
        i += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

/// Buat ujung (`CREATE`). `0` = arena penuh.
#[no_mangle]
pub extern "C" fn klatos_ujung_buat() -> u64 {
    let h = buat();
    if h == 0 {
        bump(Counter::ProdTolakPenuh);
    } else {
        bump(Counter::ProdBuat);
    }
    h
}

/// `1`/`0` apakah handle hidup.
#[no_mangle]
pub extern "C" fn klatos_ujung_hidup(index: u32, generation: u32) -> u32 {
    let r = if hidup(index, generation) { 1 } else { 0 };
    if r == 0 {
        bump(Counter::ProdTolakStale);
    }
    r
}

/// Status siklus hidup atau `INVALID`.
#[no_mangle]
pub extern "C" fn klatos_ujung_status(index: u32, generation: u32) -> u32 {
    status(index, generation)
}

/// `CREATE→ACTIVE`; `0` sukses, `-errno` gagal.
#[no_mangle]
pub extern "C" fn klatos_ujung_aktif(index: u32, generation: u32) -> i32 {
    aktif(index, generation)
}

/// Tautkan pasangan; `0` sukses, `-errno` gagal.
#[no_mangle]
pub extern "C" fn klatos_ujung_taut(i1: u32, g1: u32, i2: u32, g2: u32) -> i32 {
    taut(i1, g1, i2, g2)
}

/// Tutup (`→CLOSED`). `1` transisi pertama, `0` sudah/aspal.
#[no_mangle]
pub extern "C" fn klatos_ujung_tutup(index: u32, generation: u32) -> i32 {
    tutup(index, generation)
}

/// Lepas (`→RELEASED`). `1` sukses, `0` sudah/aspal.
#[no_mangle]
pub extern "C" fn klatos_ujung_lepas(index: u32, generation: u32) -> i32 {
    lepas(index, generation)
}

/// Predikat penunggu kirim.
#[no_mangle]
pub extern "C" fn klatos_ujung_siap_kirim(index: u32, generation: u32) -> i32 {
    siap_kirim(index, generation)
}

/// Predikat penunggu terima.
#[no_mangle]
pub extern "C" fn klatos_ujung_siap_terima(index: u32, generation: u32) -> i32 {
    siap_terima(index, generation)
}

/// `SEND → ENQUEUE`. `0` sukses, `-errno` gagal.
#[no_mangle]
pub extern "C" fn klatos_ujung_kirim(index: u32, generation: u32, data: *const u8, len: u32) -> i32 {
    kirim(index, generation, data, len)
}

/// `RECEIVE → DEQUEUE`. `>=0` panjang, `-1` EOF, `-errno` gagal.
#[no_mangle]
pub extern "C" fn klatos_ujung_terima(index: u32, generation: u32, out: *mut u8, cap: u32) -> i32 {
    terima(index, generation, out, cap)
}

/// Panjang pesan terdepan tanpa konsumsi.
#[no_mangle]
pub extern "C" fn klatos_ujung_peek(index: u32, generation: u32) -> i32 {
    peek(index, generation)
}

/// `1` bila peer tersedia.
#[no_mangle]
pub extern "C" fn klatos_ujung_peer_tersedia(index: u32, generation: u32) -> i32 {
    peer_tersedia(index, generation)
}

/// Jumlah pesan terantre.
#[no_mangle]
pub extern "C" fn klatos_ujung_qcount(index: u32, generation: u32) -> u32 {
    qcount(index, generation)
}

/// Jumlah byte terantre.
#[no_mangle]
pub extern "C" fn klatos_ujung_qbytes(index: u32, generation: u32) -> u64 {
    qbytes(index, generation)
}

/// Jumlah ujung hidup.
#[no_mangle]
pub extern "C" fn klatos_ujung_jumlah() -> u32 {
    jumlah()
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_ujung_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTERS[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Nol-kan penghitung. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_ujung_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosUjungReport` di `KlatSeam132.h`.
#[repr(C)]
pub struct KlatosUjungReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_aktif: u32,
    pub prod_taut: u32,
    pub prod_tutup: u32,
    pub prod_lepas: u32,
    pub prod_kirim_ok: u32,
    pub prod_kirim_tolak: u32,
    pub prod_terima_ok: u32,
    pub prod_terima_tolak: u32,
    pub prod_tolak_stale: u32,
    pub prod_tolak_besar: u32,
    pub prod_tolak_penuh: u32,
    pub prod_tolak_kosong: u32,
    pub status: i32,
}

const _: [(); 88] = [(); core::mem::size_of::<KlatosUjungReport>()];

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

// --- model antrean independen untuk diferensial -----------------------------

const MODEL_CAP: usize = 1024;

/// Model FIFO independen (referensi diferensial, ditulis ulang di sini).
struct Model {
    lens: [u32; MODEL_CAP],
    head: usize,
    count: usize,
    bytes: u64,
    tutup_kirim: bool,
}

impl Model {
    fn new() -> Self {
        Model {
            lens: [0u32; MODEL_CAP],
            head: 0,
            count: 0,
            bytes: 0,
            tutup_kirim: false,
        }
    }
    fn kirim(&mut self, len: u32) -> i32 {
        if len > MAX_PESAN_UKURAN {
            return -E_MSGSIZE;
        }
        if self.tutup_kirim {
            return -E_PIPE;
        }
        if self.count >= MAX_ANTREAN_PESAN as usize || self.bytes + len as u64 > MAX_ANTREAN_BYTE {
            return -E_AGAIN;
        }
        self.lens[(self.head + self.count) % MODEL_CAP] = len;
        self.count += 1;
        self.bytes += len as u64;
        0
    }
    fn terima(&mut self, cap: u32, peer_tertutup: bool) -> i32 {
        if self.count == 0 {
            return if peer_tertutup { TERIMA_EOF } else { -E_AGAIN };
        }
        let len = self.lens[self.head];
        if len > cap {
            return -E_MSGSIZE;
        }
        self.head = (self.head + 1) % MODEL_CAP;
        self.count -= 1;
        self.bytes -= len as u64;
        len as i32
    }
    fn kuras(&mut self) {
        self.head = 0;
        self.count = 0;
        self.bytes = 0;
    }
}

/// Matriks otoritas siklus ujung. Input SINTETIS.
fn run_matrix() -> KlatosUjungReport {
    let mut m = Matrix::new();
    let mut buf = [0u8; 4096];

    // --- G1: konstanta ------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_554A_4E47);
    m.catat(MAX_PESAN_UKURAN == 4096);
    m.catat(MAX_ANTREAN_BYTE == 64 * 1024);
    m.catat(MAX_ANTREAN_PESAN == 1024);
    m.catat(E_BADF == 9 && E_AGAIN == 11 && E_NOMEM == 12 && E_INVAL == 22 && E_PIPE == 32 && E_MSGSIZE == 55);
    m.catat(ST_BIAR == 0 && ST_AKTIF == 1 && ST_TUTUP == 2 && ST_LEPAS == 3);
    m.catat(TERIMA_EOF == -1);

    // Jumlah ujung hidup SEBELUM matriks. Di runtime kernel, subsistem lain
    // (mis. kanal dispatch) dapat sudah memiliki ujung; karena itu matriks ini
    // menghitung RELATIF (awal → akhir), bukan mengandaikan arena kosong.
    let jumlah_awal = jumlah();

    // --- G2: siklus hidup CREATE → ACTIVE → CLOSED → RELEASED ---------------
    let a = buat();
    m.catat(a != 0);
    let (ai, ag) = pecah(a);
    m.catat(ag >= 1);
    m.catat(hidup(ai, ag));
    m.catat(status(ai, ag) == ST_BIAR);
    m.catat(aktif(ai, ag) == 0);
    m.catat(status(ai, ag) == ST_AKTIF);
    m.catat(aktif(ai, ag) == 0);
    let b = buat();
    m.catat(b != 0);
    let (bi, bg) = pecah(b);
    m.catat(taut(ai, ag, bi, bg) == 0);
    m.catat(status(ai, ag) == ST_AKTIF && status(bi, bg) == ST_AKTIF);
    m.catat(peer_tersedia(ai, ag) == 1 && peer_tersedia(bi, bg) == 1);
    m.catat(tutup(ai, ag) == 1);
    m.catat(status(ai, ag) == ST_TUTUP);
    m.catat(peer_tersedia(bi, bg) == 0);
    m.catat(tutup(ai, ag) == 0); // tutup ganda
    m.catat(aktif(ai, ag) == -E_INVAL); // CLOSED → ACTIVE ditolak
    m.catat(lepas(ai, ag) == 1);
    m.catat(status(ai, ag) == INVALID);
    m.catat(lepas(ai, ag) == 0); // lepas ganda
    m.catat(aktif(ai, ag) == -E_BADF); // RELEASED → ACTIVE ditolak
    m.catat(peer_tersedia(bi, bg) == 0);
    m.catat(lepas(bi, bg) == 1);

    // --- G3: negatif — usang/aspal/generasi salah ---------------------------
    m.neg(!hidup(ai, ag));
    m.neg(!hidup(ai, ag.wrapping_add(1)));
    m.neg(!hidup(ai, 0));
    m.neg(!hidup(MAX_UJUNG as u32, 1));
    m.neg(!hidup(INVALID, 1));
    m.neg(status(ai, ag) == INVALID);
    m.neg(aktif(INVALID, 1) == -E_BADF);
    m.neg(tutup(INVALID, 1) == 0);
    m.neg(lepas(MAX_UJUNG as u32, 1) == 0);
    m.neg(kirim(ai, ag, core::ptr::null(), 1) == -E_BADF);
    m.neg(terima(ai, ag, core::ptr::null_mut(), 4096) == -E_BADF);
    m.neg(peek(ai, ag) == -E_BADF);
    m.neg(siap_kirim(ai, ag) == -E_BADF);
    m.neg(siap_terima(ai, ag) == -E_BADF);
    m.neg(qcount(ai, ag) == 0 && qbytes(ai, ag) == 0);
    m.neg(taut(ai, ag, ai, ag) == -E_BADF);

    // Anti-ABA: slot dipakai ulang → generasi naik, handle lama ditolak.
    let a2 = buat();
    let (a2i, a2g) = pecah(a2);
    // Anti-ABA: handle BARU sah, dan handle LAMA (generasi lama) ditolak.
    // Kesetaraan slot TIDAK diandaikan (runtime kernel dapat punya slot terpakai).
    m.catat(a2g >= 1);
    m.catat(hidup(a2i, a2g));
    // Anti-ABA: handle LAMA (slot+generasi lama) wajib mati, dan penanganan
    // baru tidak boleh identik dengan yang lama. Kesetaraan slot TIDAK
    // diandaikan: di runtime kernel arena tidak kosong sehingga `buat()` dapat
    // memilih slot lain (bukan slot `a`).
    m.neg(!hidup(ai, ag));
    m.catat(a2i != ai || a2g != ag);
    m.catat(lepas(a2i, a2g) == 1);

    // --- G4: antrean — SEND → ENQUEUE → RECEIVE → DEQUEUE -------------------
    let s = buat();
    let rt = buat();
    let (si, sg) = pecah(s);
    let (ri, rg) = pecah(rt);
    m.catat(taut(si, sg, ri, rg) == 0);
    m.catat(peek(si, sg) == -E_AGAIN); // kosong + peer tersedia
    m.catat(terima(si, sg, buf.as_mut_ptr(), 4096) == -E_AGAIN);
    m.catat(siap_terima(si, sg) == 0);
    for i in 0..4 {
        buf[i] = 0x11 + i as u8;
    }
    m.catat(kirim(si, sg, buf.as_ptr(), 4) == 0);
    m.catat(siap_terima(si, sg) == 1);
    m.catat(qcount(si, sg) == 1 && qbytes(si, sg) == 4);
    m.catat(peek(si, sg) == 4);
    let mut out = [0u8; 8];
    m.catat(terima(si, sg, out.as_mut_ptr(), 8) == 4);
    m.catat(out[0] == 0x11 && out[1] == 0x12 && out[2] == 0x13 && out[3] == 0x14);
    m.catat(qcount(si, sg) == 0 && qbytes(si, sg) == 0);
    m.catat(terima(si, sg, out.as_mut_ptr(), 8) == -E_AGAIN);
    // beberapa pesan + urutan FIFO
    let m1 = [1u8; 3];
    let m2 = [2u8; 5];
    let m3 = [3u8; 1];
    m.catat(kirim(si, sg, m1.as_ptr(), 3) == 0);
    m.catat(kirim(si, sg, m2.as_ptr(), 5) == 0);
    m.catat(kirim(si, sg, m3.as_ptr(), 1) == 0);
    m.catat(qcount(si, sg) == 3 && qbytes(si, sg) == 9);
    m.catat(terima(si, sg, out.as_mut_ptr(), 8) == 3 && out[0] == 1);
    m.catat(terima(si, sg, out.as_mut_ptr(), 8) == 5 && out[0] == 2);
    m.catat(terima(si, sg, out.as_mut_ptr(), 8) == 1 && out[0] == 3);
    m.catat(qcount(si, sg) == 0);
    // pesan nol-panjang sah
    m.catat(kirim(si, sg, core::ptr::null(), 0) == 0);
    m.catat(peek(si, sg) == 0);
    m.catat(terima(si, sg, out.as_mut_ptr(), 8) == 0);
    // ukuran > batas → EMSGSIZE (tidak masuk antrean)
    m.catat(kirim(si, sg, buf.as_ptr(), MAX_PESAN_UKURAN + 1) == -E_MSGSIZE);
    m.catat(qcount(si, sg) == 0);
    // buffer penerima terlalu kecil → EMSGSIZE (pesan tetap di antrean)
    m.catat(kirim(si, sg, buf.as_ptr(), 4) == 0);
    m.catat(terima(si, sg, out.as_mut_ptr(), 3) == -E_MSGSIZE);
    m.catat(qcount(si, sg) == 1);
    m.catat(terima(si, sg, out.as_mut_ptr(), 4) == 4);
    m.catat(qcount(si, sg) == 0);
    // antrean penuh (batas byte) → EAGAIN
    {
        buf[0] = 7;
        let mut n = 0u32;
        while n < 16 {
            n += 1;
            m.catat(kirim(si, sg, buf.as_ptr(), 4096) == 0);
        }
        m.catat(qbytes(si, sg) == 64 * 1024);
        m.catat(kirim(si, sg, buf.as_ptr(), 1) == -E_AGAIN);
        m.catat(siap_kirim(si, sg) == 0);
        let mut drained = 0u32;
        loop {
            let rr = terima(si, sg, buf.as_mut_ptr(), 4096);
            if rr < 0 {
                break;
            }
            drained += 1;
            if drained > 100 {
                break;
            }
        }
        m.catat(drained == 16);
        m.catat(qbytes(si, sg) == 0);
        m.catat(siap_kirim(si, sg) == 1);
    }
    // penutupan: ujung sendiri tertutup → EPIPE pada kirim
    m.catat(tutup(si, sg) == 1);
    m.catat(kirim(si, sg, buf.as_ptr(), 1) == -E_PIPE);
    m.catat(tutup(si, sg) == 0);
    // ujung lain: peer tertutup → EOF saat antrean kosong
    m.catat(siap_terima(ri, rg) == 1);
    m.catat(peek(ri, rg) == TERIMA_EOF);
    m.catat(terima(ri, rg, out.as_mut_ptr(), 8) == TERIMA_EOF);
    m.catat(lepas(ri, rg) == 1);
    m.catat(lepas(si, sg) == 1);
    // pesan tersisa tetap terkuras walau peer tertutup
    let s2 = buat();
    let r2 = buat();
    let (s2i, s2g) = pecah(s2);
    let (r2i, r2g) = pecah(r2);
    m.catat(taut(s2i, s2g, r2i, r2g) == 0);
    // Pesan masuk ke antrean PENERIMA (persis `peer->push` C++); pengirim lalu
    // menutup. Penerima tetap dapat menguras sisa, lalu melihat EOF.
    m.catat(kirim(r2i, r2g, m1.as_ptr(), 3) == 0);
    m.catat(tutup(s2i, s2g) == 1);
    m.catat(terima(r2i, r2g, out.as_mut_ptr(), 8) == 3);
    m.catat(terima(r2i, r2g, out.as_mut_ptr(), 8) == TERIMA_EOF);
    m.catat(lepas(r2i, r2g) == 1);
    m.catat(lepas(s2i, s2g) == 1);
    m.catat(jumlah() == jumlah_awal);

    // --- G5: taut ganda / peer ganda ditolak --------------------------------
    let p1 = buat();
    let p2 = buat();
    let p3 = buat();
    let (p1i, p1g) = pecah(p1);
    let (p2i, p2g) = pecah(p2);
    let (p3i, p3g) = pecah(p3);
    m.neg(taut(p1i, p1g, p1i, p1g) == -E_INVAL);
    m.catat(taut(p1i, p1g, p2i, p2g) == 0);
    m.neg(taut(p1i, p1g, p3i, p3g) == -E_INVAL);
    m.neg(taut(p1i, p1g.wrapping_add(1), p2i, p2g) == -E_BADF);
    m.catat(tutup(p1i, p1g) == 1);
    m.neg(taut(p1i, p1g, p2i, p2g) == -E_INVAL);
    m.catat(lepas(p1i, p1g) == 1);
    m.catat(lepas(p2i, p2g) == 1);
    m.catat(lepas(p3i, p3g) == 1);
    m.catat(jumlah() == jumlah_awal);

    // --- G6: diferensial — model antrean independen (100k kasus) ------------
    let s3 = buat();
    let r3 = buat();
    let (s3i, s3g) = pecah(s3);
    let (r3i, r3g) = pecah(r3);
    m.catat(taut(s3i, s3g, r3i, r3g) == 0);
    // Dua antrean: `model_s` = antrean ujung s3, `model_r` = antrean ujung r3.
    let mut model_s = Model::new();
    let mut model_r = Model::new();
    let mut probe: u64 = 0x5EED_1234;
    let mut n = 0u32;
    while n < 100_000 {
        probe = probe
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let op = (probe >> 60) & 0x3;
        let len = ((probe >> 33) & 0x3F) as u32;
        match op {
            0 => {
                // SEND ke ujung s3 → antrean s3.
                let got = kirim(s3i, s3g, buf.as_ptr(), len);
                let want = model_s.kirim(len);
                m.dif(got == want);
            }
            1 | 2 => {
                // RECEIVE dari ujung r3 → antrean r3 (peer = s3).
                let got = terima(r3i, r3g, buf.as_mut_ptr(), 4096);
                let want = model_r.terima(4096, model_s.tutup_kirim);
                m.dif(got == want);
            }
            _ => {
                // SEND ke ujung r3 → antrean r3.
                let got = kirim(r3i, r3g, buf.as_ptr(), len);
                let want = model_r.kirim(len);
                m.dif(got == want);
            }
        }
        n += 1;
    }
    // Kuras kedua antrean dan bandingkan dengan model.
    let mut drain_s = 0u32;
    let mut drain_r = 0u32;
    loop {
        let rr = terima(r3i, r3g, buf.as_mut_ptr(), 4096);
        if rr < 0 {
            break;
        }
        drain_r += 1;
        if drain_r > 1024 {
            break;
        }
    }
    loop {
        let rr = terima(s3i, s3g, buf.as_mut_ptr(), 4096);
        if rr < 0 {
            break;
        }
        drain_s += 1;
        if drain_s > 1024 {
            break;
        }
    }
    m.catat(model_s.count as u32 == drain_s);
    m.catat(model_r.count as u32 == drain_r);
    model_s.kuras();
    model_r.kuras();
    m.catat(qcount(s3i, s3g) == 0 && qcount(r3i, r3g) == 0);
    m.catat(lepas(r3i, r3g) == 1);
    m.catat(lepas(s3i, s3g) == 1);

    // --- G7: diferensial gerbang ukuran (10k kasus, ujung tanpa peer) --------
    let e = buat();
    let (ei, eg) = pecah(e);
    let mut probe2: u64 = 0xABCD_1234;
    let mut n2 = 0u32;
    while n2 < 10_000 {
        probe2 = probe2
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let len = match n2 % 5 {
            0 => 0,
            1 => 1,
            2 => MAX_PESAN_UKURAN,
            3 => MAX_PESAN_UKURAN + 1,
            _ => (probe2 & 0x1FFF) as u32,
        };
        let got = kirim(ei, eg, buf.as_ptr(), len);
        let want = if len > MAX_PESAN_UKURAN { -E_MSGSIZE } else { 0 };
        m.dif(got == want);
        if got == 0 {
            m.catat(terima(ei, eg, buf.as_mut_ptr(), 4096) == len as i32);
        }
        n2 += 1;
    }
    m.catat(lepas(ei, eg) == 1);
    m.catat(jumlah() == jumlah_awal);

    let status = if m.checks == m.passed && m.dif_beda == 0 { 0 } else { -1 };

    KlatosUjungReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_buat: klatos_ujung_counter(Counter::ProdBuat as u32),
        prod_aktif: klatos_ujung_counter(Counter::ProdAktif as u32),
        prod_taut: klatos_ujung_counter(Counter::ProdTaut as u32),
        prod_tutup: klatos_ujung_counter(Counter::ProdTutup as u32),
        prod_lepas: klatos_ujung_counter(Counter::ProdLepas as u32),
        prod_kirim_ok: klatos_ujung_counter(Counter::ProdKirimOk as u32),
        prod_kirim_tolak: klatos_ujung_counter(Counter::ProdKirimTolak as u32),
        prod_terima_ok: klatos_ujung_counter(Counter::ProdTerimaOk as u32),
        prod_terima_tolak: klatos_ujung_counter(Counter::ProdTerimaTolak as u32),
        prod_tolak_stale: klatos_ujung_counter(Counter::ProdTolakStale as u32),
        prod_tolak_besar: klatos_ujung_counter(Counter::ProdTolakBesar as u32),
        prod_tolak_penuh: klatos_ujung_counter(Counter::ProdTolakPenuh as u32),
        prod_tolak_kosong: klatos_ujung_counter(Counter::ProdTolakKosong as u32),
        status,
    }
}

/// Matriks otoritas siklus ujung + antrean.
#[no_mangle]
pub extern "C" fn klatos_ujung_selftest() -> KlatosUjungReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_554A_4E47);
        assert!(MAX_UJUNG == 1024);
        assert!(MAX_PESAN == 8192);
        assert!(POOL_SLOT == 2048);
        assert!(SLOT_BYTES == 4096);
        assert!(MAX_PESAN_UKURAN == 4096);
        assert!(MAX_ANTREAN_BYTE == 64 * 1024);
        assert!(MAX_ANTREAN_PESAN == 1024);
        assert!(E_BADF == 9 && E_AGAIN == 11 && E_NOMEM == 12 && E_INVAL == 22 && E_PIPE == 32 && E_MSGSIZE == 55);
        assert!(ST_BIAR == 0 && ST_AKTIF == 1 && ST_TUTUP == 2 && ST_LEPAS == 3);
        assert!(TERIMA_EOF == -1);
        assert!(Counter::ProdBuat as usize == 0);
        assert!(Counter::ProdTolakKosong as usize == 12);
        assert!(N_COUNTER == 13);
        assert!(core::mem::size_of::<KlatosUjungReport>() == 88);
        assert!(core::mem::align_of::<KlatosUjungReport>() == 8);
    };
}
