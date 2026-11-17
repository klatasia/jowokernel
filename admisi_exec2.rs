// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT


//! P2.06 — `execve` admission batch 2 (Explore batch 5a).


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

pub const EPERM: i32 = 1;
pub const EIO: i32 = 5;
pub const E2BIG: i32 = 7;
pub const ENOEXEC: i32 = 8;
pub const EINVAL: i32 = 22;
pub const ETXTBSY: i32 = 26;
pub const ELOOP: i32 = 35;
pub const EOVERFLOW: i32 = 36;

pub const REPORT_TAG: u64 = 0x4B4C_4154_4558_4532; // KLATEXE2

#[derive(Clone, Copy)]
pub enum Counter {
    CitraTakValid = 0,
    TakAdaMuat = 1,
    JenisSalah = 2,
    PetaTulis = 3,
    BukanElf = 4,
    JailSuid = 5,
    Luber = 6,
    HeaderPendek = 7,
    BacaSingkat = 8,
    InterpBerlapis = 9,
    RuangMuatSempit = 10,
}

const N_COUNTER: usize = 11;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

/// `get_required_load_range`: `!elf_image.is_valid()`.
#[inline]
pub const fn citra_tak_valid(tak_valid: u32) -> i32 {
    if tak_valid != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `get_required_load_range`: a `PT_LOAD`-less image (`range.start == range.end`).
#[inline]
pub const fn tak_ada_muat(kosong: u32) -> i32 {
    if kosong != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `get_load_offset`: a static image whose `e_type` is neither `ET_DYN` nor
/// `ET_EXEC`.
#[inline]
pub const fn jenis_salah(salah: u32) -> i32 {
    if salah != 0 {
        -EINVAL
    } else {
        0
    }
}

/// `do_exec`: the program's `SharedInodeVMObject` has writable mappings.
#[inline]
pub const fn peta_tulis(ada: u32) -> i32 {
    if ada != 0 {
        -ETXTBSY
    } else {
        0
    }
}

/// Invalid ELF image / entry point / program headers / machine / interpreter
/// header.
#[inline]
pub const fn bukan_elf(bukan: u32) -> i32 {
    if bukan != 0 {
        -ENOEXEC
    } else {
        0
    }
}

/// `do_exec`: a jailed process may not exec a SUID/SGID binary.
#[inline]
pub const fn jail_suid(jail_suid: u32) -> i32 {
    if jail_suid != 0 {
        -EPERM
    } else {
        0
    }
}

/// `Checked<size_t>` overflow in program-header arithmetic or in
/// `copy_user_strings`.
#[inline]
pub const fn luber(luber: u32) -> i32 {
    if luber != 0 {
        -EOVERFLOW
    } else {
        0
    }
}

/// The program-header list ends before the ELF header itself.
#[inline]
pub const fn header_pendek(pendek: u32) -> i32 {
    if pendek != 0 {
        -EINVAL
    } else {
        0
    }
}

/// A short read of the program-header list or of the interpreter path.
#[inline]
pub const fn baca_singkat(singkat: u32) -> i32 {
    if singkat != 0 {
        -EIO
    } else {
        0
    }
}

/// The interpreter itself carries an INTERP header.
#[inline]
pub const fn interp_berlapis(berlapis: u32) -> i32 {
    if berlapis != 0 {
        -ELOOP
    } else {
        0
    }
}

/// `get_load_offset`: the main program is too large to leave enough room for
/// adequate loader randomization.
#[inline]
pub const fn ruang_muat_sempit(sempit: u32) -> i32 {
    if sempit != 0 {
        -E2BIG
    } else {
        0
    }
}

macro_rules! abi1 {
    ($n:ident, $f:ident, $c:ident) => {
        #[no_mangle]
        pub extern "C" fn $n(v: u32) -> i32 {
            bump(Counter::$c);
            $f(v)
        }
    };
}

abi1!(klatos_exec2_citra_tak_valid, citra_tak_valid, CitraTakValid);
abi1!(klatos_exec2_tak_ada_muat, tak_ada_muat, TakAdaMuat);
abi1!(klatos_exec2_jenis_salah, jenis_salah, JenisSalah);
abi1!(klatos_exec2_peta_tulis, peta_tulis, PetaTulis);
abi1!(klatos_exec2_bukan_elf, bukan_elf, BukanElf);
abi1!(klatos_exec2_jail_suid, jail_suid, JailSuid);
abi1!(klatos_exec2_luber, luber, Luber);
abi1!(klatos_exec2_header_pendek, header_pendek, HeaderPendek);
abi1!(klatos_exec2_baca_singkat, baca_singkat, BacaSingkat);
abi1!(klatos_exec2_interp_berlapis, interp_berlapis, InterpBerlapis);
abi1!(klatos_exec2_ruang_muat_sempit, ruang_muat_sempit, RuangMuatSempit);

#[no_mangle]
pub extern "C" fn klatos_exec2_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_exec2_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

#[repr(C)]
pub struct KlatosAdmisiExec2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosAdmisiExec2Report {
    let mut c = 0;
    let mut p = 0;
    macro_rules! x {
        ($e:expr) => {{
            c += 1;
            if $e {
                p += 1;
            }
        }};
    }
    x!(REPORT_TAG == 0x4B4C_4154_4558_4532);
    x!(citra_tak_valid(1) == -EINVAL && citra_tak_valid(0) == 0);
    x!(tak_ada_muat(1) == -EINVAL && tak_ada_muat(0) == 0);
    x!(jenis_salah(1) == -EINVAL && jenis_salah(0) == 0);
    x!(peta_tulis(1) == -ETXTBSY && peta_tulis(0) == 0);
    x!(bukan_elf(1) == -ENOEXEC && bukan_elf(0) == 0);
    x!(jail_suid(1) == -EPERM && jail_suid(0) == 0);
    x!(luber(1) == -EOVERFLOW && luber(0) == 0);
    x!(header_pendek(1) == -EINVAL && header_pendek(0) == 0);
    x!(baca_singkat(1) == -EIO && baca_singkat(0) == 0);
    x!(interp_berlapis(1) == -ELOOP && interp_berlapis(0) == 0);
    x!(ruang_muat_sempit(1) == -E2BIG && ruang_muat_sempit(0) == 0);

    let mut d = 0;
    let mut b = 0;
    macro_rules! q {
        ($f:expr, $n:expr) => {{
            d += 1;
            if $f != $n {
                b += 1;
            }
        }};
    }
    for v in 0..2u32 {
        q!(citra_tak_valid(v), if v != 0 { -EINVAL } else { 0 });
        q!(tak_ada_muat(v), if v != 0 { -EINVAL } else { 0 });
        q!(jenis_salah(v), if v != 0 { -EINVAL } else { 0 });
        q!(peta_tulis(v), if v != 0 { -ETXTBSY } else { 0 });
        q!(bukan_elf(v), if v != 0 { -ENOEXEC } else { 0 });
        q!(jail_suid(v), if v != 0 { -EPERM } else { 0 });
        q!(luber(v), if v != 0 { -EOVERFLOW } else { 0 });
        q!(header_pendek(v), if v != 0 { -EINVAL } else { 0 });
        q!(baca_singkat(v), if v != 0 { -EIO } else { 0 });
        q!(interp_berlapis(v), if v != 0 { -ELOOP } else { 0 });
        q!(ruang_muat_sempit(v), if v != 0 { -E2BIG } else { 0 });
    }

    x!(d == 22);
    x!(b == 0);

    KlatosAdmisiExec2Report {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

#[no_mangle]
pub extern "C" fn klatos_exec2_selftest() -> KlatosAdmisiExec2Report {
    run()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(N_COUNTER == 11);
        assert!(core::mem::size_of::<KlatosAdmisiExec2Report>() == 32);
    };
}
