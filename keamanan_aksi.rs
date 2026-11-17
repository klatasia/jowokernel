// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.102 / REHOME-02 — **otoritas AKSI & KLASIFIKASI `Jowo/Keamanan`**.


#![allow(unsafe_code)]
use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosKeamananAksiReport` di `KlatSeam114.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4B4D_4E41; // "KLATKMNA"

// ---------------------------------------------------------------------------
// Konstanta verdict
// ---------------------------------------------------------------------------

/// UBSAN: `SourceLocation` punya nama berkas.
pub const LOKASI_ADA: u32 = 0;
/// UBSAN: `SourceLocation` tanpa nama berkas.
pub const LOKASI_TAK_ADA: u32 = 1;

/// UBSAN `pointer_overflow`: `base == 0 && result == 0`.
pub const PTR_NOL_NOL: u32 = 0;
/// UBSAN `pointer_overflow`: `base == 0 && result != 0`.
pub const PTR_NOL_HASIL: u32 = 1;
/// UBSAN `pointer_overflow`: `base != 0 && result == 0`.
pub const PTR_HASIL_NOL: u32 = 2;
/// UBSAN `pointer_overflow`: keduanya non-nol (luapan).
pub const PTR_LUAPAN: u32 = 3;

/// UBSAN `type_mismatch`: `ptr == 0`.
pub const TIPE_NOL: u32 = 0;
/// UBSAN `type_mismatch`: alamat tidak selaras.
pub const TIPE_MISALIGN: u32 = 1;
/// UBSAN `type_mismatch`: ruang tidak cukup.
pub const TIPE_RUANG: u32 = 2;

/// UBSAN `alignment_assumption`: tanpa offset.
pub const ALIGN_TANPA_OFFSET: u32 = 0;
/// UBSAN `alignment_assumption`: dengan offset.
pub const ALIGN_DENGAN_OFFSET: u32 = 1;

// ---------------------------------------------------------------------------
// Penghitung audit
// ---------------------------------------------------------------------------

/// Penghitung per-pintu (audit produksi).
#[derive(Clone, Copy)]
pub enum Counter {
    DalamGrup = 0,
    JanjiBit = 1,
    KasanMematikan = 2,
    AksesNama = 3,
    BayanganNama = 4,
    UbsanLokasi = 5,
    UbsanMematikan = 6,
    UbsanPointer = 7,
    UbsanTipe = 8,
    UbsanAlign = 9,
    UbsanKind = 10,
}
/// Jumlah penghitung.
pub const N_COUNTER: usize = 11;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Keputusan murni
// ---------------------------------------------------------------------------

/// `Credentials::in_group(gid)`: `gid == primary` ATAU `gid ∈ extra`.
#[inline]
pub fn dalam_grup(gid: u32, primary: u32, extra: &[u32]) -> u32 {
    if gid == primary {
        return 1;
    }
    let mut i = 0usize;
    while i < extra.len() {
        if extra[i] == gid {
            return 1;
        }
        i += 1;
    }
    0
}

/// Tabel nama janji, urutannya **dibekukan oleh ABI** `Pledge`
/// (`KlatKernel/Efek/Eksekusi/Process.h:106-110`, `ENUMERATE_KSM_PERMISSIONS`).
const NAMA_JANJI: [&[u8]; 28] = [
    b"stdio",
    b"rpath",
    b"wpath",
    b"cpath",
    b"dpath",
    b"inet",
    b"id",
    b"proc",
    b"ptrace",
    b"exec",
    b"unix",
    b"recvfd",
    b"sendfd",
    b"fattr",
    b"tty",
    b"chown",
    b"thread",
    b"video",
    b"accept",
    b"settime",
    b"sigaction",
    b"setkeymap",
    b"prot_exec",
    b"map_fixed",
    b"getkeymap",
    b"mount",
    b"unshare",
    b"no_error",
];

/// Perbandingan byte (tanpa `str`/`alloc`).
#[inline]
fn sama(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0usize;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Nama janji → bit (`1 << indeks`), atau `-1` bila tak dikenal.
#[inline]
pub fn janji_bit(nama: &[u8]) -> i64 {
    let mut i = 0usize;
    while i < NAMA_JANJI.len() {
        if sama(nama, NAMA_JANJI[i]) {
            return 1i64 << i;
        }
        i += 1;
    }
    -1
}

/// KASAN `g_kasan_is_deadly` → hentikan sistem (`1`) atau tidak (`0`).
#[inline]
pub const fn kasan_mematikan(deadly: u32) -> u32 {
    if deadly != 0 {
        1
    } else {
        0
    }
}

/// UBSAN `print_location`: apakah `location.filename()` ADA (non-null).
///
/// C++ asli: `if (!location.filename()) ... else ...` — jadi `ada_nama == 0`
/// berarti "berkas tak dikenal".
#[inline]
pub const fn ubsan_lokasi(ada_nama: u32) -> u32 {
    if ada_nama != 0 {
        LOKASI_ADA
    } else {
        LOKASI_TAK_ADA
    }
}

/// UBSAN `g_ubsan_is_deadly` → hentikan sistem (`1`) atau tidak (`0`).
#[inline]
pub const fn ubsan_mematikan(deadly: u32) -> u32 {
    if deadly != 0 {
        1
    } else {
        0
    }
}

/// UBSAN `__ubsan_handle_pointer_overflow` klasifikasi `(base, result)`.
///
/// `base_nol` = (`base == 0`), `hasil_nol` = (`result == 0`). Cabang mengikuti
/// C++ asli PERSIS (urutan `if`/`else if`/`else`).
#[inline]
pub const fn ubsan_pointer_kelas(base_nol: u32, hasil_nol: u32) -> u32 {
    if base_nol != 0 && hasil_nol != 0 {
        PTR_NOL_NOL
    } else if base_nol != 0 && hasil_nol == 0 {
        PTR_NOL_HASIL
    } else if base_nol == 0 && hasil_nol != 0 {
        PTR_HASIL_NOL
    } else {
        PTR_LUAPAN
    }
}

/// UBSAN `__ubsan_handle_type_mismatch_v1` klasifikasi `(ptr, alignment)`.
#[inline]
pub const fn ubsan_tipe_kelas(ptr: u64, alignment: u64) -> u32 {
    if ptr == 0 {
        TIPE_NOL
    } else if (ptr & alignment.wrapping_sub(1)) != 0 {
        TIPE_MISALIGN
    } else {
        TIPE_RUANG
    }
}

/// UBSAN `__ubsan_handle_alignment_assumption`: apakah `offset` ADA (non-null).
///
/// C++ asli: `if (offset) ... else ...`.
#[inline]
pub const fn ubsan_align_kelas(ada_offset: u32) -> u32 {
    if ada_offset != 0 {
        ALIGN_DENGAN_OFFSET
    } else {
        ALIGN_TANPA_OFFSET
    }
}

// ---------------------------------------------------------------------------
// Tabel deskripsi (KASAN) — `KlatosStr` menunjuk `.rodata` Rust (`'static`)
// ---------------------------------------------------------------------------

/// Potongan byte read-only; cermin `KlatosStr` di `KlatSeam114.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosStr {
    pub ptr: *const u8,
    pub len: usize,
}

#[inline]
const fn str_of(s: &'static [u8]) -> KlatosStr {
    KlatosStr {
        ptr: s.as_ptr(),
        len: s.len(),
    }
}

/// `to_string(AccessType)`: `Load` / `Store` / `Unknown`.
#[inline]
pub const fn akses_nama(kelas: u32) -> KlatosStr {
    let s: &'static [u8] = match kelas {
        0 => b"Load",
        1 => b"Store",
        _ => b"Unknown",
    };
    str_of(s)
}

/// `to_string(ShadowType)`: deskripsi shadow byte, atau `Unknown`.
#[inline]
pub const fn bayangan_nama(t: u32) -> KlatosStr {
    let s: &'static [u8] = match t {
        0x00 => b"8 Bytes Unpoisoned",
        0x01 => b"1 Byte Unpoisoned | 7 Bytes Poisoned",
        0x02 => b"2 Bytes Unpoisoned | 6 Bytes Poisoned",
        0x03 => b"3 Bytes Unpoisoned | 5 Bytes Poisoned",
        0x04 => b"4 Bytes Unpoisoned | 4 Bytes Poisoned",
        0x05 => b"5 Bytes Unpoisoned | 3 Bytes Poisoned",
        0x06 => b"6 Bytes Unpoisoned | 2 Bytes Poisoned",
        0x07 => b"7 Bytes Unpoisoned | 1 Byte Poisoned",
        0xF1 => b"Stack Left Redzone",
        0xF2 => b"Stack Middle Redzone",
        0xF3 => b"Stack Right Redzone",
        0xF5 => b"Use After Return",
        0xF8 => b"Use After Scope",
        0xFA => b"Generic Redzone",
        0xFB => b"Malloc Redzone",
        0xFC => b"Freed Region",
        _ => b"Unknown",
    };
    str_of(s)
}

/// Tabel `kinds[data.type_check_kind]` UBSAN (`UBSanitizer.cpp`), 12 entri.
const UBSAN_KINDS: [&[u8]; 12] = [
    b"load of",
    b"store to",
    b"reference binding to",
    b"member access within",
    b"member call on",
    b"constructor call on",
    b"downcast of",
    b"downcast of",
    b"upcast of",
    b"cast to virtual base of",
    b"_Nonnull binding to",
    b"dynamic operation on",
];

/// `kinds[data.type_check_kind]`. Indeks di luar tabel mengembalikan `"unknown"`
/// (C++ historis mengindeks di luar batas — perilaku tak terdefinisi).
pub fn ubsan_kind(kelas: u32) -> KlatosStr {
    if (kelas as usize) < UBSAN_KINDS.len() {
        str_of(UBSAN_KINDS[kelas as usize])
    } else {
        str_of(b"unknown")
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// `Credentials::in_group`. `extra_ptr` menunjuk `extra_len` buah `u32`.
///
/// # Safety
/// Pemanggil (C++) menjamin `extra_ptr` sah untuk `extra_len` `u32` selama
/// pemanggilan, atau `extra_len == 0`/`extra_ptr == null`.
#[no_mangle]
pub unsafe extern "C" fn klatos_keamanan_dalam_grup(
    gid: u32,
    primary: u32,
    extra_ptr: *const u32,
    extra_len: usize,
) -> u32 {
    bump(Counter::DalamGrup);
    let extra: &[u32] = if extra_ptr.is_null() || extra_len == 0 {
        &[]
    } else {
        // SAFETY: kontrak pemanggil di atas.
        unsafe { core::slice::from_raw_parts(extra_ptr, extra_len) }
    };
    dalam_grup(gid, primary, extra)
}

/// Cermin `KlatosJanjiParse` di `KlatSeam114.h` (2 `u32`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosJanjiParse {
    /// Topeng bit janji: OR dari bit setiap token yang dikenal.
    pub mask: u32,
    /// 1 bila SEMUA token dikenal (setara `found_invalid_pledge == true`).
    pub ok: u32,
}

/// Seluruh MEKANISME penguraian spesifikasi janji.
///
/// C++ historis memisah dengan `for_each_split_view(' ', SplitBehavior::Nothing, …)`
/// lalu memetakan tiap potongan. Kini pemisahan itu **milik Rust**:
/// token kosong dilewati (sama dengan `SplitBehavior::Nothing`), setiap token
/// dikenal di-OR ke `mask`, dan satu token tak dikenal membuat `ok = 0`
/// (pemrosesan tetap lanjut, persis seperti loop C++ yang hanya menyetel
/// `found_invalid_pledge = false`).
pub fn janji_parse(spec: &[u8]) -> KlatosJanjiParse {
    let mut mask = 0u32;
    let mut ok = 1u32;
    let mut i = 0usize;
    while i < spec.len() {
        while i < spec.len() && spec[i] == b' ' {
            i += 1;
        }
        if i >= spec.len() {
            break;
        }
        let mulai = i;
        while i < spec.len() && spec[i] != b' ' {
            i += 1;
        }
        let bit = janji_bit(&spec[mulai..i]);
        if bit < 0 {
            ok = 0;
        } else {
            mask |= bit as u32;
        }
    }
    KlatosJanjiParse { mask, ok }
}

/// Menguraikan spesifikasi janji (`ptr`/`len` = byte ASCII yang sudah disalin
/// C++ dari ruang pengguna).
///
/// # Safety
/// Pemanggil menjamin `ptr` sah untuk `len` byte selama pemanggilan, atau
/// `len == 0`/`ptr == null`.
#[no_mangle]
pub unsafe extern "C" fn klatos_keamanan_janji_parse(ptr: *const u8, len: usize) -> KlatosJanjiParse {
    bump(Counter::JanjiBit);
    let spec: &[u8] = if ptr.is_null() || len == 0 {
        &[]
    } else {
        // SAFETY: kontrak pemanggil di atas.
        unsafe { core::slice::from_raw_parts(ptr, len) }
    };
    janji_parse(spec)
}

/// KASAN kebijakan `g_kasan_is_deadly`.
#[no_mangle]
pub extern "C" fn klatos_keamanan_kasan_mematikan(deadly: u32) -> u32 {
    bump(Counter::KasanMematikan);
    kasan_mematikan(deadly)
}

/// KASAN `to_string(AccessType)`.
#[no_mangle]
pub extern "C" fn klatos_keamanan_akses_nama(kelas: u32) -> KlatosStr {
    bump(Counter::AksesNama);
    akses_nama(kelas)
}

/// KASAN `to_string(ShadowType)`.
#[no_mangle]
pub extern "C" fn klatos_keamanan_bayangan_nama(t: u32) -> KlatosStr {
    bump(Counter::BayanganNama);
    bayangan_nama(t)
}

/// UBSAN `kinds[type_check_kind]`.
#[no_mangle]
pub extern "C" fn klatos_keamanan_ubsan_kind(kelas: u32) -> KlatosStr {
    bump(Counter::UbsanKind);
    ubsan_kind(kelas)
}

/// UBSAN lokasi: apakah nama berkas ADA (`1`) atau tidak (`0`).
#[no_mangle]
pub extern "C" fn klatos_keamanan_ubsan_lokasi(ada_nama: u32) -> u32 {
    bump(Counter::UbsanLokasi);
    ubsan_lokasi(ada_nama)
}

/// UBSAN kebijakan `g_ubsan_is_deadly`.
#[no_mangle]
pub extern "C" fn klatos_keamanan_ubsan_mematikan(deadly: u32) -> u32 {
    bump(Counter::UbsanMematikan);
    ubsan_mematikan(deadly)
}

/// UBSAN `pointer_overflow` klasifikasi.
#[no_mangle]
pub extern "C" fn klatos_keamanan_ubsan_pointer_kelas(base_nol: u32, hasil_nol: u32) -> u32 {
    bump(Counter::UbsanPointer);
    ubsan_pointer_kelas(base_nol, hasil_nol)
}

/// UBSAN `type_mismatch` klasifikasi.
#[no_mangle]
pub extern "C" fn klatos_keamanan_ubsan_tipe_kelas(ptr: u64, alignment: u64) -> u32 {
    bump(Counter::UbsanTipe);
    ubsan_tipe_kelas(ptr, alignment)
}

/// UBSAN `alignment_assumption` klasifikasi: apakah offset ADA (`1`).
#[no_mangle]
pub extern "C" fn klatos_keamanan_ubsan_align_kelas(ada_offset: u32) -> u32 {
    bump(Counter::UbsanAlign);
    ubsan_align_kelas(ada_offset)
}

/// Membaca penghitung audit `i` (0 bila di luar rentang).
#[no_mangle]
pub extern "C" fn klatos_keamanan_aksi_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Reset seluruh penghitung; mengembalikan jumlahnya.
#[no_mangle]
pub extern "C" fn klatos_keamanan_aksi_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan
// ---------------------------------------------------------------------------

/// Laporan P1.102; cermin `KlatosKeamananAksiReport` di `KlatSeam114.h`.
#[repr(C)]
pub struct KlatosKeamananAksiReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn run() -> KlatosKeamananAksiReport {
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
    x!(REPORT_TAG == 0x4B4C_4154_4B4D_4E41);
    x!(NAMA_JANJI.len() == 28);
    x!(N_COUNTER == 11);
    x!(core::mem::size_of::<KlatosKeamananAksiReport>() == 32);
    x!(core::mem::size_of::<KlatosStr>() == 16);

    // --- dalam_grup --------------------------------------------------------
    x!(dalam_grup(5, 5, &[]) == 1);
    x!(dalam_grup(5, 4, &[5]) == 1);
    x!(dalam_grup(5, 4, &[1, 2, 3]) == 0);
    x!(dalam_grup(0, 0, &[]) == 1);
    // kontrol negatif: gid asing di primary & extra
    x!(dalam_grup(7, 1, &[2, 3]) == 0);

    // --- janji_bit ---------------------------------------------------------
    x!(janji_bit(b"stdio") == 1);
    x!(janji_bit(b"no_error") == (1i64 << 27));
    x!(janji_bit(b"prot_exec") == (1i64 << 22));
    x!(janji_bit(b"") == -1);
    x!(janji_bit(b"x") == -1);
    // kontrol negatif: peka huruf besar/kecil, tanpa prefiks
    x!(janji_bit(b"stdioX") == -1);
    x!(janji_bit(b"No_Error") == -1);
    x!(janji_bit(b" no_error") == -1);

    // --- janji_parse (mekanisme penguraian) --------------------------------
    let p1 = janji_parse(b"stdio");
    x!(p1.mask == 1 && p1.ok == 1);
    let p2 = janji_parse(b"stdio rpath");
    x!(p2.mask == 0b11 && p2.ok == 1);
    // token kosong (spasi berlebih / di tepi) DILEWATI, seperti SplitBehavior::Nothing
    let p3 = janji_parse(b"  stdio   no_error  ");
    x!(p3.mask == (1u32 | (1u32 << 27)) && p3.ok == 1);
    // satu token tak dikenal -> ok = 0, tapi bit token dikenal tetap terkumpul
    let p4 = janji_parse(b"stdio bogus");
    x!(p4.mask == 1 && p4.ok == 0);
    let p5 = janji_parse(b"bogus");
    x!(p5.mask == 0 && p5.ok == 0);
    // spesifikasi kosong -> tidak ada token -> ok = 1
    let p6 = janji_parse(b"");
    x!(p6.mask == 0 && p6.ok == 1);
    let p7 = janji_parse(b"   ");
    x!(p7.mask == 0 && p7.ok == 1);
    // tanpa prefiks/sufiks implisit
    x!(janji_parse(b" stdioX").ok == 0);
    x!(janji_parse(b"stdioX ").ok == 0);

    // --- ubsan_kind --------------------------------------------------------
    x!(ubsan_kind(0).len == 7); // "load of"
    x!(ubsan_kind(11).len == 20); // "dynamic operation on"
    x!(ubsan_kind(12).len == 7); // "unknown"
    x!(ubsan_kind(999).len == 7);

    // --- KASAN (kebijakan + tabel; mekanisme shadow di `kasan_bayangan.rs`) --
    x!(kasan_mematikan(0) == 0);
    x!(kasan_mematikan(1) == 1);
    x!(kasan_mematikan(7) == 1);

    // --- tabel deskripsi KASAN --------------------------------------------
    x!(akses_nama(0).len == 4);
    x!(akses_nama(1).len == 5);
    x!(akses_nama(2).len == 7);
    x!(bayangan_nama(0x00).len == 18);
    x!(bayangan_nama(0xF1).len == 18);
    x!(bayangan_nama(0xFC).len == 12);
    x!(bayangan_nama(0x99).len == 7);

    // --- UBSAN -------------------------------------------------------------
    // `ada_nama == 0` (berkas tak dikenal) -> TAK_ADA
    x!(ubsan_lokasi(0) == LOKASI_TAK_ADA);
    x!(ubsan_lokasi(1) == LOKASI_ADA);
    x!(ubsan_mematikan(0) == 0);
    x!(ubsan_mematikan(1) == 1);
    // (base_nol, hasil_nol): (1,1) -> NOL_NOL dst.
    x!(ubsan_pointer_kelas(1, 1) == PTR_NOL_NOL);
    x!(ubsan_pointer_kelas(1, 0) == PTR_NOL_HASIL);
    x!(ubsan_pointer_kelas(0, 1) == PTR_HASIL_NOL);
    x!(ubsan_pointer_kelas(0, 0) == PTR_LUAPAN);
    x!(ubsan_tipe_kelas(0, 8) == TIPE_NOL);
    x!(ubsan_tipe_kelas(0x1001, 8) == TIPE_MISALIGN);
    x!(ubsan_tipe_kelas(0x1000, 8) == TIPE_RUANG);
    // alignment == 1 -> alignment-1 == 0 -> selalu TIPE_RUANG (kecuali null)
    x!(ubsan_tipe_kelas(0x1234, 1) == TIPE_RUANG);
    // ada_offset == 0 -> TANPA_OFFSET
    x!(ubsan_align_kelas(0) == ALIGN_TANPA_OFFSET);
    x!(ubsan_align_kelas(1) == ALIGN_DENGAN_OFFSET);

    // --- diferensial -------------------------------------------------------
    let mut d = 0u32;
    let mut b = 0u32;
    macro_rules! q {
        ($got:expr, $exp:expr) => {{
            d += 1;
            if $got != $exp {
                b += 1;
            }
        }};
    }

    // dalam_grup: 6 gid × 3 primary × 3 himpunan-extra = 54
    for gid in 0..6u32 {
        for primary in 0..3u32 {
            let e0: [u32; 0] = [];
            let e1: [u32; 1] = [gid];
            let e2: [u32; 1] = [9];
            for ekstra in [&e0[..], &e1[..], &e2[..]] {
                let got = dalam_grup(gid, primary, ekstra);
                let mut exp = 0u32;
                if gid == primary {
                    exp = 1;
                } else {
                    let mut j = 0usize;
                    while j < ekstra.len() {
                        if ekstra[j] == gid {
                            exp = 1;
                        }
                        j += 1;
                    }
                }
                q!(got, exp);
            }
        }
    }

    // janji_bit: 28 nama + 3 tak dikenal = 31
    let mut i = 0usize;
    while i < NAMA_JANJI.len() {
        q!(janji_bit(NAMA_JANJI[i]), 1i64 << i);
        i += 1;
    }
    q!(janji_bit(b"nope"), -1);
    q!(janji_bit(b""), -1);
    q!(janji_bit(b"stdioX"), -1);

    // janji_parse: 28 tunggal × 2 medan = 56, + 4 kasus majemuk × 2 = 8 -> 64
    let mut i = 0usize;
    while i < NAMA_JANJI.len() {
        let r = janji_parse(NAMA_JANJI[i]);
        q!(r.mask, 1u32 << i);
        q!(r.ok, 1u32);
        i += 1;
    }
    let kosong = janji_parse(b"");
    q!(kosong.mask, 0u32);
    q!(kosong.ok, 1u32);
    let dua = janji_parse(b"stdio rpath");
    q!(dua.mask, 0b11u32);
    q!(dua.ok, 1u32);
    let campur = janji_parse(b"stdio bogus");
    q!(campur.mask, 1u32);
    q!(campur.ok, 0u32);
    let spasi = janji_parse(b"  stdio  ");
    q!(spasi.mask, 1u32);
    q!(spasi.ok, 1u32);

    // ubsan_kind: 12 dikenal + 1 di luar = 13
    for k in 0..12u32 {
        let s = ubsan_kind(k);
        let got: &[u8] = unsafe { core::slice::from_raw_parts(s.ptr, s.len) };
        q!(got, UBSAN_KINDS[k as usize]);
    }
    let luar = ubsan_kind(12);
    let luar_b: &[u8] = unsafe { core::slice::from_raw_parts(luar.ptr, luar.len) };
    q!(luar_b, &b"unknown"[..]);

    // kasan_lewati / kasan_ukuran_kelas: dipindah ke `kasan_bayangan.rs` (P1.103)

    // kasan_mematikan: 2
    for v in [0u32, 1] {
        q!(kasan_mematikan(v), if v != 0 { 1 } else { 0 });
    }

    // akses_nama: 4 (bandingkan byte)
    let akses_harap: [&[u8]; 4] = [b"Load", b"Store", b"Unknown", b"Unknown"];
    for k in 0..4u32 {
        let s = akses_nama(k);
        let got: &[u8] = unsafe { core::slice::from_raw_parts(s.ptr, s.len) };
        q!(got, akses_harap[k as usize]);
    }

    // bayangan_nama: 17 (16 nilai dikenal + 1 tak dikenal)
    let bayangan_harap: [&[u8]; 17] = [
        b"8 Bytes Unpoisoned",
        b"1 Byte Unpoisoned | 7 Bytes Poisoned",
        b"2 Bytes Unpoisoned | 6 Bytes Poisoned",
        b"3 Bytes Unpoisoned | 5 Bytes Poisoned",
        b"4 Bytes Unpoisoned | 4 Bytes Poisoned",
        b"5 Bytes Unpoisoned | 3 Bytes Poisoned",
        b"6 Bytes Unpoisoned | 2 Bytes Poisoned",
        b"7 Bytes Unpoisoned | 1 Byte Poisoned",
        b"Stack Left Redzone",
        b"Stack Middle Redzone",
        b"Stack Right Redzone",
        b"Use After Return",
        b"Use After Scope",
        b"Generic Redzone",
        b"Malloc Redzone",
        b"Freed Region",
        b"Unknown",
    ];
    let bayangan_masukan: [u32; 17] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0xF1, 0xF2, 0xF3, 0xF5, 0xF8, 0xFA, 0xFB,
        0xFC, 0x99,
    ];
    for k in 0..17usize {
        let s = bayangan_nama(bayangan_masukan[k]);
        let got: &[u8] = unsafe { core::slice::from_raw_parts(s.ptr, s.len) };
        q!(got, bayangan_harap[k]);
    }

    // ubsan_lokasi: 2 (oracle independen: `ada_nama != 0` -> ADA)
    for v in [0u32, 1] {
        q!(ubsan_lokasi(v), if v != 0 { LOKASI_ADA } else { LOKASI_TAK_ADA });
    }
    // ubsan_mematikan: 2
    for v in [0u32, 1] {
        q!(ubsan_mematikan(v), if v != 0 { 1 } else { 0 });
    }
    // ubsan_pointer_kelas: 4 — oracle independen, ditulis dari kondisi C++ asli
    // (`base == 0 && result == 0` dst.), BUKAN dari implementasi Rust.
    q!(ubsan_pointer_kelas(1, 1), PTR_NOL_NOL); // base==0, result==0
    q!(ubsan_pointer_kelas(1, 0), PTR_NOL_HASIL); // base==0, result!=0
    q!(ubsan_pointer_kelas(0, 1), PTR_HASIL_NOL); // base!=0, result==0
    q!(ubsan_pointer_kelas(0, 0), PTR_LUAPAN); // base!=0, result!=0
    // ubsan_tipe_kelas: 5 ptr × 5 alignment = 25
    for ptr in [0u64, 1, 8, 0x1000, 0x1001] {
        for align in [1u64, 2, 4, 8, 16] {
            let got = ubsan_tipe_kelas(ptr, align);
            let exp = if ptr == 0 {
                TIPE_NOL
            } else if (ptr & (align - 1)) != 0 {
                TIPE_MISALIGN
            } else {
                TIPE_RUANG
            };
            q!(got, exp);
        }
    }
    // ubsan_align_kelas: 2 (oracle independen: `ada_offset != 0` -> DENGAN)
    for v in [0u32, 1] {
        q!(ubsan_align_kelas(v), if v != 0 { ALIGN_DENGAN_OFFSET } else { ALIGN_TANPA_OFFSET });
    }

    x!(d == 220);
    x!(b == 0);

    KlatosKeamananAksiReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: b,
        status: if c == p { 0 } else { -1 },
    }
}

/// Selftest P1.102.
#[no_mangle]
pub extern "C" fn klatos_keamanan_aksi_selftest() -> KlatosKeamananAksiReport {
    run()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4B4D_4E41);
        assert!(N_COUNTER == 11);
        assert!(core::mem::size_of::<KlatosKeamananAksiReport>() == 32);
        assert!(core::mem::align_of::<KlatosKeamananAksiReport>() == 8);
        assert!(core::mem::size_of::<KlatosStr>() == 16);
        assert!(core::mem::align_of::<KlatosStr>() == 8);
        assert!(NAMA_JANJI.len() == 28);
        assert!(Counter::DalamGrup as usize == 0);
        assert!(Counter::UbsanKind as usize == 10);
        assert!(UBSAN_KINDS.len() == 12);
    };
}
