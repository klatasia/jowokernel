// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Jalur panic kernel Rust (P0.97.6 — "panic path works").


//! (entry, serial, init) sudah berada di log sebelum CPU berhenti.

use core::panic::PanicInfo;

use crate::{arch, serial};

/// Penanda bahwa `rust_begin_unwind` benar-benar dieksekusi.
pub const PANIC_MARKER: &str = "P097-PANIC-HANDLER";

/// Penanda bahwa handler mencapai titik halt terminal.
pub const PANIC_HALT_MARKER: &str = "P097-PANIC-HALT";

/// Handler panic kernel. Tidak pernah kembali.
#[panic_handler]
fn klatos_kernel_panic(info: &PanicInfo<'_>) -> ! {
    serial::init();
    serial::write_str("\n");
    serial::write_str(PANIC_MARKER);
    serial::write_str(" rust_begin_unwind reached\n");
    serial::write_fmt(core::format_args!("P097-PANIC-INFO {}\n", info));
    serial::write_str(PANIC_HALT_MARKER);
    serial::write_str(" kernel halted by Rust panic handler\n");
    arch::halt()
}

/// Probe jalur panic: memicu panic yang disengaja. Tidak pernah kembali.
pub fn probe() -> ! {
    panic!("P097-PANIC-PROBE deliberate kernel panic from Rust");
}
