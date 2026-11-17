// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! KR-29 — **OTORITAS PENDAFTARAN HANDLER IRQ BERSAMA**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering, AtomicU64};

/// Tag laporan: "KLATIRQR".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4952_5152;

/// Ajaib penanda blok: "IRQR".
pub const AJAIB: u32 = 0x4952_5152;

/// Kapasitas arena simpul global (handler terdaftar serentak). Registri IRQ
/// bersama berukuran kecil; batas longgar + gagal-tertutup.
pub const N_NODE: usize = 4096;

/// `-EINVAL`.
pub const E_INVAL: i32 = 22;
/// `-ENOSPC`.
pub const E_NOSPC: i32 = 28;

// --- blok keadaan -----------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PendaftaranKeadaan {
    pub kepala: u32,
    pub ekor: u32,
    pub jumlah: u32,
    pub generasi: u32,
    pub aktif: u32,
    pub ajaib: u32,
    pub cadangan0: u32,
    pub cadangan1: u32,
}

impl PendaftaranKeadaan {
    pub const fn kosong() -> Self {
        PendaftaranKeadaan {
            kepala: 0,
            ekor: 0,
            jumlah: 0,
            generasi: 0,
            aktif: 0,
            ajaib: 0,
            cadangan0: 0,
            cadangan1: 0,
        }
    }
}

// --- arena simpul -----------------------------------------------------------

#[derive(Clone, Copy)]
struct Node {
    hidup: bool,
    prev: u32,
    next: u32,
    token: u64,
}
const NODE_KOSONG: Node = Node { hidup: false, prev: 0, next: 0, token: 0 };

struct Kolam {
    init: bool,
    bebas: u32,
    hidup: u32,
    node: [Node; N_NODE],
}
static mut KOLAM: Kolam = Kolam { init: false, bebas: 0, hidup: 0, node: [NODE_KOSONG; N_NODE] };
static IRQ_LOCK: AtomicBool = AtomicBool::new(false);
/// Status interupsi pemegang IRQ_LOCK. Hanya pemegang yang menulis dan
/// membacanya sebelum melepas, jadi tidak perlu atomik kuat.
static IRQ_LOCK_FLAGS: AtomicU64 = AtomicU64::new(0);

#[inline]
fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci. Pemegang kunci tidak boleh
    // di-preempt dan handler interupsi tidak boleh masuk lagi ke bagian kritis;
    // keduanya membuat penunggu berputar selamanya (P0-1).
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while IRQ_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    IRQ_LOCK_FLAGS.store(flags, Ordering::Relaxed);
}
#[inline]
fn unlock() {
    // Baca status milik sendiri SEBELUM melepas kunci, supaya pengambil
    // berikutnya tidak bisa menimpa nilainya.
    let flags = IRQ_LOCK_FLAGS.load(Ordering::Relaxed);
    IRQ_LOCK.store(false, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

#[inline]
unsafe fn kolam<'a>() -> &'a mut Kolam {
    unsafe { &mut *core::ptr::addr_of_mut!(KOLAM) }
}

impl Kolam {
    fn pastikan(&mut self) {
        if self.init {
            return;
        }
        let mut i = 1u32;
        while (i as usize) < N_NODE {
            self.node[i as usize - 1].next = i + 1;
            self.node[i as usize - 1].hidup = false;
            i += 1;
        }
        self.node[N_NODE - 1].next = 0;
        self.node[N_NODE - 1].hidup = false;
        self.bebas = 1;
        self.hidup = 0;
        self.init = true;
    }
    fn alok(&mut self) -> u32 {
        if self.bebas == 0 {
            return 0;
        }
        let id = self.bebas;
        let next = self.node[id as usize - 1].next;
        self.bebas = next;
        let n = &mut self.node[id as usize - 1];
        n.hidup = true;
        n.prev = 0;
        n.next = 0;
        self.hidup += 1;
        id
    }
    fn bebaskan(&mut self, id: u32) {
        if id == 0 || id as usize > N_NODE || !self.node[id as usize - 1].hidup {
            return;
        }
        let b = self.bebas;
        let n = &mut self.node[id as usize - 1];
        n.hidup = false;
        n.prev = 0;
        n.next = b;
        n.token = 0;
        self.bebas = id;
        self.hidup -= 1;
    }
    fn link_ekor(&mut self, q: &mut PendaftaranKeadaan, id: u32) {
        self.node[id as usize - 1].prev = q.ekor;
        self.node[id as usize - 1].next = 0;
        if q.ekor != 0 {
            self.node[q.ekor as usize - 1].next = id;
        } else {
            q.kepala = id;
        }
        q.ekor = id;
        q.jumlah += 1;
    }
    fn lepas_simpul(&mut self, q: &mut PendaftaranKeadaan, id: u32) {
        let p = self.node[id as usize - 1].prev;
        let n = self.node[id as usize - 1].next;
        if p != 0 {
            self.node[p as usize - 1].next = n;
        } else {
            q.kepala = n;
        }
        if n != 0 {
            self.node[n as usize - 1].prev = p;
        } else {
            q.ekor = p;
        }
        if q.jumlah > 0 {
            q.jumlah -= 1;
        }
        self.bebaskan(id);
    }
    fn ke(&self, q: &PendaftaranKeadaan, i: u32) -> u32 {
        let mut id = q.kepala;
        let mut k = i;
        while k > 0 && id != 0 {
            id = self.node[id as usize - 1].next;
            k -= 1;
        }
        id
    }
}

// --- penghitung audit -------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    ProdBuat = 0,
    ProdDaftar = 1,
    ProdLepas = 2,
    ProdBersih = 3,
    TolakPenuh = 4,
    TolakParam = 5,
}
const N_COUNTER: usize = 6;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- pintu C ABI ------------------------------------------------------------

/// Inisialisasi blok (idempoten). `aktif` awal = 1 (cermin `m_enabled { true }`).
///
/// # Safety
/// `k` harus dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_irq_buat(k: *mut PendaftaranKeadaan) -> i32 {
    if k.is_null() {
        bump(Counter::TolakParam);
        return -E_INVAL;
    }
    let q = unsafe { &mut *k };
    let g = q.generasi.wrapping_add(1);
    *q = PendaftaranKeadaan::kosong();
    q.generasi = g;
    q.aktif = 1;
    q.ajaib = AJAIB;
    bump(Counter::ProdBuat);
    0
}

/// Daftarkan handler (token) di ekor. `0` sukses, `-ENOSPC` arena penuh.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_irq_daftar(k: *mut PendaftaranKeadaan, token: u64) -> i32 {
    if k.is_null() {
        bump(Counter::TolakParam);
        return -E_INVAL;
    }
    lock();
    let kk = unsafe { kolam() };
    kk.pastikan();
    let q = unsafe { &mut *k };
    if q.ajaib != AJAIB {
        q.ajaib = AJAIB;
    }
    let id = kk.alok();
    if id == 0 {
        unlock();
        bump(Counter::TolakPenuh);
        return -E_NOSPC;
    }
    kk.node[id as usize - 1].token = token;
    kk.link_ekor(q, id);
    unlock();
    bump(Counter::ProdDaftar);
    0
}

/// Lepas SEMUA anggota dengan `token`; `1` bila registri kini KOSONG, `0` bila
/// tidak (kebijakan auto-disable C++ memakai ini).
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_irq_lepas(k: *mut PendaftaranKeadaan, token: u64) -> u32 {
    if k.is_null() {
        return 0;
    }
    lock();
    let kk = unsafe { kolam() };
    kk.pastikan();
    let q = unsafe { &mut *k };
    let mut id = q.kepala;
    while id != 0 {
        let n = kk.node[id as usize - 1].next;
        if kk.node[id as usize - 1].token == token {
            kk.lepas_simpul(q, id);
        }
        id = n;
    }
    let kosong = q.jumlah == 0;
    unlock();
    bump(Counter::ProdLepas);
    if kosong {
        1
    } else {
        0
    }
}

/// Jumlah anggota.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_irq_jumlah(k: *const PendaftaranKeadaan) -> u32 {
    if k.is_null() {
        return 0;
    }
    unsafe { (*k).jumlah }
}

/// Baca anggota ke-`i` (0-basis dari kepala). `0` sukses, `-EINVAL` di luar
/// rentang.
///
/// # Safety
/// `k` valid; `out_token` (bila non-null) dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_irq_ambil(
    k: *mut PendaftaranKeadaan,
    i: u32,
    out_token: *mut u64,
) -> i32 {
    if k.is_null() {
        bump(Counter::TolakParam);
        return -E_INVAL;
    }
    lock();
    let kk = unsafe { kolam() };
    kk.pastikan();
    let q = unsafe { &*k };
    if i >= q.jumlah {
        unlock();
        return -E_INVAL;
    }
    let id = kk.ke(q, i);
    if id == 0 {
        unlock();
        return -E_INVAL;
    }
    let t = kk.node[id as usize - 1].token;
    unlock();
    if !out_token.is_null() {
        unsafe { core::ptr::write(out_token, t) };
    }
    0
}

/// Transisi latch aktif. `1` bila keadaan BERUBAH (pemanggil harus
/// enable/disable pengendali), `0` bila sudah dalam keadaan itu.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_irq_aktif_transisi(k: *mut PendaftaranKeadaan, want: u32) -> u32 {
    if k.is_null() {
        return 0;
    }
    let q = unsafe { &mut *k };
    let want = if want != 0 { 1 } else { 0 };
    if q.aktif == want {
        return 0;
    }
    q.aktif = want;
    1
}

/// Nilai latch aktif.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_irq_aktif(k: *const PendaftaranKeadaan) -> u32 {
    if k.is_null() {
        return 0;
    }
    unsafe { (*k).aktif }
}

/// Bebaskan seluruh anggota (tanpa mengubah `aktif`). Mengembalikan jumlah.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_irq_bersihkan(k: *mut PendaftaranKeadaan) -> u32 {
    if k.is_null() {
        return 0;
    }
    lock();
    let kk = unsafe { kolam() };
    kk.pastikan();
    let q = unsafe { &mut *k };
    let mut c = 0u32;
    let mut id = q.kepala;
    while id != 0 {
        let n = kk.node[id as usize - 1].next;
        kk.bebaskan(id);
        c += 1;
        id = n;
    }
    q.kepala = 0;
    q.ekor = 0;
    q.jumlah = 0;
    q.generasi = q.generasi.wrapping_add(1);
    unlock();
    if c > 0 {
        bump(Counter::ProdBersih);
    }
    c
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_irq_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung.
#[no_mangle]
pub extern "C" fn klatos_irq_reset_counter() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosIrqReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_daftar: u32,
    pub prod_lepas: u32,
    pub prod_bersih: u32,
    pub tolak_penuh: u32,
    pub cadangan0: u32,
    pub cadangan1: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosIrqReport>()];

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
    fn catat(&mut self, l: bool) {
        self.checks += 1;
        if l {
            self.passed += 1;
        }
    }
    fn neg(&mut self, l: bool) {
        self.neg_checks += 1;
        if l {
            self.neg_passed += 1;
        }
    }
    fn dif(&mut self, l: bool) {
        self.dif_checks += 1;
        if !l {
            self.dif_beda += 1;
        }
    }
}

const MODEL_MAX: usize = 256;
struct Model {
    n: usize,
    tok: [u64; MODEL_MAX],
}
impl Model {
    fn baru() -> Self {
        Model { n: 0, tok: [0; MODEL_MAX] }
    }
    fn daftar(&mut self, t: u64) -> bool {
        if self.n >= MODEL_MAX {
            return false;
        }
        self.tok[self.n] = t;
        self.n += 1;
        true
    }
    fn lepas(&mut self, t: u64) -> u32 {
        let mut i = 0usize;
        while i < self.n {
            if self.tok[i] == t {
                let mut j = i;
                while j + 1 < self.n {
                    self.tok[j] = self.tok[j + 1];
                    j += 1;
                }
                self.n -= 1;
            } else {
                i += 1;
            }
        }
        if self.n == 0 {
            1
        } else {
            0
        }
    }
    fn ambil(&self, i: usize) -> Option<u64> {
        if i >= self.n {
            None
        } else {
            Some(self.tok[i])
        }
    }
}

fn run_matrix() -> KlatosIrqReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_4952_5152);
    m.catat(AJAIB == 0x4952_5152);
    m.catat(N_NODE == 4096);
    m.catat(E_INVAL == 22 && E_NOSPC == 28);
    m.catat(core::mem::size_of::<PendaftaranKeadaan>() == 32);

    let _ = klatos_irq_reset_counter();

    // A) Kosong + aktif awal 1.
    let mut q = PendaftaranKeadaan::kosong();
    let pq = &mut q as *mut PendaftaranKeadaan;
    m.catat(unsafe { klatos_irq_buat(pq) } == 0);
    m.catat(unsafe { klatos_irq_jumlah(pq) } == 0);
    m.catat(unsafe { klatos_irq_aktif(pq) } == 1);

    // B) Daftar + urutan.
    m.catat(unsafe { klatos_irq_daftar(pq, 0xA1) } == 0);
    m.catat(unsafe { klatos_irq_daftar(pq, 0xA2) } == 0);
    m.catat(unsafe { klatos_irq_daftar(pq, 0xA3) } == 0);
    m.catat(unsafe { klatos_irq_jumlah(pq) } == 3);
    let mut t = 0u64;
    m.catat(unsafe { klatos_irq_ambil(pq, 0, &mut t) } == 0 && t == 0xA1);
    m.catat(unsafe { klatos_irq_ambil(pq, 1, &mut t) } == 0 && t == 0xA2);
    m.catat(unsafe { klatos_irq_ambil(pq, 2, &mut t) } == 0 && t == 0xA3);
    m.neg(unsafe { klatos_irq_ambil(pq, 3, &mut t) } == -E_INVAL);

    // C) Lepas + auto-disable.
    m.catat(unsafe { klatos_irq_lepas(pq, 0xA2) } == 0); // masih ada sisa
    m.catat(unsafe { klatos_irq_jumlah(pq) } == 2);
    m.catat(unsafe { klatos_irq_ambil(pq, 0, &mut t) } == 0 && t == 0xA1);
    m.catat(unsafe { klatos_irq_ambil(pq, 1, &mut t) } == 0 && t == 0xA3);
    m.catat(unsafe { klatos_irq_lepas(pq, 0xA1) } == 0);
    m.catat(unsafe { klatos_irq_lepas(pq, 0xA3) } == 1); // kini KOSONG -> disable
    m.catat(unsafe { klatos_irq_jumlah(pq) } == 0);
    // Lepas token yang tak ada pada registri kosong -> tetap 1 (kosong).
    m.catat(unsafe { klatos_irq_lepas(pq, 0xBB) } == 1);

    // D) Latch aktif idempoten.
    m.catat(unsafe { klatos_irq_aktif_transisi(pq, 1) } == 0); // sudah 1
    m.catat(unsafe { klatos_irq_aktif_transisi(pq, 0) } == 1); // berubah
    m.catat(unsafe { klatos_irq_aktif(pq) } == 0);
    m.catat(unsafe { klatos_irq_aktif_transisi(pq, 0) } == 0); // idempoten
    m.catat(unsafe { klatos_irq_aktif_transisi(pq, 1) } == 1);
    m.catat(unsafe { klatos_irq_aktif(pq) } == 1);

    // E) Param null.
    m.neg(unsafe { klatos_irq_buat(core::ptr::null_mut()) } == -E_INVAL);
    m.neg(unsafe { klatos_irq_daftar(core::ptr::null_mut(), 1) } == -E_INVAL);
    m.neg(unsafe { klatos_irq_lepas(core::ptr::null_mut(), 1) } == 0);
    m.neg(unsafe { klatos_irq_jumlah(core::ptr::null()) } == 0);

    // F) Bersihkan.
    m.catat(unsafe { klatos_irq_daftar(pq, 1) } == 0);
    m.catat(unsafe { klatos_irq_daftar(pq, 2) } == 0);
    m.catat(unsafe { klatos_irq_bersihkan(pq) } == 2);
    m.catat(unsafe { klatos_irq_jumlah(pq) } == 0);

    // G) Diferensial vs model murni (LCG deterministik).
    let mut qd = PendaftaranKeadaan::kosong();
    let pqd = &mut qd as *mut PendaftaranKeadaan;
    m.catat(unsafe { klatos_irq_buat(pqd) } == 0);
    let mut mdl = Model::baru();
    let mut seed: u64 = 0x2026_1005_0000_2912;
    let mut step = 0u32;
    while step < 20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let op = (seed >> 60) & 1;
        let tok = (seed >> 3) & 0x07;
        if op == 0 {
            let rg = unsafe { klatos_irq_daftar(pqd, tok) } == 0;
            let rm = mdl.daftar(tok);
            m.dif(rg == rm);
        } else {
            let rg = unsafe { klatos_irq_lepas(pqd, tok) };
            let rm = mdl.lepas(tok);
            m.dif(rg == rm);
        }
        m.dif(unsafe { klatos_irq_jumlah(pqd) } as usize == mdl.n);
        let i = (seed >> 20) as u32 % 4;
        let rg = unsafe { klatos_irq_ambil(pqd, i, core::ptr::null_mut()) } == 0;
        let rm = mdl.ambil(i as usize).is_some();
        m.dif(rg == rm);
        step += 1;
    }
    m.catat(unsafe { klatos_irq_bersihkan(pqd) } == mdl.n as u32);

    let prod_buat = COUNTERS[Counter::ProdBuat as usize].load(Ordering::Relaxed);
    let prod_daftar = COUNTERS[Counter::ProdDaftar as usize].load(Ordering::Relaxed);
    let prod_lepas = COUNTERS[Counter::ProdLepas as usize].load(Ordering::Relaxed);
    let prod_bersih = COUNTERS[Counter::ProdBersih as usize].load(Ordering::Relaxed);
    let tolak_penuh = COUNTERS[Counter::TolakPenuh as usize].load(Ordering::Relaxed);
    let _ = klatos_irq_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && m.dif_checks > 0;
    KlatosIrqReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_buat,
        prod_daftar,
        prod_lepas,
        prod_bersih,
        tolak_penuh,
        cadangan0: 0,
        cadangan1: 0,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas pendaftaran IRQ.
#[no_mangle]
pub extern "C" fn klatos_irq_selftest() -> KlatosIrqReport {
    assert!(core::mem::size_of::<KlatosIrqReport>() == 64);
    assert!(core::mem::offset_of!(KlatosIrqReport, status) == 60);
    run_matrix()
}
