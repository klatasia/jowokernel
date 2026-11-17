// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! KERNEL-RUST KR-FS — **OTORITAS OPSI BERKAS-SISTEM (generik)**.
//!
//!
//! Tabel opsi hanya disentuh dari konteks proses (ioctl mount, create FS),
//! tidak pernah dari IRQ. Karena itu kunci spin biasa sudah cukup dan benar.

#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering, AtomicU64};

/// Tag laporan: "KLATOPSI".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4F50_5349;

/// Panjang maksimum kunci; cermin `MOUNT_SPECIFIC_FLAG_KEY_STRING_MAX_LENGTH`.
pub const KEY_MAX: usize = 64;
/// Jumlah tabel (MountFile) serentak.
pub const N_TABEL: usize = 32;
/// Jumlah entri per tabel. Tabel asli C++ adalah HashMap tak-berbatas; arena ini
/// berbatas demi tanpa-alokator. Batas dipilih longgar (64 opsi/mount) dan
/// `set` mengembalikan `-ENOSPC` bila terlampaui, yang diteruskan adapter C++
/// sebagai galat (bukan panic), sama seperti `try_set` asli yang bisa gagal OOM.
pub const N_ENTRY: usize = 64;

/// Tag nilai: `unsigned` (32-bit).
pub const TAG_UNSIGNED: u8 = 0;
/// Tag nilai: `signed` (32-bit).
pub const TAG_SIGNED: u8 = 1;
/// Tag nilai: `bool`.
pub const TAG_BOOL: u8 = 2;

/// `-EBADF` (handle tak sah).
pub const E_BADF: i32 = 9;
/// `-EINVAL`.
pub const E_INVAL: i32 = 22;
/// `-ENOSPC` (tabel penuh).
pub const E_NOSPC: i32 = 28;

// --- state ------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Entry {
    live: bool,
    key_len: usize,
    key: [u8; KEY_MAX],
    tag: u8,
    val: i64,
}

#[derive(Clone, Copy)]
struct Tabel {
    live: bool,
    e: [Entry; N_ENTRY],
}

const fn entry_kosong() -> Entry {
    Entry { live: false, key_len: 0, key: [0u8; KEY_MAX], tag: 0, val: 0 }
}

const fn tabel_kosong() -> Tabel {
    Tabel { live: false, e: [entry_kosong(); N_ENTRY] }
}

/// Penghitung audit.
#[derive(Clone, Copy)]
pub enum Counter {
    ProdBuat = 0,
    ProdLepas = 1,
    ProdSet = 2,
    ProdHapus = 3,
    ProdAda = 4,
    TolakTipe = 5,
    TolakPenuh = 6,
}
const N_COUNTER: usize = 7;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// --- model murni (diuji di host tanpa state global) -------------------------

impl Tabel {
    const fn baru() -> Self {
        tabel_kosong()
    }

    fn cari(&self, key: &[u8]) -> Option<usize> {
        let mut i = 0usize;
        while i < N_ENTRY {
            let e = &self.e[i];
            if e.live && e.key_len == key.len() && e.key[..key.len()] == *key {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    /// Sisip atau ganti. `0` = entri baru, `1` = menggantikan, negatif = galat.
    fn set(&mut self, key: &[u8], tag: u8, val: i64) -> i32 {
        if tag > TAG_BOOL || key.is_empty() || key.len() > KEY_MAX {
            return -E_INVAL;
        }
        if let Some(i) = self.cari(key) {
            self.e[i].tag = tag;
            self.e[i].val = val;
            return 1;
        }
        let mut i = 0usize;
        while i < N_ENTRY {
            if !self.e[i].live {
                let e = &mut self.e[i];
                e.live = true;
                e.key_len = key.len();
                let mut k = 0usize;
                while k < key.len() {
                    e.key[k] = key[k];
                    k += 1;
                }
                e.tag = tag;
                e.val = val;
                return 0;
            }
            i += 1;
        }
        -E_NOSPC
    }

    /// `1` dihapus, `0` tidak ada, negatif = galat.
    fn hapus(&mut self, key: &[u8]) -> i32 {
        if key.is_empty() || key.len() > KEY_MAX {
            return -E_INVAL;
        }
        match self.cari(key) {
            Some(i) => {
                self.e[i] = entry_kosong();
                1
            }
            None => 0,
        }
    }

    /// `1` ada, `0` tidak ada, negatif = galat.
    fn ada(&self, key: &[u8]) -> i32 {
        if key.is_empty() || key.len() > KEY_MAX {
            return -E_INVAL;
        }
        if self.cari(key).is_some() {
            1
        } else {
            0
        }
    }

    /// `1` ditemukan, `0` tidak ada, negatif = galat. Nilai tag/val via out.
    fn baca(&self, key: &[u8], out_tag: &mut u8, out_val: &mut i64) -> i32 {
        if key.is_empty() || key.len() > KEY_MAX {
            return -E_INVAL;
        }
        match self.cari(key) {
            Some(i) => {
                *out_tag = self.e[i].tag;
                *out_val = self.e[i].val;
                1
            }
            None => 0,
        }
    }

    fn hitung(&self) -> i32 {
        let mut n = 0i32;
        let mut i = 0usize;
        while i < N_ENTRY {
            if self.e[i].live {
                n += 1;
            }
            i += 1;
        }
        n
    }
}

// --- state global -----------------------------------------------------------

static OPSI_LOCK: AtomicU32 = AtomicU32::new(0);
/// Status interupsi pemegang OPSI_LOCK; hanya pemegang yang menulis.
static OPSI_LOCK_FLAGS: AtomicU64 = AtomicU64::new(0);
static mut TABEL: [Tabel; N_TABEL] = [tabel_kosong(); N_TABEL];

fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while OPSI_LOCK.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    OPSI_LOCK_FLAGS.store(flags, Ordering::Relaxed);
}
fn unlock() {
    let flags = OPSI_LOCK_FLAGS.load(Ordering::Relaxed);
    OPSI_LOCK.store(0, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Indeks tabel dari handle (handle = indeks + 1). `None` bila tak sah/tak hidup.
fn slot(h: u32) -> Option<usize> {
    if h == 0 {
        return None;
    }
    let i = (h - 1) as usize;
    if i >= N_TABEL {
        return None;
    }
    Some(i)
}

// --- pintu C ABI ------------------------------------------------------------

/// Buat tabel baru; mengembalikan handle (>=1) atau `0` bila penuh.
#[no_mangle]
pub extern "C" fn klatos_opsi_buat() -> u32 {
    lock();
    let mut i = 0usize;
    while i < N_TABEL {
        let t = unsafe { &mut *core::ptr::addr_of_mut!(TABEL[i]) };
        if !t.live {
            *t = Tabel::baru();
            t.live = true;
            unlock();
            bump(Counter::ProdBuat);
            return (i + 1) as u32;
        }
        i += 1;
    }
    unlock();
    0
}

/// Lepas tabel. `0` ok, `-EBADF` bila handle tak sah.
#[no_mangle]
pub extern "C" fn klatos_opsi_lepas(h: u32) -> i32 {
    lock();
    let i = match slot(h) {
        Some(i) => i,
        None => {
            unlock();
            return -E_BADF;
        }
    };
    let t = unsafe { &mut *core::ptr::addr_of_mut!(TABEL[i]) };
    if !t.live {
        unlock();
        return -E_BADF;
    }
    *t = tabel_kosong();
    unlock();
    bump(Counter::ProdLepas);
    0
}

/// Sisip/ganti nilai. `0` baru, `1` ganti, negatif galat.
///
/// # Safety
/// `key` harus menunjuk `key_len` byte yang dapat dibaca.
#[no_mangle]
pub unsafe extern "C" fn klatos_opsi_set(h: u32, key: *const u8, key_len: u32, tag: u8, val: i64) -> i32 {
    if key.is_null() {
        return -E_INVAL;
    }
    if key_len as usize > KEY_MAX || key_len == 0 {
        return -E_INVAL;
    }
    let k = unsafe { core::slice::from_raw_parts(key, key_len as usize) };
    lock();
    let i = match slot(h) {
        Some(i) => i,
        None => {
            unlock();
            return -E_BADF;
        }
    };
    let t = unsafe { &mut *core::ptr::addr_of_mut!(TABEL[i]) };
    if !t.live {
        unlock();
        return -E_BADF;
    }
    let r = t.set(k, tag, val);
    unlock();
    if r == -E_INVAL {
        bump(Counter::TolakTipe);
    } else if r == -E_NOSPC {
        bump(Counter::TolakPenuh);
    } else {
        bump(Counter::ProdSet);
    }
    r
}

/// Hapus kunci. `1` dihapus, `0` tidak ada, negatif galat.
///
/// # Safety
/// `key` harus menunjuk `key_len` byte yang dapat dibaca.
#[no_mangle]
pub unsafe extern "C" fn klatos_opsi_hapus(h: u32, key: *const u8, key_len: u32) -> i32 {
    if key.is_null() || key_len == 0 || key_len as usize > KEY_MAX {
        return -E_INVAL;
    }
    let k = unsafe { core::slice::from_raw_parts(key, key_len as usize) };
    lock();
    let i = match slot(h) {
        Some(i) => i,
        None => {
            unlock();
            return -E_BADF;
        }
    };
    let t = unsafe { &mut *core::ptr::addr_of_mut!(TABEL[i]) };
    if !t.live {
        unlock();
        return -E_BADF;
    }
    let r = t.hapus(k);
    unlock();
    if r == 1 {
        bump(Counter::ProdHapus);
    }
    r
}

/// Cek kunci. `1` ada, `0` tidak ada, negatif galat.
///
/// # Safety
/// `key` harus menunjuk `key_len` byte yang dapat dibaca.
#[no_mangle]
pub unsafe extern "C" fn klatos_opsi_ada(h: u32, key: *const u8, key_len: u32) -> i32 {
    if key.is_null() || key_len == 0 || key_len as usize > KEY_MAX {
        return -E_INVAL;
    }
    let k = unsafe { core::slice::from_raw_parts(key, key_len as usize) };
    lock();
    let i = match slot(h) {
        Some(i) => i,
        None => {
            unlock();
            return -E_BADF;
        }
    };
    let t = unsafe { &*core::ptr::addr_of!(TABEL[i]) };
    if !t.live {
        unlock();
        return -E_BADF;
    }
    let r = t.ada(k);
    unlock();
    if r == 1 {
        bump(Counter::ProdAda);
    }
    r
}

/// Baca nilai bertag. `1` ditemukan, `0` tidak ada, negatif galat.
///
/// # Safety
/// `key` harus menunjuk `key_len` byte; `out_tag`/`out_val` harus dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_opsi_baca(
    h: u32,
    key: *const u8,
    key_len: u32,
    out_tag: *mut u8,
    out_val: *mut i64,
) -> i32 {
    if key.is_null() || out_tag.is_null() || out_val.is_null() {
        return -E_INVAL;
    }
    if key_len == 0 || key_len as usize > KEY_MAX {
        return -E_INVAL;
    }
    let k = unsafe { core::slice::from_raw_parts(key, key_len as usize) };
    lock();
    let i = match slot(h) {
        Some(i) => i,
        None => {
            unlock();
            return -E_BADF;
        }
    };
    let t = unsafe { &*core::ptr::addr_of!(TABEL[i]) };
    if !t.live {
        unlock();
        return -E_BADF;
    }
    let mut tag = 0u8;
    let mut val = 0i64;
    let r = t.baca(k, &mut tag, &mut val);
    unlock();
    if r == 1 {
        unsafe {
            core::ptr::write(out_tag, tag);
            core::ptr::write(out_val, val);
        }
        bump(Counter::ProdAda);
    }
    r
}

/// Jumlah entri hidup pada tabel; negatif bila handle tak sah.
#[no_mangle]
pub extern "C" fn klatos_opsi_hitung(h: u32) -> i32 {
    lock();
    let i = match slot(h) {
        Some(i) => i,
        None => {
            unlock();
            return -E_BADF;
        }
    };
    let t = unsafe { &*core::ptr::addr_of!(TABEL[i]) };
    let r = if t.live { t.hitung() } else { -E_BADF };
    unlock();
    r
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_opsi_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset seluruh tabel + penghitung (uji).
#[no_mangle]
pub extern "C" fn klatos_opsi_reset() -> u32 {
    lock();
    let mut i = 0usize;
    while i < N_TABEL {
        let t = unsafe { &mut *core::ptr::addr_of_mut!(TABEL[i]) };
        *t = tabel_kosong();
        i += 1;
    }
    unlock();
    let mut c = 0usize;
    while c < N_COUNTER {
        COUNTERS[c].store(0, Ordering::Relaxed);
        c += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas opsi berkas (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosOpsiReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_buat: u32,
    pub prod_lepas: u32,
    pub prod_set: u32,
    pub prod_hapus: u32,
    pub prod_ada: u32,
    pub tolak_tipe: u32,
    pub tolak_penuh: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosOpsiReport>()];

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

fn baca(h: u32, key: &[u8]) -> (i32, u8, i64) {
    let mut tag = 0u8;
    let mut val = 0i64;
    let r = unsafe { klatos_opsi_baca(h, key.as_ptr(), key.len() as u32, &mut tag, &mut val) };
    (r, tag, val)
}

fn run_matrix() -> KlatosOpsiReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_4F50_5349);
    m.catat(KEY_MAX == 64);
    m.catat(N_TABEL == 32 && N_ENTRY == 64);
    m.catat(TAG_UNSIGNED == 0 && TAG_SIGNED == 1 && TAG_BOOL == 2);
    m.catat(E_BADF == 9 && E_INVAL == 22 && E_NOSPC == 28);

    // A) Himpunan kosong.
    let _ = klatos_opsi_reset();
    let h = klatos_opsi_buat();
    m.catat(h != 0);
    m.catat(klatos_opsi_hitung(h) == 0);
    m.catat(baca(h, b"fd").0 == 0);

    // B) Satu opsi + baca bertag.
    m.catat(unsafe { klatos_opsi_set(h, b"rootmode".as_ptr(), 8, TAG_UNSIGNED, 0o40755) } == 0);
    let (r, tag, val) = baca(h, b"rootmode");
    m.catat(r == 1 && tag == TAG_UNSIGNED && val == 0o40755);
    m.catat(unsafe { klatos_opsi_ada(h, b"rootmode".as_ptr(), 8) } == 1);

    // C) Banyak opsi + tipe berbeda.
    m.catat(unsafe { klatos_opsi_set(h, b"uid".as_ptr(), 3, TAG_UNSIGNED, 1000) } == 0);
    m.catat(unsafe { klatos_opsi_set(h, b"gid".as_ptr(), 3, TAG_UNSIGNED, 1000) } == 0);
    m.catat(unsafe { klatos_opsi_set(h, b"n".as_ptr(), 1, TAG_SIGNED, -7) } == 0);
    m.catat(unsafe { klatos_opsi_set(h, b"ro".as_ptr(), 2, TAG_BOOL, 1) } == 0);
    m.catat(klatos_opsi_hitung(h) == 5);
    let (rn, tn, vn) = baca(h, b"n");
    m.catat(rn == 1 && tn == TAG_SIGNED && vn == -7);
    let (rb, tb, vb) = baca(h, b"ro");
    m.catat(rb == 1 && tb == TAG_BOOL && vb == 1);

    // D) Ganti (update) nilai: kunci sama -> 1.
    m.catat(unsafe { klatos_opsi_set(h, b"uid".as_ptr(), 3, TAG_UNSIGNED, 42) } == 1);
    let (_, _, vu) = baca(h, b"uid");
    m.catat(vu == 42);
    m.catat(klatos_opsi_hitung(h) == 5);

    // E) Kunci hilang.
    m.catat(baca(h, b"tidak-ada").0 == 0);
    m.catat(unsafe { klatos_opsi_ada(h, b"tidak-ada".as_ptr(), 9) } == 0);

    // F) Hapus: ada -> 1, ulang -> 0.
    m.catat(unsafe { klatos_opsi_hapus(h, b"gid".as_ptr(), 3) } == 1);
    m.catat(klatos_opsi_hitung(h) == 4);
    m.neg(unsafe { klatos_opsi_hapus(h, b"gid".as_ptr(), 3) } == 0);
    m.catat(unsafe { klatos_opsi_ada(h, b"gid".as_ptr(), 3) } == 0);

    // G) Batas panjang kunci (64 ok, 65 tolak, 0 tolak).
    let k64 = [b'a'; KEY_MAX];
    m.catat(unsafe { klatos_opsi_set(h, k64.as_ptr(), KEY_MAX as u32, TAG_BOOL, 0) } == 0);
    let k65 = [b'b'; KEY_MAX + 1];
    m.neg(unsafe { klatos_opsi_set(h, k65.as_ptr(), (KEY_MAX + 1) as u32, TAG_BOOL, 0) } == -E_INVAL);
    m.neg(unsafe { klatos_opsi_set(h, b"x".as_ptr(), 0, TAG_BOOL, 0) } == -E_INVAL);

    // H) Tag tak sah ditolak (fail-closed).
    m.neg(unsafe { klatos_opsi_set(h, b"z".as_ptr(), 1, 7, 0) } == -E_INVAL);

    // I) Handle tak sah / tabel tak hidup.
    m.neg(baca(0, b"fd").0 == -E_BADF);
    m.neg(unsafe { klatos_opsi_set(0, b"fd".as_ptr(), 2, TAG_UNSIGNED, 0) } == -E_BADF);
    m.neg(klatos_opsi_hitung(9999) == -E_BADF);
    m.neg(klatos_opsi_lepas(9999) == -E_BADF);

    // J) Pointer null ditolak.
    m.neg(unsafe { klatos_opsi_set(h, core::ptr::null(), 2, TAG_UNSIGNED, 0) } == -E_INVAL);

    // K) Tabel penuh -> -ENOSPC.
    let _ = klatos_opsi_reset();
    let hf = klatos_opsi_buat();
    let mut i = 0usize;
    while i < N_ENTRY {
        let kb = [(b'A' as u8) + (i as u8)];
        let _ = unsafe { klatos_opsi_set(hf, kb.as_ptr(), 1, TAG_UNSIGNED, i as i64) };
        i += 1;
    }
    m.catat(klatos_opsi_hitung(hf) == N_ENTRY as i32);
    m.neg(unsafe { klatos_opsi_set(hf, b"penuh".as_ptr(), 5, TAG_UNSIGNED, 0) } == -E_NOSPC);

    // L) Isolasi antar-tabel (dua handle berbeda tidak saling melihat).
    let _ = klatos_opsi_reset();
    let ha = klatos_opsi_buat();
    let hb = klatos_opsi_buat();
    m.catat(ha != 0 && hb != 0 && ha != hb);
    m.catat(unsafe { klatos_opsi_set(ha, b"k".as_ptr(), 1, TAG_UNSIGNED, 1) } == 0);
    m.catat(baca(ha, b"k").0 == 1);
    m.neg(baca(hb, b"k").0 == 0);

    // M) Lepas membebaskan slot; handle lama menjadi tak sah (anti-ABA).
    m.catat(klatos_opsi_lepas(ha) == 0);
    m.neg(baca(ha, b"k").0 == -E_BADF);
    let hc = klatos_opsi_buat();
    m.catat(hc == ha); // slot dipakai ulang, tetapi tabelnya kosong
    m.neg(baca(hc, b"k").0 == 0);

    // N) Dif: model lokal vs state global untuk urutan operasi identik.
    let mut mdl = Tabel::baru();
    let _ = klatos_opsi_reset();
    let hg = klatos_opsi_buat();
    let ops: [(&[u8], u8, i64); 6] = [
        (b"a", TAG_UNSIGNED, 1),
        (b"b", TAG_SIGNED, -2),
        (b"c", TAG_BOOL, 1),
        (b"a", TAG_UNSIGNED, 9),
        (b"d", TAG_SIGNED, 4),
        (b"b", TAG_BOOL, 0),
    ];
    for (k, t, v) in ops.iter() {
        let rg = unsafe { klatos_opsi_set(hg, k.as_ptr(), k.len() as u32, *t, *v) };
        let rm = mdl.set(k, *t, *v);
        m.dif(rg == rm);
    }
    m.dif(mdl.hitung() == klatos_opsi_hitung(hg));
    for k in [b"a".as_slice(), b"b".as_slice(), b"c".as_slice(), b"d".as_slice(), b"e".as_slice()] {
        let (rg, tg, vg) = baca(hg, k);
        let mut tm = 0u8;
        let mut vm = 0i64;
        let rm = mdl.baca(k, &mut tm, &mut vm);
        m.dif(rg == rm && tg == tm && vg == vm);
    }

    // Rekam penghitung SEBELUM reset (reset menghapusnya).
    let c_buat = klatos_opsi_counter(Counter::ProdBuat as u32);
    let c_lepas = klatos_opsi_counter(Counter::ProdLepas as u32);
    let c_set = klatos_opsi_counter(Counter::ProdSet as u32);
    let c_hapus = klatos_opsi_counter(Counter::ProdHapus as u32);
    let c_ada = klatos_opsi_counter(Counter::ProdAda as u32);
    let c_tipe = klatos_opsi_counter(Counter::TolakTipe as u32);
    let c_penuh = klatos_opsi_counter(Counter::TolakPenuh as u32);
    let _ = klatos_opsi_reset();

    let status = if m.checks == m.passed && m.neg_passed == m.neg_checks && m.dif_beda == 0 {
        0
    } else {
        -1
    };
    KlatosOpsiReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_buat: c_buat,
        prod_lepas: c_lepas,
        prod_set: c_set,
        prod_hapus: c_hapus,
        prod_ada: c_ada,
        tolak_tipe: c_tipe,
        tolak_penuh: c_penuh,
        status,
    }
}

/// Matriks otoritas opsi berkas-sistem.
#[no_mangle]
pub extern "C" fn klatos_opsi_selftest() -> KlatosOpsiReport {
    run_matrix()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4F50_5349);
        assert!(KEY_MAX == 64);
        assert!(N_TABEL == 32);
        assert!(N_ENTRY == 64);
        assert!(core::mem::size_of::<KlatosOpsiReport>() == 64);
        assert!(core::mem::offset_of!(KlatosOpsiReport, status) == 60);
    };
}
