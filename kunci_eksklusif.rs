// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! KERNEL-RUST KR-MUTEX — **OTORITAS MESIN KEADAAN KUNCI (`Jowo::Mutex`)**.


#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATMTX1".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4D54_5831;

/// Mode kunci: belum terkunci. Cermin `LockMode::Unlocked`.
pub const MODE_BUKA: u32 = 0;
/// Mode kunci: bersama. Cermin `LockMode::Shared`.
pub const MODE_BERSAMA: u32 = 1;
/// Mode kunci: eksklusif. Cermin `LockMode::Exclusive`.
pub const MODE_EKSKLUSIF: u32 = 2;

/// Perilaku: biasa. Cermin `MutexBehavior::Regular`.
pub const BEHAVIOR_BIASA: u32 = 0;
/// Perilaku: big-lock. Cermin `MutexBehavior::BigLock`.
pub const BEHAVIOR_BIGLOCK: u32 = 1;

/// `masuk`: kunci diperoleh (keadaan sudah diperbarui).
pub const KUNCI_OK: u32 = 0;
/// `masuk`: pemanggil HARUS memblokir (keadaan belum berubah).
pub const KUNCI_BLOK: u32 = 1;
/// `masuk`: masukan tak sah / invarian rusak (gagal-tertutup).
pub const KUNCI_SALAH: u32 = 2;

/// `keluar`: kunci masih dipegang (hitungan > 0).
pub const KELUAR_TAHAN: u32 = 0;
/// `keluar`: kunci dilepas penuh; pemanggil harus membangunkan penunggu.
pub const KELUAR_LEPAS: u32 = 1;
/// `keluar`: urutan/masukan tak sah.
pub const KELUAR_SALAH: u32 = 2;

/// `baca`: mode.
pub const WHICH_MODE: u32 = 0;
/// `baca`: hitungan terkunci.
pub const WHICH_TIMES: u32 = 1;
/// `baca`: jumlah pemegang bersama.
pub const WHICH_SHARED: u32 = 2;
/// `baca`: 32 bit rendah pemegang.
pub const WHICH_HOLDER_LO: u32 = 3;
/// `baca`: 32 bit tinggi pemegang.
pub const WHICH_HOLDER_HI: u32 = 4;

// --- keadaan ----------------------------------------------------------------

/// Blok keadaan kunci. Disematkan di objek C++ `Mutex`; hanya modul ini yang
/// menafsirkan/mengubahnya.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KeadaanKunci {
    /// 0 = buka, 1 = bersama, 2 = eksklusif.
    pub mode: u32,
    /// Jumlah kunci bersarang yang dipegang.
    pub times_locked: u32,
    /// Alamat thread pemegang (0 bila bersama/tak diketahui).
    pub holder: u64,
    /// Jumlah pemegang bersama saat mode bersama.
    pub shared_holders: u64,
}

impl KeadaanKunci {
    /// Keadaan awal: tak terkunci.
    pub const fn kosong() -> Self {
        KeadaanKunci { mode: MODE_BUKA, times_locked: 0, holder: 0, shared_holders: 0 }
    }
}

const _: [(); 24] = [(); core::mem::size_of::<KeadaanKunci>()];

// --- penghitung -------------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Counter {
    Masuk = 0,
    Keluar = 1,
    Bangunkan = 2,
    Paksa = 3,
    Pulih = 4,
    TolakMode = 5,
    TolakUrutan = 6,
}
const N_COUNTER: usize = 7;
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

#[inline]
fn mode_sah(m: u32) -> bool {
    m <= MODE_EKSKLUSIF
}

// --- model murni (referensi independen untuk diferensial) -------------------

/// Model referensi yang menuliskan ulang algoritma ORISINAL `Mutex.cpp`
/// (sebelum pemindahan) secara literal, agar dapat dibandingkan dengan pintu
/// Rust. Ditulis independen dari implementasi pintu.
struct ModelKunci {
    mode: u32,
    times: u32,
    holder: u64,
    shared: u64,
}

impl ModelKunci {
    const fn baru() -> Self {
        ModelKunci { mode: MODE_BUKA, times: 0, holder: 0, shared: 0 }
    }

    /// Cermin `Mutex::lock` orisinal. Setiap `VERIFY` orisinal dipetakan ke
    /// `KUNCI_SALAH` (gagal-tertutup) agar dapat dibandingkan dengan pintu.
    fn masuk(&mut self, tid: u64, mode: u32, behavior: u32) -> u32 {
        if mode == MODE_BUKA || mode > MODE_EKSKLUSIF || behavior > BEHAVIOR_BIGLOCK {
            return KUNCI_SALAH;
        }
        if !mode_sah(self.mode) {
            return KUNCI_SALAH;
        }
        match self.mode {
            MODE_BUKA => {
                if self.holder != 0 || self.shared != 0 || self.times != 0 {
                    return KUNCI_SALAH;
                }
                if mode == MODE_EKSKLUSIF {
                    self.mode = MODE_EKSKLUSIF;
                    self.holder = tid;
                    self.shared = 0;
                } else {
                    self.mode = MODE_BERSAMA;
                    self.holder = 0;
                    self.shared = 1;
                }
                self.times = 1;
                KUNCI_OK
            }
            MODE_EKSKLUSIF => {
                if self.holder == 0 || self.shared != 0 {
                    return KUNCI_SALAH;
                }
                if self.holder != tid {
                    KUNCI_BLOK
                } else {
                    self.times = self.times.wrapping_add(1);
                    KUNCI_OK
                }
            }
            MODE_BERSAMA => {
                if behavior != BEHAVIOR_BIASA || self.holder != 0 {
                    return KUNCI_SALAH;
                }
                if mode == MODE_EKSKLUSIF {
                    KUNCI_BLOK
                } else {
                    if self.shared == 0 || self.times == 0 {
                        return KUNCI_SALAH;
                    }
                    self.times = self.times.wrapping_add(1);
                    self.shared = self.shared.wrapping_add(1);
                    KUNCI_OK
                }
            }
            _ => KUNCI_SALAH,
        }
    }

    /// Cermin `Mutex::unlock` orisinal. Kembalikan `(status, prev_mode)`.
    fn keluar(&mut self, tid: u64) -> (u32, u32) {
        let prev = self.mode;
        if !mode_sah(prev) || prev == MODE_BUKA || self.times == 0 {
            return (KELUAR_SALAH, prev);
        }
        match prev {
            MODE_EKSKLUSIF => {
                if self.holder != tid || self.shared != 0 {
                    return (KELUAR_SALAH, prev);
                }
            }
            MODE_BERSAMA => {
                if self.holder != 0 || self.shared == 0 {
                    return (KELUAR_SALAH, prev);
                }
            }
            _ => return (KELUAR_SALAH, prev),
        }
        self.times -= 1;
        if prev == MODE_EKSKLUSIF {
            if self.times == 0 {
                self.holder = 0;
            }
        } else {
            self.shared -= 1;
        }
        if self.times == 0 {
            self.mode = MODE_BUKA;
            (KELUAR_LEPAS, prev)
        } else {
            (KELUAR_TAHAN, prev)
        }
    }

    fn bangun_tunggal(&mut self, tid: u64, count: u32) -> u32 {
        if tid == 0 || count == 0 || self.mode != MODE_BUKA || self.times != 0 {
            return KUNCI_SALAH;
        }
        self.mode = MODE_EKSKLUSIF;
        self.holder = tid;
        self.times = count;
        self.shared = 0;
        KUNCI_OK
    }

    fn bangun_bersama(&mut self, n: u32, jumlah: u32) -> u32 {
        if n == 0 || jumlah == 0 || self.mode != MODE_BUKA || self.times != 0 {
            return KUNCI_SALAH;
        }
        self.mode = MODE_BERSAMA;
        self.holder = 0;
        self.shared = jumlah as u64;
        self.times = jumlah;
        KUNCI_OK
    }
}

// --- pintu C ABI ------------------------------------------------------------

/// Ambil kunci. Kembalikan `KUNCI_OK` (diperoleh), `KUNCI_BLOK` (harus blokir,
/// keadaan belum berubah), atau `KUNCI_SALAH`.
///
/// # Safety
/// `st` harus menunjuk `KeadaanKunci` hidup yang selaras, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_masuk(
    st: *mut KeadaanKunci,
    tid: u64,
    mode: u32,
    behavior: u32,
) -> u32 {
    bump(Counter::Masuk);
    if st.is_null() || mode == MODE_BUKA || mode > MODE_EKSKLUSIF || behavior > BEHAVIOR_BIGLOCK {
        bump(Counter::TolakMode);
        return KUNCI_SALAH;
    }
    let s = &mut *st;
    if !mode_sah(s.mode) {
        bump(Counter::TolakMode);
        return KUNCI_SALAH;
    }
    match s.mode {
        MODE_BUKA => {
            if s.holder != 0 || s.shared_holders != 0 || s.times_locked != 0 {
                bump(Counter::TolakUrutan);
                return KUNCI_SALAH;
            }
            if mode == MODE_EKSKLUSIF {
                s.mode = MODE_EKSKLUSIF;
                s.holder = tid;
                s.shared_holders = 0;
            } else {
                s.mode = MODE_BERSAMA;
                s.holder = 0;
                s.shared_holders = 1;
            }
            s.times_locked = 1;
            KUNCI_OK
        }
        MODE_EKSKLUSIF => {
            if s.holder == 0 || s.shared_holders != 0 {
                bump(Counter::TolakUrutan);
                return KUNCI_SALAH;
            }
            if s.holder != tid {
                KUNCI_BLOK
            } else {
                s.times_locked = s.times_locked.wrapping_add(1);
                KUNCI_OK
            }
        }
        MODE_BERSAMA => {
            if behavior != BEHAVIOR_BIASA {
                bump(Counter::TolakMode);
                return KUNCI_SALAH;
            }
            if s.holder != 0 {
                bump(Counter::TolakUrutan);
                return KUNCI_SALAH;
            }
            if mode == MODE_EKSKLUSIF {
                // Peningkatan bersama→eksklusif: blokir.
                KUNCI_BLOK
            } else {
                if s.shared_holders == 0 || s.times_locked == 0 {
                    bump(Counter::TolakUrutan);
                    return KUNCI_SALAH;
                }
                s.times_locked = s.times_locked.wrapping_add(1);
                s.shared_holders = s.shared_holders.wrapping_add(1);
                KUNCI_OK
            }
        }
        _ => {
            bump(Counter::TolakMode);
            KUNCI_SALAH
        }
    }
}

/// Lepas satu tingkat kunci. Tulis mode-sebelum ke `out_prev` (bila bukan null).
/// Kembalikan `KELUAR_TAHAN`, `KELUAR_LEPAS`, atau `KELUAR_SALAH`.
///
/// # Safety
/// `st`/`out_prev` harus menunjuk memori hidup yang selaras, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_keluar(
    st: *mut KeadaanKunci,
    tid: u64,
    out_prev: *mut u32,
) -> u32 {
    bump(Counter::Keluar);
    if st.is_null() {
        bump(Counter::TolakUrutan);
        return KELUAR_SALAH;
    }
    let s = &mut *st;
    // Laporkan mode-sebelum lebih awal, termasuk pada jalur gagal, agar
    // deterministik dan setara model (pemanggil C++ tetap VERIFY pada SALAH).
    if !out_prev.is_null() {
        *out_prev = s.mode;
    }
    if !mode_sah(s.mode) || s.mode == MODE_BUKA || s.times_locked == 0 {
        bump(Counter::TolakUrutan);
        return KELUAR_SALAH;
    }
    let prev = s.mode;
    // Validasi SEBELUM mutasi apa pun.
    match prev {
        MODE_EKSKLUSIF => {
            if s.holder != tid || s.shared_holders != 0 {
                bump(Counter::TolakUrutan);
                return KELUAR_SALAH;
            }
        }
        MODE_BERSAMA => {
            if s.holder != 0 || s.shared_holders == 0 {
                bump(Counter::TolakUrutan);
                return KELUAR_SALAH;
            }
        }
        _ => {
            bump(Counter::TolakUrutan);
            return KELUAR_SALAH;
        }
    }
    s.times_locked -= 1;
    if prev == MODE_EKSKLUSIF {
        if s.times_locked == 0 {
            s.holder = 0;
        }
    } else {
        s.shared_holders -= 1;
    }
    if s.times_locked == 0 {
        if prev == MODE_EKSKLUSIF {
            if s.holder != 0 {
                bump(Counter::TolakUrutan);
                return KELUAR_SALAH;
            }
        } else if s.shared_holders != 0 {
            bump(Counter::TolakUrutan);
            return KELUAR_SALAH;
        }
        s.mode = MODE_BUKA;
        KELUAR_LEPAS
    } else {
        KELUAR_TAHAN
    }
}

/// Terapkan pembangunan penunggu bersama: `n` penunggu, total `jumlah` kunci.
///
/// # Safety
/// `st` harus menunjuk `KeadaanKunci` hidup, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_bangunkan_bersama(
    st: *mut KeadaanKunci,
    n: u32,
    jumlah: u32,
) -> u32 {
    bump(Counter::Bangunkan);
    if st.is_null() || n == 0 || jumlah == 0 {
        bump(Counter::TolakUrutan);
        return KUNCI_SALAH;
    }
    let s = &mut *st;
    if s.mode != MODE_BUKA || s.times_locked != 0 {
        bump(Counter::TolakUrutan);
        return KUNCI_SALAH;
    }
    s.mode = MODE_BERSAMA;
    s.holder = 0;
    s.shared_holders = jumlah as u64;
    s.times_locked = jumlah;
    KUNCI_OK
}

/// Terapkan pembangunan satu penunggu eksklusif dengan `count` kunci.
///
/// # Safety
/// `st` harus menunjuk `KeadaanKunci` hidup, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_bangunkan_tunggal(
    st: *mut KeadaanKunci,
    tid: u64,
    count: u32,
) -> u32 {
    bump(Counter::Bangunkan);
    if st.is_null() || tid == 0 || count == 0 {
        bump(Counter::TolakUrutan);
        return KUNCI_SALAH;
    }
    let s = &mut *st;
    if s.mode != MODE_BUKA || s.times_locked != 0 {
        bump(Counter::TolakUrutan);
        return KUNCI_SALAH;
    }
    s.mode = MODE_EKSKLUSIF;
    s.holder = tid;
    s.times_locked = count;
    s.shared_holders = 0;
    KUNCI_OK
}

/// BigLock: paksa lepas kunci eksklusif bila dipegang thread ini. Tulis jumlah
/// kunci untuk dipulihkan ke `out_restore`. Kembalikan mode-sebelum
/// (`MODE_EKSKLUSIF` bila dipaksa lepas, `MODE_BUKA` bila tidak), atau
/// `u32::MAX` bila masukan tak sah.
///
/// # Safety
/// `st`/`out_restore` harus menunjuk memori hidup, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_paksa(
    st: *mut KeadaanKunci,
    tid: u64,
    out_restore: *mut u32,
) -> u32 {
    bump(Counter::Paksa);
    if st.is_null() {
        bump(Counter::TolakUrutan);
        return u32::MAX;
    }
    let s = &mut *st;
    if !mode_sah(s.mode) {
        bump(Counter::TolakMode);
        return u32::MAX;
    }
    match s.mode {
        MODE_EKSKLUSIF => {
            if s.shared_holders != 0 || s.times_locked == 0 {
                bump(Counter::TolakUrutan);
                return u32::MAX;
            }
            if s.holder != tid {
                if !out_restore.is_null() {
                    *out_restore = 0;
                }
                MODE_BUKA
            } else {
                if !out_restore.is_null() {
                    *out_restore = s.times_locked;
                }
                s.holder = 0;
                s.times_locked = 0;
                s.mode = MODE_BUKA;
                MODE_EKSKLUSIF
            }
        }
        MODE_BUKA => {
            if !out_restore.is_null() {
                *out_restore = 0;
            }
            MODE_BUKA
        }
        _ => {
            bump(Counter::TolakUrutan);
            u32::MAX
        }
    }
}

/// BigLock: apakah `pulihkan` perlu memblokir lebih dulu? 1 = ya, 0 = tidak,
/// 2 = tak sah.
///
/// # Safety
/// `st` harus menunjuk `KeadaanKunci` hidup, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_pulih_perlu_blok(st: *const KeadaanKunci, tid: u64) -> u32 {
    if st.is_null() {
        return 2;
    }
    let s = &*st;
    if !mode_sah(s.mode) {
        return 2;
    }
    match s.mode {
        MODE_EKSKLUSIF => {
            if s.times_locked == 0 || s.shared_holders != 0 {
                2
            } else if s.holder == tid {
                0
            } else {
                1
            }
        }
        MODE_BUKA => 0,
        _ => 2,
    }
}

/// BigLock: pulihkan kunci eksklusif dengan `count` tingkat. `did_block` bukan
/// nol bila pemanggil sudah memblokir (keadaan diisi oleh pembangun).
///
/// # Safety
/// `st` harus menunjuk `KeadaanKunci` hidup, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_pulihkan(
    st: *mut KeadaanKunci,
    tid: u64,
    count: u32,
    did_block: u32,
) -> u32 {
    bump(Counter::Pulih);
    if st.is_null() || count == 0 {
        bump(Counter::TolakUrutan);
        return KUNCI_SALAH;
    }
    let s = &mut *st;
    if !mode_sah(s.mode) {
        bump(Counter::TolakMode);
        return KUNCI_SALAH;
    }
    if did_block != 0 {
        if s.mode != MODE_EKSKLUSIF || s.holder != tid || s.times_locked == 0 {
            bump(Counter::TolakUrutan);
            return KUNCI_SALAH;
        }
        return KUNCI_OK;
    }
    match s.mode {
        MODE_BUKA => {
            if s.holder != 0 || s.shared_holders != 0 || s.times_locked != 0 {
                bump(Counter::TolakUrutan);
                return KUNCI_SALAH;
            }
            s.mode = MODE_EKSKLUSIF;
            s.times_locked = count;
            s.holder = tid;
            KUNCI_OK
        }
        MODE_EKSKLUSIF => {
            if s.holder != tid || s.shared_holders != 0 || s.times_locked == 0 {
                bump(Counter::TolakUrutan);
                return KUNCI_SALAH;
            }
            s.times_locked = s.times_locked.wrapping_add(count);
            KUNCI_OK
        }
        _ => {
            bump(Counter::TolakUrutan);
            KUNCI_SALAH
        }
    }
}

/// Apakah kunci sedang dipegang? (1 = ya, 0 = tidak). Gagal-tertutup: null atau
/// mode rusak → 1.
///
/// # Safety
/// `st` harus menunjuk `KeadaanKunci` hidup, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_terkunci(st: *const KeadaanKunci) -> u32 {
    if st.is_null() {
        return 1;
    }
    let s = &*st;
    if !mode_sah(s.mode) || s.mode != MODE_BUKA {
        1
    } else {
        0
    }
}

/// Apakah kunci eksklusif dipegang thread ini? 1 = ya, 0 = tidak, 2 = tak sah
/// (dipakai pada kunci bersama).
///
/// # Safety
/// `st` harus menunjuk `KeadaanKunci` hidup, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_eksklusif_oleh(st: *const KeadaanKunci, tid: u64) -> u32 {
    if st.is_null() {
        return 2;
    }
    let s = &*st;
    match s.mode {
        MODE_BUKA => 0,
        MODE_EKSKLUSIF => {
            if s.holder == tid {
                1
            } else {
                0
            }
        }
        MODE_BERSAMA => 2,
        _ => 2,
    }
}

/// Baca satu bidang keadaan. `u32::MAX` bila tak sah.
///
/// # Safety
/// `st` harus menunjuk `KeadaanKunci` hidup, atau null.
#[no_mangle]
pub unsafe extern "C" fn klatos_kunci_baca(st: *const KeadaanKunci, which: u32) -> u32 {
    if st.is_null() {
        return u32::MAX;
    }
    let s = &*st;
    if !mode_sah(s.mode) {
        return u32::MAX;
    }
    match which {
        WHICH_MODE => s.mode,
        WHICH_TIMES => s.times_locked,
        WHICH_SHARED => s.shared_holders as u32,
        WHICH_HOLDER_LO => (s.holder & 0xFFFF_FFFF) as u32,
        WHICH_HOLDER_HI => (s.holder >> 32) as u32,
        _ => u32::MAX,
    }
}

/// Baca penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_kunci_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTERS[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Nol-kan penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_kunci_reset() -> u32 {
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTERS[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    0
}

// --- laporan + selftest -----------------------------------------------------

/// Laporan matriks otoritas mesin keadaan kunci (64 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosKunciReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub dif_checks: u32,
    pub dif_beda: u32,
    pub prod_masuk: u32,
    pub prod_keluar: u32,
    pub prod_bangunkan: u32,
    pub prod_paksa: u32,
    pub prod_pulih: u32,
    pub tolak_mode: u32,
    pub tolak_urutan: u32,
    pub status: i32,
}
const _: [(); 64] = [(); core::mem::size_of::<KlatosKunciReport>()];

struct Matrix {
    checks: u32,
    passed: u32,
    neg_checks: u32,
    neg_passed: u32,
    dif_checks: u32,
    dif_beda: u32,
}
impl Matrix {
    const fn new() -> Self {
        Matrix { checks: 0, passed: 0, neg_checks: 0, neg_passed: 0, dif_checks: 0, dif_beda: 0 }
    }
    fn catat(&mut self, l: bool) {
        self.checks += 1;
        if l {
            self.passed += 1;
        }
    }
    fn neg(&mut self, l: bool) {
        self.neg_checks += 1;
        if l {
            self.neg_passed += 1;
        }
    }
    fn dif(&mut self, l: bool) {
        self.dif_checks += 1;
        if !l {
            self.dif_beda += 1;
        }
    }
}

/// Pembungkus `klatos_kunci_masuk` untuk selftest.
unsafe fn masuk(st: *mut KeadaanKunci, tid: u64, mode: u32, behavior: u32) -> u32 {
    klatos_kunci_masuk(st, tid, mode, behavior)
}
unsafe fn keluar(st: *mut KeadaanKunci, tid: u64) -> (u32, u32) {
    let mut prev = 0u32;
    let rc = klatos_kunci_keluar(st, tid, &mut prev);
    (rc, prev)
}

fn run_matrix() -> KlatosKunciReport {
    let mut m = Matrix::new();
    let _ = klatos_kunci_reset();
    m.catat(REPORT_TAG == 0x4B4C_4154_4D54_5831);
    m.catat(MODE_BUKA == 0 && MODE_BERSAMA == 1 && MODE_EKSKLUSIF == 2);
    m.catat(BEHAVIOR_BIASA == 0 && BEHAVIOR_BIGLOCK == 1);
    m.catat(core::mem::size_of::<KeadaanKunci>() == 24);

    // A) Keadaan awal: tak terkunci.
    let mut st = KeadaanKunci::kosong();
    let p = &mut st as *mut KeadaanKunci;
    m.catat(unsafe { klatos_kunci_terkunci(p) } == 0);
    m.catat(unsafe { klatos_kunci_eksklusif_oleh(p, 0x1000) } == 0);

    // B) Ambil eksklusif pada keadaan buka.
    m.catat(unsafe { masuk(p, 0x1000, MODE_EKSKLUSIF, BEHAVIOR_BIASA) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_terkunci(p) } == 1);
    m.catat(unsafe { klatos_kunci_eksklusif_oleh(p, 0x1000) } == 1);
    m.catat(unsafe { klatos_kunci_eksklusif_oleh(p, 0x2000) } == 0);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_MODE) } == MODE_EKSKLUSIF);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_TIMES) } == 1);

    // C) Rekursif eksklusif oleh pemegang yang sama.
    m.catat(unsafe { masuk(p, 0x1000, MODE_EKSKLUSIF, BEHAVIOR_BIASA) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_TIMES) } == 2);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_SHARED) } == 0);

    // D) Thread lain harus memblokir (keadaan tak berubah).
    m.catat(unsafe { masuk(p, 0x2000, MODE_EKSKLUSIF, BEHAVIOR_BIASA) } == KUNCI_BLOK);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_TIMES) } == 2);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_HOLDER_LO) } == 0x1000);

    // E) Keluar bertingkat.
    let (r1, p1) = unsafe { keluar(p, 0x1000) };
    m.catat(r1 == KELUAR_TAHAN && p1 == MODE_EKSKLUSIF);
    let (r2, _) = unsafe { keluar(p, 0x1000) };
    m.catat(r2 == KELUAR_LEPAS);
    m.catat(unsafe { klatos_kunci_terkunci(p) } == 0);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_HOLDER_LO) } == 0);

    // F) Bersama: dua thread berbagi.
    m.catat(unsafe { masuk(p, 0x3000, MODE_BERSAMA, BEHAVIOR_BIASA) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_MODE) } == MODE_BERSAMA);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_SHARED) } == 1);
    m.catat(unsafe { masuk(p, 0x4000, MODE_BERSAMA, BEHAVIOR_BIASA) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_SHARED) } == 2);
    m.catat(unsafe { klatos_kunci_baca(p, WHICH_TIMES) } == 2);
    // Peningkatan bersama→eksklusif harus memblokir.
    m.catat(unsafe { masuk(p, 0x3000, MODE_EKSKLUSIF, BEHAVIOR_BIASA) } == KUNCI_BLOK);
    let (r3, p3) = unsafe { keluar(p, 0x4000) };
    m.catat(r3 == KELUAR_TAHAN && p3 == MODE_BERSAMA);
    let (r4, _) = unsafe { keluar(p, 0x3000) };
    m.catat(r4 == KELUAR_LEPAS);
    m.catat(unsafe { klatos_kunci_terkunci(p) } == 0);

    // G) Pembangunan penunggu eksklusif (mensimulasikan waker).
    let mut stg = KeadaanKunci::kosong();
    let pg = &mut stg as *mut KeadaanKunci;
    m.catat(unsafe { klatos_kunci_bangunkan_tunggal(pg, 0x5000, 3) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_baca(pg, WHICH_MODE) } == MODE_EKSKLUSIF);
    m.catat(unsafe { klatos_kunci_baca(pg, WHICH_TIMES) } == 3);
    m.catat(unsafe { klatos_kunci_baca(pg, WHICH_HOLDER_LO) } == 0x5000);
    m.catat(unsafe { klatos_kunci_eksklusif_oleh(pg, 0x5000) } == 1);

    // H) Pembangunan penunggu bersama (total kunci terakumulasi).
    let mut stb = KeadaanKunci::kosong();
    let pb = &mut stb as *mut KeadaanKunci;
    m.catat(unsafe { klatos_kunci_bangunkan_bersama(pb, 3, 4) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_baca(pb, WHICH_MODE) } == MODE_BERSAMA);
    m.catat(unsafe { klatos_kunci_baca(pb, WHICH_SHARED) } == 4);
    m.catat(unsafe { klatos_kunci_baca(pb, WHICH_TIMES) } == 4);

    // I) BigLock: paksa lepas + pulihkan.
    let mut stp = KeadaanKunci::kosong();
    let pp = &mut stp as *mut KeadaanKunci;
    m.catat(unsafe { masuk(pp, 0x6000, MODE_EKSKLUSIF, BEHAVIOR_BIGLOCK) } == KUNCI_OK);
    m.catat(unsafe { masuk(pp, 0x6000, MODE_EKSKLUSIF, BEHAVIOR_BIGLOCK) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_baca(pp, WHICH_TIMES) } == 2);
    let mut restore = 0u32;
    let rm = unsafe { klatos_kunci_paksa(pp, 0x6000, &mut restore) };
    m.catat(rm == MODE_EKSKLUSIF && restore == 2);
    m.catat(unsafe { klatos_kunci_terkunci(pp) } == 0);
    // pulih_perlu_blok: buka → 0.
    m.catat(unsafe { klatos_kunci_pulih_perlu_blok(pp, 0x6000) } == 0);
    m.catat(unsafe { klatos_kunci_pulihkan(pp, 0x6000, restore, 0) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_baca(pp, WHICH_TIMES) } == 2);
    m.catat(unsafe { klatos_kunci_baca(pp, WHICH_HOLDER_LO) } == 0x6000);
    // pemulihan aditif oleh pemegang yang sama.
    m.catat(unsafe { klatos_kunci_pulihkan(pp, 0x6000, 1, 0) } == KUNCI_OK);
    m.catat(unsafe { klatos_kunci_baca(pp, WHICH_TIMES) } == 3);
    // paksa oleh thread lain: tidak melepas.
    let mut r2b = 0u32;
    let rm2 = unsafe { klatos_kunci_paksa(pp, 0x7000, &mut r2b) };
    m.catat(rm2 == MODE_BUKA && r2b == 0);
    m.catat(unsafe { klatos_kunci_terkunci(pp) } == 1);

    // --- Negatif -------------------------------------------------------------
    // Mode buka/tak sah ditolak.
    m.neg(unsafe { masuk(p, 0x1000, MODE_BUKA, BEHAVIOR_BIASA) } == KUNCI_SALAH);
    m.neg(unsafe { masuk(p, 0x1000, 7, BEHAVIOR_BIASA) } == KUNCI_SALAH);
    m.neg(unsafe { masuk(p, 0x1000, MODE_EKSKLUSIF, 9) } == KUNCI_SALAH);
    // Pointer null ditolak fail-closed.
    m.neg(unsafe { masuk(core::ptr::null_mut(), 0x1000, MODE_EKSKLUSIF, 0) } == KUNCI_SALAH);
    m.neg(unsafe { klatos_kunci_terkunci(core::ptr::null()) } == 1);
    // Keluar saat belum terkunci ditolak.
    let mut stn = KeadaanKunci::kosong();
    let pn = &mut stn as *mut KeadaanKunci;
    m.neg(unsafe { keluar(pn, 0x1000).0 } == KELUAR_SALAH);
    // Keluar oleh thread yang bukan pemegang ditolak.
    let mut stx = KeadaanKunci::kosong();
    let px = &mut stx as *mut KeadaanKunci;
    m.catat(unsafe { masuk(px, 0x1000, MODE_EKSKLUSIF, BEHAVIOR_BIASA) } == KUNCI_OK);
    m.neg(unsafe { keluar(px, 0x2000).0 } == KELUAR_SALAH);
    // BigLock tak boleh bersama: keadaan sudah bersama, lalu minta bersama
    // dengan perilaku BigLock -> ditolak (setara VERIFY(m_behavior==Regular)).
    let mut sts = KeadaanKunci::kosong();
    let ps = &mut sts as *mut KeadaanKunci;
    m.catat(unsafe { masuk(ps, 0x1000, MODE_BERSAMA, BEHAVIOR_BIASA) } == KUNCI_OK);
    m.neg(unsafe { masuk(ps, 0x1000, MODE_BERSAMA, BEHAVIOR_BIGLOCK) } == KUNCI_SALAH);
    // Bangun tanpa penunggu ditolak.
    let mut sty = KeadaanKunci::kosong();
    let py = &mut sty as *mut KeadaanKunci;
    m.neg(unsafe { klatos_kunci_bangunkan_tunggal(py, 0x1000, 0) } == KUNCI_SALAH);
    m.neg(unsafe { klatos_kunci_bangunkan_bersama(py, 0, 1) } == KUNCI_SALAH);
    // baca bidang tak sah.
    m.neg(unsafe { klatos_kunci_baca(pn, 99) } == u32::MAX);
    // pulihkan count 0 ditolak.
    m.neg(unsafe { klatos_kunci_pulihkan(pn, 0x1000, 0, 0) } == KUNCI_SALAH);

    // --- Diferensial vs model ORISINAL --------------------------------------
    // Urutan operasi deterministik (LCG) atas (tid, mode, behavior).
    let mut st_d = KeadaanKunci::kosong();
    let pd = &mut st_d as *mut KeadaanKunci;
    let mut mdl = ModelKunci::baru();
    let mut seed: u64 = 0x1234_5678_9ABC_DEF1;
    let tids = [0x1000u64, 0x2000, 0x3000];
    let mut i = 0u32;
    while i < 64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let op = ((seed >> 33) % 3) as u32;
        let tid = tids[((seed >> 40) % 3) as usize];
        let mode = if ((seed >> 20) & 1) == 0 { MODE_EKSKLUSIF } else { MODE_BERSAMA };
        match op {
            0 => {
                let rg = unsafe { masuk(pd, tid, mode, BEHAVIOR_BIASA) };
                let rm = mdl.masuk(tid, mode, BEHAVIOR_BIASA);
                m.dif(rg == rm);
            }
            1 => {
                let (rg, pg2) = unsafe { keluar(pd, tid) };
                let (rm, pm2) = mdl.keluar(tid);
                m.dif(rg == rm && pg2 == pm2);
            }
            _ => {
                // Bangun hanya bila model & pintu sama-sama kosong.
                let d_buka = unsafe { klatos_kunci_baca(pd, WHICH_MODE) } == MODE_BUKA;
                if mdl.mode == MODE_BUKA && d_buka {
                    let cnt = ((seed >> 8) % 3 + 1) as u32;
                    if (seed >> 60) & 1 == 0 {
                        let rg = unsafe { klatos_kunci_bangunkan_tunggal(pd, tid, cnt) };
                        let rm = mdl.bangun_tunggal(tid, cnt);
                        m.dif(rg == rm);
                    } else {
                        let nn = ((seed >> 12) % 3 + 1) as u32;
                        let rg = unsafe { klatos_kunci_bangunkan_bersama(pd, nn, cnt) };
                        let rm = mdl.bangun_bersama(nn, cnt);
                        m.dif(rg == rm);
                    }
                } else {
                    m.dif(true);
                }
            }
        }
        m.dif(unsafe { klatos_kunci_baca(pd, WHICH_MODE) } == mdl.mode);
        m.dif(unsafe { klatos_kunci_baca(pd, WHICH_TIMES) } == mdl.times);
        m.dif(unsafe { klatos_kunci_baca(pd, WHICH_SHARED) } == mdl.shared as u32);
        m.dif(unsafe { klatos_kunci_baca(pd, WHICH_HOLDER_LO) } == (mdl.holder & 0xFFFF_FFFF) as u32);
        i += 1;
    }

    // Rekam penghitung SEBELUM reset (reset menghapusnya).
    let c_masuk = klatos_kunci_counter(Counter::Masuk as u32);
    let c_keluar = klatos_kunci_counter(Counter::Keluar as u32);
    let c_bangun = klatos_kunci_counter(Counter::Bangunkan as u32);
    let c_paksa = klatos_kunci_counter(Counter::Paksa as u32);
    let c_pulih = klatos_kunci_counter(Counter::Pulih as u32);
    let c_tmode = klatos_kunci_counter(Counter::TolakMode as u32);
    let c_turut = klatos_kunci_counter(Counter::TolakUrutan as u32);
    let _ = klatos_kunci_reset();

    let status = if m.checks == m.passed && m.neg_passed == m.neg_checks && m.dif_beda == 0 {
        0
    } else {
        -1
    };
    KlatosKunciReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        dif_checks: m.dif_checks,
        dif_beda: m.dif_beda,
        prod_masuk: c_masuk,
        prod_keluar: c_keluar,
        prod_bangunkan: c_bangun,
        prod_paksa: c_paksa,
        prod_pulih: c_pulih,
        tolak_mode: c_tmode,
        tolak_urutan: c_turut,
        status,
    }
}

/// Matriks otoritas mesin keadaan kunci.
#[no_mangle]
pub extern "C" fn klatos_kunci_selftest() -> KlatosKunciReport {
    run_matrix()
}

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4D54_5831);
        assert!(MODE_BUKA == 0 && MODE_BERSAMA == 1 && MODE_EKSKLUSIF == 2);
        assert!(core::mem::size_of::<KeadaanKunci>() == 24);
        assert!(core::mem::size_of::<KlatosKunciReport>() == 64);
        assert!(core::mem::offset_of!(KlatosKunciReport, status) == 60);
    };
}
