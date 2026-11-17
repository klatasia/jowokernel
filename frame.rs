// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P0.99 — **OTORITAS FRAME FISIK PRODUKSI** (bukan model, bukan prototipe).


use crate::ksm::{self, Izin, Keputusan};
use crate::{abi2, dispatch};

/// Ukuran halaman. Sama dengan `PAGE_SIZE` di `Paket/Platform.h`.
pub const PAGE_SIZE: u64 = 4096;

/// Satu chunk = setengah halaman. Sama dengan `PhysicalZone::ZONE_CHUNK_SIZE`.
pub const ZONE_CHUNK_SIZE: u64 = PAGE_SIZE / 2;

/// Tingkat buddy maksimum. Sama dengan `PhysicalZone::max_order`.
pub const MAX_ORDER: usize = 12;

/// Jumlah zona maksimum. Zona terbesar 16 MiB (8192 chunk); 1024 zona menampung
/// 16 GiB dalam zona 16 MiB — jauh di atas `-m 2G` bawaan `Rancang/run.py`.
/// Pendaftaran melebihi batas ini **gagal-tertutup** (`-1`).
pub const MAX_ZONES: usize = 1024;

/// Cermin `Jowo/Kontrak/KlatKernelBridge.h::KlatosZoneBucket`.
///
/// C++ menyediakan bit byte bitmap per order; Rust yang menafsirkannya.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosZoneBucket {
    /// Bit byte bitmap (dialokasikan C++). `null` bila `bits == 0`.
    pub bitmap: *mut u8,
    /// Jumlah bit sah di dalam bitmap.
    pub bits: u32,
    /// Padding eksplisit supaya tata letak identik di kedua sisi.
    pub _pad: u32,
}

// ===========================================================================
// Keadaan per zona
// ===========================================================================

#[derive(Clone, Copy)]
struct Zone {
    /// Indeks halaman fisik dari alamat basis zona (`base / PAGE_SIZE`).
    base_page_index: u64,
    /// Jumlah chunk (`page_count * 2`).
    chunk_count: u32,
    /// Order tertinggi zona ini — order tempat seluruh zona menjadi satu blok.
    top_order: usize,
    /// Chunk terpakai. **Rust satu-satunya pemilik angka ini.**
    used_chunks: u32,
    /// Basis array `PhysicalPageEntry` (media penyimpanan tautan freelist).
    entries: *mut u8,
    /// Langkah antar-entri (`sizeof(PhysicalPageEntry)`).
    stride: u32,
    /// `true` bila slot ini sudah dipakai.
    live: bool,
    /// Bitmap per order (milik C++, ditafsirkan Rust).
    bitmap: [*mut u8; MAX_ORDER + 1],
    /// Jumlah bit sah per order.
    bits: [u32; MAX_ORDER + 1],
    /// Kepala freelist per order. `-1` = kosong. **Rust pemiliknya.**
    freelist: [i16; MAX_ORDER + 1],
}

const EMPTY_ZONE: Zone = Zone {
    base_page_index: 0,
    chunk_count: 0,
    top_order: 0,
    used_chunks: 0,
    entries: core::ptr::null_mut(),
    stride: 0,
    live: false,
    bitmap: [core::ptr::null_mut(); MAX_ORDER + 1],
    bits: [0; MAX_ORDER + 1],
    freelist: [-1; MAX_ORDER + 1],
};

static mut ZONES: [Zone; MAX_ZONES] = [EMPTY_ZONE; MAX_ZONES];
static mut LIVE_ZONES: u32 = 0;
static mut TOTAL_ALLOC: u64 = 0;
static mut TOTAL_FREE: u64 = 0;
static mut TOTAL_ERR: u64 = 0;

/// Galat otoritas frame. **Semua varian gagal-tertutup.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// `handle` tidak menunjuk zona hidup.
    HandleTidakSah,
    /// `order` di luar `0..=MAX_ORDER`.
    OrderTidakSah,
    /// Indeks di luar zona, atau ganjil (indeks sah selalu genap).
    IndeksTidakSah,
    /// Tidak ada blok bebas pada order yang diminta maupun di atasnya.
    Habis,
    /// Pembebasan akan membuat `used_chunks` di bawah nol.
    PembebasanBerlebih,
    /// Invariant buddy rusak.
    InvariantRusak,
    /// Merge akan melewati order tertinggi zona (dijaga, lihat F-P099-01).
    MergeDiLuarBatas,
}

/// Statistik otoritas untuk laporan runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameStats {
    pub live_zones: u32,
    pub total_alloc: u64,
    pub total_free: u64,
    pub total_err: u64,
}

// ===========================================================================
// Akses media penyimpanan (satu-satunya tempat `unsafe` di modul ini)
// ===========================================================================

#[inline]
fn zone_mut(handle: i32) -> Option<&'static mut Zone> {
    if handle < 0 || (handle as usize) >= MAX_ZONES {
        return None;
    }
    // SAFETY: seluruh pemanggil produksi berada di dalam Spinlock<GlobalData>
    // C++ yang sama (dokumentasi modul), jadi tidak ada dua penulis serentak.
    // `handle` sudah diperiksa batasnya di atas.
    let zone = unsafe { &mut ZONES[handle as usize] };
    if zone.live {
        Some(zone)
    } else {
        None
    }
}

#[inline]
fn zone_ref(handle: i32) -> Option<&'static Zone> {
    if handle < 0 || (handle as usize) >= MAX_ZONES {
        return None;
    }
    // SAFETY: sama seperti `zone_mut`; di sini hanya dibaca.
    let zone = unsafe { &ZONES[handle as usize] };
    if zone.live {
        Some(zone)
    } else {
        None
    }
}

/// Indeks bit di dalam bitmap untuk pasangan buddy.
///
/// Cermin `BuddyBucket::buddy_bit_index`: `(index >> 1) >> (1 + order)`.
#[inline]
const fn buddy_bit_index(index: i32, order: usize) -> usize {
    (((index >> 1) >> 1) >> order) as usize
}

#[inline]
fn bitmap_get(zone: &Zone, order: usize, index: i32) -> bool {
    if zone.bits[order] == 0 || zone.bitmap[order].is_null() {
        return false;
    }
    let bit = buddy_bit_index(index, order);
    if bit as u32 >= zone.bits[order] {
        return false;
    }
    // SAFETY: `bit < bits`, dan C++ menyediakan `ceil_div(bits, 8)` byte.
    unsafe { (*zone.bitmap[order].add(bit / 8) & (1u8 << (bit % 8))) != 0 }
}

#[inline]
fn bitmap_set(zone: &Zone, order: usize, index: i32, value: bool) {
    if zone.bits[order] == 0 || zone.bitmap[order].is_null() {
        return;
    }
    let bit = buddy_bit_index(index, order);
    if bit as u32 >= zone.bits[order] {
        return;
    }
    // SAFETY: sama seperti `bitmap_get`.
    unsafe {
        let byte = zone.bitmap[order].add(bit / 8);
        if value {
            *byte |= 1u8 << (bit % 8);
        } else {
            *byte &= !(1u8 << (bit % 8));
        }
    }
}

/// Penunjuk ke entri freelist untuk `index` (selalu genap).
///
/// Cermin `PhysicalZone::get_freelist_entry`:
/// `MM.get_physical_page_entry(base.offset(index * ZONE_CHUNK_SIZE))`, yaitu
/// entri ke-`base_page_index + index/2`.
#[inline]
fn freelist_ptr(zone: &Zone, index: i32) -> *mut u8 {
    let entry_index = zone.base_page_index + (index as u64) / 2;
    // SAFETY: pemanggil sudah memastikan `valid_index(zone, index)`, sehingga
    // `entry_index < entry_count` (dijamin saat register).
    unsafe { zone.entries.add((entry_index * zone.stride as u64) as usize) }
}

#[inline]
fn freelist_next(zone: &Zone, index: i32) -> i16 {
    // SAFETY: `freelist_ptr` mengembalikan penunjuk sah untuk entri ini.
    unsafe { *(freelist_ptr(zone, index) as *const i16) }
}

#[inline]
fn freelist_prev(zone: &Zone, index: i32) -> i16 {
    // SAFETY: `prev_index` berada pada offset 2 (cermin
    // `PhysicalPageEntry::freelist`).
    unsafe { *(freelist_ptr(zone, index) as *const i16).add(1) }
}

#[inline]
fn freelist_set(zone: &Zone, index: i32, next: i16, prev: i16) {
    // SAFETY: tata letak `{ i16 next; i16 prev; }` pada offset 0 dan 2 —
    // sama persis dengan yang ditulis C++ sebelum P0.99.
    unsafe {
        let p = freelist_ptr(zone, index) as *mut i16;
        *p = next;
        *p.add(1) = prev;
    }
}

// ===========================================================================
// Algoritma buddy — INTI KEPUTUSAN
// ===========================================================================

#[inline]
fn block_size_chunks(order: usize) -> i32 {
    (2u32 << order) as i32
}

#[inline]
fn valid_index(zone: &Zone, index: i32) -> bool {
    index >= 0 && (index as u32) < zone.chunk_count && (index & 1) == 0
}

/// Melepas `index` dari freelist `order`. Cermin `remove_from_freelist`.
fn remove_from_freelist(zone: &mut Zone, order: usize, index: i32) {
    let next = freelist_next(zone, index);
    let prev = freelist_prev(zone, index);
    if prev != -1 {
        freelist_set(zone, prev as i32, next, freelist_prev(zone, prev as i32));
    }
    if next != -1 {
        freelist_set(zone, next as i32, freelist_next(zone, next as i32), prev);
    }
    if zone.freelist[order] == index as i16 {
        zone.freelist[order] = next;
    }
    freelist_set(zone, index, -1, -1);
}

/// Menaruh `index` di kepala freelist `order`.
fn push_freelist(zone: &mut Zone, order: usize, index: i32) {
    let head = zone.freelist[order];
    if head != -1 {
        freelist_set(zone, head as i32, freelist_next(zone, head as i32), index as i16);
    }
    freelist_set(zone, index, head, -1);
    zone.freelist[order] = index as i16;
}

/// Mengambil kepala freelist `order`. `None` bila kosong.
fn pop_freelist(zone: &mut Zone, order: usize) -> Option<i32> {
    let head = zone.freelist[order];
    if head == -1 {
        return None;
    }
    let index = head as i32;
    let next = freelist_next(zone, index);
    zone.freelist[order] = next;
    if next != -1 {
        freelist_set(zone, next as i32, freelist_next(zone, next as i32), -1);
    }
    Some(index)
}

/// Mengalokasi satu blok `2 << order` chunk. Mengembalikan indeks chunk.
///
/// Cermin `PhysicalZone::allocate_block_impl` — tetapi **iteratif**, sehingga
/// tidak ada reborrow `&mut` bersarang dan tidak ada rekursi tanpa batas.
fn alloc_impl(zone: &mut Zone, order: usize) -> Result<i32, FrameError> {
    if order > MAX_ORDER {
        return Err(FrameError::OrderTidakSah);
    }

    // 1. Cari bucket tidak kosong pada order >= yang diminta.
    let mut src = order;
    while src <= MAX_ORDER && zone.freelist[src] == -1 {
        src += 1;
    }
    if src > MAX_ORDER {
        return Err(FrameError::Habis);
    }

    // 2. Ambil satu blok dari bucket `src`.
    let mut index = pop_freelist(zone, src).ok_or(FrameError::Habis)?;
    if !valid_index(zone, index) {
        return Err(FrameError::IndeksTidakSah);
    }
    // Invariant C++: kepala freelist selalu punya bit buddy = true.
    if !bitmap_get(zone, src, index) {
        return Err(FrameError::InvariantRusak);
    }
    bitmap_set(zone, src, index, false);

    // 3. Pecah turun dari `src` ke `order`: paruh pertama masuk freelist,
    //    paruh kedua yang dikembalikan.
    let mut cur = src;
    while cur > order {
        cur -= 1;
        push_freelist(zone, cur, index);
        if bitmap_get(zone, cur, index) {
            return Err(FrameError::InvariantRusak);
        }
        bitmap_set(zone, cur, index, true);
        index += block_size_chunks(cur);
    }

    zone.used_chunks += 2u32 << order;
    Ok(index)
}

/// Membebaskan blok `2 << order` chunk pada `index`.
///
/// Cermin `PhysicalZone::deallocate_block_impl` — juga iteratif, dengan satu
/// perbedaan yang disengaja: merge **berhenti di `top_order`**. C++ melanjutkan
/// ke `order + 1` sehingga pada zona 16 MiB membaca `m_buckets[13]` di luar
/// array dan menyentuh entri freelist di luar zona (F-P099-01).
fn free_impl(zone: &mut Zone, index: i32, order: usize) -> Result<(), FrameError> {
    if order > MAX_ORDER {
        return Err(FrameError::OrderTidakSah);
    }
    if !valid_index(zone, index) {
        return Err(FrameError::IndeksTidakSah);
    }
    let total = 2u32 << order;
    if zone.used_chunks < total {
        // Membebaskan lebih banyak daripada yang pernah dialokasi: gagal-tertutup.
        return Err(FrameError::PembebasanBerlebih);
    }

    let mut cur = order;
    let mut idx = index;
    loop {
        if bitmap_get(zone, cur, idx) {
            // Buddy bebas -> gabung.
            if cur == zone.top_order {
                // Tidak ada order di atasnya. Bit pasangan pada order puncak
                // bersifat degenerate (pasangannya tidak ada); benarkan keadaan
                // lalu tempatkan blok utuh di freelist puncak.
                bitmap_set(zone, cur, idx, false);
                push_freelist(zone, cur, idx);
                break;
            }
            let buddy_bit = ((idx >> 1) >> 1) >> cur;
            let buddy_base = (buddy_bit << 1) << (1 + cur);
            let buddy = if idx == buddy_base {
                buddy_base + block_size_chunks(cur)
            } else {
                buddy_base
            };
            if !valid_index(zone, buddy) {
                return Err(FrameError::MergeDiLuarBatas);
            }
            remove_from_freelist(zone, cur, buddy);
            bitmap_set(zone, cur, idx, false);
            idx = buddy_base;
            cur += 1;
            continue;
        }

        // Buddy terpakai -> taruh di freelist dan tandai.
        push_freelist(zone, cur, idx);
        bitmap_set(zone, cur, idx, true);
        break;
    }

    zone.used_chunks -= total;
    Ok(())
}

// ===========================================================================
// Otoritas KSM (P0.99.4) — TIDAK ADA tabel kedua
// ===========================================================================

/// Izin untuk operasi frame, atau `None` bila KSM menolak.
///
/// Memakai operasi KSM yang SUDAH ada: `(MEMORY, MODIFY)`. Tidak ada opcode
/// baru dan tidak ada tabel kapabilitas kedua.
pub const fn izin_frame(hak_dimiliki: u32) -> Option<Izin> {
    match ksm::periksa_operasi(
        abi2::object_kind::MEMORY,
        dispatch::op_mem::MODIFY,
        hak_dimiliki,
    ) {
        Keputusan::Diizinkan(izin) => Some(izin),
        Keputusan::Ditolak(_) => None,
    }
}

// ===========================================================================
// Permukaan C ABI
// ===========================================================================

/// Mendaftarkan satu zona. `handle >= 0`, atau `-1` bila gagal-tertutup.
///
/// # Safety (kontrak C ABI)
/// `buckets` menunjuk `MAX_ORDER + 1` `KlatosZoneBucket`; `entries` menunjuk
/// array `PhysicalPageEntry` milik kernel; `stride` = `sizeof(PhysicalPageEntry)`.
/// Bitmap boleh `null` bila `bits == 0`.
#[no_mangle]
pub unsafe extern "C" fn klatos_mem_zone_register(
    base_page_index: u64,
    chunk_count: u32,
    buckets: *const KlatosZoneBucket,
    entries: *mut u8,
    stride: u32,
    entry_count: u64,
) -> i32 {
    if buckets.is_null() || entries.is_null() || stride < 4 || chunk_count < 2 {
        return -1;
    }
    if chunk_count % 2 != 0 {
        return -1;
    }
    let handle = unsafe { LIVE_ZONES };
    if (handle as usize) >= MAX_ZONES {
        return -1; // gagal-tertutup: tidak ada slot
    }

    // Order puncak: order tempat seluruh zona menjadi satu blok. Zona selalu
    // berukuran pangkat dua (16 MiB / 1 MiB), jadi ini selalu eksak.
    let top_order = (31 - chunk_count.leading_zeros() - 1) as usize;
    if block_size_chunks(top_order) as u32 != chunk_count {
        return -1; // bentuk zona tidak didukung -> gagal-tertutup
    }

    // Entri terakhir yang akan disentuh: base + (chunk_count - 1) / 2.
    let last_entry = base_page_index + (chunk_count as u64 - 1) / 2;
    if last_entry >= entry_count {
        return -1; // array entri tidak cukup -> gagal-tertutup
    }

    let slot = unsafe { &mut ZONES[handle as usize] };
    *slot = EMPTY_ZONE;
    slot.base_page_index = base_page_index;
    slot.chunk_count = chunk_count;
    slot.top_order = top_order;
    slot.entries = entries;
    slot.stride = stride;

    for order in 0..=MAX_ORDER {
        let bucket = unsafe { &*buckets.add(order) };
        slot.bitmap[order] = bucket.bitmap;
        slot.bits[order] = bucket.bits;
        if !bucket.bitmap.is_null() && bucket.bits > 0 {
            // Nolkan sendiri: setelah titik ini C++ tidak pernah menulisnya lagi.
            let bytes = (bucket.bits as usize + 7) / 8;
            unsafe { core::ptr::write_bytes(bucket.bitmap, 0, bytes) };
        }
    }

    if slot.bits[top_order] == 0 {
        return -1; // bitmap order puncak tidak disediakan
    }

    // Keadaan awal: satu blok utuh pada order puncak, index 0.
    push_freelist(slot, top_order, 0);
    bitmap_set(slot, top_order, 0, true);
    slot.used_chunks = 0;
    slot.live = true;

    unsafe { LIVE_ZONES = handle + 1 };
    handle as i32
}

/// Mengalokasi blok `2 << order` chunk dari zona `handle`.
///
/// Mengembalikan indeks chunk (`>= 0`, selalu genap) atau negatif bila gagal.
/// Ini jalur produksi kernel: ia TIDAK memeriksa hak, karena alokasi frame oleh
/// kernel adalah primitif kernel, bukan operasi ber-kapabilitas. Otoritas
/// memori ditegakkan di lapisan KSM/pemanggilan sistem, bukan di sini: tidak
/// ada varian ber-kapabilitas dari fungsi ini (D2-F4, 2026-11-09).
#[no_mangle]
pub extern "C" fn klatos_mem_zone_alloc(handle: i32, order: u32) -> i32 {
    let Some(zone) = zone_mut(handle) else {
        unsafe { TOTAL_ERR += 1 };
        return -1;
    };
    match alloc_impl(zone, order as usize) {
        Ok(index) => {
            unsafe { TOTAL_ALLOC += 1 };
            index
        }
        Err(_) => {
            unsafe { TOTAL_ERR += 1 };
            -1
        }
    }
}

/// Membebaskan blok pada `chunk_index`. `0` = berhasil, negatif = gagal.
#[no_mangle]
pub extern "C" fn klatos_mem_zone_free(handle: i32, chunk_index: i32, order: u32) -> i32 {
    let Some(zone) = zone_mut(handle) else {
        unsafe { TOTAL_ERR += 1 };
        return -1;
    };
    match free_impl(zone, chunk_index, order as usize) {
        Ok(()) => {
            unsafe { TOTAL_FREE += 1 };
            0
        }
        Err(_) => {
            unsafe { TOTAL_ERR += 1 };
            -1
        }
    }
}

/// Chunk terpakai di zona `handle`. `u64::MAX` bila handle tidak sah.
#[no_mangle]
pub extern "C" fn klatos_mem_zone_used_chunks(handle: i32) -> u64 {
    match zone_ref(handle) {
        Some(zone) => zone.used_chunks as u64,
        None => u64::MAX,
    }
}

/// Chunk bebas di zona `handle`. `u64::MAX` bila handle tidak sah.
#[no_mangle]
pub extern "C" fn klatos_mem_zone_available(handle: i32) -> u64 {
    match zone_ref(handle) {
        Some(zone) => (zone.chunk_count - zone.used_chunks) as u64,
        None => u64::MAX,
    }
}

/// Jumlah zona hidup.
#[no_mangle]
pub extern "C" fn klatos_mem_zone_count() -> u32 {
    unsafe { LIVE_ZONES }
}

/// Statistik: `[live_zones, total_alloc, total_free, total_err]`. `0` = sukses.
#[no_mangle]
pub extern "C" fn klatos_mem_zone_stats(out: *mut u64) -> u32 {
    if out.is_null() {
        return 1;
    }
    // SAFETY: pemanggil menyediakan 4 slot `u64`.
    unsafe {
        *out = LIVE_ZONES as u64;
        *out.add(1) = TOTAL_ALLOC;
        *out.add(2) = TOTAL_FREE;
        *out.add(3) = TOTAL_ERR;
    }
    0
}

/// Mencetak keadaan satu zona ke serial. Hanya diagnosis.
#[no_mangle]
pub extern "C" fn klatos_mem_zone_dump(handle: i32) {
    match zone_ref(handle) {
        Some(zone) => crate::serial::write_fmt(format_args!(
            "frame: zona {} chunk={} used={} bebas={}\n",
            handle,
            zone.chunk_count,
            zone.used_chunks,
            zone.chunk_count - zone.used_chunks
        )),
        None => crate::serial::write_str("frame: handle tidak sah\n"),
    }
}

// ===========================================================================
// Matriks runtime (dipakai `klatos_mem_frame_selftest`)
// ===========================================================================

/// Cermin `KlatosFrameReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosFrameReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub alloc_checks: u32,
    pub alloc_passed: u32,
    pub free_checks: u32,
    pub free_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub auth_checks: u32,
    pub auth_passed: u32,
    pub live_zones: u32,
    pub prod_handle: i32,
    pub prod_chunk: i32,
    pub prod_before: u64,
    pub prod_after_alloc: u64,
    pub prod_after_free: u64,
    pub status: i32,
}

/// Tag laporan; sama dengan `kernel_tag_expected` C++.
const REPORT_TAG: u64 = 0x4B4C_4154_4B52_4E4C; // "KLATKRNL"

// --- media penyimpanan zona SINTETIS untuk matriks --------------------------
//
// Matriks berjalan pada zona sintetis, bukan zona produksi, sehingga ia dapat
// menguji kehabisan, indeks tidak sah, dan pembebasan berlebih TANPA menyentuh
// memori fisik nyata. Zona produksi diuji terpisah oleh
// `MemoryManager::probe_frame_authority()` DI BAWAH kunci produksi.

/// 64 halaman = 128 chunk. Order puncak = 6.
const TEST_CHUNKS: u32 = 128;
/// Langkah entri sintetis: cukup untuk `{ i16 next; i16 prev; }`.
const TEST_STRIDE: u32 = 4;
/// 128 bit per order sudah lebih dari cukup (order 0 butuh 32 bit).
const TEST_BITMAP_BYTES: usize = 16;

static mut TEST_BITMAPS: [[u8; TEST_BITMAP_BYTES]; MAX_ORDER + 1] =
    [[0u8; TEST_BITMAP_BYTES]; MAX_ORDER + 1];
static mut TEST_ENTRIES: [u8; (TEST_CHUNKS as usize / 2) * (TEST_STRIDE as usize)] =
    [0u8; (TEST_CHUNKS as usize / 2) * (TEST_STRIDE as usize)];
/// Bitmap "sedang dipakai" — HANYA kontrol negatif di dalam matriks. Ini bukan
/// keadaan produksi dan tidak ada di dalam `Zone`.
static mut TEST_INUSE: [u64; (TEST_CHUNKS as usize + 63) / 64] =
    [0u64; (TEST_CHUNKS as usize + 63) / 64];

struct Matrix {
    checks: u32,
    passed: u32,
    alloc_checks: u32,
    alloc_passed: u32,
    free_checks: u32,
    free_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
    auth_checks: u32,
    auth_passed: u32,
}

impl Matrix {
    fn catat(&mut self, lulus: bool, group: u8) {
        self.checks += 1;
        if lulus {
            self.passed += 1;
        }
        match group {
            1 => {
                self.alloc_checks += 1;
                if lulus {
                    self.alloc_passed += 1;
                }
            }
            2 => {
                self.free_checks += 1;
                if lulus {
                    self.free_passed += 1;
                }
            }
            3 => {
                self.neg_checks += 1;
                if lulus {
                    self.neg_passed += 1;
                }
            }
            4 => {
                self.auth_checks += 1;
                if lulus {
                    self.auth_passed += 1;
                }
            }
            _ => {}
        }
    }
}

fn inuse_get(index: i32) -> bool {
    let i = index as usize;
    // SAFETY: `i < TEST_CHUNKS`.
    unsafe { (TEST_INUSE[i / 64] & (1u64 << (i % 64))) != 0 }
}

fn inuse_set(index: i32, value: bool) {
    let i = index as usize;
    // SAFETY: sama seperti `inuse_get`.
    unsafe {
        if value {
            TEST_INUSE[i / 64] |= 1u64 << (i % 64);
        } else {
            TEST_INUSE[i / 64] &= !(1u64 << (i % 64));
        }
    }
}

/// Menjalankan matriks otoritas frame pada zona sintetis.
///
/// `hak_memori` berasal dari tabel KSM C++ yang NYATA (dibaca lewat `lihat()`),
/// bukan dikarang — lihat `init.cpp` P0.99.
#[no_mangle]
pub extern "C" fn klatos_mem_frame_selftest(hak_memori: u32) -> KlatosFrameReport {
    let mut m = Matrix {
        checks: 0,
        passed: 0,
        alloc_checks: 0,
        alloc_passed: 0,
        free_checks: 0,
        free_passed: 0,
        neg_checks: 0,
        neg_passed: 0,
        auth_checks: 0,
        auth_passed: 0,
    };

    // ---- siapkan zona sintetis --------------------------------------------
    let mut buckets = [KlatosZoneBucket { bitmap: core::ptr::null_mut(), bits: 0, _pad: 0 }; MAX_ORDER + 1];
    for order in 0..=MAX_ORDER {
        let block_size = 2u32 << order;
        if block_size > TEST_CHUNKS {
            continue;
        }
        let pairs = (TEST_CHUNKS / block_size + 1) / 2;
        if pairs > 0 {
            // `addr_of_mut!` menghindari pembentukan referensi ke static mut.
            buckets[order].bitmap =
                unsafe { core::ptr::addr_of_mut!(TEST_BITMAPS[order]) as *mut u8 };
            buckets[order].bits = pairs;
        }
    }
    unsafe { TEST_INUSE = [0u64; (TEST_CHUNKS as usize + 63) / 64] };
    let handle = unsafe {
        klatos_mem_zone_register(
            0,
            TEST_CHUNKS,
            buckets.as_ptr(),
            core::ptr::addr_of_mut!(TEST_ENTRIES) as *mut u8,
            TEST_STRIDE,
            (TEST_CHUNKS as u64) / 2,
        )
    };
    m.catat(handle >= 0, 0);

    let total = TEST_CHUNKS as u64;
    m.catat(klatos_mem_zone_used_chunks(handle) == 0, 1);
    m.catat(klatos_mem_zone_available(handle) == total, 1);

    // ---- 1. ALOKASI: order 0, 1, 3 ----------------------------------------
    let orders: [usize; 3] = [0, 1, 3];
    let mut seen: [i32; 3] = [-1; 3];
    for (n, order) in orders.iter().enumerate() {
        let idx = klatos_mem_zone_alloc(handle, *order as u32);
        m.catat(idx >= 0, 1);
        m.catat(idx >= 0 && idx % block_size_chunks(*order) == 0, 1); // sejajar
        m.catat(idx >= 0 && (idx & 1) == 0, 1); // paritas: selalu genap
        if idx >= 0 {
            for c in 0..block_size_chunks(*order) {
                inuse_set(idx + c, true);
            }
        }
        seen[n] = idx;
    }
    // 2 + 4 + 16 = 22 chunk
    m.catat(klatos_mem_zone_used_chunks(handle) == 22, 1);
    m.catat(klatos_mem_zone_available(handle) == total - 22, 1);

    // ---- 2. PEMBEBASAN: keadaan harus pulih persis ------------------------
    let mut free_ok = true;
    for (n, order) in orders.iter().enumerate() {
        let idx = seen[n];
        if idx < 0 || klatos_mem_zone_free(handle, idx, *order as u32) != 0 {
            free_ok = false;
            continue;
        }
        for c in 0..block_size_chunks(*order) {
            inuse_set(idx + c, false);
        }
    }
    m.catat(free_ok, 2);
    m.catat(klatos_mem_zone_used_chunks(handle) == 0, 2);
    m.catat(klatos_mem_zone_available(handle) == total, 2);

    // ---- 3. NEGATIF --------------------------------------------------------
    m.catat(klatos_mem_zone_alloc(-1, 0) == -1, 3); // handle negatif
    m.catat(klatos_mem_zone_alloc(MAX_ZONES as i32, 0) == -1, 3); // di luar batas
    m.catat(klatos_mem_zone_alloc(handle, (MAX_ORDER + 1) as u32) == -1, 3); // order tidak sah
    m.catat(klatos_mem_zone_alloc(handle, 11) == -1, 3); // order > order puncak zona
    m.catat(klatos_mem_zone_free(handle, -2, 0) == -1, 3); // indeks negatif
    m.catat(klatos_mem_zone_free(handle, 1, 0) == -1, 3); // indeks GANJIL
    m.catat(klatos_mem_zone_free(handle, TEST_CHUNKS as i32, 0) == -1, 3); // di luar zona
    m.catat(klatos_mem_zone_free(handle, 0, (MAX_ORDER + 1) as u32) == -1, 3); // order tidak sah
    m.catat(klatos_mem_zone_free(handle, 0, 3) == -1, 3); // bebaskan tanpa alokasi

    // Pembebasan ganda: alokasi, bebaskan, lalu bebaskan LAGI. Yang kedua harus
    // ditolak karena `used_chunks` untuk blok itu sudah nol.
    let dbl = klatos_mem_zone_alloc(handle, 2);
    let mut dbl_detected = false;
    if dbl >= 0 {
        if klatos_mem_zone_free(handle, dbl, 2) == 0 {
            dbl_detected = klatos_mem_zone_free(handle, dbl, 2) == -1;
        }
    }
    m.catat(dbl >= 0, 3);
    m.catat(dbl_detected, 3);

    // Kehabisan: 64 blok order-0 harus mengisi seluruh zona, lalu GAGAL-TERTUTUP.
    let mut got = 0i32;
    let mut exhausted = false;
    for _ in 0..(TEST_CHUNKS / 2 + 8) {
        let idx = klatos_mem_zone_alloc(handle, 0);
        if idx < 0 {
            exhausted = true;
            break;
        }
        if (idx as u32) >= TEST_CHUNKS {
            exhausted = false; // indeks di luar zona = kegagalan
            break;
        }
        inuse_set(idx, true);
        got += 1;
    }
    m.catat(exhausted, 3);
    m.catat(got == (TEST_CHUNKS / 2) as i32, 3);
    m.catat(klatos_mem_zone_available(handle) == 0, 3);
    m.catat(klatos_mem_zone_alloc(handle, 0) == -1, 3); // habis -> tolak
    // Pemulihan: kembalikan seluruhnya.
    let mut restored = true;
    for i in (0..TEST_CHUNKS as i32).step_by(2) {
        if inuse_get(i) {
            if klatos_mem_zone_free(handle, i, 0) != 0 {
                restored = false;
            }
            inuse_set(i, false);
        }
    }
    m.catat(restored, 3);
    m.catat(klatos_mem_zone_used_chunks(handle) == 0, 3);
    m.catat(klatos_mem_zone_available(handle) == total, 3);

    // ---- 4. OTORITAS KSM ---------------------------------------------------
    use crate::abi2::right;
    m.catat(izin_frame(hak_memori).is_some(), 4);
    m.catat(izin_frame(0).is_none(), 4);
    // Hak yang TIDAK SAH untuk kind MEMORY (SEND milik endpoint) -> ditolak.
    m.catat(izin_frame(right::SEND).is_none(), 4);
    // Hak yang SAH untuk MEMORY tetapi KURANG (READ, tanpa CONTROL) -> ditolak.
    m.catat(izin_frame(right::READ).is_none(), 4);
    // Jalur ber-otoritas TIDAK diuji di sini: D2-F4 menghapus
    // `klatos_mem_zone_alloc_authorized` karena tidak punya konsumen. Matriks
    // di atas membuktikan pemetaan hak -> keputusan; alokasi frame sendiri
    // adalah primitif kernel tanpa pemeriksaan hak.
    m.catat(klatos_mem_zone_used_chunks(handle) == 0, 4);
    m.catat(klatos_mem_zone_available(handle) == total, 4);

    let mut stats = [0u64; 4];
    let _ = klatos_mem_zone_stats(stats.as_mut_ptr());

    KlatosFrameReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        alloc_checks: m.alloc_checks,
        alloc_passed: m.alloc_passed,
        free_checks: m.free_checks,
        free_passed: m.free_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        auth_checks: m.auth_checks,
        auth_passed: m.auth_passed,
        live_zones: stats[0] as u32,
        prod_handle: -1,
        prod_chunk: -1,
        prod_before: 0,
        prod_after_alloc: 0,
        prod_after_free: 0,
        status: (m.checks - m.passed) as i32,
    }
}
