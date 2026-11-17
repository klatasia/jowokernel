// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! F-P8 BATCH A — **WILAYAH FISIK: kebijakan tata-letak zona (Rust)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Penanda laporan; hanya ada di Rust.
pub const REPORT_TAG: u64 = 0x4B4C_4154_574C_4653; // "KLATWLFS"

const N_COUNTER: usize = 1;

/// Indeks penghitung audit.
pub const COUNTER_PANGGIL: u32 = 0;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: u32) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

/// Pembulatan ke pangkat dua berikutnya (u32). Cermin
/// `PhysicalRegion.cpp::next_power_of_two`.
pub fn next_power_of_two(mut value: u32) -> u32 {
    value = value.wrapping_sub(1);
    value |= value >> 1;
    value |= value >> 2;
    value |= value >> 4;
    value |= value >> 8;
    value |= value >> 16;
    value = value.wrapping_add(1);
    value
}

/// Order buddy untuk `count` halaman. Cermin
/// `next_power_of_two(count)` + `count_trailing_zeroes(...)`.
pub fn order_untuk(count: u32) -> u32 {
    next_power_of_two(count).trailing_zeros()
}

/// Rencana partisi zona. Cermin `PhysicalRegion::initialize_zones`.
///
/// Menulis `(base_page, page_count)` ke `out_base`/`out_pages`. Mengembalikan
/// jumlah zona; `n_zona > cap` berarti `-ENOSPC`.
pub fn rencana_zona(
    lower_page: u64,
    total_pages: u64,
    large_pages: u64,
    small_pages: u64,
    out_base: &mut [u64],
    out_pages: &mut [u64],
) -> (usize, u32, i32) {
    let mut base_page = lower_page;
    let mut remaining = total_pages;
    let mut n = 0usize;
    let mut large_zones = 0u32;

    // Zona besar dulu (16 MiB), lalu zona kecil (1 MiB).
    for (idx, size_pages) in [large_pages, small_pages].iter().enumerate() {
        if *size_pages == 0 {
            continue;
        }
        let mut count = 0u32;
        while remaining >= *size_pages {
            if n >= out_base.len() || n >= out_pages.len() {
                return (n, large_zones, -28); // -ENOSPC
            }
            out_base[n] = base_page;
            out_pages[n] = *size_pages;
            n += 1;
            base_page += *size_pages;
            remaining -= *size_pages;
            count += 1;
        }
        if idx == 0 {
            large_zones = count;
        }
    }
    (n, large_zones, 0)
}

/// Indeks zona yang memuat `paddr`. Cermin `PhysicalRegion::return_page`.
///
/// Semua argumen dalam BYTE. Mengembalikan indeks zona, atau -1 bila tidak
/// ada zona yang cocok.
pub fn indeks_zona(
    paddr: u64,
    lower: u64,
    large_zones: u32,
    large_zone_size: u64,
    small_zone_size: u64,
) -> i64 {
    let large_zone_base = lower;
    let small_zone_base = lower + (large_zones as u64) * large_zone_size;
    if paddr < small_zone_base {
        if large_zone_size == 0 {
            return -1;
        }
        ((paddr - large_zone_base) / large_zone_size) as i64
    } else {
        if small_zone_size == 0 {
            return -1;
        }
        (large_zones as u64 + (paddr - small_zone_base) / small_zone_size) as i64
    }
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

/// Cermin `KlatosRencanaZona` di `KlatSeam127.h`.
#[repr(C)]
pub struct KlatosRencanaZona {
    pub n_zona: u32,
    pub large_zones: u32,
    pub status: i32,
}

/// Order buddy untuk `count` halaman.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_wfisik_order(count: u32) -> u32 {
    bump(COUNTER_PANGGIL);
    order_untuk(count)
}

/// Hitung rencana zona; menulis ke `out_base`/`out_pages` (kapasitas `cap`).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_wfisik_rencana(
    lower_page: u64,
    total_pages: u64,
    large_pages: u64,
    small_pages: u64,
    out_base: *mut u64,
    out_pages: *mut u64,
    cap: u32,
) -> KlatosRencanaZona {
    bump(COUNTER_PANGGIL);
    if out_base.is_null() || out_pages.is_null() {
        return KlatosRencanaZona { n_zona: 0, large_zones: 0, status: -22 };
    }
    let bases = unsafe { core::slice::from_raw_parts_mut(out_base, cap as usize) };
    let pages = unsafe { core::slice::from_raw_parts_mut(out_pages, cap as usize) };
    let (n, large, st) = rencana_zona(lower_page, total_pages, large_pages, small_pages, bases, pages);
    KlatosRencanaZona { n_zona: n as u32, large_zones: large, status: st }
}

/// Indeks zona yang memuat `paddr` (byte).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_wfisik_indeks(
    paddr: u64,
    lower: u64,
    large_zones: u32,
    large_zone_size: u64,
    small_zone_size: u64,
) -> i64 {
    bump(COUNTER_PANGGIL);
    indeks_zona(paddr, lower, large_zones, large_zone_size, small_zone_size)
}

/// Jumlah pemanggilan C ABI.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_wfisik_panggil() -> u32 {
    COUNTERS[COUNTER_PANGGIL as usize].load(Ordering::Relaxed)
}

/// Reset penghitung.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_wfisik_reset() -> u32 {
    COUNTERS[COUNTER_PANGGIL as usize].swap(0, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Selftest
// ---------------------------------------------------------------------------

/// Cermin laporan `KlatosWilayahFisikReport` di `KlatSeam127.h`.
#[repr(C)]
pub struct KlatosWilayahFisikReport {
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

/// Matriks selftest deterministik.
pub fn run_matrix() -> KlatosWilayahFisikReport {
    let mut m = M::baru();

    // --- next_power_of_two / order ---
    m.catat(next_power_of_two(1) == 1);
    m.catat(next_power_of_two(2) == 2);
    m.catat(next_power_of_two(3) == 4);
    m.catat(next_power_of_two(5) == 8);
    m.catat(next_power_of_two(4096) == 4096);
    m.catat(order_untuk(1) == 0);
    m.catat(order_untuk(2) == 1);
    m.catat(order_untuk(3) == 2);
    m.catat(order_untuk(4) == 2);
    m.catat(order_untuk(5) == 3);
    m.catat(order_untuk(4096) == 12);

    // --- rencana zona ---
    // 16 MiB = 4096 halaman; 1 MiB = 256 halaman.
    let mut bases = [0u64; 128];
    let mut pages = [0u64; 128];
    // 1 GiB = 262144 halaman = 64 zona 16MiB tepat, tanpa sisa.
    let (n, large, st) = rencana_zona(0, 262144, 4096, 256, &mut bases, &mut pages);
    m.catat(st == 0);
    m.catat(n == 64 && large == 64);
    m.catat(bases[0] == 0 && pages[0] == 4096);
    m.catat(bases[63] == 63 * 4096 && pages[63] == 4096);
    // 1 GiB + 512 KiB: 64 zona besar + 2 zona kecil.
    let (n, large, st) = rencana_zona(0, 262144 + 512, 4096, 256, &mut bases, &mut pages);
    m.catat(st == 0);
    m.catat(n == 66 && large == 64);
    m.catat(bases[64] == 262144 && pages[64] == 256);
    m.catat(bases[65] == 262144 + 256 && pages[65] == 256);
    // Kapasitas kurang -> -ENOSPC.
    let (n2, _l, st2) = rencana_zona(0, 262144, 4096, 256, &mut bases[..4], &mut pages[..4]);
    m.catat(st2 == -28 && n2 == 4);
    // Sisa di bawah 1 MiB tidak menjadi zona.
    let (n3, l3, st3) = rencana_zona(0, 255, 4096, 256, &mut bases, &mut pages);
    m.catat(st3 == 0 && n3 == 0 && l3 == 0);

    // --- indeks zona ---
    let large_size = 4096 * 4096u64; // 16 MiB
    let small_size = 256 * 4096u64; // 1 MiB
    let lower = 0u64;
    let large_zones = 64u32;
    let small_base = large_zones as u64 * large_size;
    m.catat(indeks_zona(0, lower, large_zones, large_size, small_size) == 0);
    m.catat(indeks_zona(large_size, lower, large_zones, large_size, small_size) == 1);
    m.catat(indeks_zona(large_size * 63, lower, large_zones, large_size, small_size) == 63);
    m.catat(indeks_zona(small_base, lower, large_zones, large_size, small_size) == 64);
    m.catat(indeks_zona(small_base + small_size, lower, large_zones, large_size, small_size) == 65);

    m.catat(m.checks == m.passed);

    KlatosWilayahFisikReport {
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
pub extern "C" fn klatos_wfisik_selftest() -> KlatosWilayahFisikReport {
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
    }
}
