// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Serial debug dari Rust (P0.97.3 — "serial output").


// aarch64 PL011 memakai MMIO mentah (satu-satunya `unsafe` di modul ini, sejajar
// dengan pola isolasi [`crate::arch`]).
#![cfg_attr(target_arch = "aarch64", allow(unsafe_code))]

use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_arch = "x86_64")]
use crate::arch;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// x86_64 — COM1 port I/O
// ---------------------------------------------------------------------------

/// Menginisialisasi COM1. Idempoten.
///
/// Aman dipanggil berkali-kali: percobaan kedua dan seterusnya kembali segera
/// tanpa menulis ulang register UART.
#[cfg(target_arch = "x86_64")]
pub fn init() {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }
    arch::out8(arch::COM1_PORT + 1, 0x00);
    arch::out8(arch::COM1_PORT + 3, 0x80);
    arch::out8(arch::COM1_PORT + 0, 0x02);
    arch::out8(arch::COM1_PORT + 1, 0x00);
    arch::out8(arch::COM1_PORT + 3, 0x03);
    arch::out8(arch::COM1_PORT + 2, 0xC7);
    arch::out8(arch::COM1_PORT + 4, 0x0B);
}

/// Menulis satu byte, menunggu bit THR-empty (LSR bit 5).
#[cfg(target_arch = "x86_64")]
pub fn write_byte(byte: u8) {
    while (arch::in8(arch::COM1_PORT + 5) & 0x20) == 0 {
        core::hint::spin_loop();
    }
    arch::out8(arch::COM1_PORT, byte);
}

// ---------------------------------------------------------------------------
// aarch64 — PL011 MMIO
// ---------------------------------------------------------------------------

/// Offset register PL011 (PrimeCell UART).
#[cfg(target_arch = "aarch64")]
mod pl011 {
    /// Data register.
    pub const DR: usize = 0x00;
    /// Flag register (bit 5 = TXFF, TX FIFO penuh).
    pub const FR: usize = 0x18;
    /// Interrupt clear register.
    pub const ICR: usize = 0x44;
    /// Control register (UARTEN = 0x1, TXE = 0x100, RXE = 0x200).
    pub const CR: usize = 0x30;
    /// TX FIFO penuh.
    pub const FR_TXFF: u32 = 1 << 5;
    /// UARTEN | TXE | RXE.
    pub const CR_ENABLE: u32 = 0x1 | 0x100 | 0x200;
}

/// Basis MMIO PL011 untuk build aarch64. Bawaan = basis QEMU `virt`
/// (`Efek/Arsitektur/aarch64/PlatformInit/Virt.cpp`). Platform lain (mis.
/// RaspberryPi) dapat menyetelnya lewat [`set_pl011_base`] sebelum boot Rust.
#[cfg(target_arch = "aarch64")]
pub static PL011_BASE: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0x0900_0000);

/// Menyetel basis MMIO PL011 (aarch64). Dipakai platform non-`virt`.
#[cfg(target_arch = "aarch64")]
pub fn set_pl011_base(base: u64) {
    PL011_BASE.store(base, Ordering::SeqCst);
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn pl011_read(base: u64, offset: usize) -> u32 {
    core::ptr::read_volatile((base as usize + offset) as *const u32)
}

#[cfg(target_arch = "aarch64")]
#[inline]
unsafe fn pl011_write(base: u64, offset: usize, value: u32) {
    core::ptr::write_volatile((base as usize + offset) as *mut u32, value);
}

/// Menginisialisasi PL011. Idempoten.
///
/// Sengaja TIDAK menyetel baud (IBRD/FBRD): setelan itu milik platform C++ yang
/// tahu frekuensi clock UART sebenarnya. Yang dilakukan di sini hanya
/// membersihkan interupsi dan mengaktifkan UARTEN/TXE/RXE.
#[cfg(target_arch = "aarch64")]
pub fn init() {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }
    let base = PL011_BASE.load(Ordering::SeqCst);
    // SAFETY: basis PL011 adalah MMIO sah pada platform aarch64; offset register
    // bernilai tetap dan akses 32-bit selaras.
    unsafe {
        pl011_write(base, pl011::ICR, 0x7FF);
        let cr = pl011_read(base, pl011::CR);
        pl011_write(base, pl011::CR, cr | pl011::CR_ENABLE);
    }
}

/// Menulis satu byte, menunggu TX FIFO tidak penuh (FR.TXFF == 0).
#[cfg(target_arch = "aarch64")]
pub fn write_byte(byte: u8) {
    let base = PL011_BASE.load(Ordering::SeqCst);
    // SAFETY: lihat `init`; menunggu TXFF lalu menulis DR.
    unsafe {
        while (pl011_read(base, pl011::FR) & pl011::FR_TXFF) != 0 {
            core::hint::spin_loop();
        }
        pl011_write(base, pl011::DR, byte as u32);
    }
}

// ---------------------------------------------------------------------------
// Bersama
// ---------------------------------------------------------------------------

/// Apakah [`init`] sudah dijalankan.
pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::SeqCst)
}

/// Menulis teks, menerjemahkan `\n` menjadi `\r\n` seperti sisi C++.
pub fn write_str(text: &str) {
    let mut previous_was_cr = false;
    for &byte in text.as_bytes() {
        if byte == b'\n' && !previous_was_cr {
            write_byte(b'\r');
        }
        write_byte(byte);
        previous_was_cr = byte == b'\r';
    }
}

/// Penulis `core::fmt` di atas UART. Tanpa alokasi.
pub struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        write_str(text);
        Ok(())
    }
}

/// Pintu masuk `core::fmt` — dipakai [`crate::kprint`] / [`crate::kprintln`].
pub fn write_fmt(args: fmt::Arguments<'_>) {
    use fmt::Write;
    let _ = SerialWriter.write_fmt(args);
}
