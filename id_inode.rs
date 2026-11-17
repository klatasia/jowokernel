// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.35 — OTORITAS PENOMORAN INODE (Rust).


use crate::ruang_id::{
    ambil_in, buat_laporan, diferensial_gaya, Gaya, Grup, HasilId, KlatosRuangIdReport, Matrix,
    PenyimpananUji, RuangId,
};
use core::cell::Cell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Tag laporan: "KLATIINO".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4949_4E4F;

/// Batas indeks komponen SysFS (`InodeIndex` = `u64`).
pub const SYSFS_MAKS: u64 = u64::MAX;
/// Batas indeks inode RAMFS (`unsigned` = `u32`).
pub const RAMFS_MAKS: u64 = 0xFFFF_FFFF;
/// Nilai dasar nomor inode RAMFS (`RAMFS/FileSystem.h:45`).
pub const RAMFS_DASAR: u64 = 2;

/// Jumlah slot per-instance (di luar slot 0 cadangan).
pub const RAMFS_SLOT: usize = 64;
/// Total slot, termasuk slot 0.
pub const RAMFS_TOTAL: usize = RAMFS_SLOT + 1;

const RENTANG_DIFERENSIAL: u64 = 16384;
const N_COUNTER: usize = 5;

// ---------------------------------------------------------------------------
// Ruang GLOBAL produksi
// ---------------------------------------------------------------------------

/// Indeks komponen SysFS — dasar 0, pra-naik (id pertama 1).
static RUANG_SYSFS: RuangId = RuangId::baru(0, SYSFS_MAKS, Gaya::PraNaik);

// ---------------------------------------------------------------------------
// Tabel ruang PER-INSTANCE
// ---------------------------------------------------------------------------

/// Abstraksi tabel ruang per-instance. Produksi memakai statik atomik;
/// matriks memakai larik `Cell` di stack, sehingga matriks **tidak pernah**
/// menyentuh slot produksi.
pub trait TabelInstansi {
    fn slot(&self) -> usize;
    fn dipakai(&self, i: usize) -> bool;
    /// `true` bila slot `i` sebelumnya bebas dan kini diklaim.
    fn klaim(&self, i: usize) -> bool;
    fn lepas(&self, i: usize);
    fn setel(&self, i: usize, v: u64);
    fn ambil(&self, i: usize) -> HasilId;
}

/// Algoritma klaim: slot 1..n satu per satu; slot 0 selalu tersedia sebagai
/// ruang cadangan bersama, jadi klaim tidak pernah gagal.
pub fn buat_in<T: TabelInstansi>(t: &T, dasar: u64) -> usize {
    let mut i = 1;
    while i < t.slot() {
        if t.klaim(i) {
            t.setel(i, dasar);
            return i;
        }
        i += 1;
    }
    0
}

struct TabelProduksi;

static RAMFS_DIPAKAI: [AtomicBool; RAMFS_TOTAL] =
    [const { AtomicBool::new(false) }; RAMFS_TOTAL];
static RAMFS_RUANG: [RuangId; RAMFS_TOTAL] =
    [const { RuangId::baru(RAMFS_DASAR, RAMFS_MAKS, Gaya::PascaNaik) }; RAMFS_TOTAL];

impl TabelInstansi for TabelProduksi {
    fn slot(&self) -> usize {
        RAMFS_TOTAL
    }
    fn dipakai(&self, i: usize) -> bool {
        i < RAMFS_TOTAL && RAMFS_DIPAKAI[i].load(Ordering::Acquire)
    }
    fn klaim(&self, i: usize) -> bool {
        if i == 0 || i >= RAMFS_TOTAL {
            return false;
        }
        !RAMFS_DIPAKAI[i].swap(true, Ordering::AcqRel)
    }
    fn lepas(&self, i: usize) {
        if i == 0 || i >= RAMFS_TOTAL {
            return;
        }
        RAMFS_DIPAKAI[i].store(false, Ordering::Release);
    }
    fn setel(&self, i: usize, v: u64) {
        if i < RAMFS_TOTAL {
            RAMFS_RUANG[i].setel(v);
        }
    }
    fn ambil(&self, i: usize) -> HasilId {
        if i < RAMFS_TOTAL {
            RAMFS_RUANG[i].ambil()
        } else {
            HasilId::Habis
        }
    }
}

static TABEL: TabelProduksi = TabelProduksi;

/// Tabel SINTETIS untuk matriks (stack, tanpa atomik).
struct TabelUji {
    dipakai: [Cell<bool>; RAMFS_TOTAL],
    ruang: [Cell<u64>; RAMFS_TOTAL],
}

impl TabelUji {
    const fn baru() -> Self {
        TabelUji {
            dipakai: [const { Cell::new(false) }; RAMFS_TOTAL],
            ruang: [const { Cell::new(RAMFS_DASAR) }; RAMFS_TOTAL],
        }
    }
}

impl TabelInstansi for TabelUji {
    fn slot(&self) -> usize {
        RAMFS_TOTAL
    }
    fn dipakai(&self, i: usize) -> bool {
        i < RAMFS_TOTAL && self.dipakai[i].get()
    }
    fn klaim(&self, i: usize) -> bool {
        if i == 0 || i >= RAMFS_TOTAL {
            return false;
        }
        let was = self.dipakai[i].get();
        self.dipakai[i].set(true);
        !was
    }
    fn lepas(&self, i: usize) {
        if i == 0 || i >= RAMFS_TOTAL {
            return;
        }
        self.dipakai[i].set(false);
    }
    fn setel(&self, i: usize, v: u64) {
        if i < RAMFS_TOTAL {
            self.ruang[i].set(v);
        }
    }
    fn ambil(&self, i: usize) -> HasilId {
        if i < RAMFS_TOTAL {
            ambil_in(&UjiSatu(&self.ruang[i]), RAMFS_MAKS, Gaya::PascaNaik)
        } else {
            HasilId::Habis
        }
    }
}

/// Pembungkus `Cell<u64>` menjadi [`crate::ruang_id::PenyimpananId`].
struct UjiSatu<'a>(&'a Cell<u64>);

impl crate::ruang_id::PenyimpananId for UjiSatu<'_> {
    fn muat(&self) -> u64 {
        self.0.get()
    }
    fn cas(&self, cur: u64, baru: u64) -> bool {
        if self.0.get() == cur {
            self.0.set(baru);
            true
        } else {
            false
        }
    }
    fn simpan(&self, nilai: u64) {
        self.0.set(nilai);
    }
}

// ---------------------------------------------------------------------------
// Penghitung audit
// ---------------------------------------------------------------------------

/// `0` = sysfs_alloc, `1` = ramfs_alloc, `2` = ramfs_buat, `3` = habis, `4` = galat.
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(i: usize) {
    COUNTERS[i].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Permukaan C ABI — PRODUKSI
// ---------------------------------------------------------------------------

/// Indeks komponen SysFS berikutnya (`InodeIndex`).
#[no_mangle]
pub extern "C" fn klatos_id_inode_sysfs_ambil() -> u64 {
    match RUANG_SYSFS.ambil() {
        HasilId::Id(v) => {
            bump(0);
            v
        }
        HasilId::Habis => {
            bump(3);
            bump(4);
            0
        }
    }
}

/// Mengklaim satu slot ruang inode RAMFS. Selalu berhasil: `0` berarti ruang
/// cadangan bersama (slot per-instance habis), bukan kegagalan.
#[no_mangle]
pub extern "C" fn klatos_id_inode_ramfs_buat() -> u32 {
    bump(2);
    buat_in(&TABEL, RAMFS_DASAR) as u32
}

/// Melepas slot ruang inode RAMFS (dipakai matriks dan penghancuran FS).
#[no_mangle]
pub extern "C" fn klatos_id_inode_ramfs_lepas(handle: u32) {
    TABEL.lepas(handle as usize);
}

/// Nomor inode RAMFS berikutnya pada slot `handle`.
#[no_mangle]
pub extern "C" fn klatos_id_inode_ramfs_ambil(handle: u32) -> u32 {
    match TABEL.ambil(handle as usize) {
        HasilId::Id(v) => {
            bump(1);
            v as u32
        }
        HasilId::Habis => {
            bump(3);
            bump(4);
            0
        }
    }
}

/// `1` bila slot `handle` sedang dipakai.
#[no_mangle]
pub extern "C" fn klatos_id_inode_ramfs_dipakai(handle: u32) -> u32 {
    u32::from(TABEL.dipakai(handle as usize))
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_id_inode_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Nilai penghitung satu ruang (audit). `0`=sysfs, `1`=ramfs slot handle.
#[no_mangle]
pub extern "C" fn klatos_id_inode_intip(which: u32, handle: u32) -> u64 {
    match which {
        0 => RUANG_SYSFS.intip(),
        1 => {
            let h = handle as usize;
            if h < RAMFS_TOTAL {
                RAMFS_RUANG[h].intip()
            } else {
                0
            }
        }
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Matriks
// ---------------------------------------------------------------------------

fn run_matrix() -> KlatosRuangIdReport {
    let mut m = Matrix::baru();

    let sysfs_before = RUANG_SYSFS.intip();

    // --- kelompok Umum: urutan global + per-instance ------------------------
    {
        let s = PenyimpananUji::baru(0);
        m.catat(ambil_in(&s, SYSFS_MAKS, Gaya::PraNaik) == HasilId::Id(1), Grup::Umum);
        m.catat(ambil_in(&s, SYSFS_MAKS, Gaya::PraNaik) == HasilId::Id(2), Grup::Umum);
        m.catat(s.intip() == 2, Grup::Umum);

        // Tabel per-instance: dua slot berbeda, masing-masing mulai dari 2.
        let t = TabelUji::baru();
        let a = buat_in(&t, RAMFS_DASAR);
        let b = buat_in(&t, RAMFS_DASAR);
        m.catat(a != 0 && b != 0 && a != b, Grup::Umum);
        m.catat(t.dipakai(a) && t.dipakai(b), Grup::Umum);
        m.catat(t.ambil(a) == HasilId::Id(2), Grup::Umum);
        m.catat(t.ambil(a) == HasilId::Id(3), Grup::Umum);
        m.catat(t.ambil(b) == HasilId::Id(2), Grup::Umum); // instance lain mulai dari 2
        m.catat(t.ambil(b) == HasilId::Id(3), Grup::Umum);
        m.catat(t.ambil(a) == HasilId::Id(4), Grup::Umum);

        // Lepas lalu klaim ulang: nomor kembali ke dasar (instance baru).
        t.lepas(a);
        m.catat(!t.dipakai(a), Grup::Umum);
        let c = buat_in(&t, RAMFS_DASAR);
        m.catat(c == a, Grup::Umum); // slot pertama yang bebas dipakai ulang
        m.catat(t.ambil(c) == HasilId::Id(2), Grup::Umum);
    }

    // --- kelompok Peta: diferensial terhadap oracle historis ----------------
    let (d_sysfs, b_sysfs) = diferensial_gaya(Gaya::PraNaik, SYSFS_MAKS, RENTANG_DIFERENSIAL);
    let (d_ramfs, b_ramfs) = diferensial_gaya(Gaya::PascaNaik, RAMFS_MAKS, RENTANG_DIFERENSIAL);
    let diferensial_diukur = d_sysfs + d_ramfs;
    let diferensial_beda = b_sysfs + b_ramfs;
    m.catat(b_sysfs == 0, Grup::Peta);
    m.catat(b_ramfs == 0, Grup::Peta);
    m.catat(diferensial_diukur == 2 * RENTANG_DIFERENSIAL as u32, Grup::Peta);

    // Diferensial per-instance: slot dengan dasar 2, urutan sama dengan oracle.
    {
        let t = TabelUji::baru();
        let a = buat_in(&t, RAMFS_DASAR);
        let mut beda = 0u32;
        let mut seed = RAMFS_DASAR;
        let mut i = 0;
        while i < 256 {
            let r = t.ambil(a);
            if r != HasilId::Id(crate::ruang_id::kebijakan_lama(seed, Gaya::PascaNaik)) {
                beda += 1;
            }
            seed += 1;
            i += 1;
        }
        m.catat(beda == 0, Grup::Peta);
    }

    // --- kelompok Negatif ---------------------------------------------------
    {
        // SysFS global tepat di batas.
        let s = PenyimpananUji::baru(SYSFS_MAKS);
        m.catat(ambil_in(&s, SYSFS_MAKS, Gaya::PraNaik) == HasilId::Habis, Grup::Negatif);
        m.catat(s.intip() == SYSFS_MAKS, Grup::Negatif);

        // RAMFS `unsigned` tepat di batas.
        let s2 = PenyimpananUji::baru(RAMFS_MAKS);
        m.catat(ambil_in(&s2, RAMFS_MAKS, Gaya::PascaNaik) == HasilId::Id(RAMFS_MAKS), Grup::Negatif);
        m.catat(ambil_in(&s2, RAMFS_MAKS, Gaya::PascaNaik) == HasilId::Habis, Grup::Negatif);
        m.catat(RAMFS_MAKS == 0xFFFF_FFFF, Grup::Negatif);

        // Slot 0 tidak pernah diklaim; `klaim(0)` selalu gagal.
        let t = TabelUji::baru();
        m.catat(!t.klaim(0), Grup::Negatif);
        m.catat(!t.dipakai(0), Grup::Negatif);

        // Slot di luar tabel ditolak.
        m.catat(!t.klaim(RAMFS_TOTAL), Grup::Negatif);
        m.catat(!t.dipakai(RAMFS_TOTAL), Grup::Negatif);
        m.catat(t.ambil(RAMFS_TOTAL) == HasilId::Habis, Grup::Negatif);

        // Tabel penuh -> `buat_in` mengembalikan slot cadangan 0.
        let t2 = TabelUji::baru();
        let mut n = 0;
        let mut i = 1;
        while i < RAMFS_TOTAL {
            if buat_in(&t2, RAMFS_DASAR) != 0 {
                n += 1;
            }
            i += 1;
        }
        m.catat(n == RAMFS_SLOT, Grup::Negatif);
        let mut terisi = 0;
        let mut j = 1;
        while j < RAMFS_TOTAL {
            if t2.dipakai(j) {
                terisi += 1;
            }
            j += 1;
        }
        m.catat(terisi == RAMFS_SLOT, Grup::Negatif);
        m.catat(buat_in(&t2, RAMFS_DASAR) == 0, Grup::Negatif); // cadangan bersama
        m.catat(t2.ambil(0) == HasilId::Id(RAMFS_DASAR), Grup::Negatif);

        // KONTROL: ruang global produksi tidak bergeser.
        m.catat(RUANG_SYSFS.intip() == sysfs_before, Grup::Negatif);
    }

    buat_laporan(
        REPORT_TAG,
        &m,
        diferensial_diukur,
        diferensial_beda,
        klatos_id_inode_counter(0),
        klatos_id_inode_counter(1),
        klatos_id_inode_counter(2),
    )
}

/// Menjalankan matriks otoritas penomoran inode.
#[no_mangle]
pub extern "C" fn klatos_id_inode_selftest() -> KlatosRuangIdReport {
    run_matrix()
}

/// Konformansi waktu-kompilasi.
pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4949_4E4F); // "KLATIINO"
        assert!(SYSFS_MAKS == u64::MAX);
        assert!(RAMFS_MAKS == 0xFFFF_FFFF);
        assert!(RAMFS_DASAR == 2); // RAMFS/FileSystem.h:45
        assert!(RAMFS_TOTAL == 65);
        assert!(RuangId::baru(0, SYSFS_MAKS, Gaya::PraNaik).gaya() as usize == 0);
        assert!(RuangId::baru(RAMFS_DASAR, RAMFS_MAKS, Gaya::PascaNaik).gaya() as usize == 1);
        assert!(core::mem::size_of::<KlatosRuangIdReport>() == 56);
        assert!(N_COUNTER == 5);
    };
}
