// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.34 — OTORITAS IDENTITAS OBJEK VFS (Rust).


use crate::ruang_id::{
    ambil_in, buat_laporan, diferensial_gaya, Gaya, Grup, HasilId, KlatosRuangIdReport, Matrix,
    PenyimpananUji, RuangId,
};
use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATIVFS".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4956_4653;

/// Batas `FileSystemID` (`u32`).
pub const FS_MAKS: u64 = 0xFFFF_FFFF;
/// Batas `FIFO` id (`int`, bit tanda tidak boleh dipakai).
pub const FIFO_MAKS: u64 = 0x7FFF_FFFF;
/// Batas `VFSRootContext::IndexID` (`u64`).
pub const ROOT_MAKS: u64 = u64::MAX;

/// Rentang seed diferensial per ruang.
const RENTANG_DIFERENSIAL: u64 = 16384;

// ---------------------------------------------------------------------------
// Ruang id PRODUKSI
// ---------------------------------------------------------------------------

/// `FileSystemID` — dasar 0, pra-naik (id pertama 1).
static RUANG_FS: RuangId = RuangId::baru(0, FS_MAKS, Gaya::PraNaik);

/// `VFSRootContext::IndexID` — dasar 0, pasca-naik (id pertama 0).
static RUANG_ROOT: RuangId = RuangId::baru(0, ROOT_MAKS, Gaya::PascaNaik);

/// `FIFO::m_fifo_id` — dasar 1, pra-naik (id pertama 2).
static RUANG_FIFO: RuangId = RuangId::baru(1, FIFO_MAKS, Gaya::PraNaik);

const N_COUNTER: usize = 5;

/// Penghitung audit.
/// `0` = fs_alloc, `1` = root_alloc, `2` = fifo_alloc, `3` = habis, `4` = galat.
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(i: usize) {
    COUNTERS[i].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — PRODUKSI
// ---------------------------------------------------------------------------

/// `FileSystemID` berikutnya. `0` berarti ruang habis (tidak pernah terjadi).
#[no_mangle]
pub extern "C" fn klatos_id_vfs_fs_ambil() -> u32 {
    match RUANG_FS.ambil() {
        HasilId::Id(v) => {
            bump(0);
            v as u32
        }
        HasilId::Habis => {
            bump(3);
            bump(4);
            0
        }
    }
}

/// `VFSRootContext::IndexID` berikutnya.
#[no_mangle]
pub extern "C" fn klatos_id_vfs_root_ambil() -> u64 {
    match RUANG_ROOT.ambil() {
        HasilId::Id(v) => {
            bump(1);
            v
        }
        HasilId::Habis => {
            bump(3);
            bump(4);
            0
        }
    }
}

/// `FIFO::m_fifo_id` berikutnya.
#[no_mangle]
pub extern "C" fn klatos_id_vfs_fifo_ambil() -> i32 {
    match RUANG_FIFO.ambil() {
        HasilId::Id(v) => {
            bump(2);
            v as i32
        }
        HasilId::Habis => {
            bump(3);
            bump(4);
            0
        }
    }
}

/// Nilai satu penghitung audit. Lihat daftar indeks di [`COUNTERS`].
#[no_mangle]
pub extern "C" fn klatos_id_vfs_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Nilai penghitung satu ruang (audit). `0`=fs, `1`=root, `2`=fifo.
#[no_mangle]
pub extern "C" fn klatos_id_vfs_intip(which: u32) -> u64 {
    match which {
        0 => RUANG_FS.intip(),
        1 => RUANG_ROOT.intip(),
        2 => RUANG_FIFO.intip(),
        _ => 0,
    }
}

/// Membaca satu id dari ruang SINTETIS (dipakai probe ruang-pengguna lewat
/// jalur lain? Tidak — hanya audit; ruang produksi tidak pernah disetel).
#[no_mangle]
pub extern "C" fn klatos_id_vfs_batas(which: u32) -> u64 {
    match which {
        0 => FS_MAKS,
        1 => ROOT_MAKS,
        2 => FIFO_MAKS,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Matriks
// ---------------------------------------------------------------------------

fn run_matrix() -> KlatosRuangIdReport {
    let mut m = Matrix::baru();

    // Kontrol: matriks tidak boleh menggeser ruang id produksi.
    let fs_before = RUANG_FS.intip();
    let root_before = RUANG_ROOT.intip();
    let fifo_before = RUANG_FIFO.intip();

    // --- kelompok Umum: urutan + keunikan tiap ruang (SINTETIS) ------------
    {
        let s = PenyimpananUji::baru(0);
        // FileSystemID: dasar 0, pra-naik -> 1,2,3,...
        m.catat(ambil_in(&s, FS_MAKS, Gaya::PraNaik) == HasilId::Id(1), Grup::Umum);
        m.catat(ambil_in(&s, FS_MAKS, Gaya::PraNaik) == HasilId::Id(2), Grup::Umum);
        m.catat(ambil_in(&s, FS_MAKS, Gaya::PraNaik) == HasilId::Id(3), Grup::Umum);
        m.catat(s.intip() == 3, Grup::Umum);

        // VFSRootContext: dasar 0, pasca-naik -> 0,1,2,...
        let s2 = PenyimpananUji::baru(0);
        m.catat(ambil_in(&s2, ROOT_MAKS, Gaya::PascaNaik) == HasilId::Id(0), Grup::Umum);
        m.catat(ambil_in(&s2, ROOT_MAKS, Gaya::PascaNaik) == HasilId::Id(1), Grup::Umum);
        m.catat(ambil_in(&s2, ROOT_MAKS, Gaya::PascaNaik) == HasilId::Id(2), Grup::Umum);
        m.catat(s2.intip() == 3, Grup::Umum);

        // FIFO: dasar 1, pra-naik -> 2,3,4,...
        let s3 = PenyimpananUji::baru(1);
        m.catat(ambil_in(&s3, FIFO_MAKS, Gaya::PraNaik) == HasilId::Id(2), Grup::Umum);
        m.catat(ambil_in(&s3, FIFO_MAKS, Gaya::PraNaik) == HasilId::Id(3), Grup::Umum);
        m.catat(s3.intip() == 3, Grup::Umum);

        // 1024 pengambilan berturut-turut: naik tepat satu, tidak ada ulangan.
        let s4 = PenyimpananUji::baru(0);
        let mut prev: Option<u64> = None;
        let mut unik = true;
        let mut naik = true;
        let mut i = 0;
        while i < 1024 {
            if let HasilId::Id(v) = ambil_in(&s4, ROOT_MAKS, Gaya::PascaNaik) {
                if let Some(p) = prev {
                    if v != p + 1 {
                        naik = false;
                    }
                }
                if v > 1023 {
                    unik = false;
                }
                prev = Some(v);
            } else {
                naik = false;
                unik = false;
            }
            i += 1;
        }
        m.catat(naik, Grup::Umum);
        m.catat(unik, Grup::Umum);
        m.catat(prev == Some(1023), Grup::Umum);
    }

    // --- kelompok Peta: diferensial terhadap oracle historis ----------------
    let (d_fs, b_fs) = diferensial_gaya(Gaya::PraNaik, FS_MAKS, RENTANG_DIFERENSIAL);
    let (d_root, b_root) = diferensial_gaya(Gaya::PascaNaik, ROOT_MAKS, RENTANG_DIFERENSIAL);
    let (d_fifo, b_fifo) = diferensial_gaya(Gaya::PraNaik, FIFO_MAKS, RENTANG_DIFERENSIAL);
    let diferensial_diukur = d_fs + d_root + d_fifo;
    let diferensial_beda = b_fs + b_root + b_fifo;
    m.catat(b_fs == 0, Grup::Peta);
    m.catat(b_root == 0, Grup::Peta);
    m.catat(b_fifo == 0, Grup::Peta);
    m.catat(diferensial_diukur == 3 * RENTANG_DIFERENSIAL as u32, Grup::Peta);

    // Dasar bukan nol (FIFO) juga dipetakan dari seed 0 — oracle memakai
    // transisi yang sama, jadi hasilnya tetap cocok.
    m.catat(kebijakan_cocok(Gaya::PraNaik, FIFO_MAKS, 1), Grup::Peta);
    m.catat(kebijakan_cocok(Gaya::PascaNaik, ROOT_MAKS, 0), Grup::Peta);

    // --- kelompok Negatif: batas + habis + ruang produksi tidak tersentuh ---
    {
        // PraNaik tepat di `maks`: menolak, dan penghitung tidak bergeser.
        let s = PenyimpananUji::baru(FS_MAKS);
        m.catat(ambil_in(&s, FS_MAKS, Gaya::PraNaik) == HasilId::Habis, Grup::Negatif);
        m.catat(s.intip() == FS_MAKS, Grup::Negatif);
        m.catat(ambil_in(&s, FS_MAKS, Gaya::PraNaik) == HasilId::Habis, Grup::Negatif);

        // PraNaik satu di bawah `maks`: masih memberi `maks`, lalu menolak.
        let s2 = PenyimpananUji::baru(FS_MAKS - 1);
        m.catat(ambil_in(&s2, FS_MAKS, Gaya::PraNaik) == HasilId::Id(FS_MAKS), Grup::Negatif);
        m.catat(ambil_in(&s2, FS_MAKS, Gaya::PraNaik) == HasilId::Habis, Grup::Negatif);

        // PascaNaik pada ruang PENUH `u64`: memberi `maks`, lalu penghitung
        // JENUH (tidak membungkus ke 0 — lihat dokumentasi `ruang_id`). Ruang
        // ini tidak punya keadaan "habis" yang dapat diwakili.
        let s3 = PenyimpananUji::baru(ROOT_MAKS);
        m.catat(ambil_in(&s3, ROOT_MAKS, Gaya::PascaNaik) == HasilId::Id(ROOT_MAKS), Grup::Negatif);
        m.catat(s3.intip() == ROOT_MAKS, Grup::Negatif);
        m.catat(ambil_in(&s3, ROOT_MAKS, Gaya::PascaNaik) == HasilId::Id(ROOT_MAKS), Grup::Negatif);

        // Batas yang DAPAT dicapai: ruang terbatas menolak setelah id terakhir.
        let s3b = PenyimpananUji::baru(5);
        m.catat(ambil_in(&s3b, 5, Gaya::PascaNaik) == HasilId::Id(5), Grup::Negatif);
        m.catat(ambil_in(&s3b, 5, Gaya::PascaNaik) == HasilId::Habis, Grup::Negatif);
        m.catat(s3b.intip() == 6, Grup::Negatif);

        // FIFO `int`: batas bit tanda dihormati.
        let s4 = PenyimpananUji::baru(FIFO_MAKS - 1);
        m.catat(ambil_in(&s4, FIFO_MAKS, Gaya::PraNaik) == HasilId::Id(FIFO_MAKS), Grup::Negatif);
        m.catat(ambil_in(&s4, FIFO_MAKS, Gaya::PraNaik) == HasilId::Habis, Grup::Negatif);
        m.catat(FIFO_MAKS == 0x7FFF_FFFF, Grup::Negatif);

        // Batas ruang berbeda satu sama lain (bukan satu batas bersama).
        m.catat(FS_MAKS != FIFO_MAKS, Grup::Negatif);
        m.catat(ROOT_MAKS != FS_MAKS, Grup::Negatif);

        // KONTROL: ruang id PRODUKSI tidak bergeser oleh seluruh matriks.
        m.catat(RUANG_FS.intip() == fs_before, Grup::Negatif);
        m.catat(RUANG_ROOT.intip() == root_before, Grup::Negatif);
        m.catat(RUANG_FIFO.intip() == fifo_before, Grup::Negatif);
    }

    buat_laporan(
        REPORT_TAG,
        &m,
        diferensial_diukur,
        diferensial_beda,
        klatos_id_vfs_counter(0),
        klatos_id_vfs_counter(1),
        klatos_id_vfs_counter(2),
    )
}

/// Memeriksa satu rentang kecil pada dasar bukan-nol.
fn kebijakan_cocok(gaya: Gaya, maks: u64, dasar: u64) -> bool {
    let mut seed = dasar;
    while seed < dasar + 64 {
        let s = PenyimpananUji::baru(seed);
        let r = ambil_in(&s, maks, gaya);
        if r != HasilId::Id(crate::ruang_id::kebijakan_lama(seed, gaya)) {
            return false;
        }
        if s.intip() != crate::ruang_id::nilai_setelah(seed) {
            return false;
        }
        seed += 1;
    }
    true
}

/// Menjalankan matriks otoritas identitas objek VFS.
#[no_mangle]
pub extern "C" fn klatos_id_vfs_selftest() -> KlatosRuangIdReport {
    run_matrix()
}

/// Konformansi waktu-kompilasi.
pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4956_4653); // "KLATIVFS"
        assert!(FS_MAKS == 0xFFFF_FFFF);
        assert!(FIFO_MAKS == 0x7FFF_FFFF);
        assert!(ROOT_MAKS == u64::MAX);

        // Titik awal tiap ruang disalin dari C++.
        assert!(RuangId::baru(0, FS_MAKS, Gaya::PraNaik).gaya() as usize == 0);
        assert!(RuangId::baru(0, ROOT_MAKS, Gaya::PascaNaik).gaya() as usize == 1);
        assert!(RuangId::baru(1, FIFO_MAKS, Gaya::PraNaik).gaya() as usize == 0);

        assert!(core::mem::size_of::<KlatosRuangIdReport>() == 56);
        assert!(N_COUNTER == 5);
    };
}
