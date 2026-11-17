// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! FASE-6 — **PEMILIK RUST PETA KUNCI** (`Keymap` kind, ABI v2 `Keymap::GET/SET`).


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering, AtomicU64};

/// `ENAMETOOLONG` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 34).
pub const ENAMETOOLONG: i32 = 34;
/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;

/// Panjang satu peta karakter (`CHAR_MAP_SIZE`, `CharacterMapData.h:13`).
pub const CHAR_MAP_SIZE: usize = 0x80;
/// Jumlah peta: map, shift, alt, altgr, shift+altgr.
pub const MAP_COUNT: usize = 5;
/// Total kata peta yang diserialkan.
pub const TOTAL_WORDS: usize = CHAR_MAP_SIZE * MAP_COUNT;
/// Panjang maksimum nama peta (`keymap.cpp:16`).
pub const NAME_MAX: usize = 50;

/// Tag laporan; cermin `KlatosInputKeymapReport`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4B45_594D; // "KLATKEYM"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_keymap_set` dari produksi.
    ProdSet = 0,
    /// `klatos_keymap_get` dari produksi.
    ProdGet = 1,
}

const N_COUNTER: usize = 2;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Keadaan: 5 peta + nama, dijaga spinlock atomik
// ---------------------------------------------------------------------------
//
// Tidak ada `static mut` dan tidak ada `unsafe` untuk keadaan: tiap kata adalah
// `AtomicU32`, tiap byte nama `AtomicU8`. Pola ini sama dengan
// `antrean_perangkat.rs` (P1.43): cukup, karena tidak ada interupsi yang
// dinonaktifkan dan tidak ada proses konkuren di dalam satu jalur syscall.
//
// LOCK hanya menjamin **konsistensi multi-kata**: sebuah GET tidak boleh
// melihat separuh peta lama dan separuh peta baru.

struct Peta {
    map: [AtomicU32; CHAR_MAP_SIZE],
    shift: [AtomicU32; CHAR_MAP_SIZE],
    alt: [AtomicU32; CHAR_MAP_SIZE],
    altgr: [AtomicU32; CHAR_MAP_SIZE],
    shift_altgr: [AtomicU32; CHAR_MAP_SIZE],
    nama: [AtomicU8; NAME_MAX],
    nama_len: AtomicU32,
    kunci: AtomicBool,
    /// Status interupsi pemegang kunci; hanya pemegang yang menulis.
    kunci_flags: AtomicU64,
}

impl Peta {
    const fn baru() -> Self {
        Self {
            map: [const { AtomicU32::new(0) }; CHAR_MAP_SIZE],
            shift: [const { AtomicU32::new(0) }; CHAR_MAP_SIZE],
            alt: [const { AtomicU32::new(0) }; CHAR_MAP_SIZE],
            altgr: [const { AtomicU32::new(0) }; CHAR_MAP_SIZE],
            shift_altgr: [const { AtomicU32::new(0) }; CHAR_MAP_SIZE],
            nama: [const { AtomicU8::new(0) }; NAME_MAX],
            nama_len: AtomicU32::new(0),
            kunci: AtomicBool::new(false),
            kunci_flags: AtomicU64::new(0),
        }
    }

    fn kunci(&self) {
        // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
        let flags = crate::arch::simpan_dan_matikan_interupsi();
        while self
            .kunci
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        self.kunci_flags.store(flags, Ordering::Relaxed);
    }

    fn buka(&self) {
        let flags = self.kunci_flags.load(Ordering::Relaxed);
        self.kunci.store(false, Ordering::Release);
        crate::arch::pulihkan_interupsi(flags);
    }

    fn peta(&self, which: usize) -> Option<&[AtomicU32; CHAR_MAP_SIZE]> {
        match which {
            0 => Some(&self.map),
            1 => Some(&self.shift),
            2 => Some(&self.alt),
            3 => Some(&self.altgr),
            4 => Some(&self.shift_altgr),
            _ => None,
        }
    }
}

static PROD: Peta = Peta::baru();

// ---------------------------------------------------------------------------
// Keputusan murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Verdict `setkeymap` (`keymap.cpp:18-49`).
///
/// Urutan: superuser dulu (`EPERM`), lalu panjang nama (`ENAMETOOLONG`).
///
/// **Satu otoritas.** Keputusan errno TIDAK disalin di sini: ia didelegasikan
/// ke `admisi_panggilan` (P2.09), yang sudah memutuskan EPERM/ENAMETOOLONG
/// untuk jalur syscall ABI v1. Modul ini hanya memiliki URUTAN (superuser dulu)
/// dan, di bawah, KEADAAN peta — bukan salinan kedua keputusan.
#[inline]
pub const fn set_verdict(superuser: u32, nama_len: u32) -> i32 {
    let su = crate::admisi_panggilan::bukan_superuser(if superuser == 0 { 1 } else { 0 });
    if su != 0 {
        return su;
    }
    crate::admisi_panggilan::nama_peta_panjang(if nama_len > NAME_MAX as u32 { 1 } else { 0 })
}

/// Verdict nama `getkeymap` (`keymap.cpp:59-66`): buffer pemanggil harus
/// menampung seluruh nama, selain itu `ENAMETOOLONG`. Errno-nya juga milik
/// `admisi_panggilan` (satu otoritas).
#[inline]
pub const fn get_name_verdict(buffer_size: u32, nama_len: u32) -> i32 {
    crate::admisi_panggilan::nama_peta_panjang(if buffer_size < nama_len { 1 } else { 0 })
}

/// Hak ABI v2 yang diwajibkan opcode `Keymap`. Cermin tabel dispatch beku.
#[inline]
pub const fn required_right(opcode: u8) -> u32 {
    match opcode {
        0 => crate::abi2::right::READ,  // GET
        1 => crate::abi2::right::WRITE, // SET
        _ => crate::abi2::right::NONE,
    }
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA
// ---------------------------------------------------------------------------

fn naif_set_verdict(superuser: u32, nama_len: u32) -> i32 {
    match (superuser, nama_len > NAME_MAX as u32) {
        (0, _) => -EPERM,
        (_, true) => -ENAMETOOLONG,
        _ => 0,
    }
}

fn naif_get_name_verdict(buffer_size: u32, nama_len: u32) -> i32 {
    if nama_len > buffer_size {
        -ENAMETOOLONG
    } else {
        0
    }
}

/// Kontrol NEGATIF: `set_verdict` yang memeriksa panjang nama SEBELUM
/// superuser. Berdaya pada `!superuser && nama_len > NAME_MAX`: produksi
/// `-EPERM`, yang salah `-ENAMETOOLONG`.
fn set_verdict_salah(superuser: u32, nama_len: u32) -> i32 {
    if nama_len > NAME_MAX as u32 {
        return -ENAMETOOLONG;
    }
    if superuser == 0 {
        return -EPERM;
    }
    0
}

// ---------------------------------------------------------------------------
// Operasi keadaan
// ---------------------------------------------------------------------------

/// Tulis 640 kata peta (5×128) ke `dst`. `dst.len()` harus >= [`TOTAL_WORDS`].
pub fn simpan_peta(dst: &mut [u32]) -> i32 {
    if dst.len() < TOTAL_WORDS {
        return -EINVAL;
    }
    PROD.kunci();
    let mut i = 0;
    for which in 0..MAP_COUNT {
        let peta = match PROD.peta(which) {
            Some(p) => p,
            None => {
                PROD.buka();
                return -EINVAL;
            }
        };
        for k in 0..CHAR_MAP_SIZE {
            dst[i] = peta[k].load(Ordering::Relaxed);
            i += 1;
        }
    }
    PROD.buka();
    0
}

/// Muat 640 kata peta dari `src`. `src.len()` harus >= [`TOTAL_WORDS`].
pub fn muat_peta(src: &[u32]) -> i32 {
    if src.len() < TOTAL_WORDS {
        return -EINVAL;
    }
    PROD.kunci();
    let mut i = 0;
    for which in 0..MAP_COUNT {
        let peta = match PROD.peta(which) {
            Some(p) => p,
            None => {
                PROD.buka();
                return -EINVAL;
            }
        };
        for k in 0..CHAR_MAP_SIZE {
            peta[k].store(src[i], Ordering::Relaxed);
            i += 1;
        }
    }
    PROD.buka();
    0
}

/// Salin nama aktif ke `dst`; mengembalikan panjang nama (>=0) atau -errno.
pub fn salin_nama(dst: &mut [u8]) -> i32 {
    PROD.kunci();
    let len = PROD.nama_len.load(Ordering::Acquire) as usize;
    if len > NAME_MAX {
        PROD.buka();
        return -EINVAL;
    }
    let verdict = get_name_verdict(dst.len() as u32, len as u32);
    if verdict != 0 {
        PROD.buka();
        return verdict;
    }
    for i in 0..len {
        dst[i] = PROD.nama[i].load(Ordering::Relaxed);
    }
    PROD.buka();
    len as i32
}

/// Panjang nama peta yang aktif (>=0), dibaca dari keadaan Rust. Dipakai jalur
/// ABI v2 (`mekanisme_abi2`) sebagai nilai hasil `Keymap::GET` tanpa menyalin
/// peta; salinan peta tetap dilakukan lapisan efek C++ lewat [`klatos_keymap_get`].
pub fn nama_panjang() -> u32 {
    PROD.nama_len.load(Ordering::Acquire)
}

/// Setel nama aktif. `src.len()` harus <= [`NAME_MAX`].
pub fn setel_nama(src: &[u8]) -> i32 {
    if src.len() > NAME_MAX {
        return -ENAMETOOLONG;
    }
    PROD.kunci();
    for i in 0..NAME_MAX {
        let b = if i < src.len() { src[i] } else { 0 };
        PROD.nama[i].store(b, Ordering::Relaxed);
    }
    PROD.nama_len.store(src.len() as u32, Ordering::Release);
    PROD.buka();
    0
}

// ---------------------------------------------------------------------------
// Blob kanonik peta kunci (ABI v2, F-P9-A)
// ---------------------------------------------------------------------------
//
// Blob adalah representasi byte TETAP dari keadaan: lima peta `u32[128]`
// little-endian (2560 byte) diikuti nama 50 byte (NOL-padded). Blob inilah yang
// menyeberang transport bebas-penunjuk `INPUT_KEYMAP_GET/SET` dalam POTONGAN
// (<= [`crate::dispatch::KEYMAP_CHUNK_MAX`]); pemanggil mengalamati potongan
// dengan offset blob (`arg3`).

/// Panjang bagian peta di dalam blob (byte).
pub const MAP_BYTES: usize = TOTAL_WORDS * 4;
/// Ukuran blob kanonik: peta + nama.
pub const BLOB_SIZE: usize = MAP_BYTES + NAME_MAX;

/// Baca potongan blob `dst.len()` byte mulai `off` dari keadaan.
/// Mengembalikan byte tersalin (>=0) atau -errno. Rentang di luar
/// [`BLOB_SIZE`] gagal-tertutup (`-EINVAL`), tidak dipotong diam-diam.
pub fn blob_baca(off: usize, dst: &mut [u8]) -> i32 {
    match off.checked_add(dst.len()) {
        Some(end) if end <= BLOB_SIZE => {}
        _ => return -EINVAL,
    }
    PROD.kunci();
    let mut i = 0;
    while i < dst.len() {
        let pos = off + i;
        if pos < MAP_BYTES {
            let which = pos / (CHAR_MAP_SIZE * 4);
            let word = (pos % (CHAR_MAP_SIZE * 4)) / 4;
            let peta = match PROD.peta(which) {
                Some(p) => p,
                None => {
                    PROD.buka();
                    return -EINVAL;
                }
            };
            dst[i] = (peta[word].load(Ordering::Relaxed) >> ((pos % 4) * 8)) as u8;
        } else {
            dst[i] = PROD.nama[pos - MAP_BYTES].load(Ordering::Relaxed);
        }
        i += 1;
    }
    PROD.buka();
    dst.len() as i32
}

/// Tulis potongan blob `src` pada `off` ke keadaan. Mengembalikan byte
/// tersalin (>=0) atau -errno. Rentang di luar [`BLOB_SIZE`] gagal-tertutup.
/// Setelah setiap tulis, panjang nama dihitung ulang dari NOL pertama
/// (bounded [`NAME_MAX`]) supaya `nama_panjang` tetap konsisten.
pub fn blob_tulis(off: usize, src: &[u8]) -> i32 {
    match off.checked_add(src.len()) {
        Some(end) if end <= BLOB_SIZE => {}
        _ => return -EINVAL,
    }
    PROD.kunci();
    for (i, b) in src.iter().enumerate() {
        let pos = off + i;
        if pos < MAP_BYTES {
            let which = pos / (CHAR_MAP_SIZE * 4);
            let word = (pos % (CHAR_MAP_SIZE * 4)) / 4;
            let peta = match PROD.peta(which) {
                Some(p) => p,
                None => {
                    PROD.buka();
                    return -EINVAL;
                }
            };
            let shift = ((pos % 4) * 8) as u32;
            let lama = peta[word].load(Ordering::Relaxed);
            let baru = (lama & !(0xFFu32 << shift)) | ((*b as u32) << shift);
            peta[word].store(baru, Ordering::Relaxed);
        } else {
            PROD.nama[pos - MAP_BYTES].store(*b, Ordering::Relaxed);
        }
    }
    // Panjang nama = NOL pertama (bounded NAME_MAX).
    let mut len = 0;
    while len < NAME_MAX && PROD.nama[len].load(Ordering::Relaxed) != 0 {
        len += 1;
    }
    PROD.nama_len.store(len as u32, Ordering::Release);
    PROD.buka();
    src.len() as i32
}

/// Kosongkan keadaan (dipakai uji determinisme; TIDAK dipanggil produksi).
pub fn kosongkan() {
    PROD.kunci();
    for which in 0..MAP_COUNT {
        if let Some(peta) = PROD.peta(which) {
            for k in 0..CHAR_MAP_SIZE {
                peta[k].store(0, Ordering::Relaxed);
            }
        }
    }
    for i in 0..NAME_MAX {
        PROD.nama[i].store(0, Ordering::Relaxed);
    }
    PROD.nama_len.store(0, Ordering::Release);
    PROD.buka();
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Pintu `Keymap::SET`. Memvalidasi lalu menyalin 640 kata + nama ke keadaan.
///
/// `maps` menunjuk [`TOTAL_WORDS`] `u32`; `name` menunjuk `name_len` byte.
#[no_mangle]
pub extern "C" fn klatos_keymap_set(
    superuser: u32,
    maps: *const u32,
    maps_count: u32,
    name: *const u8,
    name_len: u32,
) -> i32 {
    bump(Counter::ProdSet);
    if maps_count as usize != TOTAL_WORDS || (name_len as usize) > NAME_MAX {
        return -EINVAL;
    }
    if maps.is_null() || (name_len != 0 && name.is_null()) {
        return -EINVAL;
    }
    let verdict = set_verdict(superuser, name_len);
    if verdict != 0 {
        return verdict;
    }
    // SAFETY: pemanggil menjamin `maps` menunjuk >= 640 `u32` yang terbaca
    // (`maps_count == TOTAL_WORDS` sudah diverifikasi), dan `name` menunjuk
    // `name_len` byte yang terbaca (`name_len <= NAME_MAX` sudah diverifikasi,
    // dan `name` non-null bila `name_len != 0`).
    let map_slice = unsafe { core::slice::from_raw_parts(maps, TOTAL_WORDS) };
    let rc = muat_peta(map_slice);
    if rc != 0 {
        return rc;
    }
    let nama_slice = if name_len == 0 {
        &[][..]
    } else {
        // SAFETY: lihat di atas; `name_len` byte terbaca dan `name` non-null.
        unsafe { core::slice::from_raw_parts(name, name_len as usize) }
    };
    setel_nama(nama_slice)
}

/// Pintu `Keymap::GET`. Menyalin 640 kata + nama keluar.
///
/// Mengembalikan panjang nama (>=0) atau -errno.
#[no_mangle]
pub extern "C" fn klatos_keymap_get(
    out_maps: *mut u32,
    out_maps_cap: u32,
    out_name: *mut u8,
    out_name_cap: u32,
) -> i32 {
    bump(Counter::ProdGet);
    if out_maps.is_null() || (out_maps_cap as usize) < TOTAL_WORDS {
        return -EINVAL;
    }
    // SAFETY: `out_maps` non-null dan kapasitasnya >= TOTAL_WORDS (diverifikasi
    // di atas), jadi slice yang dibentuk seluruhnya berada di dalam objek itu.
    let map_slice = unsafe { core::slice::from_raw_parts_mut(out_maps, TOTAL_WORDS) };
    let rc = simpan_peta(map_slice);
    if rc != 0 {
        return rc;
    }
    if out_name.is_null() {
        return -EINVAL;
    }
    // SAFETY: `out_name` non-null; kapasitas `out_name_cap` byte.
    let nama_slice = unsafe { core::slice::from_raw_parts_mut(out_name, out_name_cap as usize) };
    salin_nama(nama_slice)
}

/// Hak ABI v2 yang diwajibkan opcode `Keymap` (GET=0 baca, SET=1 tulis).
#[no_mangle]
pub extern "C" fn klatos_keymap_required_right(opcode: u8) -> u32 {
    required_right(opcode)
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_keymap_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung + keadaan. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_keymap_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    kosongkan();
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosInputKeymapReport`.
#[repr(C)]
pub struct KlatosInputKeymapReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub prod_set: u32,
    pub prod_get: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Jumlah kasus diferensial yang diharapkan oleh `run_matrix`.
///
/// set: superuser 2 × nama_len ∈ {0,50,51} = 6; get: buffer ∈ {0,10,50} ×
/// nama_len ∈ {0,10} = 6 ⇒ 12.
pub const DIFF_EXPECTED: u32 = 12;

fn run_matrix() -> KlatosInputKeymapReport {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    macro_rules! catat {
        ($cond:expr) => {{
            checks += 1;
            if $cond {
                passed += 1;
            }
        }};
    }

    // --- konstanta --------------------------------------------------------
    catat!(REPORT_TAG == 0x4B4C_4154_4B45_594D);
    catat!(ENAMETOOLONG == 34 && EPERM == 1 && EINVAL == 22);
    catat!(CHAR_MAP_SIZE == 0x80 && MAP_COUNT == 5 && TOTAL_WORDS == 640 && NAME_MAX == 50);
    // Hak Keymap dari tabel dispatch yang BEKU (satu model, bukan salinan).
    catat!(
        required_right(0) == crate::abi2::right::READ
            && required_right(1) == crate::abi2::right::WRITE
    );
    catat!(
        crate::dispatch::lookup(crate::abi2::object_kind::KEYMAP, 0)
            .map(|e| e.required)
            .unwrap_or(u32::MAX)
            == crate::abi2::right::READ
    );
    catat!(
        crate::dispatch::lookup(crate::abi2::object_kind::KEYMAP, 1)
            .map(|e| e.required)
            .unwrap_or(u32::MAX)
            == crate::abi2::right::WRITE
    );

    // --- keputusan --------------------------------------------------------
    catat!(set_verdict(1, 0) == 0);
    catat!(set_verdict(1, 50) == 0);
    catat!(set_verdict(1, 51) == -ENAMETOOLONG);
    catat!(set_verdict(0, 0) == -EPERM);
    catat!(set_verdict(0, 51) == -EPERM); // superuser menang atas panjang
    catat!(get_name_verdict(50, 50) == 0);
    catat!(get_name_verdict(49, 50) == -ENAMETOOLONG);

    // --- kontrol NEGATIF: urutan EPERM/ENAMETOOLONG berdaya ----------------
    catat!(set_verdict(0, 51) == -EPERM);
    catat!(set_verdict_salah(0, 51) == -ENAMETOOLONG);
    catat!(set_verdict(0, 51) != set_verdict_salah(0, 51));
    catat!(set_verdict(1, 51) == set_verdict_salah(1, 51)); // sama saat superuser

    // --- diferensial produksi vs oracle ----------------------------------
    for su in 0..2u32 {
        for nl in [0u32, 50, 51] {
            if set_verdict(su, nl) != naif_set_verdict(su, nl) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    for buf in [0u32, 10, 50] {
        for nl in [0u32, 10] {
            if get_name_verdict(buf, nl) != naif_get_name_verdict(buf, nl) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    catat!(diukur == DIFF_EXPECTED);
    catat!(beda == 0);

    // --- siklus keadaan NYATA (round-trip) --------------------------------
    {
        klatos_keymap_reset();
        let mut maps = [0u32; TOTAL_WORDS];
        for (i, m) in maps.iter_mut().enumerate() {
            *m = (i as u32).wrapping_mul(3).wrapping_add(1);
        }
        let nama = b"klat-us";
        let set_rc = klatos_keymap_set(1, maps.as_ptr(), TOTAL_WORDS as u32, nama.as_ptr(), nama.len() as u32);
        catat!(set_rc == 0);

        let mut out_maps = [0u32; TOTAL_WORDS];
        let mut out_nama = [0u8; NAME_MAX];
        let get_rc = klatos_keymap_get(
            out_maps.as_mut_ptr(),
            TOTAL_WORDS as u32,
            out_nama.as_mut_ptr(),
            NAME_MAX as u32,
        );
        catat!(get_rc == nama.len() as i32);
        catat!(out_maps == maps);
        catat!(&out_nama[..nama.len()] == nama);

        // determinisme: GET kedua identik
        let mut out2 = [0u32; TOTAL_WORDS];
        let mut out2n = [0u8; NAME_MAX];
        let get_rc2 = klatos_keymap_get(
            out2.as_mut_ptr(),
            TOTAL_WORDS as u32,
            out2n.as_mut_ptr(),
            NAME_MAX as u32,
        );
        catat!(get_rc2 == get_rc && out2 == out_maps);

        // SET tanpa superuser tidak mengubah keadaan
        let mut maps2 = maps;
        maps2[0] = 0xDEAD_BEEF;
        let bad = klatos_keymap_set(0, maps2.as_ptr(), TOTAL_WORDS as u32, nama.as_ptr(), nama.len() as u32);
        catat!(bad == -EPERM);
        let mut out3 = [0u32; TOTAL_WORDS];
        let mut out3n = [0u8; NAME_MAX];
        let _ = klatos_keymap_get(out3.as_mut_ptr(), TOTAL_WORDS as u32, out3n.as_mut_ptr(), NAME_MAX as u32);
        catat!(out3 == maps); // tidak berubah

        // buffer nama terlalu kecil -> ENAMETOOLONG, peta tetap tersalin
        let mut out4 = [0u32; TOTAL_WORDS];
        let mut small = [0u8; 4];
        let rc4 = klatos_keymap_get(out4.as_mut_ptr(), TOTAL_WORDS as u32, small.as_mut_ptr(), 4);
        catat!(rc4 == -ENAMETOOLONG);
        catat!(out4 == maps);

        // --- blob ABI v2 round-trip (F-P9-A) ------------------------------
        // GET seluruh blob harus mengembalikan 640 kata peta LE + nama
        // NOL-padded; SET potongan harus merekonstruksi keadaan yang sama.
        let mut blob = [0u8; BLOB_SIZE];
        catat!(blob_baca(0, &mut blob) == BLOB_SIZE as i32);
        let mut kata = [0u32; TOTAL_WORDS];
        for (i, w) in kata.iter_mut().enumerate() {
            let b0 = blob[i * 4] as u32;
            let b1 = blob[i * 4 + 1] as u32;
            let b2 = blob[i * 4 + 2] as u32;
            let b3 = blob[i * 4 + 3] as u32;
            *w = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
        }
        catat!(kata == maps);
        catat!(&blob[MAP_BYTES..MAP_BYTES + nama.len()] == nama);
        // Rentang melewati batas gagal-tertutup.
        let mut kecil = [0u8; 4];
        catat!(blob_baca(BLOB_SIZE - 2, &mut kecil) == -EINVAL);
        catat!(blob_tulis(BLOB_SIZE, &kecil) == -EINVAL);
        // SET potongan REGION NAMA utuh (50 byte, NOL-padded) lalu baca ulang:
        // nama menjadi "AB", peta tetap. `NAME_MAX` (50) < `KEYMAP_CHUNK_MAX`
        // (256), jadi region nama SELALU tiba dalam satu potongan utuh (NUL-
        // padded) — itulah representasi kawat sebenarnya dari protokol SET.
        let mut potong_nama = [0u8; NAME_MAX];
        potong_nama[0] = b'A';
        potong_nama[1] = b'B';
        catat!(blob_tulis(MAP_BYTES, &potong_nama) == NAME_MAX as i32);
        catat!(nama_panjang() == 2);
        let mut blob2 = [0u8; BLOB_SIZE];
        catat!(blob_baca(0, &mut blob2) == BLOB_SIZE as i32);
        catat!(&blob2[MAP_BYTES..MAP_BYTES + 2] == b"AB");
        catat!(&blob2[..MAP_BYTES] == &blob[..MAP_BYTES]);
    }

    let status = if checks == passed { 0 } else { -1 };
    KlatosInputKeymapReport {
        tag: REPORT_TAG,
        checks,
        passed,
        prod_set: klatos_keymap_counter(Counter::ProdSet as u32),
        prod_get: klatos_keymap_counter(Counter::ProdGet as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks pemilik peta kunci.
#[no_mangle]
pub extern "C" fn klatos_keymap_selftest() -> KlatosInputKeymapReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4B45_594D);
        assert!(ENAMETOOLONG == 34);
        assert!(EPERM == 1);
        assert!(EINVAL == 22);
        assert!(CHAR_MAP_SIZE == 0x80);
        assert!(MAP_COUNT == 5);
        assert!(TOTAL_WORDS == 640);
        assert!(NAME_MAX == 50);
        assert!(Counter::ProdSet as usize == 0);
        assert!(Counter::ProdGet as usize == 1);
        assert!(N_COUNTER == 2);
        assert!(DIFF_EXPECTED == 12);

        // Hak Keymap harus sama dengan tabel dispatch beku.
        assert!(required_right(0) == crate::abi2::right::READ);
        assert!(required_right(1) == crate::abi2::right::WRITE);

        // Blob ABI v2 (F-P9-A): 2560 + 50 = 2610 byte.
        assert!(MAP_BYTES == 2560);
        assert!(BLOB_SIZE == 2610);
        assert!(BLOB_SIZE == crate::dispatch::KEYMAP_BLOB_SIZE);
        assert!(MAP_BYTES == crate::dispatch::KEYMAP_NAME_OFFSET);

        // Laporan: 8 (tag) + 7*4 (checks/passed/prod_set/prod_get/diff_diukur/
        // diff_beda/status) = 36 -> align 8 -> 40.
        assert!(core::mem::size_of::<KlatosInputKeymapReport>() == 40);
        assert!(core::mem::align_of::<KlatosInputKeymapReport>() == 8);
    };
}
