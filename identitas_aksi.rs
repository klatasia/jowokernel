// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! **KEPEMILIKAN IDENTITAS `Credentials`** (Rust).

#![allow(unsafe_code)]

use core::sync::atomic::{AtomicU32, Ordering, AtomicU64};

// ---------------------------------------------------------------------------
// Konstanta
// ---------------------------------------------------------------------------

/// Kapasitas tabel identitas (berbatas).
pub const MAX_ID: usize = 8192;

/// Slot teratas dicadangkan untuk selftest.
pub const CADANGAN: usize = 64;

/// Batas atas alokasi produksi (eksklusif).
pub const BATAS_ID: usize = MAX_ID - CADANGAN;

/// Jumlah gid tambahan maksimum (cermin `NGROUPS_MAX`).
pub const N_EXTRA: usize = 32;

/// Tag laporan: "KLATIDNT".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4944_4E54;

/// Sentinel "handle tidak sah".
pub const TAK_SAH: u32 = 0xFFFF_FFFF;

// Pemilih bidang identitas (cermin `Credentials::IDW_*`).
/// `m_uid`.
pub const IDW_UID: u32 = 0;
/// `m_gid`.
pub const IDW_GID: u32 = 1;
/// `m_euid`.
pub const IDW_EUID: u32 = 2;
/// `m_egid`.
pub const IDW_EGID: u32 = 3;
/// `m_suid`.
pub const IDW_SUID: u32 = 4;
/// `m_sgid`.
pub const IDW_SGID: u32 = 5;
/// `m_sid`.
pub const IDW_SID: u32 = 6;
/// `m_pgid`.
pub const IDW_PGID: u32 = 7;
/// Jumlah bidang.
pub const IDW_N: u32 = 8;

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_identitas_daftar`.
    ProdDaftar = 0,
    /// `klatos_identitas_lepas` yang berhasil.
    ProdLepas = 1,
    /// `klatos_identitas_baca` yang berhasil.
    ProdBaca = 2,
    /// `klatos_identitas_dalam_grup`.
    ProdGrup = 3,
    /// `klatos_identitas_extra_isi` yang berhasil.
    ProdExtra = 4,
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
// Tabel identitas — array paralel, atomik
// ---------------------------------------------------------------------------

static ID_GEN: [AtomicU32; MAX_ID] = [const { AtomicU32::new(0) }; MAX_ID];
static ID_HIDUP: [AtomicU32; MAX_ID] = [const { AtomicU32::new(0) }; MAX_ID];
static ID_SCALAR: [[AtomicU32; IDW_N as usize]; MAX_ID] =
    [const { [const { AtomicU32::new(0) }; IDW_N as usize] }; MAX_ID];
static ID_EXTRA_LEN: [AtomicU32; MAX_ID] = [const { AtomicU32::new(0) }; MAX_ID];
static ID_EXTRA: [[AtomicU32; N_EXTRA]; MAX_ID] =
    [const { [const { AtomicU32::new(0) }; N_EXTRA] }; MAX_ID];

/// Kunci HANYA untuk alokasi/pelepasan slot.
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

/// Jumlah identitas hidup PRODUKSI (dijaga inkremental).
static HIDUP: AtomicU32 = AtomicU32::new(0);
static PUNCAK: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// Resolusi handle
// ---------------------------------------------------------------------------

#[inline]
fn id_sah(index: u32, gen: u32) -> bool {
    let i = index as usize;
    gen != 0 && i < MAX_ID && ID_HIDUP[i].load(Ordering::Acquire) != 0
        && ID_GEN[i].load(Ordering::Relaxed) == gen
}

// ---------------------------------------------------------------------------
// Inti identitas
// ---------------------------------------------------------------------------

fn daftar_impl(lo: usize, hi: usize) -> u64 {
    lock();
    let mut slot: Option<usize> = None;
    let mut i = lo;
    while i < hi {
        if ID_HIDUP[i].load(Ordering::Relaxed) == 0 {
            slot = Some(i);
            break;
        }
        i += 1;
    }
    let hasil = if let Some(i) = slot {
        let g = ID_GEN[i].load(Ordering::Relaxed).wrapping_add(1);
        if g == 0 {
            ID_HIDUP[i].store(0, Ordering::Release);
            0u64
        } else {
            let mut w = 0usize;
            while w < IDW_N as usize {
                ID_SCALAR[i][w].store(0, Ordering::Relaxed);
                w += 1;
            }
            ID_EXTRA_LEN[i].store(0, Ordering::Relaxed);
            ID_GEN[i].store(g, Ordering::Relaxed);
            ID_HIDUP[i].store(1, Ordering::Release);
            ((g as u64) << 32) | (i as u64)
        }
    } else {
        0u64
    };
    unlock();
    if hasil != 0 && ((hasil & 0xFFFF_FFFF) as usize) < BATAS_ID {
        let h = HIDUP.load(Ordering::Relaxed) + 1;
        HIDUP.store(h, Ordering::Relaxed);
        if h > PUNCAK.load(Ordering::Relaxed) {
            PUNCAK.store(h, Ordering::Relaxed);
        }
    }
    hasil
}

fn lepas_impl(index: u32, gen: u32) -> u32 {
    if !id_sah(index, gen) {
        return 0;
    }
    lock();
    if !id_sah(index, gen) {
        unlock();
        return 0;
    }
    let i = index as usize;
    ID_HIDUP[i].store(0, Ordering::Release);
    ID_EXTRA_LEN[i].store(0, Ordering::Relaxed);
    if i < BATAS_ID {
        let h = HIDUP.load(Ordering::Relaxed);
        if h > 0 {
            HIDUP.store(h - 1, Ordering::Relaxed);
        }
    }
    unlock();
    1
}

/// Isi `len` gid tambahan (dipotong ke `N_EXTRA`). 1 bila berhasil.
fn extra_isi_impl(index: u32, gen: u32, gids: &[u32]) -> u32 {
    if !id_sah(index, gen) {
        return 0;
    }
    let i = index as usize;
    let len = gids.len().min(N_EXTRA);
    let mut j = 0usize;
    while j < len {
        ID_EXTRA[i][j].store(gids[j], Ordering::Relaxed);
        j += 1;
    }
    ID_EXTRA_LEN[i].store(len as u32, Ordering::Release);
    1
}

/// `in_group`: `gid == primary` ATAU `gid ∈ extra`.
///
/// Memakai ulang keputusan P1.102 ([`crate::keamanan_aksi::dalam_grup`])
/// sehingga hanya ada SATU salinan aturan keanggotaan.
fn dalam_grup_impl(index: u32, gen: u32, gid: u32) -> u32 {
    if !id_sah(index, gen) {
        return TAK_SAH;
    }
    let i = index as usize;
    let primary = ID_SCALAR[i][IDW_GID as usize].load(Ordering::Relaxed);
    let len = (ID_EXTRA_LEN[i].load(Ordering::Acquire) as usize).min(N_EXTRA);
    let mut buf = [0u32; N_EXTRA];
    let mut j = 0usize;
    while j < len {
        buf[j] = ID_EXTRA[i][j].load(Ordering::Relaxed);
        j += 1;
    }
    crate::keamanan_aksi::dalam_grup(gid, primary, &buf[..len])
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Daftarkan identitas dengan delapan skalar. Mengembalikan
/// `(generation << 32) | index`, atau 0 bila tabel penuh.
#[no_mangle]
pub extern "C" fn klatos_identitas_daftar(
    uid: u32,
    gid: u32,
    euid: u32,
    egid: u32,
    suid: u32,
    sgid: u32,
    sid: u32,
    pgid: u32,
) -> u64 {
    bump(Counter::ProdDaftar);
    let h = daftar_impl(0, BATAS_ID);
    if h == 0 {
        return 0;
    }
    let idx = (h & 0xFFFF_FFFF) as u32;
    let gen = (h >> 32) as u32;
    if !id_sah(idx, gen) {
        return 0;
    }
    let i = idx as usize;
    let v = [uid, gid, euid, egid, suid, sgid, sid, pgid];
    let mut w = 0usize;
    while w < IDW_N as usize {
        ID_SCALAR[i][w].store(v[w], Ordering::Relaxed);
        w += 1;
    }
    h
}

/// Isi `len` gid tambahan dari `ptr`. 1 bila berhasil.
///
/// # Safety
/// Pemanggil (C++) menjamin `ptr` sah untuk `len` `u32` selama pemanggilan,
/// atau `len == 0`/`ptr == null`.
#[no_mangle]
pub unsafe extern "C" fn klatos_identitas_extra_isi(
    index: u32,
    gen: u32,
    ptr: *const u32,
    len: usize,
) -> u32 {
    if !id_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return 0;
    }
    let gids: &[u32] = if ptr.is_null() || len == 0 {
        &[]
    } else {
        // SAFETY: kontrak pemanggil di atas.
        unsafe { core::slice::from_raw_parts(ptr, len) }
    };
    bump(Counter::ProdExtra);
    extra_isi_impl(index, gen, gids)
}

/// Lepaskan identitas. 1 bila dihapus, 0 bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_identitas_lepas(index: u32, gen: u32) -> u32 {
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
pub extern "C" fn klatos_identitas_anggota(index: u32, gen: u32) -> u32 {
    id_sah(index, gen) as u32
}

/// Baca satu bidang identitas. `TAK_SAH` bila handle/`which` salah.
#[no_mangle]
pub extern "C" fn klatos_identitas_baca(index: u32, gen: u32, which: u32) -> u32 {
    if !id_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    if which >= IDW_N {
        return TAK_SAH;
    }
    bump(Counter::ProdBaca);
    ID_SCALAR[index as usize][which as usize].load(Ordering::Relaxed)
}

/// Panjang `extra_gids`. `TAK_SAH` bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_identitas_extra_len(index: u32, gen: u32) -> u32 {
    if !id_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    ID_EXTRA_LEN[index as usize].load(Ordering::Acquire)
}

/// Pointer ke `extra_gids` di tabel Rust (stabil selama slot hidup).
/// Null bila handle salah.
///
/// # Safety (sisi C++)
/// Pointer hanya boleh DIBACA, dan hanya selama slot masih hidup.
#[no_mangle]
pub extern "C" fn klatos_identitas_extra_ptr(index: u32, gen: u32) -> *const u32 {
    if !id_sah(index, gen) {
        return core::ptr::null();
    }
    let p: *const AtomicU32 = &ID_EXTRA[index as usize][0];
    p as *const u32
}

/// `Credentials::in_group(gid)`. 1/0; `TAK_SAH` bila handle salah.
#[no_mangle]
pub extern "C" fn klatos_identitas_dalam_grup(index: u32, gen: u32, gid: u32) -> u32 {
    if !id_sah(index, gen) {
        bump(Counter::ProdHandleSalah);
        return TAK_SAH;
    }
    bump(Counter::ProdGrup);
    dalam_grup_impl(index, gen, gid)
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_identitas_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Identitas hidup sekarang (rentang produksi).
#[no_mangle]
pub extern "C" fn klatos_identitas_hidup() -> u32 {
    HIDUP.load(Ordering::Relaxed)
}

/// Puncak identitas hidup.
#[no_mangle]
pub extern "C" fn klatos_identitas_puncak() -> u32 {
    PUNCAK.load(Ordering::Relaxed)
}

/// Kapasitas produksi tabel.
#[no_mangle]
pub extern "C" fn klatos_identitas_kapasitas() -> u32 {
    BATAS_ID as u32
}

/// Reset HANYA rentang produksi (dipakai audit; bukan produksi).
#[no_mangle]
pub extern "C" fn klatos_identitas_reset() -> u32 {
    lock();
    let mut i = 0;
    while i < BATAS_ID {
        ID_HIDUP[i].store(0, Ordering::Release);
        ID_EXTRA_LEN[i].store(0, Ordering::Relaxed);
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

struct Naif {
    scalar: [[u32; IDW_N as usize]; 16],
    extra_len: [usize; 16],
    extra: [[u32; N_EXTRA]; 16],
}

impl Naif {
    const fn baru() -> Self {
        Naif {
            scalar: [[0; IDW_N as usize]; 16],
            extra_len: [0; 16],
            extra: [[0; N_EXTRA]; 16],
        }
    }
    /// `gid == primary || extra.contains(gid)` — ditulis langsung dari C++.
    fn dalam_grup(&self, i: usize, gid: u32) -> u32 {
        if gid == self.scalar[i][IDW_GID as usize] {
            return 1;
        }
        let mut j = 0usize;
        while j < self.extra_len[i] {
            if self.extra[i][j] == gid {
                return 1;
            }
            j += 1;
        }
        0
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

/// Cermin `KlatosIdentitasReport` di `KlatSeam119.h`.
#[repr(C)]
pub struct KlatosIdentitasReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub dif_ukur: u32,
    pub dif_beda: u32,
    pub prod_daftar: u32,
    pub prod_lepas: u32,
    pub prod_baca: u32,
    pub prod_grup: u32,
    pub prod_extra: u32,
    pub prod_handle_salah: u32,
    pub status: i32,
    pub reserved: u32,
}

const _: [(); 56] = [(); core::mem::size_of::<KlatosIdentitasReport>()];

const LO: usize = BATAS_ID;
const HI: usize = MAX_ID;

fn reset_cadangan() {
    lock();
    let mut i = LO;
    while i < HI {
        ID_HIDUP[i].store(0, Ordering::Release);
        ID_GEN[i].store(0, Ordering::Relaxed);
        ID_EXTRA_LEN[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    unlock();
}

fn run_matrix() -> KlatosIdentitasReport {
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
    ck!(REPORT_TAG == 0x4B4C_4154_4944_4E54);
    ck!(MAX_ID == 8192);
    ck!(BATAS_ID == 8128);
    ck!(CADANGAN == 64);
    ck!(N_EXTRA == 32);
    ck!(IDW_N == 8);
    ck!(core::mem::size_of::<KlatosIdentitasReport>() == 56);
    ck!(klatos_identitas_kapasitas() == 8128);
    ck!(klatos_identitas_counter(99) == 0);

    // --- siklus hidup (peta) ----------------------------------------------
    reset_cadangan();
    {
        let h = daftar_impl(LO, HI);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        ck!(h != 0);
        ck!(idx >= LO as u32 && (idx as usize) < HI);
        ck!(gen == 1);
        ck!(id_sah(idx, gen));
        let i = idx as usize;
        let v = [1000u32, 2000, 1001, 2001, 1002, 2002, 7, 8];
        let mut w = 0usize;
        while w < IDW_N as usize {
            ID_SCALAR[i][w].store(v[w], Ordering::Relaxed);
            w += 1;
        }
        ck!(ID_SCALAR[i][IDW_UID as usize].load(Ordering::Relaxed) == 1000);
        ck!(ID_SCALAR[i][IDW_PGID as usize].load(Ordering::Relaxed) == 8);
        // extra
        let ex = [11u32, 22, 33];
        ck!(extra_isi_impl(idx, gen, &ex) == 1);
        ck!(ID_EXTRA_LEN[i].load(Ordering::Acquire) == 3);
        ck!(ID_EXTRA[i][0].load(Ordering::Relaxed) == 11);
        ck!(ID_EXTRA[i][2].load(Ordering::Relaxed) == 33);
        // dalam_grup
        ck!(dalam_grup_impl(idx, gen, 2000) == 1); // primary
        ck!(dalam_grup_impl(idx, gen, 22) == 1); // extra
        ck!(dalam_grup_impl(idx, gen, 999) == 0);
        ck!(lepas_impl(idx, gen) == 1);
        ck!(dalam_grup_impl(idx, gen, 2000) == TAK_SAH);
    }

    // --- negatif -----------------------------------------------------------
    ck!(klatos_identitas_lepas(0, 0) == 0);
    ck!(klatos_identitas_baca(0, 0, IDW_UID) == TAK_SAH);
    ck!(klatos_identitas_extra_len(0, 0) == TAK_SAH);
    ck!(klatos_identitas_dalam_grup(0, 0, 1) == TAK_SAH);
    ck!(klatos_identitas_anggota(0, 0) == 0);
    ck!(klatos_identitas_anggota(MAX_ID as u32, 1) == 0);
    ck!(klatos_identitas_anggota(0xFFFF_FFFF, 1) == 0);
    ck!(klatos_identitas_extra_ptr(0, 0).is_null());
    // `which` di luar rentang pada handle SAH bukan penolakan handle.
    {
        reset_cadangan();
        let h = daftar_impl(LO, HI);
        let idx = (h & 0xFFFF_FFFF) as u32;
        let gen = (h >> 32) as u32;
        let t0 = COUNTERS[Counter::ProdHandleSalah as usize].load(Ordering::Relaxed);
        ck!(klatos_identitas_baca(idx, gen, IDW_N) == TAK_SAH);
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
        let mut rng = Lcg(0x1D3A_9C7E_55F0_0001);
        let mut op = 0u32;
        while op < 40_000 {
            let r = rng.next();
            let slot = (r as usize) & 15;
            let idx = hh[slot];
            let gen = gg[slot];
            let i = idx as usize;
            // tulis satu bidang acak
            let which = ((r >> 4) % IDW_N) as usize;
            let val = (r >> 8).wrapping_mul(2654435761);
            ID_SCALAR[i][which].store(val, Ordering::Relaxed);
            naif.scalar[slot][which] = val;
            dif_ukur += 1;
            // extra acak
            let elen = ((r >> 12) % (N_EXTRA as u32 + 1)) as usize;
            let mut e = [0u32; N_EXTRA];
            let mut j = 0usize;
            while j < elen {
                e[j] = r.rotate_right((j as u32) & 31) ^ (j as u32);
                j += 1;
            }
            extra_isi_impl(idx, gen, &e[..elen]);
            naif.extra_len[slot] = elen;
            naif.extra[slot] = e;
            dif_ukur += 1;
            // baca seluruh bidang
            let mut w = 0u32;
            while w < IDW_N {
                let a = ID_SCALAR[i][w as usize].load(Ordering::Relaxed);
                let b = naif.scalar[slot][w as usize];
                dif_ukur += 1;
                if a != b {
                    dif_beda += 1;
                }
                w += 1;
            }
            // dalam_grup pada 4 gid
            let gids = [val, naif.scalar[slot][IDW_GID as usize], 0u32, 0xDEAD_BEEF];
            let mut g = 0usize;
            while g < 4 {
                let a = dalam_grup_impl(idx, gen, gids[g]);
                let b = naif.dalam_grup(slot, gids[g]);
                dif_ukur += 1;
                if a != b {
                    dif_beda += 1;
                }
                g += 1;
            }
            op += 1;
        }
        // Pemakaian ulang slot: generasi naik.
        let mut z = 0usize;
        while z < 16 {
            ck!(lepas_impl(hh[z], gg[z]) == 1);
            let h = daftar_impl(LO, HI);
            hh[z] = (h & 0xFFFF_FFFF) as u32;
            gg[z] = (h >> 32) as u32;
            ck!(gg[z] == 2);
            naif.scalar[z] = [0; IDW_N as usize];
            naif.extra_len[z] = 0;
            ck!(ID_EXTRA_LEN[hh[z] as usize].load(Ordering::Acquire) == 0);
            z += 1;
        }
        ck!(id_sah(hh[0], 1) == false);
        let mut y = 0usize;
        while y < 16 {
            lepas_impl(hh[y], gg[y]);
            y += 1;
        }
    }
    ck!(dif_ukur >= 200_000);
    ck!(dif_beda == 0);

    // --- konsistensi dengan keputusan P1.102 -------------------------------
    {
        let mut sama = true;
        let ex = [5u32, 9];
        for gid in [0u32, 5, 9, 4, 7] {
            if crate::keamanan_aksi::dalam_grup(gid, 5, &ex)
                != (if gid == 5 || gid == 9 { 1 } else { 0 })
            {
                sama = false;
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
            let i = idx as usize;
            ID_SCALAR[i][IDW_UID as usize].store(42, Ordering::Relaxed);
            ID_SCALAR[i][IDW_GID as usize].store(43, Ordering::Relaxed);
            let ex = [44u32];
            extra_isi_impl(idx, gen, &ex);
            let mut r = [0u32; 4];
            r[0] = ID_SCALAR[i][IDW_UID as usize].load(Ordering::Relaxed);
            r[1] = dalam_grup_impl(idx, gen, 43);
            r[2] = dalam_grup_impl(idx, gen, 44);
            r[3] = dalam_grup_impl(idx, gen, 45);
            lepas_impl(idx, gen);
            if pass == 0 {
                a = r;
            } else {
                b = r;
            }
            pass += 1;
        }
        ck!(a == b);
        ck!(a[0] == 42 && a[1] == 1 && a[2] == 1 && a[3] == 0);
    }

    KlatosIdentitasReport {
        tag: REPORT_TAG,
        checks,
        passed,
        dif_ukur,
        dif_beda,
        prod_daftar: COUNTERS[Counter::ProdDaftar as usize].load(Ordering::Relaxed),
        prod_lepas: COUNTERS[Counter::ProdLepas as usize].load(Ordering::Relaxed),
        prod_baca: COUNTERS[Counter::ProdBaca as usize].load(Ordering::Relaxed),
        prod_grup: COUNTERS[Counter::ProdGrup as usize].load(Ordering::Relaxed),
        prod_extra: COUNTERS[Counter::ProdExtra as usize].load(Ordering::Relaxed),
        prod_handle_salah: COUNTERS[Counter::ProdHandleSalah as usize].load(Ordering::Relaxed),
        status: if checks == passed { 0 } else { -1 },
        reserved: 0,
    }
}

/// Selftest kepemilikan identitas `Credentials`.
#[no_mangle]
pub extern "C" fn klatos_identitas_selftest() -> KlatosIdentitasReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4944_4E54);
        assert!(MAX_ID == 8192);
        assert!(CADANGAN == 64);
        assert!(BATAS_ID == 8128);
        assert!(N_EXTRA == 32);
        assert!(IDW_UID == 0 && IDW_GID == 1 && IDW_EUID == 2 && IDW_EGID == 3);
        assert!(IDW_SUID == 4 && IDW_SGID == 5 && IDW_SID == 6 && IDW_PGID == 7);
        assert!(IDW_N == 8);
        assert!(Counter::ProdDaftar as usize == 0);
        assert!(Counter::ProdHandleSalah as usize == 5);
        assert!(N_COUNTER == 6);
        assert!(core::mem::size_of::<KlatosIdentitasReport>() == 56);
        assert!(core::mem::align_of::<KlatosIdentitasReport>() == 8);
    };
}
