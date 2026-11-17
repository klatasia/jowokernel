// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.59 — **OTORITAS KEPUTUSAN PENEMPATAN RUANG WILAYAH**.


use core::sync::atomic::{AtomicU32, Ordering};

/// Batas nilai yang TIDAK mungkin menjadi alamat sah: setiap basis nyata
/// `< kernel_mapping_base < 2^48`, jadi `u64::MAX` bebas dipakai sebagai
/// penanda "tidak ada hasil". Berbeda dari [`crate::keadaan_wilayah`] yang
/// memakai kanal validitas terpisah karena `which` di sana bernilai kecil.
pub const TAK_SAH: u64 = 0xFFFF_FFFF_FFFF_FFFF;

/// Ukuran halaman; cermin `PAGE_SIZE`.
pub const HALAMAN: u64 = 4096;

// --- status `klatos_tempat_spesifik` ---------------------------------------

/// Penempatan spesifik diterima.
pub const TS_OK: i32 = 0;
/// Di luar rentang total (C++: `ENOMEM`).
pub const TS_LUAR: i32 = 1;
/// Bertumpang dengan tetangga (C++: `ENOMEM`).
pub const TS_TUMPANG: i32 = 2;
/// Argumen tidak sah (C++: `EINVAL`).
pub const TS_ARG: i32 = 3;

/// Penghitung audit.
#[derive(Clone, Copy)]
#[repr(u32)]
pub enum Counter {
    /// `klatos_tempat_menang` dipanggil dari produksi.
    ProdMenang = 0,
    /// `klatos_tempat_spesifik` dipanggil dari produksi.
    ProdSpesifik = 1,
    /// `klatos_tempat_acak` dipanggil dari produksi.
    ProdAcak = 2,
    /// `klatos_tempat_potong` dipanggil dari produksi.
    ProdPotong = 3,
    /// Keputusan yang MENOLAK (tidak ada muat / bertumpang / di luar).
    ProdTolak = 4,
    /// Argumen tak sah (perataan nol / ukuran nol / meluap).
    ProdArgSalah = 5,
}

const N_COUNTER: usize = 6;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

/// Tag laporan; cermin `KlatosTempatReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_544D_5054; // "KLATTMPT"

/// Cermin `KlatosTempatReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosTempatReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_menang: u32,
    pub prod_spesifik: u32,
    pub prod_acak: u32,
    pub prod_potong: u32,
    pub prod_tolak: u32,
    pub prod_arg_salah: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Hasil satu keputusan penempatan. `status == 0` berarti `base`/`ukuran` sah.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosTempatHasil {
    pub status: i32,
    pub ukuran: u32,
    pub base: u64,
}

/// Hasil satu keputusan pemotongan wilayah (`carve`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosTempatPotong {
    pub status: i32,
    pub jumlah: u32,
    pub kiri_lo: u64,
    pub kiri_hi: u64,
    pub kanan_lo: u64,
    pub kanan_hi: u64,
}

// ---------------------------------------------------------------------------
// Inti — kebijakan
// ---------------------------------------------------------------------------

/// `round_up_to_power_of_two` C++ (`StdLibExtras.h:28`), PERSIS termasuk
/// perilaku membungkus untuk `v == 0`.
#[inline]
pub const fn bulatkan_ke_atas(v: u64, p: u64) -> u64 {
    (v.wrapping_sub(1) & !(p.wrapping_sub(1))).wrapping_add(p)
}

/// Apakah `p` pangkat dua bukan nol.
#[inline]
pub const fn pangkat_dua(p: u64) -> bool {
    p != 0 && (p & (p.wrapping_sub(1))) == 0
}

/// `RegionTree::allocate_from_window` — apakah `[win_base, win_base+win_ukuran)`
/// dapat memuat `ukuran` dengan perataan `perataan`, dan bila ya di mana.
#[inline]
pub fn tempat_menang(win_base: u64, win_ukuran: u64, ukuran: u64, perataan: u64) -> KlatosTempatHasil {
    let gagal = KlatosTempatHasil { status: 1, ukuran: 0, base: TAK_SAH };
    if ukuran == 0 {
        return KlatosTempatHasil { status: TS_ARG, ukuran: 0, base: TAK_SAH };
    }
    if !pangkat_dua(perataan) {
        return KlatosTempatHasil { status: TS_ARG, ukuran: 0, base: TAK_SAH };
    }
    // C++: `Checked<size_t>::addition_would_overflow(size, alignment)`.
    let butuh = match ukuran.checked_add(perataan) {
        Some(v) => v,
        None => return KlatosTempatHasil { status: 2, ukuran: 0, base: TAK_SAH },
    };
    if win_ukuran < butuh {
        return gagal;
    }
    let base = bulatkan_ke_atas(win_base, perataan);
    KlatosTempatHasil { status: 0, ukuran: ukuran as u32, base }
}

/// `RegionTree::allocate_range_specific` — apakah `[base, base+ukuran)` dapat
/// ditempatkan di `[total_lo, total_hi)` mengingat satu tetangga
/// `[tetangga_lo, tetangga_hi)` (hasil `find_largest_not_above(base+ukuran-1)`).
///
/// `ada_tetangga == 0` berarti pohon mengembalikan `nullptr`.
#[inline]
pub fn tempat_spesifik(
    total_lo: u64,
    total_hi: u64,
    base: u64,
    ukuran: u64,
    tetangga_lo: u64,
    tetangga_hi: u64,
    ada_tetangga: u32,
) -> i32 {
    if ukuran == 0 {
        return TS_ARG;
    }
    // C++: `m_total_range.contains(range)` -> `berisi_rentang` (cek luap dulu).
    let akhir = match base.checked_add(ukuran) {
        Some(v) => v,
        None => return TS_LUAR,
    };
    if base < total_lo || akhir > total_hi {
        return TS_LUAR;
    }
    if ada_tetangga != 0 {
        // C++: `region->range().intersects(range)` — ketaksamaan TEGAS.
        if tetangga_lo < akhir && base < tetangga_hi {
            return TS_TUMPANG;
        }
    }
    TS_OK
}

/// `RegionTree::allocate_range_randomized` — satu cobaan kandidat acak.
///
/// C++: `round_up_to_power_of_two(get_fast_random<FlatPtr>() % total_end, alignment)`
/// lalu `m_total_range.contains(random_address, size)`.
#[inline]
pub fn tempat_acak(
    acak: u64,
    total_lo: u64,
    total_hi: u64,
    ukuran: u64,
    perataan: u64,
) -> KlatosTempatHasil {
    let gagal = KlatosTempatHasil { status: 1, ukuran: 0, base: TAK_SAH };
    if ukuran == 0 {
        return KlatosTempatHasil { status: TS_ARG, ukuran: 0, base: TAK_SAH };
    }
    if !pangkat_dua(perataan) {
        return KlatosTempatHasil { status: TS_ARG, ukuran: 0, base: TAK_SAH };
    }
    // Modulus memakai UJUNG rentang, persis seperti C++.
    if total_hi == 0 {
        return gagal;
    }
    let kandidat = bulatkan_ke_atas(acak % total_hi, perataan);
    // `berisi_rentang`: cek luap lebih dulu, lalu batas inklusif.
    let akhir = match kandidat.checked_add(ukuran) {
        Some(v) => v,
        None => return gagal,
    };
    if kandidat < total_lo || akhir > total_hi {
        return gagal;
    }
    KlatosTempatHasil { status: 0, ukuran: ukuran as u32, base: kandidat }
}

/// `VirtualRange::carve` — potongan apa yang TERSISA dari `[reg_lo, reg_hi)`
/// setelah `[potong_lo, potong_hi)` diambil.
#[inline]
pub fn tempat_potong(reg_lo: u64, reg_hi: u64, potong_lo: u64, potong_hi: u64) -> KlatosTempatPotong {
    let kosong = KlatosTempatPotong {
        status: 0,
        jumlah: 0,
        kiri_lo: 0,
        kiri_hi: 0,
        kanan_lo: 0,
        kanan_hi: 0,
    };
    if potong_lo == reg_lo && potong_hi == reg_hi {
        return kosong;
    }
    let mut hasil = kosong;
    if potong_lo > reg_lo {
        hasil.kiri_lo = reg_lo;
        hasil.kiri_hi = potong_lo;
        hasil.jumlah += 1;
    }
    if potong_hi < reg_hi {
        hasil.kanan_lo = potong_hi;
        hasil.kanan_hi = reg_hi;
        hasil.jumlah += 1;
    }
    hasil
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// `RegionTree::allocate_from_window`.
#[no_mangle]
pub extern "C" fn klatos_tempat_menang(
    win_base: u64,
    win_ukuran: u64,
    ukuran: u64,
    perataan: u64,
) -> KlatosTempatHasil {
    bump(Counter::ProdMenang);
    let h = tempat_menang(win_base, win_ukuran, ukuran, perataan);
    match h.status {
        0 => {}
        TS_ARG => bump(Counter::ProdArgSalah),
        _ => bump(Counter::ProdTolak),
    }
    h
}

/// `RegionTree::allocate_range_specific`.
#[no_mangle]
pub extern "C" fn klatos_tempat_spesifik(
    total_lo: u64,
    total_hi: u64,
    base: u64,
    ukuran: u64,
    tetangga_lo: u64,
    tetangga_hi: u64,
    ada_tetangga: u32,
) -> i32 {
    bump(Counter::ProdSpesifik);
    let s = tempat_spesifik(total_lo, total_hi, base, ukuran, tetangga_lo, tetangga_hi, ada_tetangga);
    match s {
        TS_OK => {}
        TS_ARG => bump(Counter::ProdArgSalah),
        _ => bump(Counter::ProdTolak),
    }
    s
}

/// `RegionTree::allocate_range_randomized` — satu cobaan.
#[no_mangle]
pub extern "C" fn klatos_tempat_acak(
    acak: u64,
    total_lo: u64,
    total_hi: u64,
    ukuran: u64,
    perataan: u64,
) -> KlatosTempatHasil {
    bump(Counter::ProdAcak);
    let h = tempat_acak(acak, total_lo, total_hi, ukuran, perataan);
    match h.status {
        0 => {}
        TS_ARG => bump(Counter::ProdArgSalah),
        _ => bump(Counter::ProdTolak),
    }
    h
}

/// `VirtualRange::carve`.
#[no_mangle]
pub extern "C" fn klatos_tempat_potong(
    reg_lo: u64,
    reg_hi: u64,
    potong_lo: u64,
    potong_hi: u64,
) -> KlatosTempatPotong {
    bump(Counter::ProdPotong);
    tempat_potong(reg_lo, reg_hi, potong_lo, potong_hi)
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_tempat_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0.
#[no_mangle]
pub extern "C" fn klatos_tempat_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Matriks uji-sendiri
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct Naif;

impl Naif {
    /// Transkripsi independen rumus C++ (pembanding matriks).
    fn menang(win_base: u64, win_ukuran: u64, ukuran: u64, perataan: u64) -> KlatosTempatHasil {
        let gagal = KlatosTempatHasil { status: 1, ukuran: 0, base: TAK_SAH };
        if ukuran == 0 {
            return KlatosTempatHasil { status: TS_ARG, ukuran: 0, base: TAK_SAH };
        }
        if perataan == 0 || (perataan & (perataan - 1)) != 0 {
            return KlatosTempatHasil { status: TS_ARG, ukuran: 0, base: TAK_SAH };
        }
        let butuh = match ukuran.checked_add(perataan) {
            Some(v) => v,
            None => return KlatosTempatHasil { status: 2, ukuran: 0, base: TAK_SAH },
        };
        if win_ukuran < butuh {
            return gagal;
        }
        let b = ((win_base.wrapping_sub(1)) & !(perataan.wrapping_sub(1))).wrapping_add(perataan);
        KlatosTempatHasil { status: 0, ukuran: ukuran as u32, base: b }
    }

    fn spesifik(
        total_lo: u64,
        total_hi: u64,
        base: u64,
        ukuran: u64,
        t_lo: u64,
        t_hi: u64,
        ada: u32,
    ) -> i32 {
        if ukuran == 0 {
            return TS_ARG;
        }
        let akhir = base.wrapping_add(ukuran);
        if akhir < base || base < total_lo || akhir > total_hi {
            return TS_LUAR;
        }
        if ada != 0 && t_lo < akhir && base < t_hi {
            return TS_TUMPANG;
        }
        TS_OK
    }

    fn acak(acak: u64, total_lo: u64, total_hi: u64, ukuran: u64, perataan: u64) -> KlatosTempatHasil {
        let gagal = KlatosTempatHasil { status: 1, ukuran: 0, base: TAK_SAH };
        if ukuran == 0 {
            return KlatosTempatHasil { status: TS_ARG, ukuran: 0, base: TAK_SAH };
        }
        if perataan == 0 || (perataan & (perataan - 1)) != 0 {
            return KlatosTempatHasil { status: TS_ARG, ukuran: 0, base: TAK_SAH };
        }
        if total_hi == 0 {
            return gagal;
        }
        let k = ((acak % total_hi) + perataan - 1) & !(perataan - 1);
        let akhir = k.wrapping_add(ukuran);
        if akhir < k || k < total_lo || akhir > total_hi {
            return gagal;
        }
        KlatosTempatHasil { status: 0, ukuran: ukuran as u32, base: k }
    }

    fn potong(reg_lo: u64, reg_hi: u64, p_lo: u64, p_hi: u64) -> KlatosTempatPotong {
        let mut r = KlatosTempatPotong { status: 0, jumlah: 0, kiri_lo: 0, kiri_hi: 0, kanan_lo: 0, kanan_hi: 0 };
        if p_lo == reg_lo && p_hi == reg_hi {
            return r;
        }
        if p_lo > reg_lo {
            r.kiri_lo = reg_lo;
            r.kiri_hi = p_lo;
            r.jumlah += 1;
        }
        if p_hi < reg_hi {
            r.kanan_lo = p_hi;
            r.kanan_hi = reg_hi;
            r.jumlah += 1;
        }
        r
    }
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
}

fn jalankan_matriks() -> KlatosTempatReport {
    let mut checks: u32 = 0;
    let mut passed: u32 = 0;
    let mut neg_checks: u32 = 0;
    let mut neg_passed: u32 = 0;
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    macro_rules! cek {
        ($nama:expr, $kond:expr) => {{
            checks += 1;
            if $kond {
                passed += 1;
            }
        }};
    }

    // --- G1: konstanta -----------------------------------------------------
    cek!("tag", REPORT_TAG == 0x4B4C_4154_544D_5054);
    cek!("tak_sah", TAK_SAH == u64::MAX);
    cek!("halaman", HALAMAN == 4096);
    cek!("status ok", TS_OK == 0 && TS_LUAR == 1 && TS_TUMPANG == 2 && TS_ARG == 3);
    cek!("pangkat dua", pangkat_dua(4096) && pangkat_dua(1) && !pangkat_dua(0) && !pangkat_dua(12288));
    cek!("bulatkan 0", bulatkan_ke_atas(0, 4096) == 0);
    cek!("bulatkan pas", bulatkan_ke_atas(4096, 4096) == 4096);
    cek!("bulatkan naik", bulatkan_ke_atas(4097, 4096) == 8192);

    // --- G2: menang --------------------------------------------------------
    {
        // jendela muat: 0x1000..0x9000, ukuran 0x1000, perataan 0x1000.
        let h = tempat_menang(0x1000, 0x8000, 0x1000, 0x1000);
        cek!("menang muat status", h.status == 0);
        cek!("menang muat base", h.base == 0x1000);
        cek!("menang ukuran", h.ukuran == 0x1000);
        // jendela tidak muat.
        let h2 = tempat_menang(0x1000, 0x1000, 0x1000, 0x1000);
        cek!("menang tak muat", h2.status == 1 && h2.base == TAK_SAH);
        // jendela tak selaras -> dibulatkan naik.
        let h3 = tempat_menang(0x1001, 0x8000, 0x1000, 0x1000);
        cek!("menang selaras", h3.base == 0x2000);
        // ukuran nol -> EINVAL.
        let h4 = tempat_menang(0x1000, 0x8000, 0, 0x1000);
        cek!("menang ukuran 0", h4.status == TS_ARG);
        // perataan nol -> EINVAL.
        let h5 = tempat_menang(0x1000, 0x8000, 0x1000, 0);
        cek!("menang perataan 0", h5.status == TS_ARG);
        // luapan ukuran+perataan.
        let h6 = tempat_menang(0, u64::MAX, u64::MAX, 0x1000);
        cek!("menang luapan", h6.status == 2);
    }

    // --- G3: spesifik ------------------------------------------------------
    {
        // diterima, tanpa tetangga.
        cek!("spesifik ok tanpa tetangga", tempat_spesifik(0x1000, 0x100000, 0x2000, 0x1000, 0, 0, 0) == TS_OK);
        // di luar rentang total.
        cek!("spesifik luar", tempat_spesifik(0x1000, 0x10000, 0x20000, 0x1000, 0, 0, 0) == TS_LUAR);
        // melewati ujung.
        cek!("spesifik lewat ujung", tempat_spesifik(0x1000, 0x10000, 0xF000, 0x2000, 0, 0, 0) == TS_LUAR);
        // luapan.
        cek!("spesifik luapan", tempat_spesifik(0x1000, 0x10000, u64::MAX - 8, 0x1000, 0, 0, 0) == TS_LUAR);
        // bertumpang dengan tetangga.
        cek!("spesifik tumpang", tempat_spesifik(0x1000, 0x100000, 0x2000, 0x1000, 0x2800, 0x3000, 1) == TS_TUMPANG);
        // bersebelahan TIDAK bertumpang (ketaksamaan tegas).
        cek!("spesifik bersebelahan", tempat_spesifik(0x1000, 0x100000, 0x2000, 0x1000, 0x3000, 0x4000, 1) == TS_OK);
        // tetangga di bawah, tidak bertumpang.
        cek!("spesifik tetangga bawah", tempat_spesifik(0x1000, 0x100000, 0x2000, 0x1000, 0x1000, 0x2000, 1) == TS_OK);
        // ukuran nol.
        cek!("spesifik ukuran 0", tempat_spesifik(0x1000, 0x10000, 0x2000, 0, 0, 0, 0) == TS_ARG);
    }

    // --- G4: acak ----------------------------------------------------------
    {
        // total [0x1000, 0x100000), ukuran 0x1000, perataan 0x1000.
        // acak % total_hi lalu dibulatkan; 0x1500 % 0x100000 = 0x1500 -> 0x2000.
        let h = tempat_acak(0x1500, 0x1000, 0x100000, 0x1000, 0x1000);
        cek!("acak muat", h.status == 0 && h.base == 0x2000);
        // Kandidat SETELAH pembulatan tetap di bawah total_lo -> ditolak.
        // (Perhatikan: pembulatan ke atas dapat MENGANGKAT kandidat kecil ke
        // dalam rentang; karena itu total_lo harus di atas hasil pembulatan.)
        let h2 = tempat_acak(0x100, 0x8000, 0x100000, 0x1000, 0x1000);
        cek!("acak bawah", h2.status == 1 && h2.base == TAK_SAH);
        // kandidat hampir di ujung -> melewati ujung -> ditolak.
        let h3 = tempat_acak(0xFFFF8, 0x1000, 0x100000, 0x1000, 0x1000);
        cek!("acak lewat ujung", h3.status == 1);
        // perataan nol -> EINVAL.
        let h4 = tempat_acak(0x1500, 0x1000, 0x100000, 0x1000, 0);
        cek!("acak perataan 0", h4.status == TS_ARG);
        // total_hi nol -> ditolak.
        let h5 = tempat_acak(0x1500, 0, 0, 0x1000, 0x1000);
        cek!("acak total 0", h5.status == 1);
    }

    // --- G5: potong --------------------------------------------------------
    {
        let p = tempat_potong(0x1000, 0x9000, 0x3000, 0x5000);
        cek!("potong dua", p.jumlah == 2);
        cek!("potong kiri", p.kiri_lo == 0x1000 && p.kiri_hi == 0x3000);
        cek!("potong kanan", p.kanan_lo == 0x5000 && p.kanan_hi == 0x9000);
        let p2 = tempat_potong(0x1000, 0x9000, 0x1000, 0x5000);
        cek!("potong kanan saja", p2.jumlah == 1 && p2.kanan_lo == 0x5000 && p2.kiri_hi == 0);
        let p3 = tempat_potong(0x1000, 0x9000, 0x5000, 0x9000);
        cek!("potong kiri saja", p3.jumlah == 1 && p3.kiri_lo == 0x1000 && p3.kanan_hi == 0);
        let p4 = tempat_potong(0x1000, 0x9000, 0x1000, 0x9000);
        cek!("potong penuh", p4.jumlah == 0);
        let p5 = tempat_potong(0x1000, 0x9000, 0x2000, 0x8000);
        cek!("potong dalam", p5.jumlah == 2 && p5.kiri_hi == 0x2000 && p5.kanan_lo == 0x8000);
    }

    // --- G6: diferensial vs Naif (LCG) -------------------------------------
    {
        let mut lcg = Lcg(0x9E37_79B9_7F4A_7C15);
        let mut i = 0u32;
        while i < 20000 {
            let a = lcg.next();
            let b = lcg.next();
            let c = lcg.next();

            // menang
            let wb = a & 0x0000_FFFF_FFFF_F000;
            let wu = (b & 0xFFFF_F000) + 0x1000;
            let uk = ((c & 0xFFFF) + 1) * 0x1000;
            let pa = 1u64 << (12 + (c % 8));
            let r = tempat_menang(wb, wu, uk, pa);
            let n = Naif::menang(wb, wu, uk, pa);
            diukur += 1;
            if r.status != n.status || r.base != n.base {
                beda += 1;
            }

            // spesifik
            let t_lo = a & 0xFFFF_FFFF_0000;
            let t_hi = t_lo + (b & 0xFFFF_FF00) + 0x1000;
            let sbase = (c & 0xFFFF_FFFF_0000) + (b & 0xFFF);
            let suk = ((a & 0xFF) + 1) * 0x1000;
            let ada = ((b >> 3) & 1) as u32;
            let nb_lo = sbase + (c & 0xFFFF_0000);
            let nb_hi = nb_lo + (a & 0xFFFF) + 0x1000;
            let rs = tempat_spesifik(t_lo, t_hi, sbase, suk, nb_lo, nb_hi, ada);
            let ns = Naif::spesifik(t_lo, t_hi, sbase, suk, nb_lo, nb_hi, ada);
            diukur += 1;
            if rs != ns {
                beda += 1;
            }

            // acak
            let ac = lcg.next();
            let ra = tempat_acak(ac, t_lo, t_hi, suk, pa);
            let na = Naif::acak(ac, t_lo, t_hi, suk, pa);
            diukur += 1;
            if ra.status != na.status || ra.base != na.base {
                beda += 1;
            }

            // potong
            let r_lo = a & 0x0000_FFFF_FFFF_0000;
            let r_hi = r_lo + (b & 0x00FF_FFFF_0000) + 0x1000;
            let p_lo = r_lo + (c & 0x0000_000F_FFFF_0000);
            let p_hi = p_lo + (a & 0x0000_0000_FFFF_0000) + 0x1000;
            let rp = tempat_potong(r_lo, r_hi, p_lo, p_hi);
            let np = Naif::potong(r_lo, r_hi, p_lo, p_hi);
            diukur += 1;
            if rp.jumlah != np.jumlah
                || rp.kiri_lo != np.kiri_lo
                || rp.kiri_hi != np.kiri_hi
                || rp.kanan_lo != np.kanan_lo
                || rp.kanan_hi != np.kanan_hi
            {
                beda += 1;
            }

            i += 1;
        }
    }

    // --- G7: determinisme --------------------------------------------------
    {
        let a1 = tempat_menang(0x1001, 0x8000, 0x1000, 0x1000);
        let a2 = tempat_menang(0x1001, 0x8000, 0x1000, 0x1000);
        cek!("determinisme menang", a1.base == a2.base && a1.status == a2.status);
        let b1 = tempat_spesifik(0x1000, 0x100000, 0x2000, 0x1000, 0x2800, 0x3000, 1);
        let b2 = tempat_spesifik(0x1000, 0x100000, 0x2000, 0x1000, 0x2800, 0x3000, 1);
        cek!("determinisme spesifik", b1 == b2);
        let c1 = tempat_potong(0x1000, 0x9000, 0x3000, 0x5000);
        let c2 = tempat_potong(0x1000, 0x9000, 0x3000, 0x5000);
        cek!("determinisme potong", c1.jumlah == c2.jumlah && c1.kiri_hi == c2.kiri_hi);
    }

    // --- G8: negatif -------------------------------------------------------
    {
        // Jendela kurang satu halaman dari kebutuhan -> harus gagal.
        let h = tempat_menang(0, 0x1000, 0x1000, 0x1000);
        neg_checks += 1;
        if h.status == 1 {
            neg_passed += 1;
        }
        // Jendela PERSIS sebesar ukuran+perataan -> harus berhasil.
        let h2 = tempat_menang(0, 0x2000, 0x1000, 0x1000);
        neg_checks += 1;
        if h2.status == 0 && h2.base == 0 {
            neg_passed += 1;
        }
        // Spesifik dengan tetangga persis bersebelahan kiri -> OK (TEGAS).
        neg_checks += 1;
        if tempat_spesifik(0x1000, 0x100000, 0x5000, 0x1000, 0x1000, 0x5000, 1) == TS_OK {
            neg_passed += 1;
        }
        // Potong dengan rentang di ATAS wilayah: `carve` C++ historis TETAP
        // mengembalikan potongan kiri `[reg_lo, potong_lo)` — yaitu melewati
        // ujung wilayah. Kasus ini TIDAK pernah muncul di produksi (pemanggil
        // selalu memotong rentang yang bertumpang), tetapi direplikasi apa
        // adanya; oracle host mengukur perilaku yang sama.
        let p = tempat_potong(0x1000, 0x2000, 0x9000, 0xA000);
        neg_checks += 1;
        if p.jumlah == 1 && p.kiri_lo == 0x1000 && p.kiri_hi == 0x9000 && p.kanan_hi == 0 {
            neg_passed += 1;
        }
        // acak tepat di batas bawah.
        neg_checks += 1;
        if tempat_acak(0x1000, 0x1000, 0x100000, 0x1000, 0x1000).status == 0 {
            neg_passed += 1;
        }
        // perataan bukan pangkat dua -> EINVAL.
        neg_checks += 1;
        if tempat_menang(0, 0x8000, 0x1000, 0x3000).status == TS_ARG {
            neg_passed += 1;
        }
    }

    let status = if passed == checks && beda == 0 && neg_passed == neg_checks {
        0
    } else {
        -1
    };
    KlatosTempatReport {
        tag: REPORT_TAG,
        checks,
        passed,
        peta_checks: diukur,
        peta_passed: diukur - beda,
        neg_checks,
        neg_passed,
        prod_menang: COUNTERS[Counter::ProdMenang as usize].load(Ordering::Relaxed),
        prod_spesifik: COUNTERS[Counter::ProdSpesifik as usize].load(Ordering::Relaxed),
        prod_acak: COUNTERS[Counter::ProdAcak as usize].load(Ordering::Relaxed),
        prod_potong: COUNTERS[Counter::ProdPotong as usize].load(Ordering::Relaxed),
        prod_tolak: COUNTERS[Counter::ProdTolak as usize].load(Ordering::Relaxed),
        prod_arg_salah: COUNTERS[Counter::ProdArgSalah as usize].load(Ordering::Relaxed),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Uji-sendiri; TIDAK menyentuh penghitung produksi (dipakai C++ dan host).
#[no_mangle]
pub extern "C" fn klatos_tempat_selftest() -> KlatosTempatReport {
    jalankan_matriks()
}

// ---------------------------------------------------------------------------
// Konformansi tata letak
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;
    use core::mem::{align_of, size_of};

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_544D_5054);
        assert!(TAK_SAH == u64::MAX);
        assert!(HALAMAN == 4096);
        assert!(TS_OK == 0 && TS_LUAR == 1 && TS_TUMPANG == 2 && TS_ARG == 3);
        assert!(Counter::ProdMenang as usize == 0);
        assert!(Counter::ProdArgSalah as usize == 5);
        assert!(N_COUNTER == 6);
        // Tata letak cermin C++ (`KlatKernelBridge.h`).
        assert!(size_of::<KlatosTempatHasil>() == 16);
        assert!(align_of::<KlatosTempatHasil>() == 8);
        assert!(size_of::<KlatosTempatPotong>() == 40);
        assert!(align_of::<KlatosTempatPotong>() == 8);
        // tag(8) + 15×4 = 68, dibulatkan ke kelipatan 8 -> 72.
        assert!(size_of::<KlatosTempatReport>() == 72);
        assert!(align_of::<KlatosTempatReport>() == 8);
    };
}
