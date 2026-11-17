// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! KERNEL-RUST KR-WAIT — **OTORITAS ANTREAN TUNGGU / PEMBENDUNG (generik)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering, AtomicU64};

/// Tag laporan: "KLATANTR".
pub const REPORT_TAG: u64 = 0x4B4C_4154_414E_5452;

/// Ajaib penanda blok terinisialisasi.
pub const AJAIB: u32 = 0x414E_5452; // "ANTR"

/// Ukuran arena simpul global. Batas = jumlah keanggotaan (penunggu,antrean)
/// serentak; gagal-tertutup (`-ENOSPC`) bila terlampaui (C++ asli akan
/// kehabisan memori). Dipilih longgar untuk beban kernel nyata.
pub const N_NODE: usize = 32768;

/// `-EBADF` — blok/parameter tak sah.
pub const E_BADF: i32 = 9;
/// `-EINVAL` — indeks/parameter tak sah.
pub const E_INVAL: i32 = 22;
/// `-ENOSPC` — arena penuh.
pub const E_NOSPC: i32 = 28;

// --- keadaan antrean (blok POD yang disematkan di objek C++) ----------------

/// Blok keadaan antrean. Disematkan di `Jowo::WaitQueue` dan
/// `Jowo::Thread::BlockerSet`; hanya modul ini yang menafsirkannya.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AntreanKeadaan {
    /// Simpul kepala (id+1; 0 = kosong).
    pub kepala: u32,
    /// Simpul ekor (id+1; 0 = kosong).
    pub ekor: u32,
    /// Jumlah anggota.
    pub jumlah: u32,
    /// Generasi blok (naik pada `buat`/`bersihkan`).
    pub generasi: u32,
    /// Futex: penunggu yang akan memblok (`m_imminent_waits`).
    pub imminent: u32,
    /// Bendera serbaguna: futex bit0 = `m_was_removed`, antrean-usang bit0 =
    /// `m_wake_requested`.
    pub bendera: u32,
    /// Ajaib penanda (`AJAIB`).
    pub ajaib: u32,
    /// Cadangan (padding eksplisit).
    pub cadangan: u32,
}

impl AntreanKeadaan {
    /// Blok kosong.
    pub const fn kosong() -> Self {
        AntreanKeadaan {
            kepala: 0,
            ekor: 0,
            jumlah: 0,
            generasi: 0,
            imminent: 0,
            bendera: 0,
            ajaib: 0,
            cadangan: 0,
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
    data: u64,
}

const NODE_KOSONG: Node = Node { hidup: false, prev: 0, next: 0, token: 0, data: 0 };

struct Kolam {
    init: bool,
    bebas: u32,
    hidup: u32,
    node: [Node; N_NODE],
}

static mut KOLAM: Kolam = Kolam { init: false, bebas: 0, hidup: 0, node: [NODE_KOSONG; N_NODE] };

static ANTR_LOCK: AtomicBool = AtomicBool::new(false);
/// Status interupsi pemegang ANTR_LOCK. Hanya pemegang yang menulis dan
/// membacanya sebelum melepas, jadi tidak perlu atomik kuat.
static ANTR_LOCK_FLAGS: AtomicU64 = AtomicU64::new(0);

#[inline]
fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci. Pemegang kunci tidak boleh
    // di-preempt dan handler interupsi tidak boleh masuk lagi ke bagian kritis;
    // keduanya membuat penunggu berputar selamanya (P0-1).
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while ANTR_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    ANTR_LOCK_FLAGS.store(flags, Ordering::Relaxed);
}

#[inline]
fn unlock() {
    // Baca status milik sendiri SEBELUM melepas kunci, supaya pengambil
    // berikutnya tidak bisa menimpa nilainya.
    let flags = ANTR_LOCK_FLAGS.load(Ordering::Relaxed);
    ANTR_LOCK.store(false, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Penghitung audit.
#[derive(Clone, Copy)]
pub enum Counter {
    ProdBuat = 0,
    ProdLepas = 1,
    ProdTambah = 2,
    ProdHapusToken = 3,
    ProdHapusPada = 4,
    ProdAmbil = 5,
    ProdBersih = 6,
    TolakPenuh = 7,
    TolakIndeks = 8,
    TolakParam = 9,
    /// Pelanggaran: urutan FIFO rusak (WAJIB 0).
    LanggarUrutan = 10,
    /// Pelanggaran: simpul bocor / dobel-bebas (WAJIB 0).
    LanggarBocor = 11,
    /// Pelanggaran: arena tak konsisten (WAJIB 0).
    LanggarArena = 12,
}
const N_COUNTER: usize = 13;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

#[inline]
fn set_langgar(c: Counter) {
    COUNTERS[c as usize].store(1, Ordering::Relaxed);
}

// --- operasi arena murni (dapat diuji tanpa state global) -------------------

impl Kolam {
    /// Bangun daftar-bebas bila belum.
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

    /// Ambil satu simpul bebas (id+1; 0 = penuh).
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

    /// Kembalikan simpul ke daftar-bebas.
    fn bebaskan(&mut self, id: u32) {
        if id == 0 || id as usize > N_NODE {
            set_langgar(Counter::LanggarArena);
            return;
        }
        if !self.node[id as usize - 1].hidup {
            set_langgar(Counter::LanggarBocor);
            return;
        }
        let b = self.bebas;
        let n = &mut self.node[id as usize - 1];
        n.hidup = false;
        n.prev = 0;
        n.next = b;
        n.token = 0;
        n.data = 0;
        self.bebas = id;
        self.hidup -= 1;
    }

    fn link_ekor(&mut self, q: &mut AntreanKeadaan, id: u32) {
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

    fn lepas_simpul(&mut self, q: &mut AntreanKeadaan, id: u32) {
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
        } else {
            set_langgar(Counter::LanggarArena);
        }
        self.bebaskan(id);
    }

    /// Simpul ke-`i` (0-basis) dari kepala; 0 bila di luar rentang.
    fn ke(&self, q: &AntreanKeadaan, i: u32) -> u32 {
        let mut id = q.kepala;
        let mut k = i;
        while k > 0 && id != 0 {
            id = self.node[id as usize - 1].next;
            k -= 1;
        }
        id
    }
}

// --- model murni (untuk diferensial) ----------------------------------------

const MODEL_MAX: usize = 1024;

struct Model {
    n: usize,
    tok: [u64; MODEL_MAX],
    dat: [u64; MODEL_MAX],
}

impl Model {
    fn baru() -> Self {
        Model { n: 0, tok: [0; MODEL_MAX], dat: [0; MODEL_MAX] }
    }
    fn tambah(&mut self, t: u64, d: u64) -> bool {
        if self.n >= MODEL_MAX {
            return false;
        }
        self.tok[self.n] = t;
        self.dat[self.n] = d;
        self.n += 1;
        true
    }
    fn hapus_token(&mut self, t: u64) -> u32 {
        let mut c = 0u32;
        let mut i = 0usize;
        while i < self.n {
            if self.tok[i] == t {
                let mut j = i;
                while j + 1 < self.n {
                    self.tok[j] = self.tok[j + 1];
                    self.dat[j] = self.dat[j + 1];
                    j += 1;
                }
                self.n -= 1;
                c += 1;
            } else {
                i += 1;
            }
        }
        c
    }
    fn hapus_pada(&mut self, i: usize) -> bool {
        if i >= self.n {
            return false;
        }
        let mut j = i;
        while j + 1 < self.n {
            self.tok[j] = self.tok[j + 1];
            self.dat[j] = self.dat[j + 1];
            j += 1;
        }
        self.n -= 1;
        true
    }
    fn ambil(&self, i: usize) -> Option<(u64, u64)> {
        if i >= self.n {
            return None;
        }
        Some((self.tok[i], self.dat[i]))
    }
}

// --- pintu C ABI ------------------------------------------------------------

#[inline]
unsafe fn kolam<'a>() -> &'a mut Kolam {
    unsafe { &mut *core::ptr::addr_of_mut!(KOLAM) }
}

/// Inisialisasi blok antrean (idempoten). `0` selalu.
///
/// # Safety
/// `k` harus dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_buat(k: *mut AntreanKeadaan) -> i32 {
    if k.is_null() {
        bump(Counter::TolakParam);
        return -E_INVAL;
    }
    let q = unsafe { &mut *k };
    let g = q.generasi.wrapping_add(1);
    *q = AntreanKeadaan::kosong();
    q.generasi = g;
    q.ajaib = AJAIB;
    bump(Counter::ProdBuat);
    0
}

/// Lepas blok antrean: bebaskan seluruh simpul lalu nolkan. Mengembalikan
/// jumlah simpul yang dibebaskan.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_lepas(k: *mut AntreanKeadaan) -> u32 {
    if k.is_null() {
        return 0;
    }
    lock();
    let kk = unsafe { kolam() };
    kk.pastikan();
    let q = unsafe { &mut *k };
    let mut dibebaskan = 0u32;
    let mut id = q.kepala;
    while id != 0 {
        let n = kk.node[id as usize - 1].next;
        kk.bebaskan(id);
        dibebaskan += 1;
        id = n;
    }
    let g = q.generasi.wrapping_add(1);
    *q = AntreanKeadaan::kosong();
    q.generasi = g;
    unlock();
    bump(Counter::ProdLepas);
    dibebaskan
}

/// Tambah `(token,data)` di ekor. `0` sukses, `-ENOSPC` arena penuh,
/// `-EINVAL` parameter tak sah.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_tambah(
    k: *mut AntreanKeadaan,
    token: u64,
    data: u64,
) -> i32 {
    if k.is_null() {
        bump(Counter::TolakParam);
        return -E_INVAL;
    }
    lock();
    let kk = unsafe { kolam() };
    kk.pastikan();
    let q = unsafe { &mut *k };
    if q.ajaib != AJAIB {
        // Toleran: blok nol (value-init C++) adalah antrean kosong yang sah.
        q.ajaib = AJAIB;
    }
    let id = kk.alok();
    if id == 0 {
        unlock();
        bump(Counter::TolakPenuh);
        return -E_NOSPC;
    }
    kk.node[id as usize - 1].token = token;
    kk.node[id as usize - 1].data = data;
    kk.link_ekor(q, id);
    unlock();
    bump(Counter::ProdTambah);
    0
}

/// Hapus SEMUA anggota dengan `token`; mengembalikan jumlah yang dihapus.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_hapus_token(k: *mut AntreanKeadaan, token: u64) -> u32 {
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
        if kk.node[id as usize - 1].token == token {
            kk.lepas_simpul(q, id);
            c += 1;
        }
        id = n;
    }
    unlock();
    if c > 0 {
        bump(Counter::ProdHapusToken);
    }
    c
}

/// Hapus anggota ke-`i` (0-basis dari kepala). `0` sukses, `-EINVAL` di luar
/// rentang.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_hapus_pada(k: *mut AntreanKeadaan, i: u32) -> i32 {
    if k.is_null() {
        bump(Counter::TolakParam);
        return -E_INVAL;
    }
    lock();
    let kk = unsafe { kolam() };
    kk.pastikan();
    let q = unsafe { &mut *k };
    if i >= q.jumlah {
        unlock();
        bump(Counter::TolakIndeks);
        return -E_INVAL;
    }
    let id = kk.ke(q, i);
    if id == 0 {
        unlock();
        bump(Counter::TolakIndeks);
        return -E_INVAL;
    }
    kk.lepas_simpul(q, id);
    unlock();
    bump(Counter::ProdHapusPada);
    0
}

/// Baca anggota ke-`i`. `0` sukses; `-EINVAL` di luar rentang. `out_*` boleh
/// null bila nilainya tidak diperlukan.
///
/// # Safety
/// `k` harus menunjuk blok yang valid; `out_*` (bila non-null) dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_ambil(
    k: *mut AntreanKeadaan,
    i: u32,
    out_token: *mut u64,
    out_data: *mut u64,
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
        bump(Counter::TolakIndeks);
        return -E_INVAL;
    }
    let id = kk.ke(q, i);
    if id == 0 {
        unlock();
        bump(Counter::TolakIndeks);
        return -E_INVAL;
    }
    let t = kk.node[id as usize - 1].token;
    let d = kk.node[id as usize - 1].data;
    unlock();
    if !out_token.is_null() {
        unsafe { core::ptr::write(out_token, t) };
    }
    if !out_data.is_null() {
        unsafe { core::ptr::write(out_data, d) };
    }
    bump(Counter::ProdAmbil);
    0
}

/// Jumlah anggota.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_jumlah(k: *const AntreanKeadaan) -> u32 {
    if k.is_null() {
        return 0;
    }
    unsafe { (*k).jumlah }
}

/// `1` bila kosong, `0` bila tidak.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_kosong(k: *const AntreanKeadaan) -> u32 {
    if k.is_null() {
        return 1;
    }
    if unsafe { (*k).jumlah } == 0 {
        1
    } else {
        0
    }
}

/// Bebaskan seluruh anggota (tanpa mengubah `generasi`/bendera). Mengembalikan
/// jumlah yang dibebaskan.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_bersihkan(k: *mut AntreanKeadaan) -> u32 {
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

/// Baca bendera serbaguna.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_bendera_ambil(k: *const AntreanKeadaan) -> u32 {
    if k.is_null() {
        return 0;
    }
    unsafe { (*k).bendera }
}

/// Set bendera serbaguna; mengembalikan nilai lama.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_bendera_set(k: *mut AntreanKeadaan, v: u32) -> u32 {
    if k.is_null() {
        return 0;
    }
    let q = unsafe { &mut *k };
    let old = q.bendera;
    q.bendera = v;
    old
}

/// Baca `imminent` (penunggu yang akan memblok).
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_imminent_ambil(k: *const AntreanKeadaan) -> u32 {
    if k.is_null() {
        return 0;
    }
    unsafe { (*k).imminent }
}

/// Set `imminent`; mengembalikan nilai lama.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_imminent_set(k: *mut AntreanKeadaan, v: u32) -> u32 {
    if k.is_null() {
        return 0;
    }
    let q = unsafe { &mut *k };
    let old = q.imminent;
    q.imminent = v;
    old
}

/// Baca generasi blok.
///
/// # Safety
/// `k` harus menunjuk blok yang valid.
#[no_mangle]
pub unsafe extern "C" fn klatos_antrean_generasi(k: *const AntreanKeadaan) -> u32 {
    if k.is_null() {
        return 0;
    }
    unsafe { (*k).generasi }
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_antrean_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh penghitung saja (aman dipanggil saat kernel hidup — arena
/// TIDAK disentuh).
#[no_mangle]
pub extern "C" fn klatos_antrean_reset_counter() -> u32 {
    let mut c = 0usize;
    while c < N_COUNTER {
        COUNTERS[c].store(0, Ordering::Relaxed);
        c += 1;
    }
    0
}

/// Reset arena + seluruh penghitung (hanya untuk uji/proses terisolasi —
/// JANGAN dipanggil saat ada antrean hidup).
#[no_mangle]
pub extern "C" fn klatos_antrean_reset() -> u32 {
    lock();
    let kk = unsafe { kolam() };
    let mut i = 0usize;
    while i < N_NODE {
        kk.node[i] = NODE_KOSONG;
        i += 1;
    }
    kk.init = false;
    kk.bebas = 0;
    kk.hidup = 0;
    kk.pastikan();
    unlock();
    let mut c = 0usize;
    while c < N_COUNTER {
        COUNTERS[c].store(0, Ordering::Relaxed);
        c += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas antrean (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosAntreanTungguReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_tambah: u32,
    pub prod_hapus_token: u32,
    pub prod_hapus_pada: u32,
    pub prod_bersih: u32,
    pub tolak_penuh: u32,
    pub langgar: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosAntreanTungguReport>()];

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

/// Ambil `(token,data)` pada indeks `i` dari antrean nyata.
unsafe fn ambil(k: *mut AntreanKeadaan, i: u32) -> Option<(u64, u64)> {
    let mut t = 0u64;
    let mut d = 0u64;
    let r = unsafe { klatos_antrean_ambil(k, i, &mut t, &mut d) };
    if r == 0 {
        Some((t, d))
    } else {
        None
    }
}

fn run_matrix() -> KlatosAntreanTungguReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_414E_5452);
    m.catat(AJAIB == 0x414E_5452);
    m.catat(N_NODE == 32768);
    m.catat(E_BADF == 9 && E_INVAL == 22 && E_NOSPC == 28);
    m.catat(core::mem::size_of::<AntreanKeadaan>() == 32);

    // Aman saat kernel hidup: arena global TIDAK di-reset (antrean nyata
    // memakainya); hanya penghitung yang dinolkan.
    let _ = klatos_antrean_reset_counter();

    // A) Kosong.
    let mut q = AntreanKeadaan::kosong();
    let pq = &mut q as *mut AntreanKeadaan;
    m.catat(unsafe { klatos_antrean_buat(pq) } == 0);
    m.catat(unsafe { klatos_antrean_kosong(pq) } == 1);
    m.catat(unsafe { klatos_antrean_jumlah(pq) } == 0);
    m.neg(unsafe { ambil(pq, 0) }.is_none());
    m.neg(unsafe { klatos_antrean_hapus_pada(pq, 0) } == -E_INVAL);
    m.neg(unsafe { klatos_antrean_hapus_token(pq, 0xAA) } == 0);

    // B) Tunggal.
    m.catat(unsafe { klatos_antrean_tambah(pq, 0x11, 0x21) } == 0);
    m.catat(unsafe { klatos_antrean_kosong(pq) } == 0);
    m.catat(unsafe { klatos_antrean_jumlah(pq) } == 1);
    m.catat(unsafe { ambil(pq, 0) } == Some((0x11, 0x21)));
    m.neg(unsafe { ambil(pq, 1) }.is_none());
    m.neg(unsafe { klatos_antrean_hapus_token(pq, 0x11) } == 1);
    m.catat(unsafe { klatos_antrean_kosong(pq) } == 1);

    // C) Banyak + urutan FIFO.
    m.catat(unsafe { klatos_antrean_tambah(pq, 1, 10) } == 0);
    m.catat(unsafe { klatos_antrean_tambah(pq, 2, 20) } == 0);
    m.catat(unsafe { klatos_antrean_tambah(pq, 3, 30) } == 0);
    m.catat(unsafe { klatos_antrean_jumlah(pq) } == 3);
    m.catat(unsafe { ambil(pq, 0) } == Some((1, 10)));
    m.catat(unsafe { ambil(pq, 1) } == Some((2, 20)));
    m.catat(unsafe { ambil(pq, 2) } == Some((3, 30)));
    // Urutan WAJIB kepala→ekor.
    let fifo_ok = unsafe { ambil(pq, 0) } == Some((1, 10))
        && unsafe { ambil(pq, 1) } == Some((2, 20))
        && unsafe { ambil(pq, 2) } == Some((3, 30));
    if !fifo_ok {
        set_langgar(Counter::LanggarUrutan);
    }

    // D) Hapus tengah.
    m.catat(unsafe { klatos_antrean_hapus_pada(pq, 1) } == 0);
    m.catat(unsafe { klatos_antrean_jumlah(pq) } == 2);
    m.catat(unsafe { ambil(pq, 0) } == Some((1, 10)));
    m.catat(unsafe { ambil(pq, 1) } == Some((3, 30)));
    m.catat(unsafe { klatos_antrean_hapus_pada(pq, 0) } == 0);
    m.catat(unsafe { ambil(pq, 0) } == Some((3, 30)));
    m.catat(unsafe { klatos_antrean_hapus_pada(pq, 0) } == 0);
    m.catat(unsafe { klatos_antrean_kosong(pq) } == 1);

    // E) Hapus token ganda.
    m.catat(unsafe { klatos_antrean_tambah(pq, 7, 1) } == 0);
    m.catat(unsafe { klatos_antrean_tambah(pq, 7, 2) } == 0);
    m.catat(unsafe { klatos_antrean_tambah(pq, 8, 3) } == 0);
    m.neg(unsafe { klatos_antrean_hapus_token(pq, 7) } == 2);
    m.catat(unsafe { klatos_antrean_jumlah(pq) } == 1);
    m.catat(unsafe { ambil(pq, 0) } == Some((8, 3)));
    m.catat(unsafe { klatos_antrean_bersihkan(pq) } == 1);

    // F) Pemakaian ulang simpul (add/hapus/add).
    m.catat(unsafe { klatos_antrean_tambah(pq, 100, 1) } == 0);
    m.catat(unsafe { klatos_antrean_hapus_pada(pq, 0) } == 0);
    m.catat(unsafe { klatos_antrean_tambah(pq, 101, 2) } == 0);
    m.catat(unsafe { ambil(pq, 0) } == Some((101, 2)));
    m.catat(unsafe { klatos_antrean_bersihkan(pq) } == 1);

    // G) Isolasi antar-antrean.
    let mut q2 = AntreanKeadaan::kosong();
    let pq2 = &mut q2 as *mut AntreanKeadaan;
    m.catat(unsafe { klatos_antrean_buat(pq2) } == 0);
    m.catat(unsafe { klatos_antrean_tambah(pq, 1, 1) } == 0);
    m.catat(unsafe { klatos_antrean_tambah(pq2, 2, 2) } == 0);
    m.catat(unsafe { klatos_antrean_jumlah(pq) } == 1);
    m.catat(unsafe { klatos_antrean_jumlah(pq2) } == 1);
    m.catat(unsafe { ambil(pq, 0) } == Some((1, 1)));
    m.catat(unsafe { ambil(pq2, 0) } == Some((2, 2)));
    m.catat(unsafe { klatos_antrean_bersihkan(pq) } == 1);
    m.catat(unsafe { klatos_antrean_bersihkan(pq2) } == 1);

    // H) Bendera + imminent (keadaan futex/antrean-usang).
    m.catat(unsafe { klatos_antrean_imminent_ambil(pq) } == 0);
    m.catat(unsafe { klatos_antrean_imminent_set(pq, 1) } == 0);
    m.catat(unsafe { klatos_antrean_imminent_ambil(pq) } == 1);
    m.catat(unsafe { klatos_antrean_bendera_set(pq, 1) } == 0);
    m.catat(unsafe { klatos_antrean_bendera_ambil(pq) } == 1);
    m.catat(unsafe { klatos_antrean_bendera_set(pq, 0) } == 1);
    m.catat(unsafe { klatos_antrean_bendera_ambil(pq) } == 0);

    // I) Batas indeks.
    m.neg(unsafe { klatos_antrean_hapus_pada(pq, 0) } == -E_INVAL);
    m.neg(unsafe { ambil(pq, 0) }.is_none());
    m.neg(unsafe { klatos_antrean_tambah(core::ptr::null_mut(), 0, 0) } == -E_INVAL);
    m.neg(unsafe { klatos_antrean_buat(core::ptr::null_mut()) } == -E_INVAL);

    // J) Lepas membebaskan simpul.
    m.catat(unsafe { klatos_antrean_tambah(pq, 5, 5) } == 0);
    m.catat(unsafe { klatos_antrean_tambah(pq, 6, 6) } == 0);
    m.catat(unsafe { klatos_antrean_lepas(pq) } == 2);
    m.catat(unsafe { klatos_antrean_jumlah(pq) } == 0);

    // K) Skala besar (berbatas, aman untuk arena bersama): 1000 anggota.
    let mut qb = AntreanKeadaan::kosong();
    let pqb = &mut qb as *mut AntreanKeadaan;
    m.catat(unsafe { klatos_antrean_buat(pqb) } == 0);
    let mut i = 0usize;
    let mut gagal = false;
    while i < 1000 {
        if unsafe { klatos_antrean_tambah(pqb, i as u64, i as u64) } != 0 {
            gagal = true;
            break;
        }
        i += 1;
    }
    m.catat(!gagal);
    m.catat(unsafe { klatos_antrean_jumlah(pqb) } == 1000);
    m.catat(unsafe { ambil(pqb, 0) } == Some((0, 0)));
    m.catat(unsafe { ambil(pqb, 999) } == Some((999, 999)));
    m.catat(unsafe { klatos_antrean_bersihkan(pqb) } == 1000);
    m.catat(unsafe { klatos_antrean_kosong(pqb) } == 1);

    // L) Diferensial vs model murni (LCG deterministik).
    let mut qd = AntreanKeadaan::kosong();
    let pqd = &mut qd as *mut AntreanKeadaan;
    m.catat(unsafe { klatos_antrean_buat(pqd) } == 0);
    let mut mdl = Model::baru();
    let mut seed: u64 = 0x2026_1005_0000_0001;
    let mut step = 0u32;
    while step < 20000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let op = (seed >> 60) & 3;
        let tok = (seed >> 3) & 0x0F;
        match op {
            0 => {
                let d = seed >> 8;
                let rg = unsafe { klatos_antrean_tambah(pqd, tok, d) } == 0;
                let rm = mdl.tambah(tok, d);
                m.dif(rg == rm);
            }
            1 => {
                let rg = unsafe { klatos_antrean_hapus_token(pqd, tok) };
                let rm = mdl.hapus_token(tok);
                m.dif(rg == rm);
            }
            2 => {
                let i = ((seed >> 20) % 8) as u32;
                let rg = unsafe { klatos_antrean_hapus_pada(pqd, i) } == 0;
                let rm = mdl.hapus_pada(i as usize);
                m.dif(rg == rm);
            }
            _ => {
                let i = ((seed >> 24) % 8) as u32;
                let rg = unsafe { ambil(pqd, i) };
                let rm = mdl.ambil(i as usize);
                m.dif(rg == rm);
            }
        }
        m.dif(unsafe { klatos_antrean_jumlah(pqd) } as usize == mdl.n);
        step += 1;
    }
    m.catat(unsafe { klatos_antrean_bersihkan(pqd) } == mdl.n as u32);
    m.catat(unsafe { klatos_antrean_kosong(pqd) } == 1);

    // Penghitung langgaran (WAJIB 0).
    let langgar = COUNTERS[Counter::LanggarUrutan as usize].load(Ordering::Relaxed)
        + COUNTERS[Counter::LanggarBocor as usize].load(Ordering::Relaxed)
        + COUNTERS[Counter::LanggarArena as usize].load(Ordering::Relaxed);
    m.catat(langgar == 0);

    let prod_buat = COUNTERS[Counter::ProdBuat as usize].load(Ordering::Relaxed);
    let prod_tambah = COUNTERS[Counter::ProdTambah as usize].load(Ordering::Relaxed);
    let prod_hapus_token = COUNTERS[Counter::ProdHapusToken as usize].load(Ordering::Relaxed);
    let prod_hapus_pada = COUNTERS[Counter::ProdHapusPada as usize].load(Ordering::Relaxed);
    let prod_bersih = COUNTERS[Counter::ProdBersih as usize].load(Ordering::Relaxed);
    let tolak_penuh = COUNTERS[Counter::TolakPenuh as usize].load(Ordering::Relaxed);

    let _ = klatos_antrean_reset_counter();

    let ok = m.checks == m.passed
        && m.neg_checks == m.neg_passed
        && m.dif_beda == 0
        && m.checks > 0
        && langgar == 0;
    KlatosAntreanTungguReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_buat,
        prod_tambah,
        prod_hapus_token,
        prod_hapus_pada,
        prod_bersih,
        tolak_penuh,
        langgar,
        status: if ok { 0 } else { -1 },
    }
}

/// Selftest otoritas antrean.
#[no_mangle]
pub extern "C" fn klatos_antrean_selftest() -> KlatosAntreanTungguReport {
    // Konformansi tata letak laporan.
    assert!(core::mem::size_of::<KlatosAntreanTungguReport>() == 64);
    assert!(core::mem::offset_of!(KlatosAntreanTungguReport, status) == 60);
    assert!(REPORT_TAG == 0x4B4C_4154_414E_5452);
    run_matrix()
}
