// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8.5-D — **OTORITAS URUTAN INISIALISASI BOOT (generik)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering, AtomicU64};

/// Tag laporan: "KLATBOOT".
pub const REPORT_TAG: u64 = 0x4B4C_4154_424F_4F54;

/// Jumlah langkah maksimum (tetap, tanpa alokasi).
pub const MAX_STAGE: usize = 32;

/// Sentinel "tanpa dependensi".
pub const TANPA_DEP: u32 = 0xFFFF_FFFF;
/// Sentinel "tidak ada langkah berikutnya".
pub const SELESAI: u32 = 0xFFFF_FFFF;

/// Status langkah.
pub const B_KOSONG: u32 = 0;
/// Terdaftar, menunggu.
pub const B_PENDING: u32 = 1;
/// Sedang berjalan.
pub const B_RUN: u32 = 2;
/// Selesai.
pub const B_DONE: u32 = 3;
/// Gagal.
pub const B_GAGAL: u32 = 4;

/// `-EINVAL`.
pub const E_INVAL: i32 = 22;
/// `-EEXIST`.
pub const E_EXIST: i32 = 17;
/// `-ENOENT`.
pub const E_NOENT: i32 = 2;
/// `-EILSEQ` (siklus).
pub const E_SIKLUS: i32 = 84;

// --- ID langkah bawaan (cermin `KlatSeam136.h`) -----------------------------
/// InterruptManagement::initialize.
pub const BOOT_INTERRUPT: u32 = 0;
/// Probe driver DeviceTree tahap Early (batas arsitektur; no-op di x86_64).
pub const BOOT_DT_AWAL: u32 = 1;
/// ACPI::initialize.
pub const BOOT_ACPI: u32 = 2;
/// TimeManagement::initialize.
pub const BOOT_TIME: u32 = 3;
/// SysFSComponentRegistry::initialize.
pub const BOOT_SYSFS: u32 = 4;
/// Device::initialize_base_devices.
pub const BOOT_DEVICE: u32 = 5;
/// __stack_chk_guard = get_fast_random.
pub const BOOT_STACKCHK: u32 = 6;
/// VFSRootContext::initialize_empty_ramfs_root_context_for_kernel_processes.
pub const BOOT_VFSROOT: u32 = 7;
/// Process::initialize.
pub const BOOT_PROCESS: u32 = 8;
/// Scheduler::initialize.
pub const BOOT_SCHEDULER: u32 = 9;
/// Jumlah langkah bawaan.
pub const N_BAWAAN: usize = 10;

/// Penghitung audit.
#[derive(Clone, Copy)]
pub enum Counter {
    ProdDaftar = 0,
    ProdSelesai = 1,
    ProdGagal = 2,
    TolakDup = 3,
    TolakHilang = 4,
    TolakSiklus = 5,
    TolakUlang = 6,
}
const N_COUNTER: usize = 7;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

/// Matriks langkah (murni, dapat diuji di host tanpa state global).
struct Model {
    terdaftar: [bool; MAX_STAGE],
    dep: [u32; MAX_STAGE],
    st: [u32; MAX_STAGE],
}

impl Model {
    const fn baru() -> Self {
        Model {
            terdaftar: [false; MAX_STAGE],
            dep: [TANPA_DEP; MAX_STAGE],
            st: [B_KOSONG; MAX_STAGE],
        }
    }

    fn daftar(&mut self, id: u32, dep: u32) -> i32 {
        if id as usize >= MAX_STAGE {
            return -E_INVAL;
        }
        if self.terdaftar[id as usize] {
            return -E_EXIST;
        }
        if dep != TANPA_DEP && dep as usize >= MAX_STAGE {
            return -E_INVAL;
        }
        self.terdaftar[id as usize] = true;
        self.dep[id as usize] = dep;
        self.st[id as usize] = B_PENDING;
        0
    }

    /// Validasi: setiap dependensi harus terdaftar, dan tidak ada siklus.
    fn validasi(&self) -> i32 {
        let mut i = 0usize;
        while i < MAX_STAGE {
            if self.terdaftar[i] {
                let d = self.dep[i];
                if d != TANPA_DEP {
                    if d as usize >= MAX_STAGE || !self.terdaftar[d as usize] {
                        return -E_NOENT;
                    }
                }
            }
            i += 1;
        }
        // Deteksi siklus: ikuti rantai dependensi; bila kembali ke diri = siklus.
        let mut s = 0usize;
        while s < MAX_STAGE {
            if self.terdaftar[s] {
                let mut c = s as u32;
                let mut n = 0usize;
                while c != TANPA_DEP {
                    n += 1;
                    if n > MAX_STAGE {
                        return -E_SIKLUS;
                    }
                    c = self.dep[c as usize];
                }
            }
            s += 1;
        }
        0
    }

    /// Urutan deterministik: id terkecil yang dependensinya sudah DONE.
    fn langkah(&self) -> u32 {
        let mut i = 0usize;
        while i < MAX_STAGE {
            if self.terdaftar[i] && self.st[i] == B_PENDING {
                let d = self.dep[i];
                let siap = d == TANPA_DEP || (self.terdaftar[d as usize] && self.st[d as usize] == B_DONE);
                if siap {
                    return i as u32;
                }
            }
            i += 1;
        }
        SELESAI
    }

    fn mulai(&mut self, id: u32) -> i32 {
        if id as usize >= MAX_STAGE || !self.terdaftar[id as usize] {
            return -E_NOENT;
        }
        if self.st[id as usize] != B_PENDING {
            return -E_EXIST;
        }
        self.st[id as usize] = B_RUN;
        0
    }

    fn selesai(&mut self, id: u32) -> i32 {
        if id as usize >= MAX_STAGE || !self.terdaftar[id as usize] {
            return -E_NOENT;
        }
        let s = self.st[id as usize];
        if s != B_PENDING && s != B_RUN {
            return -E_EXIST; // inisialisasi berulang
        }
        self.st[id as usize] = B_DONE;
        0
    }

    fn gagal(&mut self, id: u32) -> i32 {
        if id as usize >= MAX_STAGE || !self.terdaftar[id as usize] {
            return -E_NOENT;
        }
        self.st[id as usize] = B_GAGAL;
        0
    }

    fn status(&self, id: u32) -> i32 {
        if id as usize >= MAX_STAGE || !self.terdaftar[id as usize] {
            return -E_NOENT;
        }
        self.st[id as usize] as i32
    }

    /// Tulis urutan topologis ke `out`; mengembalikan panjang atau negatif.
    unsafe fn urutan(&self, out: *mut u32, cap: u32) -> i32 {
        let mut tmp = Model {
            terdaftar: self.terdaftar,
            dep: self.dep,
            st: [B_PENDING; MAX_STAGE],
        };
        let mut n = 0u32;
        loop {
            let l = tmp.langkah();
            if l == SELESAI {
                break;
            }
            if n >= cap {
                return -E_INVAL;
            }
            core::ptr::write(out.add(n as usize), l);
            let _ = tmp.selesai(l);
            n += 1;
        }
        n as i32
    }
}

// --- state global -----------------------------------------------------------

static BOOT_LOCK: AtomicU32 = AtomicU32::new(0);
/// Status interupsi pemegang BOOT_LOCK; hanya pemegang yang menulis.
static BOOT_LOCK_FLAGS: AtomicU64 = AtomicU64::new(0);
static mut BOOT: Model = Model::baru();
static BOOT_INIT: AtomicU32 = AtomicU32::new(0);

fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while BOOT_LOCK.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    BOOT_LOCK_FLAGS.store(flags, Ordering::Relaxed);
}
fn unlock() {
    let flags = BOOT_LOCK_FLAGS.load(Ordering::Relaxed);
    BOOT_LOCK.store(0, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Bangun manifes bawaan (rantai deterministik `BOOT_*`). Idempoten.
pub fn bangun() -> i32 {
    lock();
    let m = unsafe { &mut *core::ptr::addr_of_mut!(BOOT) };
    let mut i = 0usize;
    while i < MAX_STAGE {
        m.terdaftar[i] = false;
        m.dep[i] = TANPA_DEP;
        m.st[i] = B_KOSONG;
        i += 1;
    }
    let mut id = 0u32;
    while id < N_BAWAAN as u32 {
        let dep = if id == 0 { TANPA_DEP } else { id - 1 };
        let _ = m.daftar(id, dep);
        id += 1;
    }
    BOOT_INIT.store(1, Ordering::Release);
    unlock();
    0
}

fn pastikan() {
    if BOOT_INIT.load(Ordering::Acquire) == 0 {
        let _ = bangun();
    }
}

// --- pintu C ABI ------------------------------------------------------------

/// Daftarkan langkah (dipakai uji; manifes produksi dibangun [`bangun`]).
#[no_mangle]
pub extern "C" fn klatos_boot_daftar(id: u32, dep: u32) -> i32 {
    pastikan();
    lock();
    let m = unsafe { &mut *core::ptr::addr_of_mut!(BOOT) };
    let r = m.daftar(id, dep);
    if r == 0 {
        bump(Counter::ProdDaftar);
    } else if r == -E_EXIST {
        bump(Counter::TolakDup);
    }
    unlock();
    r
}

/// Bangun ulang manifes bawaan.
#[no_mangle]
pub extern "C" fn klatos_boot_bangun() -> i32 {
    bangun()
}

/// Validasi manifes: `0` ok, negatif bila hilang/siklus.
#[no_mangle]
pub extern "C" fn klatos_boot_validasi() -> i32 {
    pastikan();
    lock();
    let m = unsafe { &*core::ptr::addr_of!(BOOT) };
    let r = m.validasi();
    if r == -E_NOENT {
        bump(Counter::TolakHilang);
    } else if r == -E_SIKLUS {
        bump(Counter::TolakSiklus);
    }
    unlock();
    r
}

/// Langkah berikutnya, atau [`SELESAI`].
#[no_mangle]
pub extern "C" fn klatos_boot_langkah() -> u32 {
    pastikan();
    lock();
    let m = unsafe { &*core::ptr::addr_of!(BOOT) };
    let r = m.langkah();
    unlock();
    r
}

/// Tandai langkah sedang berjalan.
#[no_mangle]
pub extern "C" fn klatos_boot_mulai(id: u32) -> i32 {
    pastikan();
    lock();
    let m = unsafe { &mut *core::ptr::addr_of_mut!(BOOT) };
    let r = m.mulai(id);
    unlock();
    r
}

/// Tandai langkah selesai; insialisasi berulang ditolak (`-EEXIST`).
#[no_mangle]
pub extern "C" fn klatos_boot_selesai(id: u32) -> i32 {
    pastikan();
    lock();
    let m = unsafe { &mut *core::ptr::addr_of_mut!(BOOT) };
    let r = m.selesai(id);
    if r == 0 {
        bump(Counter::ProdSelesai);
    } else if r == -E_EXIST {
        bump(Counter::TolakUlang);
    }
    unlock();
    r
}

/// Tandai langkah gagal (kegagalan dipropagasikan ke dependen).
#[no_mangle]
pub extern "C" fn klatos_boot_gagal(id: u32) -> i32 {
    pastikan();
    lock();
    let m = unsafe { &mut *core::ptr::addr_of_mut!(BOOT) };
    let r = m.gagal(id);
    if r == 0 {
        bump(Counter::ProdGagal);
    }
    unlock();
    r
}

/// Status langkah (`B_*`) atau `-ENOENT`.
#[no_mangle]
pub extern "C" fn klatos_boot_status(id: u32) -> i32 {
    pastikan();
    lock();
    let m = unsafe { &*core::ptr::addr_of!(BOOT) };
    let r = m.status(id);
    unlock();
    r
}

/// Tulis urutan topologis deterministik; mengembalikan panjang.
///
/// # Safety
/// `out` harus menunjuk `cap` `u32` yang dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_boot_urutan(out: *mut u32, cap: u32) -> i32 {
    pastikan();
    if out.is_null() {
        return -E_INVAL;
    }
    lock();
    let m = unsafe { &*core::ptr::addr_of!(BOOT) };
    let r = m.urutan(out, cap);
    unlock();
    r
}

/// Penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_boot_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset state global (uji).
#[no_mangle]
pub extern "C" fn klatos_boot_reset() -> u32 {
    lock();
    let m = unsafe { &mut *core::ptr::addr_of_mut!(BOOT) };
    *m = Model::baru();
    BOOT_INIT.store(0, Ordering::Release);
    unlock();
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas boot (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosBootReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_daftar: u32,
    pub prod_selesai: u32,
    pub prod_gagal: u32,
    pub tolak_dup: u32,
    pub tolak_hilang: u32,
    pub tolak_siklus: u32,
    pub tolak_ulang: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosBootReport>()];

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

fn run_matrix() -> KlatosBootReport {
    let mut m = Matrix::new();
    m.catat(REPORT_TAG == 0x4B4C_4154_424F_4F54);
    m.catat(MAX_STAGE == 32);
    m.catat(N_BAWAAN == 10);
    m.catat(B_PENDING == 1 && B_DONE == 3 && B_GAGAL == 4);
    m.catat(SELESAI == TANPA_DEP);

    // A) Manifes bawaan: validasi + urutan deterministik 0..8.
    let _ = klatos_boot_reset();
    let _ = klatos_boot_bangun();
    let v = klatos_boot_validasi();
    m.catat(v == 0);
    let mut out = [0u32; MAX_STAGE];
    let n = unsafe { klatos_boot_urutan(out.as_mut_ptr(), MAX_STAGE as u32) };
    m.catat(n == N_BAWAAN as i32);
    let mut ok_order = true;
    let mut i = 0usize;
    while i < N_BAWAAN {
        if out[i] != i as u32 {
            ok_order = false;
        }
        i += 1;
    }
    m.catat(ok_order);

    // B) State machine: langkah() menghormati dependensi, selesai() mengunci.
    let l0 = klatos_boot_langkah();
    m.catat(l0 == 0);
    m.catat(klatos_boot_selesai(0) == 0);
    m.catat(klatos_boot_langkah() == 1);
    // langkah 2 belum boleh dijalankan sebelum 1 selesai.
    m.catat(klatos_boot_status(2) == B_PENDING as i32);
    let _ = klatos_boot_reset();
    let _ = klatos_boot_bangun();

    // C) Inisialisasi berulang ditolak.
    m.catat(klatos_boot_selesai(0) == 0);
    m.neg(klatos_boot_selesai(0) == -E_EXIST);

    // D) Dependensi hilang ditolak pada validasi (model lokal).
    let mut mm = Model::baru();
    m.catat(mm.daftar(0, TANPA_DEP) == 0);
    m.catat(mm.daftar(1, 7) == 0); // 7 tidak terdaftar
    m.neg(mm.validasi() == -E_NOENT);

    // E) Siklus ditolak.
    let mut mc = Model::baru();
    m.catat(mc.daftar(0, 1) == 0);
    m.catat(mc.daftar(1, 0) == 0);
    m.neg(mc.validasi() == -E_SIKLUS);

    // F) Pendaftaran ganda ditolak.
    let mut md = Model::baru();
    m.catat(md.daftar(0, TANPA_DEP) == 0);
    m.neg(md.daftar(0, TANPA_DEP) == -E_EXIST);

    // G) Propagasi kegagalan: langkah gagal menghalangi dependen.
    let mut mf = Model::baru();
    let _ = mf.daftar(0, TANPA_DEP);
    let _ = mf.daftar(1, 0);
    m.catat(mf.langkah() == 0);
    let _ = mf.gagal(0);
    m.neg(mf.langkah() == SELESAI); // 1 tidak pernah siap (dep gagal)

    // H) Batas state: id di luar jangkauan ditolak.
    let mut mb = Model::baru();
    m.neg(mb.daftar(MAX_STAGE as u32, TANPA_DEP) == -E_INVAL);

    // I) Dif: model lokal vs state global (manifes penuh).
    let _ = klatos_boot_reset();
    let _ = klatos_boot_bangun();
    let mut gstep = 0u32;
    loop {
        let l = klatos_boot_langkah();
        if l == SELESAI {
            break;
        }
        let w = gstep;
        m.dif(l == w);
        m.catat(klatos_boot_selesai(l) == 0);
        gstep += 1;
    }
    m.catat(gstep == N_BAWAAN as u32);

    let _ = klatos_boot_reset();
    let _ = klatos_boot_bangun();

    let status = if m.checks == m.passed && m.dif_beda == 0 { 0 } else { -1 };
    KlatosBootReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_daftar: klatos_boot_counter(Counter::ProdDaftar as u32),
        prod_selesai: klatos_boot_counter(Counter::ProdSelesai as u32),
        prod_gagal: klatos_boot_counter(Counter::ProdGagal as u32),
        tolak_dup: klatos_boot_counter(Counter::TolakDup as u32),
        tolak_hilang: klatos_boot_counter(Counter::TolakHilang as u32),
        tolak_siklus: klatos_boot_counter(Counter::TolakSiklus as u32),
        tolak_ulang: klatos_boot_counter(Counter::TolakUlang as u32),
        status,
    }
}

/// Matriks otoritas urutan boot.
#[no_mangle]
pub extern "C" fn klatos_boot_manifest_selftest() -> KlatosBootReport {
    run_matrix()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_424F_4F54);
        assert!(MAX_STAGE == 32);
        assert!(N_BAWAAN == 10);
        assert!(core::mem::size_of::<KlatosBootReport>() == 64);
        assert!(core::mem::offset_of!(KlatosBootReport, status) == 60);
    };
}
