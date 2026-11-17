// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-7 §7.5 — **MEKANISME OPERASI ABI v2** (jalur kernel).


use core::sync::atomic::{AtomicU32, Ordering};

use crate::abi2::{object_kind, right, Status};
use crate::dispatch::{
    op_device, op_directory, op_fileref, op_keymap, op_mem, op_task, status_from_code, token,
    CallerId, DispatchRequest, OperationMechanism,
};
use crate::ksm_kapabilitas::{self, KsmObjectId};

/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
const EINVAL: i32 = 22;

/// Ukuran halaman arsitektur kernel (4 KiB).
pub const PAGE_SIZE: u64 = 4096;

/// Penanda laporan mekanisme. Hanya ada di Rust.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4D45_4B41; // "KLATMEKA"

/// Kelas mekanisme yang dihitung. Satu kelas per keluarga operasi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Total pemanggilan `perform`.
    Total = 0,
    /// Operasi FileRef (VFS/FS pada deskriptor berkas).
    FileRef = 1,
    /// Operasi Directory (VFS).
    Directory = 2,
    /// Operasi Keymap (keadaan Rust).
    Keymap = 3,
    /// Operasi Device masukan/TTY.
    Device = 4,
    /// Operasi Task (pembuatan memori).
    Task = 5,
    /// Operasi Memory (transport).
    Memory = 6,
    /// Operasi lama yang hasilnya tetap token (disepakati seam C++).
    Legacy = 7,
    /// Operasi yang ditolak mekanisme (galat pasti).
    Ditolak = 8,
}

const N_COUNTER: usize = 9;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Baca penghitung (dipakai laporan + uji).
pub fn counter(which: Counter) -> u32 {
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung. TIDAK dipanggil produksi.
pub fn reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// Pembantu
// ---------------------------------------------------------------------------

/// Terjemahkan verdict errno otoritas Rust (`0` = lolos, `<0` = tolak) menjadi
/// hasil mekanisme. Nilai sukses diteruskan apa adanya.
#[inline]
const fn verdict(hasil: i32, sukses: u64) -> Result<u64, Status> {
    if hasil != 0 {
        return Err(Status::from_errno(-hasil));
    }
    Ok(sukses)
}

/// Apakah panjang jalur yang dibawa `(mem, off, len)` kosong?
#[inline]
const fn jalur_kosong(request: &DispatchRequest) -> u32 {
    if request.arg2 == 0 {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// FASE-15B — resolusi capability KSM (bukan otoritas baru)
// ---------------------------------------------------------------------------
//
// Sejak FASE-15B `arg0`/`arg3` TIDAK lagi membawa handle penyimpan Rust
// mentah (F-P15A-03). Operasi dua-capability (`FileRef::READ/WRITE`,
// `Memory::READ/WRITE`) menamai capability KEDUA lewat `arg0`, dan mekanisme
// menyelesaikannya lewat tabel KSM yang SAMA yang dipakai `dispatch()`.
//
// Ini bukan keputusan boleh/tidak kedua: `dispatch()` tetap satu-satunya
// gerbang untuk `request.handle`; fungsi di bawah hanya menyelesaikan
// identitas objek dari handle yang **dipegang pemanggil** dan meneruskan
// galat KSM apa adanya. Pemeriksaan hak capability kedua memang di sini karena
// `dispatch()` hanya melihat satu handle — dan itu memang satu-satunya tempat
// yang memilikinya.

/// Petakan kode status KSM (`ksm_kapabilitas::status`) ke [`Status`] ABI v2.
/// Delapan kode KSM (0..=7) menempati nilai yang **sama** di kedua enum.
#[inline]
const fn status_dari_ksm(code: i32) -> Status {
    match code {
        0 => Status::Ok,
        1 => Status::BadHandle,
        2 => Status::MissingRights,
        3 => Status::Revoked,
        4 => Status::BadObject,
        5 => Status::BadKind,
        6 => Status::Capacity,
        _ => Status::Invalid,
    }
}

/// Selesaikan `handle` milik `caller` menjadi identitas objek, dengan
/// pemeriksaan kind + hak. Gagal-tertutup: setiap galat KSM diteruskan.
fn selesaikan_cap(
    caller: CallerId,
    handle: u32,
    kind_harap: u16,
    hak_harap: u32,
) -> Result<KsmObjectId, Status> {
    // Identitas + keadaan objek (menolak objek yang dicabut/dimusnahkan).
    let obj = ksm_kapabilitas::klatos_ksm_kap_resolve_object(caller, handle);
    if obj.status != ksm_kapabilitas::status::OK {
        return Err(status_dari_ksm(obj.status));
    }
    if obj.kind != kind_harap {
        return Err(Status::BadKind);
    }
    // Hak yang dipegang capability.
    let cap = ksm_kapabilitas::klatos_ksm_kap_resolve_capability(caller, handle);
    if cap.status != ksm_kapabilitas::status::OK {
        return Err(status_dari_ksm(cap.status));
    }
    if cap.rights & hak_harap != hak_harap {
        return Err(Status::MissingRights);
    }
    Ok(KsmObjectId { kind: obj.kind, index: obj.index, generation: obj.generation })
}

/// Selesaikan capability KSM menjadi **penyimpan byte Rust** lewat
/// [`crate::ikatan_efek`]. Objek yang tidak terikat gagal-tertutup
/// (`BadObject`) — bukan efek diam-diam.
fn penyimpan(
    caller: CallerId,
    handle: u32,
    kind_harap: u16,
    hak_harap: u32,
) -> Result<u64, Status> {
    let id = selesaikan_cap(caller, handle, kind_harap, hak_harap)?;
    crate::ikatan_efek::selesaikan(kind_harap, id.index, id.generation)
        .map_err(|_| Status::BadObject)
}

// ---------------------------------------------------------------------------
// FileRef (kind 6)
// ---------------------------------------------------------------------------

/// Operasi `FileRef::READ` — **potongan vertikal NYATA** (FASE-13R/FASE-15A/
/// FASE-15B).
///
/// Konvensi transport bebas-penunjuk (FASE-15B — **tanpa handle mentah**):
///   `handle` = capability FileRef (OTORITAS, diselesaikan `dispatch()`),
///   `arg0`   = **capability Memory tujuan** (bukan handle penyimpan Rust),
///   `arg1`   = offset berkas,
///   `arg2`   = panjang,
///   `arg3`   = dicadangkan (0).
///
/// Byte NYATA dibaca dari penyimpan berkas Rust yang **terikat** ke capability
/// FileRef ([`crate::ikatan_efek`]) dan ditulis ke penyimpan memori Rust yang
/// terikat ke capability Memory tujuan. Hasil adalah jumlah byte tersalin —
/// **bukan** `arg2`. `EINVAL` bila offset di luar batas; `BadObject` bila
/// objek tidak terikat; `BadHandle`/`MissingRights` bila capability kedua tak
/// sah.
fn fileref_baca(caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
    let fr = penyimpan(caller, request.handle, object_kind::FILE_REF, right::READ)?;
    let mem = penyimpan(caller, request.arg0 as u32, object_kind::MEMORY, right::WRITE)?;
    let rc = crate::fileref_efek::baca_ke_memori(
        fr,
        request.arg1 as usize,
        mem,
        request.arg2 as usize,
    );
    if rc < 0 {
        return Err(Status::from_errno(-rc));
    }
    Ok(rc as u64)
}

/// F-P7-06 — BACA BALIK isi capability Memory ke penyangga ruang-pengguna.
///
/// Satu pintu otoritas: capability diselesaikan lewat tabel KSM yang SAMA
/// (`penyimpan` → kind `Memory` + hak `READ` + generasi hidup), lalu byte NYATA
/// disalin dari penyimpan memori Rust. Tidak ada jalur yang menyeberang:
/// identitas objek tidak pernah dibawa pemanggil, hanya handle capability.
///
/// `dst.len()` = panjang yang diminta. Mengembalikan jumlah byte tersalin
/// (<= dst.len()), atau `Status` gagal-tertutup.
pub fn baca_memori_ke(
    caller: CallerId,
    handle: u32,
    offset: usize,
    dst: &mut [u8],
) -> Result<usize, Status> {
    let store = penyimpan(caller, handle, object_kind::MEMORY, right::READ)?;
    let n = crate::memori_efek::baca(store, offset, dst);
    if n < 0 {
        return Err(Status::from_errno(-n));
    }
    Ok(n as usize)
}


/// Operasi `FileRef::WRITE` — kebalikan [`fileref_baca`]: byte NYATA dibaca
/// dari penyimpan memori Rust yang terikat ke capability Memory (`arg0`) lalu
/// ditulis ke penyimpan berkas Rust yang terikat ke capability FileRef
/// (`handle`).
fn fileref_tulis(caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
    let fr = penyimpan(caller, request.handle, object_kind::FILE_REF, right::WRITE)?;
    let mem = penyimpan(caller, request.arg0 as u32, object_kind::MEMORY, right::READ)?;
    let rc = crate::fileref_efek::tulis_dari_memori(
        fr,
        request.arg1 as usize,
        mem,
        request.arg2 as usize,
    );
    if rc < 0 {
        return Err(Status::from_errno(-rc));
    }
    Ok(rc as u64)
}

fn fileref_seek(request: &DispatchRequest) -> Result<u64, Status> {
    // `arg1` = whence (0 SET / 1 CUR / 2 END, divalidasi handler). Nilai hasil
    // adalah whence yang diterima; offset baru dihitung lapisan efek karena
    // membutuhkan posisi/ukuran deskriptor (fakta C++).
    Ok(request.arg1)
}

fn fileref_stat(request: &DispatchRequest) -> Result<u64, Status> {
    // `(mem, off)` = buffer metadata. Panjang buffer tersedia di arg2.
    Ok(request.arg2)
}

fn fileref_close(_request: &DispatchRequest) -> Result<u64, Status> {
    // `close` bukan hak (kontrak §4.3): selalu sukses.
    Ok(0)
}

fn fileref_fsstat(request: &DispatchRequest) -> Result<u64, Status> {
    Ok(request.arg2)
}

fn fileref_truncate(_request: &DispatchRequest) -> Result<u64, Status> {
    Ok(0)
}

fn fileref_fsinfo(request: &DispatchRequest) -> Result<u64, Status> {
    Ok(request.arg2)
}

// ---------------------------------------------------------------------------
// Directory (kind 7)
// ---------------------------------------------------------------------------

fn directory_mount(request: &DispatchRequest) -> Result<u64, Status> {
    // `arg3` = bendera mount (konvensi transport: arg3 = bendera spesifik-op).
    // Verdict milik `mount_bendera::putuskan` (satu otoritas, P1.33).
    let rc = crate::mount_bendera::putuskan(request.arg3 as i32);
    if rc != 0 {
        bump(Counter::Ditolak);
        return Err(Status::from_errno(-rc));
    }
    Ok(0)
}

fn directory_umount(request: &DispatchRequest) -> Result<u64, Status> {
    Ok(request.arg2)
}

fn directory_mount_info(request: &DispatchRequest) -> Result<u64, Status> {
    Ok(request.arg2)
}

// ---------------------------------------------------------------------------
// Keymap (kind 8) — keadaan RUST (input_keymap.rs)
// ---------------------------------------------------------------------------

fn keymap_get(request: &DispatchRequest) -> Result<u64, Status> {
    // Nama aktif dibaca dari keadaan Rust; verdict ukuran buffer milik
    // `input_keymap::get_name_verdict` (satu otoritas).
    let nama = crate::input_keymap::nama_panjang();
    let rc = crate::input_keymap::get_name_verdict(request.arg2 as u32, nama);
    verdict(rc, nama as u64)
}

fn keymap_set(request: &DispatchRequest) -> Result<u64, Status> {
    // Panjang nama dibawa `arg2`. Verdict panjang nama milik `input_keymap`
    // (satu otoritas). Verdict superuser tetap di lapisan efek karena fakta
    // `is_superuser` tidak menyeberang kontrak bebas-penunjuk.
    let rc = crate::input_keymap::set_verdict(1, request.arg2 as u32);
    if rc != 0 {
        bump(Counter::Ditolak);
        return Err(Status::from_errno(-rc));
    }
    Ok(0)
}

// --- F-P9-A: potongan blob peta kunci Device (jalur TANPA efek) -------------
//
// Jalur produksi memakai lapisan efek C++ (`keymap_efek`) karena pemilik peta
// NYATA yang menerjemahkan tombol adalah `InputManagement` C++. Fungsi di sini
// adalah padanan untuk konteks TANPA provider (matriks bukti/uji host): ia
// memindahkan byte NYATA antara capability Memory dan keadaan Rust
// (`input_keymap`) memakai penyimpan efek yang SAMA (`memori_efek`), dengan
// batas [`crate::dispatch::KEYMAP_CHUNK_MAX`] dan gagal-tertutup.

fn keymap_get_chunk(caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
    let store = penyimpan(caller, request.arg0 as u32, object_kind::MEMORY, right::WRITE)?;
    let len = request.arg2 as usize;
    if len == 0 || len > crate::dispatch::KEYMAP_CHUNK_MAX {
        return Err(Status::Invalid);
    }
    let mut buf = [0u8; crate::dispatch::KEYMAP_CHUNK_MAX];
    let n = crate::input_keymap::blob_baca(request.arg3 as usize, &mut buf[..len]);
    if n < 0 {
        return Err(Status::from_errno(-n));
    }
    let w = crate::memori_efek::tulis(store, request.arg1 as usize, &buf[..n as usize]);
    if w < 0 {
        return Err(Status::from_errno(-w));
    }
    Ok(w as u64)
}

fn keymap_set_chunk(caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
    let store = penyimpan(caller, request.arg0 as u32, object_kind::MEMORY, right::READ)?;
    let len = request.arg2 as usize;
    if len == 0 || len > crate::dispatch::KEYMAP_CHUNK_MAX {
        return Err(Status::Invalid);
    }
    let mut buf = [0u8; crate::dispatch::KEYMAP_CHUNK_MAX];
    let n = crate::memori_efek::baca(store, request.arg1 as usize, &mut buf[..len]);
    if n < 0 {
        return Err(Status::from_errno(-n));
    }
    let rc = crate::input_keymap::blob_tulis(request.arg3 as usize, &buf[..n as usize]);
    if rc < 0 {
        return Err(Status::from_errno(-rc));
    }
    Ok(rc as u64)
}

// ---------------------------------------------------------------------------
// Device masukan / TTY (kind 5, opcode >= 3)
// ---------------------------------------------------------------------------

fn device_input_led_get(request: &DispatchRequest) -> Result<u64, Status> {
    Ok(request.arg0)
}

fn device_input_led_set(_request: &DispatchRequest) -> Result<u64, Status> {
    Ok(0)
}

fn device_tty_baca(request: &DispatchRequest) -> Result<u64, Status> {
    Ok(request.arg2)
}

fn device_tty_winsize_set(request: &DispatchRequest) -> Result<u64, Status> {
    // Verdict perubahan ukuran milik `admisi_perangkat::winsz_berubah` (satu
    // otoritas). Ukuran lama adalah fakta C++; di lapisan ini hanya ukuran baru
    // yang tersedia, sehingga hasilnya `1` bila ukuran baru bukan 0.
    let berubah = crate::admisi_perangkat::winsz_berubah(
        request.arg0 as u32,
        request.arg1 as u32,
        0,
        0,
    );
    Ok(berubah as u64)
}

fn device_tty_ctty(_request: &DispatchRequest) -> Result<u64, Status> {
    Ok(0)
}

// ---------------------------------------------------------------------------
// Task (kind 1) — pembuatan memori
// ---------------------------------------------------------------------------

fn task_memory_create(request: &DispatchRequest) -> Result<u64, Status> {
    // `arg0` = ukuran byte. Jumlah halaman = ceil(size / PAGE_SIZE), dengan
    // penjagaan luber. Nilai hasil adalah jumlah halaman nyata.
    let size = request.arg0;
    if size == 0 {
        bump(Counter::Ditolak);
        return Err(Status::from_errno(EINVAL));
    }
    let pages = size / PAGE_SIZE + u64::from(size % PAGE_SIZE != 0);
    Ok(pages)
}

// ---------------------------------------------------------------------------
// Memory (kind 3) — EFEK BYTE NYATA (FASE-15B)
// ---------------------------------------------------------------------------
//
// Sebelum FASE-15B `memory_transfer` hanya mengembalikan `arg3` (panjang
// diminta) — F-P15A-02: "ABI v2 Memory belum mempunyai operasi efek-byte
// nyata". Sekarang `Memory::WRITE`/`Memory::READ` menyalin byte NYATA antara
// DUA capability Memory yang keduanya diselesaikan lewat KSM:
//
//   `Memory::WRITE`: `handle` = Memory TUJUAN, `arg0` = Memory SUMBER.
//   `Memory::READ` : `handle` = Memory SUMBER,  `arg0` = Memory TUJUAN.
//   `arg1` = offset sisi `handle`, `arg2` = offset sisi `arg0`, `arg3` = panjang.
//
// Hasil = jumlah byte tersalin. Kontrak ini append-only: opcode 2/3 tidak
// berubah, hanya maknanya menjadi efek nyata (sebelumnya transport).

fn memory_transfer(caller: CallerId, request: &DispatchRequest, menulis: bool) -> Result<u64, Status> {
    // `menulis=true`  -> dst = handle, src = arg0
    // `menulis=false` -> src = handle, dst = arg0
    let (dst_h, src_h) = if menulis {
        (request.handle, request.arg0 as u32)
    } else {
        (request.arg0 as u32, request.handle)
    };
    let dst = penyimpan(caller, dst_h, object_kind::MEMORY, right::WRITE)?;
    let src = penyimpan(caller, src_h, object_kind::MEMORY, right::READ)?;

    let len = request.arg3 as usize;
    if len > crate::memori_efek::KAPASITAS {
        return Err(Status::Invalid);
    }
    // Penyangga berbatas: tidak ada alokator.
    let mut buf = [0u8; crate::memori_efek::KAPASITAS];
    let n = crate::memori_efek::baca(src, request.arg2 as usize, &mut buf[..len]);
    if n < 0 {
        return Err(Status::from_errno(-n));
    }
    let w = crate::memori_efek::tulis(dst, request.arg1 as usize, &buf[..n as usize]);
    if w < 0 {
        return Err(Status::from_errno(-w));
    }
    Ok(w as u64)
}

// ---------------------------------------------------------------------------
// F-P7-01 — LAPISAN EFEK VFS/FS (seam bebas-penunjuk ke C++)
// ---------------------------------------------------------------------------
//
// Sejak FASE-7 operasi Directory mengembalikan nilai PASTI (verdict bentuk +
// panjang jalur) tetapi TIDAK melakukan mutasi VFS nyata: objek VFS (`Custody`,
// `Inode`) dimiliki C++ dan terikat `RefPtr`/pointer (F-P18-01), sehingga
// mutasinya hanya dapat dilakukan **lapisan efek C++**. Modul ini menyediakan
// seam-nya: provider efek opsional yang dipanggil `perform` untuk operasi
// Directory. Provider TIDAK menyimpan hak dan TIDAK memutuskan boleh/tidak —
// seluruh gerbang otoritas §17 sudah dijalankan `dispatch()` sebelum `perform`;
// provider hanya melaksanakan mekanisme operasi yang sudah diloloskan.
//
// Kontrak bebas-penunjuk: jalur menyeberang SEBAGAI NILAI (larik `jalur`),
// bukan pointer. `ctx` opak milik C++ diteruskan kembali apa adanya dan TIDAK
// pernah di-dereference Rust (sama seperti `KlatosAbi2Resolvers::ctx`).

/// Panjang maksimum jalur yang menyeberang ke lapisan efek. Jalur lebih
/// panjang gagal-tertutup (`Status::Invalid`), bukan terpotong diam-diam.
pub const MAX_JALUR: usize = 256;

/// Permintaan efek VFS/FS ke lapisan efek C++ (`#[repr(C)]`, bebas-penunjuk).
///
/// `handle` adalah capability Directory dari permintaan; lapisan efek
/// menyelesaikan identitas objeknya sendiri lewat tabel KSM yang SAMA, lalu
/// memetakan identitas itu ke objek VFS nyata (`Custody`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosAbi2EffectIn {
    /// Pemanggil nyata (dari KERNEL, bukan muatan).
    pub caller: u32,
    /// Capability Directory yang sudah diloloskan `dispatch()`.
    pub handle: u32,
    /// Opcode operasi ABI v2.
    pub opcode: u8,
    /// Bendera spesifik-op (transport `arg3` untuk RENAME/MOUNT).
    pub bendera: u8,
    /// Panjang jalur NYATA di `jalur`.
    pub jalur_len: u16,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    /// Byte jalur (bebas-penunjuk). Hanya `jalur_len` byte pertama yang sah.
    pub jalur: [u8; MAX_JALUR],
}

/// Hasil efek VFS/FS. `status` memakai kode [`Status`]; `value` = nilai operasi.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosAbi2EffectOut {
    pub status: i32,
    pub value: u64,
}

/// `efek(ctx, in) -> out`. `ctx` opak milik C++; Rust tidak pernah
/// me-dereference-nya.
pub type ResolveEffectFn =
    extern "C" fn(*mut core::ffi::c_void, KlatosAbi2EffectIn) -> KlatosAbi2EffectOut;

// ---------------------------------------------------------------------------
// Mekanisme kernel
// ---------------------------------------------------------------------------

/// Mekanisme operasi ABI v2 untuk jalur kernel. Menyalurkan `(kind, opcode)`
/// ke mekanisme nyata; untuk operasi Directory dengan provider efek, mutasi
/// VFS dilaksanakan lapisan efek C++.
///
/// `efek_ctx` disimpan sebagai `usize` supaya tipe ini tetap `Sync` dan dapat
/// menjadi `static`. `None` = perilaku lama (nilai pasti tanpa mutasi VFS).
#[derive(Clone, Copy)]
pub struct MekanismeKernel {
    efek_ctx: usize,
    efek_fn: Option<ResolveEffectFn>,
}

impl MekanismeKernel {
    /// Instans tanpa lapisan efek: perilaku lama (nilai pasti) dipertahankan.
    pub const TANPA_EFEK: Self = Self { efek_ctx: 0, efek_fn: None };

    /// Instans dengan lapisan efek C++.
    pub fn dengan_efek(ctx: *mut core::ffi::c_void, f: ResolveEffectFn) -> Self {
        Self { efek_ctx: ctx as usize, efek_fn: Some(f) }
    }

    /// Apakah lapisan efek terpasang.
    pub fn punya_efek(&self) -> bool {
        self.efek_fn.is_some()
    }

    #[inline]
    fn panggil_efek(&self, input: KlatosAbi2EffectIn) -> Option<KlatosAbi2EffectOut> {
        self.efek_fn.map(|f| f(self.efek_ctx as *mut core::ffi::c_void, input))
    }

    /// Operasi Directory yang membawa satu jalur `(mem, off, len)`. Verdict
    /// jalur kosong dimiliki `admisi_vfs4::resolusi_path_admisi` — satu
    /// otoritas.
    ///
    /// F-P7-01: bila lapisan efek C++ terpasang, verdict Rust di atas tetap
    /// dijalankan LEBIH DULU (satu otoritas bentuk), lalu mutasi VFS nyata
    /// dilaksanakan `directory_efek`. Tanpa provider, nilai pasti lama
    /// dikembalikan apa adanya (perilaku tidak berubah).
    fn directory_jalur(&self, caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
        let rc = crate::admisi_vfs4::resolusi_path_admisi(jalur_kosong(request));
        let pasti = verdict(rc, request.arg2)?;
        if self.punya_efek() {
            return self.directory_efek(caller, request);
        }
        Ok(pasti)
    }

    /// Jalankan operasi Directory lewat lapisan efek C++. Menyelesaikan jalur
    /// dari capability Memory (`arg0` = handle memori, `arg1` = offset,
    /// `arg2` = panjang) memakai tabel KSM yang SAMA, lalu memanggil provider.
    ///
    /// `request.handle` TIDAK diresolusi di sini: identitas objeknya sudah
    /// diputuskan `dispatch()`, dan lapisan efek menyelesaikannya sendiri lewat
    /// tabel KSM. Dengan begitu mekanisme ini tidak menjadi otoritas kedua.
    fn directory_efek(&self, caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
        let mut input = KlatosAbi2EffectIn {
            caller,
            handle: request.handle,
            opcode: request.opcode,
            bendera: 0,
            jalur_len: 0,
            arg0: request.arg0,
            arg1: request.arg1,
            arg2: request.arg2,
            arg3: request.arg3,
            jalur: [0u8; MAX_JALUR],
        };
        let len = request.arg2 as usize;
        if len > 0 {
            if len > MAX_JALUR {
                return Err(Status::Invalid);
            }
            // Capability Memory kedua: pemeriksaan haknya memang di sini karena
            // `dispatch()` hanya melihat satu handle (lihat blok FASE-15B).
            let store = penyimpan(caller, request.arg0 as u32, object_kind::MEMORY, right::READ)?;
            let n = crate::memori_efek::baca(store, request.arg1 as usize, &mut input.jalur[..len]);
            if n < 0 {
                return Err(Status::from_errno(-n));
            }
            input.jalur_len = n as u16;
        }
        let out = self.panggil_efek(input).ok_or(Status::NotSupported)?;
        if out.status != 0 {
            return Err(status_from_code(out.status));
        }
        Ok(out.value)
    }

    /// F-P9-A — operasi Device peta kunci (`INPUT_KEYMAP_GET/SET`) lewat lapisan
    /// efek C++. Pemilik peta NYATA adalah `InputManagement` C++ (yang
    /// menerjemahkan tombol), jadi produksi TIDAK memakai keadaan Rust: ia
    /// memanggil provider efek. Capability Memory (`arg0`) diselesaikan
    /// lapisan efek sendiri — pola `fileref_efek` — sehingga mekanisme ini tidak
    /// menjadi otoritas kedua. Byte potongan diangkut lewat store Memory, jadi
    /// `jalur_len = 0`.
    fn keymap_efek(&self, caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
        let input = KlatosAbi2EffectIn {
            caller,
            handle: request.handle,
            opcode: request.opcode,
            bendera: 0,
            jalur_len: 0,
            arg0: request.arg0,
            arg1: request.arg1,
            arg2: request.arg2,
            arg3: request.arg3,
            jalur: [0u8; MAX_JALUR],
        };
        let out = self.panggil_efek(input).ok_or(Status::NotSupported)?;
        if out.status != 0 {
            return Err(status_from_code(out.status));
        }
        Ok(out.value)
    }

    /// F-P7-03 — operasi `FileRef` yang punya efek VFS nyata (kini `STAT`).
    ///
    /// Tidak ada jalur yang menyeberang: capability FileRef sudah menamai
    /// berkas, jadi `jalur_len = 0`. Provider menyelesaikan identitas objek dari
    /// `handle` lewat tabel KSM yang SAMA, memetakannya ke `Custody` nyata, lalu
    /// mengembalikan ukuran `Inode` NYATA. Bentuk argumen (`arg0` = capability
    /// Memory) sudah ditegakkan handler `dispatch()` sebelum mekanisme berjalan.
    fn fileref_efek(&self, caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
        let input = KlatosAbi2EffectIn {
            caller,
            handle: request.handle,
            opcode: request.opcode,
            bendera: 0,
            jalur_len: 0,
            arg0: request.arg0,
            arg1: request.arg1,
            arg2: request.arg2,
            arg3: request.arg3,
            jalur: [0u8; MAX_JALUR],
        };
        let out = self.panggil_efek(input).ok_or(Status::NotSupported)?;
        if out.status != 0 {
            return Err(status_from_code(out.status));
        }
        Ok(out.value)
    }
}

impl OperationMechanism for MekanismeKernel {
    fn perform(&self, caller: CallerId, request: &DispatchRequest) -> Result<u64, Status> {
        bump(Counter::Total);
        let hasil = match (request.kind, request.opcode) {
            // ---- 15 entri lama: hasil disepakati dengan seam C++ ----------
            (object_kind::ENDPOINT, _) | (object_kind::SERVICE, _) => {
                bump(Counter::Legacy);
                Ok(token(request.kind, request.opcode))
            }
            (object_kind::DEVICE, op) if op <= op_device::INFO => {
                bump(Counter::Legacy);
                Ok(token(request.kind, request.opcode))
            }
            (object_kind::TASK, op) if op <= op_task::SETPGID => {
                bump(Counter::Legacy);
                Ok(token(request.kind, request.opcode))
            }
            (object_kind::MEMORY, op) if op <= op_mem::MODIFY => {
                bump(Counter::Legacy);
                Ok(token(request.kind, request.opcode))
            }
            (object_kind::FILE_REF, op) if op <= op_fileref::MODIFY => {
                bump(Counter::Legacy);
                Ok(token(request.kind, request.opcode))
            }

            // ---- FASE-7: FileRef (VFS/FS pada deskriptor) -----------------
            (object_kind::FILE_REF, op_fileref::READ) => {
                bump(Counter::FileRef);
                // F-P7-05: dengan lapisan efek C++ terpasang, READ membaca byte
                // NYATA dari Inode dan menulisnya ke penyimpan memori Rust.
                // Tanpa provider (self-test), jalur Rust lama dipertahankan.
                if self.punya_efek() {
                    self.fileref_efek(caller, request)
                } else {
                    fileref_baca(caller, request)
                }
            }
            (object_kind::FILE_REF, op_fileref::WRITE) => {
                bump(Counter::FileRef);
                // F-P7-07: dengan lapisan efek C++ terpasang, WRITE menulis byte
                // NYATA ke Inode. Tanpa provider (self-test), jalur Rust lama
                // dipertahankan.
                if self.punya_efek() {
                    self.fileref_efek(caller, request)
                } else {
                    fileref_tulis(caller, request)
                }
            }
            (object_kind::FILE_REF, op_fileref::SEEK) => {
                bump(Counter::FileRef);
                // F-P7-08: dengan lapisan efek C++ terpasang, SEEK menulis
                // POSISI NYATA ke `OpenFileDescription` (keputusan offset milik
                // `seek_kebijakan` Rust). Tanpa provider (self-test), nilai
                // pasti lama dipertahankan.
                if self.punya_efek() {
                    self.fileref_efek(caller, request)
                } else {
                    fileref_seek(request)
                }
            }
            (object_kind::FILE_REF, op_fileref::STAT) => {
                bump(Counter::FileRef);
                // F-P7-03: dengan lapisan efek C++ terpasang, STAT membaca
                // ukuran Inode NYATA dari Custody yang terikat. Tanpa provider
                // (mis. self-test), nilai pasti lama (`arg2`) dipertahankan.
                if self.punya_efek() {
                    self.fileref_efek(caller, request)
                } else {
                    fileref_stat(request)
                }
            }
            (object_kind::FILE_REF, op_fileref::CLOSE) => {
                bump(Counter::FileRef);
                fileref_close(request)
            }
            (object_kind::FILE_REF, op_fileref::FSSTAT) => {
                bump(Counter::FileRef);
                // F-P7-10: mounted filesystem values come from the real VFS
                // effect provider; the Rust-only path remains for selftests.
                if self.punya_efek() {
                    self.fileref_efek(caller, request)
                } else {
                    fileref_fsstat(request)
                }
            }
            (object_kind::FILE_REF, op_fileref::TRUNCATE) => {
                bump(Counter::FileRef);
                fileref_truncate(request)
            }
            (object_kind::FILE_REF, op_fileref::FSINFO) => {
                bump(Counter::FileRef);
                // F-P7-10: FSINFO is a real mounted-FS effect, not a token.
                if self.punya_efek() {
                    self.fileref_efek(caller, request)
                } else {
                    fileref_fsinfo(request)
                }
            }

            // ---- FASE-7: Directory (VFS) ----------------------------------
            // F-P7-12: RENAME pindah ke grup jalur-efek di bawah — transport
            // dua-jalur memakai capability Memory kedua (`arg3`), sehingga
            // verdict bentuk + mutasi VFS nyata harus lewat `directory_jalur`
            // (satu otoritas jalur-kosong) dan `directory_efek`.
            (object_kind::DIRECTORY, op_directory::MOUNT) => {
                bump(Counter::Directory);
                directory_mount(request)
            }
            (object_kind::DIRECTORY, op_directory::UMOUNT) => {
                bump(Counter::Directory);
                directory_umount(request)
            }
            (object_kind::DIRECTORY, op_directory::MOUNT_INFO) => {
                bump(Counter::Directory);
                directory_mount_info(request)
            }
            (object_kind::DIRECTORY, op_directory::LOOKUP)
            | (object_kind::DIRECTORY, op_directory::CREATE)
            | (object_kind::DIRECTORY, op_directory::MKDIR)
            | (object_kind::DIRECTORY, op_directory::MKNOD)
            | (object_kind::DIRECTORY, op_directory::UNLINK)
            | (object_kind::DIRECTORY, op_directory::RMDIR)
            | (object_kind::DIRECTORY, op_directory::RENAME)
            | (object_kind::DIRECTORY, op_directory::SYMLINK)
            | (object_kind::DIRECTORY, op_directory::READLINK)
            | (object_kind::DIRECTORY, op_directory::LINK)
            | (object_kind::DIRECTORY, op_directory::STAT)
            | (object_kind::DIRECTORY, op_directory::READDIR)
            | (object_kind::DIRECTORY, op_directory::ACCESS)
            | (object_kind::DIRECTORY, op_directory::RESOLVE)
            | (object_kind::DIRECTORY, op_directory::UTIMENS)
            | (object_kind::DIRECTORY, op_directory::CHMOD)
            | (object_kind::DIRECTORY, op_directory::CHOWN)
            | (object_kind::DIRECTORY, op_directory::FSSTAT) => {
                bump(Counter::Directory);
                self.directory_jalur(caller, request)
            }

            // ---- FASE-7: Keymap (keadaan Rust) ----------------------------
            (object_kind::KEYMAP, op_keymap::GET) => {
                bump(Counter::Keymap);
                keymap_get(request)
            }
            (object_kind::KEYMAP, op_keymap::SET) => {
                bump(Counter::Keymap);
                keymap_set(request)
            }

            // ---- FASE-7: Device masukan/TTY -------------------------------
            (object_kind::DEVICE, op_device::INPUT_LED_GET) => {
                bump(Counter::Device);
                device_input_led_get(request)
            }
            (object_kind::DEVICE, op_device::INPUT_LED_SET) => {
                bump(Counter::Device);
                device_input_led_set(request)
            }
            (object_kind::DEVICE, op_device::TTY_TERMIOS_GET)
            | (object_kind::DEVICE, op_device::TTY_WINSIZE_GET)
            | (object_kind::DEVICE, op_device::TTY_PGRP_GET)
            | (object_kind::DEVICE, op_device::PTY_INDEX_GET) => {
                bump(Counter::Device);
                device_tty_baca(request)
            }
            (object_kind::DEVICE, op_device::TTY_WINSIZE_SET) => {
                bump(Counter::Device);
                device_tty_winsize_set(request)
            }
            (object_kind::DEVICE, op_device::TTY_CTTY_SET)
            | (object_kind::DEVICE, op_device::TTY_CTTY_CLEAR) => {
                bump(Counter::Device);
                device_tty_ctty(request)
            }

            // ---- F-P9-A: Device peta kunci (efek NYATA, append-only) --------
            (object_kind::DEVICE, op_device::INPUT_KEYMAP_GET) => {
                bump(Counter::Device);
                if self.punya_efek() {
                    self.keymap_efek(caller, request)
                } else {
                    keymap_get_chunk(caller, request)
                }
            }
            (object_kind::DEVICE, op_device::INPUT_KEYMAP_SET) => {
                bump(Counter::Device);
                if self.punya_efek() {
                    self.keymap_efek(caller, request)
                } else {
                    keymap_set_chunk(caller, request)
                }
            }

            // ---- FASE-7: Task (pembuatan memori) --------------------------
            (object_kind::TASK, op_task::MEMORY_CREATE) => {
                bump(Counter::Task);
                task_memory_create(request)
            }

            // ---- FASE-15B: Memory (efek byte NYATA) -----------------------
            (object_kind::MEMORY, op_mem::WRITE) => {
                bump(Counter::Memory);
                memory_transfer(caller, request, true)
            }
            (object_kind::MEMORY, op_mem::READ) => {
                bump(Counter::Memory);
                memory_transfer(caller, request, false)
            }

            // Operasi tak dikenal: bukan milik mekanisme ini. `dispatch()`
            // sudah menolaknya dengan `NotSupported` sebelum sampai sini, jadi
            // cabang ini hanya pertahanan berlapis.
            _ => {
                bump(Counter::Ditolak);
                Err(Status::NotSupported)
            }
        };
        if hasil.is_err() {
            // Sudah dihitung di cabang spesifik; `Total` tetap naik.
        }
        hasil
    }
}

/// Satu-satunya instans mekanisme kernel TANPA lapisan efek (dipakai jalur uji
/// dan default). Jalur produksi membangun instans `dengan_efek` per panggilan
/// (lihat `ffi.rs`), sehingga provider C++ mengalir ke `perform`.
pub static MEKANISME_KERNEL: MekanismeKernel = MekanismeKernel::TANPA_EFEK;

// ---------------------------------------------------------------------------
// Laporan + matriks bukti
// ---------------------------------------------------------------------------

/// Cermin `KlatosMekanismeAbi2Report`.
#[repr(C)]
pub struct KlatosMekanismeAbi2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub total: u32,
    pub legacy: u32,
    pub vfs_fs: u32,
    pub keymap: u32,
    pub device: u32,
    pub task: u32,
    pub memory: u32,
    pub status: i32,
}

/// Jumlah op FASE-5 (non-legacy) yang WAJIB tidak lagi mengembalikan token.
/// 58 entri tabel − 15 entri lama = 43.
pub const FASE5_OP_COUNT: u32 = 45;

/// Identitas pemanggil fiktif untuk matriks bukti. Tinggi dan tidak dipakai
/// proses nyata, sehingga capability uji tidak dapat bertabrakan dengan
/// capability produksi.
const MATRIX_CALLER: CallerId = 0x7F5B_0001;

/// Siapkan objek KSM + capability + ikatan efek untuk matriks bukti.
/// Mengembalikan identitas objek, atau `None` bila salah satu langkah gagal.
fn siapkan_objek(
    caller: CallerId,
    kind: u16,
    hak: u32,
    handle: u32,
    store: u64,
) -> Option<KsmObjectId> {
    let obj = ksm_kapabilitas::klatos_ksm_kap_buat(kind);
    if obj.status != ksm_kapabilitas::status::OK {
        return None;
    }
    if ksm_kapabilitas::klatos_ksm_kap_pasang(caller, handle, obj.id, hak)
        != ksm_kapabilitas::status::OK
    {
        return None;
    }
    if crate::ikatan_efek::ikat(kind, obj.id.index, obj.id.generation, store) != 0 {
        return None;
    }
    Some(obj.id)
}

/// Bersihkan objek KSM uji: cabut capability, lepas objek, lepas ikatan.
fn bersihkan_objek(caller: CallerId, id: KsmObjectId) {
    let _ = ksm_kapabilitas::klatos_ksm_kap_cabut(id);
    let _ = ksm_kapabilitas::klatos_ksm_kap_lepas_objek(id);
    let _ = crate::ikatan_efek::lepas(id.kind, id.index, id.generation);
    let _ = caller;
}

// ---------------------------------------------------------------------------
// F-P7-01 — provider efek UJI untuk matriks bukti (bukan produksi)
// ---------------------------------------------------------------------------
//
// Provider ini membuktikan seam lapisan efek benar-benar dipanggil dan
// menerima jalur NYATA. Nilai khasnya hanya dapat berasal dari fungsi ini.

static UJI_EFEK_DIPANGGIL: AtomicU32 = AtomicU32::new(0);
static UJI_EFEK_OPCODE: AtomicU32 = AtomicU32::new(0);
static UJI_EFEK_JALUR_LEN: AtomicU32 = AtomicU32::new(0);
static UJI_EFEK_JALUR_AWAL: AtomicU32 = AtomicU32::new(0);
static UJI_EFEK_TOLAK: AtomicU32 = AtomicU32::new(0);

/// Nilai khas provider efek uji ("KLATEFK").
const UJI_EFEK_NILAI: u64 = 0x4B4C_4154_4546_4B00;

extern "C" fn uji_efek(
    _ctx: *mut core::ffi::c_void,
    input: KlatosAbi2EffectIn,
) -> KlatosAbi2EffectOut {
    UJI_EFEK_DIPANGGIL.fetch_add(1, Ordering::Relaxed);
    UJI_EFEK_OPCODE.store(input.opcode as u32, Ordering::Relaxed);
    UJI_EFEK_JALUR_LEN.store(input.jalur_len as u32, Ordering::Relaxed);
    UJI_EFEK_JALUR_AWAL.store(input.jalur[0] as u32, Ordering::Relaxed);
    if UJI_EFEK_TOLAK.load(Ordering::Relaxed) != 0 {
        return KlatosAbi2EffectOut { status: Status::MissingRights.code(), value: 0 };
    }
    KlatosAbi2EffectOut { status: 0, value: UJI_EFEK_NILAI | input.jalur_len as u64 }
}

fn reset_uji_efek() {
    UJI_EFEK_DIPANGGIL.store(0, Ordering::Relaxed);
    UJI_EFEK_OPCODE.store(0, Ordering::Relaxed);
    UJI_EFEK_JALUR_LEN.store(0, Ordering::Relaxed);
    UJI_EFEK_JALUR_AWAL.store(0, Ordering::Relaxed);
    UJI_EFEK_TOLAK.store(0, Ordering::Relaxed);
}

/// Jalankan matriks bukti mekanisme. Membuktikan tiga hal:
///
/// 1. setiap op FASE-5 menghasilkan nilai nyata atau galat pasti — **tidak
///    pernah** token (§7.5);
/// 2. mekanisme yang memiliki otoritas Rust nyata benar-benar memanggilnya
///    (jalur kosong → EINVAL; bendera mount terlarang → EINVAL; nama peta →
///    panjang nyata);
/// 3. perutean bergerak (penghitung).
pub fn run_matrix() -> KlatosMekanismeAbi2Report {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;

    macro_rules! catat {
        ($cond:expr) => {{
            checks += 1;
            if $cond {
                passed += 1;
            }
        }};
    }

        // D-9: reset() dihapus - jangan hapus state produksi.
        // Uji-sendiri kini memakai slot bebas dan melepas miliknya sendiri.

    // --- 1. Setiap op FASE-5 tidak boleh mengembalikan token ---------------
    let mut non_legacy: u32 = 0;
    let mut index = 0;
    while index < crate::dispatch::DISPATCH_TABLE.len() {
        let entry = &crate::dispatch::DISPATCH_TABLE[index];
        let legacy = match entry.kind {
            object_kind::ENDPOINT | object_kind::SERVICE => true,
            object_kind::DEVICE => entry.opcode <= op_device::INFO,
            object_kind::TASK => entry.opcode <= op_task::SETPGID,
            object_kind::MEMORY => entry.opcode <= op_mem::MODIFY,
            object_kind::FILE_REF => entry.opcode <= op_fileref::MODIFY,
            _ => false,
        };
        if !legacy {
            non_legacy += 1;
            // Permintaan berbentuk sah: jalur non-kosong, bendera mount bersih.
            let mut request = DispatchRequest::new(entry.kind, entry.opcode, 1);
            request.arg0 = 0x1000; // mem
            request.arg1 = 0; // off / old_len / whence
            request.arg2 = 16; // len / new_mem / buffer
            request.arg3 = 0; // flags / new_len
            let hasil = MEKANISME_KERNEL.perform(MATRIX_CALLER, &request);
            let token_legacy = token(entry.kind, entry.opcode);
            // "bukan token": Ok(nilai) harus berbeda dari token, atau Err.
            let bukan_token = match hasil {
                Ok(v) => v != token_legacy,
                Err(_) => true,
            };
            catat!(bukan_token);
        }
        index += 1;
    }
    catat!(non_legacy == FASE5_OP_COUNT);

    // --- 2a. Otoritas jalur kosong benar-benar dipanggil -------------------
    {
        let mut request = DispatchRequest::new(object_kind::DIRECTORY, op_directory::LOOKUP, 1);
        request.arg0 = 0x1000;
        request.arg1 = 0;
        request.arg2 = 0; // jalur kosong
        catat!(matches!(
            MEKANISME_KERNEL.perform(MATRIX_CALLER, &request),
            Err(Status::Invalid)
        ));
    }

    // --- 2b. Otoritas bendera mount benar-benar dipanggil ------------------
    {
        // Bendera yang ditolak `mount_bendera` (MS_BIND / MS_REMOUNT) — dibaca
        // dari konstanta otoritas itu, bukan disalin.
        for flags in [
            crate::mount_bendera::MS_BIND as u64,
            crate::mount_bendera::MS_REMOUNT as u64,
        ] {
            let mut request = DispatchRequest::new(object_kind::DIRECTORY, op_directory::MOUNT, 1);
            request.arg0 = 0x1000;
            request.arg1 = 0;
            request.arg2 = 8;
            request.arg3 = flags;
            catat!(matches!(
                MEKANISME_KERNEL.perform(MATRIX_CALLER, &request),
                Err(Status::Invalid)
            ));
        }
        // Bendera bersih diterima.
        let mut request = DispatchRequest::new(object_kind::DIRECTORY, op_directory::MOUNT, 1);
        request.arg0 = 0x1000;
        request.arg2 = 8;
        request.arg3 = 0;
        catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &request), Ok(0)));
    }

    // --- 2c. Keymap membaca keadaan Rust NYATA -----------------------------
    {
        crate::input_keymap::kosongkan();
        let mut get = DispatchRequest::new(object_kind::KEYMAP, op_keymap::GET, 1);
        get.arg0 = 0x1000;
        get.arg1 = 0;
        get.arg2 = 50;
        // Nama kosong -> panjang 0.
        catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &get), Ok(0)));
        // Setel nama 7 byte lalu GET harus mengembalikan 7.
        let _ = crate::input_keymap::setel_nama(b"klat-us");
        catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &get), Ok(7)));
        // Buffer lebih kecil dari nama -> ENAMETOOLONG (Status::Invalid kanonik).
        get.arg2 = 4;
        catat!(matches!(
            MEKANISME_KERNEL.perform(MATRIX_CALLER, &get),
            Err(Status::Invalid)
        ));
        crate::input_keymap::kosongkan();
    }

    // --- 2d. Task MEMORY_CREATE menghitung halaman nyata -------------------
    {
        let mut request = DispatchRequest::new(object_kind::TASK, op_task::MEMORY_CREATE, 1);
        request.arg0 = 0;
        catat!(matches!(
            MEKANISME_KERNEL.perform(MATRIX_CALLER, &request),
            Err(Status::Invalid)
        ));
        request.arg0 = 1; // 1 byte -> 1 halaman
        catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &request), Ok(1)));
        request.arg0 = 4096; // tepat 1 halaman
        catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &request), Ok(1)));
        request.arg0 = 4097; // 2 halaman
        catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &request), Ok(2)));
        request.arg0 = 1024 * 1024; // 256 halaman
        catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &request), Ok(256)));
    }

    // --- 2e. Memory READ/WRITE: EFEK BYTE NYATA antara dua capability KSM ---
    // (FASE-15B — menutup F-P15A-02: Memory kini punya operasi efek byte nyata,
    //  dan `arg0` adalah capability KSM, bukan handle penyimpan Rust.)
    {
        // D-9: reset() dihapus - jangan hapus state produksi.
        // Uji-sendiri kini memakai slot bebas dan melepas miliknya sendiri.
        let src_store = crate::memori_efek::buat(
            crate::memori_efek::HAK_BACA | crate::memori_efek::HAK_TULIS,
            64,
        )
        .unwrap();
        let dst_store = crate::memori_efek::buat(
            crate::memori_efek::HAK_BACA | crate::memori_efek::HAK_TULIS,
            64,
        )
        .unwrap();
        let mut isi = [0u8; 16];
        let mut k = 0;
        while k < 16 {
            isi[k] = (k as u8).wrapping_mul(5).wrapping_add(2);
            k += 1;
        }
        catat!(crate::memori_efek::tulis(src_store, 0, &isi) == 16);

        let src_id = siapkan_objek(
            MATRIX_CALLER,
            object_kind::MEMORY,
            right::READ | right::WRITE,
            0x0E00_0001,
            src_store,
        );
        let dst_id = siapkan_objek(
            MATRIX_CALLER,
            object_kind::MEMORY,
            right::READ | right::WRITE,
            0x0E00_0002,
            dst_store,
        );
        // Objek dengan capability tetapi TANPA ikatan -> BadObject.
        let bare = ksm_kapabilitas::klatos_ksm_kap_buat(object_kind::MEMORY);
        let bare_ok = bare.status == ksm_kapabilitas::status::OK
            && ksm_kapabilitas::klatos_ksm_kap_pasang(
                MATRIX_CALLER,
                0x0E00_0003,
                bare.id,
                right::READ | right::WRITE,
            ) == ksm_kapabilitas::status::OK;

        match (src_id, dst_id) {
            (Some(src_id), Some(dst_id)) => {
                // WRITE: handle = tujuan, arg0 = sumber.
                let mut req = DispatchRequest::new(object_kind::MEMORY, op_mem::WRITE, 0x0E00_0002);
                req.arg0 = 0x0E00_0001;
                req.arg1 = 0;
                req.arg2 = 0;
                req.arg3 = 16;
                catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &req), Ok(16)));
                let mut keluar = [0u8; 16];
                catat!(crate::memori_efek::baca(dst_store, 0, &mut keluar) == 16);
                catat!(keluar == isi);

                // READ: handle = sumber, arg0 = tujuan (salin balik).
                let mut back = DispatchRequest::new(object_kind::MEMORY, op_mem::READ, 0x0E00_0001);
                back.arg0 = 0x0E00_0002;
                back.arg1 = 0;
                back.arg2 = 0;
                back.arg3 = 16;
                catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &back), Ok(16)));

                // arg0 bukan capability yang dipegang -> BadHandle.
                let mut hbad = DispatchRequest::new(object_kind::MEMORY, op_mem::WRITE, 0x0E00_0002);
                hbad.arg0 = 0x0EFF_FFFF;
                hbad.arg3 = 4;
                catat!(matches!(
                    MEKANISME_KERNEL.perform(MATRIX_CALLER, &hbad),
                    Err(Status::BadHandle)
                ));

                // objek tanpa ikatan -> BadObject.
                if bare_ok {
                    let mut ub = DispatchRequest::new(object_kind::MEMORY, op_mem::WRITE, 0x0E00_0002);
                    ub.arg0 = 0x0E00_0003;
                    ub.arg3 = 4;
                    catat!(matches!(
                        MEKANISME_KERNEL.perform(MATRIX_CALLER, &ub),
                        Err(Status::BadObject)
                    ));
                } else {
                    catat!(false);
                }

                bersihkan_objek(MATRIX_CALLER, src_id);
                bersihkan_objek(MATRIX_CALLER, dst_id);
            }
            (s, d) => {
                if let Some(x) = s {
                    bersihkan_objek(MATRIX_CALLER, x);
                }
                if let Some(x) = d {
                    bersihkan_objek(MATRIX_CALLER, x);
                }
                catat!(false);
            }
        }
        if bare_ok {
            let _ = ksm_kapabilitas::klatos_ksm_kap_cabut(bare.id);
            let _ = ksm_kapabilitas::klatos_ksm_kap_lepas_objek(bare.id);
        }
        let _ = crate::memori_efek::lepas(src_store);
        let _ = crate::memori_efek::lepas(dst_store);
        // D-9: reset() dihapus - jangan hapus state produksi.
        // Uji-sendiri kini memakai slot bebas dan melepas miliknya sendiri.
    }

    // --- 2e'. FileRef READ/WRITE: potongan vertikal NYATA lewat capability ---
    // (FASE-15B — `handle` & `arg0` adalah capability KSM; tidak ada handle
    //  penyimpan Rust di kontrak. Menutup F-P15A-03.)
    {
        // D-9: reset() dihapus - jangan hapus state produksi.
        // Uji-sendiri kini memakai slot bebas dan melepas miliknya sendiri.
        let mut isi = [0u8; 32];
        let mut k = 0;
        while k < 32 {
            isi[k] = (k as u8).wrapping_mul(9).wrapping_add(1);
            k += 1;
        }
        let fr_store = crate::fileref_efek::buat(&isi).unwrap();
        let mem_store = crate::memori_efek::buat(
            crate::memori_efek::HAK_BACA | crate::memori_efek::HAK_TULIS,
            64,
        )
        .unwrap();
        let fr_id = siapkan_objek(
            MATRIX_CALLER,
            object_kind::FILE_REF,
            right::READ | right::WRITE,
            0x0F00_0001,
            fr_store,
        );
        let mem_id = siapkan_objek(
            MATRIX_CALLER,
            object_kind::MEMORY,
            right::READ | right::WRITE,
            0x0F00_0002,
            mem_store,
        );
        match (fr_id, mem_id) {
            (Some(fr_id), Some(mem_id)) => {
                // READ: handle = FileRef, arg0 = Memory tujuan.
                let mut req =
                    DispatchRequest::new(object_kind::FILE_REF, op_fileref::READ, 0x0F00_0001);
                req.arg0 = 0x0F00_0002;
                req.arg1 = 0;
                req.arg2 = 32;
                catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &req), Ok(32)));
                let mut keluar = [0u8; 32];
                catat!(crate::memori_efek::baca(mem_store, 0, &mut keluar) == 32);
                catat!(keluar == isi); // byte NYATA, bukan skalar sintetis

                // Offset di luar batas -> Invalid.
                let mut bad =
                    DispatchRequest::new(object_kind::FILE_REF, op_fileref::READ, 0x0F00_0001);
                bad.arg0 = 0x0F00_0002;
                bad.arg1 = 999;
                bad.arg2 = 8;
                catat!(matches!(
                    MEKANISME_KERNEL.perform(MATRIX_CALLER, &bad),
                    Err(Status::Invalid)
                ));

                // arg0 bukan capability -> BadHandle.
                let mut hbad =
                    DispatchRequest::new(object_kind::FILE_REF, op_fileref::READ, 0x0F00_0001);
                hbad.arg0 = 0x0FFF_FFFF;
                hbad.arg2 = 8;
                catat!(matches!(
                    MEKANISME_KERNEL.perform(MATRIX_CALLER, &hbad),
                    Err(Status::BadHandle)
                ));

                // arg0 kind salah (FileRef, bukan Memory) -> BadKind.
                let mut kbad =
                    DispatchRequest::new(object_kind::FILE_REF, op_fileref::READ, 0x0F00_0001);
                kbad.arg0 = 0x0F00_0001;
                kbad.arg2 = 8;
                catat!(matches!(
                    MEKANISME_KERNEL.perform(MATRIX_CALLER, &kbad),
                    Err(Status::BadKind)
                ));

                // WRITE: handle = FileRef, arg0 = Memory sumber.
                let mut reqw =
                    DispatchRequest::new(object_kind::FILE_REF, op_fileref::WRITE, 0x0F00_0001);
                reqw.arg0 = 0x0F00_0002;
                reqw.arg1 = 0;
                reqw.arg2 = 32;
                catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &reqw), Ok(32)));

                bersihkan_objek(MATRIX_CALLER, fr_id);
                bersihkan_objek(MATRIX_CALLER, mem_id);
            }
            (f, m) => {
                if let Some(x) = f {
                    bersihkan_objek(MATRIX_CALLER, x);
                }
                if let Some(x) = m {
                    bersihkan_objek(MATRIX_CALLER, x);
                }
                catat!(false);
            }
        }
        let _ = crate::fileref_efek::lepas(fr_store);
        let _ = crate::memori_efek::lepas(mem_store);
        // D-9: reset() dihapus - jangan hapus state produksi.
        // Uji-sendiri kini memakai slot bebas dan melepas miliknya sendiri.
    }

    // --- 2f. Entri LAMA tetap mengembalikan token yang disepakati seam C++ --
    // (perilaku P0.75–P0.82 tidak berubah; token di sini adalah hasil yang
    //  DISEPAKATI, bukan no-op — seam C++ memakainya untuk memilih efek.)
    {
        let request = DispatchRequest::new(object_kind::ENDPOINT, 0, 1);
        catat!(matches!(
            MEKANISME_KERNEL.perform(MATRIX_CALLER, &request),
            Ok(v) if v == token(object_kind::ENDPOINT, 0)
        ));
        catat!(counter(Counter::Legacy) > 0);
    }

    // --- 2g. F-P7-01: operasi Directory lewat LAPISAN EFEK (provider UJI) ---
    {
        reset_uji_efek();
        // D-9: reset() dihapus - jangan hapus state produksi.
        // Uji-sendiri kini memakai slot bebas dan melepas miliknya sendiri.

        // Capability Memory berisi jalur "/etc".
        let store = match crate::memori_efek::buat(
            crate::memori_efek::HAK_BACA | crate::memori_efek::HAK_TULIS,
            16,
        ) {
            Ok(h) => h,
            Err(_) => crate::memori_efek::TAK_SAH,
        };
        catat!(store != crate::memori_efek::TAK_SAH);
        catat!(crate::memori_efek::tulis(store, 0, b"/etc") == 4);
        let mem_id = siapkan_objek(
            MATRIX_CALLER,
            object_kind::MEMORY,
            right::READ,
            0x0A00_0001,
            store,
        );
        catat!(mem_id.is_some());

        let mekanisme = MekanismeKernel::dengan_efek(core::ptr::null_mut(), uji_efek);
        catat!(mekanisme.punya_efek());

        let mut req =
            DispatchRequest::new(object_kind::DIRECTORY, op_directory::STAT, 0x0A00_0002);
        req.arg0 = 0x0A00_0001;
        req.arg1 = 0;
        req.arg2 = 4; // panjang jalur "/etc"
        req.arg3 = 0;
        // Provider dipanggil; nilainya khas + jalur_len.
        catat!(matches!(
            mekanisme.perform(MATRIX_CALLER, &req),
            Ok(v) if v == (UJI_EFEK_NILAI | 4)
        ));
        catat!(UJI_EFEK_DIPANGGIL.load(Ordering::Relaxed) == 1);
        catat!(UJI_EFEK_OPCODE.load(Ordering::Relaxed) == op_directory::STAT as u32);
        catat!(UJI_EFEK_JALUR_LEN.load(Ordering::Relaxed) == 4);
        catat!(UJI_EFEK_JALUR_AWAL.load(Ordering::Relaxed) == b'/' as u32);

        // Verdict Rust tetap berlaku SEBELUM efek: jalur kosong -> EINVAL,
        // provider TIDAK dipanggil.
        let mut kosong =
            DispatchRequest::new(object_kind::DIRECTORY, op_directory::STAT, 0x0A00_0002);
        kosong.arg0 = 0x0A00_0001;
        kosong.arg2 = 0;
        catat!(matches!(
            mekanisme.perform(MATRIX_CALLER, &kosong),
            Err(Status::Invalid)
        ));
        catat!(UJI_EFEK_DIPANGGIL.load(Ordering::Relaxed) == 1);

        // Provider menolak -> galat pasti diteruskan (bukan sukses diam).
        UJI_EFEK_TOLAK.store(1, Ordering::Relaxed);
        catat!(matches!(
            mekanisme.perform(MATRIX_CALLER, &req),
            Err(Status::MissingRights)
        ));
        UJI_EFEK_TOLAK.store(0, Ordering::Relaxed);

        // Tanpa provider: perilaku lama (nilai pasti = panjang jalur) tetap.
        catat!(matches!(MEKANISME_KERNEL.perform(MATRIX_CALLER, &req), Ok(4)));

        // Capability Memory tak dikenal -> galat pasti, provider TIDAK dipanggil.
        let sebelum = UJI_EFEK_DIPANGGIL.load(Ordering::Relaxed);
        let mut hbad =
            DispatchRequest::new(object_kind::DIRECTORY, op_directory::STAT, 0x0A00_0002);
        hbad.arg0 = 0x0AFF_FFFF;
        hbad.arg2 = 4;
        catat!(matches!(
            mekanisme.perform(MATRIX_CALLER, &hbad),
            Err(Status::BadHandle)
        ));
        catat!(UJI_EFEK_DIPANGGIL.load(Ordering::Relaxed) == sebelum);

        // Jalur lebih panjang dari MAX_JALUR -> Invalid (gagal-tertutup).
        let mut panjang =
            DispatchRequest::new(object_kind::DIRECTORY, op_directory::STAT, 0x0A00_0002);
        panjang.arg0 = 0x0A00_0001;
        panjang.arg2 = (MAX_JALUR as u64) + 1;
        catat!(matches!(
            mekanisme.perform(MATRIX_CALLER, &panjang),
            Err(Status::Invalid)
        ));

        if let Some(id) = mem_id {
            bersihkan_objek(MATRIX_CALLER, id);
        }
        let _ = crate::memori_efek::lepas(store);
        // D-9: reset() dihapus - jangan hapus state produksi.
        // Uji-sendiri kini memakai slot bebas dan melepas miliknya sendiri.
        reset_uji_efek();
    }

    // --- 3. Perutean bergerak ----------------------------------------------
    catat!(counter(Counter::Total) > 0);
    catat!(counter(Counter::Directory) > 0);
    catat!(counter(Counter::Keymap) > 0);
    catat!(counter(Counter::Task) > 0);
    catat!(counter(Counter::Memory) > 0);
    catat!(counter(Counter::FileRef) > 0);

    // Determinisme: matriks ulang memberi jawaban identik.
    let a = counter(Counter::Directory);
    let mut request = DispatchRequest::new(object_kind::DIRECTORY, op_directory::LOOKUP, 1);
    request.arg0 = 0x1000;
    request.arg2 = 16;
    let _ = MEKANISME_KERNEL.perform(MATRIX_CALLER, &request);
    catat!(counter(Counter::Directory) == a + 1);

    let status = if checks == passed { 0 } else { -1 };
    KlatosMekanismeAbi2Report {
        tag: REPORT_TAG,
        checks,
        passed,
        total: counter(Counter::Total),
        legacy: counter(Counter::Legacy),
        vfs_fs: counter(Counter::Directory) + counter(Counter::FileRef),
        keymap: counter(Counter::Keymap),
        device: counter(Counter::Device),
        task: counter(Counter::Task),
        memory: counter(Counter::Memory),
        status,
    }
}

/// Titik masuk C ABI matriks bukti mekanisme.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_mekanisme_abi2_selftest() -> KlatosMekanismeAbi2Report {
    run_matrix()
}

/// Jumlah op FASE-5 yang diharapkan (45; 43 FASE-5 + 2 F-P9-A). Dapat ditanyakan kernel.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_mekanisme_abi2_fase5_count() -> u32 {
    FASE5_OP_COUNT
}

/// Nilai penghitung kelas mekanisme (indeks = [`Counter`]).
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn klatos_mekanisme_abi2_counter(idx: u32) -> u32 {
    if (idx as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[idx as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4D45_4B41);
        assert!(FASE5_OP_COUNT == 45);
        assert!(crate::dispatch::DISPATCH_TABLE.len() == 60);
        assert!(Counter::Total as usize == 0);
        assert!(Counter::Ditolak as usize == 8);
        assert!(N_COUNTER == 9);
        assert!(PAGE_SIZE == 4096);
        // Laporan: 8 (tag) + 9*4 = 44 -> align 8 -> 48.
        assert!(core::mem::size_of::<KlatosMekanismeAbi2Report>() == 48);
        assert!(core::mem::align_of::<KlatosMekanismeAbi2Report>() == 8);
    };
}
