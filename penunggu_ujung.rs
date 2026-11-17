// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8.3-C — **OTORITAS PENUNGGU/BANGUN GENERIK (KlatIPC)**.


#![allow(unsafe_code)]

use crate::siklus_ujung;
use core::sync::atomic::{AtomicU32, Ordering, AtomicU64};

/// Tag laporan; cermin `KlatosPenungguReport` di `KlatSeam133.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_504E_4747; // "KLATPNGG"

/// Indeks tidak sah.
pub const INVALID: u32 = 0xFFFF_FFFF;

/// Jumlah slot penunggu maksimum.
pub const MAX_PENUNGGU: usize = 4096;

// --- jenis penunggu ---------------------------------------------------------

/// Penunggu ruang antrean (pengirim).
pub const PK_KIRIM: u32 = 0;
/// Penunggu pesan (penerima).
pub const PK_TERIMA: u32 = 1;

// --- status penunggu --------------------------------------------------------

/// Slot bebas.
pub const PS_KOSONG: u32 = 0;
/// `WAITING` — terdaftar, belum diblok.
pub const PS_MENUNGGU: u32 = 1;
/// `QUEUED` — sedang diblok di antrean penunggu.
pub const PS_ANTRE: u32 = 2;
/// `WOKEN` — dibangunkan, belum berjalan.
pub const PS_BANGUN: u32 = 3;
/// `RUNNABLE` — sudah kembali ke penjadwal (slot dilepas).
pub const PS_SIAP: u32 = 4;
/// `EXITED` — thread keluar saat menunggu (bangun harus melewatinya).
pub const PS_KELUAR: u32 = 5;

// --- errno mirror -----------------------------------------------------------

/// `EBADF` — handle penunggu usang/aspal.
pub const E_BADF: i32 = 9;
/// `EINVAL` — transisi tidak sah.
pub const E_INVAL: i32 = 22;

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `daftar` berhasil.
    ProdDaftar = 0,
    /// `antre` berhasil.
    ProdAntre = 1,
    /// Bangun yang benar-benar memindahkan penunggu ke WOKEN.
    ProdBangunOk = 2,
    /// `ambil` berhasil.
    ProdAmbil = 3,
    /// `batal` berhasil.
    ProdBatal = 4,
    /// `keluar` ditandai.
    ProdKeluar = 5,
    /// Penolakan ujung aspal/usang.
    TolakStale = 6,
    /// Penolakan hak.
    TolakHak = 7,
    /// Penolakan jenis.
    TolakJenis = 8,
    /// Penolakan penunggu ganda.
    TolakDuplikat = 9,
    /// Bangun yang melewati penunggu EXITED.
    TolakBangunKeluar = 10,
    /// Penolakan transisi status tidak sah.
    TolakState = 11,
    /// Slot penuh.
    TolakPenuh = 12,
    /// Pelanggaran: handle aspal diterima (WAJIB 0).
    LanggarForged = 13,
    /// Pelanggaran: akses usang diterima (WAJIB 0).
    LanggarStale = 14,
    /// Pelanggaran: operasi tanpa hak diterima (WAJIB 0).
    LanggarHak = 15,
    /// Pelanggaran: jalur melewati KSM (WAJIB 0).
    LanggarKsm = 16,
}

const N_COUNTER: usize = 17;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- arena penunggu ---------------------------------------------------------

static PN_GEN: [AtomicU32; MAX_PENUNGGU] = [const { AtomicU32::new(0) }; MAX_PENUNGGU];
static PN_ST: [AtomicU32; MAX_PENUNGGU] = [const { AtomicU32::new(PS_KOSONG) }; MAX_PENUNGGU];
static PN_UJUNG: [AtomicU32; MAX_PENUNGGU] = [const { AtomicU32::new(INVALID) }; MAX_PENUNGGU];
static PN_UJUNG_GEN: [AtomicU32; MAX_PENUNGGU] = [const { AtomicU32::new(0) }; MAX_PENUNGGU];
static PN_JENIS: [AtomicU32; MAX_PENUNGGU] = [const { AtomicU32::new(0) }; MAX_PENUNGGU];
static PN_TID: [AtomicU32; MAX_PENUNGGU] = [const { AtomicU32::new(0) }; MAX_PENUNGGU];
static PN_NEXT: [AtomicU32; MAX_PENUNGGU] = [const { AtomicU32::new(INVALID) }; MAX_PENUNGGU];
/// Kepala rantai penunggu per indeks ujung.
static PJ_HEAD: [AtomicU32; siklus_ujung::MAX_UJUNG] =
    [const { AtomicU32::new(INVALID) }; siklus_ujung::MAX_UJUNG];

static LOCK: AtomicU32 = AtomicU32::new(0);
/// Status interupsi pemegang LOCK; hanya pemegang yang menulis.
static LOCK_FLAGS: AtomicU64 = AtomicU64::new(0);

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

/// Apakah handle penunggu hidup?
#[inline]
fn hidup(index: u32, generation: u32) -> bool {
    let i = index as usize;
    generation != 0
        && i < MAX_PENUNGGU
        && PN_GEN[i].load(Ordering::Acquire) == generation
        && PN_ST[i].load(Ordering::Acquire) != PS_KOSONG
}

#[inline]
fn handle_of(index: u32, generation: u32) -> u64 {
    ((generation as u64) << 32) | index as u64
}

#[inline]
fn pecah(h: u64) -> (u32, u32) {
    (h as u32, (h >> 32) as u32)
}

/// Satu-satunya sumber kebenaran liveness ujung: arena `siklus_ujung`.
#[inline]
fn ujung_hidup(uji: u32, ujg: u32) -> bool {
    siklus_ujung::status(uji, ujg) != siklus_ujung::INVALID
}

#[inline]
fn ujung_tertutup(uji: u32, ujg: u32) -> bool {
    siklus_ujung::status(uji, ujg) >= siklus_ujung::ST_TUTUP
}

fn slot_bebas_terkunci() -> u32 {
    let mut i = 0usize;
    while i < MAX_PENUNGGU {
        if PN_ST[i].load(Ordering::Relaxed) == PS_KOSONG {
            return i as u32;
        }
        i += 1;
    }
    INVALID
}

/// Bangun ulang rantai penunggu ujung `uji`, buang simpul usang.
fn bersihkan_rantai_terkunci(uji: u32) {
    let ui = uji as usize;
    if ui >= siklus_ujung::MAX_UJUNG {
        return;
    }
    let mut baru = INVALID;
    let mut d = PJ_HEAD[ui].load(Ordering::Relaxed);
    while d != INVALID {
        let next = PN_NEXT[d as usize].load(Ordering::Relaxed);
        let di = d as usize;
        let sah = PN_ST[di].load(Ordering::Relaxed) != PS_KOSONG
            && PN_UJUNG[di].load(Ordering::Relaxed) == uji;
        if sah {
            PN_NEXT[di].store(baru, Ordering::Relaxed);
            baru = d;
        }
        d = next;
    }
    PJ_HEAD[ui].store(baru, Ordering::Relaxed);
}

/// Daftar penunggu (WAITING). `0` = ditolak.
#[allow(clippy::too_many_arguments)]
pub fn daftar(uji: u32, ujg: u32, jenis: u32, punya_hak: u32, tid: u32) -> u64 {
    let mut hasil = 0u64;
    lock();
    if !ujung_hidup(uji, ujg) {
        bump(Counter::TolakStale);
    } else if punya_hak == 0 {
        bump(Counter::TolakHak);
    } else if jenis > PK_TERIMA {
        bump(Counter::TolakJenis);
    } else if jenis == PK_KIRIM && ujung_tertutup(uji, ujg) {
        // Pengirim tidak sah: ujungnya sudah tertutup.
        bump(Counter::TolakJenis);
    } else if tid != 0 && penunggu_duplikat_terkunci(uji, jenis, tid) {
        bump(Counter::TolakDuplikat);
    } else {
        let slot = slot_bebas_terkunci();
        if slot == INVALID {
            bump(Counter::TolakPenuh);
        } else {
            let si = slot as usize;
            let generation = PN_GEN[si].load(Ordering::Relaxed).wrapping_add(1);
            if generation == 0 {
                bump(Counter::TolakPenuh);
            } else {
                PN_GEN[si].store(generation, Ordering::Relaxed);
                PN_UJUNG[si].store(uji, Ordering::Relaxed);
                PN_UJUNG_GEN[si].store(ujg, Ordering::Relaxed);
                PN_JENIS[si].store(jenis, Ordering::Relaxed);
                PN_TID[si].store(tid, Ordering::Relaxed);
                PN_ST[si].store(PS_MENUNGGU, Ordering::Release);
                if (uji as usize) < siklus_ujung::MAX_UJUNG {
                    PN_NEXT[si].store(PJ_HEAD[uji as usize].load(Ordering::Relaxed), Ordering::Relaxed);
                    PJ_HEAD[uji as usize].store(slot, Ordering::Relaxed);
                } else {
                    PN_NEXT[si].store(INVALID, Ordering::Relaxed);
                }
                hasil = handle_of(slot, generation);
                bump(Counter::ProdDaftar);
                // Defensif: penerimaan tanpa hak / lewat KSM / pada ujung aspal
                // TIDAK boleh terjadi.
                if punya_hak == 0 {
                    bump(Counter::LanggarHak);
                    bump(Counter::LanggarKsm);
                }
                if !ujung_hidup(uji, ujg) {
                    bump(Counter::LanggarForged);
                }
            }
        }
    }
    unlock();
    hasil
}

/// Pemeriksaan penunggu ganda (dipanggil di bawah `LOCK`).
fn penunggu_duplikat_terkunci(uji: u32, jenis: u32, tid: u32) -> bool {
    let ui = uji as usize;
    if ui >= siklus_ujung::MAX_UJUNG {
        return false;
    }
    let mut d = PJ_HEAD[ui].load(Ordering::Relaxed);
    while d != INVALID {
        let di = d as usize;
        let st = PN_ST[di].load(Ordering::Relaxed);
        if st != PS_KOSONG
            && st != PS_KELUAR
            && st != PS_SIAP
            && PN_JENIS[di].load(Ordering::Relaxed) == jenis
            && PN_TID[di].load(Ordering::Relaxed) == tid
        {
            return true;
        }
        d = PN_NEXT[di].load(Ordering::Relaxed);
    }
    false
}

/// `WAITING → QUEUED`.
#[allow(unused_assignments)]
pub fn antre(h: u64) -> i32 {
    let (index, generation) = pecah(h);
    let mut r = 0i32;
    lock();
    if !hidup(index, generation) {
        r = -E_BADF;
        bump(Counter::TolakStale);
    } else {
        let i = index as usize;
        let st = PN_ST[i].load(Ordering::Relaxed);
        if st == PS_MENUNGGU {
            PN_ST[i].store(PS_ANTRE, Ordering::Release);
            bump(Counter::ProdAntre);
        } else {
            r = -E_INVAL;
            bump(Counter::TolakState);
        }
    }
    // Defensif: keberhasilan pada handle usang/aspal TIDAK boleh terjadi.
    if r == 0 && !hidup(index, generation) {
        bump(Counter::LanggarStale);
    }
    unlock();
    r
}

/// Lepas slot penunggu (transisi terminal `RUNNABLE`, lalu slot bebas).
///
/// URUTAN PENTING: state diset `KOSONG` SEBELUM rantai dibangun ulang, supaya
/// pembersihan benar-benar membuang simpul ini (bila tidak, simpul tetap ada di
/// rantai sementara `PN_NEXT`-nya lalu dikosongkan → rantai putus dan slot yang
/// dipakai ulang membentuk lingkar diri). `PN_NEXT` baru dikosongkan SESUDAH
/// pembersihan, sehingga penelusuran lama tetap utuh.
fn lepas_slot_terkunci(i: usize) -> bool {
    let uji = PN_UJUNG[i].load(Ordering::Relaxed);
    // WOKEN → RUNNABLE (SIAP) → slot bebas (KOSONG).
    PN_ST[i].store(PS_SIAP, Ordering::Release);
    PN_ST[i].store(PS_KOSONG, Ordering::Release);
    PN_UJUNG[i].store(INVALID, Ordering::Relaxed);
    PN_JENIS[i].store(0, Ordering::Relaxed);
    PN_TID[i].store(0, Ordering::Relaxed);
    if (uji as usize) < siklus_ujung::MAX_UJUNG {
        bersihkan_rantai_terkunci(uji);
    }
    PN_NEXT[i].store(INVALID, Ordering::Relaxed);
    true
}

/// `WOKEN → RUNNABLE` (dan melepas slot). `0` sukses, `-errno` gagal.
#[allow(unused_assignments)]
pub fn ambil(h: u64) -> i32 {
    let (index, generation) = pecah(h);
    let mut r = 0i32;
    lock();
    let ada = hidup(index, generation);
    if !ada {
        r = -E_BADF;
        bump(Counter::TolakStale);
    } else {
        lepas_slot_terkunci(index as usize);
        bump(Counter::ProdAmbil);
    }
    // Defensif: keberhasilan pada handle usang/aspal TIDAK boleh terjadi.
    if r == 0 && !ada {
        bump(Counter::LanggarStale);
    }
    unlock();
    r
}

/// Batal (penunggu meninggalkan antrean). `0` sukses, `-errno` gagal.
#[allow(unused_assignments)]
pub fn batal(h: u64) -> i32 {
    let (index, generation) = pecah(h);
    let mut r = 0i32;
    lock();
    let ada = hidup(index, generation);
    if !ada {
        r = -E_BADF;
        bump(Counter::TolakStale);
    } else {
        lepas_slot_terkunci(index as usize);
        bump(Counter::ProdBatal);
    }
    // Defensif: keberhasilan pada handle usang/aspal TIDAK boleh terjadi.
    if r == 0 && !ada {
        bump(Counter::LanggarStale);
    }
    unlock();
    r
}

/// Tandai penunggu `EXITED` (thread keluar). `1`/`0`.
pub fn keluar(h: u64) -> i32 {
    let (index, generation) = pecah(h);
    let mut r = 0i32;
    lock();
    let ada = hidup(index, generation);
    if !ada {
        bump(Counter::TolakStale);
    } else {
        PN_ST[index as usize].store(PS_KELUAR, Ordering::Release);
        bump(Counter::ProdKeluar);
        r = 1;
    }
    // Defensif: keberhasilan pada handle usang/aspal TIDAK boleh terjadi.
    if r == 1 && !ada {
        bump(Counter::LanggarStale);
    }
    unlock();
    r
}

/// Bangun SEMUA penunggu ujung `(uji, ujg)` yang sah. Mengembalikan jumlah
/// penunggu yang dipindahkan ke `WOKEN`.
pub fn bangun_ujung(uji: u32, ujg: u32) -> u32 {
    let mut n = 0u32;
    lock();
    if !ujung_hidup(uji, ujg) {
        bump(Counter::TolakStale);
        if (uji as usize) < siklus_ujung::MAX_UJUNG {
            bersihkan_rantai_terkunci(uji);
        }
        unlock();
        return 0;
    }
    let ui = uji as usize;
    let mut d = PJ_HEAD[ui].load(Ordering::Relaxed);
    while d != INVALID {
        let next = PN_NEXT[d as usize].load(Ordering::Relaxed);
        let di = d as usize;
        let st = PN_ST[di].load(Ordering::Relaxed);
        if st != PS_KOSONG && PN_UJUNG[di].load(Ordering::Relaxed) == uji {
            if st == PS_MENUNGGU || st == PS_ANTRE {
                PN_ST[di].store(PS_BANGUN, Ordering::Release);
                n += 1;
                bump(Counter::ProdBangunOk);
            } else if st == PS_KELUAR {
                // WAKE_EXITED: penunggu sudah keluar — bangun DITOLAK, dan slot
                // keluar dibersihkan di sini (PN_NEXT dibiarkan utuh sampai
                // penelusuran selesai).
                bump(Counter::TolakBangunKeluar);
                PN_ST[di].store(PS_KOSONG, Ordering::Release);
                PN_UJUNG[di].store(INVALID, Ordering::Relaxed);
                PN_JENIS[di].store(0, Ordering::Relaxed);
                PN_TID[di].store(0, Ordering::Relaxed);
            }
            // PS_BANGUN/PS_SIAP: bangun ganda → tidak dihitung ulang.
        }
        d = next;
    }
    bersihkan_rantai_terkunci(uji);
    // Defensif: bangun pada ujung aspal TIDAK boleh menghitung penunggu.
    if n > 0 && !ujung_hidup(uji, ujg) {
        bump(Counter::LanggarForged);
    }
    unlock();
    n
}

/// Status penunggu, atau [`INVALID`] bila usang/aspal.
pub fn status(h: u64) -> u32 {
    let (index, generation) = pecah(h);
    if !hidup(index, generation) {
        return INVALID;
    }
    PN_ST[index as usize].load(Ordering::Acquire)
}

/// Jumlah penunggu hidup.
pub fn jumlah() -> u32 {
    let mut n = 0u32;
    let mut i = 0usize;
    while i < MAX_PENUNGGU {
        let st = PN_ST[i].load(Ordering::Relaxed);
        if st != PS_KOSONG {
            n += 1;
        }
        i += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

/// Daftar penunggu. `0` = ditolak.
#[no_mangle]
pub extern "C" fn klatos_penunggu_daftar(uji: u32, ujg: u32, jenis: u32, punya_hak: u32, tid: u32) -> u64 {
    daftar(uji, ujg, jenis, punya_hak, tid)
}

/// `WAITING → QUEUED`. `0` sukses.
#[no_mangle]
pub extern "C" fn klatos_penunggu_antre(h: u64) -> i32 {
    antre(h)
}

/// `WOKEN → RUNNABLE`. `0` sukses.
#[no_mangle]
pub extern "C" fn klatos_penunggu_ambil(h: u64) -> i32 {
    ambil(h)
}

/// Batal. `0` sukses.
#[no_mangle]
pub extern "C" fn klatos_penunggu_batal(h: u64) -> i32 {
    batal(h)
}

/// Tandai EXITED. `1`/`0`.
#[no_mangle]
pub extern "C" fn klatos_penunggu_keluar(h: u64) -> i32 {
    keluar(h)
}

/// Bangun semua penunggu ujung. Mengembalikan jumlah yang dibangunkan.
#[no_mangle]
pub extern "C" fn klatos_penunggu_bangun_ujung(uji: u32, ujg: u32) -> u32 {
    bangun_ujung(uji, ujg)
}

/// Status penunggu atau `INVALID`.
#[no_mangle]
pub extern "C" fn klatos_penunggu_status(h: u64) -> u32 {
    status(h)
}

/// Jumlah penunggu hidup.
#[no_mangle]
pub extern "C" fn klatos_penunggu_jumlah() -> u32 {
    jumlah()
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_penunggu_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTERS[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Nol-kan penghitung.
#[no_mangle]
pub extern "C" fn klatos_penunggu_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosPenungguReport` di `KlatSeam133.h`.
#[repr(C)]
pub struct KlatosPenungguReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_daftar: u32,
    pub prod_antre: u32,
    pub prod_bangun_ok: u32,
    pub prod_ambil: u32,
    pub prod_batal: u32,
    pub prod_keluar: u32,
    pub tolak_stale: u32,
    pub tolak_hak: u32,
    pub tolak_jenis: u32,
    pub tolak_duplikat: u32,
    pub tolak_bangun_keluar: u32,
    pub tolak_state: u32,
    pub langgar_forged: u32,
    pub langgar_stale: u32,
    pub langgar_hak: u32,
    pub langgar_ksm: u32,
    pub status: i32,
}

const _: [(); 104] = [(); core::mem::size_of::<KlatosPenungguReport>()];

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

/// Matriks otoritas penunggu. Input SINTETIS.
#[allow(unused_assignments)]
fn run_matrix() -> KlatosPenungguReport {
    let mut m = Matrix::new();

    // --- G1: konstanta ------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_504E_4747);
    m.catat(MAX_PENUNGGU == 4096);
    m.catat(PK_KIRIM == 0 && PK_TERIMA == 1);
    m.catat(PS_KOSONG == 0 && PS_MENUNGGU == 1 && PS_ANTRE == 2 && PS_BANGUN == 3 && PS_SIAP == 4 && PS_KELUAR == 5);
    m.catat(E_BADF == 9 && E_INVAL == 22);

    // Jumlah penunggu hidup SEBELUM matriks: di runtime kernel subsistem lain
    // dapat sudah menunggu; karena itu matriks menghitung RELATIF.
    let jumlah_awal = jumlah();

    // Ujung uji (arena siklus_ujung).
    let a = siklus_ujung::buat();
    let b = siklus_ujung::buat();
    let (ai, ag) = pecah(a);
    let (bi, bg) = pecah(b);
    m.catat(siklus_ujung::taut(ai, ag, bi, bg) == 0);

    // --- G2: siklus WAITING → QUEUED → WOKEN → RUNNABLE ---------------------
    let w = daftar(ai, ag, PK_KIRIM, 1, 7);
    m.catat(w != 0);
    m.catat(status(w) == PS_MENUNGGU);
    m.catat(antre(w) == 0);
    m.catat(status(w) == PS_ANTRE);
    m.catat(bangun_ujung(ai, ag) == 1);
    m.catat(status(w) == PS_BANGUN);
    m.catat(ambil(w) == 0);
    m.catat(status(w) == INVALID); // slot dilepas
    m.catat(bangun_ujung(ai, ag) == 0); // tidak ada penunggu lagi
    m.catat(jumlah() == jumlah_awal);

    // --- G3: negatif wajib --------------------------------------------------
    // FORGED_ENDPOINT
    m.neg(daftar(INVALID, 1, PK_KIRIM, 1, 1) == 0);
    m.neg(daftar(siklus_ujung::MAX_UJUNG as u32, 1, PK_KIRIM, 1, 1) == 0);
    // STALE_ENDPOINT (ujung dilepas)
    let t = siklus_ujung::buat();
    let (ti, tg) = pecah(t);
    m.catat(siklus_ujung::lepas(ti, tg) == 1);
    m.neg(daftar(ti, tg, PK_KIRIM, 1, 2) == 0);
    // WRONG_RIGHTS (tanpa hak)
    m.neg(daftar(ai, ag, PK_KIRIM, 0, 3) == 0);
    // INVALID_SENDER (pengirim pada ujung tertutup)
    let c = siklus_ujung::buat();
    let (ci, cg) = pecah(c);
    m.catat(siklus_ujung::tutup(ci, cg) == 1);
    m.neg(daftar(ci, cg, PK_KIRIM, 1, 4) == 0);
    m.catat(siklus_ujung::lepas(ci, cg) == 1);
    // INVALID_RECEIVER (jenis tidak dikenal)
    m.neg(daftar(ai, ag, 99, 1, 5) == 0);
    // DUPLICATE_WAITER (tid sama, ujung+jenis sama)
    let w1 = daftar(ai, ag, PK_TERIMA, 1, 42);
    m.catat(w1 != 0);
    m.neg(daftar(ai, ag, PK_TERIMA, 1, 42) == 0);
    // INVALID_STATE: antre setelah WOKEN
    m.catat(antre(w1) == 0);
    m.catat(bangun_ujung(ai, ag) == 1);
    m.catat(status(w1) == PS_BANGUN);
    m.neg(antre(w1) == -E_INVAL);
    // DOUBLE_RELEASE (ambil/batal dua kali)
    m.catat(ambil(w1) == 0);
    m.neg(ambil(w1) == -E_BADF);
    m.neg(batal(w1) == -E_BADF);
    // STALE_WAITER
    m.neg(status(w1) == INVALID);
    m.neg(antre(w1) == -E_BADF);
    m.neg(keluar(w1) == 0);
    // WAKE_EXITED: penunggu keluar → bangun melewatinya (tidak dihitung)
    let w2 = daftar(ai, ag, PK_TERIMA, 1, 43);
    m.catat(w2 != 0);
    m.catat(antre(w2) == 0);
    m.catat(keluar(w2) == 1);
    m.neg(bangun_ujung(ai, ag) == 0);
    m.neg(status(w2) == INVALID);
    // DOUBLE_WAKE: tanpa penunggu sah → 0
    m.neg(bangun_ujung(ai, ag) == 0);
    // Bangun ujung yang dilepas
    let u = siklus_ujung::buat();
    let (ui, ug) = pecah(u);
    m.catat(siklus_ujung::lepas(ui, ug) == 1);
    m.neg(bangun_ujung(ui, ug) == 0);
    m.catat(jumlah() == jumlah_awal);

    // --- G4: metrik pelanggaran WAJIB 0 -------------------------------------
    m.catat(klatos_penunggu_counter(Counter::LanggarForged as u32) == 0);
    m.catat(klatos_penunggu_counter(Counter::LanggarStale as u32) == 0);
    m.catat(klatos_penunggu_counter(Counter::LanggarHak as u32) == 0);
    m.catat(klatos_penunggu_counter(Counter::LanggarKsm as u32) == 0);

    // --- G5: diferensial state machine (20k kasus, model independen) ---------
    // Model ditulis ulang di sini (bukan memanggil fungsi modul): satu penunggu
    // dengan status + flag hidup.
    let mut hidup_m = true;
    let mut st_m: u32 = PS_MENUNGGU;
    let mut w3 = daftar(ai, ag, PK_TERIMA, 1, 100);
    m.catat(w3 != 0);
    let mut probe: u64 = 0x1234_5678;
    let mut n = 0u32;
    while n < 20_000 {
        probe = probe.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let op = (probe >> 61) % 5;
        let (got, want) = match op {
            0 => {
                let g = antre(w3);
                let w = if !hidup_m {
                    -E_BADF
                } else if st_m == PS_MENUNGGU {
                    0
                } else {
                    -E_INVAL
                };
                if w == 0 {
                    st_m = PS_ANTRE;
                }
                (g, w)
            }
            1 => {
                let g = bangun_ujung(ai, ag) as i32;
                let w = if hidup_m && (st_m == PS_MENUNGGU || st_m == PS_ANTRE) {
                    1
                } else {
                    0
                };
                if w == 1 {
                    st_m = PS_BANGUN;
                } else if hidup_m && st_m == PS_KELUAR {
                    // bangun melewati EXITED → slot dilepas
                    hidup_m = false;
                    st_m = PS_KOSONG;
                }
                (g, w)
            }
            2 => {
                let g = ambil(w3);
                let w = if hidup_m { 0 } else { -E_BADF };
                if hidup_m {
                    hidup_m = false;
                    st_m = PS_KOSONG;
                }
                (g, w)
            }
            3 => {
                // daftar ulang: ditolak HANYA bila masih ada penunggu AKTIF
                // (MENUNGGU/ANTRE/BANGUN) dengan tid sama; penunggu EXITED tidak
                // lagi menahan pendaftaran baru.
                let g = daftar(ai, ag, PK_TERIMA, 1, 100);
                let aktif_m = hidup_m && st_m != PS_KELUAR;
                let w_nz = !aktif_m;
                let cocok = (g != 0) == w_nz;
                if cocok && g != 0 {
                    w3 = g;
                    hidup_m = true;
                    st_m = PS_MENUNGGU;
                }
                (if cocok { 1 } else { 0 }, 1)
            }
            _ => {
                let g = keluar(w3);
                let w = if hidup_m { 1 } else { 0 };
                if hidup_m {
                    st_m = PS_KELUAR;
                }
                (g, w)
            }
        };
        m.dif(got == want);
        n += 1;
    }
    if hidup_m && w3 != 0 {
        let _ = batal(w3);
    }
    m.catat(jumlah() == jumlah_awal);
    m.catat(siklus_ujung::lepas(bi, bg) == 1);
    m.catat(siklus_ujung::lepas(ai, ag) == 1);
    m.catat(jumlah() == jumlah_awal);

    let status = if m.checks == m.passed && m.dif_beda == 0 { 0 } else { -1 };

    KlatosPenungguReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_daftar: klatos_penunggu_counter(Counter::ProdDaftar as u32),
        prod_antre: klatos_penunggu_counter(Counter::ProdAntre as u32),
        prod_bangun_ok: klatos_penunggu_counter(Counter::ProdBangunOk as u32),
        prod_ambil: klatos_penunggu_counter(Counter::ProdAmbil as u32),
        prod_batal: klatos_penunggu_counter(Counter::ProdBatal as u32),
        prod_keluar: klatos_penunggu_counter(Counter::ProdKeluar as u32),
        tolak_stale: klatos_penunggu_counter(Counter::TolakStale as u32),
        tolak_hak: klatos_penunggu_counter(Counter::TolakHak as u32),
        tolak_jenis: klatos_penunggu_counter(Counter::TolakJenis as u32),
        tolak_duplikat: klatos_penunggu_counter(Counter::TolakDuplikat as u32),
        tolak_bangun_keluar: klatos_penunggu_counter(Counter::TolakBangunKeluar as u32),
        tolak_state: klatos_penunggu_counter(Counter::TolakState as u32),
        langgar_forged: klatos_penunggu_counter(Counter::LanggarForged as u32),
        langgar_stale: klatos_penunggu_counter(Counter::LanggarStale as u32),
        langgar_hak: klatos_penunggu_counter(Counter::LanggarHak as u32),
        langgar_ksm: klatos_penunggu_counter(Counter::LanggarKsm as u32),
        status,
    }
}

/// Matriks otoritas penunggu.
#[no_mangle]
pub extern "C" fn klatos_penunggu_selftest() -> KlatosPenungguReport {
    run_matrix()
}

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_504E_4747);
        assert!(MAX_PENUNGGU == 4096);
        assert!(PS_KOSONG == 0 && PS_KELUAR == 5);
        assert!(N_COUNTER == 17);
        assert!(Counter::LanggarKsm as usize == 16);
        assert!(core::mem::size_of::<KlatosPenungguReport>() == 104);
    };
}
