// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! KSM — persistent capability/object authority.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub const REPORT_TAG: u64 = 0x4B4C_4154_4B53_4D43; // "KLATKSMC"
pub const MAX_CAPS: usize = 4096;
pub const MAX_OBJS: usize = 2048;
pub const TRANSIT_TASK: u32 = u32::MAX;
pub const NONE: u32 = 0;

pub mod kind {
    pub const NONE: u16 = 0;
    pub const TASK: u16 = 1;
    pub const ENDPOINT: u16 = 2;
    pub const MEMORY: u16 = 3;
    pub const SERVICE: u16 = 4;
    pub const DEVICE: u16 = 5;
    pub const FILEREF: u16 = 6;
    /// F-P7-01 — Directory ABI v2 (kind 7). Cermin `Abi2::ObjectKind::Directory`
    /// (`Jowo/Kontrak/Abi2.h`). Sebelumnya ditolak `BAD_KIND`; sekarang KSM
    /// dapat MEMBUAT objek Directory supaya capability VFS dapat mengikat
    /// direktori NYATA (efek VFS dilaksanakan lapisan efek C++).
    pub const DIRECTORY: u16 = 7;
}

pub mod right {
    pub const NONE: u32 = 0x000;
    pub const READ: u32 = 0x001;
    pub const WRITE: u32 = 0x002;
    pub const EXECUTE: u32 = 0x004;
    pub const MAP: u32 = 0x008;
    pub const SEND: u32 = 0x010;
    pub const RECEIVE: u32 = 0x020;
    pub const DUPLICATE: u32 = 0x040;
    pub const TRANSFER: u32 = 0x080;
    pub const CONTROL: u32 = 0x100;
    pub const REVOKE: u32 = 0x200;
    pub const ALL: u32 = 0x3FF;
}

pub mod status {
    pub const OK: i32 = 0;
    pub const BAD_HANDLE: i32 = 1;
    pub const MISSING_RIGHTS: i32 = 2;
    pub const REVOKED: i32 = 3;
    pub const BAD_OBJECT: i32 = 4;
    pub const BAD_KIND: i32 = 5;
    pub const CAPACITY: i32 = 6;
    pub const INVALID: i32 = 7;
}

// Himpunan hak "pembuat" per kind. Cermin `Hak*::pembuat` di
// `KlatIpc/Kanal/OtoritasEndpoint.h`; SENGAJA bukan `right::ALL`. Nilai-nilai ini
// dipakai transaksi bootstrap (proses baru) supaya C++ tidak perlu menghitung
// ulang matriks hak saat menyerahkan operasi gabungan ke Rust.
pub mod hak {
    use super::right;
    pub const ENDPOINT_PEMBUAT: u32 = right::SEND | right::RECEIVE | right::DUPLICATE
        | right::TRANSFER | right::CONTROL | right::REVOKE; // 0x3F0
    pub const TASK_PEMBUAT: u32 = right::READ | right::WRITE | right::DUPLICATE
        | right::TRANSFER | right::CONTROL | right::REVOKE; // 0x3C3
    pub const MEMORY_PEMBUAT: u32 = right::READ | right::WRITE | right::MAP | right::EXECUTE
        | right::DUPLICATE | right::TRANSFER | right::CONTROL | right::REVOKE; // 0x3CF
    pub const DEVICE_PEMBUAT: u32 = right::READ | right::WRITE | right::MAP
        | right::DUPLICATE | right::TRANSFER | right::CONTROL | right::REVOKE; // 0x3CB
    /// F-P7-01 — hak pembuat Directory: baca/telusuri + tulis (mutasi VFS).
    pub const DIREKTORI_PEMBUAT: u32 = right::READ | right::WRITE | right::EXECUTE
        | right::DUPLICATE | right::TRANSFER | right::CONTROL | right::REVOKE; // 0x3C7
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmObjectId {
    pub kind: u16,
    pub index: u16,
    pub generation: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmObjectOut {
    pub status: i32,
    pub id: KsmObjectId,
    pub state: u8,
    pub reserved: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmCapabilityOut {
    pub status: i32,
    pub id: KsmObjectId,
    pub rights: u32,
    pub handle: u32,
    pub task: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmTransitOut {
    pub status: i32,
    pub slot: u32,
    pub rights: u32,
    pub id: KsmObjectId,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmCapabilityReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub failed: u32,
    pub reserved: u32,
    pub status: i32,
}

// Hasil transaksi dua objek / dua pemasangan (klatipc create-pair).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmPairOut {
    pub status: i32,
    pub a: KsmObjectOut,
    pub b: KsmObjectOut,
}

// Hasil bootstrap proses baru: objek Task + objek Memory.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmProsesOut {
    pub status: i32,
    pub task: KsmObjectId,
    pub memory: KsmObjectId,
}

// Keluaran resolver ABI v2. Tata letaknya WAJIB identik dengan
// `KlatosAbi2ObjectOut` (`Jowo/Kontrak/Abi2DispatchKernel.h`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmResolveObjectOut {
    pub status: i32,
    pub kind: u16,
    pub index: u16,
    pub generation: u32,
    pub state: u8,
    pub reserved0: u8,
    pub reserved1: u16,
}

// Keluaran resolver ABI v2. Tata letaknya WAJIB identik dengan
// `KlatosAbi2CapabilityOut` (`Jowo/Kontrak/Abi2DispatchKernel.h`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KsmResolveCapabilityOut {
    pub status: i32,
    pub kind: u16,
    pub index: u16,
    pub generation: u32,
    pub rights: u32,
}

// Object slots: state 0=destroyed/unrevoked, 1=alive, 2=revoked/alive,
// 3=revoked/destroyed. The fourth state preserves C++'s dicabut trail.
static OBJ_GEN: [AtomicU32; MAX_OBJS] = [const { AtomicU32::new(0) }; MAX_OBJS];
static OBJ_REF: [AtomicU32; MAX_OBJS] = [const { AtomicU32::new(0) }; MAX_OBJS];
static OBJ_KIND: [AtomicU32; MAX_OBJS] = [const { AtomicU32::new(0) }; MAX_OBJS];
static OBJ_STATE: [AtomicU32; MAX_OBJS] = [const { AtomicU32::new(0) }; MAX_OBJS];

// Capability slots: all fields are protected by KSM_LOCK as one transaction.
static CAP_USED: [AtomicU32; MAX_CAPS] = [const { AtomicU32::new(0) }; MAX_CAPS];
static CAP_TASK: [AtomicU32; MAX_CAPS] = [const { AtomicU32::new(0) }; MAX_CAPS];
static CAP_HANDLE: [AtomicU32; MAX_CAPS] = [const { AtomicU32::new(0) }; MAX_CAPS];
static CAP_INDEX: [AtomicU32; MAX_CAPS] = [const { AtomicU32::new(0) }; MAX_CAPS];
static CAP_GENERATION: [AtomicU32; MAX_CAPS] = [const { AtomicU32::new(0) }; MAX_CAPS];
static CAP_KIND: [AtomicU32; MAX_CAPS] = [const { AtomicU32::new(0) }; MAX_CAPS];
static CAP_RIGHTS: [AtomicU32; MAX_CAPS] = [const { AtomicU32::new(0) }; MAX_CAPS];
// Compact-vector length. Removal swaps the last live record into the removed
// position, matching the C++ table's observable capacity/refcount semantics.
static CAP_COUNT: AtomicU32 = AtomicU32::new(0);

static KSM_LOCK: AtomicBool = AtomicBool::new(false);
static NEXT_TRANSIT: AtomicU32 = AtomicU32::new(1);
static NEXT_TASK_HANDLE: AtomicU32 = AtomicU32::new(1);
static NEXT_MEMORY_HANDLE: AtomicU32 = AtomicU32::new(1);
static NEXT_SERVICE_HANDLE: AtomicU32 = AtomicU32::new(1);
static NEXT_FILEREF_HANDLE: AtomicU32 = AtomicU32::new(1);
static NEXT_DIRECTORY_HANDLE: AtomicU32 = AtomicU32::new(1);

#[inline]
fn lock() {
    while KSM_LOCK.swap(true, Ordering::Acquire) {
        core::hint::spin_loop();
    }
}

#[inline]
fn unlock() {
    KSM_LOCK.store(false, Ordering::Release);
}

#[inline]
fn id_none() -> KsmObjectId {
    KsmObjectId { kind: kind::NONE, index: 0, generation: 0 }
}

#[inline]
fn supported_kind(k: u16) -> bool {
    (kind::TASK..=kind::DIRECTORY).contains(&k)
}

#[inline]
fn rights_valid(k: u16, r: u32) -> bool {
    let common = right::CONTROL | right::DUPLICATE | right::TRANSFER | right::REVOKE;
    let allowed = match k {
        kind::TASK => common | right::READ | right::WRITE,
        kind::ENDPOINT => common | right::SEND | right::RECEIVE,
        kind::MEMORY => common | right::READ | right::WRITE | right::MAP | right::EXECUTE,
        kind::SERVICE => common | right::READ | right::EXECUTE,
        kind::DEVICE => common | right::READ | right::WRITE | right::MAP,
        kind::FILEREF => common | right::READ | right::WRITE | right::EXECUTE,
        kind::DIRECTORY => common | right::READ | right::WRITE | right::EXECUTE,
        _ => return false,
    };
    r != right::NONE && (r & !allowed) == 0
}

#[inline]
fn object_resolves(id: KsmObjectId) -> bool {
    // Cermin `selesaikan_objek`: indeks dalam rentang, generasi cocok (bukan 0),
    // kind SLOT didukung, dan slot tidak dalam keadaan destroyed-fresh. `id.kind`
    // diabaikan — C++ `ObjekId` hanya {index, generation}; kind milik slot.
    let i = id.index as usize;
    i < MAX_OBJS && id.generation != 0
        && OBJ_GEN[i].load(Ordering::Relaxed) == id.generation
        && supported_kind(OBJ_KIND[i].load(Ordering::Relaxed) as u16)
        && OBJ_STATE[i].load(Ordering::Acquire) != 0
}

#[inline]
fn find_cap(task: u32, handle: u32) -> Option<usize> {
    let count = CAP_COUNT.load(Ordering::Relaxed) as usize;
    let mut i = 0;
    while i < count {
        if CAP_TASK[i].load(Ordering::Relaxed) == task
            && CAP_HANDLE[i].load(Ordering::Relaxed) == handle
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[inline]
fn find_free_cap() -> Option<usize> {
    let count = CAP_COUNT.load(Ordering::Relaxed) as usize;
    if count < MAX_CAPS { Some(count) } else { None }
}

#[inline]
fn next_generation(i: usize) -> u32 {
    let n = OBJ_GEN[i].load(Ordering::Relaxed).wrapping_add(1);
    if n == 0 { 1 } else { n }
}

#[inline]
fn object_state(i: usize) -> u32 {
    OBJ_STATE[i].load(Ordering::Acquire)
}

#[inline]
fn valid_object(id: KsmObjectId) -> Result<usize, i32> {
    if !object_resolves(id) { return Err(status::BAD_OBJECT); }
    Ok(id.index as usize)
}

fn remove_cap(i: usize) {
    let count = CAP_COUNT.load(Ordering::Relaxed) as usize;
    if i >= count { return; }
    let removed_obj = CAP_INDEX[i].load(Ordering::Relaxed) as usize;
    if removed_obj < MAX_OBJS && OBJ_REF[removed_obj].load(Ordering::Relaxed) != 0 {
        let remaining = OBJ_REF[removed_obj].fetch_sub(1, Ordering::Relaxed) - 1;
        // C++ `turunkan_ref`: refcount nol selalu mematikan `hidup`, dengan
        // jejak `dicabut` dipertahankan. Alive(1) -> Destroyed-fresh(0);
        // RevokedAlive(2) -> RevokedDestroyed(3).
        if remaining == 0 {
            let st = OBJ_STATE[removed_obj].load(Ordering::Relaxed);
            if st == 1 {
                OBJ_STATE[removed_obj].store(0, Ordering::Release);
            } else if st == 2 {
                OBJ_STATE[removed_obj].store(3, Ordering::Release);
            }
        }
    }
    let last = count - 1;
    if i != last {
        CAP_TASK[i].store(CAP_TASK[last].load(Ordering::Relaxed), Ordering::Relaxed);
        CAP_HANDLE[i].store(CAP_HANDLE[last].load(Ordering::Relaxed), Ordering::Relaxed);
        CAP_INDEX[i].store(CAP_INDEX[last].load(Ordering::Relaxed), Ordering::Relaxed);
        CAP_GENERATION[i].store(CAP_GENERATION[last].load(Ordering::Relaxed), Ordering::Relaxed);
        CAP_KIND[i].store(CAP_KIND[last].load(Ordering::Relaxed), Ordering::Relaxed);
        CAP_RIGHTS[i].store(CAP_RIGHTS[last].load(Ordering::Relaxed), Ordering::Relaxed);
    }
    CAP_USED[last].store(0, Ordering::Relaxed);
    CAP_COUNT.store((count - 1) as u32, Ordering::Release);
}

fn install_locked(task: u32, handle: u32, id: KsmObjectId, rights: u32) -> i32 {
    if rights == right::NONE { return status::INVALID; }
    let oi = match valid_object(id) { Ok(i) => i, Err(e) => return e };
    // Kind OTORITATIF ada di slot objek, bukan di `ObjekId` (C++ `ObjekId` hanya
    // {index, generation}). Memakai `id.kind` akan membuat capability menyimpan
    // kind yang salah bila pemanggil memasok kind yang keliru.
    let slot_kind = OBJ_KIND[oi].load(Ordering::Relaxed) as u16;
    if object_state(oi) != 1 { return status::REVOKED; }
    if !rights_valid(slot_kind, rights) { return status::BAD_KIND; }
    if find_cap(task, handle).is_some() { return status::INVALID; }
    let ci = match find_free_cap() { Some(i) => i, None => return status::CAPACITY };
    CAP_TASK[ci].store(task, Ordering::Relaxed);
    CAP_HANDLE[ci].store(handle, Ordering::Relaxed);
    CAP_INDEX[ci].store(id.index as u32, Ordering::Relaxed);
    CAP_GENERATION[ci].store(id.generation, Ordering::Relaxed);
    CAP_KIND[ci].store(slot_kind as u32, Ordering::Relaxed);
    CAP_RIGHTS[ci].store(rights, Ordering::Relaxed);
    CAP_USED[ci].store(1, Ordering::Release);
    CAP_COUNT.store((ci + 1) as u32, Ordering::Release);
    OBJ_REF[oi].fetch_add(1, Ordering::Relaxed);
    status::OK
}

fn cap_id(i: usize) -> KsmObjectId {
    KsmObjectId {
        kind: CAP_KIND[i].load(Ordering::Relaxed) as u16,
        index: CAP_INDEX[i].load(Ordering::Relaxed) as u16,
        generation: CAP_GENERATION[i].load(Ordering::Relaxed),
    }
}

fn cap_out(status_code: i32, i: Option<usize>) -> KsmCapabilityOut {
    match i {
        Some(n) => KsmCapabilityOut {
            status: status_code,
            id: cap_id(n),
            rights: CAP_RIGHTS[n].load(Ordering::Relaxed),
            handle: CAP_HANDLE[n].load(Ordering::Relaxed),
            task: CAP_TASK[n].load(Ordering::Relaxed),
        },
        None => KsmCapabilityOut { status: status_code, id: id_none(), rights: 0, handle: 0, task: 0 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_reset() -> u32 {
    lock();
    let mut i = 0;
    CAP_COUNT.store(0, Ordering::Relaxed);
    while i < MAX_CAPS { CAP_USED[i].store(0, Ordering::Relaxed); i += 1; }
    i = 0;
    while i < MAX_OBJS {
        OBJ_GEN[i].store(0, Ordering::Relaxed);
        OBJ_REF[i].store(0, Ordering::Relaxed);
        OBJ_KIND[i].store(0, Ordering::Relaxed);
        OBJ_STATE[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    NEXT_TRANSIT.store(1, Ordering::Relaxed);
    NEXT_TASK_HANDLE.store(1, Ordering::Relaxed);
    NEXT_MEMORY_HANDLE.store(1, Ordering::Relaxed);
    NEXT_SERVICE_HANDLE.store(1, Ordering::Relaxed);
    NEXT_FILEREF_HANDLE.store(1, Ordering::Relaxed);
    NEXT_DIRECTORY_HANDLE.store(1, Ordering::Relaxed);
    unlock();
    1
}

// Inti `buat_objek`. Pemanggil WAJIB memegang KSM_LOCK. Dipakai baik oleh door
// tunggal maupun oleh transaksi gabungan supaya semantiknya hanya ada SEKALI.
fn buat_locked(kind_value: u16) -> KsmObjectOut {
    if !supported_kind(kind_value) {
        return KsmObjectOut { status: status::BAD_KIND, id: id_none(), state: 0, reserved: [0; 3] };
    }
    let mut i = 0;
    while i < MAX_OBJS {
        // C++ `buat_objek` memakai slot pertama dengan `!hidup`. `hidup` benar
        // hanya pada state Alive(1)/RevokedAlive(2); jadi slot bebas adalah
        // Destroyed-fresh(0) DAN RevokedDestroyed(3).
        let st = object_state(i);
        if st == 0 || st == 3 { break; }
        i += 1;
    }
    if i == MAX_OBJS {
        return KsmObjectOut { status: status::CAPACITY, id: id_none(), state: 0, reserved: [0; 3] };
    }
    let generation = next_generation(i);
    OBJ_GEN[i].store(generation, Ordering::Relaxed);
    OBJ_KIND[i].store(kind_value as u32, Ordering::Relaxed);
    OBJ_REF[i].store(0, Ordering::Relaxed);
    OBJ_STATE[i].store(1, Ordering::Release);
    KsmObjectOut {
        status: status::OK,
        id: KsmObjectId { kind: kind_value, index: i as u16, generation },
        state: 1,
        reserved: [0; 3],
    }
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_buat(kind_value: u16) -> KsmObjectOut {
    lock();
    let out = buat_locked(kind_value);
    unlock();
    out
}

// Inti `lepas_objek`. Pemanggil WAJIB memegang KSM_LOCK.
fn lepas_objek_locked(id: KsmObjectId) -> i32 {
    match valid_object(id) {
        Err(e) => e,
        Ok(i) => {
            if OBJ_REF[i].load(Ordering::Relaxed) != 0 { status::INVALID }
            else {
                let state = object_state(i);
                if state == 1 { OBJ_STATE[i].store(0, Ordering::Release); }
                else if state == 2 { OBJ_STATE[i].store(3, Ordering::Release); }
                status::OK
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_lepas_objek(id: KsmObjectId) -> i32 {
    lock();
    let result = lepas_objek_locked(id);
    unlock();
    result
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_cabut(id: KsmObjectId) -> i32 {
    lock();
    let result = match valid_object(id) {
        Err(e) => e,
        Ok(i) => {
            if object_state(i) == 2 || object_state(i) == 3 { status::REVOKED }
            else if object_state(i) == 0 { status::BAD_OBJECT }
            else {
                OBJ_STATE[i].store(2, Ordering::Release);
                let mut c = 0;
                while c < CAP_COUNT.load(Ordering::Relaxed) as usize {
                    // C++ membandingkan `ObjekId` saja ({index, generation});
                    // kind TIDAK ikut dibandingkan.
                    if CAP_INDEX[c].load(Ordering::Relaxed) == id.index as u32
                        && CAP_GENERATION[c].load(Ordering::Relaxed) == id.generation
                    {
                        remove_cap(c);
                    } else {
                        c += 1;
                    }
                }
                status::OK
            }
        }
    };
    unlock();
    result
}

#[inline]
fn object_out(status_code: i32, id: KsmObjectId, state_value: u8) -> KsmObjectOut {
    KsmObjectOut { status: status_code, id, state: state_value, reserved: [0; 3] }
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_keadaan(id: KsmObjectId) -> KsmObjectOut {
    lock();
    if !object_resolves(id) { unlock(); return object_out(status::BAD_OBJECT, id_none(), 0); }
    let state_value = match object_state(id.index as usize) { 1 => 0, 2 => 1, 3 => 2, _ => 0 };
    let out = object_out(status::OK, id, state_value);
    unlock();
    out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_kind(id: KsmObjectId) -> u16 {
    lock();
    // C++ `kind_objek`: kind tersimpan selama indeks valid dan generasi cocok,
    // TANPA memandang hidup/dicabut. Slot yang sudah dimusnahkan tetap
    // melaporkan kind-nya (generation trail), jadi ini bukan `object_resolves`.
    let i = id.index as usize;
    let kind_value = if i < MAX_OBJS && id.generation != 0
        && OBJ_GEN[i].load(Ordering::Relaxed) == id.generation
    {
        OBJ_KIND[i].load(Ordering::Relaxed) as u16
    } else {
        kind::NONE
    };
    unlock();
    kind_value
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_refcount(id: KsmObjectId) -> u32 {
    lock();
    let count = if object_resolves(id) { OBJ_REF[id.index as usize].load(Ordering::Relaxed) } else { 0 };
    unlock();
    count
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_jumlah_cap() -> u32 {
    CAP_COUNT.load(Ordering::Acquire)
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_jumlah_objek() -> u32 {
    lock();
    let mut count = 0; let mut i = 0;
    while i < MAX_OBJS { if object_state(i) == 1 || object_state(i) == 2 { count += 1; } i += 1; }
    unlock();
    count
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pasang(task: u32, handle: u32, id: KsmObjectId, rights: u32) -> i32 {
    lock();
    let result = install_locked(task, handle, id, rights);
    unlock();
    result
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_hapus(task: u32, handle: u32) -> i32 {
    lock();
    let result = match find_cap(task, handle) { Some(i) => { remove_cap(i); status::OK }, None => status::BAD_HANDLE };
    unlock();
    result
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_periksa(task: u32, handle: u32, required: u32) -> KsmCapabilityOut {
    lock();
    let i = match find_cap(task, handle) {
        Some(i) => i,
        None => { unlock(); return cap_out(status::BAD_HANDLE, None); }
    };
    let id = cap_id(i);
    let oi = id.index as usize;
    let result = if !object_resolves(id) { status::BAD_OBJECT }
        else if object_state(oi) != 1 { status::REVOKED }
        else if CAP_RIGHTS[i].load(Ordering::Relaxed) & required != required { status::MISSING_RIGHTS }
        else { status::OK };
    // C++ hanya mengisi `keluar` saat Ok; kegagalan meninggalkan nilai nol.
    let out = if result == status::OK { cap_out(result, Some(i)) } else { cap_out(result, None) };
    unlock();
    out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_lihat(task: u32, handle: u32) -> KsmCapabilityOut {
    lock();
    let out = match find_cap(task, handle) { Some(i) => cap_out(status::OK, Some(i)), None => cap_out(status::BAD_HANDLE, None) };
    unlock();
    out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_sempitkan(task: u32, handle: u32, requested: u32) -> KsmCapabilityOut {
    lock();
    let i = match find_cap(task, handle) { Some(i) => i, None => { unlock(); return cap_out(status::BAD_HANDLE, None); } };
    let old = CAP_RIGHTS[i].load(Ordering::Relaxed);
    // C++ hanya mengisi `keluar` saat Ok; kegagalan meninggalkan nilai nol.
    if old & right::CONTROL != right::CONTROL { unlock(); return cap_out(status::MISSING_RIGHTS, None); }
    let derived = old & requested;
    if derived == 0 { unlock(); return cap_out(status::MISSING_RIGHTS, None); }
    CAP_RIGHTS[i].store(derived, Ordering::Release);
    let out = cap_out(status::OK, Some(i));
    unlock();
    out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pindah(task: u32, handle: u32, target_task: u32, target_handle: u32, requested: u32) -> i32 {
    lock();
    let result = if task == target_task { status::INVALID }
    else if let Some(i) = find_cap(task, handle) {
        let id = cap_id(i); let rights = CAP_RIGHTS[i].load(Ordering::Relaxed);
        if rights & right::TRANSFER == 0 { status::MISSING_RIGHTS }
        else if !object_resolves(id) { status::BAD_OBJECT }
        else if object_state(id.index as usize) != 1 { status::REVOKED }
        else { let derived = rights & requested;
            if derived == 0 { status::MISSING_RIGHTS }
            else if find_cap(target_task, target_handle).is_some() { status::CAPACITY }
            else if CAP_COUNT.load(Ordering::Relaxed) as usize >= MAX_CAPS { status::CAPACITY }
            else { CAP_TASK[i].store(target_task, Ordering::Relaxed); CAP_HANDLE[i].store(target_handle, Ordering::Release); CAP_RIGHTS[i].store(derived, Ordering::Release); status::OK }
        }
    } else { status::BAD_HANDLE };
    unlock(); result
}

#[inline]
fn next_transit() -> u32 {
    // C++: `slot = m_transit_berikutnya++; if (m_transit_berikutnya == 0) = 1`.
    let slot = NEXT_TRANSIT.load(Ordering::Relaxed);
    let mut next = slot.wrapping_add(1);
    if next == 0 { next = 1; }
    NEXT_TRANSIT.store(next, Ordering::Relaxed);
    slot
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_transit(task: u32, handle: u32) -> KsmTransitOut {
    lock();
    let i = match find_cap(task, handle) { Some(i) => i, None => { unlock(); return KsmTransitOut { status: status::BAD_HANDLE, slot: 0, rights: 0, id: id_none() }; } };
    let id = cap_id(i); let rights = CAP_RIGHTS[i].load(Ordering::Relaxed);
    if rights & right::TRANSFER == 0 { unlock(); return KsmTransitOut { status: status::MISSING_RIGHTS, slot: 0, rights: 0, id }; }
    if !object_resolves(id) { unlock(); return KsmTransitOut { status: status::BAD_OBJECT, slot: 0, rights: 0, id }; }
    if object_state(id.index as usize) != 1 { unlock(); return KsmTransitOut { status: status::REVOKED, slot: 0, rights: 0, id }; }
    let slot = next_transit();
    CAP_TASK[i].store(TRANSIT_TASK, Ordering::Relaxed); CAP_HANDLE[i].store(slot, Ordering::Release);
    let out = KsmTransitOut { status: status::OK, slot, rights, id }; unlock(); out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_transit_ambil(slot: u32, task: u32, handle: u32) -> i32 {
    klatos_ksm_kap_pindah(TRANSIT_TASK, slot, task, handle, right::ALL)
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_transit_lepas(slot: u32) -> i32 {
    klatos_ksm_kap_hapus(TRANSIT_TASK, slot)
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_tugas_mati(task: u32) -> u32 {
    lock(); let mut n = 0; let mut i = 0;
    while i < CAP_COUNT.load(Ordering::Relaxed) as usize {
        if CAP_TASK[i].load(Ordering::Relaxed) == task { remove_cap(i); n += 1; } else { i += 1; }
    }
    unlock(); n
}

// ---------------------------------------------------------- auto-handle
//
// C++ `pasang_task`/`pasang_memori`/`pasang_layanan`/`pasang_referensifile`
// mengalokasikan handle dari kursor per-kind di bawah kunci yang SAMA dengan
// `pasang`, lalu mencoba slot berikutnya hanya bila slot itu sudah dipakai task
// yang sama (`Invalid`). Kegagalan lain langsung dilaporkan tanpa mencoba slot
// lain. Ruang lingkup 12 bit; slot 0 dicadangkan.

const HANDLE_TASK_PREFIX: u32 = 0x1000;
const HANDLE_MEMORY_PREFIX: u32 = 0x2000;
const HANDLE_SERVICE_PREFIX: u32 = 0x3000;
const HANDLE_FILEREF_PREFIX: u32 = 0x4000;
const HANDLE_DIRECTORY_PREFIX: u32 = 0x5000;

#[inline]
fn auto_next(cursor: &AtomicU32) -> u32 {
    let slot = cursor.load(Ordering::Relaxed);
    let mut next = slot.wrapping_add(1);
    if next > 0x0FFF { next = 1; }
    cursor.store(next, Ordering::Relaxed);
    slot
}

fn pasang_auto(task: u32, id: KsmObjectId, hak: u32, cursor: &AtomicU32, prefix: u32) -> KsmCapabilityOut {
    let mut attempt = 0;
    while attempt < 4095 {
        let slot = auto_next(cursor);
        attempt += 1;
        if slot == 0 { continue; }
        let handle = prefix | (slot & 0x0FFF);
        let code = install_locked(task, handle, id, hak);
        if code == status::OK {
            return match find_cap(task, handle) { Some(ci) => cap_out(status::OK, Some(ci)), None => cap_out(status::OK, None) };
        }
        if code != status::INVALID { return cap_out(code, None); }
    }
    cap_out(status::CAPACITY, None)
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pasang_task(task: u32, id: KsmObjectId, hak: u32) -> KsmCapabilityOut {
    lock(); let out = pasang_auto(task, id, hak, &NEXT_TASK_HANDLE, HANDLE_TASK_PREFIX); unlock(); out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pasang_memori(task: u32, id: KsmObjectId, hak: u32) -> KsmCapabilityOut {
    lock(); let out = pasang_auto(task, id, hak, &NEXT_MEMORY_HANDLE, HANDLE_MEMORY_PREFIX); unlock(); out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pasang_layanan(task: u32, id: KsmObjectId, hak: u32) -> KsmCapabilityOut {
    lock(); let out = pasang_auto(task, id, hak, &NEXT_SERVICE_HANDLE, HANDLE_SERVICE_PREFIX); unlock(); out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pasang_referensifile(task: u32, id: KsmObjectId, hak: u32) -> KsmCapabilityOut {
    lock(); let out = pasang_auto(task, id, hak, &NEXT_FILEREF_HANDLE, HANDLE_FILEREF_PREFIX); unlock(); out
}

// F-P7-01 — pasang capability Directory dengan handle otomatis (prefix 0x5000).
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pasang_direktori(task: u32, id: KsmObjectId, hak: u32) -> KsmCapabilityOut {
    lock(); let out = pasang_auto(task, id, hak, &NEXT_DIRECTORY_HANDLE, HANDLE_DIRECTORY_PREFIX); unlock(); out
}

// Pasang capability LANGSUNG di task transit (kernel memasok endpoint ke proses
// tanpa lewat `give`). Refcount objek NAIK (bandingkan `transit` yang tidak).
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pasang_di_transit(id: KsmObjectId, hak: u32) -> KsmTransitOut {
    lock();
    if hak == right::NONE { unlock(); return KsmTransitOut { status: status::INVALID, slot: 0, rights: 0, id: id_none() }; }
    let oi = match valid_object(id) { Ok(i) => i, Err(e) => { unlock(); return KsmTransitOut { status: e, slot: 0, rights: 0, id: id_none() }; } };
    if object_state(oi) != 1 { unlock(); return KsmTransitOut { status: status::REVOKED, slot: 0, rights: 0, id }; }
    // Kind otoritatif = kind slot objek, bukan kind yang dipasok pemanggil.
    let slot_kind = OBJ_KIND[oi].load(Ordering::Relaxed) as u16;
    if !rights_valid(slot_kind, hak) { unlock(); return KsmTransitOut { status: status::BAD_KIND, slot: 0, rights: 0, id }; }
    let count = CAP_COUNT.load(Ordering::Relaxed) as usize;
    if count >= MAX_CAPS { unlock(); return KsmTransitOut { status: status::CAPACITY, slot: 0, rights: 0, id }; }
    let slot = next_transit();
    CAP_TASK[count].store(TRANSIT_TASK, Ordering::Relaxed);
    CAP_HANDLE[count].store(slot, Ordering::Relaxed);
    CAP_INDEX[count].store(id.index as u32, Ordering::Relaxed);
    CAP_GENERATION[count].store(id.generation, Ordering::Relaxed);
    // Kind OTORITATIF = kind slot objek, bukan kind yang dipasok pemanggil.
    // Fasad C++ `ObjekId` tidak membawa kind (hanya {index, generation}); bila
    // baris ini menyimpan `id.kind`, setiap capability yang dipasang lewat
    // `pasang_di_transit` akan mencatat kind NONE dan merusak `kind_objek`/
    // resolver. `install_locked` sudah memakai `slot_kind`; ini menyamakannya.
    CAP_KIND[count].store(slot_kind as u32, Ordering::Relaxed);
    CAP_RIGHTS[count].store(hak, Ordering::Relaxed);
    CAP_USED[count].store(1, Ordering::Release);
    CAP_COUNT.store((count + 1) as u32, Ordering::Release);
    OBJ_REF[oi].fetch_add(1, Ordering::Relaxed);
    unlock();
    KsmTransitOut { status: status::OK, slot, rights: hak, id }
}

// Cari handle yang dipegang `task` untuk `id`. BUKAN permukaan otoritas.
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_cari_handle(task: u32, id: KsmObjectId) -> KsmCapabilityOut {
    lock();
    let count = CAP_COUNT.load(Ordering::Relaxed) as usize;
    let mut i = 0;
    let mut found = None;
    while i < count {
        // C++ `cari_handle_objek` membandingkan `ObjekId` saja.
        if CAP_TASK[i].load(Ordering::Relaxed) == task
            && CAP_INDEX[i].load(Ordering::Relaxed) == id.index as u32
            && CAP_GENERATION[i].load(Ordering::Relaxed) == id.generation
        {
            found = Some(i);
            break;
        }
        i += 1;
    }
    let out = match found { Some(ci) => cap_out(status::OK, Some(ci)), None => cap_out(status::BAD_HANDLE, None) };
    unlock(); out
}

// Pemegang pertama sebuah objek: (task, handle) capability hidup pertama.
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pemegang(id: KsmObjectId) -> KsmCapabilityOut {
    lock();
    let count = CAP_COUNT.load(Ordering::Relaxed) as usize;
    let mut i = 0;
    let mut found = None;
    while i < count {
        // C++ `pemegang_objek` membandingkan `ObjekId` saja.
        if CAP_INDEX[i].load(Ordering::Relaxed) == id.index as u32
            && CAP_GENERATION[i].load(Ordering::Relaxed) == id.generation
        {
            found = Some(i);
            break;
        }
        i += 1;
    }
    let out = match found { Some(ci) => cap_out(status::OK, Some(ci)), None => cap_out(status::BAD_HANDLE, None) };
    unlock(); out
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_jumlah_task(task: u32) -> u32 {
    lock();
    let count = CAP_COUNT.load(Ordering::Relaxed) as usize;
    let mut n = 0; let mut i = 0;
    while i < count { if CAP_TASK[i].load(Ordering::Relaxed) == task { n += 1; } i += 1; }
    unlock(); n
}

// ------------------------------------------------------- transaksi gabungan
//
// Jalur runtime C++ sering menjalankan BEBERAPA operasi tabel di bawah SATU
// penguncian (`dengan_otoritas([&]{ ... })`). Bila tiap operasi memakai door
// tunggal Rust, kunci dilepas di antaranya dan atomicity hilang. Door di bawah
// menjalankan seluruh urutan di bawah satu KSM_LOCK, sehingga urutan yang
// teramati identik dengan blok C++ aslinya.

// Cermin blok `klatipc_create`: buat dua objek ENDPOINT berturut-turut. Bila
// pembuatan pertama gagal, objek kedua TIDAK dibuat (persis C++).
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_buat_pair(kind_a: u16, kind_b: u16) -> KsmPairOut {
    lock();
    let a = buat_locked(kind_a);
    if a.status != status::OK {
        let b = object_out(a.status, id_none(), 0);
        unlock();
        return KsmPairOut { status: a.status, a, b };
    }
    let b = buat_locked(kind_b);
    unlock();
    KsmPairOut { status: b.status, a, b }
}

// Cermin blok pemasangan capability `klatipc_create`: pasang dua capability
// berturut-turut. Bila yang pertama gagal, yang kedua tidak dicoba.
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pasang_pair(
    task: u32,
    handle_a: u32,
    id_a: KsmObjectId,
    rights_a: u32,
    handle_b: u32,
    id_b: KsmObjectId,
    rights_b: u32,
) -> i32 {
    lock();
    let first = install_locked(task, handle_a, id_a, rights_a);
    if first != status::OK {
        unlock();
        return first;
    }
    let second = install_locked(task, handle_b, id_b, rights_b);
    unlock();
    second
}

// Cermin blok rollback `klatipc_create`: hapus dua capability lalu lepas dua
// objek, semua di bawah satu kunci. Hasil selalu OK (C++ mengabaikan galat).
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_batal_pair(
    task: u32,
    handle_a: u32,
    handle_b: u32,
    id_a: KsmObjectId,
    id_b: KsmObjectId,
) -> i32 {
    lock();
    if let Some(i) = find_cap(task, handle_a) { remove_cap(i); }
    if let Some(i) = find_cap(task, handle_b) { remove_cap(i); }
    let _ = lepas_objek_locked(id_a);
    let _ = lepas_objek_locked(id_b);
    unlock();
    status::OK
}

// Cermin choke point `Process::create`: di SATU penguncian, buat objek Task,
// pasang DUA capability Task (anak atas dirinya, pembuat atas anak), buat objek
// Memory, lalu pasang capability Memory untuk anak. Gagal-tertutup: bila
// pembuatan Task gagal, Memory TIDAK dibuat; bila Memory gagal, objek Task
// tetap ada (identitasnya dikembalikan) — persis urutan C++.
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_buat_proses(pid_anak: u32, pid_pembuat: u32) -> KsmProsesOut {
    lock();
    let task = buat_locked(kind::TASK);
    if task.status != status::OK {
        unlock();
        return KsmProsesOut { status: task.status, task: id_none(), memory: id_none() };
    }
    let _ = pasang_auto(pid_anak, task.id, hak::TASK_PEMBUAT, &NEXT_TASK_HANDLE, HANDLE_TASK_PREFIX);
    let _ = pasang_auto(pid_pembuat, task.id, hak::TASK_PEMBUAT, &NEXT_TASK_HANDLE, HANDLE_TASK_PREFIX);
    let memory = buat_locked(kind::MEMORY);
    if memory.status != status::OK {
        unlock();
        return KsmProsesOut { status: memory.status, task: task.id, memory: id_none() };
    }
    let _ = pasang_auto(pid_anak, memory.id, hak::MEMORY_PEMBUAT, &NEXT_MEMORY_HANDLE, HANDLE_MEMORY_PREFIX);
    unlock();
    KsmProsesOut { status: status::OK, task: task.id, memory: memory.id }
}

// Cermin `ikat_perangkat_keadaan`: pastikan objek DEVICE ada lalu pasang. Bila
// `id` sudah menunjuk objek Alive dengan refcount > 0, perangkat SUDAH terikat
// -> `MissingRights` (EPERM), tanpa efek. Bila objek belum ada/musnah, objek
// BARU dibuat dan identitasnya dikembalikan lewat `out.id` supaya pemanggil
// dapat memperbarui keadaan perangkatnya. Semua di bawah satu kunci.
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_pastikan_pasang(
    task: u32,
    handle: u32,
    id: KsmObjectId,
    kind_value: u16,
    rights: u32,
) -> KsmCapabilityOut {
    lock();
    let use_id;
    if object_resolves(id) && object_state(id.index as usize) == 1 {
        if OBJ_REF[id.index as usize].load(Ordering::Relaxed) > 0 {
            unlock();
            return cap_out(status::MISSING_RIGHTS, None);
        }
        use_id = id;
    } else {
        let fresh = buat_locked(kind_value);
        if fresh.status != status::OK {
            unlock();
            return cap_out(fresh.status, None);
        }
        use_id = fresh.id;
    }
    let code = install_locked(task, handle, use_id, rights);
    let out = if code == status::OK {
        match find_cap(task, handle) {
            Some(ci) => cap_out(status::OK, Some(ci)),
            None => cap_out(status::OK, None),
        }
    } else {
        cap_out(code, None)
    };
    unlock();
    out
}

// ------------------------------------------------------------- resolver ABI2
//
// Cermin `resolve_object_bridge` / `resolve_capability_bridge`
// (`Jowo/Kontrak/Abi2DispatchKernel.h`), tetapi membaca tabel KSM Rust,
// bukan tabel C++. Keduanya memegang KSM_LOCK selama pembacaan sehingga
// keputusan dispatch tidak pernah melihat keadaan robek.

// `resolve_object_bridge`: `lihat` lalu `keadaan_objek`. Galat resolusi
// capability -> BadHandle; objek tak dapat diselesaikan -> BadObject.
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_resolve_object(caller: u32, handle: u32) -> KsmResolveObjectOut {
    lock();
    let out = match find_cap(caller, handle) {
        None => KsmResolveObjectOut {
            status: status::BAD_HANDLE, kind: 0, index: 0, generation: 0,
            state: 0, reserved0: 0, reserved1: 0,
        },
        Some(i) => {
            let id = cap_id(i);
            if !object_resolves(id) {
                KsmResolveObjectOut {
                    status: status::BAD_OBJECT, kind: 0, index: 0, generation: 0,
                    state: 0, reserved0: 0, reserved1: 0,
                }
            } else {
                let state = match object_state(id.index as usize) {
                    1 => 0u8, // Alive
                    2 => 1u8, // Revoked
                    _ => 2u8, // Destroyed
                };
                KsmResolveObjectOut {
                    status: status::OK, kind: id.kind, index: id.index,
                    generation: id.generation, state, reserved0: 0, reserved1: 0,
                }
            }
        }
    };
    unlock();
    out
}

// `resolve_capability_bridge`: `lihat` lalu `kind_objek`. TIDAK memanggil
// `keadaan_objek`, jadi capability ke objek yang sudah dimusnahkan pun tetap
// melaporkan kind-nya (generation trail) — sama seperti C++.
#[no_mangle]
pub extern "C" fn klatos_ksm_kap_resolve_capability(caller: u32, handle: u32) -> KsmResolveCapabilityOut {
    lock();
    let out = match find_cap(caller, handle) {
        None => KsmResolveCapabilityOut {
            status: status::BAD_HANDLE, kind: 0, index: 0, generation: 0, rights: 0,
        },
        Some(i) => {
            let id = cap_id(i);
            KsmResolveCapabilityOut {
                status: status::OK, kind: id.kind, index: id.index,
                generation: id.generation, rights: CAP_RIGHTS[i].load(Ordering::Relaxed),
            }
        }
    };
    unlock();
    out
}

fn run() -> KsmCapabilityReport {
    let mut checks = 0u32; let mut passed = 0u32;
    macro_rules! check { ($e:expr) => {{ checks += 1; if $e { passed += 1; } }}; }

    klatos_ksm_kap_reset();
    check!(core::mem::size_of::<KsmObjectId>() == 8);
    check!(core::mem::size_of::<KsmCapabilityOut>() == 24);
    check!(core::mem::size_of::<KsmTransitOut>() == 20);
    check!(core::mem::size_of::<KsmPairOut>() == 36);
    check!(core::mem::size_of::<KsmProsesOut>() == 20);
    check!(core::mem::size_of::<KsmResolveObjectOut>() == 16);
    check!(core::mem::size_of::<KsmResolveCapabilityOut>() == 16);
    check!(hak::ENDPOINT_PEMBUAT == 0x3F0 && hak::TASK_PEMBUAT == 0x3C3);
    check!(hak::MEMORY_PEMBUAT == 0x3CF && hak::DEVICE_PEMBUAT == 0x3CB);
    check!(MAX_CAPS == 4096 && MAX_OBJS == 2048);

    // --- create / install / authority -------------------------------------
    let a = klatos_ksm_kap_buat(kind::ENDPOINT);
    check!(a.status == status::OK && a.id.generation != 0 && a.id.index == 0);
    check!(klatos_ksm_kap_pasang(1001, 0, a.id, right::SEND | right::TRANSFER) == status::OK);
    check!(klatos_ksm_kap_periksa(1001, 0, right::SEND).status == status::OK);
    check!(klatos_ksm_kap_periksa(1001, 0, right::READ).status == status::MISSING_RIGHTS);
    check!(klatos_ksm_kap_pasang(1001, 77, a.id, right::READ) == status::BAD_KIND);
    check!(klatos_ksm_kap_pasang(1001, 77, a.id, 0) == status::INVALID);
    check!(klatos_ksm_kap_pindah(1001, 0, 1002, 5, right::ALL) == status::OK);
    check!(klatos_ksm_kap_periksa(1001, 0, right::SEND).status == status::BAD_HANDLE);
    check!(klatos_ksm_kap_periksa(1002, 5, right::SEND).status == status::OK);
    check!(klatos_ksm_kap_lihat(1002, 5).rights == (right::SEND | right::TRANSFER));

    // --- revoke: global, idempotent; kind trail retained ------------------
    check!(klatos_ksm_kap_cabut(a.id) == status::OK);
    check!(klatos_ksm_kap_periksa(1002, 5, right::SEND).status == status::BAD_HANDLE);
    check!(klatos_ksm_kap_cabut(a.id) == status::REVOKED);
    check!(klatos_ksm_kap_kind(a.id) == kind::ENDPOINT);
    let ka = klatos_ksm_kap_keadaan(a.id);
    check!(ka.status == status::OK && ka.state == 2); // Destroyed
    check!(klatos_ksm_kap_lepas_objek(a.id) == status::OK);
    let a2 = klatos_ksm_kap_buat(kind::ENDPOINT);
    check!(a2.status == status::OK && a2.id.index == 0 && a2.id.generation == a.id.generation.wrapping_add(1));

    // --- normal destroy frees the slot (alive -> destroyed-fresh) ---------
    klatos_ksm_kap_reset();
    let b = klatos_ksm_kap_buat(kind::DEVICE);
    check!(klatos_ksm_kap_pasang(1001, 1, b.id, right::READ | right::CONTROL) == status::OK);
    check!(klatos_ksm_kap_hapus(1001, 1) == status::OK);
    check!(klatos_ksm_kap_keadaan(b.id).status == status::BAD_OBJECT);
    check!(klatos_ksm_kap_kind(b.id) == kind::DEVICE);
    let b2 = klatos_ksm_kap_buat(kind::DEVICE);
    check!(b2.status == status::OK && b2.id.index == 0);

    // --- sempitkan --------------------------------------------------------
    klatos_ksm_kap_reset();
    let c = klatos_ksm_kap_buat(kind::MEMORY);
    check!(klatos_ksm_kap_pasang(1001, 2, c.id, right::READ | right::WRITE | right::CONTROL | right::MAP) == status::OK);
    let n = klatos_ksm_kap_sempitkan(1001, 2, right::READ | right::MAP);
    check!(n.status == status::OK && n.rights == (right::READ | right::MAP));
    check!(klatos_ksm_kap_periksa(1001, 2, right::WRITE).status == status::MISSING_RIGHTS);

    // --- auto-handle allocators + introspection ---------------------------
    klatos_ksm_kap_reset();
    let t = klatos_ksm_kap_buat(kind::TASK);
    let at = klatos_ksm_kap_pasang_task(1001, t.id, right::READ | right::CONTROL | right::TRANSFER);
    check!(at.status == status::OK && at.handle == 0x1001);
    let at2 = klatos_ksm_kap_pasang_task(1001, t.id, right::READ);
    check!(at2.status == status::OK && at2.handle == 0x1002);
    let m = klatos_ksm_kap_buat(kind::MEMORY);
    let am = klatos_ksm_kap_pasang_memori(1001, m.id, right::READ | right::MAP | right::TRANSFER);
    check!(am.status == status::OK && am.handle == 0x2001);
    let s = klatos_ksm_kap_buat(kind::SERVICE);
    let asv = klatos_ksm_kap_pasang_layanan(1001, s.id, right::READ);
    check!(asv.status == status::OK && asv.handle == 0x3001);
    let f = klatos_ksm_kap_buat(kind::FILEREF);
    let af = klatos_ksm_kap_pasang_referensifile(1001, f.id, right::READ);
    check!(af.status == status::OK && af.handle == 0x4001);
    let ch = klatos_ksm_kap_cari_handle(1001, t.id);
    check!(ch.status == status::OK && ch.handle == 0x1001);
    let ph = klatos_ksm_kap_pemegang(t.id);
    check!(ph.status == status::OK && ph.task == 1001 && ph.handle == 0x1001);
    check!(klatos_ksm_kap_jumlah_task(1001) == 5);
    check!(klatos_ksm_kap_jumlah_objek() == 4);

    // --- transit handoff --------------------------------------------------
    let tr = klatos_ksm_kap_transit(1001, 0x1001);
    check!(tr.status == status::OK && tr.slot != 0);
    check!(klatos_ksm_kap_periksa(1001, 0x1001, right::READ).status == status::BAD_HANDLE);
    check!(klatos_ksm_kap_transit_ambil(tr.slot, 1003, 8) == status::OK);
    check!(klatos_ksm_kap_periksa(1003, 8, right::READ).status == status::OK);
    check!(klatos_ksm_kap_tugas_mati(1003) == 1);
    check!(klatos_ksm_kap_periksa(1003, 8, right::READ).status == status::BAD_HANDLE);
    let pd = klatos_ksm_kap_pasang_di_transit(m.id, right::READ | right::MAP | right::TRANSFER);
    check!(pd.status == status::OK && pd.slot != 0);
    check!(klatos_ksm_kap_transit_ambil(pd.slot, 1002, 9) == status::OK);
    check!(klatos_ksm_kap_periksa(1002, 9, right::MAP).status == status::OK);

    // --- error precedence -------------------------------------------------
    klatos_ksm_kap_reset();
    check!(klatos_ksm_kap_buat(0).status == status::BAD_KIND);
    check!(klatos_ksm_kap_pasang(1, 1, KsmObjectId { kind: kind::ENDPOINT, index: 0, generation: 1 }, right::SEND) == status::BAD_OBJECT);
    check!(klatos_ksm_kap_pasang(1, 1, KsmObjectId { kind: kind::ENDPOINT, index: 0, generation: 1 }, 0) == status::INVALID);
    check!(klatos_ksm_kap_periksa(1, 1, right::SEND).status == status::BAD_HANDLE);

    // --- transaksi: buat_pair / pasang_pair / batal_pair (klatipc) --------
    klatos_ksm_kap_reset();
    let p = klatos_ksm_kap_buat_pair(kind::ENDPOINT, kind::ENDPOINT);
    check!(p.status == status::OK && p.a.status == status::OK && p.b.status == status::OK);
    check!(p.a.id.index == 0 && p.b.id.index == 1);
    check!(p.a.id.generation != 0 && p.b.id.generation != 0);
    check!(klatos_ksm_kap_pasang_pair(2001, 0x11, p.a.id, hak::ENDPOINT_PEMBUAT,
                                      0x12, p.b.id, hak::ENDPOINT_PEMBUAT) == status::OK);
    check!(klatos_ksm_kap_jumlah_task(2001) == 2);
    check!(klatos_ksm_kap_batal_pair(2001, 0x11, 0x12, p.a.id, p.b.id) == status::OK);
    check!(klatos_ksm_kap_jumlah_task(2001) == 0);
    check!(klatos_ksm_kap_keadaan(p.a.id).status == status::BAD_OBJECT);
    check!(klatos_ksm_kap_keadaan(p.b.id).status == status::BAD_OBJECT);
    // pembuatan pertama gagal -> objek kedua TIDAK dibuat
    let pf = klatos_ksm_kap_buat_pair(0, kind::ENDPOINT);
    check!(pf.status == status::BAD_KIND && pf.a.status == status::BAD_KIND);
    check!(pf.b.id.generation == 0);
    check!(klatos_ksm_kap_jumlah_objek() == 0);
    // pemasangan pertama gagal -> yang kedua tidak dicoba
    let q = klatos_ksm_kap_buat_pair(kind::ENDPOINT, kind::ENDPOINT);
    check!(klatos_ksm_kap_pasang_pair(2002, 0x21, q.a.id, right::NONE,
                                      0x22, q.b.id, hak::ENDPOINT_PEMBUAT) == status::INVALID);
    check!(klatos_ksm_kap_jumlah_task(2002) == 0);

    // --- transaksi: buat_proses (fork bootstrap) --------------------------
    klatos_ksm_kap_reset();
    let pr = klatos_ksm_kap_buat_proses(3001, 3000);
    check!(pr.status == status::OK && pr.task.generation != 0 && pr.memory.generation != 0);
    check!(klatos_ksm_kap_kind(pr.task) == kind::TASK);
    check!(klatos_ksm_kap_kind(pr.memory) == kind::MEMORY);
    check!(klatos_ksm_kap_jumlah_task(3001) == 2); // task self + memory self
    check!(klatos_ksm_kap_jumlah_task(3000) == 1); // task milik pembuat
    check!(klatos_ksm_kap_periksa(3001, 0x1001, right::READ).status == status::OK);
    check!(klatos_ksm_kap_periksa(3000, 0x1002, right::CONTROL).status == status::OK);
    check!(klatos_ksm_kap_periksa(3001, 0x2001, right::MAP).status == status::OK);

    // --- transaksi: pastikan_pasang (device claim) ------------------------
    klatos_ksm_kap_reset();
    let dev0 = KsmObjectId { kind: kind::DEVICE, index: 0, generation: 0 };
    let d1 = klatos_ksm_kap_pastikan_pasang(4001, 0x40000001, dev0, kind::DEVICE, hak::DEVICE_PEMBUAT);
    check!(d1.status == status::OK && d1.id.generation != 0 && d1.handle == 0x40000001);
    // ikatan kedua: objek sudah hidup dengan refcount > 0 -> MissingRights
    let d2 = klatos_ksm_kap_pastikan_pasang(4002, 0x40000002, d1.id, kind::DEVICE, hak::DEVICE_PEMBUAT);
    check!(d2.status == status::MISSING_RIGHTS);
    // lepas ikatan pertama -> objek musnah -> ikatan berikut membuat objek BARU
    check!(klatos_ksm_kap_hapus(4001, 0x40000001) == status::OK);
    check!(klatos_ksm_kap_keadaan(d1.id).status == status::BAD_OBJECT);
    let d3 = klatos_ksm_kap_pastikan_pasang(4002, 0x40000002, d1.id, kind::DEVICE, hak::DEVICE_PEMBUAT);
    check!(d3.status == status::OK && d3.id.index == d1.id.index
        && d3.id.generation == d1.id.generation.wrapping_add(1));

    // --- resolver ABI2 (Rust) --------------------------------------------
    klatos_ksm_kap_reset();
    let r = klatos_ksm_kap_buat(kind::ENDPOINT);
    check!(klatos_ksm_kap_pasang(5001, 7, r.id, hak::ENDPOINT_PEMBUAT) == status::OK);
    let ro = klatos_ksm_kap_resolve_object(5001, 7);
    check!(ro.status == status::OK && ro.kind == kind::ENDPOINT && ro.index == r.id.index
        && ro.generation == r.id.generation && ro.state == 0); // Alive
    let rc = klatos_ksm_kap_resolve_capability(5001, 7);
    check!(rc.status == status::OK && rc.kind == kind::ENDPOINT && rc.rights == hak::ENDPOINT_PEMBUAT);
    check!(klatos_ksm_kap_resolve_object(5001, 8).status == status::BAD_HANDLE);
    check!(klatos_ksm_kap_resolve_capability(5001, 8).status == status::BAD_HANDLE);
    check!(klatos_ksm_kap_resolve_object(9999, 7).status == status::BAD_HANDLE);
    check!(klatos_ksm_kap_hapus(5001, 7) == status::OK);
    check!(klatos_ksm_kap_resolve_object(5001, 7).status == status::BAD_HANDLE);
    check!(klatos_ksm_kap_resolve_capability(5001, 7).status == status::BAD_HANDLE);

    // --- F-P7-01: kind DIRECTORY (ABI v2 kind 7) --------------------------
    klatos_ksm_kap_reset();
    check!(kind::DIRECTORY == 7);
    check!(hak::DIREKTORI_PEMBUAT == 0x3C7);
    let dir = klatos_ksm_kap_buat(kind::DIRECTORY);
    check!(dir.status == status::OK && dir.id.kind == kind::DIRECTORY && dir.id.generation != 0);
    check!(klatos_ksm_kap_kind(dir.id) == kind::DIRECTORY);
    let ad = klatos_ksm_kap_pasang_direktori(1001, dir.id, hak::DIREKTORI_PEMBUAT);
    check!(ad.status == status::OK && ad.handle == 0x5001 && ad.id.kind == kind::DIRECTORY);
    check!(klatos_ksm_kap_periksa(1001, 0x5001, right::READ | right::WRITE).status == status::OK);
    // hak yang tidak sah untuk Directory (SEND) ditolak
    check!(klatos_ksm_kap_pasang(1001, 0x77, dir.id, right::SEND) == status::BAD_KIND);
    // resolver melaporkan kind DIRECTORY
    let rdo = klatos_ksm_kap_resolve_object(1001, 0x5001);
    check!(rdo.status == status::OK && rdo.kind == kind::DIRECTORY && rdo.index == dir.id.index);
    let rdc = klatos_ksm_kap_resolve_capability(1001, 0x5001);
    check!(rdc.status == status::OK && rdc.rights == hak::DIREKTORI_PEMBUAT);
    // kind 8 (di luar himpunan) tetap ditolak
    check!(klatos_ksm_kap_buat(8).status == status::BAD_KIND);

    let failed = checks - passed;
    KsmCapabilityReport { tag: REPORT_TAG, checks, passed, failed, reserved: 0, status: if failed == 0 { 0 } else { -1 } }
}

#[no_mangle]
pub extern "C" fn klatos_ksm_kap_selftest() -> KsmCapabilityReport { run() }

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4B53_4D43);
        assert!(MAX_CAPS == 4096 && MAX_OBJS == 2048);
        assert!(kind::TASK == 1 && kind::FILEREF == 6 && kind::DIRECTORY == 7);
        assert!(right::ALL == 0x3FF);
        assert!(status::OK == 0 && status::BAD_HANDLE == 1 && status::MISSING_RIGHTS == 2);
        assert!(hak::ENDPOINT_PEMBUAT == 0x3F0);
        assert!(hak::TASK_PEMBUAT == 0x3C3);
        assert!(hak::MEMORY_PEMBUAT == 0x3CF);
        assert!(hak::DEVICE_PEMBUAT == 0x3CB);
        assert!(hak::DIREKTORI_PEMBUAT == 0x3C7);
        assert!(core::mem::size_of::<KsmObjectId>() == 8);
        assert!(core::mem::size_of::<KsmObjectOut>() == 16);
        assert!(core::mem::size_of::<KsmCapabilityOut>() == 24);
        assert!(core::mem::size_of::<KsmTransitOut>() == 20);
        assert!(core::mem::size_of::<KsmCapabilityReport>() == 32);
        assert!(core::mem::size_of::<KsmPairOut>() == 36);
        assert!(core::mem::size_of::<KsmProsesOut>() == 20);
        assert!(core::mem::size_of::<KsmResolveObjectOut>() == 16);
        assert!(core::mem::size_of::<KsmResolveCapabilityOut>() == 16);
    };
}
