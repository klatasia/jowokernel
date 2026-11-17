// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! REHOME-03 — **KEPEMILIKAN MEKANISME pohon akses jalur** milik Rust.


#![allow(unsafe_code)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosKsmPohonJalurReport` di `KlatSeam117.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_414A_414C; // "KLATAJAL"

/// Indeks "tidak ada" (handle/pointer arena).
pub const NONE: u32 = 0xFFFF_FFFF;
/// Jumlah simpul maksimum di arena global.
pub const MAX_SIMPUL: usize = 2048;
/// Ukuran blok teks (byte).
pub const BLOK: usize = 64;
/// Total byte teks di arena global.
pub const MAX_TEKS: usize = 512 * 1024;
/// Jumlah blok teks.
pub const MAX_BLOK: usize = MAX_TEKS / BLOK;
/// Komponen jalur maksimum (byte).
pub const MAKS_KOMP: usize = 255;
/// Jalur penuh maksimum (byte) — `PATH_MAX - 1`.
pub const MAKS_JALUR: usize = 4095;
/// Kedalaman komponen maksimum.
pub const MAX_KEDALAMAN: usize = 256;

/// `ENOMEM` POSIX.
pub const ENOMEM: i32 = 12;
/// `EPERM` POSIX.
pub const EPERM: i32 = 1;

// ---------------------------------------------------------------------------
// Penghitung audit
// ---------------------------------------------------------------------------

/// Penghitung per-pintu (audit produksi).
#[derive(Clone, Copy)]
pub enum Counter {
    Buat = 0,
    Bebas = 1,
    Klon = 2,
    Setel = 3,
    Perbarui = 4,
    Izin = 5,
    Iterasi = 6,
}
/// Jumlah penghitung.
pub const N_COUNTER: usize = 7;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Simpul + arena
// ---------------------------------------------------------------------------

/// Satu simpul trie. `next_sibling` dipakai ganda sebagai tautan free-list saat
/// simpul bebas (`used == 0`).
#[derive(Clone, Copy)]
struct Simpul {
    parent: u32,
    first_child: u32,
    next_sibling: u32,
    comp_off: u32,
    comp_len: u16,
    full_off: u32,
    full_len: u16,
    perms: u8,
    dinyatakan: u8,
    used: u8,
}

impl Simpul {
    const KOSONG: Simpul = Simpul {
        parent: NONE,
        first_child: NONE,
        next_sibling: NONE,
        comp_off: 0,
        comp_len: 0,
        full_off: 0,
        full_len: 0,
        perms: 0,
        dinyatakan: 0,
        used: 0,
    };
}

const fn simpul_awal() -> [Simpul; MAX_SIMPUL] {
    let mut arr = [Simpul::KOSONG; MAX_SIMPUL];
    let mut i = 0usize;
    while i < MAX_SIMPUL {
        arr[i].next_sibling = if i + 1 < MAX_SIMPUL {
            (i + 1) as u32
        } else {
            NONE
        };
        i += 1;
    }
    arr
}

/// Arena global: simpul + teks + bitmap blok + scratch perantara.
struct Arena {
    simpul: [Simpul; MAX_SIMPUL],
    teks: [u8; MAX_TEKS],
    blok: [u64; MAX_BLOK / 64],
    scratch: [u8; MAKS_JALUR + 1],
    bebas_simpul: u32,
}

impl Arena {
    const BARU: Arena = Arena {
        simpul: simpul_awal(),
        teks: [0u8; MAX_TEKS],
        blok: [0u64; MAX_BLOK / 64],
        scratch: [0u8; MAKS_JALUR + 1],
        bebas_simpul: 0,
    };

    // --- alokator simpul ---------------------------------------------------

    fn simpul_baru(&mut self) -> Option<u32> {
        let idx = self.bebas_simpul;
        if idx == NONE {
            return None;
        }
        self.bebas_simpul = self.simpul[idx as usize].next_sibling;
        self.simpul[idx as usize] = Simpul::KOSONG;
        self.simpul[idx as usize].used = 1;
        Some(idx)
    }

    fn simpul_bebas(&mut self, idx: u32) {
        let s = self.simpul[idx as usize];
        self.teks_bebas(s.comp_off, s.comp_len as usize);
        self.teks_bebas(s.full_off, s.full_len as usize);
        self.simpul[idx as usize] = Simpul::KOSONG;
        self.simpul[idx as usize].next_sibling = self.bebas_simpul;
        self.bebas_simpul = idx;
    }

    // --- alokator teks (bitmap blok) ---------------------------------------

    #[inline]
    fn blok_ada(&self, i: usize) -> bool {
        (self.blok[i / 64] >> (i % 64)) & 1 != 0
    }

    #[inline]
    fn blok_set(&mut self, i: usize, v: bool) {
        if v {
            self.blok[i / 64] |= 1u64 << (i % 64);
        } else {
            self.blok[i / 64] &= !(1u64 << (i % 64));
        }
    }

    fn blok_cari(&self, n: usize) -> Option<usize> {
        if n == 0 {
            return Some(0);
        }
        let mut run = 0usize;
        let mut i = 0usize;
        while i < MAX_BLOK {
            if self.blok_ada(i) {
                run = 0;
            } else {
                run += 1;
                if run == n {
                    return Some(i + 1 - n);
                }
            }
            i += 1;
        }
        None
    }

    fn teks_alok(&mut self, data: &[u8]) -> Option<u32> {
        if data.is_empty() {
            return Some(0);
        }
        let n = (data.len() + BLOK - 1) / BLOK;
        let idx = self.blok_cari(n)?;
        for k in 0..n {
            self.blok_set(idx + k, true);
        }
        let off = idx * BLOK;
        self.teks[off..off + data.len()].copy_from_slice(data);
        Some(off as u32)
    }

    /// Alokasi dari `self.scratch[..len]` (bidang terpisah, tanpa konflik pinjam).
    fn teks_alok_scratch(&mut self, len: usize) -> Option<u32> {
        if len == 0 {
            return Some(0);
        }
        let n = (len + BLOK - 1) / BLOK;
        let idx = self.blok_cari(n)?;
        for k in 0..n {
            self.blok_set(idx + k, true);
        }
        let off = idx * BLOK;
        self.teks[off..off + len].copy_from_slice(&self.scratch[..len]);
        Some(off as u32)
    }

    fn teks_bebas(&mut self, off: u32, len: usize) {
        if len == 0 {
            return;
        }
        let n = (len + BLOK - 1) / BLOK;
        let idx = off as usize / BLOK;
        for k in 0..n {
            self.blok_set(idx + k, false);
        }
    }

    #[inline]
    fn teks(&self, off: u32, len: usize) -> &[u8] {
        &self.teks[off as usize..off as usize + len]
    }

    // --- konstruksi pohon --------------------------------------------------

    /// Pohon akar `"/"`. Mengembalikan `NONE` bila arena penuh.
    fn akar_baru(&mut self) -> u32 {
        let idx = match self.simpul_baru() {
            Some(x) => x,
            None => return NONE,
        };
        let co = match self.teks_alok(b"/") {
            Some(x) => x,
            None => {
                self.simpul_bebas(idx);
                return NONE;
            }
        };
        let fo = match self.teks_alok(b"/") {
            Some(x) => x,
            None => {
                self.teks_bebas(co, 1);
                self.simpul_bebas(idx);
                return NONE;
            }
        };
        let s = &mut self.simpul[idx as usize];
        s.comp_off = co;
        s.comp_len = 1;
        s.full_off = fo;
        s.full_len = 1;
        idx
    }

    /// `Trie::clear()` + reset metadata akar.
    fn setel_ulang(&mut self, root: u32) {
        let mut c = self.simpul[root as usize].first_child;
        while c != NONE {
            let nx = self.simpul[c as usize].next_sibling;
            self.bebas_subpohon(c);
            c = nx;
        }
        self.simpul[root as usize].first_child = NONE;
        self.simpul[root as usize].perms = 0;
        self.simpul[root as usize].dinyatakan = 0;
        // full tetap "/"
    }

    fn bebas_subpohon(&mut self, root: u32) {
        let mut c = self.simpul[root as usize].first_child;
        while c != NONE {
            let nx = self.simpul[c as usize].next_sibling;
            self.bebas_subpohon(c);
            c = nx;
        }
        self.simpul_bebas(root);
    }

    /// Lepas `child` dari daftar anak `parent` (anak langsung). Wajib dipanggil
    /// sebelum [`Arena::bebas_subpohon`] saat rollback, supaya tidak ada tautan
    /// menggantung ke simpul yang sudah masuk free-list.
    fn lepas_anak(&mut self, parent: u32, child: u32) {
        let mut c = self.simpul[parent as usize].first_child;
        if c == child {
            self.simpul[parent as usize].first_child = self.simpul[child as usize].next_sibling;
            return;
        }
        while c != NONE {
            let nx = self.simpul[c as usize].next_sibling;
            if nx == child {
                self.simpul[c as usize].next_sibling = self.simpul[child as usize].next_sibling;
                return;
            }
            c = nx;
        }
    }

    /// `Trie::deep_copy()`.
    fn klon(&mut self, src: u32) -> u32 {
        match self.salin_subpohon(src) {
            Some(d) => d,
            None => NONE,
        }
    }

    fn salin_subpohon(&mut self, src: u32) -> Option<u32> {
        let dst = self.simpul_baru()?;
        let s = self.simpul[src as usize];
        let clen = s.comp_len as usize;
        let flen = s.full_len as usize;
        // Salin teks ke buffer lokal dulu (menghindari pinjam `self` ganda).
        let mut cb = [0u8; MAKS_KOMP];
        let mut fb = [0u8; MAKS_JALUR];
        cb[..clen].copy_from_slice(self.teks(s.comp_off, clen));
        fb[..flen].copy_from_slice(self.teks(s.full_off, flen));
        let co = match self.teks_alok(&cb[..clen]) {
            Some(x) => x,
            None => {
                self.simpul_bebas(dst);
                return None;
            }
        };
        let fo = match self.teks_alok(&fb[..flen]) {
            Some(x) => x,
            None => {
                self.teks_bebas(co, clen);
                self.simpul_bebas(dst);
                return None;
            }
        };
        {
            let d = &mut self.simpul[dst as usize];
            d.comp_off = co;
            d.comp_len = s.comp_len;
            d.full_off = fo;
            d.full_len = s.full_len;
            d.perms = s.perms;
            d.dinyatakan = s.dinyatakan;
        }
        let mut child = s.first_child;
        let mut prev = NONE;
        while child != NONE {
            let nx = self.simpul[child as usize].next_sibling;
            let c = match self.salin_subpohon(child) {
                Some(x) => x,
                None => {
                    self.bebas_subpohon(dst);
                    return None;
                }
            };
            self.simpul[c as usize].parent = dst;
            if prev == NONE {
                self.simpul[dst as usize].first_child = c;
            } else {
                self.simpul[prev as usize].next_sibling = c;
            }
            prev = c;
            child = nx;
        }
        Some(dst)
    }

    // --- pencarian ---------------------------------------------------------

    fn cari_anak(&self, parent: u32, comp: &[u8]) -> u32 {
        let mut c = self.simpul[parent as usize].first_child;
        while c != NONE {
            let s = &self.simpul[c as usize];
            if s.comp_len as usize == comp.len()
                && self.teks(s.comp_off, comp.len()) == comp
            {
                return c;
            }
            c = s.next_sibling;
        }
        NONE
    }

    fn telusuri(&self, root: u32, parts: &[&[u8]]) -> (u32, usize) {
        let mut cur = root;
        let mut i = 0usize;
        while i < parts.len() {
            let c = self.cari_anak(cur, parts[i]);
            if c == NONE {
                break;
            }
            cur = c;
            i += 1;
        }
        (cur, i)
    }

    /// Tambah anak baru dengan `full` dibangun di `self.scratch`.
    fn tambah_anak(&mut self, parent: u32, comp: &[u8], perms: u8, dinyatakan: u8) -> Option<u32> {
        if comp.len() > MAKS_KOMP {
            return None;
        }
        let plen = self.simpul[parent as usize].full_len as usize;
        let poff = self.simpul[parent as usize].full_off as usize;
        let flen = plen + 1 + comp.len();
        if flen > MAKS_JALUR {
            return None;
        }
        self.scratch[..plen].copy_from_slice(&self.teks[poff..poff + plen]);
        self.scratch[plen] = b'/';
        self.scratch[plen + 1..flen].copy_from_slice(comp);

        let idx = self.simpul_baru()?;
        let co = match self.teks_alok(comp) {
            Some(x) => x,
            None => {
                self.simpul_bebas(idx);
                return None;
            }
        };
        let fo = match self.teks_alok_scratch(flen) {
            Some(x) => x,
            None => {
                self.teks_bebas(co, comp.len());
                self.simpul_bebas(idx);
                return None;
            }
        };
        {
            let s = &mut self.simpul[idx as usize];
            s.parent = parent;
            s.comp_off = co;
            s.comp_len = comp.len() as u16;
            s.full_off = fo;
            s.full_len = flen as u16;
            s.perms = perms;
            s.dinyatakan = dinyatakan;
        }
        let mut c = self.simpul[parent as usize].first_child;
        if c == NONE {
            self.simpul[parent as usize].first_child = idx;
        } else {
            loop {
                let nx = self.simpul[c as usize].next_sibling;
                if nx == NONE {
                    self.simpul[c as usize].next_sibling = idx;
                    break;
                }
                c = nx;
            }
        }
        Some(idx)
    }

    // --- operasi akses jalur ----------------------------------------------

    /// `update_intermediate_node_permissions` (lewati sub-pohon dinyatakan).
    fn propagasi(&mut self, node: u32, perms: u8) {
        let mut c = self.simpul[node as usize].first_child;
        while c != NONE {
            let nx = self.simpul[c as usize].next_sibling;
            if self.simpul[c as usize].dinyatakan == 0 {
                self.simpul[c as usize].perms = perms;
                self.propagasi(c, perms);
            }
            c = nx;
        }
    }

    /// Inti pembaruan akses jalur. `0` / `-EPERM` / `-ENOMEM`.
    fn perbarui(&mut self, root: u32, path: &[u8], perms: u8) -> i32 {
        if path.len() > MAKS_JALUR {
            return -ENOMEM;
        }
        let mut parts: [&[u8]; MAX_KEDALAMAN] = [&[]; MAX_KEDALAMAN];
        let np = match pecah(path, &mut parts) {
            Some(n) => n,
            None => return -ENOMEM,
        };
        let (cur, mut i) = self.telusuri(root, &parts[..np]);

        if i == np {
            // Cabang EXACT.
            let s = self.simpul[cur as usize];
            if s.dinyatakan != 0 && (perms & !s.perms) != 0 {
                return -EPERM;
            }
            if s.perms != perms {
                self.propagasi(cur, perms);
            }
            self.simpul[cur as usize].dinyatakan = 1;
            self.simpul[cur as usize].perms = perms;
            return 0;
        }

        // Cabang PARSIAL: buat komponen sisa dengan metadata turunan.
        let mut parent = cur;
        let mut first_new = NONE;
        while i < np {
            let comp = parts[i];
            let pperm = self.simpul[parent as usize].perms;
            match self.tambah_anak(parent, comp, pperm, 0) {
                Some(x) => {
                    if first_new == NONE {
                        first_new = x;
                    }
                    parent = x;
                }
                None => {
                    if first_new != NONE {
                        self.lepas_anak(cur, first_new);
                        self.bebas_subpohon(first_new);
                    }
                    return -ENOMEM;
                }
            }
            i += 1;
        }
        // Metadata simpul akhir: full = jalur kanonik, perms = baru, dinyatakan = true.
        let newfo = match self.teks_alok(path) {
            Some(x) => x,
            None => {
                if first_new != NONE {
                    self.lepas_anak(cur, first_new);
                    self.bebas_subpohon(first_new);
                }
                return -ENOMEM;
            }
        };
        let old = self.simpul[parent as usize];
        self.teks_bebas(old.full_off, old.full_len as usize);
        let s = &mut self.simpul[parent as usize];
        s.full_off = newfo;
        s.full_len = path.len() as u16;
        s.perms = perms;
        s.dinyatakan = 1;
        0
    }

    /// Pembacaan izin prefiks jalur terpanjang.
    fn izin(&self, root: u32, path: &[u8]) -> u32 {
        let mut parts: [&[u8]; MAX_KEDALAMAN] = [&[]; MAX_KEDALAMAN];
        let np = match pecah(path, &mut parts) {
            Some(n) => n,
            None => return self.simpul[root as usize].perms as u32,
        };
        let (node, _) = self.telusuri(root, &parts[..np]);
        self.simpul[node as usize].perms as u32
    }

    // --- iterasi -----------------------------------------------------------

    fn dinyatakan_jumlah(&self, node: u32) -> u32 {
        let mut n = if self.simpul[node as usize].dinyatakan != 0 {
            1
        } else {
            0
        };
        let mut c = self.simpul[node as usize].first_child;
        while c != NONE {
            n += self.dinyatakan_jumlah(c);
            c = self.simpul[c as usize].next_sibling;
        }
        n
    }

    /// `for_each_node_in_tree_order` (pre-order) yang difilter `dinyatakan`.
    /// Mengembalikan `Some((perms,len))`, `None` bila `i` di luar rentang,
    /// atau `Err(())` bila `out` kurang.
    fn dinyatakan_ke(
        &self,
        node: u32,
        target: u32,
        seen: &mut u32,
        out: &mut [u8],
    ) -> Result<Option<(u32, usize)>, ()> {
        if self.simpul[node as usize].dinyatakan != 0 {
            if *seen == target {
                let s = &self.simpul[node as usize];
                let flen = s.full_len as usize;
                if flen > out.len() {
                    return Err(());
                }
                let off = s.full_off as usize;
                out[..flen].copy_from_slice(&self.teks[off..off + flen]);
                return Ok(Some((s.perms as u32, flen)));
            }
            *seen += 1;
        }
        let mut c = self.simpul[node as usize].first_child;
        while c != NONE {
            match self.dinyatakan_ke(c, target, seen, out)? {
                Some(r) => return Ok(Some(r)),
                None => {}
            }
            c = self.simpul[c as usize].next_sibling;
        }
        Ok(None)
    }
}

/// Pecah jalur menjadi komponen (lewati bagian kosong, seperti
/// `SplitBehavior::Nothing` pada jalur kanonik).
fn pecah<'a>(path: &'a [u8], out: &mut [&'a [u8]]) -> Option<usize> {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < path.len() {
        while i < path.len() && path[i] == b'/' {
            i += 1;
        }
        if i >= path.len() {
            break;
        }
        let mulai = i;
        while i < path.len() && path[i] != b'/' {
            i += 1;
        }
        if n >= out.len() {
            return None;
        }
        out[n] = &path[mulai..i];
        n += 1;
    }
    Some(n)
}

// ---------------------------------------------------------------------------
// Arena PRODUKSI + kunci
// ---------------------------------------------------------------------------

struct SyncArena(UnsafeCell<Arena>);
// SAFETY: seluruh akses terjadi di bawah [`KUNCI`].
unsafe impl Sync for SyncArena {}

static ARENA: SyncArena = SyncArena(UnsafeCell::new(Arena::BARU));
static KUNCI: AtomicBool = AtomicBool::new(false);

#[inline]
fn kunci_ambil() {
    while KUNCI.swap(true, Ordering::Acquire) {
        core::hint::spin_loop();
    }
}

#[inline]
fn kunci_lepas() {
    KUNCI.store(false, Ordering::Release);
}

#[inline]
fn arena() -> &'static mut Arena {
    // SAFETY: dipanggil hanya di antara kunci_ambil/kunci_lepas.
    unsafe { &mut *ARENA.0.get() }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Buat pohon akar `"/"`; `NONE` bila arena penuh.
#[no_mangle]
pub extern "C" fn klatos_ksm_pohon_jalur_akar_baru() -> u32 {
    bump(Counter::Buat);
    kunci_ambil();
    let r = arena().akar_baru();
    kunci_lepas();
    r
}

/// Bebaskan seluruh sub-pohon.
#[no_mangle]
pub extern "C" fn klatos_ksm_pohon_jalur_bebas(root: u32) {
    bump(Counter::Bebas);
    if root == NONE {
        return;
    }
    kunci_ambil();
    arena().bebas_subpohon(root);
    kunci_lepas();
}

/// `deep_copy`; `NONE` bila arena penuh.
#[no_mangle]
pub extern "C" fn klatos_ksm_pohon_jalur_klon(root: u32) -> u32 {
    bump(Counter::Klon);
    if root == NONE {
        return NONE;
    }
    kunci_ambil();
    let r = arena().klon(root);
    kunci_lepas();
    r
}

/// `clear()` + reset metadata akar.
#[no_mangle]
pub extern "C" fn klatos_ksm_pohon_jalur_setel_ulang(root: u32) -> u32 {
    bump(Counter::Setel);
    if root == NONE {
        return 0;
    }
    kunci_ambil();
    arena().setel_ulang(root);
    kunci_lepas();
    1
}

/// Inti pembaruan akses jalur; `0` / `-EPERM` / `-ENOMEM`.
///
/// # Safety
/// `path_ptr` sah untuk `path_len` byte selama pemanggilan.
#[no_mangle]
pub unsafe extern "C" fn klatos_ksm_pohon_jalur_perbarui(
    root: u32,
    path_ptr: *const u8,
    path_len: usize,
    perms: u32,
) -> i32 {
    bump(Counter::Perbarui);
    let path: &[u8] = if path_ptr.is_null() || path_len == 0 {
        &[]
    } else {
        // SAFETY: kontrak pemanggil.
        unsafe { core::slice::from_raw_parts(path_ptr, path_len) }
    };
    kunci_ambil();
    let r = arena().perbarui(root, path, perms as u8);
    kunci_lepas();
    r
}

/// Izin simpul cocok (longest component-prefix).
///
/// # Safety
/// `path_ptr` sah untuk `path_len` byte selama pemanggilan.
#[no_mangle]
pub unsafe extern "C" fn klatos_ksm_pohon_jalur_izin(root: u32, path_ptr: *const u8, path_len: usize) -> u32 {
    bump(Counter::Izin);
    let path: &[u8] = if path_ptr.is_null() || path_len == 0 {
        &[]
    } else {
        // SAFETY: kontrak pemanggil.
        unsafe { core::slice::from_raw_parts(path_ptr, path_len) }
    };
    kunci_ambil();
    let r = arena().izin(root, path);
    kunci_lepas();
    r
}

/// Banyak simpul dinyatakan (tree order).
#[no_mangle]
pub extern "C" fn klatos_ksm_pohon_jalur_dinyatakan_jumlah(root: u32) -> u32 {
    if root == NONE {
        return 0;
    }
    kunci_ambil();
    let r = arena().dinyatakan_jumlah(root);
    kunci_lepas();
    r
}

/// Jalur + izin simpul dinyatakan ke-`i` (tree order, pre-order).
///
/// Mengembalikan `(perms << 32) | len`, `-1` bila `i` di luar rentang, atau
/// `-ENOMEM` bila `out_cap < len`.
///
/// # Safety
/// `out` sah untuk `out_cap` byte tulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_ksm_pohon_jalur_dinyatakan_ke(
    root: u32,
    i: u32,
    out: *mut u8,
    out_cap: usize,
) -> i64 {
    bump(Counter::Iterasi);
    if root == NONE || out.is_null() || out_cap == 0 {
        return -1;
    }
    let buf: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(out, out_cap) };
    kunci_ambil();
    let mut seen = 0u32;
    let r = arena().dinyatakan_ke(root, i, &mut seen, buf);
    kunci_lepas();
    match r {
        Ok(Some((perms, len))) => ((perms as i64) << 32) | (len as i64),
        Ok(None) => -1,
        Err(()) => -(ENOMEM as i64),
    }
}

/// Membaca penghitung audit `i`.
#[no_mangle]
pub extern "C" fn klatos_ksm_pohon_jalur_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Reset seluruh penghitung; mengembalikan jumlahnya.
#[no_mangle]
pub extern "C" fn klatos_ksm_pohon_jalur_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + uji-sendiri
// ---------------------------------------------------------------------------

/// Laporan REHOME-03; cermin `KlatosKsmPohonJalurReport` di `KlatSeam117.h`.
#[repr(C)]
pub struct KlatosKsmPohonJalurReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

const R: u8 = 1;
const W: u8 = 2;
const X: u8 = 4;
const C_CR: u8 = 8;
const B: u8 = 16;

// Pembungkus aman untuk uji-sendiri: memanggil pintu C ABI `unsafe` dengan
// slice yang sah selama pemanggilan. Ini meniru konvensi `keamanan_aksi.rs`
// (uji-sendiri memakai inti aman, bukan pintu `unsafe` langsung).

#[inline]
fn pintu_perbarui(root: u32, path: &[u8], perms: u32) -> i32 {
    // SAFETY: `path` sah selama pemanggilan.
    unsafe { klatos_ksm_pohon_jalur_perbarui(root, path.as_ptr(), path.len(), perms) }
}

#[inline]
fn pintu_izin(root: u32, path: &[u8]) -> u32 {
    // SAFETY: `path` sah selama pemanggilan.
    unsafe { klatos_ksm_pohon_jalur_izin(root, path.as_ptr(), path.len()) }
}

#[inline]
fn pintu_dinyatakan_ke(root: u32, i: u32, out: &mut [u8]) -> i64 {
    // SAFETY: `out` sah untuk `out.len()` byte tulis.
    unsafe { klatos_ksm_pohon_jalur_dinyatakan_ke(root, i, out.as_mut_ptr(), out.len()) }
}

fn jalur(p: u32, out: &mut [u8; 512]) -> (u32, usize) {
    let r = pintu_dinyatakan_ke(p, 0, out);
    if r < 0 {
        (0, 0)
    } else {
        (((r >> 32) as u32), (r & 0xFFFF_FFFF) as usize)
    }
}

fn run() -> KlatosKsmPohonJalurReport {
    let mut c = 0u32;
    let mut p = 0u32;
    macro_rules! x {
        ($e:expr) => {{
            c += 1;
            if $e {
                p += 1;
            }
        }};
    }

    // --- kepala & bentuk ---------------------------------------------------
    x!(REPORT_TAG == 0x4B4C_4154_414A_414C);
    x!(N_COUNTER == 7);
    x!(core::mem::size_of::<KlatosKsmPohonJalurReport>() == 32);
    x!(MAX_SIMPUL == 2048 && BLOK == 64 && MAX_BLOK == 8192);
    x!(MAKS_KOMP == 255 && MAKS_JALUR == 4095);

    // --- pohon kosong ------------------------------------------------------
    {
        let t = klatos_ksm_pohon_jalur_akar_baru();
        x!(t != NONE);
        x!(pintu_izin(t, b"/") == 0);
        x!(pintu_izin(t, b"/apa") == 0);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 0);
        let mut tmp = [0u8; 8];
        x!(pintu_dinyatakan_ke(t, 0, &mut tmp) == -1);
        klatos_ksm_pohon_jalur_bebas(t);
    }

    // --- insert + nested + inheritance ------------------------------------
    {
        let t = klatos_ksm_pohon_jalur_akar_baru();
        let mut b = [0u8; 512];

        x!(pintu_perbarui(t, b"/a", R as u32) == 0);
        x!(pintu_izin(t, b"/a") == R as u32);
        x!(pintu_izin(t, b"/a/b") == R as u32); // parent match
        x!(pintu_izin(t, b"/zzz") == 0); // non-match
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 1);
        let (perms, len) = jalur(t, &mut b);
        x!(perms == R as u32 && len == 2 && &b[..2] == b"/a");

        // nested: /a/b = R|W (turunan /a)
        x!(pintu_perbarui(t, b"/a/b", (R | W) as u32) == 0);
        x!(pintu_izin(t, b"/a/b") == (R | W) as u32);
        x!(pintu_izin(t, b"/a/b/c") == (R | W) as u32);
        x!(pintu_izin(t, b"/a") == R as u32); // tak berubah
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 2);

        // exact-match elevasi -> EPERM
        x!(pintu_perbarui(t, b"/a", (R | W) as u32) == -EPERM);
        x!(pintu_izin(t, b"/a") == R as u32);
        // exact-match izin sama -> no-op
        x!(pintu_perbarui(t, b"/a", R as u32) == 0);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 2);
        // exact-match penurunan -> propagasi ke anak non-dinyatakan (tidak ada di sini)
        x!(pintu_perbarui(t, b"/a", 0) == 0);
        x!(pintu_izin(t, b"/a") == 0);
        x!(pintu_izin(t, b"/a/b") == (R | W) as u32); // b dinyatakan, aman

        // duplicate path (exact, sama)
        x!(pintu_perbarui(t, b"/a/b", (R | W) as u32) == 0);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 2);

        klatos_ksm_pohon_jalur_bebas(t);
    }

    // --- propagasi intermediate -------------------------------------------
    {
        let t = klatos_ksm_pohon_jalur_akar_baru();
        let mut b = [0u8; 512];
        // /p/q/r = R -> p,q intermediate; r final dinyatakan
        x!(pintu_perbarui(t, b"/p/q/r", R as u32) == 0);
        x!(pintu_izin(t, b"/p") == 0); // p intermediate
        x!(pintu_izin(t, b"/p/q") == 0);
        x!(pintu_izin(t, b"/p/q/r") == R as u32);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 1);
        // buka akses /p = R|W -> propagasi ke q (non-dinyatakan), lewati r (dinyatakan)
        x!(pintu_perbarui(t, b"/p", (R | W) as u32) == 0);
        x!(pintu_izin(t, b"/p") == (R | W) as u32);
        x!(pintu_izin(t, b"/p/q") == (R | W) as u32);
        x!(pintu_izin(t, b"/p/q/r") == R as u32); // tetap
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 2);
        // jalur p yang mula-mula perantara -> quirk "//p"
        let (perms, len) = jalur(t, &mut b);
        // urutan pre-order: p dulu (baru dinyatakan), lalu r
        x!(perms == (R | W) as u32);
        x!(len == 3 && &b[..3] == b"//p");
        klatos_ksm_pohon_jalur_bebas(t);
    }

    // --- quirk full_path: intermediate lalu exact --------------------------
    {
        let t = klatos_ksm_pohon_jalur_akar_baru();
        let mut b = [0u8; 512];
        x!(pintu_perbarui(t, b"/a/b", R as u32) == 0);
        x!(pintu_perbarui(t, b"/a", R as u32) == 0);
        // a: mula-mula perantara -> "//a"; b: final -> "/a/b"
        let (pa, la) = {
            let r = pintu_dinyatakan_ke(t, 0, &mut b);
            (((r >> 32) as u32), (r & 0xFFFF_FFFF) as usize)
        };
        x!(pa == R as u32 && la == 3 && &b[..3] == b"//a");
        let (pb, lb) = {
            let r = pintu_dinyatakan_ke(t, 1, &mut b);
            (((r >> 32) as u32), (r & 0xFFFF_FFFF) as usize)
        };
        x!(pb == R as u32 && lb == 4 && &b[..4] == b"/a/b");
        klatos_ksm_pohon_jalur_bebas(t);
    }

    // --- reset -------------------------------------------------------------
    {
        let t = klatos_ksm_pohon_jalur_akar_baru();
        x!(pintu_perbarui(t, b"/a/b", R as u32) == 0);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 1);
        x!(klatos_ksm_pohon_jalur_setel_ulang(t) == 1);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t) == 0);
        x!(pintu_izin(t, b"/a") == 0);
        x!(pintu_izin(t, b"/a/b") == 0);
        klatos_ksm_pohon_jalur_bebas(t);
    }

    // --- klon (deep_copy) --------------------------------------------------
    {
        let t = klatos_ksm_pohon_jalur_akar_baru();
        x!(pintu_perbarui(t, b"/a/b", (R | W) as u32) == 0);
        let k = klatos_ksm_pohon_jalur_klon(t);
        x!(k != NONE);
        x!(pintu_izin(k, b"/a/b") == (R | W) as u32);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(k) == 1);
        // independen: ubah asli, klon tetap
        x!(pintu_perbarui(t, b"/a", R as u32) == 0);
        x!(pintu_izin(t, b"/a") == R as u32);
        x!(pintu_izin(k, b"/a") == 0); // klon tak terpengaruh
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(k) == 1);
        klatos_ksm_pohon_jalur_bebas(k);
        klatos_ksm_pohon_jalur_bebas(t);
    }

    // --- bebas + alokasi ulang (tidak bocor) ------------------------------
    {
        let a = klatos_ksm_pohon_jalur_akar_baru();
        for _ in 0..32 {
            x!(pintu_perbarui(a, b"/x", R as u32) == 0);
        }
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(a) == 1); // /x sama (exact)
        klatos_ksm_pohon_jalur_bebas(a);
        let c2 = klatos_ksm_pohon_jalur_akar_baru();
        x!(c2 != NONE);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(c2) == 0);
        klatos_ksm_pohon_jalur_bebas(c2);
    }

    // --- determinisme ------------------------------------------------------
    {
        let t1 = klatos_ksm_pohon_jalur_akar_baru();
        let t2 = klatos_ksm_pohon_jalur_akar_baru();
        let mut b = [0u8; 512];
        for i in 0..40u32 {
            let mut path = [0u8; 16];
            path[0] = b'/';
            path[1] = b'a' + (i % 5) as u8;
            path[2] = b'/';
            path[3] = b'b' + (i % 7) as u8;
            let l = 4;
            x!(pintu_perbarui(t1, &path[..l], (i % 32) as u32) >= -1);
            x!(pintu_perbarui(t2, &path[..l], (i % 32) as u32) >= -1);
        }
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t1) == klatos_ksm_pohon_jalur_dinyatakan_jumlah(t2));
        let n = klatos_ksm_pohon_jalur_dinyatakan_jumlah(t1);
        for i in 0..n {
            let r1 = pintu_dinyatakan_ke(t1, i, &mut b);
            let l1 = (r1 & 0xFFFF_FFFF) as usize;
            let p1 = (r1 >> 32) as u32;
            let r2 = pintu_dinyatakan_ke(t2, i, &mut b);
            let l2 = (r2 & 0xFFFF_FFFF) as usize;
            let p2 = (r2 >> 32) as u32;
            x!(l1 == l2 && p1 == p2);
        }
        klatos_ksm_pohon_jalur_bebas(t1);
        klatos_ksm_pohon_jalur_bebas(t2);
    }

    // --- malformed / overflow ---------------------------------------------
    {
        let t = klatos_ksm_pohon_jalur_akar_baru();
        // jalur kosong -> cabang exact pada akar (didokumentasikan)
        x!(pintu_perbarui(t, b"", R as u32) == 0);
        x!(pintu_izin(t, b"/") == R as u32);
        klatos_ksm_pohon_jalur_bebas(t);

        // kedalaman melebihi MAX_KEDALAMAN -> -ENOMEM
        let t2 = klatos_ksm_pohon_jalur_akar_baru();
        let mut deep = [b'a'; 600];
        // sisipkan '/' tiap 2 byte untuk membuat > 256 komponen
        let mut k = 1usize;
        while k < 600 {
            deep[k] = b'/';
            k += 2;
        }
        deep[0] = b'/';
        let rc = pintu_perbarui(t2, &deep, R as u32);
        x!(rc == -ENOMEM);
        klatos_ksm_pohon_jalur_bebas(t2);

        // komponen > 255 -> -ENOMEM
        let t3 = klatos_ksm_pohon_jalur_akar_baru();
        let mut big = [b'z'; 300];
        big[0] = b'/';
        let rc = pintu_perbarui(t3, &big, R as u32);
        x!(rc == -ENOMEM);
        klatos_ksm_pohon_jalur_bebas(t3);

        // rollback rantai: "/a/" + 297 z. Komponen "a" sempat dibuat, lalu
        // komponen kedua (>255) gagal -> rollback harus melepas "a" dari akar.
        let t4 = klatos_ksm_pohon_jalur_akar_baru();
        let mut mid = [b'z'; 300];
        mid[0] = b'/';
        mid[1] = b'a';
        mid[2] = b'/';
        let rc = pintu_perbarui(t4, &mid, R as u32);
        x!(rc == -ENOMEM);
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t4) == 0); // rollback penuh
        x!(pintu_izin(t4, b"/a") == 0); // tak ada sisa "a"
        x!(pintu_perbarui(t4, b"/a", R as u32) == 0); // pohon masih sehat
        x!(pintu_izin(t4, b"/a") == R as u32);
        klatos_ksm_pohon_jalur_bebas(t4);

        // kehabisan arena simpul: sisip berulang -> sebagian -ENOMEM, sisanya
        // konsisten. Membuktikan rollback tidak meninggalkan tautan menggantung.
        let t5 = klatos_ksm_pohon_jalur_akar_baru();
        let mut ok = 0u32;
        let mut gagal = 0u32;
        for i in 0..1100u32 {
            let mut p = [0u8; 8];
            p[0] = b'/';
            p[1] = b'c';
            p[2] = b'a' + ((i / 676) % 26) as u8;
            p[3] = b'a' + ((i / 26) % 26) as u8;
            p[4] = b'a' + (i % 26) as u8;
            p[5] = b'/';
            p[6] = b'd';
            let rc = pintu_perbarui(t5, &p[..7], R as u32);
            if rc == 0 {
                ok += 1;
            } else if rc == -ENOMEM {
                gagal += 1;
            } else {
                x!(false); // tak terduga
            }
        }
        x!(ok > 0 && gagal > 0); // arena memang terbatas
        x!(klatos_ksm_pohon_jalur_dinyatakan_jumlah(t5) == ok);
        x!(pintu_izin(t5, b"/caaa/d") == R as u32); // simpul pertama utuh
        klatos_ksm_pohon_jalur_bebas(t5);
    }

    // --- diferensial internal: model naif vs arena ------------------------
    // Model: map jalur-dinyatakan -> (perms, dinyatakan). Oracle meniru SEMANTIK
    // C++ (longest component-prefix, propagasi lewati dinyatakan), ditulis ulang
    // sebagai kasus dinyatakan, bukan memanggil implementasi.
    let mut d = 0u32;
    let mut beda = 0u32;
    {
        let t = klatos_ksm_pohon_jalur_akar_baru();
        // himpunan (path, perms) yang diharapkan
        let kasus: [(&[u8], u32); 6] = [
            (b"/Sistem", R as u32),
            (b"/Sistem/Program", (R | X) as u32),
            (b"/Sistem/Program/Foo", (R | X | W) as u32),
            (b"/etc", R as u32),
            (b"/etc/konf", (R | C_CR) as u32),
            (b"/tmp", (R | W | C_CR | B) as u32),
        ];
        for (path, perms) in kasus {
            let rc = pintu_perbarui(t, path, perms);
            d += 1;
            if rc != 0 {
                beda += 1;
            }
        }
        // oracle: izin = izin dinyatakan terpanjang yang merupakan prefix komponen
        let oracle = |path: &[u8]| -> u32 {
            let mut best: Option<(&[u8], u32)> = None;
            for (kp, kp_p) in kasus {
                let is_prefix = path.len() >= kp.len()
                    && &path[..kp.len()] == kp
                    && (path.len() == kp.len() || path[kp.len()] == b'/');
                if is_prefix {
                    if best.is_none() || kp.len() > best.unwrap().0.len() {
                        best = Some((kp, kp_p));
                    }
                }
            }
            best.map(|(_, v)| v).unwrap_or(0)
        };
        let uji: [&[u8]; 9] = [
            b"/",
            b"/Sistem",
            b"/Sistem/Program",
            b"/Sistem/Program/Foo",
            b"/Sistem/Program/Foo/bar",
            b"/Sistem/Lain",
            b"/etc",
            b"/etc/konf",
            b"/zzz",
        ];
        for path in uji {
            let got = pintu_izin(t, path);
            let exp = oracle(path);
            d += 1;
            if got != exp {
                beda += 1;
            }
        }
        // exact-match elevasi / non-elevasi
        let rc = pintu_perbarui(t, b"/etc", (R | W) as u32);
        d += 1;
        if rc != -EPERM {
            beda += 1;
        }
        let rc = pintu_perbarui(t, b"/etc", R as u32);
        d += 1;
        if rc != 0 {
            beda += 1;
        }
        // baca ulang
        let got = pintu_izin(t, b"/etc/konf");
        d += 1;
        if got != (R | C_CR) as u32 {
            beda += 1;
        }
        klatos_ksm_pohon_jalur_bebas(t);
    }

    x!(d == 18);
    x!(beda == 0);

    KlatosKsmPohonJalurReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: beda,
        status: if c == p { 0 } else { -1 },
    }
}

/// Uji-sendiri REHOME-03.
#[no_mangle]
pub extern "C" fn klatos_ksm_pohon_jalur_selftest() -> KlatosKsmPohonJalurReport {
    run()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_414A_414C);
        assert!(N_COUNTER == 7);
        assert!(MAX_SIMPUL == 2048);
        assert!(BLOK == 64 && MAX_BLOK == 8192 && MAX_TEKS == 512 * 1024);
        assert!(MAKS_KOMP == 255 && MAKS_JALUR == 4095 && MAX_KEDALAMAN == 256);
        assert!(NONE == 0xFFFF_FFFF);
        assert!(core::mem::size_of::<KlatosKsmPohonJalurReport>() == 32);
        assert!(core::mem::align_of::<KlatosKsmPohonJalurReport>() == 8);
        assert!(Counter::Buat as usize == 0);
        assert!(Counter::Iterasi as usize == 6);
        assert!(ENOMEM == 12 && EPERM == 1);
    };
}
