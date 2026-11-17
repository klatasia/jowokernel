// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas keadaan salin-saat-tulis / purge / volatile `AnonymousVMObject`


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosVmoAnonReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_564D_414E; // "KLATVMAN"

// --- pengkodean `handle_cow_fault` (cermin `PageFaultResponse`) -------------

/// Volatile ⇒ tulis ke memori volatile oleh pengguna adalah bug ⇒ crash.
pub const COW_CRASH: u32 = 0;
/// Tidak ada lagi yang berbagi halaman ⇒ remap r/w, tanpa salin.
pub const COW_REMAP_RW: u32 = 1;
/// Salin dari kolam halaman committed bersama (`SharedCommittedCowPages`).
pub const COW_DARI_SHARED: u32 = 2;
/// Salin dengan mengalokasikan halaman fisik baru.
pub const COW_DARI_ALOKASI: u32 = 3;

// --- pengkodean `set_volatile` ---------------------------------------------

/// `m_volatile == is_volatile` ⇒ tidak ada yang berubah.
pub const VOL_TRANSI_TANPA: u32 = 0;
/// Beralih menjadi volatile (melepas seluruh memori committed).
pub const VOL_TRANSI_KE_VOLATILE: u32 = 1;
/// Beralih kembali menjadi non-volatile (commit ulang halaman shared-zero).
pub const VOL_TRANSI_KE_NONVOLATILE: u32 = 2;

/// Penghitung untuk audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_vmoan_should_cow`.
    ProdShouldCow = 0,
    /// Jumlah `klatos_vmoan_should_cow` yang memutuskan "ya".
    ProdShouldCowBenar = 1,
    /// Jumlah `klatos_vmoan_halaman_cow`.
    ProdHalamanCow = 2,
    /// Jumlah `klatos_vmoan_purge_boleh`.
    ProdPurgeGate = 3,
    /// Jumlah `klatos_vmoan_volatile_transisi`.
    ProdVolatile = 4,
    /// Jumlah `klatos_vmoan_cow_fault`.
    ProdCowFault = 5,
    /// Jumlah keputusan `COW_CRASH` dari `klatos_vmoan_cow_fault`.
    ProdCowFaultCrash = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Keputusan `AnonymousVMObject::should_cow` (`AnonymousVMObject.cpp:300-308`).
///
/// * `page_ada` — slot halaman tidak null;
/// * `page_istimewa` — halaman shared-zero ATAU lazy-committed;
/// * `is_shared` — pemetaan region bersifat shared;
/// * `cow_map_null` — `m_cow_map.is_null()`;
/// * `cow_bit` — bit COW halaman (hanya bermakna bila map tidak null).
///
/// `1` (harus salin) / `0` (tidak).
pub const fn should_cow(
    page_ada: u32,
    page_istimewa: u32,
    is_shared: u32,
    cow_map_null: u32,
    cow_bit: u32,
) -> u32 {
    if page_ada != 0 && page_istimewa != 0 {
        return 1;
    }
    if is_shared != 0 {
        return 0;
    }
    if cow_map_null != 0 {
        return 0;
    }
    if cow_bit != 0 {
        1
    } else {
        0
    }
}

/// Predikat bersama `reset_cow_map` (`AnonymousVMObject.cpp:277`) dan hitungan
/// `try_clone` (`AnonymousVMObject.cpp:39-42`).
///
/// `1` bila halaman BUKAN shared-zero DAN BUKAN lazy-committed (yakni
/// benar-benar menempati halaman fisik yang perlu di-COW).
pub const fn halaman_dihitung_cow(shared_zero: u32, lazy_committed: u32) -> u32 {
    if shared_zero != 0 || lazy_committed != 0 {
        0
    } else {
        1
    }
}

/// Gerbang `purge` (`AnonymousVMObject.cpp:191`) dan pra-purge `try_clone`
/// (`AnonymousVMObject.cpp:25`): `1` hanya bila purgeable DAN volatile.
pub const fn purge_boleh(purgeable: u32, volatile_: u32) -> u32 {
    if purgeable != 0 && volatile_ != 0 {
        1
    } else {
        0
    }
}

/// Transisi `set_volatile` (`AnonymousVMObject.cpp:218-221`): `VOL_TRANSI_*`.
pub const fn volatile_transisi(sekarang: u32, baru: u32) -> u32 {
    if sekarang == baru {
        VOL_TRANSI_TANPA
    } else if baru != 0 {
        VOL_TRANSI_KE_VOLATILE
    } else {
        VOL_TRANSI_KE_NONVOLATILE
    }
}

/// `set_volatile` arah non-volatile (`AnonymousVMObject.cpp:243-253`): commit
/// diperlukan (`1`) bila ada halaman shared-zero yang harus dipulihkan.
pub const fn nonvolatile_perlu_commit(jumlah_shared_zero: u32) -> u32 {
    if jumlah_shared_zero == 0 {
        0
    } else {
        1
    }
}

/// Pohon keputusan `handle_cow_fault` (`AnonymousVMObject.cpp:328-365`).
///
/// Urutan persis C++: volatile ⇒ crash; refcount==1 ⇒ remap r/w; selain itu
/// COW (dari kolam committed bersama bila ada, jika tidak dari alokasi).
pub const fn cow_fault_keputusan(
    volatile_: u32,
    ref_count_satu: u32,
    shared_committed_ada: u32,
) -> u32 {
    if volatile_ != 0 {
        return COW_CRASH;
    }
    if ref_count_satu != 0 {
        return COW_REMAP_RW;
    }
    if shared_committed_ada != 0 {
        COW_DARI_SHARED
    } else {
        COW_DARI_ALOKASI
    }
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_vmoan_should_cow(
    page_ada: u32,
    page_istimewa: u32,
    is_shared: u32,
    cow_map_null: u32,
    cow_bit: u32,
) -> u32 {
    bump(Counter::ProdShouldCow);
    let r = should_cow(page_ada, page_istimewa, is_shared, cow_map_null, cow_bit);
    if r != 0 {
        bump(Counter::ProdShouldCowBenar);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_halaman_cow(shared_zero: u32, lazy_committed: u32) -> u32 {
    bump(Counter::ProdHalamanCow);
    halaman_dihitung_cow(shared_zero, lazy_committed)
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_purge_boleh(purgeable: u32, volatile_: u32) -> u32 {
    bump(Counter::ProdPurgeGate);
    purge_boleh(purgeable, volatile_)
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_volatile_transisi(sekarang: u32, baru: u32) -> u32 {
    bump(Counter::ProdVolatile);
    volatile_transisi(sekarang, baru)
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_nonvolatile_perlu_commit(jumlah_shared_zero: u32) -> u32 {
    nonvolatile_perlu_commit(jumlah_shared_zero)
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_cow_fault(
    volatile_: u32,
    ref_count_satu: u32,
    shared_committed_ada: u32,
) -> u32 {
    bump(Counter::ProdCowFault);
    let r = cow_fault_keputusan(volatile_, ref_count_satu, shared_committed_ada);
    if r == COW_CRASH {
        bump(Counter::ProdCowFaultCrash);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_counter(idx: u32) -> u32 {
    if (idx as usize) < N_COUNTER {
        COUNTERS[idx as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_vmoan_reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Laporan + selftest
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosVmoAnonReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_should_cow: u32,
    pub prod_should_cow_benar: u32,
    pub prod_halaman_cow: u32,
    pub prod_purge_gate: u32,
    pub prod_volatile: u32,
    pub prod_cow_fault: u32,
    pub prod_cow_fault_crash: u32,
}

const _: [(); 48] = [(); core::mem::size_of::<KlatosVmoAnonReport>()];

#[no_mangle]
pub extern "C" fn klatos_vmoan_selftest() -> KlatosVmoAnonReport {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;

    macro_rules! check {
        ($cond:expr) => {
            checks += 1;
            if $cond {
                passed += 1;
            }
        };
    }

    // G1 — konstanta
    check!(REPORT_TAG == 0x4B4C_4154_564D_414E);
    check!(COW_CRASH == 0);
    check!(COW_REMAP_RW == 1);
    check!(COW_DARI_SHARED == 2);
    check!(COW_DARI_ALOKASI == 3);
    check!(VOL_TRANSI_TANPA == 0);
    check!(VOL_TRANSI_KE_VOLATILE == 1);
    check!(VOL_TRANSI_KE_NONVOLATILE == 2);

    // G2 — should_cow
    // halaman istimewa (shared-zero/lazy) selalu COW
    check!(should_cow(1, 1, 0, 0, 0) == 1);
    check!(should_cow(1, 1, 1, 0, 0) == 1);
    check!(should_cow(1, 1, 1, 1, 0) == 1);
    // slot null ⇒ bukan istimewa
    check!(should_cow(0, 1, 0, 0, 0) == 0);
    // pemetaan shared ⇒ tidak pernah COW (setelah cek istimewa)
    check!(should_cow(1, 0, 1, 0, 1) == 0);
    // map null ⇒ tidak COW
    check!(should_cow(1, 0, 0, 1, 1) == 0);
    // map ada ⇒ ikuti bit
    check!(should_cow(1, 0, 0, 0, 1) == 1);
    check!(should_cow(1, 0, 0, 0, 0) == 0);

    // G3 — halaman_dihitung_cow
    check!(halaman_dihitung_cow(0, 0) == 1);
    check!(halaman_dihitung_cow(1, 0) == 0);
    check!(halaman_dihitung_cow(0, 1) == 0);
    check!(halaman_dihitung_cow(1, 1) == 0);

    // G4 — purge_boleh
    check!(purge_boleh(1, 1) == 1);
    check!(purge_boleh(1, 0) == 0);
    check!(purge_boleh(0, 1) == 0);
    check!(purge_boleh(0, 0) == 0);

    // G5 — volatile_transisi
    check!(volatile_transisi(0, 0) == VOL_TRANSI_TANPA);
    check!(volatile_transisi(1, 1) == VOL_TRANSI_TANPA);
    check!(volatile_transisi(0, 1) == VOL_TRANSI_KE_VOLATILE);
    check!(volatile_transisi(1, 0) == VOL_TRANSI_KE_NONVOLATILE);

    // G6 — nonvolatile_perlu_commit
    check!(nonvolatile_perlu_commit(0) == 0);
    check!(nonvolatile_perlu_commit(1) == 1);
    check!(nonvolatile_perlu_commit(u32::MAX) == 1);

    // G7 — cow_fault_keputusan: presedensi
    check!(cow_fault_keputusan(1, 1, 1) == COW_CRASH);
    check!(cow_fault_keputusan(1, 0, 0) == COW_CRASH);
    check!(cow_fault_keputusan(0, 1, 1) == COW_REMAP_RW);
    check!(cow_fault_keputusan(0, 1, 0) == COW_REMAP_RW);
    check!(cow_fault_keputusan(0, 0, 1) == COW_DARI_SHARED);
    check!(cow_fault_keputusan(0, 0, 0) == COW_DARI_ALOKASI);

    // G8 — diferensial 8192-iter should_cow
    let mut d_cow: u32 = 1;
    for i in 0u32..8192 {
        let pa = i & 1;
        let pi = (i >> 1) & 1;
        let sh = (i >> 2) & 1;
        let cn = (i >> 3) & 1;
        let cb = (i >> 4) & 1;
        let rust = should_cow(pa, pi, sh, cn, cb);
        let expected: u32 = if pa != 0 && pi != 0 {
            1
        } else if sh != 0 {
            0
        } else if cn != 0 {
            0
        } else {
            cb
        };
        if rust != expected {
            d_cow = 0;
        }
    }
    check!(d_cow == 1);

    // G9 — diferensial 8192-iter cow_fault_keputusan
    let mut d_pf: u32 = 1;
    for i in 0u32..8192 {
        let vo = i & 1;
        let rc = (i >> 1) & 1;
        let sc = (i >> 2) & 1;
        let rust = cow_fault_keputusan(vo, rc, sc);
        let expected: u32 = if vo != 0 {
            COW_CRASH
        } else if rc != 0 {
            COW_REMAP_RW
        } else if sc != 0 {
            COW_DARI_SHARED
        } else {
            COW_DARI_ALOKASI
        };
        if rust != expected {
            d_pf = 0;
        }
    }
    check!(d_pf == 1);

    // G10 — konsistensi gerbang purge == pra-purge clone
    let mut d_gate: u32 = 1;
    for i in 0u32..4 {
        let p = i & 1;
        let v = (i >> 1) & 1;
        if purge_boleh(p, v) != purge_boleh(p, v) {
            d_gate = 0;
        }
    }
    check!(d_gate == 1);

    // G11 — jumlah transisi volatile atas seluruh pasangan keadaan
    let mut n_transisi: u32 = 0;
    for s in 0u32..2 {
        for b in 0u32..2 {
            if volatile_transisi(s, b) != VOL_TRANSI_TANPA {
                n_transisi += 1;
            }
        }
    }
    check!(n_transisi == 2);

    let _ = passed;
    KlatosVmoAnonReport {
        tag: REPORT_TAG,
        checks,
        passed,
        prod_should_cow: COUNTERS[Counter::ProdShouldCow as usize].load(Ordering::Relaxed),
        prod_should_cow_benar: COUNTERS[Counter::ProdShouldCowBenar as usize].load(Ordering::Relaxed),
        prod_halaman_cow: COUNTERS[Counter::ProdHalamanCow as usize].load(Ordering::Relaxed),
        prod_purge_gate: COUNTERS[Counter::ProdPurgeGate as usize].load(Ordering::Relaxed),
        prod_volatile: COUNTERS[Counter::ProdVolatile as usize].load(Ordering::Relaxed),
        prod_cow_fault: COUNTERS[Counter::ProdCowFault as usize].load(Ordering::Relaxed),
        prod_cow_fault_crash: COUNTERS[Counter::ProdCowFaultCrash as usize].load(Ordering::Relaxed),
    }
}
