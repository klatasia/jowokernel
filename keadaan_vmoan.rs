// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-8 — **KEPEMILIKAN KEADAAN `AnonymousVMObject`** (Rust).

#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering, AtomicU64};

// ---------------------------------------------------------------------------
// Konstanta
// ---------------------------------------------------------------------------

/// Kapasitas tabel keadaan (berbatas).
pub const MAX_VMOAN: usize = 16384;

/// Slot teratas dicadangkan untuk selftest.
pub const CADANGAN: usize = 64;

/// Batas atas alokasi produksi (eksklusif).
pub const BATAS_VMOAN: usize = MAX_VMOAN - CADANGAN;

/// Tag laporan: "KLATVMSA".
pub const REPORT_TAG: u64 = 0x4B4C_4154_564D_5341;

/// Sentinel "handle tidak sah".
pub const TAK_SAH: u32 = 0xFFFF_FFFF;

// Pemilih bidang (cermin `AnonymousVMObject::VW_*`).
/// `m_purgeable`.
pub const VW_PURGEABLE: u32 = 0;
/// `m_volatile`.
pub const VW_VOLATILE: u32 = 1;
/// `m_was_purged`.
pub const VW_WAS_PURGED: u32 = 2;
/// Jumlah bidang.
pub const VW_N: u32 = 3;

// Kode transisi — cermin `vmo_anon::VOL_TRANSI_*` (satu-satunya definisi).
/// Tidak ada perubahan.
pub const VOL_TANPA: u32 = 0;
/// Menjadi volatile.
pub const VOL_KE_VOLATILE: u32 = 1;
/// Menjadi non-volatile.
pub const VOL_KE_NONVOLATILE: u32 = 2;

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_vmoan_state_daftar`.
    ProdDaftar = 0,
    /// `klatos_vmoan_state_lepas` yang berhasil.
    ProdLepas = 1,
    /// `klatos_vmoan_state_baca` yang berhasil.
    ProdBaca = 2,
    /// `klatos_vmoan_state_tulis` yang berhasil.
    ProdTulis = 3,
    /// `klatos_vmoan_state_volatile` yang berhasil.
    ProdVolatile = 4,
    /// Handle basi / di luar tabel.
    ProdHandleSalah = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Tabel keadaan — array paralel, atomik
// ---------------------------------------------------------------------------

static VMO_GEN: [AtomicU32; MAX_VMOAN] = [const { AtomicU32::new(0) }; MAX_VMOAN];
static VMO_HIDUP: [AtomicU32; MAX_VMOAN] = [const { AtomicU32::new(0) }; MAX_VMOAN];
static VMO_PURGEABLE: [AtomicU32; MAX_VMOAN] = [const { AtomicU32::new(0) }; MAX_VMOAN];
static VMO_VOLATILE: [AtomicU32; MAX_VMOAN] = [const { AtomicU32::new(0) }; MAX_VMOAN];
static VMO_WAS_PURGED: [AtomicU32; MAX_VMOAN] = [const { AtomicU32::new(0) }; MAX_VMOAN];

/// Kunci HANYA untuk alokasi/pelepasan slot (bukan untuk baca/tulis keadaan).
static KUNCI: AtomicU32 = AtomicU32::new(0);
/// Status interupsi pemegang KUNCI; hanya pemegang yang menulis.
static KUNCI_FLAGS: AtomicU64 = AtomicU64::new(0);

fn lock() {
    // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
    let flags = crate::arch::simpan_dan_matikan_interupsi();
    while KUNCI.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    KUNCI_FLAGS.store(flags, Ordering::Relaxed);
}

fn unlock() {
    let flags = KUNCI_FLAGS.load(Ordering::Relaxed);
    KUNCI.store(0, Ordering::Release);
    crate::arch::pulihkan_interupsi(flags);
}

/// Jumlah objek hidup PRODUKSI (dijaga inkremental).
static HIDUP: AtomicU32 = AtomicU32::new(0);
static PUNCAK: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// Resolusi handle
// ---------------------------------------------------------------------------

#[inline]
fn vmo_sah(index: u32, gen: u32) -> bool {
    let i = index as usize;
    gen != 0 && i < MAX_VMOAN && VMO_HIDUP[i].load(Ordering::Acquire) != 0
        && VMO_GEN[i].load(Ordering::Relaxed) == gen
}

// ---------------------------------------------------------------------------
// Inti keadaan
// ---------------------------------------------------------------------------

fn daftar_impl(lo: usize, hi: usize) -> u64 {
    lock();
    let mut slot: Option<usize> = None;
    let mut i = lo;
    while i < hi {
        if VMO_HIDUP[i].load(Ordering::Relaxed) == 0 {
            slot = Some(i);
            break;
        }
        i += 1;
    }
    let hasil = if let Some(i) = slot {
        let g = VMO_GEN[i].load(Ordering::Relaxed).wrapping_add(1);
        if g == 0 {
            VMO_HIDUP[i].store(0, Ordering::Release);
            0u64
        } else {
            VMO_PURGEABLE[i].store(0, Ordering::Relaxed);
            VMO_VOLATILE[i].store(0, Ordering::Relaxed);
            VMO_WAS_PURGED[i].store(0, Ordering::Relaxed);
            VMO_GEN[i].store(g, Ordering::Relaxed);
            VMO_HIDUP[i].store(1, Ordering::Release);
            ((g as u64) << 32) | (i as u64)
        }
    } else {
        0u64
    };
    unlock();
    if hasil != 0 && ((hasil & 0xFFFF_FFFF) as usize) < BATAS_VMOAN {
        let h = HIDUP.load(Ordering::Relaxed) + 1;
        HIDUP.store(h, Ordering::Relaxed);
        if h > PUNCAK.load(Ordering::Relaxed) {
            PUNCAK.store(h, Ordering::Relaxed);
        }
    }
    hasil
}

fn lepas_impl(index: u32, gen: u32) -> u32 {
    if !vmo_sah(index, gen) {
        return 0;
    }
    lock();
    if !vmo_sah(index, gen) {
        unlock();
        return 0;
    }
    let i = index as usize;
    VMO_HIDUP[i].store(0, Ordering::Release);
    VMO_PURGEABLE[i].store(0, Ordering::Relaxed);
    VMO_VOLATILE[i].store(0, Ordering::Relaxed);
    VMO_WAS_PURGED[i].store(0, Ordering::Relaxed);
    if i < BATAS_VMOAN {
        let h = HIDUP.load(Ordering::Relaxed);
        if h > 0 {
            HIDUP.store(h - 1, Ordering::Relaxed);
        }
    }
    unlock();
    1
}

/// Baca satu bidang. `TAK_SAH` bila handle salah ATAU `which` di luar rentang.
fn baca_impl(index: u32, gen: u32, which: u32) -> u32 {
    if !vmo_sah(index, gen) || which >= VW_N {
        return TAK_SAH;
    }
    let i = index as usize;
    match which {
        VW_PURGEABLE => VMO_PURGEABLE[i].load(Ordering::Relaxed),
        VW_VOLATILE => VMO_VOLATILE[i].load(Ordering::Relaxed),
        _ => VMO_WAS_PURGED[i].load(Ordering::Relaxed),
    }
}

/// Tulis satu bidang (nilai dinormalisasi ke 0/1). 1 bila berhasil.
fn tulis_impl(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    if !vmo_sah(index, gen) || which >= VW_N {
        return 0;
    }
    let i = index as usize;
    let v = (val != 0) as u32;
    match which {
        VW_PURGEABLE => VMO_PURGEABLE[i].store(v, Ordering::Relaxed),
        VW_VOLATILE => VMO_VOLATILE[i].store(v, Ordering::Relaxed),
        _ => VMO_WAS_PURGED[i].store(v, Ordering::Relaxed),
    }
    1
}

/// Transisi volatile yang menulis keadaan: membaca `VW_VOLATILE`, memakai
/// keputusan [`crate::vmo_anon::volatile_transisi`], lalu menulis nilai baru
/// bila memang berubah. Mengembalikan kode `VOL_*`, atau `TAK_SAH`.
///
/// `VW_WAS_PURGED` SENGAJA tidak disentuh di sini: jalur non-volatile
/// `set_volatile` C++ memiliki satu cabang keluar yang tidak mereset
/// `m_was_purged` (lihat `AnonymousVMObject.cpp`), jadi C++ yang memutuskan
/// kapan mereset — modul ini hanya memegang penyimpanannya.
fn volatile_impl(index: u32, gen: u32, baru: u32) -> u32 {
    if !vmo_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    let sekarang = VMO_VOLATILE[i].load(Ordering::Relaxed);
    let b = (baru != 0) as u32;
    let transisi = crate::vmo_anon::volatile_transisi(sekarang, b);
    if transisi != VOL_TANPA {
        VMO_VOLATILE[i].store(b, Ordering::Relaxed);
    }
    transisi
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Daftarkan keadaan objek. Mengembalikan `(generation << 32) | index`, atau
/// 0 bila tabel penuh / generasi membungkus.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_daftar() -> u64 {
    bump(Counter::ProdDaftar);
    daftar_impl(0, BATAS_VMOAN)
}

/// Lepaskan keadaan objek. 1 bila dihapus, 0 bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_lepas(index: u32, gen: u32) -> u32 {
    let r = lepas_impl(index, gen);
    if r == 0 {
        bump(Counter::ProdHandleSalah);
    } else {
        bump(Counter::ProdLepas);
    }
    r
}

/// Keanggotaan tabel (1/0). Tanpa penghitung.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_anggota(index: u32, gen: u32) -> u32 {
    vmo_sah(index, gen) as u32
}

/// Baca satu bidang. `TAK_SAH` bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_baca(index: u32, gen: u32, which: u32) -> u32 {
    if !vmo_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    if which >= VW_N {
        // Handle sah, argumen di luar rentang: BUKAN penolakan handle.
        return TAK_SAH;
    }
    bump(Counter::ProdBaca);
    baca_impl(index, gen, which)
}

/// Tulis satu bidang. 1 bila berhasil.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_tulis(index: u32, gen: u32, which: u32, val: u32) -> u32 {
    if !vmo_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return 0;
    }
    if which >= VW_N {
        return 0;
    }
    bump(Counter::ProdTulis);
    tulis_impl(index, gen, which, val)
}

/// Transisi volatile + tulis keadaan. Mengembalikan kode `VOL_*`, atau
/// `TAK_SAH` bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_volatile(index: u32, gen: u32, baru: u32) -> u32 {
    if !vmo_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    bump(Counter::ProdVolatile);
    volatile_impl(index, gen, baru)
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Objek hidup sekarang (rentang produksi).
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_hidup() -> u32 {
    HIDUP.load(Ordering::Relaxed)
}

/// Puncak objek hidup.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_puncak() -> u32 {
    PUNCAK.load(Ordering::Relaxed)
}

/// Kapasitas produksi tabel.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_kapasitas() -> u32 {
    BATAS_VMOAN as u32
}

/// Reset HANYA rentang produksi (dipakai audit; bukan produksi).
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_reset() -> u32 {
    lock();
    let mut i = 0;
    while i < BATAS_VMOAN {
        VMO_HIDUP[i].store(0, Ordering::Release);
        VMO_PURGEABLE[i].store(0, Ordering::Relaxed);
        VMO_VOLATILE[i].store(0, Ordering::Relaxed);
        VMO_WAS_PURGED[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    HIDUP.store(0, Ordering::Release);
    PUNCAK.store(0, Ordering::Release);
    unlock();
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Oracle Naif — model independen semantik C++ historis
// ---------------------------------------------------------------------------

/// Model naif: tiga bidang sebagai array biasa, tanpa atomik.
struct Naif {
    purgeable: [u32; 32],
    volatile_: [u32; 32],
    was_purged: [u32; 32],
}

impl Naif {
    const fn baru() -> Self {
        Naif {
            purgeable: [0; 32],
            volatile_: [0; 32],
            was_purged: [0; 32],
        }
    }
    fn baca(&self, i: usize, which: u32) -> u32 {
        match which {
            VW_PURGEABLE => self.purgeable[i],
            VW_VOLATILE => self.volatile_[i],
            _ => self.was_purged[i],
        }
    }
    fn tulis(&mut self, i: usize, which: u32, val: u32) {
        let v = (val != 0) as u32;
        match which {
            VW_PURGEABLE => self.purgeable[i] = v,
            VW_VOLATILE => self.volatile_[i] = v,
            _ => self.was_purged[i] = v,
        }
    }
    /// Transisi volatile: tulis `volatile` bila berubah, kembalikan kode.
    fn volatile(&mut self, i: usize, baru: u32) -> u32 {
        let b = (baru != 0) as u32;
        let sekarang = self.volatile_[i];
        let t = if sekarang == b {
            VOL_TANPA
        } else if b != 0 {
            VOL_KE_VOLATILE
        } else {
            VOL_KE_NONVOLATILE
        };
        if t != VOL_TANPA {
            self.volatile_[i] = b;
        }
        t
    }
}

/// LCG deterministik (numerik saja, tidak ada alokasi).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosVmoanStateReport` di `KlatSeam119.h`.
#[repr(C)]
pub struct KlatosVmoanStateReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub dif_ukur: u32,
    pub dif_beda: u32,
    pub prod_daftar: u32,
    pub prod_lepas: u32,
    pub prod_baca: u32,
    pub prod_tulis: u32,
    pub prod_volatile: u32,
    pub prod_handle_salah: u32,
    pub status: i32,
    pub reserved: u32,
}

const _: [(); 56] = [(); core::mem::size_of::<KlatosVmoanStateReport>()];

/// Rentang slot uji (cadangan).
const LO: usize = BATAS_VMOAN;
const HI: usize = MAX_VMOAN;

/// Reset rentang CADANGAN saja (beserta generasinya).
fn reset_cadangan() {
    lock();
    let mut i = LO;
    while i < HI {
        VMO_HIDUP[i].store(0, Ordering::Release);
        VMO_GEN[i].store(0, Ordering::Relaxed);
        VMO_PURGEABLE[i].store(0, Ordering::Relaxed);
        VMO_VOLATILE[i].store(0, Ordering::Relaxed);
        VMO_WAS_PURGED[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    unlock();
}

fn run_matrix() -> KlatosVmoanStateReport {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;
    macro_rules! ck {
        ($e:expr) => {{
            checks += 1;
            if $e {
                passed += 1;
            }
        }};
    }

    // --- kepala & bentuk ---------------------------------------------------
    ck!(REPORT_TAG == 0x4B4C_4154_564D_5341);
    ck!(MAX_VMOAN == 16384);
    ck!(BATAS_VMOAN == 16320);
    ck!(CADANGAN == 64);
    ck!(VW_N == 3);
    ck!(VOL_TANPA == 0 && VOL_KE_VOLATILE == 1 && VOL_KE_NONVOLATILE == 2);
    ck!(core::mem::size_of::<KlatosVmoanStateReport>() == 56);
    ck!(klatos_vmoan_state_kapasitas() == 16320);
    ck!(klatos_vmoan_state_counter(99) == 0);

    // --- siklus hidup (peta) ----------------------------------------------
    reset_cadangan();
    {
        let h = daftar_impl(LO, HI);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        ck!(h != 0);
        ck!(idx >= LO as u32 && (idx as usize) < HI);
        ck!(gen == 1);
        ck!(vmo_sah(idx, gen));
        ck!(baca_impl(idx, gen, VW_PURGEABLE) == 0);
        ck!(baca_impl(idx, gen, VW_VOLATILE) == 0);
        ck!(baca_impl(idx, gen, VW_WAS_PURGED) == 0);
        // tulis + normalisasi
        ck!(tulis_impl(idx, gen, VW_PURGEABLE, 1) == 1);
        ck!(baca_impl(idx, gen, VW_PURGEABLE) == 1);
        ck!(tulis_impl(idx, gen, VW_VOLATILE, 7) == 1);
        ck!(baca_impl(idx, gen, VW_VOLATILE) == 1);
        ck!(tulis_impl(idx, gen, VW_WAS_PURGED, 1) == 1);
        // transisi volatile
        ck!(volatile_impl(idx, gen, 1) == VOL_TANPA); // sudah 1
        ck!(volatile_impl(idx, gen, 0) == VOL_KE_NONVOLATILE);
        ck!(baca_impl(idx, gen, VW_VOLATILE) == 0);
        ck!(volatile_impl(idx, gen, 1) == VOL_KE_VOLATILE);
        ck!(baca_impl(idx, gen, VW_VOLATILE) == 1);
        // lepas + handle basi
        ck!(lepas_impl(idx, gen) == 1);
        ck!(!vmo_sah(idx, gen));
        ck!(baca_impl(idx, gen, VW_PURGEABLE) == TAK_SAH);
    }

    // --- negatif -----------------------------------------------------------
    ck!(klatos_vmoan_state_lepas(0, 0) == 0);
    ck!(klatos_vmoan_state_baca(0, 0, VW_PURGEABLE) == TAK_SAH);
    ck!(klatos_vmoan_state_tulis(0, 0, VW_PURGEABLE, 1) == 0);
    ck!(klatos_vmoan_state_volatile(0, 0, 1) == TAK_SAH);
    ck!(klatos_vmoan_state_anggota(0, 0) == 0);
    ck!(klatos_vmoan_state_anggota(MAX_VMOAN as u32, 1) == 0);
    ck!(klatos_vmoan_state_anggota(0xFFFF_FFFF, 1) == 0);
    // `which` di luar rentang pada handle SAH bukan penolakan handle.
    {
        reset_cadangan();
        let h = daftar_impl(LO, HI);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        let t0 = COUNTERS[Counter::ProdHandleSalah as usize].load(Ordering::Relaxed);
        ck!(baca_impl(idx, gen, VW_N) == TAK_SAH);
        ck!(tulis_impl(idx, gen, VW_N, 1) == 0);
        ck!(COUNTERS[Counter::ProdHandleSalah as usize].load(Ordering::Relaxed) == t0);
        ck!(lepas_impl(idx, gen) == 1);
    }

    // --- diferensial vs oracle Naif (>= 200.000 kasus) ---------------------
    let mut dif_ukur: u32 = 0;
    let mut dif_beda: u32 = 0;
    {
        reset_cadangan();
        let mut naif = Naif::baru();
        let mut hh = [0u32; 16];
        let mut gg = [0u32; 16];
        let mut i = 0;
        while i < 16 {
            let h = daftar_impl(LO, HI);
            hh[i] = (h & 0xFFFF_FFFF) as u32;
            gg[i] = (h >> 32) as u32;
            i += 1;
        }
        let mut rng = Lcg(0x5EED_0F8A_1C3D_7721);
        let mut op = 0u32;
        while op < 60_000 {
            let r = rng.next();
            let slot = (r as usize) & 15;
            let aksi = (r >> 4) % 3;
            let idx = hh[slot];
            let gen = gg[slot];
            match aksi {
                0 => {
                    let which = (r >> 6) % VW_N;
                    let val = (r >> 9) & 1;
                    let a = tulis_impl(idx, gen, which, val);
                    naif.tulis(slot, which, val);
                    dif_ukur += 1;
                    if a != 1 {
                        dif_beda += 1;
                    }
                }
                1 => {
                    let baru = (r >> 7) & 1;
                    let a = volatile_impl(idx, gen, baru);
                    let b = naif.volatile(slot, baru);
                    dif_ukur += 1;
                    if a != b {
                        dif_beda += 1;
                    }
                }
                _ => {}
            }
            let mut w = 0u32;
            while w < VW_N {
                let a = baca_impl(idx, gen, w);
                let b = naif.baca(slot, w);
                dif_ukur += 1;
                if a != b {
                    dif_beda += 1;
                }
                w += 1;
            }
            op += 1;
        }
        // Pemakaian ulang slot: generasi naik, oracle ikut di-reset.
        let mut z = 0usize;
        while z < 16 {
            ck!(lepas_impl(hh[z], gg[z]) == 1);
            let h = daftar_impl(LO, HI);
            hh[z] = (h & 0xFFFF_FFFF) as u32;
            gg[z] = (h >> 32) as u32;
            ck!(gg[z] == 2);
            naif.purgeable[z] = 0;
            naif.volatile_[z] = 0;
            naif.was_purged[z] = 0;
            ck!(baca_impl(hh[z], gg[z], VW_PURGEABLE) == 0);
            ck!(baca_impl(hh[z], gg[z], VW_VOLATILE) == 0);
            z += 1;
        }
        ck!(vmo_sah(hh[0], 1) == false); // handle generasi lama ditolak
        let mut y = 0usize;
        while y < 16 {
            lepas_impl(hh[y], gg[y]);
            y += 1;
        }
    }
    ck!(dif_ukur >= 200_000);
    ck!(dif_beda == 0);

    // --- konsistensi dengan keputusan P1.68 --------------------------------
    //
    // Tabel transisi modul ini WAJIB sama dengan keputusan `vmo_anon`. Ini
    // membandingkan keduanya untuk seluruh pasangan keadaan.
    {
        let mut sama = true;
        for s in 0u32..2 {
            for b in 0u32..2 {
                if crate::vmo_anon::volatile_transisi(s, b)
                    != if s == b {
                        VOL_TANPA
                    } else if b != 0 {
                        VOL_KE_VOLATILE
                    } else {
                        VOL_KE_NONVOLATILE
                    }
                {
                    sama = false;
                }
            }
        }
        ck!(sama);
    }

    // --- determinisme ------------------------------------------------------
    {
        reset_cadangan();
        let mut a = [0u32; 4];
        let mut b = [0u32; 4];
        let mut pass = 0;
        while pass < 2 {
            let h = daftar_impl(LO, HI);
            let idx = (h & 0xFFFF_FFFF) as u32;
            let gen = (h >> 32) as u32;
            let mut r = [0u32; 4];
            r[0] = tulis_impl(idx, gen, VW_PURGEABLE, 1);
            r[1] = volatile_impl(idx, gen, 1);
            r[2] = tulis_impl(idx, gen, VW_WAS_PURGED, 1);
            r[3] = baca_impl(idx, gen, VW_VOLATILE);
            lepas_impl(idx, gen);
            if pass == 0 {
                a = r;
            } else {
                b = r;
            }
            pass += 1;
        }
        ck!(a == b);
        ck!(a[0] == 1 && a[1] == VOL_KE_VOLATILE && a[2] == 1 && a[3] == 1);
    }

    KlatosVmoanStateReport {
        tag: REPORT_TAG,
        checks,
        passed,
        dif_ukur,
        dif_beda,
        prod_daftar: COUNTERS[Counter::ProdDaftar as usize].load(Ordering::Relaxed),
        prod_lepas: COUNTERS[Counter::ProdLepas as usize].load(Ordering::Relaxed),
        prod_baca: COUNTERS[Counter::ProdBaca as usize].load(Ordering::Relaxed),
        prod_tulis: COUNTERS[Counter::ProdTulis as usize].load(Ordering::Relaxed),
        prod_volatile: COUNTERS[Counter::ProdVolatile as usize].load(Ordering::Relaxed),
        prod_handle_salah: COUNTERS[Counter::ProdHandleSalah as usize].load(Ordering::Relaxed),
        status: if checks == passed { 0 } else { -1 },
        reserved: 0,
    }
}

/// Selftest kepemilikan keadaan `AnonymousVMObject`.
#[no_mangle]
pub extern "C" fn klatos_vmoan_state_selftest() -> KlatosVmoanStateReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_564D_5341);
        assert!(MAX_VMOAN == 16384);
        assert!(CADANGAN == 64);
        assert!(BATAS_VMOAN == 16320);
        assert!(VW_PURGEABLE == 0 && VW_VOLATILE == 1 && VW_WAS_PURGED == 2);
        assert!(VW_N == 3);
        assert!(VOL_TANPA == 0 && VOL_KE_VOLATILE == 1 && VOL_KE_NONVOLATILE == 2);
        assert!(Counter::ProdDaftar as usize == 0);
        assert!(Counter::ProdHandleSalah as usize == 5);
        assert!(N_COUNTER == 6);
        assert!(core::mem::size_of::<KlatosVmoanStateReport>() == 56);
        assert!(core::mem::align_of::<KlatosVmoanStateReport>() == 8);
    };
}
