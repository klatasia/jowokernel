// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Arsitektur (x86_64 + aarch64) — **satu-satunya** modul di crate ini yang memuat blok


#![allow(unsafe_code)]

use core::arch::asm;

/// Port I/O COM1 (16550 UART). Sama dengan
/// `KlatosKernel/Efek/Arsitektur/x86_64/DebugOutput.cpp` (`serial_com1_io_port`).
///
/// Hanya ada pada x86_64: aarch64 tidak memiliki ruang port I/O (I/O-nya
/// MMIO). Debug serial aarch64 lewat PL011 — lihat [`crate::serial`].
#[cfg(target_arch = "x86_64")]
pub const COM1_PORT: u16 = 0x3F8;

/// Port debug Bochs/QEMU. Sama dengan `Efek/Arsitektur/x86_64/BochsDebugOutput.h`.
#[cfg(target_arch = "x86_64")]
pub const BOCHS_DEBUG_PORT: u16 = 0xE9;

/// Nilai `KlatosKernelBootInfo::arch_id` untuk x86_64.
pub const ARCH_ID_X86_64: u32 = 1;

/// Nilai `KlatosKernelBootInfo::arch_id` untuk aarch64.
pub const ARCH_ID_AARCH64: u32 = 2;

/// Nilai `KlatosKernelBootInfo::arch_id` untuk arsitektur build ini.
#[cfg(target_arch = "x86_64")]
pub const ARCH_ID: u32 = ARCH_ID_X86_64;
/// Lihat [`ARCH_ID`].
#[cfg(target_arch = "aarch64")]
pub const ARCH_ID: u32 = ARCH_ID_AARCH64;

/// Nama arsitektur, dipakai di jejak serial.
#[cfg(target_arch = "x86_64")]
pub const ARCH_NAME: &str = "x86_64";
/// Nama arsitektur, dipakai di jejak serial.
#[cfg(target_arch = "aarch64")]
pub const ARCH_NAME: &str = "aarch64";

/// Lebar penunjuk dalam bit.
pub const POINTER_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

/// Bukti waktu-kompilasi bahwa crate ini dibangun untuk target 64-bit.
const _: () = assert!(POINTER_BITS == 64);

/// Menulis satu byte ke port I/O (`out dx, al`).
///
/// Hanya x86_64. aarch64 tidak punya instruksi port I/O.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn out8(port: u16, value: u8) {
    // SAFETY: `out` hanya menyentuh ruang port I/O, bukan memori, dan legal
    // pada CPL0 (kernel berjalan di CPL0). Port adalah argumen eksplisit.
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        )
    }
}

/// Membaca satu byte dari port I/O (`in al, dx`).
///
/// Hanya x86_64. aarch64 tidak punya instruksi port I/O.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn in8(port: u16) -> u8 {
    let value: u8;
    // SAFETY: lihat `out8`; `in` hanya membaca satu byte dari port dan tidak
    // menyentuh memori.
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        )
    }
    value
}

/// Mematikan interupsi pada CPU ini (`cli`).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn disable_interrupts() {
    // SAFETY: `cli` hanya membersihkan bit IF di RFLAGS dan selalu legal pada
    // CPL0. Tidak menyentuh memori.
    unsafe { asm!("cli", options(nomem, nostack)) }
}

/// Mematikan interupsi IRQ pada CPU ini (`msr daifset, #2`).
///
/// Bit I dari `PSTATE.DAIF` adalah topeng IRQ; `daifset` menyalakannya.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn disable_interrupts() {
    // SAFETY: `msr daifset, #2` hanya mengubah bit I dari PSTATE.DAIF dan legal
    // pada EL1/EL0. Tidak menyentuh memori.
    unsafe { asm!("msr daifset, #2", options(nomem, nostack)) }
}

/// Menyalakan interupsi pada CPU ini (`sti`).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn enable_interrupts() {
    // SAFETY: `sti` hanya menyetel bit IF. Pemanggil WAJIB memastikan IDT sudah
    // siap; satu-satunya pemanggil di crate ini ([`crate::interrupt::probe`])
    // hanya memanggilnya bila interupsi memang sudah menyala sebelum probe.
    unsafe { asm!("sti", options(nomem, nostack)) }
}

/// Menyalakan interupsi IRQ pada CPU ini (`msr daifclr, #2`).
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn enable_interrupts() {
    // SAFETY: `msr daifclr, #2` hanya membersihkan bit I dari PSTATE.DAIF.
    // Pemanggil WAJIB memastikan vektor/IRQ controller sudah siap; pemanggil di
    // crate ini ([`crate::interrupt::probe`]) hanya menyalakannya bila memang
    // sudah menyala sebelum probe.
    unsafe { asm!("msr daifclr, #2", options(nomem, nostack)) }
}

/// Membaca bit IF (interrupt flag) dari RFLAGS.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn interrupts_enabled() -> bool {
    let flags: u64;
    // SAFETY: `pushfq`/`pop` seimbang (RSP kembali ke nilai semula) dan hanya
    // membaca RFLAGS. Stack kernel valid pada seluruh titik pemanggilan.
    // `preserves_flags` benar: baik `pushfq` maupun `pop` tidak mengubah flag.
    unsafe { asm!("pushfq; pop {}", out(reg) flags, options(preserves_flags)) }
    flags & (1u64 << 9) != 0
}

/// Membaca bit I (IRQ mask) dari `PSTATE.DAIF`; interupsi AKTIF bila bit == 0.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn interrupts_enabled() -> bool {
    let daif: u64;
    // SAFETY: `mrs` hanya membaca state sistem; tidak menyentuh memori.
    unsafe { asm!("mrs {}, daif", out(reg) daif, options(nomem, nostack)) }
    // Bit I dari DAIF ada pada bit 7 (0x80). 0 = IRQ tidak dimask.
    daif & (1u64 << 7) == 0
}

/// Simpan status interupsi CPU ini, lalu matikan. Kembalikan status sebelumnya.
///
/// Spinlock kernel Rust WAJIB memakai ini di sekitar bagian kritis. Tanpa
/// mematikan interupsi, pemegang kunci dapat di-preempt atau dimasuki ulang dari
/// handler interupsi; penunggu yang berputar dengan IF=0 lalu tidak akan pernah
/// melihat kunci dilepas (P0-1: deadlock di klatos_keadaan_proses_ambil_bendera).
///
/// Nilai kembalian wajib diteruskan ke pulihkan_interupsi(); jangan pernah
/// menyalakan interupsi tanpa syarat, karena pemanggil bisa saja sudah memasukinya
/// dengan interupsi mati.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn simpan_dan_matikan_interupsi() -> u64 {
    let flags: u64;
    // SAFETY: pushfq/pop seimbang (RSP kembali ke nilai semula) dan hanya
    // membaca RFLAGS. cli hanya membersihkan bit IF dan legal pada CPL0.
    unsafe { asm!("pushfq; pop {}", out(reg) flags, options(preserves_flags)) };
    unsafe { asm!("cli", options(nomem, nostack)) };
    flags
}

/// Lihat simpan_dan_matikan_interupsi untuk versi x86_64.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn simpan_dan_matikan_interupsi() -> u64 {
    let daif: u64;
    // SAFETY: mrs hanya membaca state sistem; msr daifset, #2 hanya menyetel
    // bit I dari PSTATE.DAIF. Keduanya legal pada EL1.
    unsafe { asm!("mrs {}, daif", out(reg) daif, options(nomem, nostack)) };
    unsafe { asm!("msr daifset, #2", options(nomem, nostack)) };
    daif
}

/// Pulihkan status interupsi yang dikembalikan oleh
/// simpan_dan_matikan_interupsi.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn pulihkan_interupsi(flags: u64) {
    // Hanya nyalakan bila sebelumnya memang menyala.
    if flags & (1u64 << 9) != 0 {
        // SAFETY: bit IF pada RFLAGS sebelumnya sudah menyala, jadi sti
        // mengembalikan keadaan semula, bukan menyalakan yang tadinya mati.
        unsafe { asm!("sti", options(nomem, nostack)) };
    }
}

/// Lihat pulihkan_interupsi untuk versi x86_64.
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn pulihkan_interupsi(daif: u64) {
    // Bit I bernilai 0 berarti IRQ tidak dimask sebelumnya.
    if daif & (1u64 << 7) == 0 {
        // SAFETY: IRQ sebelumnya tidak dimask, jadi daifclr mengembalikan
        // keadaan semula.
        unsafe { asm!("msr daifclr, #2", options(nomem, nostack)) };
    }
}

/// Menghentikan CPU ini sampai interupsi berikutnya (`hlt`).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn wait_for_interrupt() {
    // SAFETY: `hlt` legal pada CPL0. Bila interupsi dimatikan, CPU berhenti
    // sampai reset — yang justru merupakan perilaku terminal yang diinginkan
    // pada jalur halt/panic.
    unsafe { asm!("hlt", options(nomem, nostack)) }
}

/// Menghentikan CPU ini sampai interupsi berikutnya (`wfi`).
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn wait_for_interrupt() {
    // SAFETY: `wfi` legal pada EL1. Bila IRQ dimask, CPU tetap bangun untuk
    // event lain; pada jalur panic ia tidak berpengaruh (loop akan mengulang).
    unsafe { asm!("wfi", options(nomem, nostack)) }
}

/// Halt terminal: menghentikan CPU ini selamanya.
///
/// Dipakai jalur panic dan jalur "safe halt" prototipe boot (P0.97.3).
#[inline]
pub fn halt() -> ! {
    loop {
        wait_for_interrupt();
    }
}

/// Berputar selamanya tanpa menyentuh perangkat keras.
///
/// Berbeda dari [`halt`], ini tidak memerlukan interupsi untuk bangun dan tidak
/// menghentikan vCPU; dipakai bila hanya "berhenti di sini" yang dibutuhkan.
#[inline]
pub fn spin_forever() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
