// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **OTORITAS PENJADWAL & THREAD, BATCH 2: kendali thread + tunggu**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `ESRCH` — indeks 3.
pub const ESRCH: i32 = 3;
/// `EINTR` — indeks 4.
pub const EINTR: i32 = 4;
/// `E2BIG` — indeks 7.
pub const E2BIG: i32 = 7;
/// `ECHILD` — indeks 10.
pub const ECHILD: i32 = 10;
/// `EFAULT` — indeks 14.
pub const EFAULT: i32 = 14;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `EOVERFLOW` — indeks 36.
pub const EOVERFLOW: i32 = 36;
/// `EDEADLK` — indeks 68.
pub const EDEADLK: i32 = 68;
/// `ENOTSUP` — indeks 74.
pub const ENOTSUP: i32 = 74;

/// Tag laporan; cermin `KlatosAdmisiJadwalReport` di `Jowo/Kontrak/KlatSeam98.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5447_4153; // "KLATJDWL"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_tugas_thread_izin`.
    ProdThreadIzin = 0,
    /// `klatos_tugas_proses_ada`.
    ProdProsesAda = 1,
    /// `klatos_tugas_peer_ada`.
    ProdPeerAda = 2,
    /// `klatos_tugas_stack_overflow`.
    ProdStackOverflow = 3,
    /// `klatos_tugas_stack_tak_sah`.
    ProdStackTakSah = 4,
    /// `klatos_tugas_prioritas`.
    ProdPrioritas = 5,
    /// `klatos_tugas_detach_tak_joinable`.
    ProdDetachTakJoinable = 6,
    /// `klatos_tugas_join_diri`.
    ProdJoinDiri = 7,
    /// `klatos_tugas_sinyal_rentang`.
    ProdSinyalRentang = 8,
    /// `klatos_tugas_waitee_tak_ada`.
    ProdWaiteeTakAda = 9,
    /// `klatos_tugas_disown_echild`.
    ProdDisownEchild = 10,
    /// `klatos_tugas_scoped_id`.
    ProdScopedId = 11,
    /// `klatos_tugas_tunggu_interupsi`.
    ProdTungguInterupsi = 12,
    /// `klatos_tugas_tunggu_proses`.
    ProdTungguProses = 13,
    /// `klatos_tugas_tunggu_bukan_anak`.
    ProdTungguBukanAnak = 14,
    /// `klatos_tugas_tunggu_grup`.
    ProdTungguGrup = 15,
    /// `klatos_tugas_idtype`.
    ProdIdtype = 16,
    /// `klatos_tugas_spawn_argmax`.
    ProdSpawnArgmax = 17,
    /// `klatos_tugas_spawn_kosong`.
    ProdSpawnKosong = 18,
    /// `klatos_tugas_spawn_attr`.
    ProdSpawnAttr = 19,
    /// `klatos_tugas_spawn_overflow`.
    ProdSpawnOverflow = 20,
    /// `klatos_tugas_disown_proses`.
    ProdDisownProses = 21,
    /// `klatos_tugas_disown_bukan_anak`.
    ProdDisownBukanAnak = 22,
}

const N_COUNTER: usize = 23;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Hanya superuser boleh mengakses thread proses lain. Transkripsi `sched.cpp:44`.
#[inline]
pub const fn jadwal_thread_izin(izin: u32) -> i32 {
    if izin == 0 {
        return -EPERM;
    }
    0
}

/// Proses penjadwal tak ditemukan. Transkripsi `sched.cpp:54`.
#[inline]
pub const fn jadwal_proses_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -ESRCH;
    }
    0
}

/// Thread peer penjadwal tak ditemukan. Transkripsi `sched.cpp:73`.
#[inline]
pub const fn jadwal_peer_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -ESRCH;
    }
    0
}

/// Penjumlahan `stack_location + stack_size` meluap. Transkripsi `thread.cpp:31`.
#[inline]
pub const fn jadwal_stack_overflow(meluap: u32) -> i32 {
    if meluap != 0 {
        return -EOVERFLOW;
    }
    0
}

/// Stack pengguna tak sah. Transkripsi `thread.cpp:35`.
#[inline]
pub const fn jadwal_stack_tak_sah(sah: u32) -> i32 {
    if sah == 0 {
        return -EFAULT;
    }
    0
}

/// Prioritas thread di luar `[THREAD_PRIORITY_MIN, THREAD_PRIORITY_MAX]`.
/// Transkripsi `thread.cpp:43`.
#[inline]
pub const fn jadwal_prioritas(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// `detach_thread` pada thread tak-joinable. Transkripsi `thread.cpp:143`.
#[inline]
pub const fn jadwal_detach_tak_joinable(tak_joinable: u32) -> i32 {
    if tak_joinable != 0 {
        return -EINVAL;
    }
    0
}

/// `join_thread` pada diri sendiri. Transkripsi `thread.cpp:157`.
#[inline]
pub const fn jadwal_join_diri(diri: u32) -> i32 {
    if diri != 0 {
        return -EDEADLK;
    }
    0
}

/// `kill_thread` sinyal di luar rentang. Transkripsi `thread.cpp:187`.
#[inline]
pub const fn jadwal_sinyal_rentang(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// `WaitBlocker` waitee tak ada di himpunan. Transkripsi `ThreadBlockers.cpp:705`.
#[inline]
pub const fn jadwal_waitee_tak_ada() -> i32 {
    -ECHILD
}

/// `WaitBlocker` disown. Transkripsi `ThreadBlockers.cpp:734`.
#[inline]
pub const fn jadwal_disown_echild() -> i32 {
    -ECHILD
}

/// id `ScopedProcessList` negatif. Transkripsi `ScopedProcessList.cpp:28`.
#[inline]
pub const fn jadwal_scoped_id(negatif: u32) -> i32 {
    if negatif != 0 {
        return -EINVAL;
    }
    0
}

/// `waitid` terinterupsi. Transkripsi `waitid.cpp:20`.
#[inline]
pub const fn jadwal_tunggu_interupsi(interupsi: u32) -> i32 {
    if interupsi != 0 {
        return -EINTR;
    }
    0
}

/// `waitid` proses waitee tak ada. Transkripsi `waitid.cpp:37`.
#[inline]
pub const fn jadwal_tunggu_proses(ada: u32) -> i32 {
    if ada == 0 {
        return -ECHILD;
    }
    0
}

/// `waitid` waitee bukan anak/tracee. Transkripsi `waitid.cpp:41`.
#[inline]
pub const fn jadwal_tunggu_bukan_anak(bukan: u32) -> i32 {
    if bukan != 0 {
        return -ECHILD;
    }
    0
}

/// `waitid` grup waitee tak ada. Transkripsi `waitid.cpp:48`.
#[inline]
pub const fn jadwal_tunggu_grup(ada: u32) -> i32 {
    if ada == 0 {
        return -ECHILD;
    }
    0
}

/// `waitid` idtype tak dikenal. Transkripsi `waitid.cpp:54`.
#[inline]
pub const fn jadwal_idtype(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// `posix_spawn` argumen/lingkungan melebihi `ARG_MAX`. Transkripsi `posix_spawn.cpp:34`.
#[inline]
pub const fn jadwal_spawn_argmax(lebih: u32) -> i32 {
    if lebih != 0 {
        return -E2BIG;
    }
    0
}

/// `posix_spawn` tanpa argumen. Transkripsi `posix_spawn.cpp:36`.
#[inline]
pub const fn jadwal_spawn_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        return -EINVAL;
    }
    0
}

/// `posix_spawn` atribut/file-actions tak didukung. Transkripsi `posix_spawn.cpp:40`.
#[inline]
pub const fn jadwal_spawn_attr(ada: u32) -> i32 {
    if ada != 0 {
        return -ENOTSUP;
    }
    0
}

/// Penjumlahan ukuran string `posix_spawn` meluap. Transkripsi `posix_spawn.cpp:51`.
#[inline]
pub const fn jadwal_spawn_overflow(meluap: u32) -> i32 {
    if meluap != 0 {
        return -EOVERFLOW;
    }
    0
}

/// `disown` proses tak ditemukan. Transkripsi `disown.cpp:20`.
#[inline]
pub const fn jadwal_disown_proses(ada: u32) -> i32 {
    if ada == 0 {
        return -ESRCH;
    }
    0
}

/// `disown` sasaran bukan anak pemanggil. Transkripsi `disown.cpp:23`.
#[inline]
pub const fn jadwal_disown_bukan_anak(bukan: u32) -> i32 {
    if bukan != 0 {
        return -ECHILD;
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

/// Kontrol NEGATIF: `jadwal_join_diri` yang salah mengembalikan `EINVAL`
/// (bukan `EDEADLK`). Sengaja SALAH.
fn jadwal_join_diri_salah(diri: u32) -> i32 {
    if diri != 0 {
        return -EINVAL;
    }
    0
}

/// Kontrol NEGATIF: `jadwal_spawn_attr` yang salah mengembalikan `EINVAL`
/// (bukan `ENOTSUP`). Sengaja SALAH.
fn jadwal_spawn_attr_salah(ada: u32) -> i32 {
    if ada != 0 {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`jadwal_thread_izin`].
#[no_mangle]
pub extern "C" fn klatos_tugas_thread_izin(izin: u32) -> i32 {
    bump(Counter::ProdThreadIzin);
    jadwal_thread_izin(izin)
}

/// Lihat [`jadwal_proses_ada`].
#[no_mangle]
pub extern "C" fn klatos_tugas_proses_ada(ada: u32) -> i32 {
    bump(Counter::ProdProsesAda);
    jadwal_proses_ada(ada)
}

/// Lihat [`jadwal_peer_ada`].
#[no_mangle]
pub extern "C" fn klatos_tugas_peer_ada(ada: u32) -> i32 {
    bump(Counter::ProdPeerAda);
    jadwal_peer_ada(ada)
}

/// Lihat [`jadwal_stack_overflow`].
#[no_mangle]
pub extern "C" fn klatos_tugas_stack_overflow(meluap: u32) -> i32 {
    bump(Counter::ProdStackOverflow);
    jadwal_stack_overflow(meluap)
}

/// Lihat [`jadwal_stack_tak_sah`].
#[no_mangle]
pub extern "C" fn klatos_tugas_stack_tak_sah(sah: u32) -> i32 {
    bump(Counter::ProdStackTakSah);
    jadwal_stack_tak_sah(sah)
}

/// Lihat [`jadwal_prioritas`].
#[no_mangle]
pub extern "C" fn klatos_tugas_prioritas(salah: u32) -> i32 {
    bump(Counter::ProdPrioritas);
    jadwal_prioritas(salah)
}

/// Lihat [`jadwal_detach_tak_joinable`].
#[no_mangle]
pub extern "C" fn klatos_tugas_detach_tak_joinable(tak_joinable: u32) -> i32 {
    bump(Counter::ProdDetachTakJoinable);
    jadwal_detach_tak_joinable(tak_joinable)
}

/// Lihat [`jadwal_join_diri`].
#[no_mangle]
pub extern "C" fn klatos_tugas_join_diri(diri: u32) -> i32 {
    bump(Counter::ProdJoinDiri);
    jadwal_join_diri(diri)
}

/// Lihat [`jadwal_sinyal_rentang`].
#[no_mangle]
pub extern "C" fn klatos_tugas_sinyal_rentang(salah: u32) -> i32 {
    bump(Counter::ProdSinyalRentang);
    jadwal_sinyal_rentang(salah)
}

/// Lihat [`jadwal_waitee_tak_ada`].
#[no_mangle]
pub extern "C" fn klatos_tugas_waitee_tak_ada() -> i32 {
    bump(Counter::ProdWaiteeTakAda);
    jadwal_waitee_tak_ada()
}

/// Lihat [`jadwal_disown_echild`].
#[no_mangle]
pub extern "C" fn klatos_tugas_disown_echild() -> i32 {
    bump(Counter::ProdDisownEchild);
    jadwal_disown_echild()
}

/// Lihat [`jadwal_scoped_id`].
#[no_mangle]
pub extern "C" fn klatos_tugas_scoped_id(negatif: u32) -> i32 {
    bump(Counter::ProdScopedId);
    jadwal_scoped_id(negatif)
}

/// Lihat [`jadwal_tunggu_interupsi`].
#[no_mangle]
pub extern "C" fn klatos_tugas_tunggu_interupsi(interupsi: u32) -> i32 {
    bump(Counter::ProdTungguInterupsi);
    jadwal_tunggu_interupsi(interupsi)
}

/// Lihat [`jadwal_tunggu_proses`].
#[no_mangle]
pub extern "C" fn klatos_tugas_tunggu_proses(ada: u32) -> i32 {
    bump(Counter::ProdTungguProses);
    jadwal_tunggu_proses(ada)
}

/// Lihat [`jadwal_tunggu_bukan_anak`].
#[no_mangle]
pub extern "C" fn klatos_tugas_tunggu_bukan_anak(bukan: u32) -> i32 {
    bump(Counter::ProdTungguBukanAnak);
    jadwal_tunggu_bukan_anak(bukan)
}

/// Lihat [`jadwal_tunggu_grup`].
#[no_mangle]
pub extern "C" fn klatos_tugas_tunggu_grup(ada: u32) -> i32 {
    bump(Counter::ProdTungguGrup);
    jadwal_tunggu_grup(ada)
}

/// Lihat [`jadwal_idtype`].
#[no_mangle]
pub extern "C" fn klatos_tugas_idtype(salah: u32) -> i32 {
    bump(Counter::ProdIdtype);
    jadwal_idtype(salah)
}

/// Lihat [`jadwal_spawn_argmax`].
#[no_mangle]
pub extern "C" fn klatos_tugas_spawn_argmax(lebih: u32) -> i32 {
    bump(Counter::ProdSpawnArgmax);
    jadwal_spawn_argmax(lebih)
}

/// Lihat [`jadwal_spawn_kosong`].
#[no_mangle]
pub extern "C" fn klatos_tugas_spawn_kosong(kosong: u32) -> i32 {
    bump(Counter::ProdSpawnKosong);
    jadwal_spawn_kosong(kosong)
}

/// Lihat [`jadwal_spawn_attr`].
#[no_mangle]
pub extern "C" fn klatos_tugas_spawn_attr(ada: u32) -> i32 {
    bump(Counter::ProdSpawnAttr);
    jadwal_spawn_attr(ada)
}

/// Lihat [`jadwal_spawn_overflow`].
#[no_mangle]
pub extern "C" fn klatos_tugas_spawn_overflow(meluap: u32) -> i32 {
    bump(Counter::ProdSpawnOverflow);
    jadwal_spawn_overflow(meluap)
}

/// Lihat [`jadwal_disown_proses`].
#[no_mangle]
pub extern "C" fn klatos_tugas_disown_proses(ada: u32) -> i32 {
    bump(Counter::ProdDisownProses);
    jadwal_disown_proses(ada)
}

/// Lihat [`jadwal_disown_bukan_anak`].
#[no_mangle]
pub extern "C" fn klatos_tugas_disown_bukan_anak(bukan: u32) -> i32 {
    bump(Counter::ProdDisownBukanAnak);
    jadwal_disown_bukan_anak(bukan)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_tugas_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_tugas_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiJadwalReport` di `Jowo/Kontrak/KlatSeam98.h`.
#[repr(C)]
pub struct KlatosAdmisiJadwalReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub teras_checks: u32,
    pub teras_passed: u32,
    pub sub_checks: u32,
    pub sub_passed: u32,
    pub prod_thread_izin: u32,
    pub prod_proses_ada: u32,
    pub prod_peer_ada: u32,
    pub prod_stack_overflow: u32,
    pub prod_stack_tak_sah: u32,
    pub prod_prioritas: u32,
    pub prod_detach_tak_joinable: u32,
    pub prod_join_diri: u32,
    pub prod_sinyal_rentang: u32,
    pub prod_waitee_tak_ada: u32,
    pub prod_disown_echild: u32,
    pub prod_scoped_id: u32,
    pub prod_tunggu_interupsi: u32,
    pub prod_tunggu_proses: u32,
    pub prod_tunggu_bukan_anak: u32,
    pub prod_tunggu_grup: u32,
    pub prod_idtype: u32,
    pub prod_spawn_argmax: u32,
    pub prod_spawn_kosong: u32,
    pub prod_spawn_attr: u32,
    pub prod_spawn_overflow: u32,
    pub prod_disown_proses: u32,
    pub prod_disown_bukan_anak: u32,
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

/// Jumlah kasus diferensial yang diharapkan: **44**.
pub const DIFF_EXPECTED: u32 = 44;

fn run_matrix() -> KlatosAdmisiJadwalReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5447_4153, Group::Teras);
    m.catat(
        EPERM == 1
            && ESRCH == 3
            && EINTR == 4
            && E2BIG == 7
            && ECHILD == 10
            && EFAULT == 14
            && EINVAL == 22
            && EOVERFLOW == 36
            && EDEADLK == 68
            && ENOTSUP == 74,
        Group::Teras,
    );

    // --- inti (Teras): penjadwal + thread ---------------------------------
    m.catat(jadwal_thread_izin(0) == -EPERM, Group::Teras);
    m.catat(jadwal_thread_izin(1) == 0, Group::Teras);
    m.catat(jadwal_proses_ada(0) == -ESRCH, Group::Teras);
    m.catat(jadwal_proses_ada(1) == 0, Group::Teras);
    m.catat(jadwal_peer_ada(0) == -ESRCH, Group::Teras);
    m.catat(jadwal_peer_ada(1) == 0, Group::Teras);
    m.catat(jadwal_stack_overflow(1) == -EOVERFLOW, Group::Teras);
    m.catat(jadwal_stack_overflow(0) == 0, Group::Teras);
    m.catat(jadwal_stack_tak_sah(0) == -EFAULT, Group::Teras);
    m.catat(jadwal_stack_tak_sah(1) == 0, Group::Teras);
    m.catat(jadwal_prioritas(1) == -EINVAL, Group::Teras);
    m.catat(jadwal_prioritas(0) == 0, Group::Teras);
    m.catat(jadwal_detach_tak_joinable(1) == -EINVAL, Group::Teras);
    m.catat(jadwal_detach_tak_joinable(0) == 0, Group::Teras);
    m.catat(jadwal_join_diri(1) == -EDEADLK, Group::Teras);
    m.catat(jadwal_join_diri(0) == 0, Group::Teras);
    m.catat(jadwal_sinyal_rentang(1) == -EINVAL, Group::Teras);
    m.catat(jadwal_sinyal_rentang(0) == 0, Group::Teras);

    // --- subsistem (Sub): tunggu + spawn + disown -------------------------
    m.catat(jadwal_waitee_tak_ada() == -ECHILD, Group::Sub);
    m.catat(jadwal_disown_echild() == -ECHILD, Group::Sub);
    m.catat(jadwal_scoped_id(1) == -EINVAL, Group::Sub);
    m.catat(jadwal_scoped_id(0) == 0, Group::Sub);
    m.catat(jadwal_tunggu_interupsi(1) == -EINTR, Group::Sub);
    m.catat(jadwal_tunggu_interupsi(0) == 0, Group::Sub);
    m.catat(jadwal_tunggu_proses(0) == -ECHILD, Group::Sub);
    m.catat(jadwal_tunggu_proses(1) == 0, Group::Sub);
    m.catat(jadwal_tunggu_bukan_anak(1) == -ECHILD, Group::Sub);
    m.catat(jadwal_tunggu_bukan_anak(0) == 0, Group::Sub);
    m.catat(jadwal_tunggu_grup(0) == -ECHILD, Group::Sub);
    m.catat(jadwal_tunggu_grup(1) == 0, Group::Sub);
    m.catat(jadwal_idtype(1) == -EINVAL, Group::Sub);
    m.catat(jadwal_idtype(0) == 0, Group::Sub);
    m.catat(jadwal_spawn_argmax(1) == -E2BIG, Group::Sub);
    m.catat(jadwal_spawn_argmax(0) == 0, Group::Sub);
    m.catat(jadwal_spawn_kosong(1) == -EINVAL, Group::Sub);
    m.catat(jadwal_spawn_kosong(0) == 0, Group::Sub);
    m.catat(jadwal_spawn_attr(1) == -ENOTSUP, Group::Sub);
    m.catat(jadwal_spawn_attr(0) == 0, Group::Sub);
    m.catat(jadwal_spawn_overflow(1) == -EOVERFLOW, Group::Sub);
    m.catat(jadwal_spawn_overflow(0) == 0, Group::Sub);
    m.catat(jadwal_disown_proses(0) == -ESRCH, Group::Sub);
    m.catat(jadwal_disown_proses(1) == 0, Group::Sub);
    m.catat(jadwal_disown_bukan_anak(1) == -ECHILD, Group::Sub);
    m.catat(jadwal_disown_bukan_anak(0) == 0, Group::Sub);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // EDEADLK (bukan EINVAL) pada join diri sendiri.
        m.catat(jadwal_join_diri(1) == -EDEADLK, Group::Teras);
        m.catat(jadwal_join_diri_salah(1) == -EINVAL, Group::Teras);
        m.catat(
            jadwal_join_diri(1) != jadwal_join_diri_salah(1),
            Group::Teras,
        );
        m.catat(
            jadwal_join_diri(0) == jadwal_join_diri_salah(0),
            Group::Teras,
        );

        // ENOTSUP (bukan EINVAL) pada atribut posix_spawn.
        m.catat(jadwal_spawn_attr(1) == -ENOTSUP, Group::Sub);
        m.catat(jadwal_spawn_attr_salah(1) == -EINVAL, Group::Sub);
        m.catat(
            jadwal_spawn_attr(1) != jadwal_spawn_attr_salah(1),
            Group::Sub,
        );
        m.catat(
            jadwal_spawn_attr(0) == jadwal_spawn_attr_salah(0),
            Group::Sub,
        );

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if jadwal_waitee_tak_ada() != -ECHILD {
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
    ukur!(Group::Sub, jadwal_waitee_tak_ada(), -ECHILD);
    ukur!(Group::Sub, jadwal_disown_echild(), -ECHILD);

    // verdict boolean tunggal (21 verdict × 2).
    for v in 0..2u32 {
        ukur!(Group::Teras, jadwal_thread_izin(v), naif_terbalik(v, -EPERM));
        ukur!(Group::Teras, jadwal_proses_ada(v), naif_terbalik(v, -ESRCH));
        ukur!(Group::Teras, jadwal_peer_ada(v), naif_terbalik(v, -ESRCH));
        ukur!(Group::Teras, jadwal_stack_overflow(v), naif(v, -EOVERFLOW));
        ukur!(Group::Teras, jadwal_stack_tak_sah(v), naif_terbalik(v, -EFAULT));
        ukur!(Group::Teras, jadwal_prioritas(v), naif(v, -EINVAL));
        ukur!(Group::Teras, jadwal_detach_tak_joinable(v), naif(v, -EINVAL));
        ukur!(Group::Teras, jadwal_join_diri(v), naif(v, -EDEADLK));
        ukur!(Group::Teras, jadwal_sinyal_rentang(v), naif(v, -EINVAL));
        ukur!(Group::Sub, jadwal_scoped_id(v), naif(v, -EINVAL));
        ukur!(Group::Sub, jadwal_tunggu_interupsi(v), naif(v, -EINTR));
        ukur!(Group::Sub, jadwal_tunggu_proses(v), naif_terbalik(v, -ECHILD));
        ukur!(Group::Sub, jadwal_tunggu_bukan_anak(v), naif(v, -ECHILD));
        ukur!(Group::Sub, jadwal_tunggu_grup(v), naif_terbalik(v, -ECHILD));
        ukur!(Group::Sub, jadwal_idtype(v), naif(v, -EINVAL));
        ukur!(Group::Sub, jadwal_spawn_argmax(v), naif(v, -E2BIG));
        ukur!(Group::Sub, jadwal_spawn_kosong(v), naif(v, -EINVAL));
        ukur!(Group::Sub, jadwal_spawn_attr(v), naif(v, -ENOTSUP));
        ukur!(Group::Sub, jadwal_spawn_overflow(v), naif(v, -EOVERFLOW));
        ukur!(Group::Sub, jadwal_disown_proses(v), naif_terbalik(v, -ESRCH));
        ukur!(Group::Sub, jadwal_disown_bukan_anak(v), naif(v, -ECHILD));
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Teras);
    m.catat(beda == 0, Group::Teras);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiJadwalReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        teras_checks: m.teras_checks,
        teras_passed: m.teras_passed,
        sub_checks: m.sub_checks,
        sub_passed: m.sub_passed,
        prod_thread_izin: klatos_tugas_counter(Counter::ProdThreadIzin as u32),
        prod_proses_ada: klatos_tugas_counter(Counter::ProdProsesAda as u32),
        prod_peer_ada: klatos_tugas_counter(Counter::ProdPeerAda as u32),
        prod_stack_overflow: klatos_tugas_counter(Counter::ProdStackOverflow as u32),
        prod_stack_tak_sah: klatos_tugas_counter(Counter::ProdStackTakSah as u32),
        prod_prioritas: klatos_tugas_counter(Counter::ProdPrioritas as u32),
        prod_detach_tak_joinable: klatos_tugas_counter(Counter::ProdDetachTakJoinable as u32),
        prod_join_diri: klatos_tugas_counter(Counter::ProdJoinDiri as u32),
        prod_sinyal_rentang: klatos_tugas_counter(Counter::ProdSinyalRentang as u32),
        prod_waitee_tak_ada: klatos_tugas_counter(Counter::ProdWaiteeTakAda as u32),
        prod_disown_echild: klatos_tugas_counter(Counter::ProdDisownEchild as u32),
        prod_scoped_id: klatos_tugas_counter(Counter::ProdScopedId as u32),
        prod_tunggu_interupsi: klatos_tugas_counter(Counter::ProdTungguInterupsi as u32),
        prod_tunggu_proses: klatos_tugas_counter(Counter::ProdTungguProses as u32),
        prod_tunggu_bukan_anak: klatos_tugas_counter(Counter::ProdTungguBukanAnak as u32),
        prod_tunggu_grup: klatos_tugas_counter(Counter::ProdTungguGrup as u32),
        prod_idtype: klatos_tugas_counter(Counter::ProdIdtype as u32),
        prod_spawn_argmax: klatos_tugas_counter(Counter::ProdSpawnArgmax as u32),
        prod_spawn_kosong: klatos_tugas_counter(Counter::ProdSpawnKosong as u32),
        prod_spawn_attr: klatos_tugas_counter(Counter::ProdSpawnAttr as u32),
        prod_spawn_overflow: klatos_tugas_counter(Counter::ProdSpawnOverflow as u32),
        prod_disown_proses: klatos_tugas_counter(Counter::ProdDisownProses as u32),
        prod_disown_bukan_anak: klatos_tugas_counter(Counter::ProdDisownBukanAnak as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas penjadwal & thread (batch 2).
#[no_mangle]
pub extern "C" fn klatos_tugas_selftest() -> KlatosAdmisiJadwalReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5447_4153);
        assert!(EPERM == 1);
        assert!(ESRCH == 3);
        assert!(EINTR == 4);
        assert!(E2BIG == 7);
        assert!(ECHILD == 10);
        assert!(EFAULT == 14);
        assert!(EINVAL == 22);
        assert!(EOVERFLOW == 36);
        assert!(EDEADLK == 68);
        assert!(ENOTSUP == 74);
        assert!(Counter::ProdThreadIzin as usize == 0);
        assert!(Counter::ProdDisownBukanAnak as usize == 22);
        assert!(N_COUNTER == 23);
        assert!(DIFF_EXPECTED == 44);

        assert!(jadwal_thread_izin(0) == -EPERM);
        assert!(jadwal_stack_overflow(1) == -EOVERFLOW);
        assert!(jadwal_join_diri(1) == -EDEADLK);
        assert!(jadwal_spawn_attr(1) == -ENOTSUP);
        assert!(jadwal_waitee_tak_ada() == -ECHILD);

        // 8 (tag) + 32*4 (u32) = 136.
        assert!(core::mem::size_of::<KlatosAdmisiJadwalReport>() == 136);
        assert!(core::mem::align_of::<KlatosAdmisiJadwalReport>() == 8);
    };
}
