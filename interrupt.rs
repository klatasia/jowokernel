// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Fondasi interupsi (P0.97.1 `interrupt`; P0.97.6 "interrupt foundation works


use crate::arch;

/// Jumlah vektor IDT pada x86_64.
pub const IDT_VECTORS: u32 = 256;

/// Jumlah jalur IRQ pada 8259 PIC klasik (master + slave).
pub const PIC_IRQ_LINES: u32 = 16;

/// Hasil probe fondasi interupsi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterruptFoundation {
    /// Jumlah vektor yang dimodelkan.
    pub vectors: u32,
    /// Jumlah jalur IRQ yang dimodelkan.
    pub irq_lines: u32,
    /// Keadaan bit IF **sebelum** probe.
    pub enabled_before: bool,
    /// `true` bila `cli` terbukti membersihkan IF.
    pub disable_verified: bool,
    /// `true` bila probe mencoba menyalakan interupsi (hanya bila aman).
    pub enable_attempted: bool,
    /// `true` bila `sti` terbukti menyetel IF.
    pub enable_verified: bool,
}

impl InterruptFoundation {
    /// Lulus bila arah `cli` terbukti, dan bila arah `sti` yang *dicoba* juga
    /// terbukti. Bila `sti` tidak dicoba (karena tidak aman), itu bukan
    /// kegagalan.
    pub const fn ok(&self) -> bool {
        self.disable_verified && self.enable_verified == self.enable_attempted
    }
}

/// Menjalankan probe fondasi interupsi dan mengembalikan keadaan awal CPU.
pub fn probe() -> InterruptFoundation {
    let enabled_before = arch::interrupts_enabled();

    arch::disable_interrupts();
    let disable_verified = !arch::interrupts_enabled();

    let mut enable_attempted = false;
    let mut enable_verified = false;
    if enabled_before {
        // SAFETY dari sudut pandang arsitektur: interupsi sudah menyala, jadi
        // IDT pasti sudah siap. Probe hanya mengembalikan keadaan semula.
        enable_attempted = true;
        arch::enable_interrupts();
        enable_verified = arch::interrupts_enabled();
    }

    // Kembalikan keadaan awal CPU tanpa syarat.
    if !enabled_before {
        arch::disable_interrupts();
    }

    InterruptFoundation {
        vectors: IDT_VECTORS,
        irq_lines: PIC_IRQ_LINES,
        enabled_before,
        disable_verified,
        enable_attempted,
        enable_verified,
    }
}
