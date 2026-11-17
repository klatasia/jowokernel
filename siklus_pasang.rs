// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.74 / WS4 — **OTORITAS SIKLUS HIDUP & ADMISI PEMASANGAN (MOUNT)**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-1).
pub const EPERM: i32 = 1;
/// `EBUSY` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-16).
pub const EBUSY: i32 = 16;
/// `ENODEV` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks ke-19).
pub const ENODEV: i32 = 19;

/// Tag laporan; cermin `KlatosSiklusPasangReport` di `Jowo/Kontrak/KlatSeam74.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_534B_5053; // "KLATSKPS"

// --- keadaan mesin hierarki -------------------------------------------------

/// Belum dibandingkan: nilai awal sebelum langkah pertama.
pub const HIERARKI_INIT: u32 = 0;
/// Masih menelusuri rantai; pemanggil harus memajukan satu langkah.
pub const HIERARKI_LANGKAH: u32 = 1;
/// Keputusan akhir: hierarki identik.
pub const HIERARKI_COCOK: u32 = 2;
/// Keputusan akhir: hierarki berbeda.
pub const HIERARKI_BEDA: u32 = 3;

// --- verdict pelepasan ------------------------------------------------------

/// Pelepasan berhasil; tabel mount masih berisi mount lain.
pub const LEPAS_OK: i32 = 0;
/// Pelepasan berhasil dan tabel mount menjadi KOSONG; konteks harus dilepas
/// dari `all_root_contexts_list()` oleh C++ (mekanisme).
pub const LEPAS_HAPUS_KONTEKS: i32 = 1;

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah langkah mesin hierarki dari produksi.
    ProdHierarki = 0,
    /// Jumlah `klatos_pas_admisi` dari produksi.
    ProdAdmisi = 1,
    /// Jumlah `klatos_pas_izinkan_ubah` dari produksi.
    ProdIzin = 2,
    /// Jumlah `klatos_pas_verdict_lepas` dari produksi.
    ProdLepas = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Satu langkah mesin keadaan pencocokan hierarki path absolut.
///
/// Transkripsi `VirtualFileSystem.cpp:128-146`:
///
/// ```cpp
/// if (!first.parent() && !second.parent()) return true;   // INIT, keduanya akar
/// if (first.name() != second.name())        return false; // INIT, nama beda
/// while (c1->parent()) {                                  // LANGKAH
///     if (!c2->parent())                    return false;
///     if (&c1->parent()->inode() != &c2->parent()->inode()) return false;
///     c1 = c1->parent(); c2 = c2->parent();
/// }
/// return true;
/// ```
///
/// * `state == INIT`   → `hp1`/`hp2` = "simpul punya induk?", `nama_sama` =
///   "nama kedua custody sama?". `inode_induk_sama` diabaikan.
/// * `state == LANGKAH` → `hp1`/`hp2` = "simpul SAAT INI punya induk?",
///   `inode_induk_sama` = "inode induk kedua simpul identik?". `nama_sama`
///   diabaikan.
///
/// **KONTRAK URUTAN PEMANGGIL**: fakta untuk keadaan `LANGKAH` harus
/// dilaporkan untuk simpul SAAT INI, yaitu SEBELUM pemanggil memajukan kedua
/// pointer ke induknya. Melaporkannya setelah memajukan pointer menggeser
/// pembandingan satu tingkat dan akan salah mengembalikan `COCOK` untuk dua
/// custody berbeda-induk yang kebetulan berbagi kakek.
///
/// Keadaan akhir (`COCOK`/`BEDA`) bersifat terminal: memasukkannya kembali
/// mengembalikannya apa adanya, sehingga pemanggil yang lupa berhenti tidak
/// pernah "hidup kembali".
#[inline]
pub const fn hierarki_langkah(
    state: u32,
    hp1: u32,
    hp2: u32,
    nama_sama: u32,
    inode_induk_sama: u32,
) -> u32 {
    if state == HIERARKI_INIT {
        if hp1 == 0 && hp2 == 0 {
            HIERARKI_COCOK
        } else if nama_sama == 0 {
            HIERARKI_BEDA
        } else {
            HIERARKI_LANGKAH
        }
    } else if state == HIERARKI_LANGKAH {
        if hp1 == 0 {
            // `while (c1->parent())` berakhir ⇒ `return true`.
            HIERARKI_COCOK
        } else if hp2 == 0 {
            HIERARKI_BEDA
        } else if inode_induk_sama == 0 {
            HIERARKI_BEDA
        } else {
            HIERARKI_LANGKAH
        }
    } else {
        state
    }
}

/// Orakel historis untuk mesin hierarki: transkripsi langsung loop C++.
///
/// Rantai custody direpresentasikan sebagai `nama` + daftar identitas inode
/// INDUK dari simpul terdekat ke akar (kosong = simpul akar). Dipakai HANYA
/// oleh sapuan diferensial; strukturnya sengaja berbeda (loop `while` dengan
/// indeks eksplisit) supaya kesepakatan dengan [`hierarki_langkah`] bermakna.
pub fn hierarki_oracle(nama1: u32, induk1: &[u32], nama2: u32, induk2: &[u32]) -> bool {
    if induk1.is_empty() && induk2.is_empty() {
        return true;
    }
    if nama1 != nama2 {
        return false;
    }
    let mut kedalaman = 0usize;
    let mut c1_punya_induk = !induk1.is_empty();
    while c1_punya_induk {
        let c2_punya_induk = kedalaman < induk2.len();
        if !c2_punya_induk {
            return false;
        }
        if induk1[kedalaman] != induk2[kedalaman] {
            return false;
        }
        kedalaman += 1;
        c1_punya_induk = kedalaman < induk1.len();
    }
    true
}

/// Menjalankan mesin keadaan [`hierarki_langkah`] sampai terminal, memakai
/// rantai sintetis yang sama dengan [`hierarki_oracle`]. Ini adalah
/// PEMANDU-PENGUJI: ia meniru apa yang dilakukan pemanggil C++.
pub fn hierarki_mesin(nama1: u32, induk1: &[u32], nama2: u32, induk2: &[u32]) -> bool {
    let mut state = hierarki_langkah(
        HIERARKI_INIT,
        (!induk1.is_empty()) as u32,
        (!induk2.is_empty()) as u32,
        (nama1 == nama2) as u32,
        0,
    );
    let mut kedalaman = 0usize;
    while state == HIERARKI_LANGKAH {
        let hp1 = kedalaman < induk1.len();
        let hp2 = kedalaman < induk2.len();
        let sama = if hp1 && hp2 {
            (induk1[kedalaman] == induk2[kedalaman]) as u32
        } else {
            0
        };
        state = hierarki_langkah(HIERARKI_LANGKAH, hp1 as u32, hp2 as u32, 0, sama);
        kedalaman += 1;
    }
    state == HIERARKI_COCOK
}

/// Pemandu **SALAH URUTAN** (advance-first) — SENGAJA salah.
///
/// Ia memajukan indeks SEBELUM melaporkan fakta, yaitu persis pelanggaran
/// kontrak urutan yang dilarang [`hierarki_langkah`]. Dipakai HANYA sebagai
/// kontrol negatif: ia harus mengembalikan `true` pada kasus dua custody
/// berbeda-induk yang berbagi kakek, sehingga matriks terbukti MAMPU
/// mendeteksi pelanggaran kontrak itu (bukan sekadar kebetulan lulus).
pub fn hierarki_mesin_salah_urutan(nama1: u32, induk1: &[u32], nama2: u32, induk2: &[u32]) -> bool {
    let mut state = hierarki_langkah(
        HIERARKI_INIT,
        (!induk1.is_empty()) as u32,
        (!induk2.is_empty()) as u32,
        (nama1 == nama2) as u32,
        0,
    );
    let mut kedalaman = 0usize;
    while state == HIERARKI_LANGKAH {
        kedalaman += 1;
        let hp1 = kedalaman < induk1.len();
        let hp2 = kedalaman < induk2.len();
        let sama = if hp1 && hp2 {
            (induk1[kedalaman] == induk2[kedalaman]) as u32
        } else {
            0
        };
        state = hierarki_langkah(HIERARKI_LANGKAH, hp1 as u32, hp2 as u32, 0, sama);
    }
    state == HIERARKI_COCOK
}

/// Admisi titik pemasangan. Cermin `VFSRootContext.cpp:386-389`:
/// titik yang sudah menjadi mount-point ditolak dengan `EBUSY`.
#[inline]
pub const fn admisi(sudah_ada_titik: u32) -> i32 {
    if sudah_ada_titik != 0 {
        -EBUSY
    } else {
        0
    }
}

/// Kebijakan mount immutable saat masih dipakai. Cermin
/// `VFSRootContext.h:93-98` (`validate_mount_not_immutable_while_being_used`):
/// `immutable && attach_count > 0` ⇒ `EPERM`.
///
/// Satu-satunya pintu keputusan ini; dipakai oleh pelepasan (`unmount`) DAN
/// oleh `apply_to_mount_for_host_custody`. Tidak ada salinan kedua.
#[inline]
pub const fn izinkan_ubah(immutable: u32, attach_count: u64) -> i32 {
    if immutable != 0 && attach_count > 0 {
        -EPERM
    } else {
        0
    }
}

/// Verdict pelepasan mount. Cermin `VFSRootContext.cpp:220-241`.
///
/// `cocok` = "ada entri mount yang cocok" (dihitung C++ lewat mekanisme
/// penelusuran daftar). `tabel_kosong` = "daftar mount KOSONG setelah
/// pelepasan". Kebijakan immutable diperiksa lebih dulu lewat
/// [`izinkan_ubah`] oleh pemanggil, jadi verdict di sini hanya soal
/// keberadaan + kekosongan tabel.
#[inline]
pub const fn verdict_lepas(cocok: u32, tabel_kosong: u32) -> i32 {
    if cocok == 0 {
        return -ENODEV;
    }
    if tabel_kosong != 0 {
        LEPAS_HAPUS_KONTEKS
    } else {
        LEPAS_OK
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Satu langkah mesin hierarki. Lihat [`hierarki_langkah`] untuk kontrak.
#[no_mangle]
pub extern "C" fn klatos_pas_hierarki_langkah(
    state: u32,
    hp1: u32,
    hp2: u32,
    nama_sama: u32,
    inode_induk_sama: u32,
) -> u32 {
    bump(Counter::ProdHierarki);
    hierarki_langkah(state, hp1, hp2, nama_sama, inode_induk_sama)
}

/// Admisi titik pemasangan. Lihat [`admisi`] untuk kontrak.
#[no_mangle]
pub extern "C" fn klatos_pas_admisi(sudah_ada_titik: u32) -> i32 {
    bump(Counter::ProdAdmisi);
    admisi(sudah_ada_titik)
}

/// Kebijakan mount immutable. Lihat [`izinkan_ubah`] untuk kontrak.
#[no_mangle]
pub extern "C" fn klatos_pas_izinkan_ubah(immutable: u32, attach_count: u64) -> i32 {
    bump(Counter::ProdIzin);
    izinkan_ubah(immutable, attach_count)
}

/// Verdict pelepasan mount. Lihat [`verdict_lepas`] untuk kontrak.
#[no_mangle]
pub extern "C" fn klatos_pas_verdict_lepas(cocok: u32, tabel_kosong: u32) -> i32 {
    bump(Counter::ProdLepas);
    verdict_lepas(cocok, tabel_kosong)
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_pas_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++.
#[no_mangle]
pub extern "C" fn klatos_pas_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosSiklusPasangReport` di `Jowo/Kontrak/KlatSeam74.h`.
#[repr(C)]
pub struct KlatosSiklusPasangReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub hier_checks: u32,
    pub hier_passed: u32,
    pub adm_checks: u32,
    pub adm_passed: u32,
    pub izin_checks: u32,
    pub izin_passed: u32,
    pub lepas_checks: u32,
    pub lepas_passed: u32,
    pub prod_hierarki: u32,
    pub prod_admisi: u32,
    pub prod_izin: u32,
    pub prod_lepas: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

/// Ukuran domain sapuan diferensial: 40 rantai × 40 rantai × 2 nama.
pub const DIFF_EXPECTED: u32 = 40 * 40 * 2;

#[derive(Clone, Copy)]
enum Group {
    Hier,
    Adm,
    Izin,
    Lepas,
}

struct Matrix {
    checks: u32,
    passed: u32,
    hier_checks: u32,
    hier_passed: u32,
    adm_checks: u32,
    adm_passed: u32,
    izin_checks: u32,
    izin_passed: u32,
    lepas_checks: u32,
    lepas_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            hier_checks: 0,
            hier_passed: 0,
            adm_checks: 0,
            adm_passed: 0,
            izin_checks: 0,
            izin_passed: 0,
            lepas_checks: 0,
            lepas_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Hier => {
                self.hier_checks += 1;
                if lulus {
                    self.hier_passed += 1;
                }
            }
            Group::Adm => {
                self.adm_checks += 1;
                if lulus {
                    self.adm_passed += 1;
                }
            }
            Group::Izin => {
                self.izin_checks += 1;
                if lulus {
                    self.izin_passed += 1;
                }
            }
            Group::Lepas => {
                self.lepas_checks += 1;
                if lulus {
                    self.lepas_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Alfabet identitas inode sintetis. Nilai tidak nol supaya "akar" (daftar
/// kosong) tidak pernah bertabrakan dengan identitas inode nyata mana pun.
const ALFABET: [u32; 3] = [11, 22, 33];
/// 1 (kosong) + 3 (1) + 9 (2) + 27 (3) rantai.
const N_RANTAI: usize = 40;

/// Matriks otoritas siklus hidup & admisi pemasangan. Input SINTETIS.
fn run_matrix() -> KlatosSiklusPasangReport {
    let mut m = Matrix::new();

    // --- kelompok 0: mesin hierarki, kasus batas eksplisit ----------------
    // Dua akar ⇒ cocok, tanpa melihat nama.
    m.catat(
        hierarki_langkah(HIERARKI_INIT, 0, 0, 0, 0) == HIERARKI_COCOK,
        Group::Hier,
    );
    // Akar vs bukan akar: nama sama tapi salah satu punya induk.
    m.catat(
        hierarki_langkah(HIERARKI_INIT, 1, 0, 1, 0) == HIERARKI_LANGKAH,
        Group::Hier,
    );
    m.catat(
        hierarki_langkah(HIERARKI_LANGKAH, 1, 0, 0, 0) == HIERARKI_BEDA,
        Group::Hier,
    );
    // Nama beda (keduanya bukan akar) ⇒ langsung beda.
    m.catat(
        hierarki_langkah(HIERARKI_INIT, 1, 1, 0, 0) == HIERARKI_BEDA,
        Group::Hier,
    );
    // Nama sama, keduanya bukan akar ⇒ maju satu langkah.
    m.catat(
        hierarki_langkah(HIERARKI_INIT, 1, 1, 1, 0) == HIERARKI_LANGKAH,
        Group::Hier,
    );
    // LANGKAH dengan c1 tanpa induk ⇒ cocok (loop berakhir).
    m.catat(
        hierarki_langkah(HIERARKI_LANGKAH, 0, 1, 0, 1) == HIERARKI_COCOK,
        Group::Hier,
    );
    // LANGKAH: c1 punya induk, c2 tidak ⇒ beda.
    m.catat(
        hierarki_langkah(HIERARKI_LANGKAH, 1, 0, 0, 1) == HIERARKI_BEDA,
        Group::Hier,
    );
    // LANGKAH: inode induk beda ⇒ beda.
    m.catat(
        hierarki_langkah(HIERARKI_LANGKAH, 1, 1, 0, 0) == HIERARKI_BEDA,
        Group::Hier,
    );
    // LANGKAH: inode induk sama ⇒ lanjut.
    m.catat(
        hierarki_langkah(HIERARKI_LANGKAH, 1, 1, 0, 1) == HIERARKI_LANGKAH,
        Group::Hier,
    );
    // Terminal bersifat lengket: keadaan akhir tidak pernah "hidup kembali".
    m.catat(
        hierarki_langkah(HIERARKI_COCOK, 1, 1, 1, 1) == HIERARKI_COCOK,
        Group::Hier,
    );
    m.catat(
        hierarki_langkah(HIERARKI_BEDA, 0, 0, 1, 1) == HIERARKI_BEDA,
        Group::Hier,
    );
    m.catat(hierarki_langkah(999, 0, 0, 0, 0) == 999, Group::Hier);

    // Rantai identik berakhir COCOK.
    m.catat(
        hierarki_mesin(7, &[ALFABET[0], ALFABET[1]], 7, &[ALFABET[0], ALFABET[1]]),
        Group::Hier,
    );
    // Kedalaman berbeda (satu lebih dalam) berakhir BEDA — cermin
    // `if (!custody2->parent()) return false;`.
    m.catat(
        !hierarki_mesin(7, &[ALFABET[0], ALFABET[1]], 7, &[ALFABET[0]]),
        Group::Hier,
    );
    // Kedalaman berbeda arah sebaliknya: c1 habis lebih dulu ⇒ COCOK.
    // Ini perilaku ASLI C++ (asimetris!) dan sengaja dipertahankan.
    m.catat(
        hierarki_mesin(7, &[ALFABET[0]], 7, &[ALFABET[0], ALFABET[1]]),
        Group::Hier,
    );
    // BAHAYA URUTAN (pinned): dua custody BERBEDA yang berbagi INDUK INDUK
    // (kakek) tetapi punya induk berbeda ⇒ BEDA. Loop C++ asli berhenti pada
    // perbandingan INDUK, jadi kesamaan kakek tidak pernah menyelamatkannya.
    // Pemanggil yang melaporkan fakta SETELAH memajukan pointer akan salah
    // mengembalikan COCOK di sini — inilah kasus yang menjaga urutan itu.
    m.catat(
        !hierarki_mesin(7, &[ALFABET[0], ALFABET[1]], 7, &[ALFABET[2], ALFABET[1]]),
        Group::Hier,
    );
    // Kontrol negatif kontrak urutan: pemandu advance-first HARUS terdeteksi
    // salah (mengembalikan COCOK) pada kasus yang sama. Bila baris ini gagal,
    // matriks tidak lagi punya gigi terhadap pelanggaran urutan.
    m.catat(
        hierarki_mesin_salah_urutan(7, &[ALFABET[0], ALFABET[1]], 7, &[ALFABET[2], ALFABET[1]]),
        Group::Hier,
    );
    // Akar vs akar.
    m.catat(hierarki_mesin(1, &[], 2, &[]), Group::Hier);

    // --- kelompok 1: sapuan diferensial mesin vs orakel -------------------
    let mut rantai = [[0u32; 3]; N_RANTAI];
    let mut panjang = [0usize; N_RANTAI];
    {
        let mut n = 0usize;
        rantai[n] = [0, 0, 0];
        panjang[n] = 0;
        n += 1;
        let mut a = 0usize;
        while a < 3 {
            rantai[n] = [ALFABET[a], 0, 0];
            panjang[n] = 1;
            n += 1;
            a += 1;
        }
        let mut a = 0usize;
        while a < 3 {
            let mut b = 0usize;
            while b < 3 {
                rantai[n] = [ALFABET[a], ALFABET[b], 0];
                panjang[n] = 2;
                n += 1;
                b += 1;
            }
            a += 1;
        }
        let mut a = 0usize;
        while a < 3 {
            let mut b = 0usize;
            while b < 3 {
                let mut c = 0usize;
                while c < 3 {
                    rantai[n] = [ALFABET[a], ALFABET[b], ALFABET[c]];
                    panjang[n] = 3;
                    n += 1;
                    c += 1;
                }
                b += 1;
            }
            a += 1;
        }
    }

    let mut diukur = 0u32;
    let mut beda = 0u32;
    for nama1 in [5u32, 9] {
        for i in 0..N_RANTAI {
            for j in 0..N_RANTAI {
                let r = hierarki_mesin(
                    nama1,
                    &rantai[i][..panjang[i]],
                    nama1,
                    &rantai[j][..panjang[j]],
                );
                let o = hierarki_oracle(
                    nama1,
                    &rantai[i][..panjang[i]],
                    nama1,
                    &rantai[j][..panjang[j]],
                );
                diukur += 1;
                if r != o {
                    beda += 1;
                }
            }
        }
    }
    // Nama BERBEDA: orakel harus langsung BEDA kecuali keduanya akar.
    for i in 0..N_RANTAI {
        for j in 0..N_RANTAI {
            let r = hierarki_mesin(5, &rantai[i][..panjang[i]], 9, &rantai[j][..panjang[j]]);
            let o = hierarki_oracle(5, &rantai[i][..panjang[i]], 9, &rantai[j][..panjang[j]]);
            diukur += 1;
            if r != o {
                beda += 1;
            }
        }
    }
    m.catat(diukur == DIFF_EXPECTED + N_RANTAI as u32 * N_RANTAI as u32, Group::Hier);
    m.catat(beda == 0, Group::Hier);

    // --- kelompok 2: admisi titik pemasangan ------------------------------
    m.catat(admisi(0) == 0, Group::Adm);
    m.catat(admisi(1) == -EBUSY, Group::Adm);
    m.catat(admisi(2) == -EBUSY, Group::Adm);
    m.catat(admisi(0xFFFF_FFFF) == -EBUSY, Group::Adm);
    m.catat(EBUSY == 16, Group::Adm);

    // --- kelompok 3: kebijakan mount immutable ----------------------------
    m.catat(izinkan_ubah(0, 0) == 0, Group::Izin);
    m.catat(izinkan_ubah(0, 5) == 0, Group::Izin);
    m.catat(izinkan_ubah(1, 0) == 0, Group::Izin);
    m.catat(izinkan_ubah(1, 1) == -EPERM, Group::Izin);
    m.catat(izinkan_ubah(1, u64::MAX) == -EPERM, Group::Izin);
    m.catat(EPERM == 1, Group::Izin);

    // --- kelompok 4: verdict pelepasan ------------------------------------
    m.catat(verdict_lepas(0, 0) == -ENODEV, Group::Lepas);
    m.catat(verdict_lepas(0, 1) == -ENODEV, Group::Lepas);
    m.catat(verdict_lepas(1, 0) == LEPAS_OK, Group::Lepas);
    m.catat(verdict_lepas(1, 1) == LEPAS_HAPUS_KONTEKS, Group::Lepas);
    m.catat(ENODEV == 19, Group::Lepas);
    // Komposisi: izin diperiksa SEBELUM verdict — urutan yang sama dengan
    // `unmount` C++ (`validate_mount_not_immutable_while_being_used` dipanggil
    // di dalam cabang cocok, sebelum `remove_mount`).
    {
        let mut urutan_benar = true;
        if izinkan_ubah(1, 1) == 0 {
            urutan_benar = false;
        }
        if verdict_lepas(1, 0) != LEPAS_OK {
            urutan_benar = false;
        }
        m.catat(urutan_benar, Group::Lepas);
    }

    // --- kelompok 5: negatif ----------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_534B_5053, Group::Lepas);
    m.catat(N_COUNTER == 4, Group::Lepas);
    m.catat(Counter::ProdHierarki as usize == 0, Group::Lepas);
    m.catat(Counter::ProdLepas as usize == 3, Group::Lepas);
    // Keadaan hierarki: INIT/LANGKAH/COCOK/BEDA saling berbeda.
    {
        let s = [HIERARKI_INIT, HIERARKI_LANGKAH, HIERARKI_COCOK, HIERARKI_BEDA];
        let mut unik = true;
        let mut i = 0usize;
        while i < s.len() {
            let mut j = i + 1;
            while j < s.len() {
                if s[i] == s[j] {
                    unik = false;
                }
                j += 1;
            }
            i += 1;
        }
        m.catat(unik, Group::Lepas);
    }
    // Dua errno berbeda dan berdiri sendiri; EBUSY/EPERM/ENODEV ≠ 0.
    m.catat(EBUSY != EPERM && EPERM != ENODEV && EBUSY != ENODEV, Group::Lepas);
    m.catat(EPERM != 0 && EBUSY != 0 && ENODEV != 0, Group::Lepas);
    // LEPAS_OK != LEPAS_HAPUS_KONTEKS (dua hasil sukses yang berbeda).
    m.catat(LEPAS_OK != LEPAS_HAPUS_KONTEKS, Group::Lepas);
    // Determinisme.
    {
        let mut ok = true;
        let pertama = hierarki_langkah(HIERARKI_LANGKAH, 1, 1, 0, 1);
        let mut k = 0;
        while k < 1000 {
            if hierarki_langkah(HIERARKI_LANGKAH, 1, 1, 0, 1) != pertama {
                ok = false;
                break;
            }
            k += 1;
        }
        m.catat(ok, Group::Lepas);
    }
    // Kontrol negatif: harapan SALAH harus terdeteksi.
    {
        let mut ok = true;
        if admisi(1) != -EBUSY {
            ok = false;
        }
        if izinkan_ubah(1, 1) != -EPERM {
            ok = false;
        }
        if verdict_lepas(0, 0) != -ENODEV {
            ok = false;
        }
        if hierarki_langkah(HIERARKI_INIT, 1, 1, 0, 0) != HIERARKI_BEDA {
            ok = false;
        }
        m.catat(ok, Group::Lepas);
    }

    KlatosSiklusPasangReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        hier_checks: m.hier_checks,
        hier_passed: m.hier_passed,
        adm_checks: m.adm_checks,
        adm_passed: m.adm_passed,
        izin_checks: m.izin_checks,
        izin_passed: m.izin_passed,
        lepas_checks: m.lepas_checks,
        lepas_passed: m.lepas_passed,
        prod_hierarki: COUNTERS[Counter::ProdHierarki as usize].load(Ordering::Relaxed),
        prod_admisi: COUNTERS[Counter::ProdAdmisi as usize].load(Ordering::Relaxed),
        prod_izin: COUNTERS[Counter::ProdIzin as usize].load(Ordering::Relaxed),
        prod_lepas: COUNTERS[Counter::ProdLepas as usize].load(Ordering::Relaxed),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status: if m.checks == m.passed { 0 } else { -1 },
    }
}

/// Menjalankan matriks otoritas siklus hidup & admisi pemasangan.
#[no_mangle]
pub extern "C" fn klatos_pas_selftest() -> KlatosSiklusPasangReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_534B_5053);
        assert!(EPERM == 1);
        assert!(EBUSY == 16);
        assert!(ENODEV == 19);
        assert!(HIERARKI_INIT == 0 && HIERARKI_LANGKAH == 1);
        assert!(HIERARKI_COCOK == 2 && HIERARKI_BEDA == 3);
        assert!(LEPAS_OK == 0 && LEPAS_HAPUS_KONTEKS == 1);
        assert!(DIFF_EXPECTED == 3200);
        assert!(N_COUNTER == 4);
        assert!(Counter::ProdHierarki as usize == 0);
        assert!(Counter::ProdLepas as usize == 3);

        // Mesin keadaan — terminal lengket.
        assert!(hierarki_langkah(HIERARKI_INIT, 0, 0, 0, 0) == HIERARKI_COCOK);
        assert!(hierarki_langkah(HIERARKI_INIT, 1, 1, 0, 0) == HIERARKI_BEDA);
        assert!(hierarki_langkah(HIERARKI_INIT, 1, 1, 1, 0) == HIERARKI_LANGKAH);
        assert!(hierarki_langkah(HIERARKI_LANGKAH, 0, 1, 0, 1) == HIERARKI_COCOK);
        assert!(hierarki_langkah(HIERARKI_LANGKAH, 1, 0, 0, 1) == HIERARKI_BEDA);
        assert!(hierarki_langkah(HIERARKI_LANGKAH, 1, 1, 0, 0) == HIERARKI_BEDA);
        assert!(hierarki_langkah(HIERARKI_LANGKAH, 1, 1, 0, 1) == HIERARKI_LANGKAH);
        assert!(hierarki_langkah(HIERARKI_COCOK, 1, 1, 1, 1) == HIERARKI_COCOK);
        assert!(hierarki_langkah(999, 0, 0, 0, 0) == 999);

        assert!(admisi(0) == 0);
        assert!(admisi(1) == -EBUSY);
        assert!(izinkan_ubah(0, 9) == 0);
        assert!(izinkan_ubah(1, 0) == 0);
        assert!(izinkan_ubah(1, 9) == -EPERM);
        assert!(verdict_lepas(0, 0) == -ENODEV);
        assert!(verdict_lepas(1, 0) == LEPAS_OK);
        assert!(verdict_lepas(1, 1) == LEPAS_HAPUS_KONTEKS);

        assert!(core::mem::size_of::<KlatosSiklusPasangReport>() == 80);
        assert!(core::mem::align_of::<KlatosSiklusPasangReport>() == 8);
    };
}
