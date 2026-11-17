// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Rust-owned bounded COW bitmap arena for `AnonymousVMObject`.
#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering};

pub const REPORT_TAG: u64 = 0x4B4C_4154_434F_574D; // KLATCOWM
pub const MAX_MAPS: usize = 16_320;
/// Eight MiB of bits, bounded and shared by all live maps.
pub const MAX_WORDS: usize = 1_048_576;
pub const MAX_PAGES: u64 = (MAX_WORDS as u64) * 64;
pub const INVALID: u32 = 0xFFFF_FFFF;

static MAP_GEN: [AtomicU32; MAX_MAPS] = [const { AtomicU32::new(0) }; MAX_MAPS];
static MAP_ALIVE: [AtomicU32; MAX_MAPS] = [const { AtomicU32::new(0) }; MAX_MAPS];
static MAP_OFFSET: [AtomicU32; MAX_MAPS] = [const { AtomicU32::new(0) }; MAX_MAPS];
static MAP_WORDS: [AtomicU32; MAX_MAPS] = [const { AtomicU32::new(0) }; MAX_MAPS];
static MAP_BITS: [AtomicU64; MAX_MAPS] = [const { AtomicU64::new(0) }; MAX_MAPS];
static WORD_OWNER: [AtomicU16; MAX_WORDS] = [const { AtomicU16::new(0) }; MAX_WORDS];
static WORDS: [AtomicU64; MAX_WORDS] = [const { AtomicU64::new(0) }; MAX_WORDS];
static LOCK: AtomicU32 = AtomicU32::new(0);
/// Status interupsi pemegang LOCK; hanya pemegang yang menulis.
static LOCK_FLAGS: AtomicU64 = AtomicU64::new(0);

static CREATE_COUNT: AtomicU32 = AtomicU32::new(0);
static RELEASE_COUNT: AtomicU32 = AtomicU32::new(0);
static GET_COUNT: AtomicU32 = AtomicU32::new(0);
static SET_COUNT: AtomicU32 = AtomicU32::new(0);
static INVALID_COUNT: AtomicU32 = AtomicU32::new(0);

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

#[inline]
fn valid(index: u32, generation: u32) -> bool {
    let i = index as usize;
    generation != 0 && i < MAX_MAPS
        && MAP_ALIVE[i].load(Ordering::Acquire) != 0
        && MAP_GEN[i].load(Ordering::Acquire) == generation
}

#[inline]
fn words_for(bits: u64) -> Option<usize> {
    if bits == 0 || bits > MAX_PAGES { return None; }
    Some(((bits - 1) / 64 + 1) as usize)
}

#[inline]
fn last_mask(bits: u64) -> u64 {
    let rem = bits & 63;
    if rem == 0 { u64::MAX } else { (1u64 << rem) - 1 }
}

/// Create a map and return `(generation << 32) | slot`; zero means ENOMEM/too large.
#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_create(bits: u64) -> u64 {
    CREATE_COUNT.fetch_add(1, Ordering::Relaxed);
    let words = match words_for(bits) { Some(v) => v, None => { INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; } };
    lock();
    let mut slot = None;
    let mut i = 0usize;
    while i < MAX_MAPS {
        if MAP_ALIVE[i].load(Ordering::Relaxed) == 0 { slot = Some(i); break; }
        i += 1;
    }
    let mut start = None;
    if slot.is_some() {
        let mut s = 0usize;
        while s + words <= MAX_WORDS {
            let mut free = true;
            let mut w = 0usize;
            while w < words {
                if WORD_OWNER[s + w].load(Ordering::Relaxed) != 0 { free = false; break; }
                w += 1;
            }
            if free { start = Some(s); break; }
            s += 1;
        }
    }
    let result = if let (Some(slot), Some(start)) = (slot, start) {
        let old = MAP_GEN[slot].load(Ordering::Relaxed);
        let generation = old.wrapping_add(1);
        if generation == 0 {
            0
        } else {
            let owner = (slot + 1) as u16;
            let mut w = 0usize;
            while w < words {
                WORD_OWNER[start + w].store(owner, Ordering::Relaxed);
                WORDS[start + w].store(u64::MAX, Ordering::Relaxed);
                w += 1;
            }
            WORDS[start + words - 1].fetch_and(last_mask(bits), Ordering::Relaxed);
            MAP_OFFSET[slot].store(start as u32, Ordering::Relaxed);
            MAP_WORDS[slot].store(words as u32, Ordering::Relaxed);
            MAP_BITS[slot].store(bits, Ordering::Relaxed);
            MAP_GEN[slot].store(generation, Ordering::Relaxed);
            MAP_ALIVE[slot].store(1, Ordering::Release);
            ((generation as u64) << 32) | slot as u64
        }
    } else { 0 };
    unlock();
    if result == 0 { INVALID_COUNT.fetch_add(1, Ordering::Relaxed); }
    result
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_release(index: u32, generation: u32) -> u32 {
    if !valid(index, generation) { INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; }
    lock();
    let i = index as usize;
    if !valid(index, generation) { unlock(); INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; }
    let start = MAP_OFFSET[i].load(Ordering::Relaxed) as usize;
    let words = MAP_WORDS[i].load(Ordering::Relaxed) as usize;
    let mut w = 0usize;
    while w < words { WORDS[start + w].store(0, Ordering::Relaxed); WORD_OWNER[start + w].store(0, Ordering::Relaxed); w += 1; }
    MAP_ALIVE[i].store(0, Ordering::Release);
    RELEASE_COUNT.fetch_add(1, Ordering::Relaxed);
    unlock();
    1
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_clear(index: u32, generation: u32) -> u32 {
    if !valid(index, generation) { INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; }
    lock();
    let i = index as usize;
    if !valid(index, generation) { unlock(); INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; }
    let start = MAP_OFFSET[i].load(Ordering::Relaxed) as usize;
    let words = MAP_WORDS[i].load(Ordering::Relaxed) as usize;
    let mut w = 0usize;
    while w < words { WORDS[start + w].store(0, Ordering::Relaxed); w += 1; }
    unlock();
    1
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_get(index: u32, generation: u32, bit: u64) -> u32 {
    if !valid(index, generation) { INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; }
    let i = index as usize;
    let bits = MAP_BITS[i].load(Ordering::Acquire);
    if bit >= bits { return 0; }
    let start = MAP_OFFSET[i].load(Ordering::Acquire) as usize;
    GET_COUNT.fetch_add(1, Ordering::Relaxed);
    ((WORDS[start + (bit as usize / 64)].load(Ordering::Acquire) >> (bit & 63)) & 1) as u32
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_set(index: u32, generation: u32, bit: u64, value: u32) -> u32 {
    if !valid(index, generation) { INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; }
    lock();
    let i = index as usize;
    if !valid(index, generation) { unlock(); INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; }
    let bits = MAP_BITS[i].load(Ordering::Relaxed);
    if bit >= bits { unlock(); return 0; }
    let word = MAP_OFFSET[i].load(Ordering::Relaxed) as usize + bit as usize / 64;
    let mask = 1u64 << (bit & 63);
    if value == 0 { WORDS[word].fetch_and(!mask, Ordering::Relaxed); } else { WORDS[word].fetch_or(mask, Ordering::Relaxed); }
    SET_COUNT.fetch_add(1, Ordering::Relaxed);
    unlock();
    1
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_count(index: u32, generation: u32) -> u64 {
    if !valid(index, generation) { INVALID_COUNT.fetch_add(1, Ordering::Relaxed); return 0; }
    let i = index as usize;
    let bits = MAP_BITS[i].load(Ordering::Acquire);
    let start = MAP_OFFSET[i].load(Ordering::Acquire) as usize;
    let words = MAP_WORDS[i].load(Ordering::Acquire) as usize;
    let mut total = 0u64;
    let mut w = 0usize;
    while w < words { let mut value = WORDS[start + w].load(Ordering::Acquire); if w + 1 == words { value &= last_mask(bits); } total += value.count_ones() as u64; w += 1; }
    total
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_capacity_words() -> u32 { MAX_WORDS as u32 }
#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_live() -> u32 {
    let mut n = 0u32; let mut i = 0usize; while i < MAX_MAPS { n += MAP_ALIVE[i].load(Ordering::Relaxed); i += 1; } n
}
#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_counter(which: u32) -> u32 {
    match which { 0 => CREATE_COUNT.load(Ordering::Relaxed), 1 => RELEASE_COUNT.load(Ordering::Relaxed), 2 => GET_COUNT.load(Ordering::Relaxed), 3 => SET_COUNT.load(Ordering::Relaxed), 4 => INVALID_COUNT.load(Ordering::Relaxed), _ => 0 }
}

#[repr(C)]
pub struct KlatosCowMapReport { pub tag: u64, pub checks: u32, pub passed: u32, pub differential: u32, pub mismatches: u32, pub status: i32 }
const _: [(); 32] = [(); core::mem::size_of::<KlatosCowMapReport>()];

/// Bounded arena, bit semantics, count, release, and generation differential.
#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_map_selftest() -> KlatosCowMapReport {
    let mut checks = 0u32; let mut passed = 0u32;
    macro_rules! ck { ($ok:expr) => {{ checks += 1; if $ok { passed += 1; } }}; }
    ck!(REPORT_TAG == 0x4B4C_4154_434F_574D); ck!(MAX_WORDS == 1_048_576); ck!(MAX_PAGES == 67_108_864);
    let h = klatos_vmoan_cow_map_create(130); ck!(h != 0);
    let i = h as u32; let g = (h >> 32) as u32;
    ck!(klatos_vmoan_cow_map_count(i, g) == 130); ck!(klatos_vmoan_cow_map_get(i, g, 129) == 1); ck!(klatos_vmoan_cow_map_get(i, g, 130) == 0);
    ck!(klatos_vmoan_cow_map_set(i, g, 3, 0) == 1); ck!(klatos_vmoan_cow_map_get(i, g, 3) == 0); ck!(klatos_vmoan_cow_map_count(i, g) == 129);
    ck!(klatos_vmoan_cow_map_clear(i, g) == 1); ck!(klatos_vmoan_cow_map_count(i, g) == 0); ck!(klatos_vmoan_cow_map_release(i, g) == 1); ck!(klatos_vmoan_cow_map_get(i, g, 3) == 0);
    let mut ref_bits = [true; 257]; let h2 = klatos_vmoan_cow_map_create(257); let i2 = h2 as u32; let g2 = (h2 >> 32) as u32; let mut n = 0u32; let mut state = 0xC0FFEEu64;
    while n < 50_000 { state = state.wrapping_mul(6364136223846793005).wrapping_add(1); let bit = (state as usize) & 256; let value = ((state >> 17) & 1) != 0; ref_bits[bit] = value; let _ = klatos_vmoan_cow_map_set(i2, g2, bit as u64, value as u32); let got = klatos_vmoan_cow_map_get(i2, g2, bit as u64) != 0; checks += 1; if got == value { passed += 1; } else { /* mismatch counted below */ } n += 1; }
    let mut mismatches = 0u32; for bit in 0..257 { checks += 1; let got = klatos_vmoan_cow_map_get(i2, g2, bit) != 0; if got == ref_bits[bit as usize] { passed += 1; } else { mismatches += 1; } }
    let expected = ref_bits.iter().filter(|x| **x).count() as u64; ck!(klatos_vmoan_cow_map_count(i2, g2) == expected); ck!(klatos_vmoan_cow_map_release(i2, g2) == 1);
    KlatosCowMapReport { tag: REPORT_TAG, checks, passed, differential: 50_257, mismatches, status: if checks == passed && mismatches == 0 { 0 } else { -1 } }
}
