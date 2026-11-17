// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas keadaan halaman `VMObject`/`InodeVMObject` + keputusan pemetaan


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosVmoInodeReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_564D_494E; // "KLATVMIN"

/// Penghitung untuk audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_vmoin_jumlah_halaman`.
    ProdJumlahHalaman = 0,
    /// Jumlah `klatos_vmoin_halaman_bersih`.
    ProdHalamanBersih = 1,
    /// Jumlah `klatos_vmoin_halaman_kotor`.
    ProdHalamanKotor = 2,
    /// Jumlah `klatos_vmoin_boleh_lepas`.
    ProdBolehLepas = 3,
    /// Jumlah `klatos_vmoin_peta_tulis`.
    ProdPetaTulis = 4,
    /// Jumlah `klatos_vmoin_peta_tulis_halaman`.
    ProdPetaTulisHalaman = 5,
    /// Jumlah `klatos_vmoin_kotor_saat_tulis`.
    ProdKotorSaatTulis = 6,
    /// Jumlah `klatos_vmoin_batas_lepas`.
    ProdBatasLepas = 7,
}

const N_COUNTER: usize = 8;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Jumlah halaman untuk `size` byte dengan ukuran halaman `ukuran_halaman`
/// (`VMObject::try_create_physical_pages`, `VMObject.cpp:34`).
///
/// Pembagian pembulatan ke atas. `ukuran_halaman == 0` ⇒ 0 (tidak pernah
/// terjadi di produksi; dijaga agar tidak membagi nol).
pub const fn jumlah_halaman(ukuran: u64, ukuran_halaman: u64) -> u64 {
    if ukuran_halaman == 0 {
        return 0;
    }
    (ukuran + ukuran_halaman - 1) / ukuran_halaman
}

/// Halaman "bersih" (`InodeVMObject::amount_clean`, `InodeVMObject.cpp:38`):
/// `1` bila TIDAK kotor DAN slot halaman ada.
pub const fn halaman_bersih(kotor: u32, ada: u32) -> u32 {
    if kotor == 0 && ada != 0 {
        1
    } else {
        0
    }
}

/// Halaman kotor (`InodeVMObject::amount_dirty`, `InodeVMObject.cpp:48`).
pub const fn halaman_kotor(kotor: u32) -> u32 {
    if kotor != 0 {
        1
    } else {
        0
    }
}

/// Predikat pelepasan halaman (`InodeVMObject::try_release_clean_pages`,
/// `InodeVMObject.cpp:77`): `1` bila TIDAK kotor DAN slot ada.
pub const fn boleh_lepas(kotor: u32, ada: u32) -> u32 {
    halaman_bersih(kotor, ada)
}

/// Writable efektif sebuah PTE (`Region::map_individual_page_impl`,
/// `Region.cpp:320`): `writeable && !(cow || dirty_on_write)`.
pub const fn peta_tulis(writeable: u32, cow: u32, kotor_saat_tulis: u32) -> u32 {
    if writeable != 0 && cow == 0 && kotor_saat_tulis == 0 {
        1
    } else {
        0
    }
}

/// Writable sebuah halaman saat pemetaan (`Region::map_individual_page_impl`,
/// `Region.cpp:292`): `writeable && !shared_zero && !lazy_committed`.
pub const fn peta_tulis_halaman(writeable: u32, shared_zero: u32, lazy_committed: u32) -> u32 {
    if writeable != 0 && shared_zero == 0 && lazy_committed == 0 {
        1
    } else {
        0
    }
}

/// `Region::should_dirty_on_write` (`Region.cpp:270-286`): `1` bila vmobject
/// adalah inode DAN halaman BELUM kotor (yakni penulisan harus menandainya
/// kotor). Bila bukan inode ⇒ `0`.
pub const fn kotor_saat_tulis(is_inode: u32, kotor: u32) -> u32 {
    if is_inode == 0 {
        return 0;
    }
    if kotor == 0 {
        1
    } else {
        0
    }
}

/// Batas jumlah halaman yang boleh dilepas dalam satu lintasan
/// (`InodeVMObject::try_release_clean_pages`, `InodeVMObject.cpp:76`):
/// `min(permintaan, jumlah_halaman)`.
pub const fn batas_lepas(permintaan: u64, jumlah_halaman_: u64) -> u64 {
    if permintaan < jumlah_halaman_ {
        permintaan
    } else {
        jumlah_halaman_
    }
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_vmoin_jumlah_halaman(ukuran: u64, ukuran_halaman: u64) -> u64 {
    bump(Counter::ProdJumlahHalaman);
    jumlah_halaman(ukuran, ukuran_halaman)
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_halaman_bersih(kotor: u32, ada: u32) -> u32 {
    bump(Counter::ProdHalamanBersih);
    halaman_bersih(kotor, ada)
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_halaman_kotor(kotor: u32) -> u32 {
    bump(Counter::ProdHalamanKotor);
    halaman_kotor(kotor)
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_boleh_lepas(kotor: u32, ada: u32) -> u32 {
    bump(Counter::ProdBolehLepas);
    boleh_lepas(kotor, ada)
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_peta_tulis(writeable: u32, cow: u32, kotor_saat_tulis_: u32) -> u32 {
    bump(Counter::ProdPetaTulis);
    peta_tulis(writeable, cow, kotor_saat_tulis_)
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_peta_tulis_halaman(writeable: u32, shared_zero: u32, lazy_committed: u32) -> u32 {
    bump(Counter::ProdPetaTulisHalaman);
    peta_tulis_halaman(writeable, shared_zero, lazy_committed)
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_kotor_saat_tulis(is_inode: u32, kotor: u32) -> u32 {
    bump(Counter::ProdKotorSaatTulis);
    kotor_saat_tulis(is_inode, kotor)
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_batas_lepas(permintaan: u64, jumlah_halaman_: u64) -> u64 {
    bump(Counter::ProdBatasLepas);
    batas_lepas(permintaan, jumlah_halaman_)
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_counter(idx: u32) -> u32 {
    if (idx as usize) < N_COUNTER {
        COUNTERS[idx as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_vmoin_reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Laporan + selftest
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosVmoInodeReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_jumlah_halaman: u32,
    pub prod_halaman_bersih: u32,
    pub prod_halaman_kotor: u32,
    pub prod_boleh_lepas: u32,
    pub prod_peta_tulis: u32,
    pub prod_peta_tulis_halaman: u32,
    pub prod_kotor_saat_tulis: u32,
    pub prod_batas_lepas: u32,
}

const _: [(); 48] = [(); core::mem::size_of::<KlatosVmoInodeReport>()];

#[no_mangle]
pub extern "C" fn klatos_vmoin_selftest() -> KlatosVmoInodeReport {
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
    check!(REPORT_TAG == 0x4B4C_4154_564D_494E);

    // G2 — jumlah_halaman (ceil_div)
    check!(jumlah_halaman(0, 4096) == 0);
    check!(jumlah_halaman(1, 4096) == 1);
    check!(jumlah_halaman(4096, 4096) == 1);
    check!(jumlah_halaman(4097, 4096) == 2);
    check!(jumlah_halaman(8192, 4096) == 2);
    check!(jumlah_halaman(1_000_000, 4096) == 245);
    check!(jumlah_halaman(123, 0) == 0);

    // G3 — halaman_bersih
    check!(halaman_bersih(0, 1) == 1);
    check!(halaman_bersih(1, 1) == 0);
    check!(halaman_bersih(0, 0) == 0);
    check!(halaman_bersih(1, 0) == 0);

    // G4 — halaman_kotor
    check!(halaman_kotor(0) == 0);
    check!(halaman_kotor(1) == 1);
    check!(halaman_kotor(0xFFFFFFFF) == 1);

    // G5 — boleh_lepas == halaman_bersih
    for k in 0u32..2 {
        for a in 0u32..2 {
            check!(boleh_lepas(k, a) == halaman_bersih(k, a));
        }
    }

    // G6 — peta_tulis: writeable && !cow && !dirty
    check!(peta_tulis(1, 0, 0) == 1);
    check!(peta_tulis(1, 1, 0) == 0);
    check!(peta_tulis(1, 0, 1) == 0);
    check!(peta_tulis(1, 1, 1) == 0);
    check!(peta_tulis(0, 0, 0) == 0);

    // G7 — peta_tulis_halaman
    check!(peta_tulis_halaman(1, 0, 0) == 1);
    check!(peta_tulis_halaman(1, 1, 0) == 0);
    check!(peta_tulis_halaman(1, 0, 1) == 0);
    check!(peta_tulis_halaman(0, 0, 0) == 0);

    // G8 — kotor_saat_tulis
    check!(kotor_saat_tulis(1, 0) == 1);
    check!(kotor_saat_tulis(1, 1) == 0);
    check!(kotor_saat_tulis(0, 0) == 0);
    check!(kotor_saat_tulis(0, 1) == 0);

    // G9 — batas_lepas
    check!(batas_lepas(0, 10) == 0);
    check!(batas_lepas(3, 10) == 3);
    check!(batas_lepas(10, 10) == 10);
    check!(batas_lepas(11, 10) == 10);
    check!(batas_lepas(u64::MAX, 10) == 10);

    // G10 — diferensial 4096-iter peta_tulis vs oracle eksplisit
    let mut d_pt: u32 = 1;
    for i in 0u32..4096 {
        let w = i & 1;
        let c = (i >> 1) & 1;
        let d = (i >> 2) & 1;
        let rust = peta_tulis(w, c, d);
        let expected: u32 = if w != 0 && c == 0 && d == 0 { 1 } else { 0 };
        if rust != expected {
            d_pt = 0;
        }
    }
    check!(d_pt == 1);

    // G11 — diferensial 4096-iter jumlah_halaman vs oracle loop
    let mut d_jh: u32 = 1;
    for i in 0u32..4096 {
        let ukuran = i as u64 * 1000;
        let rust = jumlah_halaman(ukuran, 4096);
        // Oracle: hitung manual dengan penambahan.
        let mut n: u64 = 0;
        let mut sisa = ukuran;
        while sisa > 0 {
            sisa = if sisa >= 4096 { sisa - 4096 } else { 0 };
            n += 1;
        }
        if rust != n {
            d_jh = 0;
        }
    }
    check!(d_jh == 1);

    let _ = passed;
    KlatosVmoInodeReport {
        tag: REPORT_TAG,
        checks,
        passed,
        prod_jumlah_halaman: COUNTERS[Counter::ProdJumlahHalaman as usize].load(Ordering::Relaxed),
        prod_halaman_bersih: COUNTERS[Counter::ProdHalamanBersih as usize].load(Ordering::Relaxed),
        prod_halaman_kotor: COUNTERS[Counter::ProdHalamanKotor as usize].load(Ordering::Relaxed),
        prod_boleh_lepas: COUNTERS[Counter::ProdBolehLepas as usize].load(Ordering::Relaxed),
        prod_peta_tulis: COUNTERS[Counter::ProdPetaTulis as usize].load(Ordering::Relaxed),
        prod_peta_tulis_halaman: COUNTERS[Counter::ProdPetaTulisHalaman as usize].load(Ordering::Relaxed),
        prod_kotor_saat_tulis: COUNTERS[Counter::ProdKotorSaatTulis as usize].load(Ordering::Relaxed),
        prod_batas_lepas: COUNTERS[Counter::ProdBatasLepas as usize].load(Ordering::Relaxed),
    }
}
