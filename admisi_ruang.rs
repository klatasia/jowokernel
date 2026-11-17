// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas admisi ruang alamat pengguna (P1.67).

#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EPERM: i32 = 1;
/// `EFAULT` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EFAULT: i32 = 14;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EINVAL: i32 = 22;
/// `EOVERFLOW` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EOVERFLOW: i32 = 36;

/// Tag laporan; cermin `KlatosAdmisiRuangReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4144_4D53; // "KLATADMS"

// ---------------------------------------------------------------------------
// Kode keputusan gerbang masuk syscall
// ---------------------------------------------------------------------------

/// Lolos gerbang; syscall dilanjutkan.
pub const SYS_ENTRY_OK: u32 = 0;
/// Stack pointer tak menunjuk memori pengguna yang terpetakan ⇒ `SIGSEGV`.
pub const SYS_ENTRY_BAD_STACK: u32 = 1;
/// IP tak berada di region mana pun ⇒ `SIGSEGV`.
pub const SYS_ENTRY_UNKNOWN_REGION: u32 = 2;
/// IP berada di region yang dapat ditulisi ⇒ `SIGSEGV`.
pub const SYS_ENTRY_WRITABLE: u32 = 3;
/// `enforces_syscall_regions()` dan IP bukan region syscall ⇒ `SIGSEGV`.
pub const SYS_ENTRY_NON_SYSCALL: u32 = 4;

/// Penghitung untuk audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_admisi_izin_unmap`.
    ProdUnmap = 0,
    /// Jumlah penolakan `-EPERM` dari `klatos_admisi_izin_unmap`.
    ProdUnmapEperm = 1,
    /// Jumlah `klatos_admisi_stack_pengguna_sah`.
    ProdStack = 2,
    /// Jumlah penolakan (hasil `0`) dari `klatos_admisi_stack_pengguna_sah`.
    ProdStackTolak = 3,
    /// Jumlah `klatos_admisi_prekondisi_syscall`.
    ProdSyscall = 4,
    /// Jumlah keputusan selain `SYS_ENTRY_OK` dari gerbang syscall.
    ProdSyscallCrash = 5,
    /// Jumlah `klatos_admisi_rentang_menutupi`.
    ProdRentang = 6,
    /// Jumlah `klatos_admisi_vmo_izin`.
    ProdVmo = 7,
    /// Jumlah penolakan `-EOVERFLOW` dari `klatos_admisi_vmo_izin`.
    ProdVmoOverflow = 8,
    /// Jumlah penolakan `-EINVAL` dari `klatos_admisi_vmo_izin`.
    ProdVmoEval = 9,
}

const N_COUNTER: usize = 10;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Izin menghapus pemetaan mmap dari satu region.
///
/// Cermin `AddressSpace::unmap_mmap_range` (`AddressSpace.cpp:60-64`,
/// `:72-76`, `:104-109`): region harus `mmap` DAN tidak immutable; keduanya
/// `-EPERM`. `0` (terima) atau `-EPERM`.
pub const fn izin_unmap(mmap: u32, immutable: u32) -> i32 {
    if mmap == 0 {
        return -EPERM;
    }
    if immutable != 0 {
        return -EPERM;
    }
    0
}

/// Ukuran nol pada `unmap_mmap_range` ⇒ `EINVAL`.
///
/// Cermin `AddressSpace.cpp:52-53`. Dijalankan SEBELUM perluasan rentang.
pub const fn unmap_ukuran(ukuran: u64) -> i32 {
    if ukuran == 0 {
        -EINVAL
    } else {
        0
    }
}

/// Rentang hasil perluasan bukan rentang pengguna ⇒ `EFAULT`.
///
/// Cermin `AddressSpace.cpp:57-58`. Dijalankan SESUDAH perluasan rentang.
pub const fn unmap_rentang_pengguna(adalah_pengguna: u32) -> i32 {
    if adalah_pengguna == 0 {
        -EFAULT
    } else {
        0
    }
}

/// Kesahihan stack pointer pengguna.
///
/// Cermin `MemoryManager::validate_user_stack` (`MemoryManager.cpp:1671-1686`):
/// alamat harus alamat pengguna; lalu region yang memuat `vaddr` harus
/// user+stack, ATAU (cadangan) region yang memuat `vaddr-1` harus user+stack
/// DAN berakhir persis di `vaddr`.
///
/// `r1_*` = region yang memuat `vaddr`; `r2_*` = region yang memuat `vaddr-1`.
/// `1` (sah) / `0` (tidak).
pub const fn stack_pengguna_sah(
    is_alamat_pengguna: u32,
    r1_ada: u32,
    r1_pengguna: u32,
    r1_stack: u32,
    r2_ada: u32,
    r2_pengguna: u32,
    r2_stack: u32,
    r2_akhir_sama_sp: u32,
) -> u32 {
    if is_alamat_pengguna == 0 {
        return 0;
    }
    if r1_ada != 0 && r1_pengguna != 0 && r1_stack != 0 {
        return 1;
    }
    if r2_ada != 0 && r2_pengguna != 0 && r2_stack != 0 && r2_akhir_sama_sp != 0 {
        return 1;
    }
    0
}

/// Gerbang masuk syscall.
///
/// Cermin `MemoryManager::validate_syscall_preconditions`
/// (`MemoryManager.cpp:1082-1132`). Urutan penolakan persis seperti C++:
/// stack tak sah DAN tak terpetakan → `SYS_ENTRY_BAD_STACK`; IP tanpa region
/// → `SYS_ENTRY_UNKNOWN_REGION`; IP writable → `SYS_ENTRY_WRITABLE`;
/// menegakkan region syscall dan IP bukan region syscall →
/// `SYS_ENTRY_NON_SYSCALL`; selain itu `SYS_ENTRY_OK`.
///
/// `sp_sah` = hasil [`stack_pengguna_sah`]; `sp_mapped` = cadangan "SP menunjuk
/// memori pengguna terpetakan" (region memuat SP, ATAU region memuat SP-1 yang
/// berakhir di SP).
pub const fn prekondisi_syscall(
    sp_sah: u32,
    sp_mapped: u32,
    ip_ada_region: u32,
    ip_writable: u32,
    menegakkan_region_syscall: u32,
    ip_region_syscall: u32,
) -> u32 {
    if sp_sah == 0 && sp_mapped == 0 {
        return SYS_ENTRY_BAD_STACK;
    }
    if ip_ada_region == 0 {
        return SYS_ENTRY_UNKNOWN_REGION;
    }
    if ip_writable != 0 {
        return SYS_ENTRY_WRITABLE;
    }
    if menegakkan_region_syscall != 0 && ip_region_syscall == 0 {
        return SYS_ENTRY_NON_SYSCALL;
    }
    SYS_ENTRY_OK
}

/// Predikat "rentang terlindungi `[r_mulai, r_mulai+r_panjang)` menutupi
/// `[mulai, akhir)`".
///
/// Cermin `MemoryManager::is_allowed_to_read_physical_memory_for_userspace`
/// (`MemoryManager.cpp:247-253`): lewati bila `r_mulai > mulai`, lewati bila
/// `r_mulai + r_panjang < akhir`, selain itu tutup. `1`/`0`.
pub const fn rentang_dilindungi_menutupi(
    r_mulai: u64,
    r_panjang: u64,
    mulai: u64,
    akhir: u64,
) -> u32 {
    if r_mulai > mulai {
        return 0;
    }
    if r_mulai.wrapping_add(r_panjang) < akhir {
        return 0;
    }
    1
}

/// Predikat luber penambahan `a + b`.
///
/// Cermin `(address + size) < address` pada
/// `VirtualRange::expand_to_page_boundaries` (`VirtualRange.cpp:64-65`),
/// `shrink_range_to_page_boundaries` (`MemoryManager.h:371`), dan
/// `PhysicalAddress::offset_addition_would_overflow`
/// (`MemoryManager.cpp:242`). `1` (luber) / `0`.
pub const fn penambahan_meluber(a: u64, b: u64) -> u32 {
    if a.wrapping_add(b) < a {
        1
    } else {
        0
    }
}

/// Validasi `offset_in_vmobject` terhadap ukuran VMObject.
///
/// Cermin `AddressSpace::allocate_region_with_vmobject`
/// (`AddressSpace.cpp:196-207`): luber ⇒ `-EOVERFLOW` (MENANG atas `EINVAL`),
/// offset di luar ⇒ `-EINVAL`, ujung di luar ⇒ `-EINVAL`.
///
/// `offset_meluber` = `Checked<size_t>::addition_would_overflow(offset, size)`;
/// `offset_ge_size` = `offset >= vmobject->size()`;
/// `akhir_gt_size` = `offset + size > vmobject->size()`.
pub const fn vmo_izin(offset_meluber: u32, offset_ge_size: u32, akhir_gt_size: u32) -> i32 {
    if offset_meluber != 0 {
        return -EOVERFLOW;
    }
    if offset_ge_size != 0 {
        return -EINVAL;
    }
    if akhir_gt_size != 0 {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// C-ABI
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn klatos_admisi_izin_unmap(mmap: u32, immutable: u32) -> i32 {
    bump(Counter::ProdUnmap);
    let r = izin_unmap(mmap, immutable);
    if r == -EPERM {
        bump(Counter::ProdUnmapEperm);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_admisi_unmap_ukuran(ukuran: u64) -> i32 {
    unmap_ukuran(ukuran)
}

#[no_mangle]
pub extern "C" fn klatos_admisi_unmap_rentang(adalah_pengguna: u32) -> i32 {
    unmap_rentang_pengguna(adalah_pengguna)
}

#[no_mangle]
pub extern "C" fn klatos_admisi_stack_pengguna_sah(
    is_alamat_pengguna: u32,
    r1_ada: u32,
    r1_pengguna: u32,
    r1_stack: u32,
    r2_ada: u32,
    r2_pengguna: u32,
    r2_stack: u32,
    r2_akhir_sama_sp: u32,
) -> u32 {
    bump(Counter::ProdStack);
    let r = stack_pengguna_sah(
        is_alamat_pengguna,
        r1_ada,
        r1_pengguna,
        r1_stack,
        r2_ada,
        r2_pengguna,
        r2_stack,
        r2_akhir_sama_sp,
    );
    if r == 0 {
        bump(Counter::ProdStackTolak);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_admisi_prekondisi_syscall(
    sp_sah: u32,
    sp_mapped: u32,
    ip_ada_region: u32,
    ip_writable: u32,
    menegakkan_region_syscall: u32,
    ip_region_syscall: u32,
) -> u32 {
    bump(Counter::ProdSyscall);
    let r = prekondisi_syscall(
        sp_sah,
        sp_mapped,
        ip_ada_region,
        ip_writable,
        menegakkan_region_syscall,
        ip_region_syscall,
    );
    if r != SYS_ENTRY_OK {
        bump(Counter::ProdSyscallCrash);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_admisi_rentang_menutupi(
    r_mulai: u64,
    r_panjang: u64,
    mulai: u64,
    akhir: u64,
) -> u32 {
    bump(Counter::ProdRentang);
    rentang_dilindungi_menutupi(r_mulai, r_panjang, mulai, akhir)
}

#[no_mangle]
pub extern "C" fn klatos_admisi_penambahan_meluber(a: u64, b: u64) -> u32 {
    penambahan_meluber(a, b)
}

#[no_mangle]
pub extern "C" fn klatos_admisi_vmo_izin(
    offset_meluber: u32,
    offset_ge_size: u32,
    akhir_gt_size: u32,
) -> i32 {
    bump(Counter::ProdVmo);
    let r = vmo_izin(offset_meluber, offset_ge_size, akhir_gt_size);
    if r == -EOVERFLOW {
        bump(Counter::ProdVmoOverflow);
    } else if r == -EINVAL {
        bump(Counter::ProdVmoEval);
    }
    r
}

#[no_mangle]
pub extern "C" fn klatos_admisi_counter(idx: u32) -> u32 {
    if (idx as usize) < N_COUNTER {
        COUNTERS[idx as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn klatos_admisi_reset() {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Laporan + selftest
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct KlatosAdmisiRuangReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_unmap: u32,
    pub prod_unmap_eperm: u32,
    pub prod_stack: u32,
    pub prod_stack_tolak: u32,
    pub prod_syscall: u32,
    pub prod_syscall_crash: u32,
    pub prod_rentang: u32,
    pub prod_vmo: u32,
    pub prod_vmo_overflow: u32,
    pub prod_vmo_eval: u32,
}

const _: [(); 56] = [(); core::mem::size_of::<KlatosAdmisiRuangReport>()];

#[no_mangle]
pub extern "C" fn klatos_admisi_selftest() -> KlatosAdmisiRuangReport {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;

    macro_rules! check {
        ($cond:expr) => {
            checks += 1;
            if $cond {
                passed += 1;
            }
        };
    }

    // G1 — konstanta cermin POSIX
    check!(EPERM == 1);
    check!(EFAULT == 14);
    check!(EINVAL == 22);
    check!(EOVERFLOW == 36);
    check!(REPORT_TAG == 0x4B4C_4154_4144_4D53);
    check!(SYS_ENTRY_OK == 0);
    check!(SYS_ENTRY_BAD_STACK == 1);
    check!(SYS_ENTRY_UNKNOWN_REGION == 2);
    check!(SYS_ENTRY_WRITABLE == 3);
    check!(SYS_ENTRY_NON_SYSCALL == 4);

    // G2 — izin_unmap
    check!(izin_unmap(1, 0) == 0);
    check!(izin_unmap(0, 0) == -EPERM);
    check!(izin_unmap(1, 1) == -EPERM);
    check!(izin_unmap(0, 1) == -EPERM);

    // G3 — unmap_ukuran / unmap_rentang_pengguna
    check!(unmap_ukuran(0) == -EINVAL);
    check!(unmap_ukuran(1) == 0);
    check!(unmap_ukuran(u64::MAX) == 0);
    check!(unmap_rentang_pengguna(0) == -EFAULT);
    check!(unmap_rentang_pengguna(1) == 0);

    // G4 — stack_pengguna_sah
    // bukan alamat pengguna ⇒ 0, apa pun regionnya
    check!(stack_pengguna_sah(0, 1, 1, 1, 1, 1, 1, 1) == 0);
    // region 1 user+stack ⇒ 1
    check!(stack_pengguna_sah(1, 1, 1, 1, 0, 0, 0, 0) == 1);
    // region 1 ada tapi bukan stack ⇒ jatuh ke cadangan
    check!(stack_pengguna_sah(1, 1, 1, 0, 0, 0, 0, 0) == 0);
    check!(stack_pengguna_sah(1, 1, 1, 0, 1, 1, 1, 1) == 1);
    // cadangan harus berakhir persis di SP
    check!(stack_pengguna_sah(1, 0, 0, 0, 1, 1, 1, 0) == 0);
    check!(stack_pengguna_sah(1, 0, 0, 0, 1, 1, 0, 1) == 0);
    check!(stack_pengguna_sah(1, 0, 0, 0, 1, 0, 1, 1) == 0);
    check!(stack_pengguna_sah(1, 0, 0, 0, 0, 0, 0, 0) == 0);

    // G5 — prekondisi_syscall: presedensi empat penolakan
    // stack tak sah DAN tak terpetakan ⇒ BAD_STACK (menang atas semua)
    check!(prekondisi_syscall(0, 0, 0, 1, 1, 0) == SYS_ENTRY_BAD_STACK);
    // stack tak sah TAPI terpetakan ⇒ lanjut; IP tanpa region ⇒ UNKNOWN
    check!(prekondisi_syscall(0, 1, 0, 0, 0, 0) == SYS_ENTRY_UNKNOWN_REGION);
    // IP writable ⇒ WRITABLE (menang atas NON_SYSCALL)
    check!(prekondisi_syscall(1, 0, 1, 1, 1, 0) == SYS_ENTRY_WRITABLE);
    // menegakkan region syscall + IP bukan region syscall ⇒ NON_SYSCALL
    check!(prekondisi_syscall(1, 0, 1, 0, 1, 0) == SYS_ENTRY_NON_SYSCALL);
    // tidak menegakkan ⇒ OK walau bukan region syscall
    check!(prekondisi_syscall(1, 0, 1, 0, 0, 0) == SYS_ENTRY_OK);
    // region syscall ⇒ OK
    check!(prekondisi_syscall(1, 0, 1, 0, 1, 1) == SYS_ENTRY_OK);
    // stack sah, IP read-only biasa ⇒ OK
    check!(prekondisi_syscall(1, 0, 1, 0, 0, 0) == SYS_ENTRY_OK);

    // G6 — rentang_dilindungi_menutupi
    check!(rentang_dilindungi_menutupi(0, 100, 10, 50) == 1);
    check!(rentang_dilindungi_menutupi(10, 40, 10, 50) == 1);
    check!(rentang_dilindungi_menutupi(10, 39, 10, 50) == 0);
    check!(rentang_dilindungi_menutupi(11, 100, 10, 50) == 0);
    check!(rentang_dilindungi_menutupi(0, 0, 0, 0) == 1);

    // G7 — penambahan_meluber
    check!(penambahan_meluber(0, 0) == 0);
    check!(penambahan_meluber(1, 1) == 0);
    check!(penambahan_meluber(u64::MAX, 1) == 1);
    check!(penambahan_meluber(u64::MAX, u64::MAX) == 1);
    check!(penambahan_meluber(u64::MAX - 1, 1) == 0);

    // G8 — vmo_izin: EOVERFLOW MENANG atas EINVAL
    check!(vmo_izin(1, 1, 1) == -EOVERFLOW);
    check!(vmo_izin(1, 0, 0) == -EOVERFLOW);
    check!(vmo_izin(0, 1, 1) == -EINVAL);
    check!(vmo_izin(0, 1, 0) == -EINVAL);
    check!(vmo_izin(0, 0, 1) == -EINVAL);
    check!(vmo_izin(0, 0, 0) == 0);

    // G9 — diferensial 4096-iter izin_unmap
    let mut d_unmap: u32 = 1;
    for i in 0u32..4096 {
        let mmap = i & 1;
        let imm = (i >> 1) & 1;
        let rust = izin_unmap(mmap, imm);
        let expected: i32 = if mmap == 0 || imm != 0 { -EPERM } else { 0 };
        if rust != expected {
            d_unmap = 0;
        }
    }
    check!(d_unmap == 1);

    // G10 — diferensial 4096-iter stack_pengguna_sah
    let mut d_stack: u32 = 1;
    for i in 0u32..4096 {
        let ua = i & 1;
        let r1a = (i >> 1) & 1;
        let r1u = (i >> 2) & 1;
        let r1s = (i >> 3) & 1;
        let r2a = (i >> 4) & 1;
        let r2u = (i >> 5) & 1;
        let r2s = (i >> 6) & 1;
        let r2e = (i >> 7) & 1;
        let rust = stack_pengguna_sah(ua, r1a, r1u, r1s, r2a, r2u, r2s, r2e);
        let expected: u32 = if ua == 0 {
            0
        } else if r1a != 0 && r1u != 0 && r1s != 0 {
            1
        } else if r2a != 0 && r2u != 0 && r2s != 0 && r2e != 0 {
            1
        } else {
            0
        };
        if rust != expected {
            d_stack = 0;
        }
    }
    check!(d_stack == 1);

    // G11 — diferensial 4096-iter prekondisi_syscall
    let mut d_sys: u32 = 1;
    for i in 0u32..4096 {
        let sps = i & 1;
        let spm = (i >> 1) & 1;
        let ipa = (i >> 2) & 1;
        let ipw = (i >> 3) & 1;
        let enf = (i >> 4) & 1;
        let ips = (i >> 5) & 1;
        let rust = prekondisi_syscall(sps, spm, ipa, ipw, enf, ips);
        let expected: u32 = if sps == 0 && spm == 0 {
            SYS_ENTRY_BAD_STACK
        } else if ipa == 0 {
            SYS_ENTRY_UNKNOWN_REGION
        } else if ipw != 0 {
            SYS_ENTRY_WRITABLE
        } else if enf != 0 && ips == 0 {
            SYS_ENTRY_NON_SYSCALL
        } else {
            SYS_ENTRY_OK
        };
        if rust != expected {
            d_sys = 0;
        }
    }
    check!(d_sys == 1);

    // G12 — diferensial 4096-iter vmo_izin
    let mut d_vmo: u32 = 1;
    for i in 0u32..4096 {
        let of = i & 1;
        let og = (i >> 1) & 1;
        let ag = (i >> 2) & 1;
        let rust = vmo_izin(of, og, ag);
        let expected: i32 = if of != 0 {
            -EOVERFLOW
        } else if og != 0 {
            -EINVAL
        } else if ag != 0 {
            -EINVAL
        } else {
            0
        };
        if rust != expected {
            d_vmo = 0;
        }
    }
    check!(d_vmo == 1);

    // G13 — determinisme
    check!(izin_unmap(1, 0) == izin_unmap(1, 0));
    check!(prekondisi_syscall(1, 0, 1, 1, 1, 0) == prekondisi_syscall(1, 0, 1, 1, 1, 0));
    check!(stack_pengguna_sah(1, 1, 1, 1, 0, 0, 0, 0) == stack_pengguna_sah(1, 1, 1, 1, 0, 0, 0, 0));
    check!(vmo_izin(1, 1, 1) == vmo_izin(1, 1, 1));
    // urutan argumen prekondisi_syscall: ip_writable sebelum enforces
    check!(prekondisi_syscall(1, 0, 1, 1, 0, 0) == SYS_ENTRY_WRITABLE);
    check!(prekondisi_syscall(1, 0, 1, 0, 1, 0) == SYS_ENTRY_NON_SYSCALL);

    KlatosAdmisiRuangReport {
        tag: REPORT_TAG,
        checks,
        passed,
        prod_unmap: COUNTERS[Counter::ProdUnmap as usize].load(Ordering::Relaxed),
        prod_unmap_eperm: COUNTERS[Counter::ProdUnmapEperm as usize].load(Ordering::Relaxed),
        prod_stack: COUNTERS[Counter::ProdStack as usize].load(Ordering::Relaxed),
        prod_stack_tolak: COUNTERS[Counter::ProdStackTolak as usize].load(Ordering::Relaxed),
        prod_syscall: COUNTERS[Counter::ProdSyscall as usize].load(Ordering::Relaxed),
        prod_syscall_crash: COUNTERS[Counter::ProdSyscallCrash as usize].load(Ordering::Relaxed),
        prod_rentang: COUNTERS[Counter::ProdRentang as usize].load(Ordering::Relaxed),
        prod_vmo: COUNTERS[Counter::ProdVmo as usize].load(Ordering::Relaxed),
        prod_vmo_overflow: COUNTERS[Counter::ProdVmoOverflow as usize].load(Ordering::Relaxed),
        prod_vmo_eval: COUNTERS[Counter::ProdVmoEval as usize].load(Ordering::Relaxed),
    }
}

#[no_mangle]
pub extern "C" fn klatos_admisi_report() -> KlatosAdmisiRuangReport {
    KlatosAdmisiRuangReport {
        tag: REPORT_TAG,
        checks: 0,
        passed: 0,
        prod_unmap: COUNTERS[Counter::ProdUnmap as usize].load(Ordering::Relaxed),
        prod_unmap_eperm: COUNTERS[Counter::ProdUnmapEperm as usize].load(Ordering::Relaxed),
        prod_stack: COUNTERS[Counter::ProdStack as usize].load(Ordering::Relaxed),
        prod_stack_tolak: COUNTERS[Counter::ProdStackTolak as usize].load(Ordering::Relaxed),
        prod_syscall: COUNTERS[Counter::ProdSyscall as usize].load(Ordering::Relaxed),
        prod_syscall_crash: COUNTERS[Counter::ProdSyscallCrash as usize].load(Ordering::Relaxed),
        prod_rentang: COUNTERS[Counter::ProdRentang as usize].load(Ordering::Relaxed),
        prod_vmo: COUNTERS[Counter::ProdVmo as usize].load(Ordering::Relaxed),
        prod_vmo_overflow: COUNTERS[Counter::ProdVmoOverflow as usize].load(Ordering::Relaxed),
        prod_vmo_eval: COUNTERS[Counter::ProdVmoEval as usize].load(Ordering::Relaxed),
    }
}
