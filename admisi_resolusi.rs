// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS ADMISI RESOLUSI PATH**.
//!


#![allow(unsafe_code)] // rustc classifies #[no_mangle] as unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

pub const EACCES: i32 = 13;
pub const ENOTDIR: i32 = 20;
pub const ELOOP: i32 = 35;
pub const REPORT_TAG: u64 = 0x4B4C_4154_5253_4F4C; // "KLATSOL"

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    ProdRecursi = 0,
    ProdKomponen = 1,
    ProdSimlink = 2,
}

const N_COUNTER: usize = 3;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Recursion admission, before any path inspection.
#[inline]
pub const fn resolusi_recursi_admisi(level: u32, limit: u32) -> i32 {
    if level >= limit { -ELOOP } else { 0 }
}

/// Per-component parent admission. ENOTDIR has precedence over EACCES.
#[inline]
pub const fn resolusi_komponen_admisi(parent_is_dir: u32, parent_may_execute: u32) -> i32 {
    if parent_is_dir == 0 {
        -ENOTDIR
    } else if parent_may_execute == 0 {
        -EACCES
    } else {
        0
    }
}

/// Terminal symlink policy.
///
/// Returns `-ELOOP` for terminal O_NOFOLLOW, `1` to stop and return the
/// symlink custody for O_NOFOLLOW_NOERROR, and `0` to continue/follow.
#[inline]
pub const fn resolusi_simlink_akhir_admisi(
    is_symlink: u32,
    have_more_parts: u32,
    nofollow: u32,
    nofollow_noerror: u32,
) -> i32 {
    if is_symlink == 0 || have_more_parts != 0 {
        0
    } else if nofollow != 0 {
        -ELOOP
    } else if nofollow_noerror != 0 {
        1
    } else {
        0
    }
}

fn naif_recursi(level: u32, limit: u32) -> i32 {
    match level.checked_sub(limit) {
        Some(_) => -ELOOP,
        None if level == limit => -ELOOP,
        None => 0,
    }
}

fn naif_komponen(dir: u32, exec: u32) -> i32 {
    let mut result = 0;
    if exec == 0 { result = -EACCES; }
    if dir == 0 { result = -ENOTDIR; }
    result
}

fn naif_simlink(is_link: u32, more: u32, nofollow: u32, noerror: u32) -> i32 {
    if is_link == 1 && more == 0 {
        if nofollow == 1 { return -ELOOP; }
        if noerror == 1 { return 1; }
    }
    0
}

fn resolusi_komponen_salah(parent_is_dir: u32, parent_may_execute: u32) -> i32 {
    if parent_may_execute == 0 { -EACCES }
    else if parent_is_dir == 0 { -ENOTDIR }
    else { 0 }
}

fn resolusi_simlink_salah(is_symlink: u32, have_more_parts: u32, nofollow: u32, noerror: u32) -> i32 {
    if is_symlink == 1 && have_more_parts == 0 && noerror != 0 { 1 }
    else if is_symlink == 1 && have_more_parts == 0 && nofollow != 0 { -ELOOP }
    else { 0 }
}

#[no_mangle]
pub extern "C" fn klatos_resolusi_recursi(level: u32, limit: u32) -> i32 {
    bump(Counter::ProdRecursi);
    resolusi_recursi_admisi(level, limit)
}

#[no_mangle]
pub extern "C" fn klatos_resolusi_komponen(parent_is_dir: u32, parent_may_execute: u32) -> i32 {
    bump(Counter::ProdKomponen);
    resolusi_komponen_admisi(parent_is_dir, parent_may_execute)
}

#[no_mangle]
pub extern "C" fn klatos_resolusi_simlink_akhir(
    is_symlink: u32,
    have_more_parts: u32,
    nofollow: u32,
    nofollow_noerror: u32,
) -> i32 {
    bump(Counter::ProdSimlink);
    resolusi_simlink_akhir_admisi(is_symlink, have_more_parts, nofollow, nofollow_noerror)
}

#[no_mangle]
pub extern "C" fn klatos_resolusi_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER { return 0; }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn klatos_resolusi_reset() -> u32 {
    for counter in COUNTERS.iter() { counter.store(0, Ordering::Release); }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiResolusiReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub recursi_checks: u32,
    pub recursi_passed: u32,
    pub komponen_checks: u32,
    pub komponen_passed: u32,
    pub simlink_checks: u32,
    pub simlink_passed: u32,
    pub prod_recursi: u32,
    pub prod_komponen: u32,
    pub prod_simlink: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group { Recursi, Komponen, Simlink }

struct Matrix { checks: u32, passed: u32, groups: [(u32, u32); 3] }
impl Matrix {
    const fn new() -> Self { Self { checks: 0, passed: 0, groups: [(0, 0); 3] } }
    fn catat(&mut self, value: bool, group: Group) {
        self.checks += 1;
        let pair = &mut self.groups[group as usize];
        pair.0 += 1;
        if value { self.passed += 1; pair.1 += 1; }
    }
}

/// Recursi 0..=10 (11) + komponen 2×2 (4) + simlink 2⁴ (16) = 31.
pub const DIFF_EXPECTED: u32 = 31;

fn run_matrix() -> KlatosAdmisiResolusiReport {
    let mut matrix = Matrix::new();
    let mut measured = 0u32;
    let mut different = 0u32;

    matrix.catat(REPORT_TAG == 0x4B4C_4154_5253_4F4C, Group::Recursi);
    matrix.catat(EACCES == 13 && ENOTDIR == 20 && ELOOP == 35, Group::Recursi);
    matrix.catat(resolusi_recursi_admisi(0, 8) == 0, Group::Recursi);
    matrix.catat(resolusi_recursi_admisi(7, 8) == 0, Group::Recursi);
    matrix.catat(resolusi_recursi_admisi(8, 8) == -ELOOP, Group::Recursi);
    matrix.catat(resolusi_recursi_admisi(9, 8) == -ELOOP, Group::Recursi);

    matrix.catat(resolusi_komponen_admisi(1, 1) == 0, Group::Komponen);
    matrix.catat(resolusi_komponen_admisi(0, 1) == -ENOTDIR, Group::Komponen);
    matrix.catat(resolusi_komponen_admisi(1, 0) == -EACCES, Group::Komponen);
    matrix.catat(resolusi_komponen_admisi(0, 0) == -ENOTDIR, Group::Komponen);
    matrix.catat(resolusi_komponen_admisi(0, 0) != resolusi_komponen_salah(0, 0), Group::Komponen);

    matrix.catat(resolusi_simlink_akhir_admisi(0, 0, 1, 1) == 0, Group::Simlink);
    matrix.catat(resolusi_simlink_akhir_admisi(1, 1, 1, 1) == 0, Group::Simlink);
    matrix.catat(resolusi_simlink_akhir_admisi(1, 0, 1, 1) == -ELOOP, Group::Simlink);
    matrix.catat(resolusi_simlink_akhir_admisi(1, 0, 0, 1) == 1, Group::Simlink);
    matrix.catat(resolusi_simlink_akhir_admisi(1, 0, 0, 0) == 0, Group::Simlink);
    matrix.catat(resolusi_simlink_akhir_admisi(0, 0, 1, 0) == 0, Group::Simlink);
    matrix.catat(resolusi_simlink_akhir_admisi(1, 0, 1, 1) != resolusi_simlink_salah(1, 0, 1, 1), Group::Simlink);
    matrix.catat(resolusi_simlink_akhir_admisi(1, 1, 1, 0) == resolusi_simlink_salah(1, 1, 1, 0), Group::Simlink);

    for level in 0..=10u32 {
        if resolusi_recursi_admisi(level, 8) != naif_recursi(level, 8) { different += 1; }
        measured += 1;
    }
    for dir in 0..2u32 {
        for exec in 0..2u32 {
            if resolusi_komponen_admisi(dir, exec) != naif_komponen(dir, exec) { different += 1; }
            measured += 1;
        }
    }
    for link in 0..2u32 {
        for more in 0..2u32 {
            for nofollow in 0..2u32 {
                for noerror in 0..2u32 {
                    if resolusi_simlink_akhir_admisi(link, more, nofollow, noerror)
                        != naif_simlink(link, more, nofollow, noerror) { different += 1; }
                    measured += 1;
                }
            }
        }
    }
    matrix.catat(measured == DIFF_EXPECTED, Group::Recursi);
    matrix.catat(different == 0, Group::Recursi);

    let status = if matrix.checks == matrix.passed { 0 } else { -1 };
    KlatosAdmisiResolusiReport {
        tag: REPORT_TAG,
        checks: matrix.checks,
        passed: matrix.passed,
        recursi_checks: matrix.groups[Group::Recursi as usize].0,
        recursi_passed: matrix.groups[Group::Recursi as usize].1,
        komponen_checks: matrix.groups[Group::Komponen as usize].0,
        komponen_passed: matrix.groups[Group::Komponen as usize].1,
        simlink_checks: matrix.groups[Group::Simlink as usize].0,
        simlink_passed: matrix.groups[Group::Simlink as usize].1,
        prod_recursi: klatos_resolusi_counter(Counter::ProdRecursi as u32),
        prod_komponen: klatos_resolusi_counter(Counter::ProdKomponen as u32),
        prod_simlink: klatos_resolusi_counter(Counter::ProdSimlink as u32),
        diferensial_diukur: measured,
        diferensial_beda: different,
        status,
    }
}

#[no_mangle]
pub extern "C" fn klatos_resolusi_selftest() -> KlatosAdmisiResolusiReport { run_matrix() }

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5253_4F4C);
        assert!(EACCES == 13 && ENOTDIR == 20 && ELOOP == 35);
        assert!(Counter::ProdRecursi as usize == 0);
        assert!(Counter::ProdKomponen as usize == 1);
        assert!(Counter::ProdSimlink as usize == 2);
        assert!(N_COUNTER == 3);
        assert!(DIFF_EXPECTED == 31);
        assert!(core::mem::size_of::<KlatosAdmisiResolusiReport>() == 64);
        assert!(core::mem::align_of::<KlatosAdmisiResolusiReport>() == 8);
    };
}
