// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT
//! P1.103 / REHOME-02b — **MEKANISME memori-bayangan KASAN** milik Rust.

#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Tag laporan; cermin `KlatosKasanBayanganReport` di `KlatSeam115.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4B53_4E42; // "KLATKSNB"

// --- verdict gerbang (cermin `keamanan_aksi`/`KlatSeam114.h`) ---------------
/// Lanjutkan pemeriksaan.
pub const KASAN_LANJUT: u32 = 0;
/// `size == 0` — lewati.
pub const KASAN_UKURAN_NOL: u32 = 1;
/// Belum diinisialisasi — lewati.
pub const KASAN_BELUM_INIT: u32 = 2;
/// Alamat di luar `[MAPPING_BASE, SHADOW_BASE)` — lewati.
pub const KASAN_DI_LUAR: u32 = 3;

// --- tata letak shadow ------------------------------------------------------
/// `log2(8)` — 8 byte nyata per 1 byte bayangan.
pub const SKALA_GESER: u32 = 3;
/// Skala shadow (8).
pub const SKALA: u64 = 8;
/// Topeng dalam-granula (7).
pub const TOPENG: u64 = 7;
/// Byte bayangan "8 byte bersih".
pub const BERSIH8: u8 = 0x00;

/// Penghitung audit (hanya jalur DINGIN — `check` TIDAK dihitung karena
/// dipanggil pada SETIAP akses memori ber-instrumentasi).
#[derive(Clone, Copy)]
pub enum Counter {
    Init = 0,
    Fill = 1,
    Mark = 2,
}
/// Jumlah penghitung.
pub const N_COUNTER: usize = 3;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

// --- keadaan (dulu `static` di AddressSanitizer.cpp) ------------------------
static SHADOW_BASE: AtomicU64 = AtomicU64::new(0);
static SHADOW_OFFSET: AtomicU64 = AtomicU64::new(0);
static MAPPING_BASE: AtomicU64 = AtomicU64::new(0);
static INITIALIZED: AtomicU32 = AtomicU32::new(0);

/// `va_to_shadow(a) = (a >> 3) + SHADOW_OFFSET` sebagai pointer byte.
#[inline]
fn va_to_shadow(address: u64) -> *mut u8 {
    let off = SHADOW_OFFSET.load(Ordering::Relaxed);
    ((address >> SKALA_GESER).wrapping_add(off)) as *mut u8
}

/// `round_up_to_power_of_two(v, 8)`.
#[inline]
const fn bulatkan(v: usize) -> usize {
    (v + 7) & !7
}

// --- inisialisasi -----------------------------------------------------------

/// `AddressSanitizer::init(shadow_base)`.
///
/// `kernel_mapping_base` = `g_boot_info.kernel_mapping_base` (C++ membacanya).
#[no_mangle]
pub extern "C" fn klatos_kasan_init(shadow_base: u64, kernel_mapping_base: u64) {
    bump(Counter::Init);
    SHADOW_BASE.store(shadow_base, Ordering::Release);
    MAPPING_BASE.store(kernel_mapping_base, Ordering::Release);
    SHADOW_OFFSET.store(
        shadow_base.wrapping_sub(kernel_mapping_base >> SKALA_GESER),
        Ordering::Release,
    );
    INITIALIZED.store(1, Ordering::Release);
}

/// Apakah KASAN sudah diinisialisasi.
#[no_mangle]
pub extern "C" fn klatos_kasan_initialized() -> u32 {
    INITIALIZED.load(Ordering::Acquire)
}

// --- mekanisme tulis --------------------------------------------------------

/// `fill_shadow(address, size, type)`.
#[no_mangle]
pub extern "C" fn klatos_kasan_fill(address: u64, size: usize, t: u32) {
    bump(Counter::Fill);
    if INITIALIZED.load(Ordering::Acquire) == 0 {
        return;
    }
    let p = va_to_shadow(address);
    // SAFETY: pemanggil (C++ `fill_shadow`/`__asan_*`) menjamin `address`/`size`
    // berada di dalam rentang pemetaan shadow yang sah dan `size % 8 == 0`.
    unsafe { core::ptr::write_bytes(p, t as u8, size >> SKALA_GESER) };
}

/// `mark_region(address, valid_size, total_size, type)`.
#[no_mangle]
pub extern "C" fn klatos_kasan_mark(address: u64, valid_size: usize, total_size: usize, t: u32) {
    bump(Counter::Mark);
    if INITIALIZED.load(Ordering::Acquire) == 0 {
        return;
    }
    let p = va_to_shadow(address);
    let valid_shadow_size = valid_size >> SKALA_GESER;
    // SAFETY: seperti `fill`; pemanggil menjamin `total_size % 8 == 0` dan
    // `total_size >= round_up(valid_size, 8)`.
    unsafe { core::ptr::write_bytes(p, BERSIH8, valid_shadow_size) };
    let unaligned = valid_size & (TOPENG as usize);
    if unaligned != 0 {
        // SAFETY: `valid_shadow_size` < panjang bayangan karena `valid_size > 0`.
        unsafe { *p.add(valid_shadow_size) = unaligned as u8 };
    }
    let poisoned = total_size.wrapping_sub(bulatkan(valid_size)) >> SKALA_GESER;
    let start = valid_shadow_size + if unaligned != 0 { 1 } else { 0 };
    // SAFETY: sama seperti di atas.
    unsafe { core::ptr::write_bytes(p.add(start), t as u8, poisoned) };
}

// --- mekanisme periksa ------------------------------------------------------

/// Pemeriksa satu byte (byte-ke-`address`).
#[inline]
fn cek_1b(address: u64, st: &mut u8) -> bool {
    // SAFETY: pemanggil `check` sudah melewati gerbang rentang.
    let shadow = unsafe { *va_to_shadow(address) };
    let minimal = ((address & TOPENG) + 1) as i8;
    if shadow == BERSIH8 || minimal <= (shadow as i8) {
        true
    } else {
        *st = shadow;
        false
    }
}

#[inline]
fn cek_2b(address: u64, st: &mut u8) -> bool {
    if (address >> SKALA_GESER) != ((address + 1) >> SKALA_GESER) {
        return cek_1b(address, st) && cek_1b(address + 1, st);
    }
    // SAFETY: seperti `cek_1b`.
    let shadow = unsafe { *va_to_shadow(address) };
    let minimal = (((address + 1) & TOPENG) + 1) as i8;
    if shadow == BERSIH8 || minimal <= (shadow as i8) {
        true
    } else {
        *st = shadow;
        false
    }
}

#[inline]
fn cek_4b(address: u64, st: &mut u8) -> bool {
    if (address >> SKALA_GESER) != ((address + 3) >> SKALA_GESER) {
        return cek_2b(address, st) && cek_2b(address + 2, st);
    }
    // SAFETY: seperti `cek_1b`.
    let shadow = unsafe { *va_to_shadow(address) };
    let minimal = (((address + 3) & TOPENG) + 1) as i8;
    if shadow == BERSIH8 || minimal <= (shadow as i8) {
        true
    } else {
        *st = shadow;
        false
    }
}

#[inline]
fn cek_8b(address: u64, st: &mut u8) -> bool {
    if (address >> SKALA_GESER) != ((address + 7) >> SKALA_GESER) {
        return cek_4b(address, st) && cek_4b(address + 4, st);
    }
    // SAFETY: seperti `cek_1b`.
    let shadow = unsafe { *va_to_shadow(address) };
    let minimal = (((address + 7) & TOPENG) + 1) as i8;
    if shadow == BERSIH8 || minimal <= (shadow as i8) {
        true
    } else {
        *st = shadow;
        false
    }
}

fn cek_nb(address: u64, n: usize, st: &mut u8) -> bool {
    let mut a = address;
    let mut n = n;
    while (a & TOPENG) != 0 && n > 0 {
        if !cek_1b(a, st) {
            return false;
        }
        a += 1;
        n -= 1;
    }
    while n >= 8 {
        if !cek_8b(a, st) {
            return false;
        }
        a += 8;
        n -= 8;
    }
    while n > 0 {
        if !cek_1b(a, st) {
            return false;
        }
        a += 1;
        n -= 1;
    }
    true
}

/// Cermin `KlatosKasanCheck` di `KlatSeam115.h` (3 `u32`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosKasanCheck {
    /// 1 = akses sah.
    pub valid: u32,
    /// Byte bayangan yang dilaporkan saat TIDAK sah (0 bila sah).
    pub bayangan: u32,
    /// Alasan gerbang: `KASAN_LANJUT`/`UKURAN_NOL`/`BELUM_INIT`/`DI_LUAR`.
    pub lewati: u32,
}

/// Gerbang awal `shadow_va_check` (urutannya PERSIS seperti C++).
#[inline]
fn gerbang(size: usize, address: u64) -> u32 {
    if size == 0 {
        return KASAN_UKURAN_NOL;
    }
    if INITIALIZED.load(Ordering::Acquire) == 0 {
        return KASAN_BELUM_INIT;
    }
    let mb = MAPPING_BASE.load(Ordering::Relaxed);
    let sb = SHADOW_BASE.load(Ordering::Relaxed);
    if address < mb || address >= sb {
        return KASAN_DI_LUAR;
    }
    KASAN_LANJUT
}

/// `shadow_va_check(address, size)`: gerbang + pemilihan + algoritma.
///
/// Tidak dihitung (`Counter`) karena berada di jalur panas SETIAP akses
/// memori ber-instrumentasi.
#[no_mangle]
pub extern "C" fn klatos_kasan_check(address: u64, size: usize) -> KlatosKasanCheck {
    let lewati = gerbang(size, address);
    if lewati != KASAN_LANJUT {
        return KlatosKasanCheck {
            valid: 0,
            bayangan: 0,
            lewati,
        };
    }
    let mut st: u8 = BERSIH8;
    let valid = match size {
        1 => cek_1b(address, &mut st),
        2 => cek_2b(address, &mut st),
        4 => cek_4b(address, &mut st),
        8 => cek_8b(address, &mut st),
        _ => cek_nb(address, size, &mut st),
    };
    KlatosKasanCheck {
        valid: valid as u32,
        bayangan: st as u32,
        lewati: KASAN_LANJUT,
    }
}

/// Membaca penghitung audit `i` (0 bila di luar rentang).
#[no_mangle]
pub extern "C" fn klatos_kasan_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Reset penghitung; mengembalikan jumlahnya.
#[no_mangle]
pub extern "C" fn klatos_kasan_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan
// ---------------------------------------------------------------------------

/// Laporan P1.103; cermin `KlatosKasanBayanganReport` di `KlatSeam115.h`.
#[repr(C)]
pub struct KlatosKasanBayanganReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Buffer bayangan sintetis untuk selftest host/kernel.
static mut SHADOW_UJI: [u8; 4096] = [0u8; 4096];

/// Oracle NAIF & INDEPENDEN: sebuah byte `a` sah bila `model[(a-base)>>3] == 0`
/// ATAU `(a & 7) as i8 < model[(a-base)>>3] as i8` (perbandingan BERTANDA).
/// Tidak memakai `cek_*b` sama sekali.
fn naif_valid(model: &[u8], base: u64, alamat: u64, size: usize) -> bool {
    let mut i = 0usize;
    while i < size {
        let a = alamat + i as u64;
        let b = model[((a - base) >> SKALA_GESER) as usize];
        let off = (a & TOPENG) as i8;
        if !(b == BERSIH8 || off < (b as i8)) {
            return false;
        }
        i += 1;
    }
    true
}

fn run() -> KlatosKasanBayanganReport {
    let mut c = 0u32;
    let mut p = 0u32;
    macro_rules! x {
        ($e:expr) => {{
            c += 1;
            if $e {
                p += 1;
            }
        }};
    }

    // Simpan keadaan nyata dan pulihkan di akhir (selftest TIDAK boleh
    // merusak KASAN kernel yang sudah aktif).
    let simpan = (
        SHADOW_BASE.load(Ordering::Relaxed),
        SHADOW_OFFSET.load(Ordering::Relaxed),
        MAPPING_BASE.load(Ordering::Relaxed),
        INITIALIZED.load(Ordering::Relaxed),
    );

    x!(REPORT_TAG == 0x4B4C_4154_4B53_4E42);
    x!(N_COUNTER == 3);
    x!(core::mem::size_of::<KlatosKasanBayanganReport>() == 32);
    x!(core::mem::size_of::<KlatosKasanCheck>() == 12);
    x!(SKALA == 8 && TOPENG == 7 && SKALA_GESER == 3);
    x!(bulatkan(0) == 0 && bulatkan(1) == 8 && bulatkan(8) == 8 && bulatkan(9) == 16);

    // --- pasang model ------------------------------------------------------
    let buf = core::ptr::addr_of_mut!(SHADOW_UJI) as *mut u8;
    let buf_addr = buf as u64;
    // Basis pemetaan SINTETIS. `gerbang()` menuntut `MAPPING_BASE <= a < SHADOW_BASE`,
    // jadi basis ini HARUS berada di bawah buffer bayangan, dan 8-sejajar supaya
    // granula `alamat & 7` jatuh di tempat yang diharapkan.
    //
    // Basis TETAP (0x1000_0000_0000) hanya kebetulan sah di host PIE, yang .bss-nya
    // berada di 0x55xx…; di kernel .bss berada jauh di bawah basis itu, sehingga
    // SETIAP pemeriksaan jatuh ke `KASAN_DI_LUAR` dan 8 dari 32 pemeriksaan gagal —
    // itulah yang membuat gerbang integrasi (otoritas 104) menolak modul ini dan
    // `rust_kernel_integrasi_selftest` mem-panic kernel. Menurunkannya dari alamat
    // buffer membuat matriks sah di kedua lingkungan.
    let mapping: u64 = (buf_addr - 4096) & !7u64;
    // Nolkan buffer supaya hasil deterministik (selftest dijalankan 2×).
    // SAFETY: buffer statis 4096 byte milik modul ini.
    unsafe { core::ptr::write_bytes(buf, 0, 4096) };
    klatos_kasan_init(buf_addr, mapping);

    x!(klatos_kasan_initialized() == 1);
    // va_to_shadow(mapping) == buf_addr
    x!(va_to_shadow(mapping) as u64 == buf_addr);

    // 128 byte daerah; isi bersih dulu.
    klatos_kasan_fill(mapping, 128, BERSIH8 as u32);
    // valid 20 byte dari total 128, penanda racun 0xFB (Malloc).
    klatos_kasan_mark(mapping, 20, 128, 0xFB);

    // Baca model: 24 byte bayangan pertama (diferensial menjangkau indeks 17).
    let mut model = [0u8; 24];
    // SAFETY: 24 byte pertama buffer (4096 byte).
    unsafe { core::ptr::copy_nonoverlapping(buf, model.as_mut_ptr(), 24) };

    // Byte bayangan yang DIHARAPKAN: [0,0,4,FB×13].
    x!(model[0] == 0);
    x!(model[1] == 0);
    x!(model[2] == 4);
    x!(model[3] == 0xFB);
    x!(model[15] == 0xFB);

    // --- gerbang -----------------------------------------------------------
    let g0 = klatos_kasan_check(mapping, 0);
    x!(g0.lewati == KASAN_UKURAN_NOL && g0.valid == 0);
    let g1 = klatos_kasan_check(mapping - 1, 1);
    x!(g1.lewati == KASAN_DI_LUAR);
    let g2 = klatos_kasan_check(buf_addr, 1);
    x!(g2.lewati == KASAN_DI_LUAR);
    let g3 = klatos_kasan_check(mapping, 1);
    x!(g3.lewati == KASAN_LANJUT && g3.valid == 1);

    // --- kasus eksplisit ---------------------------------------------------
    // 8 byte di +0 -> granula 0 bersih -> sah.
    x!(klatos_kasan_check(mapping, 8).valid == 1);
    // 1 byte di +16 -> granula 2 = 4 -> offset 0 < 4 -> sah.
    x!(klatos_kasan_check(mapping + 16, 1).valid == 1);
    // 4 byte di +16 -> minimal 4 <= 4 -> sah.
    x!(klatos_kasan_check(mapping + 16, 4).valid == 1);
    // 8 byte di +16 -> minimal 8 > 4 -> TIDAK sah, bayangan 4.
    let bad1 = klatos_kasan_check(mapping + 16, 8);
    x!(bad1.valid == 0 && bad1.bayangan == 4);
    // 1 byte di +24 -> granula 3 = 0xFB -> tidak sah.
    let bad2 = klatos_kasan_check(mapping + 24, 1);
    x!(bad2.valid == 0 && bad2.bayangan == 0xFB);
    // 8 byte di +24 -> tidak sah.
    x!(klatos_kasan_check(mapping + 24, 8).valid == 0);
    // akses tak selaras 4 byte di +17 -> menyeberang granula -> tidak sah.
    x!(klatos_kasan_check(mapping + 17, 4).valid == 0);

    // belum init -> lewati (dan fill/mark jadi no-op, tidak crash)
    INITIALIZED.store(0, Ordering::Release);
    x!(klatos_kasan_check(mapping, 1).lewati == KASAN_BELUM_INIT);
    klatos_kasan_fill(mapping, 8, 0);
    klatos_kasan_mark(mapping, 8, 8, 0);
    x!(klatos_kasan_initialized() == 0);
    INITIALIZED.store(1, Ordering::Release);

    // --- diferensial: cepat vs oracle naif --------------------------------
    let mut d = 0u32;
    let mut b = 0u32;
    macro_rules! q {
        ($got:expr, $exp:expr) => {{
            d += 1;
            if $got != $exp {
                b += 1;
            }
        }};
    }
    let sizes: [usize; 8] = [1, 2, 3, 4, 5, 7, 8, 16];
    let mut off = 0u64;
    while off < 128 {
        for size in sizes {
            if off + size as u64 > 128 {
                continue;
            }
            let a = mapping + off;
            let cepat = klatos_kasan_check(a, size).valid == 1;
            let naif = naif_valid(&model, mapping, a, size);
            q!(cepat, naif);
        }
        off += 1;
    }

    // --- mark_region varian unaligned -------------------------------------
    // valid 20 byte (unaligned 4) -> granula 2 = 4; total 32 -> racun 1 granula.
    unsafe { core::ptr::write_bytes(buf, 0, 4096) };
    klatos_kasan_mark(mapping, 20, 32, 0xFB);
    // SAFETY: baca 8 byte pertama.
    unsafe { core::ptr::copy_nonoverlapping(buf, model.as_mut_ptr(), 8) };
    x!(model[0] == 0 && model[1] == 0 && model[2] == 4 && model[3] == 0xFB);
    // diferensial kecil pada daerah ini
    let mut off2 = 0u64;
    while off2 < 32 {
        for size in [1usize, 2, 3, 4, 8] {
            if off2 + size as u64 > 32 {
                continue;
            }
            let a = mapping + off2;
            let cepat = klatos_kasan_check(a, size).valid == 1;
            let naif = naif_valid(&model, mapping, a, size);
            q!(cepat, naif);
        }
        off2 += 1;
    }

    // --- fill_shadow ------------------------------------------------------
    unsafe { core::ptr::write_bytes(buf, 0xFF, 4096) };
    klatos_kasan_fill(mapping, 24, BERSIH8 as u32);
    // SAFETY: baca 8 byte.
    unsafe { core::ptr::copy_nonoverlapping(buf, model.as_mut_ptr(), 8) };
    x!(model[0] == 0 && model[1] == 0 && model[2] == 0 && model[3] == 0xFF);
    x!(klatos_kasan_check(mapping + 16, 8).valid == 1);
    x!(klatos_kasan_check(mapping + 24, 1).valid == 0);

    x!(d == 1133);
    x!(b == 0);

    // Pulihkan keadaan nyata.
    SHADOW_BASE.store(simpan.0, Ordering::Release);
    SHADOW_OFFSET.store(simpan.1, Ordering::Release);
    MAPPING_BASE.store(simpan.2, Ordering::Release);
    INITIALIZED.store(simpan.3, Ordering::Release);

    KlatosKasanBayanganReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

/// Selftest P1.103.
#[no_mangle]
pub extern "C" fn klatos_kasan_bayangan_selftest() -> KlatosKasanBayanganReport {
    run()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4B53_4E42);
        assert!(N_COUNTER == 3);
        assert!(core::mem::size_of::<KlatosKasanBayanganReport>() == 32);
        assert!(core::mem::align_of::<KlatosKasanBayanganReport>() == 8);
        assert!(core::mem::size_of::<KlatosKasanCheck>() == 12);
        assert!(core::mem::align_of::<KlatosKasanCheck>() == 4);
        assert!(SKALA_GESER == 3 && SKALA == 8 && TOPENG == 7);
        assert!(Counter::Init as usize == 0);
        assert!(Counter::Mark as usize == 2);
        assert!(KASAN_LANJUT == 0 && KASAN_UKURAN_NOL == 1 && KASAN_BELUM_INIT == 2);
        assert!(KASAN_DI_LUAR == 3);
    };
}
