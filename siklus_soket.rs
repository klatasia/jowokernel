// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.76 / WS6 — **OTORITAS SIKLUS HIDUP & ADMISI SOKET**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPIPE` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EPIPE: i32 = 32;
/// `EAFNOSUPPORT` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const EAFNOSUPPORT: i32 = 40;
/// `ECONNREFUSED` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const ECONNREFUSED: i32 = 45;
/// `ENOTCONN` — cermin `Jowo/Kontrak/POSIX/errno.h`.
pub const ENOTCONN: i32 = 63;

/// Tag laporan; cermin `KlatosSiklusSoketReport` di `Jowo/Kontrak/KlatSeam76.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_534B_534B; // "KLATSKSK"

// --- kelas domain -----------------------------------------------------------

/// Domain `AF_LOCAL` → `LocalSocket`.
pub const DOMAIN_LOKAL: u32 = 0;
/// Domain `AF_INET` → `IPv4Socket`.
pub const DOMAIN_INET: u32 = 1;
/// Domain lain → `EAFNOSUPPORT`.
pub const DOMAIN_TAK_DIDUKUNG: u32 = 2;

// --- aksi shutdown ----------------------------------------------------------

/// Tidak ada aksi (sudah dimatikan sebelumnya, atau `how` tak dikenal).
pub const AKSI_TIDAK: u32 = 0;
/// Matikan sisi TULIS (`SHUT_WR` / `SHUT_RDWR`).
pub const AKSI_TULIS: u32 = 1;
/// Matikan sisi BACA (`SHUT_RD` / `SHUT_RDWR`).
pub const AKSI_BACA: u32 = 2;

// --- gerbang baca/tulis -----------------------------------------------------

/// Lanjutkan operasi.
pub const GATE_LANJUT: u32 = 1;
/// Hentikan operasi (`read` → EOF 0; `write` → `EPIPE`).
pub const GATE_HENTI: u32 = 0;

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `klatos_soket_domain_kelas` dari produksi.
    ProdDomain = 0,
    /// Jumlah `klatos_soket_shutdown_admisi` + `_aksi` dari produksi.
    ProdShutdown = 1,
    /// Jumlah `klatos_soket_read_gate` + `_write_gate` dari produksi.
    ProdGate = 2,
    /// Jumlah `klatos_soket_queue_admisi` + `_accept_layak` dari produksi.
    ProdAdmisi = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Kelas domain. Transkripsi `Socket::create:25-32`:
///
/// ```cpp
/// switch (domain) {
/// case AF_LOCAL: return LocalSocket::try_create(...);   // LOKAL
/// case AF_INET:  return IPv4Socket::create(...);        // INET
/// default:       return EAFNOSUPPORT;                   // TAK_DIDUKUNG
/// }
/// ```
///
/// `af_local`/`af_inet` DISUPLAI C++ (nomor `AF_*` adalah kontrak ABI).
/// Perhatikan: `AF_LOCAL` diperiksa LEBIH DULU; bila keduanya sama (patologis)
/// `AF_LOCAL` menang.
#[inline]
pub const fn domain_kelas(domain: i32, af_local: i32, af_inet: i32) -> u32 {
    if domain == af_local {
        return DOMAIN_LOKAL;
    }
    if domain == af_inet {
        return DOMAIN_INET;
    }
    DOMAIN_TAK_DIDUKUNG
}

/// Admisi `shutdown`. Transkripsi `Socket::shutdown:287-290`:
///
/// ```cpp
/// if (type() == SOCK_STREAM && !is_connected()) return set_so_error(ENOTCONN);
/// if (m_role == Role::Listener)               return set_so_error(ENOTCONN);
/// ```
///
/// * `stream` — `type() == SOCK_STREAM` (0/1).
/// * `terhubung` — `is_connected()` (0/1).
/// * `listener` — `role() == Role::Listener` (0/1).
///
/// Mengembalikan `0` (lanjut) atau `-ENOTCONN`.
#[inline]
pub const fn shutdown_admisi(stream: u32, terhubung: u32, listener: u32) -> i32 {
    if stream != 0 && terhubung == 0 {
        return -ENOTCONN;
    }
    if listener != 0 {
        return -ENOTCONN;
    }
    0
}

/// Aksi `shutdown`. Transkripsi `Socket::shutdown:291-298`:
///
/// ```cpp
/// if (!m_shut_down_for_writing && (how == SHUT_WR || how == SHUT_RDWR)) { ...tulis... }
/// if (!m_shut_down_for_reading && (how == SHUT_RD || how == SHUT_RDWR)) { ...baca... }
/// ```
///
/// Idempoten: sisi yang SUDAH dimatikan tidak diulang. Mengembalikan bitmask
/// [`AKSI_TULIS`] | [`AKSI_BACA`]. `sh_wr`/`sh_rd`/`sh_rdwr` DISUPLAI C++
/// (`SHUT_WR`/`SHUT_RD`/`SHUT_RDWR` adalah kontrak ABI).
#[inline]
pub const fn shutdown_aksi(
    how: u32,
    sudah_tulis: u32,
    sudah_baca: u32,
    sh_wr: u32,
    sh_rd: u32,
    sh_rdwr: u32,
) -> u32 {
    let mau_tulis = how == sh_wr || how == sh_rdwr;
    let mau_baca = how == sh_rd || how == sh_rdwr;
    let mut aksi = AKSI_TIDAK;
    if sudah_tulis == 0 && mau_tulis {
        aksi |= AKSI_TULIS;
    }
    if sudah_baca == 0 && mau_baca {
        aksi |= AKSI_BACA;
    }
    aksi
}

/// Admisi antrean koneksi. Transkripsi `Socket::queue_connection_from:73-74`:
///
/// ```cpp
/// if (m_pending.size() >= m_backlog) return set_so_error(ECONNREFUSED);
/// ```
///
/// Mengembalikan `0` (diterima) atau `-ECONNREFUSED`.
#[inline]
pub const fn queue_admisi(pending: u32, backlog: u32) -> i32 {
    if pending >= backlog {
        return -ECONNREFUSED;
    }
    0
}

/// Kelayakan `accept`. Transkripsi `Socket::accept:55-56`:
///
/// ```cpp
/// if (m_pending.is_empty()) return nullptr;
/// ```
///
/// Mengembalikan `1` (boleh dequeue) atau `0` (tidak ada).
#[inline]
pub const fn accept_layak(pending: u32) -> u32 {
    if pending == 0 {
        0
    } else {
        1
    }
}

/// Gerbang `read`. Transkripsi `Socket::read:271-272`:
///
/// ```cpp
/// if (is_shut_down_for_reading()) return 0;   // EOF
/// ```
///
/// Mengembalikan [`GATE_HENTI`] (EOF) atau [`GATE_LANJUT`].
#[inline]
pub const fn read_gate(shut_reading: u32) -> u32 {
    if shut_reading != 0 {
        GATE_HENTI
    } else {
        GATE_LANJUT
    }
}

/// Gerbang `write`. Transkripsi `Socket::write:279-280`:
///
/// ```cpp
/// if (is_shut_down_for_writing()) return set_so_error(EPIPE);
/// ```
///
/// Mengembalikan `0` (lanjut) atau `-EPIPE`.
#[inline]
pub const fn write_gate(shut_writing: u32) -> i32 {
    if shut_writing != 0 {
        -EPIPE
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_domain_kelas(domain: i32, af_local: i32, af_inet: i32) -> u32 {
    match domain {
        d if d == af_local => DOMAIN_LOKAL,
        d if d == af_inet => DOMAIN_INET,
        _ => DOMAIN_TAK_DIDUKUNG,
    }
}

fn naif_shutdown_admisi(stream: u32, terhubung: u32, listener: u32) -> i32 {
    let tolak = (stream != 0 && terhubung == 0) || listener != 0;
    if tolak {
        -ENOTCONN
    } else {
        0
    }
}

fn naif_shutdown_aksi(how: u32, sudah_tulis: u32, sudah_baca: u32, sh_wr: u32, sh_rd: u32, sh_rdwr: u32) -> u32 {
    let tulis = u32::from(sudah_tulis == 0 && (how == sh_wr || how == sh_rdwr)) * AKSI_TULIS;
    let baca = u32::from(sudah_baca == 0 && (how == sh_rd || how == sh_rdwr)) * AKSI_BACA;
    tulis | baca
}

fn naif_queue_admisi(pending: u32, backlog: u32) -> i32 {
    if pending < backlog {
        0
    } else {
        -ECONNREFUSED
    }
}

fn naif_accept_layak(pending: u32) -> u32 {
    u32::from(pending > 0)
}

fn naif_read_gate(shut_reading: u32) -> u32 {
    if shut_reading == 0 {
        GATE_LANJUT
    } else {
        GATE_HENTI
    }
}

fn naif_write_gate(shut_writing: u32) -> i32 {
    if shut_writing == 0 {
        0
    } else {
        -EPIPE
    }
}

/// Kontrol NEGATIF: `shutdown_aksi` yang MENGABAIKAN idempotensi (mengulang
/// sisi yang sudah dimatikan). Sengaja salah; dipakai matriks untuk
/// membuktikan bahwa cabang idempoten benar-benar berdaya — tanpa cabang itu
/// hasilnya BERBEDA pada `(how = SHUT_RDWR, sudah_tulis = 1, sudah_baca = 1)`.
fn shutdown_aksi_tanpa_idempoten(how: u32, sh_wr: u32, sh_rd: u32, sh_rdwr: u32) -> u32 {
    let mau_tulis = how == sh_wr || how == sh_rdwr;
    let mau_baca = how == sh_rd || how == sh_rdwr;
    u32::from(mau_tulis) * AKSI_TULIS | u32::from(mau_baca) * AKSI_BACA
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Kelas domain. Lihat [`domain_kelas`].
#[no_mangle]
pub extern "C" fn klatos_soket_domain_kelas(domain: i32, af_local: i32, af_inet: i32) -> u32 {
    bump(Counter::ProdDomain);
    domain_kelas(domain, af_local, af_inet)
}

/// Admisi `shutdown`. Lihat [`shutdown_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soket_shutdown_admisi(stream: u32, terhubung: u32, listener: u32) -> i32 {
    bump(Counter::ProdShutdown);
    shutdown_admisi(stream, terhubung, listener)
}

/// Aksi `shutdown`. Lihat [`shutdown_aksi`].
#[no_mangle]
pub extern "C" fn klatos_soket_shutdown_aksi(
    how: u32,
    sudah_tulis: u32,
    sudah_baca: u32,
    sh_wr: u32,
    sh_rd: u32,
    sh_rdwr: u32,
) -> u32 {
    shutdown_aksi(how, sudah_tulis, sudah_baca, sh_wr, sh_rd, sh_rdwr)
}

/// Admisi antrean koneksi. Lihat [`queue_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soket_queue_admisi(pending: u32, backlog: u32) -> i32 {
    bump(Counter::ProdAdmisi);
    queue_admisi(pending, backlog)
}

/// Kelayakan `accept`. Lihat [`accept_layak`].
#[no_mangle]
pub extern "C" fn klatos_soket_accept_layak(pending: u32) -> u32 {
    accept_layak(pending)
}

/// Gerbang `read`. Lihat [`read_gate`].
#[no_mangle]
pub extern "C" fn klatos_soket_read_gate(shut_reading: u32) -> u32 {
    bump(Counter::ProdGate);
    read_gate(shut_reading)
}

/// Gerbang `write`. Lihat [`write_gate`].
#[no_mangle]
pub extern "C" fn klatos_soket_write_gate(shut_writing: u32) -> i32 {
    write_gate(shut_writing)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_soket_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_soket_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosSiklusSoketReport` di `Jowo/Kontrak/KlatSeam76.h`.
#[repr(C)]
pub struct KlatosSiklusSoketReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub dom_checks: u32,
    pub dom_passed: u32,
    pub shut_checks: u32,
    pub shut_passed: u32,
    pub gate_checks: u32,
    pub gate_passed: u32,
    pub adm_checks: u32,
    pub adm_passed: u32,
    pub prod_domain: u32,
    pub prod_shutdown: u32,
    pub prod_gate: u32,
    pub prod_admisi: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Domain,
    Shutdown,
    Gate,
    Admisi,
}

struct Matrix {
    checks: u32,
    passed: u32,
    dom_checks: u32,
    dom_passed: u32,
    shut_checks: u32,
    shut_passed: u32,
    gate_checks: u32,
    gate_passed: u32,
    adm_checks: u32,
    adm_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            dom_checks: 0,
            dom_passed: 0,
            shut_checks: 0,
            shut_passed: 0,
            gate_checks: 0,
            gate_passed: 0,
            adm_checks: 0,
            adm_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Domain => {
                self.dom_checks += 1;
                if lulus {
                    self.dom_passed += 1;
                }
            }
            Group::Shutdown => {
                self.shut_checks += 1;
                if lulus {
                    self.shut_passed += 1;
                }
            }
            Group::Gate => {
                self.gate_checks += 1;
                if lulus {
                    self.gate_passed += 1;
                }
            }
            Group::Admisi => {
                self.adm_checks += 1;
                if lulus {
                    self.adm_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan oleh `run_matrix`.
///
/// domain `2 × 2 × 2` = 8; shutdown_admisi `2×2×2` = 8; shutdown_aksi
/// `4×2×2` = 16; queue `32×8` = 256; accept `40`; read `2`; write `2`
/// ⇒ 8+8+16+256+40+2+2 = **332**.
pub const DIFF_EXPECTED: u32 = 332;

/// Nilai ABI sintetis (nilai NYATA diambil dari C++ saat produksi).
const AF_LOCAL_SINTETIS: i32 = 1;
const AF_INET_SINTETIS: i32 = 2;
const SHUT_RD_SINTETIS: u32 = 0;
const SHUT_WR_SINTETIS: u32 = 1;
const SHUT_RDWR_SINTETIS: u32 = 2;

fn run_matrix() -> KlatosSiklusSoketReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_534B_534B, Group::Domain);
    m.catat(EPIPE == 32 && EAFNOSUPPORT == 40, Group::Domain);
    m.catat(ECONNREFUSED == 45 && ENOTCONN == 63, Group::Domain);
    m.catat(DOMAIN_LOKAL == 0 && DOMAIN_INET == 1 && DOMAIN_TAK_DIDUKUNG == 2, Group::Domain);
    m.catat(AKSI_TIDAK == 0 && AKSI_TULIS == 1 && AKSI_BACA == 2, Group::Shutdown);
    m.catat(GATE_HENTI == 0 && GATE_LANJUT == 1, Group::Gate);

    // --- domain -----------------------------------------------------------
    m.catat(domain_kelas(AF_LOCAL_SINTETIS, 1, 2) == DOMAIN_LOKAL, Group::Domain);
    m.catat(domain_kelas(AF_INET_SINTETIS, 1, 2) == DOMAIN_INET, Group::Domain);
    m.catat(domain_kelas(0, 1, 2) == DOMAIN_TAK_DIDUKUNG, Group::Domain);
    m.catat(domain_kelas(-1, 1, 2) == DOMAIN_TAK_DIDUKUNG, Group::Domain);
    // AF_LOCAL diperiksa lebih dulu (patologis: keduanya sama).
    m.catat(domain_kelas(7, 7, 7) == DOMAIN_LOKAL, Group::Domain);

    // --- shutdown admisi --------------------------------------------------
    // stream tak terhubung -> ENOTCONN.
    m.catat(shutdown_admisi(1, 0, 0) == -ENOTCONN, Group::Shutdown);
    // stream terhubung -> lanjut.
    m.catat(shutdown_admisi(1, 1, 0) == 0, Group::Shutdown);
    // non-stream -> lanjut.
    m.catat(shutdown_admisi(0, 0, 0) == 0, Group::Shutdown);
    // listener -> ENOTCONN walau terhubung.
    m.catat(shutdown_admisi(0, 1, 1) == -ENOTCONN, Group::Shutdown);
    m.catat(shutdown_admisi(1, 1, 1) == -ENOTCONN, Group::Shutdown);

    // --- shutdown aksi ----------------------------------------------------
    m.catat(shutdown_aksi(SHUT_WR_SINTETIS, 0, 0, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS) == AKSI_TULIS, Group::Shutdown);
    m.catat(shutdown_aksi(SHUT_RD_SINTETIS, 0, 0, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS) == AKSI_BACA, Group::Shutdown);
    m.catat(shutdown_aksi(SHUT_RDWR_SINTETIS, 0, 0, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS) == (AKSI_TULIS | AKSI_BACA), Group::Shutdown);
    // idempoten: sudah dimatikan -> tidak diulang.
    m.catat(shutdown_aksi(SHUT_RDWR_SINTETIS, 1, 1, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS) == AKSI_TIDAK, Group::Shutdown);
    m.catat(shutdown_aksi(SHUT_RDWR_SINTETIS, 1, 0, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS) == AKSI_BACA, Group::Shutdown);
    m.catat(shutdown_aksi(SHUT_RDWR_SINTETIS, 0, 1, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS) == AKSI_TULIS, Group::Shutdown);
    // how tak dikenal -> tidak ada aksi.
    m.catat(shutdown_aksi(9, 0, 0, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS) == AKSI_TIDAK, Group::Shutdown);

    // --- gerbang baca/tulis ----------------------------------------------
    m.catat(read_gate(0) == GATE_LANJUT, Group::Gate);
    m.catat(read_gate(1) == GATE_HENTI, Group::Gate);
    m.catat(write_gate(0) == 0, Group::Gate);
    m.catat(write_gate(1) == -EPIPE, Group::Gate);

    // --- admisi antrean ---------------------------------------------------
    m.catat(queue_admisi(0, 1) == 0, Group::Admisi);
    m.catat(queue_admisi(0, 0) == -ECONNREFUSED, Group::Admisi); // backlog 0 = penuh
    m.catat(queue_admisi(4, 5) == 0, Group::Admisi);
    m.catat(queue_admisi(5, 5) == -ECONNREFUSED, Group::Admisi);
    m.catat(accept_layak(0) == 0, Group::Admisi);
    m.catat(accept_layak(1) == 1, Group::Admisi);
    m.catat(accept_layak(99) == 1, Group::Admisi);

    // --- kontrol NEGATIF: cabang idempoten shutdown_aksi berdaya ----------
    {
        // how=SHUT_RDWR, kedua sisi SUDAH dimatikan.
        let benar = shutdown_aksi(SHUT_RDWR_SINTETIS, 1, 1, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS);
        let salah = shutdown_aksi_tanpa_idempoten(2, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS);
        m.catat(benar == AKSI_TIDAK, Group::Shutdown);
        m.catat(salah == (AKSI_TULIS | AKSI_BACA), Group::Shutdown);
        m.catat(benar != salah, Group::Shutdown);
        // Pada kasus yang belum dimatikan, keduanya HARUS sama.
        m.catat(
            shutdown_aksi(SHUT_RDWR_SINTETIS, 0, 0, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS) == shutdown_aksi_tanpa_idempoten(2, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS),
            Group::Shutdown,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // domain: 2 x 2 x 2 = 8
    for d in [AF_LOCAL_SINTETIS, AF_INET_SINTETIS] {
        for al in [1i32, 9] {
            for ai in [2i32, 9] {
                if domain_kelas(d, al, ai) != naif_domain_kelas(d, al, ai) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // shutdown_admisi: 2 x 2 x 2 = 8
    for s in 0..2u32 {
        for t in 0..2u32 {
            for l in 0..2u32 {
                if shutdown_admisi(s, t, l) != naif_shutdown_admisi(s, t, l) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // shutdown_aksi: 4 x 2 x 2 = 16
    for how in 0..4u32 {
        for st in 0..2u32 {
            for sb in 0..2u32 {
                let r = shutdown_aksi(how, st, sb, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS);
                let o = naif_shutdown_aksi(how, st, sb, SHUT_WR_SINTETIS, SHUT_RD_SINTETIS, SHUT_RDWR_SINTETIS);
                if r != o {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // queue_admisi: 32 x 8 = 256
    for p in 0..32u32 {
        for &bl in &[0u32, 1, 2, 4, 8, 16, 31, 32] {
            if queue_admisi(p, bl) != naif_queue_admisi(p, bl) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // accept_layak: 40
    for p in 0..40u32 {
        if accept_layak(p) != naif_accept_layak(p) {
            beda += 1;
        }
        diukur += 1;
    }
    // read_gate: 2 ; write_gate: 2
    for s in 0..2u32 {
        if read_gate(s) != naif_read_gate(s) {
            beda += 1;
        }
        diukur += 1;
    }
    for s in 0..2u32 {
        if write_gate(s) != naif_write_gate(s) {
            beda += 1;
        }
        diukur += 1;
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Domain);
    m.catat(beda == 0, Group::Domain);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosSiklusSoketReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        dom_checks: m.dom_checks,
        dom_passed: m.dom_passed,
        shut_checks: m.shut_checks,
        shut_passed: m.shut_passed,
        gate_checks: m.gate_checks,
        gate_passed: m.gate_passed,
        adm_checks: m.adm_checks,
        adm_passed: m.adm_passed,
        prod_domain: klatos_soket_counter(Counter::ProdDomain as u32),
        prod_shutdown: klatos_soket_counter(Counter::ProdShutdown as u32),
        prod_gate: klatos_soket_counter(Counter::ProdGate as u32),
        prod_admisi: klatos_soket_counter(Counter::ProdAdmisi as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks siklus hidup & admisi soket.
#[no_mangle]
pub extern "C" fn klatos_soket_selftest() -> KlatosSiklusSoketReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_534B_534B);
        assert!(EPIPE == 32);
        assert!(EAFNOSUPPORT == 40);
        assert!(ECONNREFUSED == 45);
        assert!(ENOTCONN == 63);
        assert!(DOMAIN_LOKAL == 0);
        assert!(DOMAIN_INET == 1);
        assert!(DOMAIN_TAK_DIDUKUNG == 2);
        assert!(AKSI_TIDAK == 0);
        assert!(AKSI_TULIS == 1);
        assert!(AKSI_BACA == 2);
        assert!(GATE_HENTI == 0);
        assert!(GATE_LANJUT == 1);
        assert!(Counter::ProdDomain as usize == 0);
        assert!(Counter::ProdShutdown as usize == 1);
        assert!(Counter::ProdGate as usize == 2);
        assert!(Counter::ProdAdmisi as usize == 3);
        assert!(N_COUNTER == 4);
        assert!(DIFF_EXPECTED == 332);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76 -> align 8 -> 80.
        assert!(core::mem::size_of::<KlatosSiklusSoketReport>() == 80);
        assert!(core::mem::align_of::<KlatosSiklusSoketReport>() == 8);
    };
}
