// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Fondasi memori (P0.97.1 `memory`) — **tanpa alokator** (P0.97.2).


use crate::arch;

/// Ukuran halaman x86_64. Sama dengan `PAGE_SIZE` di `Paket/Platform.h`.
pub const PAGE_SIZE: u64 = 4096;

/// Jumlah kata `u64` yang dipakai probe round-trip (512 byte stack).
pub const STACK_WORDS: usize = 64;

/// Pengali pola (konstanta Weyl) — dipakai agar pola tidak mudah berpola-rata.
const PATTERN_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Nilai yang diharapkan pada indeks `index`.
const fn pattern_at(index: usize) -> u64 {
    PATTERN_SEED ^ ((index as u64) << 32) ^ (index as u64)
}

/// Hasil probe fondasi memori.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryFoundation {
    /// Ukuran halaman yang dimodelkan.
    pub page_size: u64,
    /// Lebar penunjuk dalam bit.
    pub pointer_bits: u32,
    /// Selalu `false` — crate ini tidak memasang alokator.
    pub allocator: bool,
    /// Jumlah kata yang diuji round-trip.
    pub stack_words: u32,
    /// Checksum hasil pembacaan ulang.
    pub checksum: u64,
    /// `true` bila seluruh kata terbaca kembali persis seperti ditulis.
    pub round_trip_ok: bool,
}

impl MemoryFoundation {
    /// Lulus bila round-trip stack berhasil, target 64-bit, ukuran halaman
    /// konsisten, dan **tidak ada** alokator.
    pub const fn ok(&self) -> bool {
        self.round_trip_ok
            && self.pointer_bits == 64
            && self.page_size == PAGE_SIZE
            && !self.allocator
    }
}

/// Menjalankan probe fondasi memori pada stack kernel yang nyata.
pub fn probe() -> MemoryFoundation {
    let mut buffer = [0u64; STACK_WORDS];

    for (index, slot) in buffer.iter_mut().enumerate() {
        *slot = pattern_at(index);
    }

    let mut checksum = 0u64;
    let mut round_trip_ok = true;
    for (index, slot) in buffer.iter().enumerate() {
        if *slot != pattern_at(index) {
            round_trip_ok = false;
        }
        checksum = checksum.rotate_left(7) ^ *slot;
    }

    MemoryFoundation {
        page_size: PAGE_SIZE,
        pointer_bits: arch::POINTER_BITS,
        allocator: false,
        stack_words: STACK_WORDS as u32,
        checksum,
        round_trip_ok,
    }
}

// ===========================================================================
// P0.98.1 — MODEL MEMORI BERTIPE
// ===========================================================================
//
// P0.98.1 meminta tipe yang jelas dan menghindari integer mentah. Karena itu
// setiap alamat dan setiap nomor halaman/frame dibungkus, medannya privat, dan
// satu-satunya jalan masuk adalah konstruktor yang memeriksa bentuknya.
//
// # Yang TIDAK dilakukan model ini
//
// Ia **tidak** menyentuh MMU, page table, atau CR3. Ini model **pembukuan**:
// ia mencatat pemetaan yang *seharusnya* ada, sehingga otoritas dan kesalahan
// dapat diuji di dalam kernel Rust tanpa mengambil alih manajemen memori C++
// (P0.98 "belum berarti mengganti scheduler/memory/process C++ production").
//
// # Otoritas (P0.98.2)
//
// Setiap operasi yang mengubah keadaan memerlukan [`crate::ksm::Izin`] yang
// diterbitkan [`crate::ksm::periksa_operasi`]. Model ini **tidak menyimpan hak**
// dan tidak dapat membuat token — lihat dokumentasi `ksm::Izin`.

use crate::ksm::{Izin, Keputusan};
use crate::{abi2, dispatch};

/// Jumlah pemetaan maksimum per ruang alamat. Tetap, tanpa alokasi.
pub const MAX_MAPPINGS: usize = 16;

/// Batas alamat fisik yang dimodelkan (1 TiB). Bukan batas perangkat keras;
/// hanya penjaga bentuk supaya alamat yang jelas salah tidak diterima.
const PHYSICAL_LIMIT: u64 = 1 << 40;

/// Batas alamat virtual yang dimodelkan.
const VIRTUAL_LIMIT: u64 = 1 << 48;

/// Alamat fisik. Medan privat: hanya [`PhysicalAddress::new`] yang dapat
/// membuatnya, dan konstruktor itu menolak bentuk yang tidak sah.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    /// `None` bila alamat tidak selaras halaman atau di luar batas model.
    pub const fn new(raw: u64) -> Option<Self> {
        if raw % PAGE_SIZE != 0 || raw >= PHYSICAL_LIMIT {
            None
        } else {
            Some(PhysicalAddress(raw))
        }
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Frame yang memuat alamat ini.
    pub const fn frame(self) -> Frame {
        Frame(self.0 / PAGE_SIZE)
    }
}

/// Alamat virtual.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtualAddress(u64);

impl VirtualAddress {
    /// `None` bila alamat di luar batas model. Alamat virtual **boleh**
    /// tidak selaras halaman — `fault` yang memutuskan.
    pub const fn new(raw: u64) -> Option<Self> {
        if raw >= VIRTUAL_LIMIT {
            None
        } else {
            Some(VirtualAddress(raw))
        }
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Halaman yang memuat alamat ini.
    pub const fn page(self) -> Page {
        Page(self.0 / PAGE_SIZE)
    }

    /// Offset di dalam halaman.
    pub const fn offset(self) -> u64 {
        self.0 % PAGE_SIZE
    }
}

/// Nomor halaman **virtual**.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page(u64);

impl Page {
    pub const fn new(number: u64) -> Option<Self> {
        if number >= VIRTUAL_LIMIT / PAGE_SIZE {
            None
        } else {
            Some(Page(number))
        }
    }

    pub const fn number(self) -> u64 {
        self.0
    }

    /// Alamat awal halaman ini.
    pub const fn start(self) -> Option<VirtualAddress> {
        VirtualAddress::new(self.0 * PAGE_SIZE)
    }
}

/// Nomor **frame** fisik.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(u64);

impl Frame {
    pub const fn new(number: u64) -> Option<Self> {
        if number >= PHYSICAL_LIMIT / PAGE_SIZE {
            None
        } else {
            Some(Frame(number))
        }
    }

    pub const fn number(self) -> u64 {
        self.0
    }

    pub const fn start(self) -> Option<PhysicalAddress> {
        PhysicalAddress::new(self.0 * PAGE_SIZE)
    }
}

/// Satu pemetaan halaman → frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mapping {
    page: Page,
    frame: Frame,
    rights: u32,
    present: bool,
}

impl Mapping {
    pub const fn page(&self) -> Page {
        self.page
    }
    pub const fn frame(&self) -> Frame {
        self.frame
    }
    pub const fn rights(&self) -> u32 {
        self.rights
    }
    pub const fn present(&self) -> bool {
        self.present
    }
}

/// Hasil klasifikasi fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    /// Pemetaan ada dan halaman hadir — bukan fault.
    TidakBermasalah,
    /// Tidak ada pemetaan untuk halaman itu.
    TidakDipetakan,
    /// Pemetaan ada tetapi haknya tidak memuat hak yang diminta.
    HakKurang,
}

/// Galat model memori. **Semua** negatif gagal-tertutup: tidak ada varian yang
/// berarti "lanjut saja".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryError {
    /// Alamat/halaman/frame tidak sah menurut bentuknya.
    AlamatTidakSah,
    /// Halaman itu sudah dipetakan.
    SudahDipetakan,
    /// Halaman itu belum dipetakan.
    TidakDipetakan,
    /// Ruang alamat penuh.
    Penuh,
    /// Token izin tidak ada atau bukan untuk operasi ini.
    IzinTidakSah,
}

/// Ruang alamat: himpunan pemetaan yang tetap dan tanpa alokasi.
#[derive(Clone, Copy, Debug)]
pub struct AddressSpace {
    mappings: [Option<Mapping>; MAX_MAPPINGS],
    count: usize,
}

impl AddressSpace {
    /// Ruang alamat kosong.
    pub const fn new() -> Self {
        AddressSpace { mappings: [None; MAX_MAPPINGS], count: 0 }
    }

    /// Jumlah pemetaan aktif.
    pub const fn len(&self) -> usize {
        self.count
    }

    /// `true` bila belum ada pemetaan.
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Pemetaan untuk `page`, bila ada.
    pub fn lookup(&self, page: Page) -> Option<Mapping> {
        for slot in self.mappings.iter().flatten() {
            if slot.page == page {
                return Some(*slot);
            }
        }
        None
    }

    /// Menerjemahkan halaman virtual ke frame fisik, bila terpetakan.
    pub fn translate(&self, page: Page) -> Option<Frame> {
        self.lookup(page).map(|mapping| mapping.frame)
    }

    /// Memetakan `page` ke `frame` dengan hak `rights`.
    ///
    /// `izin` **wajib** token untuk `(MEMORY, op_mem::MODIFY)`; token untuk
    /// operasi lain ditolak dengan [`MemoryError::IzinTidakSah`], termasuk
    /// token lintas otoritas (mis. token TASK).
    pub fn map(&mut self, page: Page, frame: Frame, rights: u32, izin: Option<Izin>) -> Result<(), MemoryError> {
        let izin = izin.ok_or(MemoryError::IzinTidakSah)?;
        if !izin.untuk(abi2::object_kind::MEMORY, dispatch::op_mem::MODIFY) {
            return Err(MemoryError::IzinTidakSah);
        }
        if !abi2::rights_valid_for_kind(abi2::object_kind::MEMORY, rights) {
            return Err(MemoryError::IzinTidakSah);
        }
        if self.lookup(page).is_some() {
            return Err(MemoryError::SudahDipetakan);
        }
        let slot = match self.mappings.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => slot,
            None => return Err(MemoryError::Penuh),
        };
        *slot = Some(Mapping { page, frame, rights, present: true });
        self.count += 1;
        Ok(())
    }

    /// Melepas pemetaan `page`. Mengembalikan frame yang dilepas.
    pub fn unmap(&mut self, page: Page, izin: Option<Izin>) -> Result<Frame, MemoryError> {
        let izin = izin.ok_or(MemoryError::IzinTidakSah)?;
        if !izin.untuk(abi2::object_kind::MEMORY, dispatch::op_mem::MODIFY) {
            return Err(MemoryError::IzinTidakSah);
        }
        let index = match self
            .mappings
            .iter()
            .position(|slot| matches!(slot, Some(mapping) if mapping.page == page))
        {
            Some(index) => index,
            None => return Err(MemoryError::TidakDipetakan),
        };
        let frame = match self.mappings[index] {
            Some(mapping) => mapping.frame,
            None => return Err(MemoryError::TidakDipetakan),
        };
        self.mappings[index] = None;
        self.count -= 1;
        Ok(frame)
    }

    /// Mengklasifikasi akses ke `address` dengan hak `wanted`.
    pub fn fault(&self, address: VirtualAddress, wanted: u32) -> FaultKind {
        match self.lookup(address.page()) {
            None => FaultKind::TidakDipetakan,
            Some(mapping) => {
                if !mapping.present {
                    FaultKind::TidakDipetakan
                } else if abi2::rights_contain(mapping.rights, wanted) {
                    FaultKind::TidakBermasalah
                } else {
                    FaultKind::HakKurang
                }
            }
        }
    }
}

impl Default for AddressSpace {
    fn default() -> Self {
        Self::new()
    }
}

/// Izin untuk operasi memori, atau `None` bila KSM menolak.
///
/// `hak_dimiliki` berasal dari KSM C++ yang nyata — lihat [`crate::ksm::Izin`].
pub const fn izin_memori(hak_dimiliki: u32) -> Option<Izin> {
    match crate::ksm::periksa_operasi(
        abi2::object_kind::MEMORY,
        dispatch::op_mem::MODIFY,
        hak_dimiliki,
    ) {
        Keputusan::Diizinkan(izin) => Some(izin),
        Keputusan::Ditolak(_) => None,
    }
}
