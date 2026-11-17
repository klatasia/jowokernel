// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Permukaan C ABI inti kernel Rust (P0.97.4).


#![allow(unsafe_code)]

use core::ffi::c_void;

use crate::abi2::{Capability, Handle, ObjectId, ObjectState, Status};
use crate::boot;
use crate::dispatch::{
    self, CallerId, CapabilityResolver, DispatchContext, DispatchRequest, DispatchResult,
    ObjectResolver, ResolvedObject, status_from_code,
};
use crate::entry;
use crate::ksm;
use crate::mekanisme_abi2::{MekanismeKernel, ResolveEffectFn};

/// Penanda mekanisme **lama** (P0.75/P0.76), dipertahankan apa adanya supaya
/// `P075-RUST-TAG` di kernel C++ tidak berubah.
pub const LEGACY_MECHANISM_TAG: u64 = 0x4B4C_4154_5255_5354; // "KLATRUST"

/// Konteks resolver yang disediakan pemanggil (C++). Opak bagi Rust.
pub type KlatosAbi2Context = *mut c_void;

// ---------------------------------------------------------------------------
// Keluaran resolver (cermin C++: Jowo/Kontrak/Abi2DispatchKernel.h)
// ---------------------------------------------------------------------------

/// Hasil resolusi identitas objek.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosAbi2ObjectOut {
    /// Kode [`Status`]. `0` = sukses.
    pub status: i32,
    /// `abi2::object_kind` objek.
    pub kind: u16,
    /// Slot objek di dalam tabel milik kind-nya.
    pub index: u16,
    /// Generasi anti-ABA.
    pub generation: u32,
    /// [`ObjectState`] sebagai `u8`.
    pub state: u8,
    pub reserved0: u8,
    pub reserved1: u16,
}

/// Hasil resolusi capability (identitas objek + hak yang dipegang).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosAbi2CapabilityOut {
    /// Kode [`Status`]. `0` = sukses.
    pub status: i32,
    pub kind: u16,
    pub index: u16,
    pub generation: u32,
    /// Hak yang dipegang capability (bitmask `abi2::right`).
    pub rights: u32,
}

/// `resolve(ctx, caller, handle) -> object`.
pub type ResolveObjectFn =
    extern "C" fn(KlatosAbi2Context, CallerId, Handle) -> KlatosAbi2ObjectOut;

/// `resolve(ctx, caller, handle) -> capability`.
pub type ResolveCapabilityFn =
    extern "C" fn(KlatosAbi2Context, CallerId, Handle) -> KlatosAbi2CapabilityOut;

/// Konteks dispatch eksplisit: pemanggil menyediakan konteks + dua resolver.
/// Tidak ada singleton global dan tidak ada registry.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosAbi2Resolvers {
    pub ctx: KlatosAbi2Context,
    pub resolve_object: ResolveObjectFn,
    pub resolve_capability: ResolveCapabilityFn,
    /// F-P7-01 — provider efek VFS/FS opsional (lapisan efek C++). `None`
    /// (penunjuk nol) = perilaku lama tanpa mutasi VFS. Cermin C++
    /// (`Jowo/Kontrak/Abi2DispatchKernel.h`): `resolve_effect`, dengan nilai
    /// awal `nullptr` sehingga setiap pemanggil lama tidak berubah.
    pub resolve_effect: Option<ResolveEffectFn>,
}

// ---------------------------------------------------------------------------
// Adaptor: penunjuk fungsi C -> trait P0.74
// ---------------------------------------------------------------------------

struct BridgeObjects<'a> {
    resolvers: &'a KlatosAbi2Resolvers,
}

impl ObjectResolver for BridgeObjects<'_> {
    fn resolve(&self, caller: CallerId, handle: Handle) -> Result<ResolvedObject, Status> {
        let out = (self.resolvers.resolve_object)(self.resolvers.ctx, caller, handle);
        if out.status != 0 {
            return Err(status_from_code(out.status));
        }
        Ok(ResolvedObject::new(
            ObjectId::new(out.kind, out.index, out.generation),
            object_state_from_u8(out.state),
        ))
    }
}

struct BridgeCapabilities<'a> {
    resolvers: &'a KlatosAbi2Resolvers,
}

impl CapabilityResolver for BridgeCapabilities<'_> {
    fn resolve(&self, caller: CallerId, handle: Handle) -> Result<Capability, Status> {
        let out = (self.resolvers.resolve_capability)(self.resolvers.ctx, caller, handle);
        if out.status != 0 {
            return Err(status_from_code(out.status));
        }
        Ok(Capability::new(
            ObjectId::new(out.kind, out.index, out.generation),
            out.rights,
        ))
    }
}

/// Dekode `ObjectState` dari `u8`. Nilai tak dikenal → `Alive` **bukan** karena
/// permisif: `Alive` adalah satu-satunya nilai yang membiarkan dispatcher
/// melanjutkan ke pemeriksaan hak, sehingga dekode yang salah gagal-tertutup.
const fn object_state_from_u8(value: u8) -> ObjectState {
    match value {
        1 => ObjectState::Revoked,
        2 => ObjectState::Destroyed,
        _ => ObjectState::Alive,
    }
}

// ---------------------------------------------------------------------------
// Simbol lama — tidak berubah (P0.75/P0.76)
// ---------------------------------------------------------------------------

/// Batas dispatch ABI v2 yang dipanggil kernel C++.
///
/// FASE-7 §7.5: jalur ini memakai MEKANISME nyata
/// ([`crate::mekanisme_abi2::MekanismeKernel`]), bukan [`dispatch::TokenMechanism`].
/// Operasi lama tetap mengembalikan token yang disepakati seam C++; operasi
/// VFS/FS/input menghasilkan nilai nyata atau galat pasti.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_dispatch(
    caller: CallerId,
    request: DispatchRequest,
    resolvers: KlatosAbi2Resolvers,
) -> DispatchResult {
    let objects = BridgeObjects { resolvers: &resolvers };
    let capabilities = BridgeCapabilities { resolvers: &resolvers };
    let mekanisme = match resolvers.resolve_effect {
        Some(f) => MekanismeKernel::dengan_efek(resolvers.ctx, f),
        None => MekanismeKernel::TANPA_EFEK,
    };
    let context = DispatchContext::dengan_mekanisme(caller, &objects, &capabilities, &mekanisme);
    dispatch::dispatch(&context, &request)
}

/// Penanda mekanisme lama. Nilainya hanya dapat berasal dari kode Rust ini.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_mechanism_tag() -> u64 {
    LEGACY_MECHANISM_TAG
}

/// Jumlah entri tabel dispatch yang tertanam di kernel.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_table_count() -> u32 {
    dispatch::DISPATCH_TABLE.len() as u32
}

/// Model operasi mekanisme, dapat ditanyakan saat runtime (P0.76).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_operation_rights(kind: u16, opcode: u8) -> u64 {
    ksm::operation_rights(kind, opcode)
}

// ---------------------------------------------------------------------------
// FASE-7 (B1) — pintu masuk ABI v2 Design B (dari KlatIpc/src/entry.rs)
// ---------------------------------------------------------------------------
//
// Simbol `klatos_abi2_entry_*` sebelumnya HANYA ada di `KlatIpcKernel` (crate
// yang TIDAK ditautkan ke kernel). C++ (`Jowo/Kontrak/Abi2EntryKernel.h`,
// `KlatIpc/Kanal/KlatDispatchChannel.cpp`) sudah memanggilnya, sehingga tautan
// kernel akan gagal. Blok ini memindahkan kelima simbol itu ke `KlatKernel`
// (arsip yang BENAR-BENAR ada di baris tautan, lihat Jowo/CMakeLists.txt:815).
//
// Muatan 48 byte menyeberang batas sebagai 6 kata `u64` **berdasarkan nilai**,
// bukan penunjuk mentah; itu menjaga invariant crate (`unsafe` = 0 di badan
// fungsi) dan memakai codec TUNGGAL milik `entry.rs`.

/// Muatan permintaan 48 byte sebagai 6 kata `u64` little-endian. Cermin C++:
/// `Jowo/Kontrak/Abi2EntryKernel.h` (`KlatosAbi2EntryWords`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosAbi2EntryWords {
    pub w0: u64,
    pub w1: u64,
    pub w2: u64,
    pub w3: u64,
    pub w4: u64,
    pub w5: u64,
}

impl KlatosAbi2EntryWords {
    pub const fn to_array(self) -> [u64; entry::REQUEST_WORD_COUNT] {
        [self.w0, self.w1, self.w2, self.w3, self.w4, self.w5]
    }
}

/// Muatan hasil 16 byte sebagai 2 kata `u64` little-endian. Cermin C++:
/// `KlatosAbi2EntryResult`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosAbi2EntryResult {
    pub w0: u64,
    pub w1: u64,
}

impl KlatosAbi2EntryResult {
    pub const fn from_array(words: [u64; entry::RESULT_WORD_COUNT]) -> Self {
        Self { w0: words[0], w1: words[1] }
    }
}

/// Validasi framing pesan KlatIPC untuk jalur dispatch (opcode pesan, jenis
/// permintaan, panjang muatan, versi protokol pesan). `0` = `Ok`.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_entry_validate(
    protocol_version: u8,
    opcode: u8,
    is_request: u8,
    payload_len: u32,
) -> i32 {
    entry::validate_request_message(
        protocol_version,
        opcode,
        is_request != 0,
        payload_len as usize,
    )
    .code()
}

/// Pintu masuk generik ABI v2: 48 byte permintaan (6 kata) → 16 byte hasil
/// (2 kata). `caller` berasal dari KERNEL, bukan dari permintaan. FASE-7:
/// memakai MEKANISME nyata (§7.5).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_entry_handle(
    caller: CallerId,
    words: KlatosAbi2EntryWords,
    resolvers: KlatosAbi2Resolvers,
) -> KlatosAbi2EntryResult {
    let objects = BridgeObjects { resolvers: &resolvers };
    let capabilities = BridgeCapabilities { resolvers: &resolvers };
    let mekanisme = match resolvers.resolve_effect {
        Some(f) => MekanismeKernel::dengan_efek(resolvers.ctx, f),
        None => MekanismeKernel::TANPA_EFEK,
    };
    let result = entry::handle_request_words_dengan_mekanisme(
        caller,
        words.to_array(),
        &objects,
        &capabilities,
        &mekanisme,
    );
    KlatosAbi2EntryResult::from_array(entry::encode_result_words(&result))
}

/// Opcode pesan KlatIPC yang menandai permintaan dispatch ABI v2.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_entry_message_opcode() -> u8 {
    entry::MESSAGE_OPCODE_DISPATCH
}

/// Jumlah kata `u64` muatan permintaan (6).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_entry_request_words() -> u32 {
    entry::REQUEST_WORD_COUNT as u32
}

/// Jumlah kata `u64` muatan hasil (2).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_entry_result_words() -> u32 {
    entry::RESULT_WORD_COUNT as u32
}

/// Opcode pesan KlatIPC untuk permintaan baca-balik Memory (F-P7-06).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_memory_readback_opcode() -> u8 {
    entry::MESSAGE_OPCODE_MEMORY_READBACK
}

/// Ukuran muatan permintaan baca-balik Memory (24).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_memory_readback_request_size() -> u32 {
    entry::MEMORY_READBACK_REQUEST_SIZE as u32
}

/// Batas byte baca-balik dalam satu panggilan (4096).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_memory_readback_max() -> u32 {
    entry::MEMORY_READBACK_MAX as u32
}

/// Pintu masuk baca-balik Memory ABI v2 (F-P7-06).
///
/// `caller` berasal dari KERNEL, bukan dari permintaan. Capability diselesaikan
/// lewat tabel KSM yang SAMA (`Memory` + hak `READ` + generasi hidup). `out`
/// menerima `length` byte NYATA; `out_n` menerima jumlah byte tersalin.
/// Mengembalikan kode `Status` (0 = `Ok`).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_abi2_memory_readback(
    caller: CallerId,
    handle: u32,
    offset: u32,
    length: u32,
    out: *mut u8,
    out_cap: u32,
    out_n: *mut u32,
) -> i32 {
    if !out_n.is_null() {
        // SAFETY: pemanggil menyerahkan penunjuk `u32` yang sah (kontrak C ABI).
        unsafe { *out_n = 0 };
    }
    if out.is_null() || length > out_cap || (length as usize) > entry::MEMORY_READBACK_MAX {
        return Status::Invalid.code();
    }
    let dst = if length == 0 {
        &mut [][..]
    } else {
        // SAFETY: `out` non-null dan `length` byte berada di dalam objek
        // pemanggil (kontrak C ABI modul ini).
        unsafe { core::slice::from_raw_parts_mut(out, length as usize) }
    };
    match crate::mekanisme_abi2::baca_memori_ke(caller, handle, offset as usize, dst) {
        Ok(n) => {
            if !out_n.is_null() {
                // SAFETY: seperti di atas.
                unsafe { *out_n = n as u32 };
            }
            Status::Ok.code()
        }
        Err(s) => s.code(),
    }
}

// ---------------------------------------------------------------------------
// Simbol baru — prototipe inti kernel (P0.97)
// ---------------------------------------------------------------------------

/// Data boot yang diserahkan kernel C++ ke titik masuk Rust.
///
/// Cermin C++ ada di `Jowo/Kontrak/KlatKernelBridge.h` (`KlatosKernelBootInfo`).
/// Ukuran 32 byte, align 8 — dibuktikan `static_assert` di sisi C++ dan
/// `assert!` di [`conformance`] di sisi Rust.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosKernelBootInfo {
    /// [`crate::arch::ARCH_ID_X86_64`] untuk x86_64.
    pub arch_id: u32,
    /// Jumlah CPU yang dikenal kernel C++ saat entry.
    pub cpu_count: u32,
    /// `BootMethod` sebagai `u32`.
    pub boot_method: u32,
    /// Panjang cmdline kernel dalam byte.
    pub cmdline_length: u32,
    /// Ukuran halaman yang dilaporkan kernel C++.
    pub page_size: u64,
    /// Bitflag yang dicadangkan; harus `0` pada gate ini.
    pub flags: u64,
}

/// Laporan yang dikembalikan titik masuk Rust ke kernel C++.
///
/// Ukuran 64 byte, align 8. Cermin C++ ada di
/// `Jowo/Kontrak/KlatKernelBridge.h` (`KlatosKernelBootReport`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosKernelBootReport {
    /// [`ksm::MECHANISM_TAG`] — hanya dapat berasal dari Rust.
    pub tag: u64,
    /// Jumlah langkah yang direncanakan ([`boot::BOOT_STEPS`]).
    pub steps_total: u32,
    /// Jumlah langkah yang selesai.
    pub steps_completed: u32,
    /// Langkah serial: COM1 diinisialisasi Rust.
    pub serial_ok: u32,
    /// Langkah interupsi: flag IF dapat dibaca/dikendalikan Rust.
    pub interrupt_ok: u32,
    /// Langkah memori: round-trip stack + tanpa alokator.
    pub memory_ok: u32,
    /// Langkah penjadwal: model menyerahkan kendali, tanpa preemption.
    pub scheduler_ok: u32,
    /// Langkah IPC: tabel dispatch ABI v2 tertanam.
    pub ipc_ok: u32,
    /// Langkah KSM: model operasi menjawab konsisten.
    pub ksm_ok: u32,
    /// Keadaan bit IF **sebelum** probe interupsi.
    pub interrupts_enabled_before: u32,
    /// Jumlah entri tabel dispatch yang tertanam.
    pub dispatch_table_entries: u32,
    /// Checksum round-trip memori stack.
    pub memory_checksum: u64,
    /// `0` = seluruh langkah lulus; selain itu indeks langkah pertama yang
    /// gagal (1-based).
    pub status: i32,
}

impl KlatosKernelBootReport {
    /// Laporan kosong. `const` supaya dapat dipakai tanpa alokasi.
    pub const fn empty() -> Self {
        Self {
            tag: ksm::MECHANISM_TAG,
            steps_total: boot::BOOT_STEPS,
            steps_completed: 0,
            serial_ok: 0,
            interrupt_ok: 0,
            memory_ok: 0,
            scheduler_ok: 0,
            ipc_ok: 0,
            ksm_ok: 0,
            interrupts_enabled_before: 0,
            dispatch_table_entries: 0,
            memory_checksum: 0,
            status: 0,
        }
    }

    /// Lulus bila seluruh langkah selesai dan tidak ada status galat.
    pub const fn ok(&self) -> bool {
        self.status == 0
            && self.steps_completed == self.steps_total
            && self.steps_total == boot::BOOT_STEPS
    }
}

/// Hasil probe fondasi interupsi yang dapat diulang **kapan saja**.
///
/// Berbeda dari [`KlatosKernelBootReport::interrupt_ok`], yang diambil sekali di
/// titik masuk `init()` — yaitu saat kernel C++ **sengaja** mematikan interupsi
/// (`Jowo/Arsitektur/Processor.cpp:84`), sehingga arah `sti` tidak dapat diuji di
/// sana. Struktur ini dipakai oleh probe kedua yang dijalankan dari
/// `init_stage2`, ketika interupsi sudah hidup, sehingga arah `sti` benar-benar
/// dieksekusi dan keadaannya dikembalikan.
///
/// Ukuran 40 byte, align 8. Cermin C++ ada di
/// `Jowo/Kontrak/KlatKernelBridge.h` (`KlatosKernelInterruptProbe`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosKernelInterruptProbe {
    /// [`ksm::MECHANISM_TAG`] — hanya dapat berasal dari Rust.
    pub tag: u64,
    /// Jumlah vektor IDT yang dimodelkan.
    pub vectors: u32,
    /// Jumlah jalur IRQ yang dimodelkan.
    pub irq_lines: u32,
    /// Keadaan bit IF **sebelum** probe.
    pub enabled_before: u32,
    /// `cli` terbukti membersihkan IF.
    pub disable_verified: u32,
    /// Probe mencoba `sti` (hanya bila interupsi sudah hidup).
    pub enable_attempted: u32,
    /// `sti` terbukti menyetel IF.
    pub enable_verified: u32,
    /// `0` = lulus; selain itu gagal.
    pub status: i32,
}

impl KlatosKernelInterruptProbe {
    /// Lulus bila arah `cli` terbukti, arah `sti` yang dicoba terbukti, dan
    /// keadaan awal dipulihkan. Arah `sti` **wajib** dicoba pada probe ini.
    pub const fn ok(&self) -> bool {
        self.status == 0
            && self.enable_attempted == 1
            && self.disable_verified == 1
            && self.enable_verified == 1
    }
}

/// Titik masuk inti kernel Rust (P0.97.3).
///
/// Dipanggil kernel C++ lewat `Jowo/Kontrak/KlatKernelBridge.h`. Fungsi ini
/// menjalankan urutan inisialisasi Rust, menulis jejak ke serial, lalu
/// **mengembalikan kendali** ke pemanggil — jalur boot produksi tidak diganti.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_kernel_entry(info: KlatosKernelBootInfo) -> KlatosKernelBootReport {
    boot::entry(info)
}

/// Probe fondasi interupsi **kedua**, dipanggil dari `init_stage2` ketika
/// interupsi sudah hidup, sehingga arah `sti` benar-benar dieksekusi.
///
/// Aman dipanggil kapan saja: `interrupt::probe()` selalu mengembalikan keadaan
/// awal CPU, dan hanya mencoba `sti` bila interupsi memang sudah menyala.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_kernel_interrupt_probe() -> KlatosKernelInterruptProbe {
    let f = crate::interrupt::probe();
    let mut status = 0i32;
    if !f.disable_verified {
        status = 1;
    } else if !f.enable_attempted {
        status = 2;
    } else if !f.enable_verified {
        status = 3;
    }
    KlatosKernelInterruptProbe {
        tag: ksm::MECHANISM_TAG,
        vectors: f.vectors,
        irq_lines: f.irq_lines,
        enabled_before: f.enabled_before as u32,
        disable_verified: f.disable_verified as u32,
        enable_attempted: f.enable_attempted as u32,
        enable_verified: f.enable_verified as u32,
        status,
    }
}

/// Penanda mekanisme inti kernel. Hanya dapat berasal dari Rust.
#[unsafe(no_mangle)]
pub extern "C" fn klatos_kernel_mechanism_tag() -> u64 {
    ksm::MECHANISM_TAG
}

/// Laporan matriks memori + task P0.98.6. Cermin C++ ada di
/// `Jowo/Kontrak/KlatKernelBridge.h` (`KlatosKernelMemTaskReport`).
///
/// Ukuran 56 byte, align 8.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosKernelMemTaskReport {
    /// [`ksm::MECHANISM_TAG`] — hanya dapat berasal dari Rust.
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub mem_checks: u32,
    pub mem_passed: u32,
    pub task_checks: u32,
    pub task_passed: u32,
    pub auth_checks: u32,
    pub auth_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    /// `0` = lulus; selain itu jumlah kegagalan.
    pub status: i32,
}

impl KlatosKernelMemTaskReport {
    /// Lulus bila tidak ada kegagalan dan semua kelompok negatif lengkap.
    pub const fn ok(&self) -> bool {
        self.status == 0
            && self.checks == self.passed
            && self.checks > 0
            && self.neg_checks > 0
            && self.neg_checks == self.neg_passed
    }
}

/// Menjalankan matriks P0.98.6 terhadap model memori + task Rust.
///
/// `hak_memori` dan `hak_task` **wajib** berasal dari KSM C++ yang nyata
/// (`KlatIpc/Kanal/OtoritasEndpoint.h`). Rust tidak menambah, mengurangi, atau
/// menyimpan nilai itu — lihat [`crate::ksm::Izin`].
#[unsafe(no_mangle)]
pub extern "C" fn klatos_kernel_mem_task_selftest(
    hak_memori: u32,
    hak_task: u32,
) -> KlatosKernelMemTaskReport {
    let laporan = crate::memtask::jalankan(hak_memori, hak_task);
    KlatosKernelMemTaskReport {
        tag: laporan.tag,
        checks: laporan.checks,
        passed: laporan.passed,
        mem_checks: laporan.mem_checks,
        mem_passed: laporan.mem_passed,
        task_checks: laporan.task_checks,
        task_passed: laporan.task_passed,
        auth_checks: laporan.auth_checks,
        auth_passed: laporan.auth_passed,
        neg_checks: laporan.neg_checks,
        neg_passed: laporan.neg_passed,
        status: laporan.status,
    }
}

/// Probe jalur panic. **Tidak pernah kembali** (P0.97.6).
#[unsafe(no_mangle)]
pub extern "C" fn klatos_kernel_panic_probe() -> ! {
    crate::panic::probe()
}

// ---------------------------------------------------------------------------
// Konformansi tata letak batas FFI
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;
    use crate::abi2;
    use core::mem::{align_of, offset_of, size_of};

    const _: () = {
        assert!(size_of::<KlatosAbi2ObjectOut>() == 16);
        assert!(align_of::<KlatosAbi2ObjectOut>() == 4);
        assert!(offset_of!(KlatosAbi2ObjectOut, status) == 0);
        assert!(offset_of!(KlatosAbi2ObjectOut, kind) == 4);
        assert!(offset_of!(KlatosAbi2ObjectOut, index) == 6);
        assert!(offset_of!(KlatosAbi2ObjectOut, generation) == 8);
        assert!(offset_of!(KlatosAbi2ObjectOut, state) == 12);

        assert!(size_of::<KlatosAbi2CapabilityOut>() == 16);
        assert!(align_of::<KlatosAbi2CapabilityOut>() == 4);
        assert!(offset_of!(KlatosAbi2CapabilityOut, status) == 0);
        assert!(offset_of!(KlatosAbi2CapabilityOut, kind) == 4);
        assert!(offset_of!(KlatosAbi2CapabilityOut, index) == 6);
        assert!(offset_of!(KlatosAbi2CapabilityOut, generation) == 8);
        assert!(offset_of!(KlatosAbi2CapabilityOut, rights) == 12);

        // P0.97 — cermin boot info/report.
        assert!(size_of::<KlatosKernelBootInfo>() == 32);
        assert!(align_of::<KlatosKernelBootInfo>() == 8);
        assert!(offset_of!(KlatosKernelBootInfo, arch_id) == 0);
        assert!(offset_of!(KlatosKernelBootInfo, cpu_count) == 4);
        assert!(offset_of!(KlatosKernelBootInfo, boot_method) == 8);
        assert!(offset_of!(KlatosKernelBootInfo, cmdline_length) == 12);
        assert!(offset_of!(KlatosKernelBootInfo, page_size) == 16);
        assert!(offset_of!(KlatosKernelBootInfo, flags) == 24);

        assert!(size_of::<KlatosKernelBootReport>() == 64);
        assert!(align_of::<KlatosKernelBootReport>() == 8);
        assert!(offset_of!(KlatosKernelBootReport, tag) == 0);
        assert!(offset_of!(KlatosKernelBootReport, steps_total) == 8);
        assert!(offset_of!(KlatosKernelBootReport, steps_completed) == 12);
        assert!(offset_of!(KlatosKernelBootReport, serial_ok) == 16);
        assert!(offset_of!(KlatosKernelBootReport, interrupt_ok) == 20);
        assert!(offset_of!(KlatosKernelBootReport, memory_ok) == 24);
        assert!(offset_of!(KlatosKernelBootReport, scheduler_ok) == 28);
        assert!(offset_of!(KlatosKernelBootReport, ipc_ok) == 32);
        assert!(offset_of!(KlatosKernelBootReport, ksm_ok) == 36);
        assert!(offset_of!(KlatosKernelBootReport, interrupts_enabled_before) == 40);
        assert!(offset_of!(KlatosKernelBootReport, dispatch_table_entries) == 44);
        assert!(offset_of!(KlatosKernelBootReport, memory_checksum) == 48);
        assert!(offset_of!(KlatosKernelBootReport, status) == 56);

        // P0.97 — cermin probe interupsi kedua.
        assert!(size_of::<KlatosKernelInterruptProbe>() == 40);
        assert!(align_of::<KlatosKernelInterruptProbe>() == 8);
        assert!(offset_of!(KlatosKernelInterruptProbe, tag) == 0);
        assert!(offset_of!(KlatosKernelInterruptProbe, vectors) == 8);
        assert!(offset_of!(KlatosKernelInterruptProbe, irq_lines) == 12);
        assert!(offset_of!(KlatosKernelInterruptProbe, enabled_before) == 16);
        assert!(offset_of!(KlatosKernelInterruptProbe, disable_verified) == 20);
        assert!(offset_of!(KlatosKernelInterruptProbe, enable_attempted) == 24);
        assert!(offset_of!(KlatosKernelInterruptProbe, enable_verified) == 28);
        assert!(offset_of!(KlatosKernelInterruptProbe, status) == 32);

        // P0.98 — cermin matriks memori + task.
        assert!(size_of::<KlatosKernelMemTaskReport>() == 56);
        assert!(align_of::<KlatosKernelMemTaskReport>() == 8);
        assert!(offset_of!(KlatosKernelMemTaskReport, tag) == 0);
        assert!(offset_of!(KlatosKernelMemTaskReport, checks) == 8);
        assert!(offset_of!(KlatosKernelMemTaskReport, passed) == 12);
        assert!(offset_of!(KlatosKernelMemTaskReport, mem_checks) == 16);
        assert!(offset_of!(KlatosKernelMemTaskReport, mem_passed) == 20);
        assert!(offset_of!(KlatosKernelMemTaskReport, task_checks) == 24);
        assert!(offset_of!(KlatosKernelMemTaskReport, task_passed) == 28);
        assert!(offset_of!(KlatosKernelMemTaskReport, auth_checks) == 32);
        assert!(offset_of!(KlatosKernelMemTaskReport, auth_passed) == 36);
        assert!(offset_of!(KlatosKernelMemTaskReport, neg_checks) == 40);
        assert!(offset_of!(KlatosKernelMemTaskReport, neg_passed) == 44);
        assert!(offset_of!(KlatosKernelMemTaskReport, status) == 48);

        // Penanda: lama tidak berubah, baru berbeda dari lama.
        assert!(LEGACY_MECHANISM_TAG == 0x4B4C_4154_5255_5354);
        assert!(ksm::MECHANISM_TAG == 0x4B4C_4154_4B52_4E4C);
        assert!(LEGACY_MECHANISM_TAG != ksm::MECHANISM_TAG);
        // FASE-5: tabel tumbuh 15 -> 58 (append-only).
        assert!(dispatch::DISPATCH_TABLE.len() == 60);

        // Dekode keadaan: hanya Alive yang membiarkan lanjut.
        assert!(matches!(object_state_from_u8(0), ObjectState::Alive));
        assert!(matches!(object_state_from_u8(1), ObjectState::Revoked));
        assert!(matches!(object_state_from_u8(2), ObjectState::Destroyed));

        // Model KSM harus sama dengan tabel statis (bukan tabel kedua).
        assert!(ksm::operation_rights(abi2::object_kind::ENDPOINT, dispatch::op_endpoint::SEND)
            == ksm::FOUND | abi2::right::SEND as u64);
        assert!(ksm::operation_rights(abi2::object_kind::ENDPOINT, 200) == 0);

        // FASE-7 (B1): bentuk muatan pintu masuk ABI v2 di crate yang DITautkan.
        assert!(size_of::<KlatosAbi2EntryWords>() == 48);
        assert!(align_of::<KlatosAbi2EntryWords>() == 8);
        assert!(size_of::<KlatosAbi2EntryResult>() == 16);
        assert!(align_of::<KlatosAbi2EntryResult>() == 8);
        assert!(entry::REQUEST_WIRE_SIZE == 48);
        assert!(entry::RESULT_WIRE_SIZE == 16);
        assert!(entry::REQUEST_WORD_COUNT == 6);
        assert!(entry::RESULT_WORD_COUNT == 2);
        assert!(entry::MESSAGE_OPCODE_DISPATCH == 0xA2);

        // FASE-7 (B2): mekanisme nyata terdaftar di manifes integrasi.
        assert!(crate::mekanisme_abi2::FASE5_OP_COUNT == 45);
        assert!(crate::mekanisme_abi2::REPORT_TAG == 0x4B4C_4154_4D45_4B41);
    };
}
