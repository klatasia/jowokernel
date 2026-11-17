// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Batas FPU/SSE untuk jalur trap (P1.04).
//!
//!
//! Modul ini membuktikan **protokol** penjaga (validasi, paritas, penghitung).

use core::arch::{asm, global_asm};

/// Ukuran cermin `Jowo::FPUState` (x86_64).
///
/// `LegacyRegion` (512) + `SIMD::Header` (64) + `ext_save_area` (256) = 832,
/// `alignas(64)` dan `packed` di sisi C++. Ini adalah **POD boundary** yang
/// diminta §P1.04.3: layout tetap, `#[repr(C)]`, tanpa pointer ke objek C++.
#[cfg(target_arch = "x86_64")]
pub const FPU_STATE_BYTES: u32 = 832;

/// Ukuran cermin `Jowo::FPUState` (aarch64).
///
/// `Efek/Arsitektur/aarch64/FPUState.h` mendefinisikan `u8 buffer[512]`
/// (`[[gnu::aligned(16)]]`); `FPUState.S` menyimpan q0..q31 (32 × 16 byte = 512)
/// lewat `stp`/`ldp`. Jadi ukuran state di aarch64 adalah 512.
#[cfg(target_arch = "aarch64")]
pub const FPU_STATE_BYTES: u32 = 512;

/// Ukuran area penjaga FPU (FXSAVE64).
pub const FPU_GUARD_BYTES: u32 = 512;

/// Tag laporan; **BERBEDA** dari tag laporan gate lain supaya laporan tidak
/// kehilangan daya pisah saat diaudit ("KLATFPUU").
pub const REPORT_TAG: u64 = 0x4B4C_4154_4650_5555;

/// Ajaib penjaga; membedakan area penjaga yang SAH dari sampah.
pub const GUARD_MAGIC: u64 = 0x4B4C_4154_4650_4755; // "KLATFPGU"

/// Ukuran pola uji yang ditulis/dibaca di sisi C++.
pub const PATTERN_BYTES: u32 = 256;

/// Penghitung audit.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Counter {
    /// Pemakaian penjaga pada jalur produksi (`guard_enter`/`guard_exit`).
    ProdGuard = 0,
    /// Penolakan penjaga (salah selaras, ajaib salah, ukuran salah).
    ErrGuard = 1,
    /// Tabrakan: penjaga dilepas pada konteks yang tidak pernah dimasuki.
    DriftGuard = 2,
    /// Pemanggilan `klatos_fpu_rust_clobber` (jalur uji yang merusak XMM).
    Clobber = 3,
}

const COUNTERS: usize = 4;

// ---------------------------------------------------------------------------
// Save/restore — ASSEMBLY UTUH, bukan blok `asm!`
// ---------------------------------------------------------------------------

// Setiap rutin di sini adalah fungsi lengkap: tidak ada satu instruksi pun
// yang dihasilkan LLVM di dalamnya. Itulah inti jaminannya, lihat dokumen
// modul bagian "Mengapa save/restore ditulis sebagai ASSEMBLY UTUH".
//
// `fxsave64`/`fxrstor64` mensyaratkan area 16 byte selaras. Pemanggil Rust
// memakai `(*guard).data` dari `KlatosXmmGuard` yang `align(64)`, jadi
// `rdi` selalu selaras 64. Pemeriksaan selaras tetap dilakukan di sisi Rust
// supaya pelanggaran kontrak gagal-tertutup, bukan `#GP`.
// CATATAN SINTAKS — Rust memakai sintaks Intel sebagai default, dan assembler
// menolak bentuk AT&T (`fxsave64 (%rdi)`) dengan "unknown token in expression".
// Karena itu seluruh rutin di bawah ditulis Intel. Jangan menambahkan direktif
// `.intel_syntax`/`.att_syntax` di sini: direktif sintaks berlaku sampai akhir
// berkas, sehingga ia akan mengubah arti setiap `global_asm!` berikutnya di
// crate ini — dan rustc sendiri memperingatkan hal itu.
#[cfg(target_arch = "x86_64")]
global_asm!(
    // --- simpan ---------------------------------------------------------
    ".globl klatos_fpu_asm_save",
    ".type  klatos_fpu_asm_save, @function",
    "klatos_fpu_asm_save:",
    "    fxsave64 [rdi]",
    // Intel mensyaratkan byte 464..511 bernilai 1 saat FXRSTOR. FXSAVE
    // menulisnya, tetapi nilainya bergantung implementasi, jadi ditetapkan
    // eksplisit — dengan instruksi integer, supaya rutin ini tidak menyentuh
    // register XMM di luar fxsave64 itu sendiri.
    "    mov qword ptr [rdi + 464], -1",
    "    mov qword ptr [rdi + 472], -1",
    "    mov qword ptr [rdi + 480], -1",
    "    mov qword ptr [rdi + 488], -1",
    "    mov qword ptr [rdi + 496], -1",
    "    mov qword ptr [rdi + 504], -1",
    "    ret",
    ".size klatos_fpu_asm_save, . - klatos_fpu_asm_save",

    // --- pulihkan -------------------------------------------------------
    ".globl klatos_fpu_asm_restore",
    ".type  klatos_fpu_asm_restore, @function",
    "klatos_fpu_asm_restore:",
    "    fxrstor64 [rdi]",
    "    ret",
    ".size klatos_fpu_asm_restore, . - klatos_fpu_asm_restore",
);

// aarch64: FP/SIMD state = q0..q31 (512 byte). Cermin `FPUState.S` C++
// (`Efek/Arsitektur/aarch64/FPUState.S`), dengan ABI AAPCS64: argumen pertama di
// `x0`. Rutin ini menyentuh seluruh register q, jadi ia memang harus assembly
// utuh (LLVM tidak boleh menyisipkan instruksi di antara simpan/pulih).
#[cfg(target_arch = "aarch64")]
global_asm!(
    // --- simpan ---------------------------------------------------------
    ".globl klatos_fpu_asm_save",
    ".type  klatos_fpu_asm_save, %function",
    "klatos_fpu_asm_save:",
    "    stp q0,  q1,  [x0, #(0 * 16)]",
    "    stp q2,  q3,  [x0, #(2 * 16)]",
    "    stp q4,  q5,  [x0, #(4 * 16)]",
    "    stp q6,  q7,  [x0, #(6 * 16)]",
    "    stp q8,  q9,  [x0, #(8 * 16)]",
    "    stp q10, q11, [x0, #(10 * 16)]",
    "    stp q12, q13, [x0, #(12 * 16)]",
    "    stp q14, q15, [x0, #(14 * 16)]",
    "    stp q16, q17, [x0, #(16 * 16)]",
    "    stp q18, q19, [x0, #(18 * 16)]",
    "    stp q20, q21, [x0, #(20 * 16)]",
    "    stp q22, q23, [x0, #(22 * 16)]",
    "    stp q24, q25, [x0, #(24 * 16)]",
    "    stp q26, q27, [x0, #(26 * 16)]",
    "    stp q28, q29, [x0, #(28 * 16)]",
    "    stp q30, q31, [x0, #(30 * 16)]",
    "    ret",
    ".size klatos_fpu_asm_save, . - klatos_fpu_asm_save",

    // --- pulihkan -------------------------------------------------------
    ".globl klatos_fpu_asm_restore",
    ".type  klatos_fpu_asm_restore, %function",
    "klatos_fpu_asm_restore:",
    "    ldp q0,  q1,  [x0, #(0 * 16)]",
    "    ldp q2,  q3,  [x0, #(2 * 16)]",
    "    ldp q4,  q5,  [x0, #(4 * 16)]",
    "    ldp q6,  q7,  [x0, #(6 * 16)]",
    "    ldp q8,  q9,  [x0, #(8 * 16)]",
    "    ldp q10, q11, [x0, #(10 * 16)]",
    "    ldp q12, q13, [x0, #(12 * 16)]",
    "    ldp q14, q15, [x0, #(14 * 16)]",
    "    ldp q16, q17, [x0, #(16 * 16)]",
    "    ldp q18, q19, [x0, #(18 * 16)]",
    "    ldp q20, q21, [x0, #(20 * 16)]",
    "    ldp q22, q23, [x0, #(22 * 16)]",
    "    ldp q24, q25, [x0, #(24 * 16)]",
    "    ldp q26, q27, [x0, #(26 * 16)]",
    "    ldp q28, q29, [x0, #(28 * 16)]",
    "    ldp q30, q31, [x0, #(30 * 16)]",
    "    ret",
    ".size klatos_fpu_asm_restore, . - klatos_fpu_asm_restore",
);

extern "C" {
    /// Menyimpan XMM0..15 + MXCSR + x87 ke area 512 byte di `rdi`.
    fn klatos_fpu_asm_save(area: *mut u8);
    /// Memulihkan XMM0..15 + MXCSR + x87 dari area 512 byte di `rdi`.
    fn klatos_fpu_asm_restore(area: *const u8);
}

// ---------------------------------------------------------------------------
// POD boundary — cermin Jowo::FPUState
// ---------------------------------------------------------------------------

/// Cermin POD `Jowo::FPUState` (832 byte, align 64).
///
/// Ini BUKAN pemilik state FPU thread. Ia hanya wadah yang dipakai uji P1.04
/// untuk membuktikan bahwa fungsi produksi C++
/// (`Processor::store_fpu_state`/`load_fpu_state`) memulihkan pola yang
/// ditulis Rust secara utuh.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct KlatosFpuState {
    /// Isi mentah; tata letaknya milik C++.
    pub data: [u8; FPU_STATE_BYTES as usize],
}

/// Area penjaga FXSAVE + metadata validasi.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct KlatosXmmGuard {
    /// Ajaib `GUARD_MAGIC` bila area ini pernah diisi `guard_enter`.
    pub magic: u64,
    /// Paritas pemakaian; ganjil = masuk tanpa keluar.
    pub parity: u64,
    /// Area FXSAVE64; HARUS berada pada kelipatan 16 dari awal struct.
    pub data: [u8; FPU_GUARD_BYTES as usize],
}

/// Laporan selftest FPU/trap untuk C++.
///
/// `clobber_hits` dan `guard_hits` diisi oleh **sisi C++** (fisika XMM), bukan
/// oleh `run_matrix`; lihat dokumen modul. `run_matrix` menandainya dengan
/// `SENTINEL_C_FILLS` supaya audit dapat membuktikan bahwa angka yang
/// dilaporkan memang berasal dari pengukuran C++, bukan dari Rust.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosFpuReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub ownership_checks: u32,
    pub ownership_passed: u32,
    pub guard_checks: u32,
    pub guard_passed: u32,
    pub roundtrip_checks: u32,
    pub roundtrip_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    /// Diharapkan `SENTINEL_C_FILLS` dari Rust.
    pub clobber_hits: u32,
    /// Diharapkan `SENTINEL_C_FILLS` dari Rust.
    pub guard_hits: u32,
    pub state_bytes: u32,
    pub guard_bytes: u32,
    pub status: i32,
}

/// Nilai penanda "kolom ini diisi sisi C++".
pub const SENTINEL_C_FILLS: u32 = 0xFFFF_FFFF;

// ---------------------------------------------------------------------------
// Penghitung
// ---------------------------------------------------------------------------

static mut COUNTER_VALUES: [u32; COUNTERS] = [0; COUNTERS];

/// Indeks -> varian, tanpa `transmute` (nilai di luar rentang tidak pernah
/// menjadi nilai enum yang tak sah).
const fn counter_from_index(index: usize) -> Option<Counter> {
    match index {
        0 => Some(Counter::ProdGuard),
        1 => Some(Counter::ErrGuard),
        2 => Some(Counter::DriftGuard),
        3 => Some(Counter::Clobber),
        _ => None,
    }
}

fn bump(counter: Counter) {
    // SAFETY dari sudut pandang konkurensi: kernel memanggil permukaan ini dari
    // init_stage2 (satu CPU, interupsi mati selama matriks) dan dari jalur trap
    // yang sudah memasang penjaga. `u32` non-atomik dipakai karena itu perilaku
    // yang SAMA dengan modul otoritas lain di crate ini (`clock_id.rs`,
    // `ipc_handle.rs`) dan tidak ada jalur SMP di gate ini.
    //
    // Penulisan lewat pointer mentah (bukan `&mut static mut`) supaya tidak
    // memakai referensi ke `static mut` — pola yang dilarang lint
    // `static_mut_refs`.
    unsafe {
        let slot = core::ptr::addr_of_mut!(COUNTER_VALUES[counter as usize]);
        *slot = (*slot).wrapping_add(1);
    }
}

fn read_counter(counter: Counter) -> u32 {
    unsafe { core::ptr::read(core::ptr::addr_of!(COUNTER_VALUES[counter as usize])) }
}

fn reset_counters() {
    unsafe {
        for index in 0..COUNTERS {
            core::ptr::write(core::ptr::addr_of_mut!(COUNTER_VALUES[index]), 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Penjaga — protokol
// ---------------------------------------------------------------------------

/// Memasang penjaga: menyimpan XMM0..15 + MXCSR + x87 milik pemanggil.
///
/// # Safety
/// `guard` harus menunjuk `KlatosXmmGuard` yang hidup dan selaras 64.
unsafe fn guard_enter_raw(guard: *mut KlatosXmmGuard) -> i32 {
    if (guard as usize) % 64 != 0 {
        bump(Counter::ErrGuard);
        return -1;
    }
    // SIMPAN adalah tindakan XMM PERTAMA di region yang dijaga, dan ia berada
    // di rutin assembly yang LLVM tidak dapat menyisipkan instruksi ke
    // dalamnya. Semua yang terjadi sesudah baris ini boleh merusak XMM.
    klatos_fpu_asm_save(core::ptr::addr_of_mut!((*guard).data) as *mut u8);

    // Bookkeeping SETELAH penyimpanan. Kalau LLVM memakai XMM untuk menulis
    // dua bidang ini, kerusakannya tidak relevan: salinan pemanggil sudah ada
    // di `data`.
    let parity = (*guard).parity;
    (*guard).magic = GUARD_MAGIC;
    (*guard).parity = parity ^ 1;
    bump(Counter::ProdGuard);
    0
}

/// Melepas penjaga: memulihkan XMM0..15 + MXCSR + x87 milik pemanggil.
///
/// Kembalikan `0` bila berhasil; negatif bila area tidak sah (gagal-tertutup).
///
/// # Safety
/// `guard` harus menunjuk `KlatosXmmGuard` hidup dan selaras 64 yang berasal
/// dari [`guard_enter_raw`].
unsafe fn guard_exit_raw(guard: *const KlatosXmmGuard) -> i32 {
    if (guard as usize) % 64 != 0 {
        bump(Counter::ErrGuard);
        return -1;
    }
    if (*guard).magic != GUARD_MAGIC {
        bump(Counter::ErrGuard);
        return -2;
    }
    if (*guard).parity != 1 {
        // Melepas penjaga yang tidak pernah dipasang, atau dipasang dua kali.
        bump(Counter::DriftGuard);
        return -3;
    }

    // Bookkeeping SEBELUM pemulihan: setelah `fxrstor64` selesai kita tidak
    // boleh menyentuh XMM lagi, jadi semua yang bukan pemulihan harus
    // dikerjakan lebih dulu.
    let mutable = guard as *mut KlatosXmmGuard;
    (*mutable).magic = 0;
    (*mutable).parity = 0;
    bump(Counter::ProdGuard);

    // PEMULIHAN adalah tindakan XMM TERAKHIR. Rutin asm ini hanya berisi
    // `fxrstor64` dan `ret`.
    klatos_fpu_asm_restore(core::ptr::addr_of!((*guard).data) as *const u8);
    0
}

// ---------------------------------------------------------------------------
// Primitif uji sisi Rust
//
// Yang diuji di sini adalah PROTOKOL penjaga, yang seluruhnya aritmetika
// integer dan karena itu dapat diukur dengan jujur dari Rust. Fisika XMM
// diukur dari C++.
// ---------------------------------------------------------------------------

struct Matrix {
    checks: u32,
    passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Self { checks: 0, passed: 0 }
    }

    fn catat(&mut self, condition: bool) {
        self.checks += 1;
        if condition {
            self.passed += 1;
        }
    }
}

/// Membangun pola deterministik 256 byte dari `seed`.
///
/// Aritmetika integer murni: tidak ada float, jadi pola tidak bergantung pada
/// pembulatan FP dan dapat direproduksi PERSIS di sisi C++ (yang mencetak
/// pola yang diharapkan dengan rumus yang sama).
fn build_pattern(seed: u32, out: &mut [u8; PATTERN_BYTES as usize]) {
    for (index, byte) in out.iter_mut().enumerate() {
        let value = (seed as usize)
            .wrapping_mul(131)
            .wrapping_add(index.wrapping_mul(7))
            .wrapping_add(0x5A);
        *byte = value as u8;
    }
}

/// Menjalankan matriks P1.04.
pub fn run_matrix() -> KlatosFpuReport {
    let mut report = KlatosFpuReport {
        tag: REPORT_TAG,
        checks: 0,
        passed: 0,
        ownership_checks: 0,
        ownership_passed: 0,
        guard_checks: 0,
        guard_passed: 0,
        roundtrip_checks: 0,
        roundtrip_passed: 0,
        neg_checks: 0,
        neg_passed: 0,
        clobber_hits: SENTINEL_C_FILLS,
        guard_hits: SENTINEL_C_FILLS,
        state_bytes: FPU_STATE_BYTES,
        guard_bytes: FPU_GUARD_BYTES,
        status: -1,
    };

    let mut total = Matrix::new();
    let mut ownership = Matrix::new();
    let mut guard_m = Matrix::new();
    let mut roundtrip = Matrix::new();
    let mut negative = Matrix::new();

    // --- 1. Kepemilikan & tata letak POD -----------------------------------
    ownership.catat(core::mem::size_of::<KlatosFpuState>() == FPU_STATE_BYTES as usize);
    ownership.catat(core::mem::size_of::<KlatosFpuState>() % 64 == 0);
    ownership.catat(core::mem::align_of::<KlatosFpuState>() == 64);
    ownership.catat(core::mem::align_of::<KlatosXmmGuard>() == 64);
    ownership.catat(core::mem::size_of::<KlatosXmmGuard>() == 576);
    ownership.catat(core::mem::align_of::<KlatosXmmGuard>() == 64);
    // `data` harus 16-byte aligned mutlak: syarat FXSAVE64.
    let mut probe = KlatosXmmGuard { magic: 0, parity: 0, data: [0u8; FPU_GUARD_BYTES as usize] };
    let data_address = core::ptr::addr_of_mut!(probe.data) as usize;
    ownership.catat(data_address % 16 == 0);
    ownership.catat(FPU_GUARD_BYTES == 512);
    ownership.catat(core::mem::size_of::<KlatosFpuReport>() == 72);
    ownership.catat(core::mem::align_of::<KlatosFpuReport>() == 8);
    ownership.catat(REPORT_TAG == 0x4B4C_4154_4650_5555);

    // --- 2. Protokol penjaga ----------------------------------------------
    // P / M : pasang lalu lepas, sekali. Keduanya harus menerima.
    let mut once = KlatosXmmGuard { magic: 0, parity: 0, data: [0u8; FPU_GUARD_BYTES as usize] };
    let entered = unsafe { guard_enter_raw(&mut once) };
    guard_m.catat(entered == 0);
    guard_m.catat(once.magic == GUARD_MAGIC);
    guard_m.catat(once.parity == 1);
    let exited = unsafe { guard_exit_raw(&once) };
    guard_m.catat(exited == 0);
    // Setelah pelepasan yang berhasil, area TIDAK boleh lagi tampak sah.
    guard_m.catat(once.magic == 0);
    guard_m.catat(once.parity == 0);

    // --- 3. Bolak-balik A -> B -> A pada primitif yang SAMA dengan context
    //        switch produksi (fxsave64/fxrstor64) ---------------------------
    //
    // CATATAN METODE — kenapa TIGA area, bukan dua
    //   `guard_exit_raw` menghapus ajaib setelah pelepasan yang berhasil,
    //   sehingga pelepasan kedua DITOLAK (-2). Invarian itu benar untuk sebuah
    //   penjaga dan memang diuji di N2. Versi pertama matriks ini melepas
    //   `context_a` dua kali untuk melakukan "A -> B -> A", lalu melaporkan
    //   kegagalan yang sebenarnya cacat ujinya sendiri. Kunjungan kedua ke A
    //   karena itu menyimpan keadaannya SENDIRI, seperti kunjungan nyata kedua
    //   ke sebuah thread.
    let mut context_a1 = KlatosXmmGuard { magic: 0, parity: 0, data: [0u8; FPU_GUARD_BYTES as usize] };
    let mut context_b = KlatosXmmGuard { magic: 0, parity: 0, data: [0u8; FPU_GUARD_BYTES as usize] };
    let mut context_a2 = KlatosXmmGuard { magic: 0, parity: 0, data: [0u8; FPU_GUARD_BYTES as usize] };

    let saved_a1 = unsafe { guard_enter_raw(&mut context_a1) };
    let saved_b = unsafe { guard_enter_raw(&mut context_b) };
    let saved_a2 = unsafe { guard_enter_raw(&mut context_a2) };
    roundtrip.catat(saved_a1 == 0 && saved_b == 0 && saved_a2 == 0);

    // Tiga konteks hidup berdampingan: inilah yang membuat ini "switch" dan
    // bukan sekadar "simpan lalu pulihkan".
    roundtrip.catat(context_a1.magic == GUARD_MAGIC && context_b.magic == GUARD_MAGIC
        && context_a2.magic == GUARD_MAGIC);
    // tiga paritas ganjil: tiga penjaga terpasang, belum ada yang dilepas.
    roundtrip.catat(context_a1.parity == 1 && context_b.parity == 1 && context_a2.parity == 1);

    // Lepas dalam urutan A -> B -> A. Ketiganya harus diterima.
    roundtrip.catat(unsafe { guard_exit_raw(&context_a1) } == 0);
    roundtrip.catat(unsafe { guard_exit_raw(&context_b) } == 0);
    roundtrip.catat(unsafe { guard_exit_raw(&context_a2) } == 0);
    roundtrip.catat(context_a1.magic == 0 && context_b.magic == 0 && context_a2.magic == 0);
    // Area yang sudah dilepas tidak dapat dilepas lagi.
    roundtrip.catat(unsafe { guard_exit_raw(&context_a1) } != 0);

    // --- 4. Negatif: semua harus fail-closed -------------------------------
    // N1: area penjaga yang belum pernah dimasuki ditolak (ajaib 0).
    let untouched = KlatosXmmGuard { magic: 0, parity: 0, data: [0u8; FPU_GUARD_BYTES as usize] };
    negative.catat(unsafe { guard_exit_raw(&untouched) } == -2);

    // N2: pelepasan ganda ditolak, dan area itu TIDAK berubah menjadi "sah".
    let mut double = KlatosXmmGuard { magic: 0, parity: 0, data: [0u8; FPU_GUARD_BYTES as usize] };
    let first = unsafe { guard_enter_raw(&mut double) };
    let second = unsafe { guard_exit_raw(&double) };
    let third = unsafe { guard_exit_raw(&double) };
    negative.catat(first == 0 && second == 0 && third == -2);
    negative.catat(double.magic == 0 && double.parity == 0);

    // N3: ajaib benar tetapi paritas salah (seolah disisipkan) ditolak sebagai
    // pelepasan menyimpang — bukan sebagai ajaib salah.
    let mut forged = KlatosXmmGuard { magic: GUARD_MAGIC, parity: 0, data: [0u8; FPU_GUARD_BYTES as usize] };
    negative.catat(unsafe { guard_exit_raw(&forged) } == -3);
    // Paritas genap lain (3) juga menyimpang: yang sah HANYA 1.
    forged.parity = 3;
    negative.catat(unsafe { guard_exit_raw(&forged) } == -3);
    // Dan pelepasan menyimpang TIDAK mengubah area menjadi "sudah dilepas".
    negative.catat(forged.magic == GUARD_MAGIC);

    total.checks = ownership.checks + guard_m.checks + roundtrip.checks + negative.checks;
    total.passed = ownership.passed + guard_m.passed + roundtrip.passed + negative.passed;

    report.checks = total.checks;
    report.passed = total.passed;
    report.ownership_checks = ownership.checks;
    report.ownership_passed = ownership.passed;
    report.guard_checks = guard_m.checks;
    report.guard_passed = guard_m.passed;
    report.roundtrip_checks = roundtrip.checks;
    report.roundtrip_passed = roundtrip.passed;
    report.neg_checks = negative.checks;
    report.neg_passed = negative.passed;
    report.status = if total.checks == total.passed { 0 } else { -1 };

    report
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Titik masuk penjaga: menyimpan XMM0..15 + MXCSR + x87 ke `out`.
///
/// Kembalikan `0` bila berhasil, negatif bila gagal-tertutup (salah selaras).
///
/// # Safety
/// `out` harus menunjuk `KlatosXmmGuard` hidup dan selaras 64.
#[no_mangle]
pub unsafe extern "C" fn klatos_fpu_guard_enter(out: *mut KlatosXmmGuard) -> i32 {
    guard_enter_raw(out)
}

/// Titik keluar penjaga: memulihkan XMM0..15 + MXCSR + x87 dari `state`.
///
/// Kembalikan `0` bila berhasil, negatif bila area tidak sah.
///
/// # Safety
/// `state` harus menunjuk `KlatosXmmGuard` hidup dan selaras 64 yang berasal
/// dari [`klatos_fpu_guard_enter`].
#[no_mangle]
pub unsafe extern "C" fn klatos_fpu_guard_exit(state: *const KlatosXmmGuard) -> i32 {
    guard_exit_raw(state)
}

/// Ukuran cermin `Jowo::FPUState` dalam byte (832).
#[no_mangle]
pub extern "C" fn klatos_fpu_state_bytes() -> u32 {
    FPU_STATE_BYTES
}

/// Ukuran area penjaga dalam byte (512).
#[no_mangle]
pub extern "C" fn klatos_fpu_guard_bytes() -> u32 {
    FPU_GUARD_BYTES
}

/// Offset `KlatosXmmGuard.data` dari awal struct (16). Dipakai C++ untuk
/// memverifikasi tata letak cermin, bukan untuk menghitungnya sendiri.
#[no_mangle]
pub extern "C" fn klatos_fpu_guard_data_offset() -> u32 {
    16
}

/// Tag laporan.
#[no_mangle]
pub extern "C" fn klatos_fpu_tag() -> u64 {
    REPORT_TAG
}

/// Ajaib penjaga (dipakai C++ untuk memverifikasi cermin).
#[no_mangle]
pub extern "C" fn klatos_fpu_guard_magic() -> u64 {
    GUARD_MAGIC
}

/// Penanda "kolom ini diisi sisi C++".
#[no_mangle]
pub extern "C" fn klatos_fpu_sentinel() -> u32 {
    SENTINEL_C_FILLS
}

/// Penghitung audit: 0 = jalur produksi, 1 = penolakan, 2 = pelepasan
/// menyimpang, 3 = pemakaian pembersih XMM uji.
#[no_mangle]
pub extern "C" fn klatos_fpu_counter(index: u32) -> u32 {
    match counter_from_index(index as usize) {
        Some(counter) => read_counter(counter),
        None => u32::MAX,
    }
}

/// Reset penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_fpu_reset() -> u32 {
    reset_counters();
    0
}

/// Menulis pola deterministik `seed` ke dalam `out` (256 byte).
///
/// BUKAN jalur uji XMM: ini hanya rumus pola, dipakai C++ sebagai ACUAN.
/// C++ membandingkan pola yang ia tulis ke XMM dengan byte yang dihasilkan di
/// sini, sehingga rumusnya tidak hidup di dua tempat.
///
/// # Safety
/// `out` harus menunjuk setidaknya 256 byte yang dapat ditulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_fpu_pattern_fill(seed: u32, out: *mut u8) {
    let mut buffer = [0u8; PATTERN_BYTES as usize];
    build_pattern(seed, &mut buffer);
    core::ptr::copy_nonoverlapping(buffer.as_ptr(), out, PATTERN_BYTES as usize);
}

/// Menulis SEMUA XMM0..15 dengan `seed` dari sisi Rust.
///
/// Dipakai oleh uji C++ sebagai **kontrol negatif**: kode Rust inilah yang
/// merusak XMM, dan ia nyata — bukan simulasi. Ia sengaja memakai salinan
/// 128-bit (`u128`) sehingga codegen pasti menghasilkan instruksi XMM.
///
/// # Safety
/// Aman dipanggil kapan saja; tidak menyentuh memori pemanggil.
#[no_mangle]
pub unsafe extern "C" fn klatos_fpu_rust_clobber(seed: u32) {
    bump(Counter::Clobber);
    // Pola 256 byte dibangun lebih dulu: pembangunannya sendiri boleh memakai
    // XMM sesuka hati, karena tujuannya memang merusak.
    let mut buffer = [0u8; PATTERN_BYTES as usize];
    build_pattern(seed, &mut buffer);
    // Memuat SELURUH XMM0..15 dari buffer. Ini asm tangan dan memang begitu:
    // ini alat uji, bukan jaminan. Yang dijamin adalah kebalikannya — bahwa
    // kode Rust SANGAT MAMPU menghancurkan XMM pemanggil, sehingga penjaga di
    // sekelilingnya punya sesuatu untuk dibuktikan.
    //
    // `out("xmmN") _` di sini benar dan perlu: blok ini memang MENULIS
    // seluruh register XMM, jadi LLVM harus tahu bahwa nilai sebelumnya mati.
    #[cfg(target_arch = "x86_64")]
    asm!(
        "movups xmm0,  [{p} +   0]",
        "movups xmm1,  [{p} +  16]",
        "movups xmm2,  [{p} +  32]",
        "movups xmm3,  [{p} +  48]",
        "movups xmm4,  [{p} +  64]",
        "movups xmm5,  [{p} +  80]",
        "movups xmm6,  [{p} +  96]",
        "movups xmm7,  [{p} + 112]",
        "movups xmm8,  [{p} + 128]",
        "movups xmm9,  [{p} + 144]",
        "movups xmm10, [{p} + 160]",
        "movups xmm11, [{p} + 176]",
        "movups xmm12, [{p} + 192]",
        "movups xmm13, [{p} + 208]",
        "movups xmm14, [{p} + 224]",
        "movups xmm15, [{p} + 240]",
        p = in(reg) buffer.as_ptr(),
        out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
        out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
        out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
        out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
        options(nostack, preserves_flags)
    );

    // aarch64: memuat SELURUH q0..q31 dari buffer 256 byte (offset diulang tiap
    // 256 byte). Kontrol negatif yang sama: kode Rust inilah yang merusak
    // register SIMD pemanggil, dan ia nyata.
    #[cfg(target_arch = "aarch64")]
    asm!(
        "ldp q0,  q1,  [{p}, #0]",
        "ldp q2,  q3,  [{p}, #32]",
        "ldp q4,  q5,  [{p}, #64]",
        "ldp q6,  q7,  [{p}, #96]",
        "ldp q8,  q9,  [{p}, #128]",
        "ldp q10, q11, [{p}, #160]",
        "ldp q12, q13, [{p}, #192]",
        "ldp q14, q15, [{p}, #224]",
        "ldp q16, q17, [{p}, #0]",
        "ldp q18, q19, [{p}, #32]",
        "ldp q20, q21, [{p}, #64]",
        "ldp q22, q23, [{p}, #96]",
        "ldp q24, q25, [{p}, #128]",
        "ldp q26, q27, [{p}, #160]",
        "ldp q28, q29, [{p}, #192]",
        "ldp q30, q31, [{p}, #224]",
        p = in(reg) buffer.as_ptr(),
        out("q0") _, out("q1") _, out("q2") _, out("q3") _,
        out("q4") _, out("q5") _, out("q6") _, out("q7") _,
        out("q8") _, out("q9") _, out("q10") _, out("q11") _,
        out("q12") _, out("q13") _, out("q14") _, out("q15") _,
        out("q16") _, out("q17") _, out("q18") _, out("q19") _,
        out("q20") _, out("q21") _, out("q22") _, out("q23") _,
        out("q24") _, out("q25") _, out("q26") _, out("q27") _,
        out("q28") _, out("q29") _, out("q30") _, out("q31") _,
        options(nostack, preserves_flags)
    );
}

/// Salinan 256 byte yang dihasilkan COMPILER (bukan asm tangan) dan memakai
/// register XMM; diaudit lewat disassembly arsip.
///
/// # Safety
/// `src` harus menunjuk 256 byte yang dapat dibaca; `dst` 256 byte yang dapat
/// ditulis; kedua area tidak boleh tumpang tindih.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn klatos_fpu_vector_copy(src: *const u8, dst: *mut u8) {
    let source = src as *const u128;
    let destination = dst as *mut u128;
    for index in 0..(PATTERN_BYTES as usize / 16) {
        destination.add(index).write_unaligned(source.add(index).read_unaligned());
    }
}

/// Menjalankan matriks P1.04.
#[no_mangle]
pub extern "C" fn klatos_fpu_selftest() -> KlatosFpuReport {
    run_matrix()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_is_deterministic() {
        let mut a = [0u8; PATTERN_BYTES as usize];
        let mut b = [0u8; PATTERN_BYTES as usize];
        build_pattern(7, &mut a);
        build_pattern(7, &mut b);
        assert_eq!(a, b);
        build_pattern(8, &mut b);
        assert_ne!(a, b);
    }

    #[test]
    fn layout_is_frozen() {
        #[cfg(target_arch = "x86_64")]
        assert_eq!(FPU_STATE_BYTES, 832);
        #[cfg(target_arch = "aarch64")]
        assert_eq!(FPU_STATE_BYTES, 512);
        assert_eq!(FPU_GUARD_BYTES, 512);
        assert_eq!(core::mem::size_of::<KlatosFpuState>(), FPU_STATE_BYTES as usize);
        assert_eq!(core::mem::size_of::<KlatosXmmGuard>(), 576);
        assert_eq!(core::mem::align_of::<KlatosXmmGuard>(), 64);
        assert_eq!(core::mem::size_of::<KlatosFpuReport>(), 72);
    }
}
