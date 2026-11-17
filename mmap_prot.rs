// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas kebijakan proteksi mmap W^X Jowo (P1.09).


use core::sync::atomic::{AtomicU32, Ordering};

/// Kode galat POSIX `EINVAL` (cermin `<errno.h>` C).
pub const EINVAL: i32 = 22;

/// Cermin `<sys/mman.h>` `PROT_READ`.
pub const PROT_READ: i32 = 0x1;

/// Cermin `<sys/mman.h>` `PROT_WRITE`.
pub const PROT_WRITE: i32 = 0x2;

/// Cermin `<sys/mman.h>` `PROT_EXEC`.
pub const PROT_EXEC: i32 = 0x4;

/// Cermin `<unistd.h>` `MS_WXALLOWED` (mount mengizinkan W+X).
pub const MS_WXALLOWED: i32 = 1 << 6;

/// Cermin `<unistd.h>` `MS_AXALLOWED` (mount mengizinkan anonim-eksekusi).
pub const MS_AXALLOWED: i32 = 1 << 7;

/// Tag laporan. BERBEDA dari `frame` ("KLATKRNL"), `task_id` ("KLATTKID"),
/// `priority_bucket` ("KLATSCHD"), `ipc_handle` ("KLATIPCH"),
/// `clock_id` ("KLATCID"), dan `timer` — masing-masing laporan adalah
/// keputusan berbeda dan C++ memeriksanya lewat konstanta berbeda.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4D4D_4150; // "KLATMMAP"

/// Galat otoritas proteksi mmap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MmapProtError {
    /// Permintaan melanggar kebijakan W^X. Galat POSIX `EINVAL`.
    Ditolak,
}

impl MmapProtError {
    /// Kode C ABI: negatif, sehingga `>= 0` berarti diizinkan dan errno
    /// absolut bisa dipulihkan dengan `(-e.kode())`.
    pub const fn kode(&self) -> i32 {
        match self {
            MmapProtError::Ditolak => -EINVAL,
        }
    }

    /// Kode POSIX errno absolut (untuk dibandingkan dengan `EINVAL`).
    pub const fn errno(&self) -> i32 {
        match self {
            MmapProtError::Ditolak => EINVAL,
        }
    }
}

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah total pemanggilan [`klatos_mmap_prot_decide`] dari produksi.
    ProdCalls = 0,
    /// Jumlah penolakan dari produksi.
    ProdReject = 1,
    /// Jumlah keputusan `Ok` yang SEHARUSNYA `EINVAL` menurut keputusan
    /// C++ historis (harus 0 jika byte-identik).
    DriftReject = 2,
}

const N_COUNTER: usize = 3;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mengembalikan `Ok(())` bila permintaan diizinkan, atau
/// `Err(MmapProtError::Ditolak)` bila ditolak.
///
/// Urutan dan isi keputusan pada input yang sama SAMA PERSIS dengan urutan
/// byte keputusan C++ (`KlatKernel/Efek/Memori/mmap.cpp:62-88`). Enam pemeriksaan
/// dijalankan berurutan; yang pertama cocok menentukan hasil.
///
/// `region_has_been_executable` dan `region_has_been_writable` hanya
/// bermakna bila `has_region` benar (cermin `region != nullptr`).
///
/// `# Safety`: tidak ada — fungsi ini murni, tidak menyentuh memori bersama.
#[inline]
#[allow(clippy::too_many_arguments)]
pub const fn putuskan(
    prot: i32,
    map_stack: bool,
    map_anonymous: bool,
    ax_allowed: bool,
    wx_allowed: bool,
    has_region: bool,
    region_has_been_executable: bool,
    region_has_been_writable: bool,
    tolak_transisi: bool,
) -> Result<(), MmapProtError> {
    let make_writable = prot & PROT_WRITE != 0;
    let make_executable = prot & PROT_EXEC != 0;

    // 1. mmap anonim yang dapat-dieksekusi butuh mount MS_AXALLOWED.
    if map_anonymous && make_executable && !ax_allowed {
        return Err(MmapProtError::Ditolak);
    }

    // 2. stack yang dapat-dieksekusi selalu ditolak.
    if map_stack && make_executable {
        return Err(MmapProtError::Ditolak);
    }

    // 3. mount MS_WXALLOWED membebaskan sisa pemeriksaan.
    if wx_allowed {
        return Ok(());
    }

    // 4. W+X polos ditolak.
    if make_writable && make_executable {
        return Err(MmapProtError::Ditolak);
    }

    // 5. riwayat region (hanya bila region ada).
    if has_region {
        if make_writable && region_has_been_executable {
            return Err(MmapProtError::Ditolak);
        }
        if make_executable && region_has_been_writable && tolak_transisi {
            return Err(MmapProtError::Ditolak);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan `0` (= diizinkan) atau `< 0` (= kode galat absolut,
/// `EINVAL`). Parameter flag adalah `u32` (`0` = salah, selain itu = benar)
/// karena C ABI tidak memiliki `bool` yang stabil antar-kompiler.
///
/// Menaikkan `prod_calls`, dan `prod_reject` bila menolak.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn klatos_mmap_prot_decide(
    prot: i32,
    map_stack: u32,
    map_anonymous: u32,
    ax_allowed: u32,
    wx_allowed: u32,
    has_region: u32,
    region_has_been_executable: u32,
    region_has_been_writable: u32,
    tolak_transisi: u32,
) -> i32 {
    bump(Counter::ProdCalls);
    match putuskan(
        prot,
        map_stack != 0,
        map_anonymous != 0,
        ax_allowed != 0,
        wx_allowed != 0,
        has_region != 0,
        region_has_been_executable != 0,
        region_has_been_writable != 0,
        tolak_transisi != 0,
    ) {
        Ok(()) => 0,
        Err(e) => {
            bump(Counter::ProdReject);
            e.kode()
        }
    }
}

/// Kode galat absolut POSIX untuk keputusan `Err(Ditolak)`. Konstanta murni.
#[no_mangle]
pub extern "C" fn klatos_mmap_prot_einval() -> i32 {
    EINVAL
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_mmap_prot_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++
/// (matriks tidak menyentuh produksi), tetapi disediakan untuk konsistensi
/// dengan modul lain.
#[no_mangle]
pub extern "C" fn klatos_mmap_prot_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosMmapProtReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
/// Tata letaknya dijaga `static_assert` di sisi C++ dan diuji harness host.
#[repr(C)]
pub struct KlatosMmapProtReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub allow_checks: u32,
    pub allow_passed: u32,
    pub reject_checks: u32,
    pub reject_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub prod_reject: u32,
    pub prod_reject_expected: u32,
    pub prod_calls: u32,
    pub einval: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Allow,
    Reject,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    allow_checks: u32,
    allow_passed: u32,
    reject_checks: u32,
    reject_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            allow_checks: 0,
            allow_passed: 0,
            reject_checks: 0,
            reject_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Allow => {
                self.allow_checks += 1;
                if lulus {
                    self.allow_passed += 1;
                }
            }
            Group::Reject => {
                self.reject_checks += 1;
                if lulus {
                    self.reject_passed += 1;
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

const ALLOW: Result<(), MmapProtError> = Ok(());
const TOLAK: Result<(), MmapProtError> = Err(MmapProtError::Ditolak);

/// Matriks otoritas proteksi mmap. Berjalan pada nilai SINTETIS — tidak
/// pernah memanggil [`klatos_mmap_prot_decide`] sehingga tidak menggeser
/// penghitung produksi; produksi dilihat terpisah oleh `init_stage2` lewat
/// [`klatos_mmap_prot_counter`].
fn run_matrix() -> KlatosMmapProtReport {
    let mut m = Matrix::new();

    // --- kelompok 0: diizinkan ---------------------------------------------
    // PROT_READ polos, tanpa region.
    m.catat(
        putuskan(PROT_READ, false, false, false, false, false, false, false, true) == ALLOW,
        Group::Allow,
    );
    // PROT_READ|PROT_WRITE pada mount MS_WXALLOWED.
    m.catat(
        putuskan(PROT_READ | PROT_WRITE, false, false, false, true, false, false, false, true)
            == ALLOW,
        Group::Allow,
    );
    // anonim + eksekusi pada mount MS_AXALLOWED.
    m.catat(
        putuskan(PROT_READ | PROT_EXEC, false, true, true, false, false, false, false, true)
            == ALLOW,
        Group::Allow,
    );
    // eksekusi tanpa tulis, tanpa region (region null melewati cek 5).
    m.catat(
        putuskan(PROT_READ | PROT_EXEC, false, false, false, false, false, false, false, true)
            == ALLOW,
        Group::Allow,
    );
    // eksekusi pada region yang pernah-writable bila transisi TIDAK ditolak.
    m.catat(
        putuskan(PROT_READ | PROT_EXEC, false, false, false, false, true, false, true, false)
            == ALLOW,
        Group::Allow,
    );
    // tulis pada region yang pernah-eksekusi bila mount WX (cek 3 lebih dulu).
    m.catat(
        putuskan(PROT_READ | PROT_WRITE, false, false, false, true, true, true, false, true)
            == ALLOW,
        Group::Allow,
    );
    // prot = 0 (tanpa tulis/eksekusi).
    m.catat(
        putuskan(0, false, false, false, false, true, true, true, true) == ALLOW,
        Group::Allow,
    );

    // --- kelompok 1: ditolak ------------------------------------------------
    // anonim + eksekusi tanpa MS_AXALLOWED.
    m.catat(
        putuskan(PROT_EXEC, false, true, false, false, false, false, false, true) == TOLAK,
        Group::Reject,
    );
    // stack + eksekusi.
    m.catat(
        putuskan(PROT_EXEC, true, false, false, false, false, false, false, true) == TOLAK,
        Group::Reject,
    );
    // W+X polos tanpa MS_WXALLOWED.
    m.catat(
        putuskan(PROT_WRITE | PROT_EXEC, false, false, false, false, false, false, false, true)
            == TOLAK,
        Group::Reject,
    );
    // tulis pada region yang pernah-eksekusi.
    m.catat(
        putuskan(PROT_READ | PROT_WRITE, false, false, false, false, true, true, false, true)
            == TOLAK,
        Group::Reject,
    );
    // eksekusi pada region yang pernah-writable bila transisi DITOLAK.
    m.catat(
        putuskan(PROT_READ | PROT_EXEC, false, false, false, false, true, false, true, true)
            == TOLAK,
        Group::Reject,
    );

    // --- kelompok 2: urutan (prioritas pemeriksaan) -------------------------
    // anonim+eksekusi tanpa AX menang atas jalan-pintas MS_WXALLOWED.
    m.catat(
        putuskan(PROT_EXEC, false, true, false, true, false, false, false, true) == TOLAK,
        Group::Reject,
    );
    // stack+eksekusi menang atas jalan-pintas MS_WXALLOWED.
    m.catat(
        putuskan(PROT_EXEC, true, false, false, true, false, false, false, true) == TOLAK,
        Group::Reject,
    );
    // W+X tanpa WX menang atas cek region (region ada, riwayat kosong).
    m.catat(
        putuskan(PROT_WRITE | PROT_EXEC, false, false, false, false, true, false, false, true)
            == TOLAK,
        Group::Reject,
    );
    // cek 5a (tulis vs pernah-eksekusi) menang atas cek 5b.
    m.catat(
        putuskan(PROT_WRITE, false, false, false, false, true, true, true, true) == TOLAK,
        Group::Reject,
    );
    // has_region=false menonaktifkan cek 5 sepenuhnya.
    m.catat(
        putuskan(PROT_WRITE, false, false, false, false, false, true, true, true) == ALLOW,
        Group::Allow,
    );
    // PROT_WRITE saja, tanpa region -> diizinkan.
    m.catat(
        putuskan(PROT_WRITE, false, false, false, false, false, false, false, true) == ALLOW,
        Group::Allow,
    );

    // --- kelompok 3: negatif (harus BISA gagal) -----------------------------
    m.catat(EINVAL == 22, Group::Neg);
    m.catat(PROT_READ == 0x1, Group::Neg);
    m.catat(PROT_WRITE == 0x2, Group::Neg);
    m.catat(PROT_EXEC == 0x4, Group::Neg);
    m.catat(MS_WXALLOWED == (1 << 6), Group::Neg);
    m.catat(MS_AXALLOWED == (1 << 7), Group::Neg);
    m.catat(MmapProtError::Ditolak.kode() == -EINVAL, Group::Neg);
    m.catat(MmapProtError::Ditolak.errno() == EINVAL, Group::Neg);
    // Konsistensi: 1000 panggilan dengan input sama menghasilkan keputusan
    // sama (tidak ada fluktuasi).
    let mut last: Option<Result<(), MmapProtError>> = None;
    let mut ok = true;
    for _ in 0..1000 {
        let r = putuskan(PROT_READ | PROT_WRITE, false, false, false, false, false, false, false, true);
        if last.is_none() {
            last = Some(r);
        } else if last != Some(r) {
            ok = false;
            break;
        }
    }
    m.catat(ok, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosMmapProtReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        allow_checks: m.allow_checks,
        allow_passed: m.allow_passed,
        reject_checks: m.reject_checks,
        reject_passed: m.reject_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_reject: klatos_mmap_prot_counter(Counter::ProdReject as u32),
        prod_reject_expected: 0,
        prod_calls: klatos_mmap_prot_counter(Counter::ProdCalls as u32),
        einval: EINVAL as u32,
        status,
    }
}

/// Menjalankan matriks otoritas proteksi mmap.
#[no_mangle]
pub extern "C" fn klatos_mmap_prot_selftest() -> KlatosMmapProtReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    /// Pembantu konformansi: `true` bila keputusan `Ok`.
    const fn diizinkan(r: Result<(), MmapProtError>) -> bool {
        matches!(r, Ok(()))
    }

    const _: () = {
        // Konstanta dasar.
        assert!(EINVAL == 22);
        assert!(PROT_READ == 0x1);
        assert!(PROT_WRITE == 0x2);
        assert!(PROT_EXEC == 0x4);
        assert!(MS_WXALLOWED == 64);
        assert!(MS_AXALLOWED == 128);

        // Tag laporan konstan.
        assert!(REPORT_TAG == 0x4B4C_4154_4D4D_4150);

        // byte-identik: PROT_READ polos diizinkan.
        assert!(diizinkan(putuskan(PROT_READ, false, false, false, false, false, false, false, true)));
        // byte-identik: W+X polos ditolak.
        assert!(!diizinkan(putuskan(
            PROT_WRITE | PROT_EXEC,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            true
        )));
        // Prioritas: anonim+eksekusi tanpa AX ditolak walau WX diizinkan.
        assert!(!diizinkan(putuskan(PROT_EXEC, false, true, false, true, false, false, false, true)));
        // stack+eksekusi ditolak walau WX diizinkan.
        assert!(!diizinkan(putuskan(PROT_EXEC, true, false, false, true, false, false, false, true)));

        // Kode galat negatif.
        assert!(MmapProtError::Ditolak.kode() < 0);
        assert!(MmapProtError::Ditolak.kode() == -EINVAL);
        assert!(MmapProtError::Ditolak.errno() == EINVAL);

        // Urutan penghitung audit stabil.
        assert!(Counter::ProdCalls as usize == 0);
        assert!(Counter::ProdReject as usize == 1);
        assert!(Counter::DriftReject as usize == 2);

        // Ukuran laporan: 8 (tag) + 12*4 (u32) + 4 (status) = 60 -> 64
        // dengan padding 4 byte di akhir. Cermin `KlatosMmapProtReport`
        // di KlatKernelBridge.h.
        assert!(core::mem::size_of::<KlatosMmapProtReport>() == 64);
        assert!(core::mem::align_of::<KlatosMmapProtReport>() == 8);
    };
}
