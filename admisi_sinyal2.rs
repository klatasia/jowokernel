// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.98 / PRIORITAS 6 — otoritas signal, profiling, dan coredump.


//! Memindahkan verdict errno murni; mekanisme FPU, region, buffer, timer, dan
//! serialisasi tetap C++.
#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

pub const EPERM: i32 = 1;
pub const EINTR: i32 = 4;
pub const EAGAIN: i32 = 11;
pub const EFAULT: i32 = 14;
pub const EINVAL: i32 = 22;
pub const ENOMEM: i32 = 12;
pub const ESRCH: i32 = 3;
pub const ENOTSUP: i32 = 74;
pub const REPORT_TAG: u64 = 0x4B4C_4154_5349_4732; // KLATSIG2

#[derive(Clone, Copy)]
pub enum Counter {
    SigprocmaskHow = 0, RemapEmpty = 1, RemapUser = 2, RemapMmap = 3,
    RemapObject = 4, RemapNoRegion = 5, AltActive = 6, AltSize = 7,
    AltOverflow = 8, AltFlags = 9, WaitInterrupted = 10, WaitTimeout = 11,
    SuspendInterrupted = 12, ProfileSuperuser = 13, ProfileProcess = 14,
    ProfileOwner = 15, ProfileBuffer = 16, ProfileTimer = 17,
    ProfileState = 18, CoreDumpable = 19, CoreDirectory = 20,
    CoreRegion = 21, CoreConsistent = 22,
}
const N_COUNTER: usize = 23;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(c: Counter) { COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed); }

#[inline] pub const fn sinyal2_sigprocmask_how(sah: u32) -> i32 { if sah != 0 { -EINVAL } else { 0 } }
#[inline] pub const fn sinyal2_remap_empty(kosong: u32) -> i32 { if kosong != 0 { -EINVAL } else { 0 } }
#[inline] pub const fn sinyal2_remap_user(sah: u32) -> i32 { if sah == 0 { -EFAULT } else { 0 } }
#[inline] pub const fn sinyal2_remap_mmap(mmap: u32) -> i32 { if mmap == 0 { -EPERM } else { 0 } }
#[inline] pub const fn sinyal2_remap_object(sah: u32) -> i32 { if sah == 0 { -EINVAL } else { 0 } }
#[inline] pub const fn sinyal2_remap_no_region(ada: u32) -> i32 { if ada == 0 { -EINVAL } else { 0 } }
#[inline] pub const fn sinyal2_alt_active(aktif: u32) -> i32 { if aktif != 0 { -EPERM } else { 0 } }
#[inline] pub const fn sinyal2_alt_size(kecil: u32) -> i32 { if kecil != 0 { -ENOMEM } else { 0 } }
#[inline] pub const fn sinyal2_alt_overflow(meluap: u32) -> i32 { if meluap != 0 { -ENOMEM } else { 0 } }
#[inline] pub const fn sinyal2_alt_flags(sah: u32) -> i32 { if sah == 0 { -EINVAL } else { 0 } }
#[inline] pub const fn sinyal2_wait_interrupted(interupsi: u32) -> i32 { if interupsi != 0 { -EINTR } else { 0 } }
#[inline] pub const fn sinyal2_wait_timeout(timeout: u32) -> i32 { if timeout != 0 { -EAGAIN } else { 0 } }
#[inline] pub const fn sinyal2_suspend_interrupted(interupsi: u32) -> i32 { if interupsi != 0 { -EINTR } else { 0 } }
#[inline] pub const fn sinyal2_profile_superuser(izin: u32) -> i32 { if izin == 0 { -EPERM } else { 0 } }
#[inline] pub const fn sinyal2_profile_process(ada: u32) -> i32 { if ada == 0 { -ESRCH } else { 0 } }
#[inline] pub const fn sinyal2_profile_owner(izin: u32) -> i32 { if izin == 0 { -EPERM } else { 0 } }
#[inline] pub const fn sinyal2_profile_buffer(ada: u32) -> i32 { if ada == 0 { -ENOMEM } else { 0 } }
#[inline] pub const fn sinyal2_profile_timer(ada: u32) -> i32 { if ada == 0 { -ENOTSUP } else { 0 } }
#[inline] pub const fn sinyal2_profile_state_aktif(aktif: u32) -> i32 { if aktif != 0 { -EINVAL } else { 0 } }
#[inline] pub const fn sinyal2_core_dumpable(izin: u32) -> i32 { if izin == 0 { -EPERM } else { 0 } }
#[inline] pub const fn sinyal2_core_directory(sah: u32) -> i32 { if sah == 0 { -EINVAL } else { 0 } }
#[inline] pub const fn sinyal2_core_region(ada: u32) -> i32 { if ada == 0 { -EFAULT } else { 0 } }
#[inline] pub const fn sinyal2_core_consistent(sah: u32) -> i32 { if sah == 0 { -EINVAL } else { 0 } }

macro_rules! abi1 { ($name:ident, $fun:ident, $counter:ident, $arg:ident : $ty:ty) => {
    #[no_mangle] pub extern "C" fn $name($arg: $ty) -> i32 { bump(Counter::$counter); $fun($arg) }
}; }
abi1!(klatos_sinyal2_sigprocmask_how, sinyal2_sigprocmask_how, SigprocmaskHow, sah: u32);
abi1!(klatos_sinyal2_remap_empty, sinyal2_remap_empty, RemapEmpty, kosong: u32);
abi1!(klatos_sinyal2_remap_user, sinyal2_remap_user, RemapUser, sah: u32);
abi1!(klatos_sinyal2_remap_mmap, sinyal2_remap_mmap, RemapMmap, mmap: u32);
abi1!(klatos_sinyal2_remap_object, sinyal2_remap_object, RemapObject, sah: u32);
abi1!(klatos_sinyal2_remap_no_region, sinyal2_remap_no_region, RemapNoRegion, ada: u32);
abi1!(klatos_sinyal2_alt_active, sinyal2_alt_active, AltActive, aktif: u32);
abi1!(klatos_sinyal2_alt_size, sinyal2_alt_size, AltSize, kecil: u32);
abi1!(klatos_sinyal2_alt_overflow, sinyal2_alt_overflow, AltOverflow, meluap: u32);
abi1!(klatos_sinyal2_alt_flags, sinyal2_alt_flags, AltFlags, sah: u32);
abi1!(klatos_sinyal2_wait_interrupted, sinyal2_wait_interrupted, WaitInterrupted, interupsi: u32);
abi1!(klatos_sinyal2_wait_timeout, sinyal2_wait_timeout, WaitTimeout, timeout: u32);
abi1!(klatos_sinyal2_suspend_interrupted, sinyal2_suspend_interrupted, SuspendInterrupted, interupsi: u32);
abi1!(klatos_sinyal2_profile_superuser, sinyal2_profile_superuser, ProfileSuperuser, izin: u32);
abi1!(klatos_sinyal2_profile_process, sinyal2_profile_process, ProfileProcess, ada: u32);
abi1!(klatos_sinyal2_profile_owner, sinyal2_profile_owner, ProfileOwner, izin: u32);
abi1!(klatos_sinyal2_profile_buffer, sinyal2_profile_buffer, ProfileBuffer, ada: u32);
abi1!(klatos_sinyal2_profile_timer, sinyal2_profile_timer, ProfileTimer, ada: u32);
abi1!(klatos_sinyal2_profile_state, sinyal2_profile_state_aktif, ProfileState, aktif: u32);
abi1!(klatos_sinyal2_core_dumpable, sinyal2_core_dumpable, CoreDumpable, izin: u32);
abi1!(klatos_sinyal2_core_directory, sinyal2_core_directory, CoreDirectory, sah: u32);
abi1!(klatos_sinyal2_core_region, sinyal2_core_region, CoreRegion, ada: u32);
abi1!(klatos_sinyal2_core_consistent, sinyal2_core_consistent, CoreConsistent, sah: u32);

#[no_mangle] pub extern "C" fn klatos_sinyal2_counter(which: u32) -> u32 { if (which as usize) < N_COUNTER { COUNTERS[which as usize].load(Ordering::Relaxed) } else { 0 } }
#[no_mangle] pub extern "C" fn klatos_sinyal2_reset() -> u32 { for c in COUNTERS.iter() { c.store(0, Ordering::Release); } N_COUNTER as u32 }

#[repr(C)]
pub struct KlatosAdmisiSinyal2Report {
    pub tag: u64, pub checks: u32, pub passed: u32, pub teras_checks: u32, pub teras_passed: u32,
    pub sub_checks: u32, pub sub_passed: u32, pub counters: [u32; N_COUNTER],
    pub diferensial_diukur: u32, pub diferensial_beda: u32, pub status: i32,
}

fn check(ok: bool, c: &mut u32, p: &mut u32) { *c += 1; if ok { *p += 1; } }
fn run_matrix() -> KlatosAdmisiSinyal2Report {
    let mut checks=0; let mut passed=0; let mut teras_checks=0; let mut teras_passed=0; let mut sub_checks=0; let mut sub_passed=0;
    macro_rules! t { ($x:expr) => {{ check($x, &mut teras_checks, &mut teras_passed); checks+=1; if $x { passed+=1; } }}; }
    macro_rules! s { ($x:expr) => {{ check($x, &mut sub_checks, &mut sub_passed); checks+=1; if $x { passed+=1; } }}; }
    t!(REPORT_TAG == 0x4B4C_4154_5349_4732); t!(sinyal2_sigprocmask_how(1)==-EINVAL);
    t!(sinyal2_remap_empty(1)==-EINVAL); t!(sinyal2_remap_user(0)==-EFAULT); t!(sinyal2_remap_mmap(0)==-EPERM);
    t!(sinyal2_remap_object(0)==-EINVAL); t!(sinyal2_remap_no_region(0)==-EINVAL); t!(sinyal2_alt_active(1)==-EPERM);
    t!(sinyal2_alt_size(1)==-ENOMEM); t!(sinyal2_alt_overflow(1)==-ENOMEM); t!(sinyal2_alt_flags(0)==-EINVAL);
    s!(sinyal2_wait_interrupted(1)==-EINTR); s!(sinyal2_wait_timeout(1)==-EAGAIN); s!(sinyal2_suspend_interrupted(1)==-EINTR);
    s!(sinyal2_profile_superuser(0)==-EPERM); s!(sinyal2_profile_process(0)==-ESRCH); s!(sinyal2_profile_owner(0)==-EPERM);
    s!(sinyal2_profile_buffer(0)==-ENOMEM); s!(sinyal2_profile_timer(0)==-ENOTSUP); s!(sinyal2_profile_state_aktif(1)==-EINVAL);
    s!(sinyal2_core_dumpable(0)==-EPERM); s!(sinyal2_core_directory(0)==-EINVAL); s!(sinyal2_core_region(0)==-EFAULT); s!(sinyal2_core_consistent(0)==-EINVAL);
    let mut beda=0; let mut diukur=0;
    macro_rules! d { ($p:expr, $n:expr) => {{ diukur+=1; if $p != $n { beda+=1; } }}; }
    for v in 0..2 { d!(sinyal2_sigprocmask_how(v), if v!=0 {-EINVAL} else {0}); d!(sinyal2_remap_empty(v), if v!=0 {-EINVAL} else {0}); d!(sinyal2_remap_user(v), if v==0 {-EFAULT} else {0}); d!(sinyal2_remap_mmap(v), if v==0 {-EPERM} else {0}); d!(sinyal2_remap_object(v), if v==0 {-EINVAL} else {0}); d!(sinyal2_remap_no_region(v), if v==0 {-EINVAL} else {0}); d!(sinyal2_alt_active(v), if v!=0 {-EPERM} else {0}); d!(sinyal2_alt_size(v), if v!=0 {-ENOMEM} else {0}); d!(sinyal2_alt_overflow(v), if v!=0 {-ENOMEM} else {0}); d!(sinyal2_alt_flags(v), if v==0 {-EINVAL} else {0}); d!(sinyal2_wait_interrupted(v), if v!=0 {-EINTR} else {0}); d!(sinyal2_wait_timeout(v), if v!=0 {-EAGAIN} else {0}); d!(sinyal2_suspend_interrupted(v), if v!=0 {-EINTR} else {0}); d!(sinyal2_profile_superuser(v), if v==0 {-EPERM} else {0}); d!(sinyal2_profile_process(v), if v==0 {-ESRCH} else {0}); d!(sinyal2_profile_owner(v), if v==0 {-EPERM} else {0}); d!(sinyal2_profile_buffer(v), if v==0 {-ENOMEM} else {0}); d!(sinyal2_profile_timer(v), if v==0 {-ENOTSUP} else {0}); d!(sinyal2_profile_state_aktif(v), if v!=0 {-EINVAL} else {0}); d!(sinyal2_core_dumpable(v), if v==0 {-EPERM} else {0}); d!(sinyal2_core_directory(v), if v==0 {-EINVAL} else {0}); d!(sinyal2_core_region(v), if v==0 {-EFAULT} else {0}); d!(sinyal2_core_consistent(v), if v==0 {-EINVAL} else {0}); }
    check(diukur==46, &mut teras_checks, &mut teras_passed); checks+=1; if diukur==46 {passed+=1;} check(beda==0,&mut teras_checks,&mut teras_passed); checks+=1;if beda==0{passed+=1;}
    let mut counters=[0u32;N_COUNTER]; for i in 0..N_COUNTER { counters[i]=klatos_sinyal2_counter(i as u32); }
    KlatosAdmisiSinyal2Report{tag:REPORT_TAG,checks,passed,teras_checks,teras_passed,sub_checks,sub_passed,counters,diferensial_diukur:diukur,diferensial_beda:beda,status:if checks==passed{0}else{-1}}
}
#[no_mangle] pub extern "C" fn klatos_sinyal2_selftest() -> KlatosAdmisiSinyal2Report { run_matrix() }

pub mod conformance { use super::*; const _: () = { assert!(REPORT_TAG==0x4B4C_4154_5349_4732); assert!(N_COUNTER==23); assert!(core::mem::size_of::<KlatosAdmisiSinyal2Report>()==136); }; }
