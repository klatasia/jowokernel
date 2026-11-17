// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8 BATCH C — **FAT 8.3 SFN MECHANISM**.


#![allow(unsafe_code)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub const REPORT_TAG: u64 = 0x4B4C_4154_5346_4E31; // "KLATSFN1"
pub const TAK_SAH: u64 = u64::MAX;
pub const N_SLOT: usize = 4096;
const NAME_MAX: usize = 6;
const EXT_MAX: usize = 3;

static KUNCI: AtomicBool = AtomicBool::new(false);
static COUNTER_BUAT: AtomicU32 = AtomicU32::new(0);
static COUNTER_LEPAS: AtomicU32 = AtomicU32::new(0);
static COUNTER_TOLAK: AtomicU32 = AtomicU32::new(0);

struct Slot {
    gen: AtomicU32,
    live: AtomicBool,
    name_len: AtomicU32,
    name: UnsafeCell<[u8; NAME_MAX]>,
    ext_len: AtomicU32,
    ext: UnsafeCell<[u8; EXT_MAX]>,
    unique: AtomicU64,
}

impl Slot {
    const fn baru() -> Self {
        Self {
            gen: AtomicU32::new(0),
            live: AtomicBool::new(false),
            name_len: AtomicU32::new(0),
            name: UnsafeCell::new([0; NAME_MAX]),
            ext_len: AtomicU32::new(0),
            ext: UnsafeCell::new([0; EXT_MAX]),
            unique: AtomicU64::new(1),
        }
    }
}

static SLOTS: [Slot; N_SLOT] = [const { Slot::baru() }; N_SLOT];

// SFN byte arrays are initialized while the creator lock is held and are
// immutable for the remainder of a live handle. `UnsafeCell` is used only for
// that one-time initialization; slot lifetime and generation checks protect
// all readers.
unsafe impl Sync for Slot {}

fn lock() {
    while KUNCI.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
}

fn unlock() {
    KUNCI.store(false, Ordering::Release);
}

#[inline]
unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 || ptr.is_null() {
        &[]
    } else {
        core::slice::from_raw_parts(ptr, len)
    }
}

#[inline]
fn split_handle(h: u64) -> Option<(usize, u32)> {
    let raw = (h & 0xFFFF_FFFF) as usize;
    if raw == 0 || raw > N_SLOT {
        return None;
    }
    Some((raw - 1, (h >> 32) as u32))
}

fn slot(h: u64) -> Option<&'static Slot> {
    let (idx, generation) = split_handle(h)?;
    let s = &SLOTS[idx];
    if !s.live.load(Ordering::Acquire) || s.gen.load(Ordering::Acquire) != generation {
        return None;
    }
    Some(s)
}

fn valid_char(c: u8) -> bool {
    (b'A'..=b'Z').contains(&c)
        || (b'0'..=b'9').contains(&c)
        || matches!(c, b'$' | b'%' | b'\'' | b'-' | b'_' | b'@' | b' ' | b'~' | b'`' | b'!' | b'(' | b')')
}

fn uppercase(c: u8) -> u8 {
    if (b'a'..=b'z').contains(&c) {
        c - (b'a' - b'A')
    } else {
        c
    }
}

fn find_last_dot(s: &[u8]) -> Option<usize> {
    let mut i = s.len();
    while i > 0 {
        i -= 1;
        if s[i] == b'.' {
            return Some(i);
        }
    }
    None
}

/// Cermin `is_valid_sfn`.
pub fn is_valid_sfn(sfn: &[u8]) -> bool {
    let (name, ext) = match sfn.iter().position(|&c| c == b'.') {
        None => (sfn, &[][..]),
        Some(dot) => {
            if dot + 1 >= sfn.len() {
                return false;
            }
            (&sfn[..dot], &sfn[dot + 1..])
        }
    };
    if name.len() > 8 || ext.len() > 3 {
        return false;
    }
    if name.iter().chain(ext.iter()).any(|&c| !valid_char(c)) {
        return false;
    }
    !name.is_empty() && name[0] != b' '
}

fn make_slot(name: &[u8], ext: &[u8], unique: u64) -> u64 {
    if name.is_empty() || name.len() > NAME_MAX || ext.len() > EXT_MAX {
        COUNTER_TOLAK.fetch_add(1, Ordering::Relaxed);
        return TAK_SAH;
    }
    lock();
    let mut result = TAK_SAH;
    for (idx, s) in SLOTS.iter().enumerate() {
        if s.live.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
            // Fields are written before the handle is returned; `live` was
            // acquired under the global lock, so no other creator can observe
            // this slot in the middle of initialization.
            // Atomic-free arrays are safe here because the slot is exclusively
            // owned under the global creator lock until initialization ends.
            unsafe {
                let name_dst = s.name.get();
                (*name_dst) = [0; NAME_MAX];
                (&mut *name_dst)[..name.len()].copy_from_slice(name);
                let ext_dst = s.ext.get();
                (*ext_dst) = [0; EXT_MAX];
                (&mut *ext_dst)[..ext.len()].copy_from_slice(ext);
            }
            s.name_len.store(name.len() as u32, Ordering::Relaxed);
            s.ext_len.store(ext.len() as u32, Ordering::Relaxed);
            s.unique.store(unique, Ordering::Relaxed);
            let generation = s.gen.load(Ordering::Relaxed);
            result = ((generation as u64) << 32) | (idx as u64 + 1);
            COUNTER_BUAT.fetch_add(1, Ordering::Relaxed);
            break;
        }
    }
    unlock();
    if result == TAK_SAH {
        COUNTER_TOLAK.fetch_add(1, Ordering::Relaxed);
    }
    result
}

fn digits(unique: u64) -> u32 {
    let mut n = unique;
    let mut d = 0;
    while {
        n /= 10;
        n != 0
    } {
        d += 1;
    }
    d
}

fn write_decimal(mut value: u64, out: &mut [u8]) -> usize {
    let mut tmp = [0u8; 20];
    let mut n = 0;
    loop {
        tmp[n] = b'0' + (value % 10) as u8;
        n += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for i in 0..n {
        out[i] = tmp[n - i - 1];
    }
    n
}

fn serialize_name(s: &Slot, out: &mut [u8]) -> i32 {
    if out.len() < 8 {
        return -22;
    }
    let name_len = s.name_len.load(Ordering::Acquire) as usize;
    let unique = s.unique.load(Ordering::Acquire);
    let d = digits(unique) as usize;
    if name_len == 0 || d > name_len {
        return -22;
    }
    let prefix = name_len - d;
    let name = unsafe { &*s.name.get() };
    out[..prefix].copy_from_slice(&name[..prefix]);
    let mut decimal = [0u8; 20];
    let suffix_len = write_decimal(unique, &mut decimal);
    out[prefix] = b'~';
    out[prefix + 1..prefix + 1 + suffix_len].copy_from_slice(&decimal[..suffix_len]);
    let used = prefix + 1 + suffix_len;
    for b in &mut out[used..8] {
        *b = b' ';
    }
    8
}

fn serialize_ext(s: &Slot, out: &mut [u8]) -> i32 {
    if out.len() < 3 {
        return -22;
    }
    let n = s.ext_len.load(Ordering::Acquire) as usize;
    let ext = unsafe { &*s.ext.get() };
    out[..n].copy_from_slice(&ext[..n]);
    for b in &mut out[n..3] {
        *b = b' ';
    }
    3
}

/// Cermin `create_sfn_from_lfn` tanpa alokasi.
fn from_lfn(lfn: &[u8]) -> u64 {
    let mut start = 0;
    let mut end = lfn.len();
    while start < end && lfn[start] == b'.' {
        start += 1;
    }
    while end > start && lfn[end - 1] == b'.' {
        end -= 1;
    }
    let trimmed = &lfn[start..end];
    let last_dot = find_last_dot(trimmed);
    let mut out = [0u8; 256];
    let mut n = 0;
    for (i, &c) in trimmed.iter().enumerate() {
        if c == b' ' {
            continue;
        }
        if c == b'.' && Some(i) != last_dot {
            continue;
        }
        if n >= out.len() {
            return TAK_SAH;
        }
        out[n] = uppercase(c);
        n += 1;
    }
    for c in &mut out[..n] {
        if !valid_char(*c) && *c != b'.' {
            *c = b'_';
        }
    }
    let last_period = find_last_dot(&out[..n]);
    match last_period {
        None => make_slot(&out[..core::cmp::min(n, NAME_MAX)], &[], 1),
        Some(dot) => {
            let name_n = core::cmp::min(dot, NAME_MAX);
            let ext_n = core::cmp::min(n - dot - 1, EXT_MAX);
            make_slot(&out[..name_n], &out[dot + 1..dot + 1 + ext_n], 1)
        }
    }
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_buat(name: *const u8, name_len: usize, extension: *const u8, extension_len: usize, unique: u64) -> u64 {
    make_slot(unsafe { bytes(name, name_len) }, unsafe { bytes(extension, extension_len) }, unique)
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_dari_lfn(lfn: *const u8, lfn_len: usize) -> u64 {
    from_lfn(unsafe { bytes(lfn, lfn_len) })
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_lepas(handle: u64) -> i32 {
    let Some((idx, generation)) = split_handle(handle) else {
        COUNTER_TOLAK.fetch_add(1, Ordering::Relaxed);
        return -22;
    };
    let s = &SLOTS[idx];
    lock();
    let ok = s.live.load(Ordering::Acquire) && s.gen.load(Ordering::Acquire) == generation;
    if ok {
        s.live.store(false, Ordering::Release);
        s.gen.fetch_add(1, Ordering::Relaxed);
        COUNTER_LEPAS.fetch_add(1, Ordering::Relaxed);
    } else {
        COUNTER_TOLAK.fetch_add(1, Ordering::Relaxed);
    }
    unlock();
    if ok { 0 } else { -22 }
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_tambah_unique(handle: u64) -> i32 {
    let Some(s) = slot(handle) else {
        COUNTER_TOLAK.fetch_add(1, Ordering::Relaxed);
        return -22;
    };
    s.unique.fetch_add(1, Ordering::Relaxed);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_digits(handle: u64) -> u32 {
    slot(handle).map(|s| digits(s.unique.load(Ordering::Acquire))).unwrap_or(u32::MAX)
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_serialisasi_nama(handle: u64, out: *mut u8, cap: usize) -> i32 {
    let Some(s) = slot(handle) else { return -22; };
    if out.is_null() { return -22; }
    serialize_name(s, unsafe { core::slice::from_raw_parts_mut(out, cap) })
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_serialisasi_ekstensi(handle: u64, out: *mut u8, cap: usize) -> i32 {
    let Some(s) = slot(handle) else { return -22; };
    if out.is_null() { return -22; }
    serialize_ext(s, unsafe { core::slice::from_raw_parts_mut(out, cap) })
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_valid(sfn: *const u8, len: usize) -> bool {
    is_valid_sfn(unsafe { bytes(sfn, len) })
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_hidup() -> u32 {
    SLOTS.iter().filter(|s| s.live.load(Ordering::Acquire)).count() as u32
}

#[repr(C)]
pub struct KlatosSfnReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub hidup: u32,
    pub prod_buat: u32,
    pub prod_lepas: u32,
    pub prod_tolak: u32,
    pub reserved: u32,
    pub status: i32,
    pub reserved2: u32,
}

struct M { checks: u32, passed: u32 }
impl M {
    fn catat(&mut self, ok: bool) { self.checks += 1; if ok { self.passed += 1; } }
}

pub fn run_matrix() -> KlatosSfnReport {
    let mut m = M { checks: 0, passed: 0 };
    m.catat(!is_valid_sfn(b"foo.txt"));
    m.catat(!is_valid_sfn(b"FOO.txt"));
    m.catat(is_valid_sfn(b"FOO.TXT"));
    m.catat(is_valid_sfn(b"LONGNAME.TXT"));
    m.catat(!is_valid_sfn(b"VERYLONGNAME.TXT"));
    m.catat(!is_valid_sfn(b"LONGEXT.HTML"));
    m.catat(!is_valid_sfn(b"FOO."));
    m.catat(!is_valid_sfn(b"FOO.."));
    m.catat(!is_valid_sfn(b"FOO.BAR.TXT"));
    m.catat(is_valid_sfn(b"FOO BAR.TXT"));

    let expected = [
        (b"foo.txt".as_slice(), b"FOO~1   .TXT".as_slice()),
        (b"FOO.TXT".as_slice(), b"FOO~1   .TXT".as_slice()),
        (b"main.c.o".as_slice(), b"MAINC~1 .O  ".as_slice()),
        (b"longname.txt".as_slice(), b"LONGNA~1.TXT".as_slice()),
        (b"verylongname.txt".as_slice(), b"VERYLO~1.TXT".as_slice()),
        (b"longext.html".as_slice(), b"LONGEX~1.HTM".as_slice()),
        (b"foo.".as_slice(), b"FOO~1   .   ".as_slice()),
        (b"foo..".as_slice(), b"FOO~1   .   ".as_slice()),
        (b"foo...".as_slice(), b"FOO~1   .   ".as_slice()),
        (b"foo.bar.txt".as_slice(), b"FOOBAR~1.TXT".as_slice()),
        (b"foo bar.txt".as_slice(), b"FOOBAR~1.TXT".as_slice()),
        (b"support@klatos.org".as_slice(), b"SUPPOR~1.ORG".as_slice()),
    ];
    for (lfn, want) in expected {
        let h = from_lfn(lfn);
        let mut n = [0u8; 8];
        let mut e = [0u8; 3];
        let ok = h != TAK_SAH
            && serialize_name(slot(h).unwrap(), &mut n) == 8
            && serialize_ext(slot(h).unwrap(), &mut e) == 3
            && n == want[..8]
            && e == want[9..12]
            && klatos_sfn_lepas(h) == 0;
        m.catat(ok);
    }
    // State transition: the uniqueness suffix belongs to Rust, and the
    // serialized result changes after the C++ adapter requests an increment.
    let h = make_slot(b"FOO", b"TXT", 1);
    let mut name = [0u8; 8];
    let before = h != TAK_SAH
        && serialize_name(slot(h).unwrap(), &mut name) == 8
        && name == *b"FOO~1   ";
    m.catat(before);
    m.catat(klatos_sfn_tambah_unique(h) == 0);
    let after = serialize_name(slot(h).unwrap(), &mut name) == 8
        && name == *b"FOO~2   "
        && klatos_sfn_digits(h) == 0;
    m.catat(after);
    m.catat(klatos_sfn_lepas(h) == 0);

    // Stale/invalid handle and slot release are explicit negative cases.
    m.catat(klatos_sfn_lepas(TAK_SAH) < 0);
    m.catat(m.checks == m.passed);

    KlatosSfnReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        hidup: klatos_sfn_hidup(),
        prod_buat: COUNTER_BUAT.load(Ordering::Relaxed),
        prod_lepas: COUNTER_LEPAS.load(Ordering::Relaxed),
        prod_tolak: COUNTER_TOLAK.load(Ordering::Relaxed),
        reserved: 0,
        status: if m.checks == m.passed { 0 } else { 1 },
        reserved2: 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn klatos_sfn_selftest() -> KlatosSfnReport { run_matrix() }

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
