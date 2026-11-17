// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas **buku besar registry proses** Jowo (P1.46).


use core::sync::atomic::{AtomicBool, AtomicU32, Ordering, AtomicU64};

/// Kapasitas tabel registry proses (berbatas).
pub const MAX_PROSES: usize = 4096;

/// Tag laporan; cermin `KlatosBukuBesarProsesReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5052_5347; // "KLATPRSG"

/// Nilai yang dikembalikan [`klatos_proses_ppid`] bila pid tidak terdaftar.
pub const PPID_TAK_DITEMUKAN: u32 = 0xFFFF_FFFF;

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_proses_daftar`] dari produksi.
    ProdDaftar = 0,
    /// Jumlah total pemanggilan [`klatos_proses_lepas`] dari produksi.
    ProdLepas = 1,
    /// Jumlah pendaftaran yang DITOLAK (duplikat atau tabel penuh).
    ProdTolak = 2,
    /// Jumlah pelepasan untuk pid yang TIDAK terdaftar.
    ProdTakDitemukan = 3,
    /// Jumlah pelepasan yang meninggalkan anak terdaftar (peristiwa yatim).
    ProdAnakYatim = 4,
    /// Jumlah proses KERNEL yang pernah terdaftar.
    ProdKernel = 5,
    /// Jumlah proses PENGGUNA yang pernah terdaftar.
    ProdPengguna = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Buku besar utama
// ---------------------------------------------------------------------------

static KUNCI: AtomicBool = AtomicBool::new(false);
/// Status interupsi pemegang KUNCI. Hanya pemegang yang menulis dan
/// membacanya sebelum melepas, jadi tidak perlu atomik kuat.
static KUNCI_FLAGS: AtomicU64 = AtomicU64::new(0);

/// 0 = slot bebas, 1 = terpakai (proses pengguna), 2 = terpakai (proses kernel).
static SLOT_BENDERA: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];

/// pid yang menempati slot.
static SLOT_PID: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];

/// ppid yang menempati slot.
static SLOT_PPID: [AtomicU32; MAX_PROSES] = [const { AtomicU32::new(0) }; MAX_PROSES];

/// Proses hidup sekarang (dijaga inkremental di bawah kunci).
static HIDUP: AtomicU32 = AtomicU32::new(0);

/// Puncak proses hidup yang pernah teramati.
static PUNCAK: AtomicU32 = AtomicU32::new(0);

fn lock() {
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

fn unlock() {
    // Baca status milik sendiri SEBELUM melepas kunci, supaya pengambil
    // berikutnya tidak bisa menimpa nilainya.
    let flags = KUNCI_FLAGS.load(Ordering::Relaxed);
    KUNCI.store(false, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Indeks slot yang menampung `pid`, atau `None`.
fn find(pid: u32) -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_PROSES {
        if SLOT_BENDERA[i].load(Ordering::Relaxed) != 0
            && SLOT_PID[i].load(Ordering::Relaxed) == pid
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Indeks slot bebas pertama, atau `None`.
fn free_slot() -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_PROSES {
        if SLOT_BENDERA[i].load(Ordering::Relaxed) == 0 {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Mendaftarkan proses `(pid, ppid, kernel?)`. Mengembalikan 1 bila berhasil,
/// 0 bila duplikat atau tabel penuh.
fn daftar_impl(pid: u32, ppid: u32, is_kernel: u32) -> u32 {
    bump(Counter::ProdDaftar);
    let bendera = if is_kernel != 0 { 2u32 } else { 1u32 };

    lock();
    let duplicate = find(pid).is_some();
    let slot = if duplicate { None } else { free_slot() };
    let ok = slot.is_some();
    if let Some(i) = slot {
        SLOT_PID[i].store(pid, Ordering::Relaxed);
        SLOT_PPID[i].store(ppid, Ordering::Relaxed);
        SLOT_BENDERA[i].store(bendera, Ordering::Release);
    }
    let hidup = if ok {
        let h = HIDUP.load(Ordering::Relaxed) + 1;
        HIDUP.store(h, Ordering::Relaxed);
        if h > PUNCAK.load(Ordering::Relaxed) {
            PUNCAK.store(h, Ordering::Relaxed);
        }
        h
    } else {
        HIDUP.load(Ordering::Relaxed)
    };
    unlock();

    if ok {
        if is_kernel != 0 {
            bump(Counter::ProdKernel);
        } else {
            bump(Counter::ProdPengguna);
        }
    } else {
        bump(Counter::ProdTolak);
    }
    hidup
}

/// Melepas proses `pid`. Mengembalikan 1 bila dihapus, 0 bila tidak ditemukan.
/// Menghitung peristiwa yatim bila masih ada anak terdaftar.
fn lepas_impl(pid: u32) -> u32 {
    bump(Counter::ProdLepas);

    lock();
    let slot = find(pid);
    let mut yatim = 0u32;
    if let Some(i) = slot {
        SLOT_BENDERA[i].store(0, Ordering::Release);
        SLOT_PID[i].store(0, Ordering::Relaxed);
        SLOT_PPID[i].store(0, Ordering::Relaxed);
        let h = HIDUP.load(Ordering::Relaxed);
        if h > 0 {
            HIDUP.store(h - 1, Ordering::Relaxed);
        }
        // Hitung anak langsung yang masih terdaftar.
        let mut j = 0usize;
        while j < MAX_PROSES {
            if SLOT_BENDERA[j].load(Ordering::Relaxed) != 0
                && SLOT_PPID[j].load(Ordering::Relaxed) == pid
            {
                yatim += 1;
            }
            j += 1;
        }
    }
    unlock();

    if slot.is_none() {
        bump(Counter::ProdTakDitemukan);
        return 0;
    }
    if yatim > 0 {
        bump(Counter::ProdAnakYatim);
    }
    1
}

/// Reset seluruh buku besar ke 0. TIDAK dipanggil dari produksi.
fn reset_impl() {
    lock();
    for i in 0..MAX_PROSES {
        SLOT_BENDERA[i].store(0, Ordering::Release);
        SLOT_PID[i].store(0, Ordering::Relaxed);
        SLOT_PPID[i].store(0, Ordering::Relaxed);
    }
    HIDUP.store(0, Ordering::Release);
    PUNCAK.store(0, Ordering::Release);
    unlock();
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// Algoritma — oracle pembanding deterministik
// ---------------------------------------------------------------------------

/// Oracle: model independen registry sebagai dua array (pid, ppid) + penanda.
struct Model {
    pid: [u32; MAX_PROSES],
    ppid: [u32; MAX_PROSES],
    used: [bool; MAX_PROSES],
    len: usize,
    puncak: usize,
}

impl Model {
    const fn baru() -> Self {
        Model {
            pid: [0; MAX_PROSES],
            ppid: [0; MAX_PROSES],
            used: [false; MAX_PROSES],
            len: 0,
            puncak: 0,
        }
    }
    fn daftar(&mut self, pid: u32, ppid: u32) -> bool {
        let mut i = 0usize;
        while i < MAX_PROSES {
            if self.used[i] && self.pid[i] == pid {
                return false;
            }
            i += 1;
        }
        if self.len == MAX_PROSES {
            return false;
        }
        let mut j = 0usize;
        while j < MAX_PROSES {
            if !self.used[j] {
                self.used[j] = true;
                self.pid[j] = pid;
                self.ppid[j] = ppid;
                self.len += 1;
                if self.len > self.puncak {
                    self.puncak = self.len;
                }
                return true;
            }
            j += 1;
        }
        false
    }
    fn lepas(&mut self, pid: u32) -> bool {
        let mut i = 0usize;
        while i < MAX_PROSES {
            if self.used[i] && self.pid[i] == pid {
                self.used[i] = false;
                self.len -= 1;
                return true;
            }
            i += 1;
        }
        false
    }
    fn anak(&self, ppid: u32) -> u32 {
        let mut n = 0u32;
        let mut i = 0usize;
        while i < MAX_PROSES {
            if self.used[i] && self.ppid[i] == ppid {
                n += 1;
            }
            i += 1;
        }
        n
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mendaftarkan proses `pid` dengan induk `ppid`. `is_kernel` 0/1.
/// Mengembalikan jumlah proses hidup setelah operasi (0 berarti ditolak DAN
/// tabel kosong; pakai [`klatos_proses_cari`] untuk membedakan).
#[no_mangle]
pub extern "C" fn klatos_proses_daftar(pid: u32, ppid: u32, is_kernel: u32) -> u32 {
    daftar_impl(pid, ppid, is_kernel)
}

/// Melepas proses `pid`. Mengembalikan 1 bila dihapus, 0 bila tidak ada.
#[no_mangle]
pub extern "C" fn klatos_proses_lepas(pid: u32) -> u32 {
    lepas_impl(pid)
}

/// 1 bila `pid` terdaftar, 0 bila tidak.
#[no_mangle]
pub extern "C" fn klatos_proses_cari(pid: u32) -> u32 {
    lock();
    let ada = find(pid).is_some();
    unlock();
    ada as u32
}

/// ppid dari `pid`, atau [`PPID_TAK_DITEMUKAN`] bila tidak terdaftar.
#[no_mangle]
pub extern "C" fn klatos_proses_ppid(pid: u32) -> u32 {
    lock();
    let v = match find(pid) {
        Some(i) => SLOT_PPID[i].load(Ordering::Relaxed),
        None => PPID_TAK_DITEMUKAN,
    };
    unlock();
    v
}

/// 1 bila `pid` adalah proses kernel, 0 bila pengguna atau tidak terdaftar.
#[no_mangle]
pub extern "C" fn klatos_proses_kernel(pid: u32) -> u32 {
    lock();
    let v = match find(pid) {
        Some(i) => {
            if SLOT_BENDERA[i].load(Ordering::Relaxed) == 2 {
                1
            } else {
                0
            }
        }
        None => 0,
    };
    unlock();
    v
}

/// Jumlah anak LANGSUNG dari `ppid` yang masih terdaftar.
#[no_mangle]
pub extern "C" fn klatos_proses_anak(ppid: u32) -> u32 {
    lock();
    let mut n = 0u32;
    let mut i = 0usize;
    while i < MAX_PROSES {
        if SLOT_BENDERA[i].load(Ordering::Relaxed) != 0
            && SLOT_PPID[i].load(Ordering::Relaxed) == ppid
        {
            n += 1;
        }
        i += 1;
    }
    unlock();
    n
}

/// Proses hidup sekarang.
#[no_mangle]
pub extern "C" fn klatos_proses_hidup() -> u32 {
    HIDUP.load(Ordering::Relaxed)
}

/// Puncak proses hidup yang pernah teramati.
#[no_mangle]
pub extern "C" fn klatos_proses_puncak() -> u32 {
    PUNCAK.load(Ordering::Relaxed)
}

/// Kapasitas tabel registry.
#[no_mangle]
pub extern "C" fn klatos_proses_kapasitas() -> u32 {
    MAX_PROSES as u32
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_proses_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset ke 0.
#[no_mangle]
pub extern "C" fn klatos_proses_reset() -> u32 {
    reset_impl();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosBukuBesarProsesReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosBukuBesarProsesReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_daftar: u32,
    pub prod_lepas: u32,
    pub prod_tolak: u32,
    pub prod_tak_ditemukan: u32,
    pub prod_anak_yatim: u32,
    pub prod_kernel: u32,
    pub prod_pengguna: u32,
    pub kapasitas: u32,
    pub hidup: u32,
    pub puncak: u32,
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
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
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

/// Matriks buku besar registry proses. Input SINTETIS.
fn run_matrix() -> KlatosBukuBesarProsesReport {
    let mut m = Matrix::new();

    // --- kelompok 0: konstanta -------------------------------------------
    m.catat(MAX_PROSES == 4096, Group::Peta);
    m.catat(REPORT_TAG == 0x4B4C_4154_5052_5347, Group::Peta);
    m.catat(klatos_proses_kapasitas() == 4096, Group::Peta);
    m.catat(PPID_TAK_DITEMUKAN == 0xFFFF_FFFF, Group::Peta);
    m.catat(klatos_proses_counter(99) == 0, Group::Peta);

    // --- kelompok 1: oracle vs produksi untuk urutan deterministik --------
    reset_impl();
    let mut oracle = Model::baru();

    // 300 pendaftaran pid 1000..1299, induk 1 (root), pengguna.
    for k in 0..300u32 {
        let pid = 1000 + k;
        let r = daftar_impl(pid, 1, 0);
        let o = oracle.daftar(pid, 1);
        m.catat(o, Group::Peta);
        m.catat(r as usize == oracle.len, Group::Peta);
        m.catat(klatos_proses_cari(pid) == 1, Group::Peta);
        m.catat(klatos_proses_ppid(pid) == 1, Group::Peta);
    }
    m.catat(klatos_proses_hidup() == 300, Group::Peta);
    m.catat(klatos_proses_puncak() == 300, Group::Peta);
    m.catat(klatos_proses_anak(1) == 300, Group::Peta);

    // 50 proses kernel (pid 2000..2049), induk 0.
    for k in 0..50u32 {
        let pid = 2000 + k;
        daftar_impl(pid, 0, 1);
        oracle.daftar(pid, 0);
        m.catat(klatos_proses_kernel(pid) == 1, Group::Peta);
    }
    m.catat(klatos_proses_hidup() == 350, Group::Peta);
    m.catat(klatos_proses_anak(0) == 50, Group::Peta);

    // Duplikat ditolak.
    for _ in 0..10u32 {
        let r = daftar_impl(1000, 1, 0);
        m.catat(r != 0, Group::Peta);
        m.catat(klatos_proses_cari(1000) == 1, Group::Peta);
    }
    m.catat(klatos_proses_hidup() == 350, Group::Peta);

    // 120 pelepasan.
    for k in 0..120u32 {
        let pid = 1000 + k;
        let r = lepas_impl(pid);
        let o = oracle.lepas(pid);
        m.catat(r == o as u32 && r == 1, Group::Peta);
    }
    m.catat(klatos_proses_hidup() == 230, Group::Peta);
    m.catat(klatos_proses_anak(1) == 180, Group::Peta);
    // Puncak tidak turun.
    m.catat(klatos_proses_puncak() == 350, Group::Peta);

    // Bandingkan produksi vs oracle per-slot.
    for k in 0..300u32 {
        let pid = 1000 + k;
        let oracle_ada = {
            let mut found = false;
            let mut i = 0usize;
            while i < MAX_PROSES {
                if oracle.used[i] && oracle.pid[i] == pid {
                    found = true;
                    break;
                }
                i += 1;
            }
            found
        };
        m.catat(klatos_proses_cari(pid) == oracle_ada as u32, Group::Peta);
    }
    m.catat(klatos_proses_hidup() as usize == oracle.len, Group::Peta);
    m.catat(klatos_proses_puncak() as usize == oracle.puncak, Group::Peta);
    // Penghitungan anak langsung: produksi vs oracle.
    m.catat(klatos_proses_anak(1) == oracle.anak(1), Group::Peta);
    m.catat(klatos_proses_anak(0) == oracle.anak(0), Group::Peta);
    m.catat(klatos_proses_anak(123456) == 0, Group::Peta);

    // --- kelompok 2: peristiwa yatim --------------------------------------
    // Daftarkan induk + 3 anak, lalu lepas induk -> anak yatim.
    reset_impl();
    daftar_impl(5000, 1, 0);
    for k in 0..3u32 {
        daftar_impl(5100 + k, 5000, 0);
    }
    m.catat(klatos_proses_anak(5000) == 3, Group::Peta);
    let sebelum_yatim = klatos_proses_counter(Counter::ProdAnakYatim as u32);
    m.catat(lepas_impl(5000) == 1, Group::Peta);
    let sesudah_yatim = klatos_proses_counter(Counter::ProdAnakYatim as u32);
    m.catat(sesudah_yatim == sebelum_yatim + 1, Group::Peta);
    // Anak tetap terdaftar walau induk hilang.
    m.catat(klatos_proses_cari(5100) == 1, Group::Peta);
    // Pelepasan daun TIDAK dihitung yatim.
    let sebelum_daun = klatos_proses_counter(Counter::ProdAnakYatim as u32);
    m.catat(lepas_impl(5100) == 1, Group::Peta);
    m.catat(
        klatos_proses_counter(Counter::ProdAnakYatim as u32) == sebelum_daun,
        Group::Peta,
    );

    // --- kelompok 3: determinisme (baca berulang jawaban sama) ------------
    {
        let prev_hidup = klatos_proses_hidup();
        let prev_puncak = klatos_proses_puncak();
        let prev_anak = klatos_proses_anak(1);
        let mut ok = true;
        for _ in 0..10_000 {
            if klatos_proses_hidup() != prev_hidup
                || klatos_proses_puncak() != prev_puncak
                || klatos_proses_anak(1) != prev_anak
            {
                ok = false;
                break;
            }
        }
        m.catat(ok, Group::Peta);
    }

    // --- kelompok 4: negatif ----------------------------------------------
    reset_impl();
    m.catat(klatos_proses_hidup() == 0, Group::Neg);
    m.catat(klatos_proses_puncak() == 0, Group::Neg);
    m.catat(klatos_proses_cari(1) == 0, Group::Neg);
    m.catat(klatos_proses_cari(1000) == 0, Group::Neg);
    m.catat(klatos_proses_ppid(1000) == PPID_TAK_DITEMUKAN, Group::Neg);
    m.catat(klatos_proses_kernel(1000) == 0, Group::Neg);
    m.catat(klatos_proses_anak(1) == 0, Group::Neg);
    // Lepas pid tak terdaftar -> 0.
    m.catat(lepas_impl(9999) == 0, Group::Neg);
    m.catat(
        klatos_proses_counter(Counter::ProdTakDitemukan as u32) == 1,
        Group::Neg,
    );
    // Duplikat dalam tabel kosong-setelah-reset: daftar dua kali.
    m.catat(daftar_impl(77, 1, 0) == 1, Group::Neg);
    let hidup_sebelum = klatos_proses_hidup();
    daftar_impl(77, 1, 0);
    m.catat(klatos_proses_hidup() == hidup_sebelum, Group::Neg);
    m.catat(klatos_proses_counter(Counter::ProdTolak as u32) >= 1, Group::Neg);
    // ppid akar (1) sendiri tidak terdaftar -> TAK_DITEMUKAN.
    m.catat(klatos_proses_ppid(1) == PPID_TAK_DITEMUKAN, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosBukuBesarProsesReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_daftar: klatos_proses_counter(Counter::ProdDaftar as u32),
        prod_lepas: klatos_proses_counter(Counter::ProdLepas as u32),
        prod_tolak: klatos_proses_counter(Counter::ProdTolak as u32),
        prod_tak_ditemukan: klatos_proses_counter(Counter::ProdTakDitemukan as u32),
        prod_anak_yatim: klatos_proses_counter(Counter::ProdAnakYatim as u32),
        prod_kernel: klatos_proses_counter(Counter::ProdKernel as u32),
        prod_pengguna: klatos_proses_counter(Counter::ProdPengguna as u32),
        kapasitas: MAX_PROSES as u32,
        hidup: klatos_proses_hidup(),
        puncak: klatos_proses_puncak(),
        status,
    }
}

/// Menjalankan matriks buku besar registry proses.
#[no_mangle]
pub extern "C" fn klatos_proses_selftest() -> KlatosBukuBesarProsesReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(MAX_PROSES == 4096);
        assert!(REPORT_TAG == 0x4B4C_4154_5052_5347);
        assert!(PPID_TAK_DITEMUKAN == 0xFFFF_FFFF);
        assert!(Counter::ProdDaftar as usize == 0);
        assert!(Counter::ProdLepas as usize == 1);
        assert!(Counter::ProdTolak as usize == 2);
        assert!(Counter::ProdTakDitemukan as usize == 3);
        assert!(Counter::ProdAnakYatim as usize == 4);
        assert!(Counter::ProdKernel as usize == 5);
        assert!(Counter::ProdPengguna as usize == 6);
        assert!(N_COUNTER == 7);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76, dibulatkan ke kelipatan
        // align 8 -> 80.
        assert!(core::mem::size_of::<KlatosBukuBesarProsesReport>() == 80);
        assert!(core::mem::align_of::<KlatosBukuBesarProsesReport>() == 8);
    };
}
