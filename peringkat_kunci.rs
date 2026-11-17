// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8 BATCH D — **LOCK-RANK STATE + ORDERING**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub const REPORT_TAG: u64 = 0x4B4C_4154_4C52_4E31; // "KLATLRN1"
pub const N_SLOT: usize = 8192;
pub const NONE: u32 = 0;
const ERR_INVALID: i32 = -22;
const ERR_ORDER: i32 = -1;

static KUNCI: AtomicBool = AtomicBool::new(false);
static ACQUIRE: AtomicU32 = AtomicU32::new(0);
static RELEASE: AtomicU32 = AtomicU32::new(0);
static TOLAK: AtomicU32 = AtomicU32::new(0);

struct Slot {
    key: AtomicU64,
    mask: AtomicU32,
}
impl Slot {
    const fn baru() -> Self { Self { key: AtomicU64::new(0), mask: AtomicU32::new(0) } }
}
static SLOTS: [Slot; N_SLOT] = [const { Slot::baru() }; N_SLOT];

fn lock() {
    while KUNCI.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
}
fn unlock() { KUNCI.store(false, Ordering::Release); }

#[inline]
fn key(tid: u64) -> Option<u64> {
    let k = tid.checked_add(1)?;
    if k == 0 { None } else { Some(k) }
}

#[inline]
fn one_bit(rank: u32) -> bool { rank != 0 && (rank & (rank - 1)) == 0 }

fn find(key: u64) -> Option<usize> {
    let start = (key as usize) % N_SLOT;
    // Scan the complete bounded table: reset/release clears a key, so an
    // early-empty stop would break lookup for a later colliding key.
    for step in 0..N_SLOT {
        let i = (start + step) % N_SLOT;
        if SLOTS[i].key.load(Ordering::Acquire) == key { return Some(i); }
    }
    None
}

fn find_or_create(key: u64) -> Option<usize> {
    let start = (key as usize) % N_SLOT;
    let mut first_empty = None;
    for step in 0..N_SLOT {
        let i = (start + step) % N_SLOT;
        let k = SLOTS[i].key.load(Ordering::Acquire);
        if k == key { return Some(i); }
        if k == 0 && first_empty.is_none() { first_empty = Some(i); }
    }
    let i = first_empty?;
    SLOTS[i].mask.store(0, Ordering::Relaxed);
    SLOTS[i].key.store(key, Ordering::Release);
    Some(i)
}

/// Acquire requires the new rank to be strictly lower than the highest rank
/// currently held, matching the historical `m_lock_rank_mask > rank` check.
pub fn acquire(tid: u64, rank: u32) -> i32 {
    let Some(k) = key(tid) else { TOLAK.fetch_add(1, Ordering::Relaxed); return ERR_INVALID; };
    if !one_bit(rank) { TOLAK.fetch_add(1, Ordering::Relaxed); return ERR_INVALID; }
    lock();
    let result = match find_or_create(k) {
        None => ERR_INVALID,
        Some(i) => {
            let mask = SLOTS[i].mask.load(Ordering::Relaxed);
            if mask != 0 && mask <= rank {
                TOLAK.fetch_add(1, Ordering::Relaxed);
                ERR_ORDER
            } else {
                SLOTS[i].mask.store(mask | rank, Ordering::Relaxed);
                ACQUIRE.fetch_add(1, Ordering::Relaxed);
                0
            }
        }
    };
    unlock();
    result
}

/// Release requires a single-bit rank held at the highest currently-held
/// position. This is exactly the historical C++ `rank_is_in_order` predicate.
pub fn release(tid: u64, rank: u32) -> i32 {
    let Some(k) = key(tid) else { TOLAK.fetch_add(1, Ordering::Relaxed); return ERR_INVALID; };
    if !one_bit(rank) { TOLAK.fetch_add(1, Ordering::Relaxed); return ERR_INVALID; }
    lock();
    let result = match find(k) {
        None => ERR_INVALID,
        Some(i) => {
            let mask = SLOTS[i].mask.load(Ordering::Relaxed);
            let in_order = (mask & (mask - 1)) | rank;
            if mask & rank == 0 || in_order != mask {
                TOLAK.fetch_add(1, Ordering::Relaxed);
                ERR_ORDER
            } else {
                let new_mask = mask ^ rank;
                SLOTS[i].mask.store(new_mask, Ordering::Relaxed);
                if new_mask == 0 { SLOTS[i].key.store(0, Ordering::Release); }
                RELEASE.fetch_add(1, Ordering::Relaxed);
                0
            }
        }
    };
    unlock();
    result
}

pub fn reset(tid: u64) -> i32 {
    let Some(k) = key(tid) else { return ERR_INVALID; };
    lock();
    if let Some(i) = find(k) {
        SLOTS[i].mask.store(0, Ordering::Relaxed);
        SLOTS[i].key.store(0, Ordering::Release);
    }
    unlock();
    0
}

pub fn mask(tid: u64) -> u32 {
    let Some(k) = key(tid) else { return 0; };
    find(k).map(|i| SLOTS[i].mask.load(Ordering::Acquire)).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_lockrank_acquire(tid: u64, rank: u32) -> i32 { acquire(tid, rank) }
#[unsafe(no_mangle)]
pub extern "C" fn klatos_lockrank_release(tid: u64, rank: u32) -> i32 { release(tid, rank) }
#[unsafe(no_mangle)]
pub extern "C" fn klatos_lockrank_reset(tid: u64) -> i32 { reset(tid) }
#[unsafe(no_mangle)]
pub extern "C" fn klatos_lockrank_mask(tid: u64) -> u32 { mask(tid) }
#[unsafe(no_mangle)]
pub extern "C" fn klatos_lockrank_hidup() -> u32 {
    SLOTS.iter().filter(|s| s.key.load(Ordering::Acquire) != 0).count() as u32
}

#[repr(C)]
pub struct KlatosLockRankReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub hidup: u32,
    pub prod_acquire: u32,
    pub prod_release: u32,
    pub prod_tolak: u32,
    pub reserved: u32,
    pub status: i32,
    pub reserved2: u32,
}
struct M { checks: u32, passed: u32 }
impl M { fn catat(&mut self, ok: bool) { self.checks += 1; if ok { self.passed += 1; } } }

pub fn run_matrix() -> KlatosLockRankReport {
    let mut m = M { checks: 0, passed: 0 };
    let tid = 0xF8D0u64;
    reset(tid);
    m.catat(acquire(tid, 0x10) == 0); // Process
    m.catat(mask(tid) == 0x10);
    m.catat(acquire(tid, 0x08) == 0); // Thread below Process
    m.catat(mask(tid) == 0x18);
    m.catat(acquire(tid, 0x08) == 0); // same bit is idempotent while higher rank is held
    m.catat(acquire(tid, 0x20) == ERR_ORDER); // greater rank rejected
    m.catat(release(tid, 0x10) == ERR_ORDER); // not highest
    m.catat(release(tid, 0x08) == 0);
    m.catat(mask(tid) == 0x10);
    m.catat(release(tid, 0x10) == 0);
    m.catat(mask(tid) == 0);
    m.catat(acquire(tid, 3) == ERR_INVALID); // not one bit
    m.catat(release(tid, 0x01) == ERR_INVALID); // no slot
    m.catat(acquire(tid, 0x01) == 0);
    m.catat(reset(tid) == 0 && mask(tid) == 0);
    m.catat(m.checks == m.passed);
    KlatosLockRankReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        hidup: klatos_lockrank_hidup(),
        prod_acquire: ACQUIRE.load(Ordering::Relaxed),
        prod_release: RELEASE.load(Ordering::Relaxed),
        prod_tolak: TOLAK.load(Ordering::Relaxed),
        reserved: 0,
        status: if m.checks == m.passed { 0 } else { 1 },
        reserved2: 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_lockrank_selftest() -> KlatosLockRankReport { run_matrix() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matrix_passes() {
        let r = run_matrix();
        assert_eq!(r.tag, REPORT_TAG);
        assert_eq!(r.checks, r.passed);
    }
}
