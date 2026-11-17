// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS PROSES, BATCH 1: kendali proses + identitas**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `ESRCH` — indeks 3.
pub const ESRCH: i32 = 3;
/// `EACCES` — indeks 13.
pub const EACCES: i32 = 13;
/// `EFAULT` — indeks 14.
pub const EFAULT: i32 = 14;
/// `EBUSY` — indeks 16.
pub const EBUSY: i32 = 16;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;

/// Batas jumlah grup tambahan; cermin `NGROUPS_MAX`.
pub const NGROUPS_MAKS: u64 = 32;
/// Batas panjang kunci/nilai metadata coredump (16 KiB); cermin `prctl.cpp:45,47`.
pub const COREDUMP_METADATA_MAKS: u64 = 16 * 1024;

/// Tag laporan; cermin `KlatosAdmisiProsesReport` di `Jowo/Kontrak/KlatSeam97.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5052_4F53; // "KLATPROS"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_pros_ptrace_peer`.
    ProdPtracePeer = 0,
    /// `klatos_pros_ptrace_diri`.
    ProdPtraceDiri = 1,
    /// `klatos_pros_ptrace_kredensial`.
    ProdPtraceKredensial = 2,
    /// `klatos_pros_ptrace_dumpable`.
    ProdPtraceDumpable = 3,
    /// `klatos_pros_ptrace_sudah`.
    ProdPtraceSudah = 4,
    /// `klatos_pros_ptrace_tracer`.
    ProdPtraceTracer = 5,
    /// `klatos_pros_ptrace_tracer_cocok`.
    ProdPtraceTracerCocok = 6,
    /// `klatos_pros_ptrace_berhenti`.
    ProdPtraceBerhenti = 7,
    /// `klatos_pros_ptrace_regs`.
    ProdPtraceRegs = 8,
    /// `klatos_pros_ptrace_setregs_mode`.
    ProdPtraceSetregsMode = 9,
    /// `klatos_pros_ptrace_singlestep_mode`.
    ProdPtraceSinglestepMode = 10,
    /// `klatos_pros_ptrace_permintaan`.
    ProdPtracePermintaan = 11,
    /// `klatos_pros_ptrace_reg_index`.
    ProdPtraceRegIndex = 12,
    /// `klatos_pros_pgid_negatif`.
    ProdPgidNegatif = 13,
    /// `klatos_pros_pgid_proses`.
    ProdPgidProses = 14,
    /// `klatos_pros_pgid_pemimpin`.
    ProdPgidPemimpin = 15,
    /// `klatos_pros_pgid_anak_sesi`.
    ProdPgidAnakSesi = 16,
    /// `klatos_pros_pgid_pindah_sesi`.
    ProdPgidPindahSesi = 17,
    /// `klatos_pros_pgid_grup`.
    ProdPgidGrup = 18,
    /// `klatos_pros_kill_sinyal`.
    ProdKillSinyal = 19,
    /// `klatos_pros_killpg_sinyal`.
    ProdKillpgSinyal = 20,
    /// `klatos_pros_kill_pid_min`.
    ProdKillPidMin = 21,
    /// `klatos_pros_kill_pgrp_negatif`.
    ProdKillPgrpNegatif = 22,
    /// `klatos_pros_kill_proses`.
    ProdKillProses = 23,
    /// `klatos_pros_kill_grup_kosong`.
    ProdKillGrupKosong = 24,
    /// `klatos_pros_kill_inti`.
    ProdKillInti = 25,
    /// `klatos_pros_prctl_dumpable`.
    ProdPrctlDumpable = 26,
    /// `klatos_pros_prctl_annotasi`.
    ProdPrctlAnnotasi = 27,
    /// `klatos_pros_prctl_coredump`.
    ProdPrctlCoredump = 28,
    /// `klatos_pros_prctl_nama`.
    ProdPrctlNama = 29,
    /// `klatos_pros_prctl_opsi`.
    ProdPrctlOpsi = 30,
    /// `klatos_pros_id_sentinel`.
    ProdIdSentinel = 31,
    /// `klatos_pros_grup_count`.
    ProdGrupCount = 32,
    /// `klatos_pros_setgroups_izin`.
    ProdSetgroupsIzin = 33,
}

const N_COUNTER: usize = 34;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// `ptrace` peer tak ditemukan. Transkripsi `ptrace.cpp:34`.
#[inline]
pub const fn pros_ptrace_peer_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -ESRCH;
    }
    0
}

/// `ptrace` pada diri sendiri. Transkripsi `ptrace.cpp:37`.
#[inline]
pub const fn pros_ptrace_diri_sendiri(sendiri: u32) -> i32 {
    if sendiri != 0 {
        return -EPERM;
    }
    0
}

/// `ptrace` kredensial tak cocok atau proses setuid. Transkripsi `ptrace.cpp:45`.
#[inline]
pub const fn pros_ptrace_kredensial(cocok: u32) -> i32 {
    if cocok == 0 {
        return -EACCES;
    }
    0
}

/// `ptrace` proses tak dumpable. Transkripsi `ptrace.cpp:48`.
#[inline]
pub const fn pros_ptrace_dumpable(dumpable: u32) -> i32 {
    if dumpable == 0 {
        return -EACCES;
    }
    0
}

/// `ptrace` sudah dilacak. Satu verdict, dua pemanggil. `ptrace.cpp:26,53`.
#[inline]
pub const fn pros_ptrace_sudah_dilacak(sudah: u32) -> i32 {
    if sudah != 0 {
        return -EBUSY;
    }
    0
}

/// `ptrace` tanpa tracer. Transkripsi `ptrace.cpp:68`.
#[inline]
pub const fn pros_ptrace_tracer_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -EPERM;
    }
    0
}

/// `ptrace` tracer bukan pemanggil. Transkripsi `ptrace.cpp:71`.
#[inline]
pub const fn pros_ptrace_tracer_cocok(cocok: u32) -> i32 {
    if cocok == 0 {
        return -EBUSY;
    }
    0
}

/// `ptrace` peer tidak berhenti. Transkripsi `ptrace.cpp:74`.
#[inline]
pub const fn pros_ptrace_berhenti(berhenti: u32) -> i32 {
    if berhenti == 0 {
        return -EBUSY;
    }
    0
}

/// `PT_GETREGS`/`PT_SETREGS` tanpa register tersimpan.
/// Satu verdict, dua pemanggil. `ptrace.cpp:95,103`.
#[inline]
pub const fn pros_ptrace_regs_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -EINVAL;
    }
    0
}

/// `PT_SETREGS` bukan konteks pengguna. Transkripsi `ptrace.cpp:111`.
#[inline]
pub const fn pros_ptrace_setregs_mode(pengguna: u32) -> i32 {
    if pengguna == 0 {
        return -EFAULT;
    }
    0
}

/// `PT_SINGLESTEP` bukan konteks pengguna. Transkripsi `ptrace.cpp:122`.
#[inline]
pub const fn pros_ptrace_singlestep_mode(pengguna: u32) -> i32 {
    if pengguna == 0 {
        return -EPERM;
    }
    0
}

/// Permintaan `ptrace` tak dikenal. Transkripsi `ptrace.cpp:183`.
#[inline]
pub const fn pros_ptrace_permintaan_tak_dikenal() -> i32 {
    -EINVAL
}

/// Indeks register debug tak sah. Satu verdict, dua pemanggil.
/// Transkripsi `ptrace.cpp:282,318`.
#[inline]
pub const fn pros_ptrace_reg_index(sah: u32) -> i32 {
    if sah == 0 {
        return -EINVAL;
    }
    0
}

/// `setpgid` pgid negatif. Transkripsi `setpgid.cpp:178`.
#[inline]
pub const fn pros_pgid_negatif(negatif: u32) -> i32 {
    if negatif != 0 {
        return -EINVAL;
    }
    0
}

/// Proses sasaran tak ditemukan. Satu verdict, tiga pemanggil.
/// Transkripsi `setpgid.cpp:182,261,265`.
#[inline]
pub const fn pros_pgid_proses_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -ESRCH;
    }
    0
}

/// `setpgid` sasaran adalah pemimpin sesi. Transkripsi `setpgid.cpp:195`.
#[inline]
pub const fn pros_pgid_pemimpin_sesi(pemimpin: u32) -> i32 {
    if pemimpin != 0 {
        return -EPERM;
    }
    0
}

/// `setpgid` anak berada di sesi berbeda. Transkripsi `setpgid.cpp:201`.
#[inline]
pub const fn pros_pgid_anak_beda_sesi(beda: u32) -> i32 {
    if beda != 0 {
        return -EPERM;
    }
    0
}

/// `setpgid` memindahkan proses antar sesi. Transkripsi `setpgid.cpp:209`.
#[inline]
pub const fn pros_pgid_pindah_sesi(pindah: u32) -> i32 {
    if pindah != 0 {
        return -EPERM;
    }
    0
}

/// `setpgid` grup proses tujuan tak ada. Transkripsi `setpgid.cpp:214`.
#[inline]
pub const fn pros_pgid_grup_tak_ada(tak_ada: u32) -> i32 {
    if tak_ada != 0 {
        return -EPERM;
    }
    0
}

/// `kill` sinyal di luar rentang 0..31. Transkripsi `kill.cpp:126`.
#[inline]
pub const fn pros_kill_sinyal_rentang(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// `killpg` sinyal di luar rentang 1..31. Transkripsi `kill.cpp:154`.
#[inline]
pub const fn pros_killpg_sinyal_rentang(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// `kill` dengan pid `INT_MIN`. Transkripsi `kill.cpp:129`.
#[inline]
pub const fn pros_kill_pid_min(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// `killpg` dengan pgrp negatif. Transkripsi `kill.cpp:156`.
#[inline]
pub const fn pros_kill_pgrp_negatif(negatif: u32) -> i32 {
    if negatif != 0 {
        return -EINVAL;
    }
    0
}

/// `kill` proses tak ditemukan. Transkripsi `kill.cpp:144`.
#[inline]
pub const fn pros_kill_proses_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -ESRCH;
    }
    0
}

/// `killpg` grup kosong. Transkripsi `kill.cpp:74`.
#[inline]
pub const fn pros_kill_grup_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        return -ESRCH;
    }
    0
}

/// `kill` sasaran adalah proses inti. Transkripsi `kill.cpp:41`.
#[inline]
pub const fn pros_kill_inti(inti: u32) -> i32 {
    if inti != 0 {
        return -EPERM;
    }
    0
}

/// `PR_SET_DUMPABLE` nilai bukan 0/1. Transkripsi `prctl.cpp:25`.
#[inline]
pub const fn pros_prctl_dumpable_nilai(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// `PR_SET_NO_NEW_SYSCALL_REGION_ANNOTATIONS` nilai ≠ 0.
/// Transkripsi `prctl.cpp:36`.
#[inline]
pub const fn pros_prctl_annotasi_nilai(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// Metadata coredump: kunci kosong/terlalu panjang, atau nilai terlalu panjang.
/// Transkripsi `prctl.cpp:46,48`.
#[inline]
pub const fn pros_prctl_coredump_panjang(kunci: u64, nilai: u64) -> i32 {
    if kunci == 0 || kunci > COREDUMP_METADATA_MAKS {
        return -EINVAL;
    }
    if nilai > COREDUMP_METADATA_MAKS {
        return -EINVAL;
    }
    0
}

/// `PR_SET_PROCESS_NAME` nama kosong/spasi. Transkripsi `prctl.cpp:62`.
#[inline]
pub const fn pros_prctl_nama_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        return -EINVAL;
    }
    0
}

/// Opsi `prctl` tak dikenal. Transkripsi `prctl.cpp:124`.
#[inline]
pub const fn pros_prctl_opsi_tak_dikenal() -> i32 {
    -EINVAL
}

/// Sentinel `(id_t)-1` pada `set*id`. Satu verdict, lima pemanggil.
/// Transkripsi `setuid.cpp:38,71,104,137,181`.
#[inline]
pub const fn pros_id_sentinel(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// `setgroups` jumlah grup melebihi `NGROUPS_MAX`. Transkripsi `setuid.cpp:317`.
#[inline]
pub const fn pros_grup_count_lebih(lebih: u32) -> i32 {
    if lebih != 0 {
        return -EINVAL;
    }
    0
}

/// `setgroups` tanpa hak superuser. Transkripsi `setuid.cpp:323`.
#[inline]
pub const fn pros_setgroups_izin(izin: u32) -> i32 {
    if izin == 0 {
        return -EPERM;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif(v: u32, errno: i32) -> i32 {
    match v {
        0 => 0,
        _ => errno,
    }
}
fn naif_terbalik(v: u32, errno: i32) -> i32 {
    match v {
        0 => errno,
        _ => 0,
    }
}
fn naif_coredump(kunci: u64, nilai: u64) -> i32 {
    let kunci_sah = (1..=COREDUMP_METADATA_MAKS).contains(&kunci);
    let nilai_sah = nilai <= COREDUMP_METADATA_MAKS;
    if kunci_sah && nilai_sah {
        0
    } else {
        -EINVAL
    }
}

/// Kontrol NEGATIF: `pros_ptrace_singlestep_mode` yang salah mengembalikan
/// `EFAULT` (bukan `EPERM`). Sengaja SALAH.
fn pros_ptrace_singlestep_mode_salah(pengguna: u32) -> i32 {
    if pengguna == 0 {
        return -EFAULT;
    }
    0
}

/// Kontrol NEGATIF: `pros_prctl_coredump_panjang` yang salah memakai
/// `>= COREDUMP_METADATA_MAKS` (bukan `>`). Sengaja SALAH; berdaya pada
/// `kunci == COREDUMP_METADATA_MAKS`.
fn pros_prctl_coredump_panjang_salah(kunci: u64, nilai: u64) -> i32 {
    if kunci == 0 || kunci >= COREDUMP_METADATA_MAKS {
        return -EINVAL;
    }
    if nilai >= COREDUMP_METADATA_MAKS {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`pros_ptrace_peer_ada`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_peer(ada: u32) -> i32 {
    bump(Counter::ProdPtracePeer);
    pros_ptrace_peer_ada(ada)
}

/// Lihat [`pros_ptrace_diri_sendiri`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_diri(sendiri: u32) -> i32 {
    bump(Counter::ProdPtraceDiri);
    pros_ptrace_diri_sendiri(sendiri)
}

/// Lihat [`pros_ptrace_kredensial`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_kredensial(cocok: u32) -> i32 {
    bump(Counter::ProdPtraceKredensial);
    pros_ptrace_kredensial(cocok)
}

/// Lihat [`pros_ptrace_dumpable`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_dumpable(dumpable: u32) -> i32 {
    bump(Counter::ProdPtraceDumpable);
    pros_ptrace_dumpable(dumpable)
}

/// Lihat [`pros_ptrace_sudah_dilacak`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_sudah(sudah: u32) -> i32 {
    bump(Counter::ProdPtraceSudah);
    pros_ptrace_sudah_dilacak(sudah)
}

/// Lihat [`pros_ptrace_tracer_ada`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_tracer(ada: u32) -> i32 {
    bump(Counter::ProdPtraceTracer);
    pros_ptrace_tracer_ada(ada)
}

/// Lihat [`pros_ptrace_tracer_cocok`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_tracer_cocok(cocok: u32) -> i32 {
    bump(Counter::ProdPtraceTracerCocok);
    pros_ptrace_tracer_cocok(cocok)
}

/// Lihat [`pros_ptrace_berhenti`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_berhenti(berhenti: u32) -> i32 {
    bump(Counter::ProdPtraceBerhenti);
    pros_ptrace_berhenti(berhenti)
}

/// Lihat [`pros_ptrace_regs_ada`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_regs(ada: u32) -> i32 {
    bump(Counter::ProdPtraceRegs);
    pros_ptrace_regs_ada(ada)
}

/// Lihat [`pros_ptrace_setregs_mode`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_setregs_mode(pengguna: u32) -> i32 {
    bump(Counter::ProdPtraceSetregsMode);
    pros_ptrace_setregs_mode(pengguna)
}

/// Lihat [`pros_ptrace_singlestep_mode`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_singlestep_mode(pengguna: u32) -> i32 {
    bump(Counter::ProdPtraceSinglestepMode);
    pros_ptrace_singlestep_mode(pengguna)
}

/// Lihat [`pros_ptrace_permintaan_tak_dikenal`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_permintaan() -> i32 {
    bump(Counter::ProdPtracePermintaan);
    pros_ptrace_permintaan_tak_dikenal()
}

/// Lihat [`pros_ptrace_reg_index`].
#[no_mangle]
pub extern "C" fn klatos_pros_ptrace_reg_index(sah: u32) -> i32 {
    bump(Counter::ProdPtraceRegIndex);
    pros_ptrace_reg_index(sah)
}

/// Lihat [`pros_pgid_negatif`].
#[no_mangle]
pub extern "C" fn klatos_pros_pgid_negatif(negatif: u32) -> i32 {
    bump(Counter::ProdPgidNegatif);
    pros_pgid_negatif(negatif)
}

/// Lihat [`pros_pgid_proses_ada`].
#[no_mangle]
pub extern "C" fn klatos_pros_pgid_proses(ada: u32) -> i32 {
    bump(Counter::ProdPgidProses);
    pros_pgid_proses_ada(ada)
}

/// Lihat [`pros_pgid_pemimpin_sesi`].
#[no_mangle]
pub extern "C" fn klatos_pros_pgid_pemimpin(pemimpin: u32) -> i32 {
    bump(Counter::ProdPgidPemimpin);
    pros_pgid_pemimpin_sesi(pemimpin)
}

/// Lihat [`pros_pgid_anak_beda_sesi`].
#[no_mangle]
pub extern "C" fn klatos_pros_pgid_anak_sesi(beda: u32) -> i32 {
    bump(Counter::ProdPgidAnakSesi);
    pros_pgid_anak_beda_sesi(beda)
}

/// Lihat [`pros_pgid_pindah_sesi`].
#[no_mangle]
pub extern "C" fn klatos_pros_pgid_pindah_sesi(pindah: u32) -> i32 {
    bump(Counter::ProdPgidPindahSesi);
    pros_pgid_pindah_sesi(pindah)
}

/// Lihat [`pros_pgid_grup_tak_ada`].
#[no_mangle]
pub extern "C" fn klatos_pros_pgid_grup(tak_ada: u32) -> i32 {
    bump(Counter::ProdPgidGrup);
    pros_pgid_grup_tak_ada(tak_ada)
}

/// Lihat [`pros_kill_sinyal_rentang`].
#[no_mangle]
pub extern "C" fn klatos_pros_kill_sinyal(salah: u32) -> i32 {
    bump(Counter::ProdKillSinyal);
    pros_kill_sinyal_rentang(salah)
}

/// Lihat [`pros_killpg_sinyal_rentang`].
#[no_mangle]
pub extern "C" fn klatos_pros_killpg_sinyal(salah: u32) -> i32 {
    bump(Counter::ProdKillpgSinyal);
    pros_killpg_sinyal_rentang(salah)
}

/// Lihat [`pros_kill_pid_min`].
#[no_mangle]
pub extern "C" fn klatos_pros_kill_pid_min(salah: u32) -> i32 {
    bump(Counter::ProdKillPidMin);
    pros_kill_pid_min(salah)
}

/// Lihat [`pros_kill_pgrp_negatif`].
#[no_mangle]
pub extern "C" fn klatos_pros_kill_pgrp_negatif(negatif: u32) -> i32 {
    bump(Counter::ProdKillPgrpNegatif);
    pros_kill_pgrp_negatif(negatif)
}

/// Lihat [`pros_kill_proses_ada`].
#[no_mangle]
pub extern "C" fn klatos_pros_kill_proses(ada: u32) -> i32 {
    bump(Counter::ProdKillProses);
    pros_kill_proses_ada(ada)
}

/// Lihat [`pros_kill_grup_kosong`].
#[no_mangle]
pub extern "C" fn klatos_pros_kill_grup_kosong(kosong: u32) -> i32 {
    bump(Counter::ProdKillGrupKosong);
    pros_kill_grup_kosong(kosong)
}

/// Lihat [`pros_kill_inti`].
#[no_mangle]
pub extern "C" fn klatos_pros_kill_inti(inti: u32) -> i32 {
    bump(Counter::ProdKillInti);
    pros_kill_inti(inti)
}

/// Lihat [`pros_prctl_dumpable_nilai`].
#[no_mangle]
pub extern "C" fn klatos_pros_prctl_dumpable(salah: u32) -> i32 {
    bump(Counter::ProdPrctlDumpable);
    pros_prctl_dumpable_nilai(salah)
}

/// Lihat [`pros_prctl_annotasi_nilai`].
#[no_mangle]
pub extern "C" fn klatos_pros_prctl_annotasi(salah: u32) -> i32 {
    bump(Counter::ProdPrctlAnnotasi);
    pros_prctl_annotasi_nilai(salah)
}

/// Lihat [`pros_prctl_coredump_panjang`].
#[no_mangle]
pub extern "C" fn klatos_pros_prctl_coredump(kunci: u64, nilai: u64) -> i32 {
    bump(Counter::ProdPrctlCoredump);
    pros_prctl_coredump_panjang(kunci, nilai)
}

/// Lihat [`pros_prctl_nama_kosong`].
#[no_mangle]
pub extern "C" fn klatos_pros_prctl_nama(kosong: u32) -> i32 {
    bump(Counter::ProdPrctlNama);
    pros_prctl_nama_kosong(kosong)
}

/// Lihat [`pros_prctl_opsi_tak_dikenal`].
#[no_mangle]
pub extern "C" fn klatos_pros_prctl_opsi() -> i32 {
    bump(Counter::ProdPrctlOpsi);
    pros_prctl_opsi_tak_dikenal()
}

/// Lihat [`pros_id_sentinel`].
#[no_mangle]
pub extern "C" fn klatos_pros_id_sentinel(salah: u32) -> i32 {
    bump(Counter::ProdIdSentinel);
    pros_id_sentinel(salah)
}

/// Lihat [`pros_grup_count_lebih`].
#[no_mangle]
pub extern "C" fn klatos_pros_grup_count(lebih: u32) -> i32 {
    bump(Counter::ProdGrupCount);
    pros_grup_count_lebih(lebih)
}

/// Lihat [`pros_setgroups_izin`].
#[no_mangle]
pub extern "C" fn klatos_pros_setgroups_izin(izin: u32) -> i32 {
    bump(Counter::ProdSetgroupsIzin);
    pros_setgroups_izin(izin)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_pros_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_pros_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiProsesReport` di `Jowo/Kontrak/KlatSeam97.h`.
#[repr(C)]
pub struct KlatosAdmisiProsesReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub teras_checks: u32,
    pub teras_passed: u32,
    pub sub_checks: u32,
    pub sub_passed: u32,
    pub prod_ptrace_peer: u32,
    pub prod_ptrace_diri: u32,
    pub prod_ptrace_kredensial: u32,
    pub prod_ptrace_dumpable: u32,
    pub prod_ptrace_sudah: u32,
    pub prod_ptrace_tracer: u32,
    pub prod_ptrace_tracer_cocok: u32,
    pub prod_ptrace_berhenti: u32,
    pub prod_ptrace_regs: u32,
    pub prod_ptrace_setregs_mode: u32,
    pub prod_ptrace_singlestep_mode: u32,
    pub prod_ptrace_permintaan: u32,
    pub prod_ptrace_reg_index: u32,
    pub prod_pgid_negatif: u32,
    pub prod_pgid_proses: u32,
    pub prod_pgid_pemimpin: u32,
    pub prod_pgid_anak_sesi: u32,
    pub prod_pgid_pindah_sesi: u32,
    pub prod_pgid_grup: u32,
    pub prod_kill_sinyal: u32,
    pub prod_killpg_sinyal: u32,
    pub prod_kill_pid_min: u32,
    pub prod_kill_pgrp_negatif: u32,
    pub prod_kill_proses: u32,
    pub prod_kill_grup_kosong: u32,
    pub prod_kill_inti: u32,
    pub prod_prctl_dumpable: u32,
    pub prod_prctl_annotasi: u32,
    pub prod_prctl_coredump: u32,
    pub prod_prctl_nama: u32,
    pub prod_prctl_opsi: u32,
    pub prod_id_sentinel: u32,
    pub prod_grup_count: u32,
    pub prod_setgroups_izin: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Teras,
    Sub,
}

struct Matrix {
    checks: u32,
    passed: u32,
    teras_checks: u32,
    teras_passed: u32,
    sub_checks: u32,
    sub_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            teras_checks: 0,
            teras_passed: 0,
            sub_checks: 0,
            sub_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Teras => {
                self.teras_checks += 1;
                if lulus {
                    self.teras_passed += 1;
                }
            }
            Group::Sub => {
                self.sub_checks += 1;
                if lulus {
                    self.sub_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **68**.
pub const DIFF_EXPECTED: u32 = 68;

fn run_matrix() -> KlatosAdmisiProsesReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5052_4F53, Group::Teras);
    m.catat(
        EPERM == 1
            && ESRCH == 3
            && EACCES == 13
            && EFAULT == 14
            && EBUSY == 16
            && EINVAL == 22
            && NGROUPS_MAKS == 32
            && COREDUMP_METADATA_MAKS == 16 * 1024,
        Group::Teras,
    );

    // --- inti (Teras): ptrace ---------------------------------------------
    m.catat(pros_ptrace_peer_ada(0) == -ESRCH, Group::Teras);
    m.catat(pros_ptrace_peer_ada(1) == 0, Group::Teras);
    m.catat(pros_ptrace_diri_sendiri(1) == -EPERM, Group::Teras);
    m.catat(pros_ptrace_diri_sendiri(0) == 0, Group::Teras);
    m.catat(pros_ptrace_kredensial(0) == -EACCES, Group::Teras);
    m.catat(pros_ptrace_kredensial(1) == 0, Group::Teras);
    m.catat(pros_ptrace_dumpable(0) == -EACCES, Group::Teras);
    m.catat(pros_ptrace_dumpable(1) == 0, Group::Teras);
    m.catat(pros_ptrace_sudah_dilacak(1) == -EBUSY, Group::Teras);
    m.catat(pros_ptrace_sudah_dilacak(0) == 0, Group::Teras);
    m.catat(pros_ptrace_tracer_ada(0) == -EPERM, Group::Teras);
    m.catat(pros_ptrace_tracer_ada(1) == 0, Group::Teras);
    m.catat(pros_ptrace_tracer_cocok(0) == -EBUSY, Group::Teras);
    m.catat(pros_ptrace_tracer_cocok(1) == 0, Group::Teras);
    m.catat(pros_ptrace_berhenti(0) == -EBUSY, Group::Teras);
    m.catat(pros_ptrace_berhenti(1) == 0, Group::Teras);
    m.catat(pros_ptrace_regs_ada(0) == -EINVAL, Group::Teras);
    m.catat(pros_ptrace_regs_ada(1) == 0, Group::Teras);
    m.catat(pros_ptrace_setregs_mode(0) == -EFAULT, Group::Teras);
    m.catat(pros_ptrace_setregs_mode(1) == 0, Group::Teras);
    m.catat(pros_ptrace_singlestep_mode(0) == -EPERM, Group::Teras);
    m.catat(pros_ptrace_singlestep_mode(1) == 0, Group::Teras);
    m.catat(pros_ptrace_permintaan_tak_dikenal() == -EINVAL, Group::Teras);
    m.catat(pros_ptrace_reg_index(0) == -EINVAL, Group::Teras);
    m.catat(pros_ptrace_reg_index(1) == 0, Group::Teras);

    // --- subsistem (Sub): setpgid + kill + prctl + setuid -----------------
    m.catat(pros_pgid_negatif(1) == -EINVAL, Group::Sub);
    m.catat(pros_pgid_negatif(0) == 0, Group::Sub);
    m.catat(pros_pgid_proses_ada(0) == -ESRCH, Group::Sub);
    m.catat(pros_pgid_proses_ada(1) == 0, Group::Sub);
    m.catat(pros_pgid_pemimpin_sesi(1) == -EPERM, Group::Sub);
    m.catat(pros_pgid_pemimpin_sesi(0) == 0, Group::Sub);
    m.catat(pros_pgid_anak_beda_sesi(1) == -EPERM, Group::Sub);
    m.catat(pros_pgid_anak_beda_sesi(0) == 0, Group::Sub);
    m.catat(pros_pgid_pindah_sesi(1) == -EPERM, Group::Sub);
    m.catat(pros_pgid_pindah_sesi(0) == 0, Group::Sub);
    m.catat(pros_pgid_grup_tak_ada(1) == -EPERM, Group::Sub);
    m.catat(pros_pgid_grup_tak_ada(0) == 0, Group::Sub);
    m.catat(pros_kill_sinyal_rentang(1) == -EINVAL, Group::Sub);
    m.catat(pros_kill_sinyal_rentang(0) == 0, Group::Sub);
    m.catat(pros_killpg_sinyal_rentang(1) == -EINVAL, Group::Sub);
    m.catat(pros_killpg_sinyal_rentang(0) == 0, Group::Sub);
    m.catat(pros_kill_pid_min(1) == -EINVAL, Group::Sub);
    m.catat(pros_kill_pid_min(0) == 0, Group::Sub);
    m.catat(pros_kill_pgrp_negatif(1) == -EINVAL, Group::Sub);
    m.catat(pros_kill_pgrp_negatif(0) == 0, Group::Sub);
    m.catat(pros_kill_proses_ada(0) == -ESRCH, Group::Sub);
    m.catat(pros_kill_proses_ada(1) == 0, Group::Sub);
    m.catat(pros_kill_grup_kosong(1) == -ESRCH, Group::Sub);
    m.catat(pros_kill_grup_kosong(0) == 0, Group::Sub);
    m.catat(pros_kill_inti(1) == -EPERM, Group::Sub);
    m.catat(pros_kill_inti(0) == 0, Group::Sub);
    m.catat(pros_prctl_dumpable_nilai(1) == -EINVAL, Group::Sub);
    m.catat(pros_prctl_dumpable_nilai(0) == 0, Group::Sub);
    m.catat(pros_prctl_annotasi_nilai(1) == -EINVAL, Group::Sub);
    m.catat(pros_prctl_annotasi_nilai(0) == 0, Group::Sub);
    m.catat(pros_prctl_coredump_panjang(0, 0) == -EINVAL, Group::Sub);
    m.catat(pros_prctl_coredump_panjang(5, 5) == 0, Group::Sub);
    m.catat(pros_prctl_coredump_panjang(5, COREDUMP_METADATA_MAKS + 1) == -EINVAL, Group::Sub);
    m.catat(pros_prctl_nama_kosong(1) == -EINVAL, Group::Sub);
    m.catat(pros_prctl_nama_kosong(0) == 0, Group::Sub);
    m.catat(pros_prctl_opsi_tak_dikenal() == -EINVAL, Group::Sub);
    m.catat(pros_id_sentinel(1) == -EINVAL, Group::Sub);
    m.catat(pros_id_sentinel(0) == 0, Group::Sub);
    m.catat(pros_grup_count_lebih(1) == -EINVAL, Group::Sub);
    m.catat(pros_grup_count_lebih(0) == 0, Group::Sub);
    m.catat(pros_setgroups_izin(0) == -EPERM, Group::Sub);
    m.catat(pros_setgroups_izin(1) == 0, Group::Sub);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // EPERM (bukan EFAULT) pada PT_SINGLESTEP di luar konteks pengguna.
        m.catat(pros_ptrace_singlestep_mode(0) == -EPERM, Group::Teras);
        m.catat(pros_ptrace_singlestep_mode_salah(0) == -EFAULT, Group::Teras);
        m.catat(
            pros_ptrace_singlestep_mode(0) != pros_ptrace_singlestep_mode_salah(0),
            Group::Teras,
        );
        m.catat(
            pros_ptrace_singlestep_mode(1) == pros_ptrace_singlestep_mode_salah(1),
            Group::Teras,
        );

        // Batas kunci coredump memakai `>` (bukan `>=`).
        m.catat(
            pros_prctl_coredump_panjang(COREDUMP_METADATA_MAKS, 5) == 0,
            Group::Sub,
        );
        m.catat(
            pros_prctl_coredump_panjang_salah(COREDUMP_METADATA_MAKS, 5) == -EINVAL,
            Group::Sub,
        );
        m.catat(
            pros_prctl_coredump_panjang(COREDUMP_METADATA_MAKS, 5)
                != pros_prctl_coredump_panjang_salah(COREDUMP_METADATA_MAKS, 5),
            Group::Sub,
        );
        m.catat(
            pros_prctl_coredump_panjang(5, 5) == pros_prctl_coredump_panjang_salah(5, 5),
            Group::Sub,
        );

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if pros_prctl_opsi_tak_dikenal() != -EINVAL {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Sub);
    }

    // --- diferensial produksi vs oracle -----------------------------------
    macro_rules! ukur {
        ($g:expr, $p:expr, $n:expr) => {{
            if $p != $n {
                beda += 1;
            }
            diukur += 1;
            let _ = $g;
        }};
    }

    // verdict tanpa argumen.
    ukur!(Group::Teras, pros_ptrace_permintaan_tak_dikenal(), -EINVAL);
    ukur!(Group::Sub, pros_prctl_opsi_tak_dikenal(), -EINVAL);

    // verdict boolean tunggal (28 verdict × 2).
    for v in 0..2u32 {
        ukur!(Group::Teras, pros_ptrace_peer_ada(v), naif_terbalik(v, -ESRCH));
        ukur!(Group::Teras, pros_ptrace_diri_sendiri(v), naif(v, -EPERM));
        ukur!(Group::Teras, pros_ptrace_kredensial(v), naif_terbalik(v, -EACCES));
        ukur!(Group::Teras, pros_ptrace_dumpable(v), naif_terbalik(v, -EACCES));
        ukur!(Group::Teras, pros_ptrace_sudah_dilacak(v), naif(v, -EBUSY));
        ukur!(Group::Teras, pros_ptrace_tracer_ada(v), naif_terbalik(v, -EPERM));
        ukur!(Group::Teras, pros_ptrace_tracer_cocok(v), naif_terbalik(v, -EBUSY));
        ukur!(Group::Teras, pros_ptrace_berhenti(v), naif_terbalik(v, -EBUSY));
        ukur!(Group::Teras, pros_ptrace_regs_ada(v), naif_terbalik(v, -EINVAL));
        ukur!(Group::Teras, pros_ptrace_setregs_mode(v), naif_terbalik(v, -EFAULT));
        ukur!(Group::Teras, pros_ptrace_singlestep_mode(v), naif_terbalik(v, -EPERM));
        ukur!(Group::Sub, pros_pgid_negatif(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_pgid_proses_ada(v), naif_terbalik(v, -ESRCH));
        ukur!(Group::Sub, pros_pgid_pemimpin_sesi(v), naif(v, -EPERM));
        ukur!(Group::Sub, pros_pgid_anak_beda_sesi(v), naif(v, -EPERM));
        ukur!(Group::Sub, pros_pgid_pindah_sesi(v), naif(v, -EPERM));
        ukur!(Group::Sub, pros_pgid_grup_tak_ada(v), naif(v, -EPERM));
        ukur!(Group::Sub, pros_kill_pid_min(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_kill_pgrp_negatif(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_kill_proses_ada(v), naif_terbalik(v, -ESRCH));
        ukur!(Group::Sub, pros_kill_grup_kosong(v), naif(v, -ESRCH));
        ukur!(Group::Sub, pros_kill_inti(v), naif(v, -EPERM));
        ukur!(Group::Sub, pros_prctl_dumpable_nilai(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_prctl_annotasi_nilai(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_prctl_nama_kosong(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_id_sentinel(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_grup_count_lebih(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_setgroups_izin(v), naif_terbalik(v, -EPERM));
    }

    // verdict dengan argumen tambahan.
    for v in 0..2u32 {
        ukur!(Group::Teras, pros_ptrace_reg_index(v), naif_terbalik(v, -EINVAL));
        ukur!(Group::Sub, pros_kill_sinyal_rentang(v), naif(v, -EINVAL));
        ukur!(Group::Sub, pros_killpg_sinyal_rentang(v), naif(v, -EINVAL));
    }
    for (k, n) in [
        (0u64, 0u64),
        (5, 5),
        (5, COREDUMP_METADATA_MAKS),
        (5, COREDUMP_METADATA_MAKS + 1),
    ] {
        ukur!(
            Group::Sub,
            pros_prctl_coredump_panjang(k, n),
            naif_coredump(k, n)
        );
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Teras);
    m.catat(beda == 0, Group::Teras);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiProsesReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        teras_checks: m.teras_checks,
        teras_passed: m.teras_passed,
        sub_checks: m.sub_checks,
        sub_passed: m.sub_passed,
        prod_ptrace_peer: klatos_pros_counter(Counter::ProdPtracePeer as u32),
        prod_ptrace_diri: klatos_pros_counter(Counter::ProdPtraceDiri as u32),
        prod_ptrace_kredensial: klatos_pros_counter(Counter::ProdPtraceKredensial as u32),
        prod_ptrace_dumpable: klatos_pros_counter(Counter::ProdPtraceDumpable as u32),
        prod_ptrace_sudah: klatos_pros_counter(Counter::ProdPtraceSudah as u32),
        prod_ptrace_tracer: klatos_pros_counter(Counter::ProdPtraceTracer as u32),
        prod_ptrace_tracer_cocok: klatos_pros_counter(Counter::ProdPtraceTracerCocok as u32),
        prod_ptrace_berhenti: klatos_pros_counter(Counter::ProdPtraceBerhenti as u32),
        prod_ptrace_regs: klatos_pros_counter(Counter::ProdPtraceRegs as u32),
        prod_ptrace_setregs_mode: klatos_pros_counter(Counter::ProdPtraceSetregsMode as u32),
        prod_ptrace_singlestep_mode: klatos_pros_counter(Counter::ProdPtraceSinglestepMode as u32),
        prod_ptrace_permintaan: klatos_pros_counter(Counter::ProdPtracePermintaan as u32),
        prod_ptrace_reg_index: klatos_pros_counter(Counter::ProdPtraceRegIndex as u32),
        prod_pgid_negatif: klatos_pros_counter(Counter::ProdPgidNegatif as u32),
        prod_pgid_proses: klatos_pros_counter(Counter::ProdPgidProses as u32),
        prod_pgid_pemimpin: klatos_pros_counter(Counter::ProdPgidPemimpin as u32),
        prod_pgid_anak_sesi: klatos_pros_counter(Counter::ProdPgidAnakSesi as u32),
        prod_pgid_pindah_sesi: klatos_pros_counter(Counter::ProdPgidPindahSesi as u32),
        prod_pgid_grup: klatos_pros_counter(Counter::ProdPgidGrup as u32),
        prod_kill_sinyal: klatos_pros_counter(Counter::ProdKillSinyal as u32),
        prod_killpg_sinyal: klatos_pros_counter(Counter::ProdKillpgSinyal as u32),
        prod_kill_pid_min: klatos_pros_counter(Counter::ProdKillPidMin as u32),
        prod_kill_pgrp_negatif: klatos_pros_counter(Counter::ProdKillPgrpNegatif as u32),
        prod_kill_proses: klatos_pros_counter(Counter::ProdKillProses as u32),
        prod_kill_grup_kosong: klatos_pros_counter(Counter::ProdKillGrupKosong as u32),
        prod_kill_inti: klatos_pros_counter(Counter::ProdKillInti as u32),
        prod_prctl_dumpable: klatos_pros_counter(Counter::ProdPrctlDumpable as u32),
        prod_prctl_annotasi: klatos_pros_counter(Counter::ProdPrctlAnnotasi as u32),
        prod_prctl_coredump: klatos_pros_counter(Counter::ProdPrctlCoredump as u32),
        prod_prctl_nama: klatos_pros_counter(Counter::ProdPrctlNama as u32),
        prod_prctl_opsi: klatos_pros_counter(Counter::ProdPrctlOpsi as u32),
        prod_id_sentinel: klatos_pros_counter(Counter::ProdIdSentinel as u32),
        prod_grup_count: klatos_pros_counter(Counter::ProdGrupCount as u32),
        prod_setgroups_izin: klatos_pros_counter(Counter::ProdSetgroupsIzin as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas proses (batch 1).
#[no_mangle]
pub extern "C" fn klatos_pros_selftest() -> KlatosAdmisiProsesReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5052_4F53);
        assert!(EPERM == 1);
        assert!(ESRCH == 3);
        assert!(EACCES == 13);
        assert!(EFAULT == 14);
        assert!(EBUSY == 16);
        assert!(EINVAL == 22);
        assert!(NGROUPS_MAKS == 32);
        assert!(COREDUMP_METADATA_MAKS == 16384);
        assert!(Counter::ProdPtracePeer as usize == 0);
        assert!(Counter::ProdSetgroupsIzin as usize == 33);
        assert!(N_COUNTER == 34);
        assert!(DIFF_EXPECTED == 68);

        assert!(pros_ptrace_singlestep_mode(0) == -EPERM);
        assert!(pros_ptrace_setregs_mode(0) == -EFAULT);
        assert!(pros_prctl_coredump_panjang(0, 0) == -EINVAL);
        assert!(pros_prctl_coredump_panjang(5, 5) == 0);
        assert!(pros_kill_grup_kosong(1) == -ESRCH);

        // 8 (tag) + 42*4 (u32) + 4 (status) = 180 -> 184 dengan padding.
        assert!(core::mem::size_of::<KlatosAdmisiProsesReport>() == 184);
        assert!(core::mem::align_of::<KlatosAdmisiProsesReport>() == 8);
    };
}
