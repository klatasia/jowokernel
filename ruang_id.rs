// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.34 — INFRASTRUKTUR RUANG-ID RUST, dipakai bersama oleh P1.34–P1.36.


use core::sync::atomic::{AtomicU64, Ordering};

/// Gaya transisi historis C++.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gaya {
    /// `++s` — naikkan dulu, kembalikan nilai baru. Id pertama = dasar + 1.
    PraNaik = 0,
    /// `s++` — kembalikan nilai lama, baru naikkan. Id pertama = dasar.
    PascaNaik = 1,
}

/// Hasil satu pengambilan id.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HasilId {
    /// Id yang sah.
    Id(u64),
    /// Ruang id habis; id TIDAK diberikan (C++ historis membungkus diam-diam).
    Habis,
}

// ---------------------------------------------------------------------------
// Oracle historis — dipakai diferensial
// ---------------------------------------------------------------------------

/// Nilai yang DIKEMBALIKAN oleh transisi C++ historis.
///
/// Ini terjemahan harfiah dari dua bentuk di atas; ia tidak memakai tipe Rust
/// apa pun dari mekanisme produksi, sehingga diferensial benar-benar
/// membandingkan mekanisme dengan riwayat, bukan dengan dirinya sendiri.
pub const fn kebijakan_lama(nilai: u64, gaya: Gaya) -> u64 {
    match gaya {
        Gaya::PraNaik => nilai.wrapping_add(1),
        Gaya::PascaNaik => nilai,
    }
}

/// Nilai penghitung C++ SETELAH satu transisi (selalu +1 pada kedua gaya).
pub const fn nilai_setelah(nilai: u64) -> u64 {
    nilai.wrapping_add(1)
}

// ---------------------------------------------------------------------------
// Algoritma TUNGGAL
// ---------------------------------------------------------------------------

/// Penyimpanan satu ruang id. Abstraksi ini ada supaya matriks memakai
/// algoritma produksi yang **sama** ([`ambil_in`]), bukan salinannya.
pub trait PenyimpananId {
    fn muat(&self) -> u64;
    /// `true` bila nilai masih `cur` dan berhasil diganti `baru`.
    fn cas(&self, cur: u64, baru: u64) -> bool;
    fn simpan(&self, nilai: u64);
}

/// Algoritma produksi: ambil satu id dari ruang dengan batas [`maks`].
///
/// Deterministik dan tanpa kunci. Bila penghitung sudah di ujung, kembalikan
/// [`HasilId::Habis`] **tanpa** menggeser penghitung, sehingga panggilan
/// berikutnya juga menolak (id yang membungkus tidak pernah diberikan).
///
/// `checked_add` — bukan `wrapping_add`: kalau penghitung membungkus, id `0`
/// akan diberikan LAGI dan bertabrakan dengan id pertama ruang ini. Itu tepat
/// kerusakan yang modul ini ada untuk mencegah, jadi di ujung atas ruang kita
/// MENOLAK, bukan membungkus.
///
/// Satu kasus yang tidak dapat diwakili: ruang penuh `u64` (`maks == u64::MAX`)
/// pada [`Gaya::PascaNaik`] tidak punya keadaan "habis" karena `maks + 1`
/// meluap. Di sana id terakhir diberikan dan penghitung **jenuh** (tidak
/// membungkus ke 0). Tidak dapat dicapai dalam praktik (1 GHz → ~584 tahun),
/// dan tidak mengubah hasil di dalam domain yang terdefinisi.
pub fn ambil_in<S: PenyimpananId>(s: &S, maks: u64, gaya: Gaya) -> HasilId {
    loop {
        let cur = s.muat();
        let (hasil, berikut) = match gaya {
            Gaya::PraNaik => {
                let n = match cur.checked_add(1) {
                    Some(n) => n,
                    None => return HasilId::Habis,
                };
                if n > maks {
                    return HasilId::Habis;
                }
                (n, n)
            }
            Gaya::PascaNaik => {
                if cur > maks {
                    return HasilId::Habis;
                }
                match cur.checked_add(1) {
                    Some(n) => (cur, n),
                    None => (cur, cur),
                }
            }
        };
        if s.cas(cur, berikut) {
            return HasilId::Id(hasil);
        }
    }
}

// ---------------------------------------------------------------------------
// Penyimpanan PRODUKSI (statik, atomik, tanpa kunci)
// ---------------------------------------------------------------------------

/// Satu ruang id produksi.
pub struct RuangId {
    nilai: AtomicU64,
    maks: u64,
    gaya: Gaya,
}

impl RuangId {
    /// `dasar` = nilai awal penghitung C++ (mis. `static u32 s_x;` → 0).
    pub const fn baru(dasar: u64, maks: u64, gaya: Gaya) -> Self {
        RuangId {
            nilai: AtomicU64::new(dasar),
            maks,
            gaya,
        }
    }

    /// Mengambil id berikutnya dari ruang ini.
    pub fn ambil(&self) -> HasilId {
        ambil_in(self, self.maks, self.gaya)
    }

    /// Nilai penghitung saat ini (bukan id terakhir yang diberikan).
    pub fn intip(&self) -> u64 {
        self.nilai.load(Ordering::Acquire)
    }

    /// Menyetel penghitung (hanya audit/matriks; jalur produksi tidak
    /// memanggilnya).
    pub fn setel(&self, v: u64) {
        self.nilai.store(v, Ordering::Release);
    }

    pub fn maks(&self) -> u64 {
        self.maks
    }

    pub const fn gaya(&self) -> Gaya {
        self.gaya
    }
}

impl PenyimpananId for RuangId {
    fn muat(&self) -> u64 {
        self.nilai.load(Ordering::Acquire)
    }
    fn cas(&self, cur: u64, baru: u64) -> bool {
        self.nilai
            .compare_exchange(cur, baru, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    fn simpan(&self, nilai: u64) {
        self.nilai.store(nilai, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// Penyimpanan SINTETIS (stack) — hanya untuk matriks
// ---------------------------------------------------------------------------

/// Ruang id sintetis. Matriks memakainya supaya ruang id PRODUKSI tidak
/// pernah tersentuh (lihat kontrol `prod_*` di setiap matriks gate).
pub struct PenyimpananUji {
    nilai: core::cell::Cell<u64>,
}

impl PenyimpananUji {
    pub const fn baru(nilai: u64) -> Self {
        PenyimpananUji {
            nilai: core::cell::Cell::new(nilai),
        }
    }
    pub fn intip(&self) -> u64 {
        self.nilai.get()
    }
    pub fn setel(&self, v: u64) {
        self.nilai.set(v);
    }
}

impl PenyimpananId for PenyimpananUji {
    fn muat(&self) -> u64 {
        self.nilai.get()
    }
    fn cas(&self, cur: u64, baru: u64) -> bool {
        if self.nilai.get() == cur {
            self.nilai.set(baru);
            true
        } else {
            false
        }
    }
    fn simpan(&self, nilai: u64) {
        self.nilai.set(nilai);
    }
}

// ---------------------------------------------------------------------------
// Diferensial bersama
// ---------------------------------------------------------------------------

/// Menjalankan diferensial mekanisme Rust terhadap oracle C++ historis untuk
/// satu gaya, atas seed `0..rentang`.
///
/// Mengembalikan `(diukur, beda)`. `beda` harus 0 di dalam domain yang
/// terdefinisi (`rentang <= maks`).
pub fn diferensial_gaya(gaya: Gaya, maks: u64, rentang: u64) -> (u32, u32) {
    let mut diukur = 0u32;
    let mut beda = 0u32;
    let mut seed = 0u64;
    while seed < rentang {
        let s = PenyimpananUji::baru(seed);
        let r = ambil_in(&s, maks, gaya);
        diukur += 1;
        let oracle = kebijakan_lama(seed, gaya);
        let nilai_ok = s.intip() == nilai_setelah(seed);
        match r {
            HasilId::Id(v) => {
                if v != oracle || !nilai_ok {
                    beda += 1;
                }
            }
            HasilId::Habis => {
                beda += 1;
            }
        }
        seed += 1;
    }
    (diukur, beda)
}

// ---------------------------------------------------------------------------
// Matriks + laporan bersama
// ---------------------------------------------------------------------------

/// Kelompok pemeriksaan, sama untuk ketiga gate ruang-id.
#[derive(Clone, Copy)]
pub enum Grup {
    /// Alokasi/urutan normal.
    Umum = 0,
    /// Diferensial terhadap oracle historis.
    Peta = 1,
    /// Kontrol negatif (batas, habis, ruang produksi tidak tersentuh).
    Negatif = 2,
}

/// Akumulator matriks.
pub struct Matrix {
    pub checks: u32,
    pub passed: u32,
    pub grup_checks: [u32; 3],
    pub grup_passed: [u32; 3],
}

impl Matrix {
    pub const fn baru() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            grup_checks: [0; 3],
            grup_passed: [0; 3],
        }
    }
    pub fn catat(&mut self, lulus: bool, grup: Grup) {
        let g = grup as usize;
        self.checks += 1;
        self.grup_checks[g] += 1;
        if lulus {
            self.passed += 1;
            self.grup_passed[g] += 1;
        }
    }
}

/// Cermin `KlatosRuangIdReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
///
/// Tata letaknya dijaga `static_assert` di sisi C++.
#[repr(C)]
pub struct KlatosRuangIdReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_a: u32,
    pub prod_b: u32,
    pub prod_c: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Membangun laporan dari matriks + diferensial. `prod_a..prod_c` adalah
/// penghitung produksi domain (mis. fs/root/fifo).
pub fn buat_laporan(
    tag: u64,
    m: &Matrix,
    diferensial_diukur: u32,
    diferensial_beda: u32,
    prod_a: u32,
    prod_b: u32,
    prod_c: u32,
) -> KlatosRuangIdReport {
    let mut report = KlatosRuangIdReport {
        tag,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.grup_checks[1],
        peta_passed: m.grup_passed[1],
        neg_checks: m.grup_checks[2],
        neg_passed: m.grup_passed[2],
        prod_a,
        prod_b,
        prod_c,
        diferensial_diukur,
        diferensial_beda,
        status: 0,
    };
    if report.checks != report.passed {
        report.status = -1;
    }
    report
}

/// Konformansi waktu-kompilasi infrastruktur.
pub mod conformance {
    use super::*;

    const _: () = {
        // Ukuran laporan: 8 + 4*12 = 56 byte. Sama dengan cermin C++.
        assert!(core::mem::size_of::<KlatosRuangIdReport>() == 56);

        // Urutan gaya stabil (dipakai C++ lewat `klatos_ruang_id_*`).
        assert!(Gaya::PraNaik as usize == 0);
        assert!(Gaya::PascaNaik as usize == 1);

        // Oracle: PraNaik mengembalikan nilai+1, PascaNaik nilai lama.
        assert!(kebijakan_lama(0, Gaya::PraNaik) == 1);
        assert!(kebijakan_lama(7, Gaya::PraNaik) == 8);
        assert!(kebijakan_lama(0, Gaya::PascaNaik) == 0);
        assert!(kebijakan_lama(7, Gaya::PascaNaik) == 7);
        assert!(nilai_setelah(7) == 8);

        // Batas: tepat di `maks` PraNaik sudah menolak, PascaNaik masih menerima.
        // (Diuji di matriks tiap gate; di sini hanya invarian konstanta.)
        assert!(nilai_setelah(u64::MAX) == 0); // wrapping, seperti C++
    };
}
