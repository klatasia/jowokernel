// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Batas IPC (P0.97.1 `ipc`) — mekanisme dispatch ABI v2, **tidak berubah**.


pub use crate::dispatch::{DISPATCH_ENTRY_COUNT_EXPECTED, DISPATCH_TABLE};
pub use crate::ffi::{
    KlatosAbi2CapabilityOut, KlatosAbi2ObjectOut, KlatosAbi2Resolvers, ResolveCapabilityFn,
    ResolveObjectFn,
};

/// Jumlah entri tabel dispatch yang tertanam di kernel.
pub fn table_entries() -> u32 {
    DISPATCH_TABLE.len() as u32
}
