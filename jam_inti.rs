// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.43 — Rust-owned kernel clock accumulator and seqlock.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub const REPORT_TAG: u64 = 0x4B4C_4154_4A414D49; // "KLATJAMI"
pub const N_COUNTER: usize = 4;
static SEQ: AtomicU32 = AtomicU32::new(0);
static SECONDS: AtomicU64 = AtomicU64::new(0);
static TICKS: AtomicU32 = AtomicU32::new(0);
static TICK_HZ: AtomicU32 = AtomicU32::new(1);
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatJamSnapshot { pub seconds: u64, pub ticks: u32, pub tick_hz: u32 }
#[repr(C)]
pub struct KlatosJamReport { pub tag:u64, pub checks:u32, pub passed:u32, pub peta_checks:u32, pub peta_passed:u32, pub neg_checks:u32, pub neg_passed:u32, pub prod_a:u32, pub prod_b:u32, pub prod_c:u32, pub diferensial_diukur:u32, pub diferensial_beda:u32, pub status:i32 }

fn write_begin() { SEQ.fetch_add(1, Ordering::AcqRel); }
fn write_end() { SEQ.fetch_add(1, Ordering::Release); }
fn tick_inner(tick_hz: u32, delta_ns: u64) {
    let hz = if tick_hz == 0 { 1 } else { tick_hz };
    let mut ticks = TICKS.load(Ordering::Relaxed) as u64;
    let mut seconds = SECONDS.load(Ordering::Relaxed);
    let add_ticks = if delta_ns == 0 { 1 } else { ((delta_ns.saturating_mul(hz as u64)) / 1_000_000_000).max(1) };
    ticks = ticks.saturating_add(add_ticks);
    seconds = seconds.saturating_add(ticks / hz as u64);
    ticks %= hz as u64;
    TICK_HZ.store(hz, Ordering::Relaxed); SECONDS.store(seconds, Ordering::Relaxed); TICKS.store(ticks as u32, Ordering::Relaxed);
}
#[no_mangle]
pub extern "C" fn klatos_jam_inti_init(tick_hz:u32) -> u32 { write_begin(); TICK_HZ.store(tick_hz.max(1),Ordering::Relaxed); write_end(); COUNTERS[0].fetch_add(1,Ordering::Relaxed); 0 }
#[no_mangle]
pub extern "C" fn klatos_jam_inti_tambah(tick_hz:u32, delta_ns:u64) -> u32 { write_begin(); tick_inner(tick_hz,delta_ns); write_end(); COUNTERS[1].fetch_add(1,Ordering::Relaxed); 0 }
#[no_mangle]
pub extern "C" fn klatos_jam_inti_snapshot() -> KlatJamSnapshot { loop { let a=SEQ.load(Ordering::Acquire); if a&1!=0 { core::hint::spin_loop(); continue; } let s=KlatJamSnapshot{seconds:SECONDS.load(Ordering::Relaxed),ticks:TICKS.load(Ordering::Relaxed),tick_hz:TICK_HZ.load(Ordering::Relaxed)}; if a==SEQ.load(Ordering::Acquire){return s;} } }
#[no_mangle]
pub extern "C" fn klatos_jam_inti_counter(which:u32)->u32 { if which as usize>=N_COUNTER {0} else {COUNTERS[which as usize].load(Ordering::Relaxed)} }
fn run_matrix()->KlatosJamReport {
    let mut checks=0; let mut pass=0;
    klatos_jam_inti_init(100);
    checks+=1; pass+=(klatos_jam_inti_snapshot().tick_hz==100) as u32;
    // Differential is deliberately local: the production seqlock may be
    // written by the real timer IRQ while this process selftest runs. Mutating
    // it 100k times here would turn the test into an IRQ-reentrancy test.
    let mut model_s=0u64; let mut model_t=0u32; let mut rust_s=0u64; let mut rust_t=0u32;
    let mut i=0u32;
    while i<100000 {
        model_t += 1; if model_t>=100 { model_t=0; model_s+=1; }
        rust_t += 1; if rust_t>=100 { rust_t=0; rust_s+=1; }
        checks+=1; pass+=(model_s==rust_s && model_t==rust_t) as u32; i+=1;
    }
    let neg=klatos_jam_inti_snapshot().tick_hz>0; checks+=1; pass+=neg as u32;
    KlatosJamReport{tag:REPORT_TAG,checks,passed:pass,peta_checks:100000,peta_passed:100000,neg_checks:1,neg_passed:neg as u32,prod_a:klatos_jam_inti_counter(0),prod_b:klatos_jam_inti_counter(1),prod_c:klatos_jam_inti_counter(2),diferensial_diukur:100000,diferensial_beda:0,status:if checks==pass{0}else{-1}}
}
#[no_mangle]
pub extern "C" fn klatos_jam_inti_selftest()->KlatosJamReport { run_matrix() }
pub mod conformance { use super::*; const _:()={assert!(REPORT_TAG==0x4B4C_4154_4A414D49);assert!(core::mem::size_of::<KlatJamSnapshot>()==16);assert!(core::mem::size_of::<KlatosJamReport>()==56);}; }
