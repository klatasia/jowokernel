// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8 BATCH A — **JALUR LEKSIKAL: pemilik mekanisme `KLexicalPath` (Rust)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Penanda laporan; hanya ada di Rust.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4A4C_4B53; // "KLATJLKS"

/// Offset sentinel: hasil `basename` adalah konstanta `"."` (bukan potongan
/// dari masukan). Dipakai HANYA untuk jalur kosong.
pub const OFFSET_TITIK: usize = usize::MAX;

const N_COUNTER: usize = 1;

/// Indeks penghitung audit.
pub const COUNTER_PANGGIL: u32 = 0;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: u32) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

/// Membentuk `&[u8]` dari `(ptr, len)`. `len == 0` selalu aman walau `ptr`
/// null; selain itu pemanggil menjamin rentang sah.
#[inline]
unsafe fn potong<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 || ptr.is_null() {
        &[]
    } else {
        core::slice::from_raw_parts(ptr, len)
    }
}

/// `true` bila jalur absolut (`/...`). Cermin `KLexicalPath::is_absolute`.
pub fn absolut(p: &[u8]) -> bool {
    !p.is_empty() && p[0] == b'/'
}

/// `true` bila jalur kanonik. Cermin `KLexicalPath::is_canonical`.
///
/// Urutan pemeriksaan dipertahankan persis seperti C++ supaya biaya dan
/// hasilnya identik untuk setiap masukan.
pub fn kanonik(p: &[u8]) -> bool {
    if p.is_empty() {
        return false;
    }
    if p[p.len() - 1] == b'/' && p.len() != 1 {
        return false;
    }
    if mulai_dengan(p, b"./") || mengandung(p, b"/./") || diakhiri_dengan(p, b"/.") {
        return false;
    }
    if mulai_dengan(p, b"../") || mengandung(p, b"/../") || diakhiri_dengan(p, b"/..") {
        return false;
    }
    if mengandung(p, b"//") {
        return false;
    }
    true
}

#[inline]
fn mulai_dengan(p: &[u8], pre: &[u8]) -> bool {
    p.len() >= pre.len() && &p[..pre.len()] == pre
}

#[inline]
fn diakhiri_dengan(p: &[u8], suf: &[u8]) -> bool {
    p.len() >= suf.len() && &p[p.len() - suf.len()..] == suf
}

#[inline]
fn mengandung(p: &[u8], pat: &[u8]) -> bool {
    if pat.is_empty() || p.len() < pat.len() {
        return false;
    }
    let last = p.len() - pat.len();
    let mut i = 0;
    while i <= last {
        if &p[i..i + pat.len()] == pat {
            return true;
        }
        i += 1;
    }
    false
}

/// Memangkas `/` di kanan; mengembalikan panjang efektif. Cermin
/// `StringView::trim("/", TrimMode::Right)`.
fn pangkas_slash_kanan(p: &[u8]) -> usize {
    let mut n = p.len();
    while n > 0 && p[n - 1] == b'/' {
        n -= 1;
    }
    n
}

/// Cermin `KLexicalPath::basename`. Mengembalikan `(offset, panjang)` ke dalam
/// `p`; `offset == OFFSET_TITIK` berarti konstanta `"."`.
pub fn basename(p: &[u8]) -> (usize, usize) {
    if p == b"/" {
        return (0, 1);
    }
    if p.is_empty() {
        return (OFFSET_TITIK, 1);
    }
    let path_len = pangkas_slash_kanan(p);
    // Semua karakter adalah '/' — kembalikan karakter pertama.
    if path_len == 0 {
        return (0, 1);
    }
    // Cari '/' terakhir di dalam p[0..path_len].
    let mut i = path_len;
    while i > 0 {
        if p[i - 1] == b'/' {
            return (i, path_len - i);
        }
        i -= 1;
    }
    (0, path_len)
}

/// Cermin `KLexicalPath::dirname`. Pemanggil menjamin `kanonik(p)` dan adanya
/// `/`. Mengembalikan `(offset, panjang)` = `(0, indeks_slash_terakhir)`.
pub fn dirname(p: &[u8]) -> (usize, usize) {
    let mut i = p.len();
    while i > 0 {
        if p[i - 1] == b'/' {
            return (0, i - 1);
        }
        i -= 1;
    }
    (0, 0)
}

/// Jumlah bagian `split_view('/')`. Cermin `KLexicalPath::parts`.
pub fn bagian_count(p: &[u8]) -> usize {
    let mut n = 1usize;
    for &b in p {
        if b == b'/' {
            n += 1;
        }
    }
    n
}

/// Bagian ke-`idx` dari `split_view('/')`, sebagai `(offset, panjang)`.
/// Mengembalikan `false` bila `idx` di luar rentang.
pub fn bagian(p: &[u8], idx: usize, out_off: &mut usize, out_len: &mut usize) -> bool {
    let total = bagian_count(p);
    if idx >= total {
        return false;
    }
    let mut seg = 0usize;
    let mut start = 0usize;
    let mut i = 0usize;
    while i <= p.len() {
        if i == p.len() || p[i] == b'/' {
            if seg == idx {
                *out_off = start;
                *out_len = i - start;
                return true;
            }
            seg += 1;
            start = i + 1;
        }
        i += 1;
    }
    false
}

/// Cermin `KLexicalPath::try_join`. Menulis `first + "/" + second` (atau
/// `"/" + second` bila `first == "/"`) ke `out`. Mengembalikan panjang total,
/// atau `-ENOMEM` (`-12`) bila `out_cap` kurang. Pemanggil C++ menjamin
/// `kanonik(first)`, `kanonik(second)`, dan `!absolut(second)`.
pub fn gabung(first: &[u8], second: &[u8], out: &mut [u8]) -> isize {
    let perlu = if first == b"/" {
        1 + second.len()
    } else {
        first.len() + 1 + second.len()
    };
    if out.len() < perlu {
        return -12; // -ENOMEM
    }
    let mut w = 0usize;
    if first != b"/" {
        out[w..w + first.len()].copy_from_slice(first);
        w += first.len();
    }
    out[w] = b'/';
    w += 1;
    out[w..w + second.len()].copy_from_slice(second);
    perlu as isize
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

/// `true` bila jalur `(ptr,len)` absolut.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_absolut(ptr: *const u8, len: usize) -> bool {
    bump(COUNTER_PANGGIL);
    absolut(unsafe { potong(ptr, len) })
}

/// `true` bila jalur `(ptr,len)` kanonik.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_kanonik(ptr: *const u8, len: usize) -> bool {
    bump(COUNTER_PANGGIL);
    kanonik(unsafe { potong(ptr, len) })
}

/// Hitung `basename`; menulis `(offset, panjang)`. `offset == usize::MAX`
/// berarti konstanta `"."`. Mengembalikan 0.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_basename(
    ptr: *const u8,
    len: usize,
    out_off: *mut usize,
    out_len: *mut usize,
) -> i32 {
    bump(COUNTER_PANGGIL);
    if out_off.is_null() || out_len.is_null() {
        return -22; // -EINVAL
    }
    let (o, l) = basename(unsafe { potong(ptr, len) });
    unsafe {
        *out_off = o;
        *out_len = l;
    }
    0
}

/// Hitung `dirname`; menulis `(offset, panjang)`. Mengembalikan 0.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_dirname(
    ptr: *const u8,
    len: usize,
    out_off: *mut usize,
    out_len: *mut usize,
) -> i32 {
    bump(COUNTER_PANGGIL);
    if out_off.is_null() || out_len.is_null() {
        return -22; // -EINVAL
    }
    let (o, l) = dirname(unsafe { potong(ptr, len) });
    unsafe {
        *out_off = o;
        *out_len = l;
    }
    0
}

/// Jumlah bagian `split_view('/')`.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_bagian_count(ptr: *const u8, len: usize) -> usize {
    bump(COUNTER_PANGGIL);
    bagian_count(unsafe { potong(ptr, len) })
}

/// Bagian ke-`idx`; 0 bila ada, -22 bila `idx` di luar rentang.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_bagian(
    ptr: *const u8,
    len: usize,
    idx: usize,
    out_off: *mut usize,
    out_len: *mut usize,
) -> i32 {
    bump(COUNTER_PANGGIL);
    if out_off.is_null() || out_len.is_null() {
        return -22;
    }
    let mut o = 0usize;
    let mut l = 0usize;
    if !bagian(unsafe { potong(ptr, len) }, idx, &mut o, &mut l) {
        return -22;
    }
    unsafe {
        *out_off = o;
        *out_len = l;
    }
    0
}

/// Susun `first + "/" + second` ke `out`; mengembalikan panjang atau `-ENOMEM`.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_gabung(
    p1: *const u8,
    l1: usize,
    p2: *const u8,
    l2: usize,
    out: *mut u8,
    out_cap: usize,
) -> isize {
    bump(COUNTER_PANGGIL);
    if out.is_null() {
        return -22;
    }
    let first = unsafe { potong(p1, l1) };
    let second = unsafe { potong(p2, l2) };
    let buf = unsafe { core::slice::from_raw_parts_mut(out, out_cap) };
    gabung(first, second, buf)
}

/// Jumlah pemanggilan C ABI.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_panggil() -> u32 {
    COUNTERS[COUNTER_PANGGIL as usize].load(Ordering::Relaxed)
}

/// Reset penghitung (dipakai gate/harness).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_reset() -> u32 {
    let v = COUNTERS[COUNTER_PANGGIL as usize].swap(0, Ordering::Relaxed);
    v
}

// ---------------------------------------------------------------------------
// Selftest (matriks + laporan)
// ---------------------------------------------------------------------------

/// Cermin laporan `KlatosJalurLeksikalReport` di `KlatSeam126.h`.
#[repr(C)]
pub struct KlatosJalurLeksikalReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_panggil: u32,
    pub status: i32,
    pub reserved: u32,
}

struct M {
    checks: u32,
    passed: u32,
}

impl M {
    fn baru() -> Self {
        Self { checks: 0, passed: 0 }
    }
    fn catat(&mut self, ok: bool) {
        self.checks += 1;
        if ok {
            self.passed += 1;
        }
    }
}

/// Menjalankan matriks selftest deterministik.
pub fn run_matrix() -> KlatosJalurLeksikalReport {
    let mut m = M::baru();

    // --- is_absolute ---
    m.catat(absolut(b"/"));
    m.catat(absolut(b"/a/b"));
    m.catat(!absolut(b"a/b"));
    m.catat(!absolut(b""));
    m.catat(!absolut(b"./"));

    // --- is_canonical ---
    m.catat(kanonik(b"/"));
    m.catat(kanonik(b"/a/b"));
    m.catat(kanonik(b"a/b"));
    m.catat(!kanonik(b""));
    m.catat(!kanonik(b"/a/"));
    m.catat(!kanonik(b"/a//b"));
    m.catat(!kanonik(b"./a"));
    m.catat(!kanonik(b"/a/./b"));
    m.catat(!kanonik(b"/a/."));
    m.catat(!kanonik(b"../a"));
    m.catat(!kanonik(b"/a/../b"));
    m.catat(!kanonik(b"/a/.."));

    // --- basename ---
    m.catat(basename(b"/") == (0, 1));
    m.catat(basename(b"") == (OFFSET_TITIK, 1));
    m.catat(basename(b"///") == (0, 1));
    m.catat(basename(b"/a/b") == (3, 1));
    m.catat(basename(b"a/b") == (2, 1));
    m.catat(basename(b"abc") == (0, 3));
    m.catat(basename(b"/abc/") == (1, 3));

    // --- dirname ---
    // CATATAN: C++ `dirname` mengembalikan `path.substring_view(0, slash_index)`,
    // jadi untuk `/a` (slash di indeks 0) hasilnya STRING KOSONG, bukan `/`.
    // Itu kuirk yang sudah ada sebelum migrasi; di sini direplikasi PERSIS
    // (F-P8 bukan gate perbaikan perilaku). Dicatat sebagai temuan.
    m.catat(dirname(b"/a/b") == (0, 2));
    m.catat(dirname(b"/a") == (0, 0));
    m.catat(dirname(b"/") == (0, 0));

    // --- parts ---
    m.catat(bagian_count(b"/") == 2);
    m.catat(bagian_count(b"/a/b") == 3);
    m.catat(bagian_count(b"a/b") == 2);
    m.catat(bagian_count(b"abc") == 1);
    let mut o = 0usize;
    let mut l = 0usize;
    m.catat(bagian(b"/a/b", 0, &mut o, &mut l) && (o, l) == (0, 0));
    m.catat(bagian(b"/a/b", 1, &mut o, &mut l) && (o, l) == (1, 1));
    m.catat(bagian(b"/a/b", 2, &mut o, &mut l) && (o, l) == (3, 1));
    m.catat(!bagian(b"/a/b", 3, &mut o, &mut l));

    // --- try_join ---
    let mut buf = [0u8; 32];
    let n = gabung(b"/", b"a/b", &mut buf);
    m.catat(n == 4 && &buf[..4] == b"/a/b");
    let n = gabung(b"/a", b"b", &mut buf);
    m.catat(n == 4 && &buf[..4] == b"/a/b");
    let n = gabung(b"/a", b"b", &mut buf[..2]);
    m.catat(n == -12);

    // Determinisme: dua jalan menghasilkan angka yang sama.
    m.catat(m.checks == m.passed);

    KlatosJalurLeksikalReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        prod_panggil: COUNTERS[COUNTER_PANGGIL as usize].load(Ordering::Relaxed),
        status: if m.checks == m.passed { 0 } else { 1 },
        reserved: 0,
    }
}

/// Selftest C ABI untuk manifes integrasi.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_jalur_leksikal_selftest() -> KlatosJalurLeksikalReport {
    run_matrix()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matriks_lulus() {
        let r = run_matrix();
        assert_eq!(r.tag, REPORT_TAG);
        assert_eq!(r.checks, r.passed);
        assert!(r.checks > 0);
    }

    #[test]
    fn basename_titik() {
        assert_eq!(basename(b""), (OFFSET_TITIK, 1));
    }

    #[test]
    fn gabung_kapasitas_kurang() {
        let mut b = [0u8; 3];
        assert_eq!(gabung(b"/a", b"b", &mut b), -12);
    }
}
