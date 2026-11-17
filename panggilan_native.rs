// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **GENERASI ABI NATIF KLATOS — otoritas rute + jembatan otoritas KSM.**


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosNativeReport` di sisi C++.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4E41_4249; // "KLATNABI"

/// Versi generasi natif; cermin `Jowo::AbiNative::version`.
pub const VERSION: u32 = 0x0003_0000;

/// Basis & masker ruang nomor natif; cermin `sc_base`/`sc_mask`.
pub const SC_BASE: u64 = 0x4B4C_0000;
pub const SC_MASK: u64 = 0xFFFF_0000;

// --- rute generasi ----------------------------------------------------------

/// `function` bukan milik generasi natif → jalur kompatibilitas ABI v1.
pub const RUTE_KOMPAT: u32 = 0;
/// `function` adalah panggilan natif pada domain yang DIKENAL.
pub const RUTE_NATIF: u32 = 1;
/// `function` adalah panggilan natif pada domain yang DICADANGKAN (belum ada).
pub const RUTE_DICADANGKAN: u32 = 2;
/// `function` adalah panggilan natif pada domain Otoritas (KSM).
pub const RUTE_OTORITAS: u32 = 3;

/// Domain natif; cermin `Jowo::AbiNative::Domain`.
pub const DOM_TUGAS: u32 = 0x1;
pub const DOM_MEMORI: u32 = 0x2;
pub const DOM_BERKAS: u32 = 0x3;
pub const DOM_IPC: u32 = 0x4;
pub const DOM_OTORITAS: u32 = 0x5;
pub const DOM_WAKTU: u32 = 0x6;
pub const DOM_PERANGKAT: u32 = 0x7;
pub const DOM_SISTEM: u32 = 0x8;

/// Status KSM; cermin `status` di [`crate::ksm_kapabilitas`].
pub const KSM_OK: i32 = 0;
pub const KSM_BAD_HANDLE: i32 = 1;
pub const KSM_MISSING_RIGHTS: i32 = 2;
pub const KSM_REVOKED: i32 = 3;
pub const KSM_BAD_OBJECT: i32 = 4;
pub const KSM_BAD_KIND: i32 = 5;
pub const KSM_CAPACITY: i32 = 6;
pub const KSM_INVALID: i32 = 7;
pub const KSM_BELUM_ADA: i32 = 8;

/// Apakah `function` berada di ruang nomor natif?
#[inline]
pub fn adalah_natif(function: u64) -> bool {
    (function & SC_MASK) == SC_BASE
}

/// Domain dari nomor natif.
#[inline]
pub fn domain(function: u64) -> u32 {
    ((function >> 12) & 0xF) as u32
}

/// Rute generasi untuk sebuah nomor panggilan.
///
/// Keputusan murni: tidak menyentuh tabel ABI v1, tidak menyentuh KSM.
#[inline]
pub fn rute(function: u64) -> u32 {
    if !adalah_natif(function) {
        return RUTE_KOMPAT;
    }
    match domain(function) {
        DOM_OTORITAS => RUTE_OTORITAS,
        DOM_TUGAS | DOM_MEMORI | DOM_BERKAS | DOM_IPC | DOM_WAKTU | DOM_PERANGKAT | DOM_SISTEM => {
            RUTE_NATIF
        }
        _ => RUTE_DICADANGKAN,
    }
}

// --- pintu C ABI ------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_native_versi() -> u32 {
    VERSION
}

#[no_mangle]
pub extern "C" fn klatos_native_adalah_natif(function: u64) -> u32 {
    if adalah_natif(function) { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn klatos_native_domain(function: u64) -> u32 {
    domain(function)
}

/// Rute generasi. Mengembalikan salah satu `RUTE_*`.
#[no_mangle]
pub extern "C" fn klatos_native_rute(function: u64) -> u32 {
    let r = rute(function);
    bump(RuteCounter::Dipanggil);
    match r {
        RUTE_KOMPAT => bump(RuteCounter::Kompat),
        RUTE_NATIF => bump(RuteCounter::Natif),
        RUTE_DICADANGKAN => bump(RuteCounter::Dicadangkan),
        RUTE_OTORITAS => bump(RuteCounter::Otoritas),
        _ => {}
    }
    r
}

/// **Otoritas KSM** — apakah `task` memegang `hak` atas `handle`?
///
/// Ini pengganti natif untuk `pledge`/`unveil`: pertanyaan capability, bukan
/// janji. Delegasi penuh ke [`crate::ksm_kapabilitas::klatos_ksm_kap_periksa`].
#[no_mangle]
pub extern "C" fn klatos_native_otoritas_tanya(task: u32, handle: u32, hak: u32) -> i32 {
    let out = crate::ksm_kapabilitas::klatos_ksm_kap_periksa(task, handle, hak);
    bump(RuteCounter::OtoritasTanya);
    if out.status != KSM_OK {
        bump(RuteCounter::OtoritasTolak);
    }
    out.status
}

/// Persempit hak capability (intersection; hak tidak pernah tumbuh).
#[no_mangle]
pub extern "C" fn klatos_native_otoritas_batas(task: u32, handle: u32, hak: u32) -> i32 {
    crate::ksm_kapabilitas::klatos_ksm_kap_sempitkan(task, handle, hak).status
}

/// Serahkan capability ke tugas lain; sumber kehilangan handle.
#[no_mangle]
pub extern "C" fn klatos_native_otoritas_serah(
    task: u32,
    handle: u32,
    pid_tujuan: u32,
    handle_tujuan: u32,
    hak: u32,
) -> i32 {
    crate::ksm_kapabilitas::klatos_ksm_kap_pindah(task, handle, pid_tujuan, handle_tujuan, hak)
}

/// Cabut capability object secara global.
#[no_mangle]
pub extern "C" fn klatos_native_otoritas_cabut(task: u32, handle: u32) -> i32 {
    let cap = crate::ksm_kapabilitas::klatos_ksm_kap_lihat(task, handle);
    if cap.status != KSM_OK {
        return cap.status;
    }
    crate::ksm_kapabilitas::klatos_ksm_kap_cabut(cap.id)
}

// --- penghitung & selftest --------------------------------------------------

#[derive(Clone, Copy)]
enum RuteCounter {
    Dipanggil = 0,
    Kompat = 1,
    Natif = 2,
    Dicadangkan = 3,
    Otoritas = 4,
    OtoritasTanya = 5,
    OtoritasTolak = 6,
}

const N_COUNTER: usize = 7;
static COUNTERS: [AtomicU32; N_COUNTER] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

#[inline]
fn bump(c: RuteCounter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn klatos_native_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTERS[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_native_reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Relaxed);
    }
}

/// Laporan selftest; cermin `KlatosNativeReport` di C++.
#[repr(C)]
pub struct NativeReport {
    pub tag: u64,
    pub versi: u32,
    pub checks: u32,
    pub passed: u32,
    pub failed: u32,
    pub status: i32,
}

struct Checker {
    checks: u32,
    passed: u32,
}

impl Checker {
    fn new() -> Self {
        Self { checks: 0, passed: 0 }
    }
    fn check(&mut self, ok: bool) {
        self.checks += 1;
        if ok {
            self.passed += 1;
        }
    }
}

/// Selftest mandiri: rute generasi + pemisahan ABI v1 + otoritas KSM.
#[no_mangle]
pub extern "C" fn klatos_native_selftest() -> NativeReport {
    let mut c = Checker::new();

    // §1 identitas & ruang nomor
    c.check(VERSION == 0x0003_0000);
    c.check(adalah_natif(0x4B4C_5000));
    c.check(!adalah_natif(0));
    c.check(!adalah_natif(163));
    c.check(!adalah_natif(89)); // pledge (kompat) bukan natif
    c.check(!adalah_natif(151)); // unveil (kompat) bukan natif

    // §2 rute: v1 → kompat, natif otoritas → otoritas, domain lain → natif,
    //     domain tak dikenal → dicadangkan
    c.check(rute(0) == RUTE_KOMPAT);
    c.check(rute(163) == RUTE_KOMPAT);
    c.check(rute(89) == RUTE_KOMPAT);
    c.check(rute(0x4B4C_5000) == RUTE_OTORITAS);
    c.check(rute(0x4B4C_1000) == RUTE_NATIF);
    c.check(rute(0x4B4C_2000) == RUTE_NATIF);
    c.check(rute(0x4B4C_F000) == RUTE_DICADANGKAN);

    // §3 domain
    c.check(domain(0x4B4C_5000) == DOM_OTORITAS);
    c.check(domain(0x4B4C_1000) == DOM_TUGAS);
    c.check(domain(0x4B4C_8000) == DOM_SISTEM);

    // §4 otoritas KSM — handle tak dikenal ditolak (negatif, fail-closed)
    let bad = klatos_native_otoritas_tanya(0xFFFF_FFFE, 0xFFFF_FFFF, crate::ksm_kapabilitas::right::ALL);
    c.check(bad == KSM_BAD_HANDLE);

    let checks = c.checks;
    let passed = c.passed;
    let failed = checks - passed;
    NativeReport {
        tag: REPORT_TAG,
        versi: VERSION,
        checks,
        passed,
        failed,
        status: if failed == 0 { 0 } else { 1 },
    }
}
