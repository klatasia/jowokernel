// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8.2-B — **SERIALISASI JALUR ABSOLUT CUSTODY (Rust)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosKustodiJalurReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4B4A_4C52; // "KLATKJLR"

/// Panjang jalur absolut maksimum. Cermin batas yang dipakai C++.
pub const MAX_JALUR: usize = 4096;

/// `EINVAL`.
pub const EINVAL: i32 = 22;
/// `ENOMEM`.
pub const ENOMEM: i32 = 12;
/// `ENAMETOOLONG`.
pub const ENAMETOOLONG: i32 = 36;

/// Jumlah pemanggilan C ABI (audit).
static PANGGIL: AtomicU32 = AtomicU32::new(0);

#[inline]
fn bump() {
    PANGGIL.fetch_add(1, Ordering::Relaxed);
}

/// Membentuk `&[u8]` dari `(ptr,len)`. `len == 0` selalu aman walau `ptr` null.
#[inline]
unsafe fn potong<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 || ptr.is_null() {
        &[]
    } else {
        core::slice::from_raw_parts(ptr, len)
    }
}

/// Komponen ke-`i` (urutan terdalam-dulu) dari rantai NUL-terpisah.
/// Mengembalikan `(offset, panjang)`. `None` bila rantai lebih pendek dari
/// `n_komponen`.
fn komponen(chain: &[u8], n: usize, i: usize) -> Option<(usize, usize)> {
    if i >= n {
        return None;
    }
    let mut off = 0usize;
    let mut idx = 0usize;
    // Lewati `i` pemisah NUL; bila pemisah habis lebih dulu, komponen ke-`i`
    // tidak ada (rantai lebih pendek dari `n`).
    while idx < i {
        let mut e = off;
        while e < chain.len() && chain[e] != 0 {
            e += 1;
        }
        if e >= chain.len() {
            return None;
        }
        off = e + 1;
        idx += 1;
    }
    let mut e = off;
    while e < chain.len() && chain[e] != 0 {
        e += 1;
    }
    Some((off, e - off))
}

/// Panjang jalur absolut yang dihasilkan dari rantai `(chain, n)`.
///
/// * `n == 0` atau `n == 1` (root saja) ⇒ `1` (jalur `"/"`).
/// * selain itu ⇒ jumlah `(1 + panjang_nama)` atas seluruh komponen BUKAN root
///   (root = komponen terakhir, dilewati).
///
/// `Err(EINVAL)` bila rantai tidak konsisten; `Err(ENAMETOOLONG)` hanya bila
/// panjang meluap secara aritmetika. `MAX_JALUR` dipertahankan sebagai ambang
/// historis/konstanta kontrak, bukan batas keras.
pub fn panjang(chain: &[u8], n: usize) -> Result<usize, i32> {
    if n <= 1 {
        return Ok(1);
    }
    // Komponen root (terakhir) WAJIB ada; rantai yang lebih pendek dari `n`
    // adalah malformed -> EINVAL.
    if komponen(chain, n, n - 1).is_none() {
        return Err(EINVAL);
    }
    let mut total = 0usize;
    let mut j = 0usize;
    // komponen 0..n-1 (BUKAN root, yaitu bukan komponen terakhir)
    while j + 1 < n {
        let (_, len) = komponen(chain, n, j).ok_or(EINVAL)?;
        // F-P8.2-C: panjang TIDAK dibatasi MAX_JALUR. Jalur absolut yang
        // dibentuk dari komponen custody nyata sah-sah saja melebihi PATH_MAX
        // (mis. getcwd/realpath setelah chdir berlapis); batas keras di sini
        // membuat jalur yang valid gagal dengan ENAMETOOLONG. Satu-satunya
        // batas yang tersisa adalah luapan aritmetika (ENAMETOOLONG).
        total = total.checked_add(1 + len).ok_or(ENAMETOOLONG)?;
        j += 1;
    }
    Ok(total)
}

/// Menulis jalur absolut hasil serialisasi ke `out` (kapasitas `out_cap`).
///
/// Mengembalikan jumlah byte yang ditulis (TIDAK termasuk terminator), atau
/// errno negatif. Bila `out_cap > panjang`, byte NUL terminator ditulis di
/// `out[panjang]`.
pub fn serialisasi(chain: &[u8], n: usize, out: &mut [u8]) -> Result<usize, i32> {
    let perlu = panjang(chain, n)?;
    if out.len() < perlu {
        return Err(ENAMETOOLONG);
    }
    if perlu == 1 {
        out[0] = b'/';
        if out.len() > 1 {
            out[1] = 0;
        }
        return Ok(1);
    }
    let mut w = 0usize;
    // Urutan keluaran root-dulu SETELAH root: j dari n-2 turun ke 0.
    let mut j = n - 1;
    while j > 0 {
        j -= 1;
        let (off, len) = komponen(chain, n, j).ok_or(EINVAL)?;
        out[w] = b'/';
        w += 1;
        out[w..w + len].copy_from_slice(&chain[off..off + len]);
        w += len;
    }
    if out.len() > perlu {
        out[perlu] = 0;
    }
    Ok(w)
}

/// Cermin transkripsi algoritma C++ HISTORIS, ditulis ulang di Rust HANYA
/// sebagai pembanding diferensial. Tidak dipakai jalur produksi.
///
/// Menulis hasil ke `out` dan mengembalikan jumlah byte (tanpa terminator).
pub fn serialisasi_lama(chain: &[u8], n: usize, out: &mut [u8]) -> Result<usize, i32> {
    // C++ historis: bila `!parent()` (root saja) -> "/". C++ tidak punya
    // `n_komponen`, ia menelusuri parent. Di sini root saja setara `n <= 1`.
    if n <= 1 {
        if out.is_empty() {
            return Err(ENAMETOOLONG);
        }
        out[0] = b'/';
        return Ok(1);
    }
    // path_length = sum(len(name)+1) atas SELURUH komponen (termasuk root "").
    let mut path_length = 0usize;
    let mut j = 0usize;
    while j < n {
        let (_, len) = komponen(chain, n, j).ok_or(EINVAL)?;
        path_length += len + 1;
        j += 1;
    }
    let hasil = path_length - 1;
    if out.len() < hasil {
        return Err(ENAMETOOLONG);
    }
    // Disusun dengan melewati komponen root (indeks 0 pada rantai root-dulu =
    // komponen terakhir di sini).
    let mut w = 0usize;
    let mut r = n - 1;
    while r > 0 {
        r -= 1;
        let (off, len) = komponen(chain, n, r).ok_or(EINVAL)?;
        out[w] = b'/';
        w += 1;
        out[w..w + len].copy_from_slice(&chain[off..off + len]);
        w += len;
    }
    Ok(w)
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

/// Panjang jalur absolut dari rantai `(ptr,len,n_komponen)`. `>= 1` atau errno.
#[no_mangle]
pub extern "C" fn klatos_kustodi_jalur_panjang(
    chain_ptr: *const u8,
    chain_len: usize,
    n_komponen: usize,
) -> isize {
    bump();
    let chain = unsafe { potong(chain_ptr, chain_len) };
    match panjang(chain, n_komponen) {
        Ok(l) => l as isize,
        Err(e) => -(e as isize),
    }
}

/// Serialisasi jalur absolut ke `out`. Jumlah byte ditulis, atau errno negatif.
#[no_mangle]
pub extern "C" fn klatos_kustodi_jalur(
    chain_ptr: *const u8,
    chain_len: usize,
    n_komponen: usize,
    out_ptr: *mut u8,
    out_cap: usize,
) -> isize {
    bump();
    let chain = unsafe { potong(chain_ptr, chain_len) };
    if out_ptr.is_null() || out_cap == 0 {
        return -(EINVAL as isize);
    }
    let out = unsafe { core::slice::from_raw_parts_mut(out_ptr, out_cap) };
    match serialisasi(chain, n_komponen, out) {
        Ok(l) => l as isize,
        Err(e) => -(e as isize),
    }
}

/// Batas jalur (cermin `MAX_JALUR`).
#[no_mangle]
pub extern "C" fn klatos_kustodi_jalur_max() -> usize {
    MAX_JALUR
}

/// Jumlah pemanggilan C ABI.
#[no_mangle]
pub extern "C" fn klatos_kustodi_jalur_panggil() -> u32 {
    PANGGIL.load(Ordering::Relaxed)
}

/// Reset penghitung.
#[no_mangle]
pub extern "C" fn klatos_kustodi_jalur_reset() -> u32 {
    PANGGIL.store(0, Ordering::Relaxed);
    1
}

// ---------------------------------------------------------------------------
// Laporan + selftest
// ---------------------------------------------------------------------------

/// Cermin `KlatosKustodiJalurReport` di `KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosKustodiJalurReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_panggil: u32,
    pub status: i32,
}

const _: [(); 40] = [(); core::mem::size_of::<KlatosKustodiJalurReport>()];

/// `true` bila hasil serialisasi sama dengan `expected`.
fn jalur_eq(chain: &[u8], n: usize, expected: &[u8]) -> bool {
    // Semua kasus self-test pendek; buffer kecil menghindari 4 KiB di stack.
    let mut buf = [0u8; 512];
    match serialisasi(chain, n, &mut buf) {
        Ok(l) => &buf[..l] == expected,
        Err(_) => false,
    }
}

/// Selftest mekanisme serialisasi jalur custody.
#[no_mangle]
pub extern "C" fn klatos_kustodi_jalur_selftest() -> KlatosKustodiJalurReport {
    let mut checks = 0u32;
    let mut passed = 0u32;
    let mut neg_checks = 0u32;
    let mut neg_passed = 0u32;
    let mut dif_checks = 0u32;
    let mut dif_beda = 0u32;

    macro_rules! ck {
        ($e:expr) => {{
            checks += 1;
            if $e {
                passed += 1;
            }
        }};
    }
    macro_rules! ng {
        ($e:expr) => {{
            checks += 1;
            neg_checks += 1;
            if $e {
                passed += 1;
                neg_passed += 1;
            }
        }};
    }

    // G1 — konstanta.
    ck!(REPORT_TAG == 0x4B4C_4154_4B4A_4C52);
    ck!(MAX_JALUR == 4096);
    ck!(EINVAL == 22 && ENOMEM == 12 && ENAMETOOLONG == 36);

    // G2 — root khusus.
    ck!(jalur_eq(b"", 1, b"/"));
    ck!(jalur_eq(b"", 0, b"/"));

    // G3 — satu tingkat: ["a", root("")].
    ck!(jalur_eq(b"a\0", 2, b"/a"));
    // G4 — dua tingkat: ["b","a",""] -> "/a/b".
    ck!(jalur_eq(b"b\0a\0", 3, b"/a/b"));
    // G5 — tiga tingkat: ["c","b","a",""] -> "/a/b/c".
    ck!(jalur_eq(b"c\0b\0a\0", 4, b"/a/b/c"));
    // G6 — nama dengan titik (bukan normalisasi; hanya serialisasi).
    ck!(jalur_eq(b"file.txt\0dir\0", 3, b"/dir/file.txt"));

    // G7 — panjang() konsisten.
    ck!(panjang(b"", 1) == Ok(1));
    ck!(panjang(b"a\0", 2) == Ok(2));
    ck!(panjang(b"b\0a\0", 3) == Ok(4));
    ck!(panjang(b"c\0b\0a\0", 4) == Ok(6));

    // G8 — negatif: batas & malformed.
    // out terlalu kecil -> ENAMETOOLONG.
    {
        let mut small = [0u8; 2];
        ng!(serialisasi(b"bb\0aa\0", 3, &mut small) == Err(ENAMETOOLONG));
    }
    // n melebihi jumlah komponen sebenarnya -> EINVAL.
    ng!(panjang(b"a\0", 3) == Err(EINVAL));
    // out null / cap 0.
    ng!(klatos_kustodi_jalur(b"a\0".as_ptr(), 2, 2, core::ptr::null_mut(), 0) < 0);
    // jalur melebihi MAX_JALUR TETAP sah (batas keras dihapus):
    // 5000 komponen "a" -> 4999 * (1 + 1) = 9998.
    {
        let mut big = [0u8; 12000];
        let mut w = 0usize;
        let mut k = 0usize;
        while k < 5000 {
            big[w] = b'a';
            w += 1;
            big[w] = 0;
            w += 1;
            k += 1;
        }
        ck!(panjang(&big[..w], 5000) == Ok(9998));
    }

    // G9 — diferensial >= 10000 terhadap transkripsi C++ historis.
    let mut state: u64 = 0x9E3779B97F4A7C15;
    let mut n_iter = 0u32;
    while n_iter < 12_000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let depth = ((state >> 33) % 6) as usize + 1; // 1..=6
        // Rantai terdalam-dulu: `depth-1` nama (0..=5 huruf) diikuti komponen
        // root kosong (direpresentasikan oleh NUL penutup terakhir).
        let mut chain = [0u8; 256];
        let mut w = 0usize;
        let mut k = 0usize;
        while k + 1 < depth {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let nm_len = ((state >> 40) % 6) as usize;
            let mut q = 0usize;
            while q < nm_len {
                chain[w] = b'a' + ((state >> (q * 3)) % 26) as u8;
                w += 1;
                q += 1;
            }
            chain[w] = 0;
            w += 1;
            k += 1;
        }
        let chain_slice = &chain[..w];

        let mut ref_buf = [0u8; 512];
        let mut got_buf = [0u8; 512];
        let r = serialisasi_lama(chain_slice, depth, &mut ref_buf);
        let g = serialisasi(chain_slice, depth, &mut got_buf);
        dif_checks += 1;
        checks += 1;
        let ok = match (r, g) {
            (Ok(rl), Ok(gl)) => rl == gl && ref_buf[..rl] == got_buf[..gl],
            _ => false,
        };
        if ok {
            passed += 1;
        } else {
            dif_beda += 1;
        }
        n_iter += 1;
    }

    let status = if checks == passed && dif_beda == 0 { 0 } else { -1 };
    KlatosKustodiJalurReport {
        tag: REPORT_TAG,
        checks,
        passed,
        neg_checks,
        neg_passed,
        dif_checks,
        dif_beda,
        prod_panggil: PANGGIL.load(Ordering::Relaxed),
        status,
    }
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4B4A_4C52);
        assert!(MAX_JALUR == 4096);
        assert!(core::mem::size_of::<KlatosKustodiJalurReport>() == 40);
    };
}
