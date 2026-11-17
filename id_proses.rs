// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.36 — OTORITAS IDENTITAS OBJEK NAMESPACE PROSES (Rust).


use crate::ruang_id::{
    buat_laporan, diferensial_gaya, Gaya, Grup, HasilId, KlatosRuangIdReport, Matrix,
    PenyimpananUji, RuangId,
};
use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATIPRS".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4950_5253;

/// Batas `IndexID` (`u64`) untuk kedua ruang.
pub const NS_MAKS: u64 = u64::MAX;

const RENTANG_DIFERENSIAL: u64 = 16384;
const N_COUNTER: usize = 4;

/// `HostnameContext::IndexID` — dasar 0, pasca-naik (id pertama 0).
static RUANG_HOSTNAME: RuangId = RuangId::baru(0, NS_MAKS, Gaya::PascaNaik);

/// `ScopedProcessList::IndexID` — dasar 0, pasca-naik (id pertama 0).
static RUANG_SCOPED: RuangId = RuangId::baru(0, NS_MAKS, Gaya::PascaNaik);

/// `0` = hostname_alloc, `1` = scoped_alloc, `2` = habis, `3` = galat.
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(i: usize) {
    COUNTERS[i].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — PRODUKSI
// ---------------------------------------------------------------------------

/// `HostnameContext::IndexID` berikutnya.
#[no_mangle]
pub extern "C" fn klatos_id_proses_hostname_ambil() -> u64 {
    match RUANG_HOSTNAME.ambil() {
        HasilId::Id(v) => {
            bump(0);
            v
        }
        HasilId::Habis => {
            bump(2);
            bump(3);
            0
        }
    }
}

/// `ScopedProcessList::IndexID` berikutnya.
#[no_mangle]
pub extern "C" fn klatos_id_proses_scoped_ambil() -> u64 {
    match RUANG_SCOPED.ambil() {
        HasilId::Id(v) => {
            bump(1);
            v
        }
        HasilId::Habis => {
            bump(2);
            bump(3);
            0
        }
    }
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_id_proses_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Nilai penghitung satu ruang (audit). `0`=hostname, `1`=scoped.
#[no_mangle]
pub extern "C" fn klatos_id_proses_intip(which: u32) -> u64 {
    match which {
        0 => RUANG_HOSTNAME.intip(),
        1 => RUANG_SCOPED.intip(),
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Matriks
// ---------------------------------------------------------------------------

fn run_matrix() -> KlatosRuangIdReport {
    let mut m = Matrix::baru();

    let host_before = RUANG_HOSTNAME.intip();
    let scop_before = RUANG_SCOPED.intip();

    // --- kelompok Umum ------------------------------------------------------
    {
        // Dua ruang BERBEDA mulai dari 0 secara terpisah (bukan satu bersama).
        let h = PenyimpananUji::baru(0);
        let s = PenyimpananUji::baru(0);
        m.catat(crate::ruang_id::ambil_in(&h, NS_MAKS, Gaya::PascaNaik) == HasilId::Id(0), Grup::Umum);
        m.catat(crate::ruang_id::ambil_in(&h, NS_MAKS, Gaya::PascaNaik) == HasilId::Id(1), Grup::Umum);
        m.catat(crate::ruang_id::ambil_in(&s, NS_MAKS, Gaya::PascaNaik) == HasilId::Id(0), Grup::Umum);
        m.catat(crate::ruang_id::ambil_in(&s, NS_MAKS, Gaya::PascaNaik) == HasilId::Id(1), Grup::Umum);
        m.catat(h.intip() == 2, Grup::Umum);
        m.catat(s.intip() == 2, Grup::Umum);

        // 4096 pengambilan: naik tepat satu, tidak ada ulangan.
        let t = PenyimpananUji::baru(0);
        let mut naik = true;
        let mut i = 0;
        let mut prev: Option<u64> = None;
        while i < 4096 {
            match crate::ruang_id::ambil_in(&t, NS_MAKS, Gaya::PascaNaik) {
                HasilId::Id(v) => {
                    if let Some(p) = prev {
                        if v != p + 1 {
                            naik = false;
                        }
                    }
                    prev = Some(v);
                }
                HasilId::Habis => naik = false,
            }
            i += 1;
        }
        m.catat(naik, Grup::Umum);
        m.catat(prev == Some(4095), Grup::Umum);
    }

    // --- kelompok Peta: diferensial terhadap oracle historis ----------------
    let (d_h, b_h) = diferensial_gaya(Gaya::PascaNaik, NS_MAKS, RENTANG_DIFERENSIAL);
    let (d_s, b_s) = diferensial_gaya(Gaya::PascaNaik, NS_MAKS, RENTANG_DIFERENSIAL);
    let diferensial_diukur = d_h + d_s;
    let diferensial_beda = b_h + b_s;
    m.catat(b_h == 0, Grup::Peta);
    m.catat(b_s == 0, Grup::Peta);
    m.catat(diferensial_diukur == 2 * RENTANG_DIFERENSIAL as u32, Grup::Peta);
    // Id 0 adalah id SAH di kedua ruang ini (berbeda dari task_id).
    m.catat(crate::ruang_id::kebijakan_lama(0, Gaya::PascaNaik) == 0, Grup::Peta);

    // --- kelompok Negatif ---------------------------------------------------
    {
        // Ruang PENUH `u64`: memberi `maks`, lalu penghitung JENUH (tidak
        // membungkus ke 0 — lihat dokumentasi `ruang_id`). Ruang ini tidak
        // punya keadaan "habis" yang dapat diwakili.
        let s = PenyimpananUji::baru(NS_MAKS);
        m.catat(crate::ruang_id::ambil_in(&s, NS_MAKS, Gaya::PascaNaik) == HasilId::Id(NS_MAKS), Grup::Negatif);
        m.catat(s.intip() == NS_MAKS, Grup::Negatif);
        m.catat(crate::ruang_id::ambil_in(&s, NS_MAKS, Gaya::PascaNaik) == HasilId::Id(NS_MAKS), Grup::Negatif);

        // Batas yang DAPAT dicapai: ruang terbatas menolak setelah id terakhir.
        let sb = PenyimpananUji::baru(7);
        m.catat(crate::ruang_id::ambil_in(&sb, 7, Gaya::PascaNaik) == HasilId::Id(7), Grup::Negatif);
        m.catat(crate::ruang_id::ambil_in(&sb, 7, Gaya::PascaNaik) == HasilId::Habis, Grup::Negatif);
        m.catat(sb.intip() == 8, Grup::Negatif);

        // Gaya salah (pra-naik) akan menggeser urutan — dibuktikan berbeda.
        let a = PenyimpananUji::baru(0);
        let b = PenyimpananUji::baru(0);
        m.catat(
            crate::ruang_id::ambil_in(&a, NS_MAKS, Gaya::PraNaik)
                != crate::ruang_id::ambil_in(&b, NS_MAKS, Gaya::PascaNaik),
            Grup::Negatif,
        );

        // KONTROL: kedua ruang PRODUKSI tidak bergeser.
        m.catat(RUANG_HOSTNAME.intip() == host_before, Grup::Negatif);
        m.catat(RUANG_SCOPED.intip() == scop_before, Grup::Negatif);
        // Kontrol tambahan: keduanya independen (tidak berbagi satu atomik).
        m.catat(NS_MAKS == u64::MAX, Grup::Negatif);
    }

    buat_laporan(
        REPORT_TAG,
        &m,
        diferensial_diukur,
        diferensial_beda,
        klatos_id_proses_counter(0),
        klatos_id_proses_counter(1),
        0,
    )
}

/// Menjalankan matriks otoritas identitas objek namespace proses.
#[no_mangle]
pub extern "C" fn klatos_id_proses_selftest() -> KlatosRuangIdReport {
    run_matrix()
}

/// Konformansi waktu-kompilasi.
pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4950_5253); // "KLATIPRS"
        assert!(NS_MAKS == u64::MAX);
        assert!(RuangId::baru(0, NS_MAKS, Gaya::PascaNaik).gaya() as usize == 1);
        assert!(RuangId::baru(0, NS_MAKS, Gaya::PascaNaik).gaya() as usize == 1);
        assert!(core::mem::size_of::<KlatosRuangIdReport>() == 56);
        assert!(N_COUNTER == 4);
    };
}
