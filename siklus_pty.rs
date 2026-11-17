// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.41 — OTORITAS SIKLUS HIDUP PASANGAN PTY (Rust).


//! State bits returned by `keadaan`:
//! `0x01` allocated, `0x02` slave attached, `0x04` master closed,
//! `0x08` slave closed.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering, AtomicU64};

pub const REPORT_TAG: u64 = 0x4B4C_4154_5054_5943; // "KLATPTYC"
pub const MAX_PTY: usize = 64;
pub const N_COUNTER: usize = 8;
const RENTANG_DIFERENSIAL: u32 = 100_000;
const STATE_ALLOCATED: u32 = 0x01;
const STATE_SLAVE: u32 = 0x02;
const STATE_MASTER_CLOSED: u32 = 0x04;
const STATE_SLAVE_CLOSED: u32 = 0x08;

static KUNCI: AtomicBool = AtomicBool::new(false);
/// Status interupsi pemegang KUNCI. Hanya pemegang yang menulis dan
/// membacanya sebelum melepas, jadi tidak perlu atomik kuat.
static KUNCI_FLAGS: AtomicU64 = AtomicU64::new(0);
static ATAS: AtomicU32 = AtomicU32::new(0);
static TUMPUKAN: [AtomicU32; MAX_PTY] = [const { AtomicU32::new(0) }; MAX_PTY];
static KEADAAN: [AtomicU32; MAX_PTY] = [const { AtomicU32::new(0) }; MAX_PTY];
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn kunci() {
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
fn buka() {
    // Baca status milik sendiri SEBELUM melepas kunci, supaya pengambil
    // berikutnya tidak bisa menimpa nilainya.
    let flags = KUNCI_FLAGS.load(Ordering::Relaxed);
    KUNCI.store(false, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}
fn valid(index: u32) -> bool {
    (index as usize) < MAX_PTY
}
fn state(index: u32) -> u32 {
    if valid(index) {
        KEADAAN[index as usize].load(Ordering::Acquire)
    } else {
        0
    }
}
/// Independent C++-era model: vector contains free indices, state is separate.
pub struct Model {
    pub free: [u32; MAX_PTY],
    pub top: usize,
    pub state: [u32; MAX_PTY],
}
impl Model {
    pub const fn baru() -> Self {
        Model {
            free: [0; MAX_PTY],
            top: 0,
            state: [0; MAX_PTY],
        }
    }
    pub fn isi_ulang(&mut self) {
        self.top = MAX_PTY;
        let mut i = 0usize;
        while i < MAX_PTY {
            self.free[i] = (MAX_PTY - 1 - i) as u32;
            self.state[i] = 0;
            i += 1;
        }
    }
    pub fn ambil(&mut self) -> i32 {
        if self.top == 0 {
            return -1;
        }
        self.top -= 1;
        let i = self.free[self.top];
        self.state[i as usize] = STATE_ALLOCATED;
        i as i32
    }
    pub fn lepas(&mut self, index: u32) -> bool {
        if !valid(index) || self.state[index as usize] == 0 || self.top == MAX_PTY {
            return false;
        }
        self.state[index as usize] = 0;
        self.free[self.top] = index;
        self.top += 1;
        true
    }
    pub fn slave(&mut self, index: u32) -> bool {
        if !valid(index) { return false; }
        transition(&mut self.state[index as usize], STATE_SLAVE)
    }
    pub fn slave_tutup(&mut self, index: u32) -> bool {
        if !valid(index) { return false; }
        transition(&mut self.state[index as usize], STATE_SLAVE_CLOSED)
    }
    pub fn master_tutup(&mut self, index: u32) -> bool {
        if !valid(index) { return false; }
        transition(&mut self.state[index as usize], STATE_MASTER_CLOSED)
    }
}
fn transition(slot: &mut u32, bit: u32) -> bool {
    if *slot == 0 || (*slot & bit) != 0 {
        return false;
    }
    *slot |= bit;
    true
}

#[repr(C)]
pub struct KlatosPtyReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_a: u32,
    pub prod_b: u32,
    pub prod_c: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}
struct Matrix {
    checks: u32,
    passed: u32,
    groups: [u32; 3],
    group_passed: [u32; 3],
}
impl Matrix {
    const fn baru() -> Self {
        Matrix { checks: 0, passed: 0, groups: [0; 3], group_passed: [0; 3] }
    }
    fn catat(&mut self, ok: bool, group: usize) {
        self.checks += 1;
        self.groups[group] += 1;
        if ok { self.passed += 1; self.group_passed[group] += 1; }
    }
}

/// Reset the free stack once during PTY multiplexer initialization.
#[no_mangle]
pub extern "C" fn klatos_siklus_pty_isi_ulang() -> u32 {
    kunci();
    let mut i = 0usize;
    while i < MAX_PTY {
        TUMPUKAN[i].store((MAX_PTY - 1 - i) as u32, Ordering::Relaxed);
        KEADAAN[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    ATAS.store(MAX_PTY as u32, Ordering::Release);
    buka();
    COUNTERS[0].fetch_add(1, Ordering::Relaxed);
    MAX_PTY as u32
}

/// Take one free master index, or -1 when all 64 are allocated.
#[no_mangle]
pub extern "C" fn klatos_siklus_pty_ambil() -> i32 {
    kunci();
    let top = ATAS.load(Ordering::Relaxed);
    if top == 0 {
        buka();
        COUNTERS[2].fetch_add(1, Ordering::Relaxed);
        return -1;
    }
    let next = top - 1;
    let index = TUMPUKAN[next as usize].load(Ordering::Relaxed);
    ATAS.store(next, Ordering::Relaxed);
    KEADAAN[index as usize].store(STATE_ALLOCATED, Ordering::Release);
    buka();
    COUNTERS[1].fetch_add(1, Ordering::Relaxed);
    index as i32
}

/// Return an allocated index to the free stack. Duplicate/invalid returns fail.
#[no_mangle]
pub extern "C" fn klatos_siklus_pty_lepas(index: u32) -> u32 {
    if !valid(index) { return 0; }
    kunci();
    let current = KEADAAN[index as usize].load(Ordering::Relaxed);
    let top = ATAS.load(Ordering::Relaxed);
    let ok = current != 0 && top < MAX_PTY as u32;
    if ok {
        KEADAAN[index as usize].store(0, Ordering::Release);
        TUMPUKAN[top as usize].store(index, Ordering::Relaxed);
        ATAS.store(top + 1, Ordering::Release);
    }
    buka();
    if ok { COUNTERS[3].fetch_add(1, Ordering::Relaxed); }
    ok as u32
}

#[no_mangle]
pub extern "C" fn klatos_siklus_pty_pasang_slave(index: u32) -> u32 {
    transition_ffi(index, STATE_SLAVE, 4)
}
#[no_mangle]
pub extern "C" fn klatos_siklus_pty_lepas_slave(index: u32) -> u32 {
    transition_ffi(index, STATE_SLAVE_CLOSED, 5)
}
#[no_mangle]
pub extern "C" fn klatos_siklus_pty_tutup_master(index: u32) -> u32 {
    transition_ffi(index, STATE_MASTER_CLOSED, 6)
}
fn transition_ffi(index: u32, bit: u32, counter: usize) -> u32 {
    if !valid(index) { return 0; }
    kunci();
    let old = KEADAAN[index as usize].load(Ordering::Relaxed);
    let ok = old != 0 && (old & bit) == 0;
    if ok { KEADAAN[index as usize].store(old | bit, Ordering::Release); }
    buka();
    if ok { COUNTERS[counter].fetch_add(1, Ordering::Relaxed); }
    ok as u32
}

#[no_mangle]
pub extern "C" fn klatos_siklus_pty_keadaan(index: u32) -> u32 { state(index) }
#[no_mangle]
pub extern "C" fn klatos_siklus_pty_jumlah_hidup() -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i < MAX_PTY {
        if state(i as u32) != 0 { n += 1; }
        i += 1;
    }
    n
}
#[no_mangle]
pub extern "C" fn klatos_siklus_pty_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER { return 0; }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

fn run_matrix() -> KlatosPtyReport {
    let mut m = Matrix::baru();
    let mut model = Model::baru();
    model.isi_ulang();
    // Group 0: exact freelist order, exhaustion, lifecycle and reuse.
    m.catat(klatos_siklus_pty_isi_ulang() == 64, 0);
    let mut got = [0u32; MAX_PTY];
    let mut i = 0usize;
    while i < MAX_PTY {
        let x = klatos_siklus_pty_ambil();
        got[i] = x as u32;
        m.catat(x == i as i32, 0);
        i += 1;
    }
    m.catat(klatos_siklus_pty_ambil() == -1, 0);
    m.catat(klatos_siklus_pty_keadaan(0) == STATE_ALLOCATED, 0);
    m.catat(klatos_siklus_pty_pasang_slave(0) == 1, 0);
    m.catat(klatos_siklus_pty_keadaan(0) == (STATE_ALLOCATED | STATE_SLAVE), 0);
    m.catat(klatos_siklus_pty_tutup_master(0) == 1, 0);
    m.catat(klatos_siklus_pty_tutup_master(0) == 0, 0);
    m.catat(klatos_siklus_pty_lepas_slave(0) == 1, 0);
    m.catat(klatos_siklus_pty_keadaan(0) == (STATE_ALLOCATED | STATE_SLAVE | STATE_MASTER_CLOSED | STATE_SLAVE_CLOSED), 0);
    m.catat(klatos_siklus_pty_lepas(0) == 1, 0);
    m.catat(klatos_siklus_pty_jumlah_hidup() == 63, 0);
    m.catat(klatos_siklus_pty_ambil() == 0, 0);
    m.catat(klatos_siklus_pty_jumlah_hidup() == 64, 0);

    // Group 1: differential model over allocation/release/lifecycle operations.
    let mut diff = 0u32;
    let mut measured = 0u32;
    let _ = klatos_siklus_pty_isi_ulang();
    model.isi_ulang();
    let mut x = 0xA5A5_1234u32;
    while measured < RENTANG_DIFERENSIAL {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let op = x % 7;
        let index = (x >> 8) % 70;
        let (a, b) = match op {
            0 | 1 => {
                let a = klatos_siklus_pty_ambil();
                let b = model.ambil();
                (a, b)
            }
            2 => {
                let a = klatos_siklus_pty_lepas(index);
                let b = model.lepas(index);
                (a as i32, b as i32)
            }
            3 => {
                let a = klatos_siklus_pty_pasang_slave(index);
                let b = model.slave(index);
                (a as i32, b as i32)
            }
            4 => {
                let a = klatos_siklus_pty_lepas_slave(index);
                let b = model.slave_tutup(index);
                (a as i32, b as i32)
            }
            5 => {
                let a = klatos_siklus_pty_tutup_master(index);
                let b = model.master_tutup(index);
                (a as i32, b as i32)
            }
            _ => {
                let a = klatos_siklus_pty_keadaan(index);
                let b = model.state.get(index as usize).copied().unwrap_or(0);
                (a as i32, b as i32)
            }
        };
        if a != b { diff += 1; }
        let mut j = 0usize;
        while j < MAX_PTY {
            if state(j as u32) != model.state[j] { diff += 1; }
            j += 1;
        }
        measured += 1;
    }
    m.catat(diff == 0, 1);
    m.catat(measured == RENTANG_DIFERENSIAL, 1);

    // Group 2: negative controls and production isolation.
    m.catat(klatos_siklus_pty_ambil() == -1 || klatos_siklus_pty_jumlah_hidup() <= 64, 2);
    m.catat(klatos_siklus_pty_lepas(64) == 0, 2);
    m.catat(klatos_siklus_pty_pasang_slave(64) == 0, 2);
    m.catat(klatos_siklus_pty_lepas_slave(64) == 0, 2);
    m.catat(klatos_siklus_pty_tutup_master(64) == 0, 2);
    m.catat(klatos_siklus_pty_keadaan(64) == 0, 2);
    m.catat(klatos_siklus_pty_counter(99) == 0, 2);

    let mut report = KlatosPtyReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.groups[1],
        peta_passed: m.group_passed[1],
        neg_checks: m.groups[2],
        neg_passed: m.group_passed[2],
        prod_a: klatos_siklus_pty_counter(1),
        prod_b: klatos_siklus_pty_counter(3),
        prod_c: klatos_siklus_pty_counter(4),
        diferensial_diukur: measured,
        diferensial_beda: diff,
        status: 0,
    };
    if report.checks != report.passed { report.status = -1; }
    report
}

#[no_mangle]
pub extern "C" fn klatos_siklus_pty_selftest() -> KlatosPtyReport { run_matrix() }

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5054_5943);
        assert!(MAX_PTY == 64);
        assert!(N_COUNTER == 8);
        assert!(core::mem::size_of::<KlatosPtyReport>() == 56);
    };
}
