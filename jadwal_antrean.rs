// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.56 — **OTORITAS ANTREAN JALAN PENJADWAL** (Rust).


use core::sync::atomic::{AtomicU32, Ordering};

use crate::graf_proses_benang as graf;

// ---------------------------------------------------------------------------
// Konstanta
// ---------------------------------------------------------------------------

/// Kapasitas tabel anggota (= kapasitas tabel benang).
pub const MAX_ANGGOTA: usize = graf::MAX_BENANG;

/// Batas atas alokasi produksi (eksklusif); di atas ini = arena uji.
pub const BATAS_ANGGOTA: usize = graf::BATAS_BENANG;

/// Jumlah ember prioritas (cermin `ThreadReadyQueues::count`).
pub const N_EMBER: usize = 32;

/// Penanda "tidak ada anggota".
pub const TAK_ADA: u32 = 0xFFFF_FFFF;

/// Tag laporan: "KLATJDWL".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4A44_574C;

// ---------------------------------------------------------------------------
// Penghitung audit
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_jadwal_masuk` yang berhasil.
    ProdMasuk = 0,
    /// `klatos_jadwal_keluar` yang benar-benar mengeluarkan.
    ProdKeluar = 1,
    /// `klatos_jadwal_pilih` yang mengembalikan benang (bukan 0).
    ProdPilih = 2,
    /// `klatos_jadwal_intip` yang menemukan benang.
    ProdIntip = 3,
    /// Handle basi / argumen tak sah.
    ProdHandleSalah = 4,
}

const N_COUNTER: usize = 5;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Ambil satu penghitung audit (untuk saksi produksi).
#[no_mangle]
pub extern "C" fn klatos_jadwal_counter(which: u32) -> u32 {
    if which as usize >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Diagnostik penolakan
// ---------------------------------------------------------------------------

static TOLAK: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
static TOLAK_BASE: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
static TOLAK_SEEN: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
static TOLAK_IDX: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
static TOLAK_GEN: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];
static TOLAK_ARG: [AtomicU32; 16] = [const { AtomicU32::new(0) }; 16];

#[inline]
fn tolak(fn_id: u32, index: u32, gen: u32, arg: u32) {
    bump(Counter::ProdHandleSalah);
    let i = (fn_id & 0xF) as usize;
    TOLAK[i].fetch_add(1, Ordering::Relaxed);
    if TOLAK_SEEN[i].swap(1, Ordering::AcqRel) == 0 {
        TOLAK_IDX[i].store(index, Ordering::Relaxed);
        TOLAK_GEN[i].store(gen, Ordering::Relaxed);
        TOLAK_ARG[i].store(arg, Ordering::Relaxed);
    }
}

/// Jumlah penolakan pada titik masuk `fn_id`.
#[no_mangle]
pub extern "C" fn klatos_jadwal_tolak(fn_id: u32) -> u32 {
    TOLAK[(fn_id & 0xF) as usize].load(Ordering::Relaxed)
}

/// Jumlah penolakan SEJAK snapshot terakhir.
#[no_mangle]
pub extern "C" fn klatos_jadwal_tolak_prod(fn_id: u32) -> u32 {
    let i = (fn_id & 0xF) as usize;
    TOLAK[i]
        .load(Ordering::Relaxed)
        .saturating_sub(TOLAK_BASE[i].load(Ordering::Relaxed))
}

/// Ambil baseline histogram penolakan.
#[no_mangle]
pub extern "C" fn klatos_jadwal_tolak_snapshot() -> u32 {
    let mut i = 0usize;
    while i < 16 {
        TOLAK_BASE[i].store(TOLAK[i].load(Ordering::Relaxed), Ordering::Relaxed);
        TOLAK_SEEN[i].store(0, Ordering::Relaxed);
        i += 1;
    }
    1
}

/// Detail penolakan pertama titik masuk `fn_id`. `sel`: 0=index 1=gen 2=arg.
#[no_mangle]
pub extern "C" fn klatos_jadwal_tolak_detail(fn_id: u32, sel: u32) -> u32 {
    let i = (fn_id & 0xF) as usize;
    match sel {
        0 => TOLAK_IDX[i].load(Ordering::Relaxed),
        1 => TOLAK_GEN[i].load(Ordering::Relaxed),
        _ => TOLAK_ARG[i].load(Ordering::Relaxed),
    }
}

// ---------------------------------------------------------------------------
// Tabel antrean — array paralel, atomik
// ---------------------------------------------------------------------------
//
// Daftar per ember adalah senarai berantai ganda berbasis INDEKS (bukan
// pointer): `J_NEXT[i]`/`J_PREV[i]` adalah indeks benang berikut/sebelumnya,
// atau `TAK_ADA`.

static J_NEXT: [AtomicU32; MAX_ANGGOTA] = [const { AtomicU32::new(TAK_ADA) }; MAX_ANGGOTA];
static J_PREV: [AtomicU32; MAX_ANGGOTA] = [const { AtomicU32::new(TAK_ADA) }; MAX_ANGGOTA];
static J_BUCKET: [AtomicU32; MAX_ANGGOTA] = [const { AtomicU32::new(TAK_ADA) }; MAX_ANGGOTA];
static J_GEN: [AtomicU32; MAX_ANGGOTA] = [const { AtomicU32::new(0) }; MAX_ANGGOTA];

static J_HEAD: [AtomicU32; N_EMBER] = [const { AtomicU32::new(TAK_ADA) }; N_EMBER];
static J_TAIL: [AtomicU32; N_EMBER] = [const { AtomicU32::new(TAK_ADA) }; N_EMBER];
static J_MASK: AtomicU32 = AtomicU32::new(0);

// Arena uji (indeks >= BATAS_ANGGOTA) — struktur ember terpisah.
static S_HEAD: [AtomicU32; N_EMBER] = [const { AtomicU32::new(TAK_ADA) }; N_EMBER];
static S_TAIL: [AtomicU32; N_EMBER] = [const { AtomicU32::new(TAK_ADA) }; N_EMBER];
static S_MASK: AtomicU32 = AtomicU32::new(0);

static HIDUP: AtomicU32 = AtomicU32::new(0);
static PUNCAK: AtomicU32 = AtomicU32::new(0);


/// Kurangi kedalaman antrean secara saturasi (tidak pernah menurun di bawah 0).
fn hidup_turun(n: u32) {
    let mut cur = HIDUP.load(Ordering::Relaxed);
    while cur > 0 && n > 0 {
        let next = if n >= cur { 0 } else { cur - n };
        match HIDUP.compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(v) => cur = v,
        }
    }
}

// ---------------------------------------------------------------------------
// Arena — produksi (`J_*`) vs uji (`S_*`)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Arena {
    head: &'static [AtomicU32; N_EMBER],
    tail: &'static [AtomicU32; N_EMBER],
    mask: &'static AtomicU32,
}

const ARENA_PROD: Arena = Arena {
    head: &J_HEAD,
    tail: &J_TAIL,
    mask: &J_MASK,
};
const ARENA_UJI: Arena = Arena {
    head: &S_HEAD,
    tail: &S_TAIL,
    mask: &S_MASK,
};

/// Arena untuk sebuah indeks: produksi bila `< BATAS_ANGGOTA`, selain itu uji.
#[inline]
fn arena_untuk(index: u32) -> Arena {
    if (index as usize) < BATAS_ANGGOTA {
        ARENA_PROD
    } else {
        ARENA_UJI
    }
}

// ---------------------------------------------------------------------------
// Inti
// ---------------------------------------------------------------------------

/// Keanggotaan: 1 bila `(index, gen)` sedang ada di antrean.
#[inline]
fn j_sah(index: u32, gen: u32) -> bool {
    if index as usize >= MAX_ANGGOTA || gen == 0 {
        return false;
    }
    let i = index as usize;
    J_BUCKET[i].load(Ordering::Relaxed) != TAK_ADA && J_GEN[i].load(Ordering::Relaxed) == gen
}

/// Buang `i` dari senarai embernya. Pemanggil adalah mutator tunggal arena itu
/// (produksi: di bawah `g_scheduler_lock`; uji: satu CPU saat boot).
fn j_hapus(a: Arena, i: usize) {
    let b = J_BUCKET[i].load(Ordering::Relaxed) as usize;
    let p = J_PREV[i].load(Ordering::Relaxed);
    let n = J_NEXT[i].load(Ordering::Relaxed);
    if p == TAK_ADA {
        a.head[b].store(n, Ordering::Relaxed);
    } else {
        J_NEXT[p as usize].store(n, Ordering::Relaxed);
    }
    if n == TAK_ADA {
        a.tail[b].store(p, Ordering::Relaxed);
    } else {
        J_PREV[n as usize].store(p, Ordering::Relaxed);
    }
    J_BUCKET[i].store(TAK_ADA, Ordering::Relaxed);
    J_NEXT[i].store(TAK_ADA, Ordering::Relaxed);
    J_PREV[i].store(TAK_ADA, Ordering::Relaxed);
    if a.head[b].load(Ordering::Relaxed) == TAK_ADA {
        a.mask.fetch_and(!(1u32 << b), Ordering::Relaxed);
    }
    hidup_turun(1);
}

/// Tambahkan `(index, gen)` ke ekor ember `bucket`. 1 = sukses,
/// 0 = sudah anggota (Q3).
fn masuk_impl(index: u32, gen: u32, bucket: u32) -> u32 {
    let a = arena_untuk(index);
    let i = index as usize;
    if J_BUCKET[i].load(Ordering::Relaxed) != TAK_ADA {
        return 0;
    }
    let b = bucket as usize;
    J_BUCKET[i].store(bucket, Ordering::Relaxed);
    J_GEN[i].store(gen, Ordering::Relaxed);
    J_NEXT[i].store(TAK_ADA, Ordering::Relaxed);
    let tail = a.tail[b].load(Ordering::Relaxed);
    J_PREV[i].store(tail, Ordering::Relaxed);
    if tail == TAK_ADA {
        a.head[b].store(index, Ordering::Relaxed);
    } else {
        J_NEXT[tail as usize].store(index, Ordering::Relaxed);
    }
    a.tail[b].store(index, Ordering::Relaxed);
    a.mask.fetch_or(1u32 << b, Ordering::Relaxed);
    let h = HIDUP.fetch_add(1, Ordering::Relaxed) + 1;
    PUNCAK.fetch_max(h, Ordering::Relaxed);
    1
}

/// Buang `(index, gen)`. 1 = benar-benar dikeluarkan, 0 = tidak ada di antrean
/// (termasuk ketidakcocokan afinitas bila `cek_afinitas` — Q14).
fn keluar_impl(index: u32, gen: u32, cek_afinitas: u32, cpu_mask: u32) -> u32 {
    if index as usize >= MAX_ANGGOTA || gen == 0 {
        return 0;
    }
    let a = arena_untuk(index);
    let i = index as usize;
    if J_BUCKET[i].load(Ordering::Relaxed) == TAK_ADA
        || J_GEN[i].load(Ordering::Relaxed) != gen
    {
        return 0;
    }
    if cek_afinitas != 0 && (graf::benang_ambil(index, gen, graf::TW_AFFINITY) & cpu_mask) == 0 {
        return 0;
    }
    j_hapus(a, i);
    1
}

/// Pilih benang berikutnya dari arena `a`. `elij(index, gen)` = layak (Q10+Q11).
/// `hapus = true` untuk `pilih` (Q12: hapus), `false` untuk `intip` (Q13).
fn pilih_inti<F: Fn(u32, u32) -> bool>(a: Arena, hapus: bool, elij: F) -> u64 {
    let mut hasil = 0u64;
    let mut b = 0usize;
    // Batas iterasi: pemilihan otoritatif (di bawah `g_scheduler_lock`) tidak
    // pernah menyentuh lebih dari `MAX_ANGGOTA` simpul, jadi batas ini tidak
    // mengubah jawabannya. Pembaca IRQ bebas-kunci (`intip`) memakainya supaya
    // pembacaan basi tidak dapat memutari siklus.
    let mut sisa = MAX_ANGGOTA;
    'luar: while b < N_EMBER {
        if (a.mask.load(Ordering::Relaxed) & (1u32 << b)) != 0 {
            let mut cur = a.head[b].load(Ordering::Relaxed);
            while cur != TAK_ADA && sisa > 0 {
                sisa -= 1;
                let i = cur as usize;
                let g = J_GEN[i].load(Ordering::Relaxed);
                if elij(cur, g) {
                    hasil = ((g as u64) << 32) | (cur as u64);
                    if hapus {
                        j_hapus(a, i);
                    }
                    break 'luar;
                }
                cur = J_NEXT[i].load(Ordering::Relaxed);
            }
        }
        b += 1;
    }
    hasil
}

/// Kelayakan produksi: tidak aktif (Q10) dan afinitas cocok (Q11).
#[inline]
fn elij_produksi(index: u32, gen: u32, cpu_mask: u32) -> bool {
    if graf::benang_flag(index, gen, graf::TF_ACTIVE) != 0 {
        return false;
    }
    (graf::benang_ambil(index, gen, graf::TW_AFFINITY) & cpu_mask) != 0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Tambahkan benang ke antrean ember `bucket`. 1 = sukses, 0 = ditolak.
#[no_mangle]
pub extern "C" fn klatos_jadwal_masuk(index: u32, gen: u32, bucket: u32) -> u32 {
    if index as usize >= MAX_ANGGOTA || gen == 0 || bucket as usize >= N_EMBER {
        tolak(1, index, gen, bucket);
        return 0;
    }
    let r = masuk_impl(index, gen, bucket);
    if r == 1 {
        bump(Counter::ProdMasuk);
    } else {
        tolak(1, index, gen, bucket);
    }
    r
}

/// Buang benang dari antrean. 1 = dikeluarkan, 0 = tidak ada (normal).
#[no_mangle]
pub extern "C" fn klatos_jadwal_keluar(
    index: u32,
    gen: u32,
    cek_afinitas: u32,
    cpu_mask: u32,
) -> u32 {
    let r = keluar_impl(index, gen, cek_afinitas, cpu_mask);
    if r == 1 {
        bump(Counter::ProdKeluar);
    }
    r
}

/// Pilih benang berikutnya untuk `cpu_mask` dan KELUARKAN dari antrean.
/// 0 = tidak ada (pemanggil jatuh ke benang idle).
#[no_mangle]
pub extern "C" fn klatos_jadwal_pilih(cpu_mask: u32) -> u64 {
    let h = pilih_inti(ARENA_PROD, true, |i, g| elij_produksi(i, g, cpu_mask));
    if h != 0 {
        bump(Counter::ProdPilih);
    }
    h
}

/// Intip benang berikutnya (tanpa mengubah). 0 = tidak ada.
#[no_mangle]
pub extern "C" fn klatos_jadwal_intip(cpu_mask: u32) -> u64 {
    let h = pilih_inti(ARENA_PROD, false, |i, g| elij_produksi(i, g, cpu_mask));
    if h != 0 {
        bump(Counter::ProdIntip);
    }
    h
}

/// Topeng ember produksi (bit `b` terpasang ⟺ ember `b` tidak kosong).
#[no_mangle]
pub extern "C" fn klatos_jadwal_mask() -> u32 {
    J_MASK.load(Ordering::Relaxed)
}

/// Keanggotaan `(index, gen)` — 1/0.
#[no_mangle]
pub extern "C" fn klatos_jadwal_anggota(index: u32, gen: u32) -> u32 {
    j_sah(index, gen) as u32
}

/// Ember `(index, gen)`, atau `TAK_SAH` bila bukan anggota.
#[no_mangle]
pub extern "C" fn klatos_jadwal_bucket(index: u32, gen: u32) -> u32 {
    if !j_sah(index, gen) {
        return TAK_ADA;
    }
    J_BUCKET[index as usize].load(Ordering::Relaxed)
}

/// Jumlah anggota siap-jalan saat ini.
#[no_mangle]
pub extern "C" fn klatos_jadwal_hidup() -> u32 {
    HIDUP.load(Ordering::Relaxed)
}

/// Puncak jumlah anggota.
#[no_mangle]
pub extern "C" fn klatos_jadwal_puncak() -> u32 {
    PUNCAK.load(Ordering::Relaxed)
}

/// Kapasitas produksi.
#[no_mangle]
pub extern "C" fn klatos_jadwal_kapasitas() -> u32 {
    BATAS_ANGGOTA as u32
}

/// Kosongkan SELURUH tabel antrean (audit; bukan jalur produksi).
#[no_mangle]
pub extern "C" fn klatos_jadwal_reset() -> u32 {
    let mut i = 0usize;
    while i < MAX_ANGGOTA {
        J_BUCKET[i].store(TAK_ADA, Ordering::Relaxed);
        J_GEN[i].store(0, Ordering::Relaxed);
        J_NEXT[i].store(TAK_ADA, Ordering::Relaxed);
        J_PREV[i].store(TAK_ADA, Ordering::Relaxed);
        i += 1;
    }
    let mut b = 0usize;
    while b < N_EMBER {
        J_HEAD[b].store(TAK_ADA, Ordering::Relaxed);
        J_TAIL[b].store(TAK_ADA, Ordering::Relaxed);
        S_HEAD[b].store(TAK_ADA, Ordering::Relaxed);
        S_TAIL[b].store(TAK_ADA, Ordering::Relaxed);
        b += 1;
    }
    J_MASK.store(0, Ordering::Relaxed);
    S_MASK.store(0, Ordering::Relaxed);
    HIDUP.store(0, Ordering::Relaxed);
    PUNCAK.store(0, Ordering::Relaxed);
    1
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosJadwalReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosJadwalReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_masuk: u32,
    pub prod_keluar: u32,
    pub prod_pilih: u32,
    pub prod_intip: u32,
    pub prod_handle_salah: u32,
    pub hidup: u32,
    pub puncak: u32,
    pub kapasitas: u32,
    pub mask: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Peta,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    peta_checks: u32,
    peta_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            peta_checks: 0,
            peta_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, g: Group) {
        self.checks += 1;
        match g {
            Group::Peta => {
                self.peta_checks += 1;
                if lulus {
                    self.peta_passed += 1;
                }
            }
            Group::Neg => {
                self.neg_checks += 1;
                if lulus {
                    self.neg_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Ember untuk arena uji: tiap anggota diuji di ember `idx & 0x1F` supaya
/// sebarannya tetap dan deterministik.
#[inline]
fn uji_bucket(i: usize) -> u32 {
    ((i * 7) % N_EMBER) as u32
}

/// Kosongkan arena uji (indeks `>= BATAS_ANGGOTA`) TANPA menyentuh produksi.
///
/// `HIDUP` dihitung ulang: setiap anggota uji yang dibuang mengurangi
/// penghitung bersama, supaya matriks tidak membocorkan kedalaman antrean.
fn reset_uji() {
    let mut dibuang = 0u32;
    let mut i = BATAS_ANGGOTA;
    while i < MAX_ANGGOTA {
        if J_BUCKET[i].load(Ordering::Relaxed) != TAK_ADA {
            dibuang += 1;
        }
        J_BUCKET[i].store(TAK_ADA, Ordering::Relaxed);
        J_GEN[i].store(0, Ordering::Relaxed);
        J_NEXT[i].store(TAK_ADA, Ordering::Relaxed);
        J_PREV[i].store(TAK_ADA, Ordering::Relaxed);
        i += 1;
    }
    let mut b = 0usize;
    while b < N_EMBER {
        S_HEAD[b].store(TAK_ADA, Ordering::Relaxed);
        S_TAIL[b].store(TAK_ADA, Ordering::Relaxed);
        b += 1;
    }
    S_MASK.store(0, Ordering::Relaxed);
    hidup_turun(dibuang);
}

/// Arena uji: daftarkan satu benang cadangan dengan afinitas & bendera aktif.
fn uji_daftar(aff: u32, aktif: bool) -> (u32, u32) {
    let h = graf::benang_cadangan_daftar();
    let idx = (h & 0xFFFF_FFFF) as u32;
    let gen = (h >> 32) as u32;
    graf::benang_cadangan_taruh(idx, gen, graf::TW_AFFINITY, aff);
    graf::benang_cadangan_set_flag(idx, gen, graf::TF_ACTIVE, if aktif { 1 } else { 0 });
    (idx, gen)
}

/// Kelayakan arena uji: tidak aktif dan afinitas cocok (pola yang sama).
#[inline]
fn elij_uji(index: u32, gen: u32, cpu_mask: u32) -> bool {
    if graf::benang_flag(index, gen, graf::TF_ACTIVE) != 0 {
        return false;
    }
    (graf::benang_ambil(index, gen, graf::TW_AFFINITY) & cpu_mask) != 0
}

/// Generator kongruensial-linear sederhana (deterministik, tanpa alokasi).
struct Lcg(u64);
impl Lcg {
    #[inline]
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
}

/// Model naif independen dari antrean (oracle diferensial).
struct Naif {
    queued: [bool; 64],
    bucket: [u32; 64],
    seq: [u64; 64],
    aff: [u32; 64],
    aktif: [bool; 64],
    next_seq: u64,
}

impl Naif {
    fn baru(aff: [u32; 64], aktif: [bool; 64]) -> Self {
        Naif {
            queued: [false; 64],
            bucket: [0; 64],
            seq: [0; 64],
            aff,
            aktif,
            next_seq: 1,
        }
    }
    fn masuk(&mut self, i: usize, b: u32) -> u32 {
        if self.queued[i] {
            return 0;
        }
        self.queued[i] = true;
        self.bucket[i] = b;
        self.seq[i] = self.next_seq;
        self.next_seq += 1;
        1
    }
    fn keluar(&mut self, i: usize, cek_af: bool, cpu_mask: u32) -> u32 {
        if !self.queued[i] {
            return 0;
        }
        if cek_af && (self.aff[i] & cpu_mask) == 0 {
            return 0;
        }
        self.queued[i] = false;
        1
    }
    /// Pilih: ember terendah dulu, lalu FIFO (seq terkecil). `hapus` = hapus.
    fn pilih(&mut self, cpu_mask: u32, hapus: bool) -> i64 {
        let mut b = 0u32;
        while b < N_EMBER as u32 {
            let mut best: i64 = -1;
            let mut i = 0usize;
            while i < 64 {
                if self.queued[i]
                    && self.bucket[i] == b
                    && !self.aktif[i]
                    && (self.aff[i] & cpu_mask) != 0
                {
                    if best < 0 || self.seq[i] < self.seq[best as usize] {
                        best = i as i64;
                    }
                }
                i += 1;
            }
            if best >= 0 {
                if hapus {
                    self.queued[best as usize] = false;
                }
                return best;
            }
            b += 1;
        }
        -1
    }
    fn mask(&self) -> u32 {
        let mut m = 0u32;
        let mut i = 0usize;
        while i < 64 {
            if self.queued[i] {
                m |= 1u32 << self.bucket[i];
            }
            i += 1;
        }
        m
    }
}

fn run_matrix() -> KlatosJadwalReport {
    let mut m = Matrix::new();

    // --- kelompok 0: konstanta -------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4A44_574C, Group::Peta);
    m.catat(MAX_ANGGOTA == 4096, Group::Peta);
    m.catat(BATAS_ANGGOTA == 4032, Group::Peta);
    m.catat(N_EMBER == 32, Group::Peta);
    m.catat(TAK_ADA == 0xFFFF_FFFF, Group::Peta);
    m.catat(klatos_jadwal_kapasitas() == 4032, Group::Peta);
    m.catat(klatos_jadwal_counter(99) == 0, Group::Peta);
    m.catat(klatos_jadwal_bucket(0, 0) == TAK_ADA, Group::Peta);
    m.catat(klatos_jadwal_anggota(0, 0) == 0, Group::Peta);
    m.catat(!graf::benang_sah(0, 0), Group::Peta);

    // --- kelompok 1: siklus hidup anggota (peta) -------------------------
    graf::benang_cadangan_reset();
    reset_uji();
    {
        let (i0, g0) = uji_daftar(0xFFFF_FFFF, false);
        let (i1, g1) = uji_daftar(0xFFFF_FFFF, false);
        let (i2, g2) = uji_daftar(0xFFFF_FFFF, false);
        let b0 = uji_bucket(i0 as usize);

        m.catat(masuk_impl(i0, g0, b0) == 1, Group::Peta);
        m.catat(j_sah(i0, g0) && graf::benang_sah(i0, g0), Group::Peta);
        m.catat(klatos_jadwal_bucket(i0, g0) == b0, Group::Peta);
        m.catat((S_MASK.load(Ordering::Relaxed) & (1u32 << b0)) != 0, Group::Peta);
        m.catat(klatos_jadwal_anggota(i0, g0) == 1, Group::Peta);
        // Q3: duplikat ditolak.
        m.catat(masuk_impl(i0, g0, b0) == 0, Group::Peta);
        // generasi salah tidak mengenali anggota.
        m.catat(!j_sah(i0, g0.wrapping_add(1)), Group::Peta);
        // keluar sekali berhasil, kedua tidak.
        m.catat(keluar_impl(i0, g0, 0, 1) == 1, Group::Peta);
        m.catat(!j_sah(i0, g0), Group::Peta);
        m.catat((S_MASK.load(Ordering::Relaxed) & (1u32 << b0)) == 0, Group::Peta);
        m.catat(keluar_impl(i0, g0, 0, 1) == 0, Group::Peta);
        // dua anggota lain, ember berbeda, keluar tak berurutan.
        let b1 = uji_bucket(i1 as usize);
        let b2 = uji_bucket(i2 as usize);
        let _ = masuk_impl(i1, g1, b1);
        let _ = masuk_impl(i2, g2, b2);
        m.catat(keluar_impl(i1, g1, 0, 1) == 1, Group::Peta);
        m.catat(j_sah(i2, g2), Group::Peta);
        m.catat(keluar_impl(i2, g2, 0, 1) == 1, Group::Peta);
        // Lepas slot cadangan (tanpa penghitung) supaya arena uji bersih.
        m.catat(graf::benang_cadangan_lepas(i1, g1) == 1, Group::Peta);
        m.catat(graf::benang_cadangan_lepas(i2, g2) == 1, Group::Peta);
    }

    // --- kelompok 2: urutan (FIFO + ember menaik) ------------------------
    graf::benang_cadangan_reset();
    reset_uji();
    {
        let (a, ga) = uji_daftar(0xFFFF_FFFF, false);
        let (b, gb) = uji_daftar(0xFFFF_FFFF, false);
        let (c, gc) = uji_daftar(0xFFFF_FFFF, false);
        let (d, gd) = uji_daftar(0xFFFF_FFFF, false);
        // a->ember 5, b->ember 2, c->ember 5, d->ember 0
        m.catat(masuk_impl(a, ga, 5) == 1, Group::Peta);
        m.catat(masuk_impl(b, gb, 2) == 1, Group::Peta);
        m.catat(masuk_impl(c, gc, 5) == 1, Group::Peta);
        m.catat(masuk_impl(d, gd, 0) == 1, Group::Peta);
        // Q8: ember terendah dulu -> d (0), lalu b (2), lalu a (5) sebelum c (FIFO).
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 1)) == hd(d, gd), Group::Peta);
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 1)) == hd(b, gb), Group::Peta);
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 1)) == hd(a, ga), Group::Peta);
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 1)) == hd(c, gc), Group::Peta);
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 1)) == 0, Group::Peta);
        m.catat(S_MASK.load(Ordering::Relaxed) == 0, Group::Peta);
    }

    // --- kelompok 3: kelayakan (aktif + afinitas) ------------------------
    graf::benang_cadangan_reset();
    reset_uji();
    {
        let (a, ga) = uji_daftar(0xFFFF_FFFF, true); // aktif -> dilewati
        let (b, gb) = uji_daftar(0xFFFF_FFFF, false);
        let (c, gc) = uji_daftar(0x2, false); // hanya cpu 1
        m.catat(masuk_impl(a, ga, 0) == 1, Group::Peta);
        m.catat(masuk_impl(b, gb, 0) == 1, Group::Peta);
        m.catat(masuk_impl(c, gc, 0) == 1, Group::Peta);
        // Q10: `a` aktif -> `b` (FIFO berikutnya) yang dipilih untuk cpu 0.
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 1)) == hd(b, gb), Group::Peta);
        // Q11: `c` afinitas 0x2 tidak cocok cpu 0; tersisa `a` (aktif) -> kosong.
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 1)) == 0, Group::Peta);
        // untuk cpu 1: `c` cocok, tapi `a` masih aktif -> `c`.
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 2)) == hd(c, gc), Group::Peta);
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 2)) == 0, Group::Peta);
    }

    // --- kelompok 4: `intip` tidak mengubah (Q13) ------------------------
    graf::benang_cadangan_reset();
    reset_uji();
    {
        let (a, ga) = uji_daftar(0xFFFF_FFFF, false);
        let _ = masuk_impl(a, ga, 3);
        m.catat(pilih_inti(ARENA_UJI, false, |i, g| elij_uji(i, g, 1)) == hd(a, ga), Group::Peta);
        m.catat(j_sah(a, ga), Group::Peta); // masih anggota
        m.catat((S_MASK.load(Ordering::Relaxed) & (1u32 << 3)) != 0, Group::Peta);
        m.catat(pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, 1)) == hd(a, ga), Group::Peta);
        m.catat(!j_sah(a, ga), Group::Peta); // sekarang terhapus
    }

    // --- kelompok 5: diferensial vs model naif (>= 250.000) --------------
    let mut dif_ukur: u32 = 0;
    let mut dif_beda: u32 = 0;
    {
        graf::benang_cadangan_reset();
        reset_uji();
        // 64 benang cadangan dengan afinitas & keaktifan tetap.
        let mut idx = [0u32; 64];
        let mut gen = [0u32; 64];
        let mut aff = [0u32; 64];
        let mut aktif = [false; 64];
        let mut k = 0usize;
        while k < 64 {
            let a = match k % 4 {
                0 => 0xFFFF_FFFF,
                1 => 0x1,
                2 => 0x2,
                _ => 0x3,
            };
            let ak = (k % 7) == 3;
            let (ix, gx) = uji_daftar(a, ak);
            idx[k] = ix;
            gen[k] = gx;
            aff[k] = a;
            aktif[k] = ak;
            k += 1;
        }
        let mut naif = Naif::baru(aff, aktif);
        let mut rng = Lcg(0x0BAD_F00D_D1CE_5EED);
        let mut op = 0u32;
        while op < 100_000 {
            let r = rng.next();
            let k = (r % 64) as usize;
            let aksi = (r >> 8) % 4;
            let cpu = 1u32 << ((r >> 16) % 2);
            match aksi {
                0 => {
                    let b = (r >> 24) % N_EMBER as u32;
                    let nyata = masuk_impl(idx[k], gen[k], b);
                    let model = naif.masuk(k, b);
                    dif_ukur += 2;
                    if nyata != model {
                        dif_beda += 1;
                    }
                }
                1 => {
                    let cek = ((r >> 20) & 1) as u32;
                    let nyata = keluar_impl(idx[k], gen[k], cek, cpu);
                    let model = naif.keluar(k, cek != 0, cpu);
                    dif_ukur += 2;
                    if nyata != model {
                        dif_beda += 1;
                    }
                }
                2 => {
                    let nyata = pilih_inti(ARENA_UJI, true, |i, g| elij_uji(i, g, cpu));
                    let model = naif.pilih(cpu, true);
                    dif_ukur += 3;
                    let cocok = if model < 0 {
                        nyata == 0
                    } else {
                        nyata == hd(idx[model as usize], gen[model as usize])
                    };
                    if !cocok {
                        dif_beda += 1;
                    }
                }
                _ => {
                    let nyata = pilih_inti(ARENA_UJI, false, |i, g| elij_uji(i, g, cpu));
                    let model = naif.pilih(cpu, false);
                    dif_ukur += 3;
                    let cocok = if model < 0 {
                        nyata == 0
                    } else {
                        nyata == hd(idx[model as usize], gen[model as usize])
                    };
                    if !cocok {
                        dif_beda += 1;
                    }
                }
            }
            // Q9: topeng harus cocok dengan model setiap langkah.
            dif_ukur += 1;
            if S_MASK.load(Ordering::Relaxed) != naif.mask() {
                dif_beda += 1;
            }
            op += 1;
        }
    }
    m.catat(dif_ukur >= 250_000, Group::Peta);
    m.catat(dif_beda == 0, Group::Peta);

    // --- kelompok 6: determinisme ----------------------------------------
    {
        graf::benang_cadangan_reset();
        reset_uji();
        let (a, ga) = uji_daftar(0xFFFF_FFFF, false);
        let (b, gb) = uji_daftar(0x1, false);
        let _ = masuk_impl(a, ga, 1);
        let _ = masuk_impl(b, gb, 1);
        let m1 = pilih_inti(ARENA_UJI, false, |i, g| elij_uji(i, g, 1));
        let m2 = pilih_inti(ARENA_UJI, false, |i, g| elij_uji(i, g, 1));
        let m3 = pilih_inti(ARENA_UJI, false, |i, g| elij_uji(i, g, 1));
        m.catat(m1 == m2 && m2 == m3 && m1 == hd(a, ga), Group::Peta);
    }

    // --- kelompok 7: negatif ---------------------------------------------
    graf::benang_cadangan_reset();
    reset_uji();
    {
        let (a, ga) = uji_daftar(0xFFFF_FFFF, false);
        // ember di luar rentang (Q15).
        m.catat(klatos_jadwal_masuk(a, ga, 32) == 0, Group::Neg);
        m.catat(klatos_jadwal_masuk(a, ga, 0xFFFF_FFFF) == 0, Group::Neg);
        // indeks di luar tabel.
        m.catat(klatos_jadwal_masuk(MAX_ANGGOTA as u32, 1, 0) == 0, Group::Neg);
        // generasi nol = tidak terdaftar.
        m.catat(klatos_jadwal_masuk(a, 0, 0) == 0, Group::Neg);
        // keluar benang yang tidak ada = 0 (bukan penolakan).
        let salah = klatos_jadwal_counter(Counter::ProdHandleSalah as u32);
        m.catat(klatos_jadwal_keluar(a, ga, 0, 1) == 0, Group::Neg);
        m.catat(klatos_jadwal_counter(Counter::ProdHandleSalah as u32) == salah, Group::Neg);
        // Q14: ketidakcocokan afinitas membiarkan anggota di antrean.
        let (c, gc) = uji_daftar(0x2, false);
        let _ = masuk_impl(c, gc, 4);
        m.catat(keluar_impl(c, gc, 1, 1) == 0, Group::Neg);
        m.catat(j_sah(c, gc), Group::Neg);
        m.catat(keluar_impl(c, gc, 1, 2) == 1, Group::Neg);
    }

    // --- kelompok 8: pembungkus C-ABI (peta) -----------------------------
    graf::benang_cadangan_reset();
    reset_uji();
    {
        let (a, ga) = uji_daftar(0xFFFF_FFFF, false);
        let sebelum = klatos_jadwal_counter(Counter::ProdMasuk as u32);
        m.catat(klatos_jadwal_masuk(a, ga, 6) == 1, Group::Peta);
        // `>` bukan `== sebelum + 1`: produksi bisa menaikkan penghitung yang
        // sama di CPU lain selama selftest (F-P156-03).
        m.catat(klatos_jadwal_counter(Counter::ProdMasuk as u32) > sebelum, Group::Peta);
        m.catat(klatos_jadwal_anggota(a, ga) == 1, Group::Peta);
        m.catat(klatos_jadwal_bucket(a, ga) == 6, Group::Peta);
        // Topeng arena uji (bukan topeng produksi `J_MASK`).
        m.catat((S_MASK.load(Ordering::Relaxed) & (1u32 << 6)) != 0, Group::Peta);
        // Arena uji TERPISAH: produksi tidak pernah mencapai anggota uji.
        // Dibuktikan BACA-SAJA (`hapus = false`) supaya matriks tidak mencuri
        // pekerjaan produksi yang nyata (F-P156-02).
        m.catat(
            pilih_inti(ARENA_PROD, false, |i, g| i == a && g == ga) == 0,
            Group::Peta,
        );
        m.catat(klatos_jadwal_intip(1) != hd(a, ga), Group::Peta);
        // Pembungkus C-ABI `pilih` produksi diuji TANPA mencuri pekerjaan:
        // `cpu_mask = 0` membuat tidak ada benang yang layak, jadi Q12 tidak
        // pernah menghapus apa pun (F-P156-02).
        m.catat(klatos_jadwal_pilih(0) == 0, Group::Peta);
        let sk = klatos_jadwal_counter(Counter::ProdKeluar as u32);
        m.catat(klatos_jadwal_keluar(a, ga, 0, 1) == 1, Group::Peta);
        m.catat(klatos_jadwal_counter(Counter::ProdKeluar as u32) > sk, Group::Peta);
        m.catat(klatos_jadwal_anggota(a, ga) == 0, Group::Peta);
    }

    let status = if m.checks == m.passed { 0 } else { -1 };
    KlatosJadwalReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_masuk: klatos_jadwal_counter(Counter::ProdMasuk as u32),
        prod_keluar: klatos_jadwal_counter(Counter::ProdKeluar as u32),
        prod_pilih: klatos_jadwal_counter(Counter::ProdPilih as u32),
        prod_intip: klatos_jadwal_counter(Counter::ProdIntip as u32),
        prod_handle_salah: klatos_jadwal_counter(Counter::ProdHandleSalah as u32),
        hidup: klatos_jadwal_hidup(),
        puncak: klatos_jadwal_puncak(),
        kapasitas: BATAS_ANGGOTA as u32,
        mask: klatos_jadwal_mask(),
        status,
    }
}

/// Handle gabungan `(gen << 32) | index`.
#[inline]
fn hd(index: u32, gen: u32) -> u64 {
    ((gen as u64) << 32) | (index as u64)
}

/// Selftest otoritas antrean jadwal.
#[no_mangle]
pub extern "C" fn klatos_jadwal_selftest() -> KlatosJadwalReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4A44_574C);
        assert!(MAX_ANGGOTA == 4096);
        assert!(BATAS_ANGGOTA == 4032);
        assert!(N_EMBER == 32);
        assert!(TAK_ADA == 0xFFFF_FFFF);
        assert!(Counter::ProdMasuk as usize == 0);
        assert!(Counter::ProdKeluar as usize == 1);
        assert!(Counter::ProdPilih as usize == 2);
        assert!(Counter::ProdIntip as usize == 3);
        assert!(Counter::ProdHandleSalah as usize == 4);
        assert!(N_COUNTER == 5);
        assert!(core::mem::size_of::<KlatosJadwalReport>() == 72);
        assert!(core::mem::align_of::<KlatosJadwalReport>() == 8);
    };
}
