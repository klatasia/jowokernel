// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.42 — OTORITAS PENDAFTARAN PERANGKAT (Rust).


use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub const REPORT_TAG: u64 = 0x4B4C_4154_5044_5652; // "KLATPDVR"
pub const MAX_DEVICES: usize = 1024;
pub const N_COUNTER: usize = 7;
const RENTANG_DIFERENSIAL: u32 = 100_000;
pub const KIND_BLOCK: u32 = 1;
pub const KIND_CHAR: u32 = 2;

static KUNCI: AtomicBool = AtomicBool::new(false);
static KUNCI_ENTRY: [AtomicU64; MAX_DEVICES] = [const { AtomicU64::new(0) }; MAX_DEVICES];
static DIPAKAI: [AtomicU32; MAX_DEVICES] = [const { AtomicU32::new(0) }; MAX_DEVICES];
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

/// Status interupsi pemegang KUNCI. Hanya pemegang yang menulis dan membacanya
/// sebelum melepas, jadi tidak perlu atomik kuat.
static KUNCI_FLAGS: AtomicU64 = AtomicU64::new(0);

fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while KUNCI.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    KUNCI_FLAGS.store(flags, Ordering::Relaxed);
}

fn unlock() {
    let flags = KUNCI_FLAGS.load(Ordering::Relaxed);
    KUNCI.store(false, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}
fn valid_kind(kind: u32) -> bool { kind == KIND_BLOCK || kind == KIND_CHAR }
fn packed(kind: u32, major: u32, minor: u32) -> u64 {
    (((kind - 1) as u64) << 63) | ((major as u64) << 32) | minor as u64
}
fn find(v: u64) -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_DEVICES {
        if DIPAKAI[i].load(Ordering::Relaxed) != 0 && KUNCI_ENTRY[i].load(Ordering::Relaxed) == v { return Some(i); }
        i += 1;
    }
    None
}
fn count() -> u32 {
    let mut n = 0; let mut i = 0;
    while i < MAX_DEVICES { if DIPAKAI[i].load(Ordering::Acquire) != 0 { n += 1; } i += 1; }
    n
}

/// Independent historical model: two maps represented as one key set.
pub struct Model { pub keys: [u64; MAX_DEVICES], pub used: [bool; MAX_DEVICES], pub len: usize }
impl Model {
    pub const fn baru() -> Self { Model { keys: [0; MAX_DEVICES], used: [false; MAX_DEVICES], len: 0 } }
    pub fn register(&mut self, kind: u32, major: u32, minor: u32) -> bool {
        let key = packed(kind, major, minor);
        let mut i = 0; while i < MAX_DEVICES { if self.used[i] && self.keys[i] == key { return false; } i += 1; }
        if self.len == MAX_DEVICES { return false; }
        let mut j = 0; while j < MAX_DEVICES { if !self.used[j] { self.used[j] = true; self.keys[j] = key; self.len += 1; return true; } j += 1; }
        false
    }
    pub fn remove(&mut self, kind: u32, major: u32, minor: u32) -> bool {
        let key = packed(kind, major, minor); let mut i = 0;
        while i < MAX_DEVICES { if self.used[i] && self.keys[i] == key { self.used[i] = false; self.len -= 1; return true; } i += 1; }
        false
    }
    pub fn contains(&self, kind: u32, major: u32, minor: u32) -> bool { let key = packed(kind, major, minor); let mut i = 0; while i < MAX_DEVICES { if self.used[i] && self.keys[i] == key { return true; } i += 1; } false }
}
pub fn kebijakan_lama(m: &mut Model, op: u32, kind: u32, major: u32, minor: u32) -> bool {
    if op == 0 { m.register(kind, major, minor) } else { m.remove(kind, major, minor) }
}

/// Register a device key. Return 1 on insertion, 0 on invalid/duplicate/full.
#[no_mangle]
pub extern "C" fn klatos_pendaftaran_perangkat_daftar(kind: u32, major: u32, minor: u32) -> u32 {
    if !valid_kind(kind) || major > 0x7fff_ffff { COUNTERS[5].fetch_add(1, Ordering::Relaxed); return 0; }
    let key = packed(kind, major, minor);
    lock();
    let duplicate = find(key).is_some();
    let mut free = None;
    if !duplicate { let mut i = 0; while i < MAX_DEVICES { if DIPAKAI[i].load(Ordering::Relaxed) == 0 { free = Some(i); break; } i += 1; } }
    let ok = !duplicate && free.is_some();
    if let Some(i) = free { DIPAKAI[i].store(1, Ordering::Relaxed); KUNCI_ENTRY[i].store(key, Ordering::Release); }
    unlock();
    if ok { COUNTERS[0].fetch_add(1, Ordering::Relaxed); } else { COUNTERS[2].fetch_add(1, Ordering::Relaxed); }
    ok as u32
}

/// Remove a registered key. Return 1 on removal.
#[no_mangle]
pub extern "C" fn klatos_pendaftaran_perangkat_lepas(kind: u32, major: u32, minor: u32) -> u32 {
    if !valid_kind(kind) || major > 0x7fff_ffff { COUNTERS[5].fetch_add(1, Ordering::Relaxed); return 0; }
    let key = packed(kind, major, minor); lock();
    let found = find(key); if let Some(i) = found { DIPAKAI[i].store(0, Ordering::Release); KUNCI_ENTRY[i].store(0, Ordering::Relaxed); }
    unlock();
    if found.is_some() { COUNTERS[1].fetch_add(1, Ordering::Relaxed); } else { COUNTERS[3].fetch_add(1, Ordering::Relaxed); }
    found.is_some() as u32
}

#[no_mangle]
pub extern "C" fn klatos_pendaftaran_perangkat_ada(kind: u32, major: u32, minor: u32) -> u32 {
    if !valid_kind(kind) || major > 0x7fff_ffff { return 0; }
    let key = packed(kind, major, minor); lock(); let ok = find(key).is_some(); unlock(); ok as u32
}
#[no_mangle]
pub extern "C" fn klatos_pendaftaran_perangkat_jumlah() -> u32 { count() }
#[no_mangle]
pub extern "C" fn klatos_pendaftaran_perangkat_counter(which: u32) -> u32 { if which as usize >= N_COUNTER { return 0; } COUNTERS[which as usize].load(Ordering::Relaxed) }

#[repr(C)]
pub struct KlatosPendaftaranReport { pub tag: u64, pub checks: u32, pub passed: u32, pub peta_checks: u32, pub peta_passed: u32, pub neg_checks: u32, pub neg_passed: u32, pub prod_a: u32, pub prod_b: u32, pub prod_c: u32, pub diferensial_diukur: u32, pub diferensial_beda: u32, pub status: i32 }
struct Matrix { checks: u32, passed: u32, groups: [u32; 3], pass: [u32; 3] }
impl Matrix { const fn baru() -> Self { Matrix { checks: 0, passed: 0, groups: [0; 3], pass: [0; 3] } } fn catat(&mut self, ok: bool, g: usize) { self.checks += 1; self.groups[g] += 1; if ok { self.passed += 1; self.pass[g] += 1; } } }

fn run_matrix() -> KlatosPendaftaranReport {
    let mut m = Matrix::baru(); let before = count();
    // group 0: register/duplicate/contains/remove/reuse
    let _ = klatos_pendaftaran_perangkat_daftar(KIND_BLOCK, 1, 2);
    m.catat(klatos_pendaftaran_perangkat_ada(KIND_BLOCK, 1, 2) == 1, 0);
    m.catat(klatos_pendaftaran_perangkat_jumlah() == before + 1, 0);
    m.catat(klatos_pendaftaran_perangkat_daftar(KIND_BLOCK, 1, 2) == 0, 0);
    m.catat(klatos_pendaftaran_perangkat_daftar(KIND_CHAR, 1, 2) == 1, 0);
    m.catat(klatos_pendaftaran_perangkat_jumlah() == before + 2, 0);
    m.catat(klatos_pendaftaran_perangkat_lepas(KIND_BLOCK, 1, 2) == 1, 0);
    m.catat(klatos_pendaftaran_perangkat_ada(KIND_BLOCK, 1, 2) == 0, 0);
    m.catat(klatos_pendaftaran_perangkat_lepas(KIND_BLOCK, 1, 2) == 0, 0);
    m.catat(klatos_pendaftaran_perangkat_lepas(KIND_CHAR, 1, 2) == 1, 0);
    m.catat(klatos_pendaftaran_perangkat_jumlah() == before, 0);
    // group 1: differential on an isolated local instance. The production
    // table already contains boot devices and must never be reset by a test.
    let mut model = Model::baru();
    let mut rust_local = Model::baru();
    let mut diff = 0;
    let mut measured = 0;
    let mut x = 0x1234_5678u32;
    while measured < RENTANG_DIFERENSIAL {
        x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let op = x & 1;
        let kind = if x & 2 == 0 { KIND_BLOCK } else { KIND_CHAR };
        let major = 1000 + ((x >> 8) & 15);
        let minor = 1000 + ((x >> 16) & 31);
        let a = if op == 0 {
            rust_local.register(kind, major, minor)
        } else {
            rust_local.remove(kind, major, minor)
        };
        let b = kebijakan_lama(&mut model, op, kind, major, minor);
        if a != b { diff += 1; }
        if rust_local.contains(kind, major, minor) != model.contains(kind, major, minor) { diff += 1; }
        measured += 1;
    }
    m.catat(diff == 0, 1); m.catat(measured == RENTANG_DIFERENSIAL, 1);
    // group 2: invalid and production isolation
    m.catat(klatos_pendaftaran_perangkat_daftar(0, 1, 1) == 0, 2);
    m.catat(klatos_pendaftaran_perangkat_daftar(KIND_BLOCK, 0x8000_0000, 1) == 0, 2);
    m.catat(klatos_pendaftaran_perangkat_lepas(99, 1, 1) == 0, 2);
    m.catat(klatos_pendaftaran_perangkat_ada(99, 1, 1) == 0, 2);
    m.catat(klatos_pendaftaran_perangkat_counter(99) == 0, 2);
    let mut report = KlatosPendaftaranReport { tag: REPORT_TAG, checks: m.checks, passed: m.passed, peta_checks: m.groups[1], peta_passed: m.pass[1], neg_checks: m.groups[2], neg_passed: m.pass[2], prod_a: klatos_pendaftaran_perangkat_counter(0), prod_b: klatos_pendaftaran_perangkat_counter(1), prod_c: klatos_pendaftaran_perangkat_counter(2), diferensial_diukur: measured, diferensial_beda: diff, status: 0 };
    if report.checks != report.passed { report.status = -1; } report
}
#[no_mangle]
pub extern "C" fn klatos_pendaftaran_perangkat_selftest() -> KlatosPendaftaranReport { run_matrix() }

pub mod conformance { use super::*; const _: () = { assert!(REPORT_TAG == 0x4B4C_4154_5044_5652); assert!(MAX_DEVICES == 1024); assert!(N_COUNTER == 7); assert!(core::mem::size_of::<KlatosPendaftaranReport>() == 56); }; }
