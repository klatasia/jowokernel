// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.78 / WS6 — **OTORITAS ADMISI PANGGILAN SISTEM SOKET**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EACCES` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 13).
pub const EACCES: i32 = 13;
/// `EINVAL` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 22).
pub const EINVAL: i32 = 22;
/// `ENOTSUP` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 74).
pub const ENOTSUP: i32 = 74;

/// Tag laporan; cermin `KlatosPanggilanSoketReport` di `Jowo/Kontrak/KlatSeam78.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_5350_4B53; // "KLATSPKS"

/// Aksi `accept4` — bitmask.
pub const AKSI_TIDAK: u32 = 0;
/// Setel deskriptor non-blocking (`SOCK_NONBLOCK`).
pub const AKSI_NONBLOCK: u32 = 1;
/// Setel `FD_CLOEXEC` (`SOCK_CLOEXEC`).
pub const AKSI_CLOEXEC: u32 = 2;

/// Penghitung audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_soketp_raw` dari produksi (setiap pembuatan soket).
    ProdBuat = 0,
    /// `klatos_soketp_listen_backlog` + `_listen_admisi` dari produksi.
    ProdListen = 1,
    /// `klatos_soketp_accept4` + `_sendmsg_iov` + `_recvmsg_iov` + `_nama_ukuran`.
    ProdTerima = 2,
    /// `klatos_soketp_shutdown` + `_socketpair` dari produksi.
    ProdLain = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Hak soket mentah. Transkripsi `sys$socket:46-47`:
///
/// ```cpp
/// if ((type & SOCK_TYPE_MASK) == SOCK_RAW && !credentials->is_superuser())
///     return EACCES;
/// ```
///
/// `mask`/`sock_raw` DISUPLAI C++ (`SOCK_TYPE_MASK`/`SOCK_RAW` kontrak ABI).
/// Mengembalikan `0` (lanjut) atau `-EACCES`.
#[inline]
pub const fn raw_admisi(tipe: u32, mask: u32, sock_raw: u32, superuser: u32) -> i32 {
    if (tipe & mask) == sock_raw && superuser == 0 {
        return -EACCES;
    }
    0
}

/// Normalisasi `backlog`. Transkripsi `sys$listen:81-82`:
///
/// ```cpp
/// if (backlog <= 0) backlog = 1;
/// ```
///
/// POSIX: backlog 0/negatif berperilaku seperti minimum implementasi. KlatOS
/// menormalkannya ke 1. Mengembalikan backlog ternormalisasi (`>= 1`).
#[inline]
pub const fn listen_backlog(backlog: i32) -> u32 {
    if backlog <= 0 {
        1
    } else {
        backlog as u32
    }
}

/// Admisi `listen`. Transkripsi `sys$listen:88-89`:
///
/// ```cpp
/// if (socket.is_connected()) return EINVAL;
/// ```
///
/// Mengembalikan `0` (lanjut) atau `-EINVAL`.
#[inline]
pub const fn listen_admisi(terhubung: u32) -> i32 {
    if terhubung != 0 {
        return -EINVAL;
    }
    0
}

/// Admisi `how` pada `shutdown`. Transkripsi `sys$shutdown:176-177`:
///
/// ```cpp
/// if (how != SHUT_RD && how != SHUT_WR && how != SHUT_RDWR) return EINVAL;
/// ```
///
/// `shut_rd`/`shut_wr`/`shut_rdwr` DISUPLAI C++ (kontrak ABI).
/// Mengembalikan `0` (lanjut) atau `-EINVAL`.
#[inline]
pub const fn shutdown_admisi(how: i32, shut_rd: i32, shut_wr: i32, shut_rdwr: i32) -> i32 {
    if how != shut_rd && how != shut_wr && how != shut_rdwr {
        return -EINVAL;
    }
    0
}

/// Penerapan flag `accept4`. Transkripsi `sys$accept4:143-147`:
///
/// ```cpp
/// if (flags & SOCK_NONBLOCK) accepted_socket_description->set_blocking(false);
/// int fd_flags = 0;
/// if (flags & SOCK_CLOEXEC) fd_flags |= FD_CLOEXEC;
/// ```
///
/// `sock_nonblock`/`sock_cloexec` DISUPLAI C++ (kontrak ABI). Mengembalikan
/// bitmask [`AKSI_NONBLOCK`] | [`AKSI_CLOEXEC`].
#[inline]
pub const fn accept4_terapkan(flags: u32, sock_nonblock: u32, sock_cloexec: u32) -> u32 {
    let mut aksi = AKSI_TIDAK;
    if (flags & sock_nonblock) != 0 {
        aksi |= AKSI_NONBLOCK;
    }
    if (flags & sock_cloexec) != 0 {
        aksi |= AKSI_CLOEXEC;
    }
    aksi
}

/// Batas `iovlen` pada `sendmsg`. Transkripsi `sys$sendmsg:215-216`:
///
/// ```cpp
/// if (msg.msg_iovlen > IOV_MAX) return EINVAL;
/// ```
///
/// `iov_max` DISUPLAI C++ (kontrak ABI). Mengembalikan `0` (lanjut) atau `-EINVAL`.
#[inline]
pub const fn sendmsg_iov_admisi(iovlen: u32, iov_max: u32) -> i32 {
    if iovlen > iov_max {
        return -EINVAL;
    }
    0
}

/// Bentuk `iovlen` pada `recvmsg`. Transkripsi `sys$recvmsg:326-327`:
///
/// ```cpp
/// if (msg.msg_iovlen != 1) return ENOTSUP; // FIXME: Support this :)
/// ```
///
/// Penerima Mozilla hanya memakai `msg_iovlen == 1`, jadi bentuk lain ditolak
/// `ENOTSUP` (BUKAN `EINVAL`). Mengembalikan `0` (lanjut) atau `-ENOTSUP`.
#[inline]
pub const fn recvmsg_iov_admisi(iovlen: u32) -> i32 {
    if iovlen != 1 {
        return -ENOTSUP;
    }
    0
}

/// Ukuran alamat nama pada `getsockname`/`getpeername`. Transkripsi
/// `get_sock_or_peer_name:403-404`:
///
/// ```cpp
/// if (addrlen_value <= 0) return EINVAL;
/// ```
///
/// Mengembalikan `0` (lanjut) atau `-EINVAL`.
#[inline]
pub const fn nama_ukuran_admisi(addrlen: i32) -> i32 {
    if addrlen <= 0 {
        return -EINVAL;
    }
    0
}

/// Admisi domain/protokol `socketpair`. Transkripsi `sys$socketpair:482-486`:
///
/// ```cpp
/// if (params.domain != AF_LOCAL) return EINVAL;
/// if (params.protocol != 0 && params.protocol != PF_LOCAL) return EINVAL;
/// ```
///
/// `af_local`/`pf_local` DISUPLAI C++ (kontrak ABI). Mengembalikan `0`
/// (lanjut) atau `-EINVAL`.
#[inline]
pub const fn socketpair_admisi(domain: i32, protokol: i32, af_local: i32, pf_local: i32) -> i32 {
    if domain != af_local {
        return -EINVAL;
    }
    if protokol != 0 && protokol != pf_local {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_raw_admisi(tipe: u32, mask: u32, sock_raw: u32, superuser: u32) -> i32 {
    let mentah = (tipe & mask) == sock_raw;
    if mentah && superuser == 0 {
        -EACCES
    } else {
        0
    }
}

fn naif_listen_backlog(backlog: i32) -> u32 {
    if backlog > 0 {
        backlog as u32
    } else {
        1
    }
}

fn naif_listen_admisi(terhubung: u32) -> i32 {
    if terhubung == 0 {
        0
    } else {
        -EINVAL
    }
}

fn naif_shutdown_admisi(how: i32, shut_rd: i32, shut_wr: i32, shut_rdwr: i32) -> i32 {
    let dikenal = how == shut_rd || how == shut_wr || how == shut_rdwr;
    if dikenal {
        0
    } else {
        -EINVAL
    }
}

fn naif_accept4_terapkan(flags: u32, sock_nonblock: u32, sock_cloexec: u32) -> u32 {
    u32::from((flags & sock_nonblock) != 0) * AKSI_NONBLOCK
        | u32::from((flags & sock_cloexec) != 0) * AKSI_CLOEXEC
}

fn naif_sendmsg_iov_admisi(iovlen: u32, iov_max: u32) -> i32 {
    if iovlen <= iov_max {
        0
    } else {
        -EINVAL
    }
}

fn naif_recvmsg_iov_admisi(iovlen: u32) -> i32 {
    if iovlen == 1 {
        0
    } else {
        -ENOTSUP
    }
}

fn naif_nama_ukuran_admisi(addrlen: i32) -> i32 {
    if addrlen > 0 {
        0
    } else {
        -EINVAL
    }
}

fn naif_socketpair_admisi(domain: i32, protokol: i32, af_local: i32, pf_local: i32) -> i32 {
    let domain_ok = domain == af_local;
    let protokol_ok = protokol == 0 || protokol == pf_local;
    if domain_ok && protokol_ok {
        0
    } else {
        -EINVAL
    }
}

/// Kontrol NEGATIF: `sendmsg_iov_admisi` yang memakai `>=` alih-alih `>`.
/// Sengaja salah; berdaya pada `iovlen == iov_max` — produksi `0`, yang salah
/// `-EINVAL`.
fn sendmsg_iov_admisi_salah_batas(iovlen: u32, iov_max: u32) -> i32 {
    if iovlen >= iov_max {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Hak soket mentah. Lihat [`raw_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soketp_raw(tipe: u32, mask: u32, sock_raw: u32, superuser: u32) -> i32 {
    bump(Counter::ProdBuat);
    raw_admisi(tipe, mask, sock_raw, superuser)
}

/// Normalisasi `backlog`. Lihat [`listen_backlog`].
#[no_mangle]
pub extern "C" fn klatos_soketp_listen_backlog(backlog: i32) -> u32 {
    bump(Counter::ProdListen);
    listen_backlog(backlog)
}

/// Admisi `listen`. Lihat [`listen_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soketp_listen_admisi(terhubung: u32) -> i32 {
    listen_admisi(terhubung)
}

/// Admisi `shutdown`. Lihat [`shutdown_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soketp_shutdown(how: i32, shut_rd: i32, shut_wr: i32, shut_rdwr: i32) -> i32 {
    bump(Counter::ProdLain);
    shutdown_admisi(how, shut_rd, shut_wr, shut_rdwr)
}

/// Penerapan flag `accept4`. Lihat [`accept4_terapkan`].
#[no_mangle]
pub extern "C" fn klatos_soketp_accept4(flags: u32, sock_nonblock: u32, sock_cloexec: u32) -> u32 {
    bump(Counter::ProdTerima);
    accept4_terapkan(flags, sock_nonblock, sock_cloexec)
}

/// Batas `iovlen` `sendmsg`. Lihat [`sendmsg_iov_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soketp_sendmsg_iov(iovlen: u32, iov_max: u32) -> i32 {
    bump(Counter::ProdTerima);
    sendmsg_iov_admisi(iovlen, iov_max)
}

/// Bentuk `iovlen` `recvmsg`. Lihat [`recvmsg_iov_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soketp_recvmsg_iov(iovlen: u32) -> i32 {
    bump(Counter::ProdTerima);
    recvmsg_iov_admisi(iovlen)
}

/// Ukuran alamat nama. Lihat [`nama_ukuran_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soketp_nama_ukuran(addrlen: i32) -> i32 {
    bump(Counter::ProdTerima);
    nama_ukuran_admisi(addrlen)
}

/// Admisi `socketpair`. Lihat [`socketpair_admisi`].
#[no_mangle]
pub extern "C" fn klatos_soketp_socketpair(domain: i32, protokol: i32, af_local: i32, pf_local: i32) -> i32 {
    bump(Counter::ProdLain);
    socketpair_admisi(domain, protokol, af_local, pf_local)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_soketp_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_soketp_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosPanggilanSoketReport` di `Jowo/Kontrak/KlatSeam78.h`.
#[repr(C)]
pub struct KlatosPanggilanSoketReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub buat_checks: u32,
    pub buat_passed: u32,
    pub listen_checks: u32,
    pub listen_passed: u32,
    pub terima_checks: u32,
    pub terima_passed: u32,
    pub lain_checks: u32,
    pub lain_passed: u32,
    pub prod_buat: u32,
    pub prod_listen: u32,
    pub prod_terima: u32,
    pub prod_lain: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Buat,
    Listen,
    Terima,
    Lain,
}

struct Matrix {
    checks: u32,
    passed: u32,
    buat_checks: u32,
    buat_passed: u32,
    listen_checks: u32,
    listen_passed: u32,
    terima_checks: u32,
    terima_passed: u32,
    lain_checks: u32,
    lain_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            buat_checks: 0,
            buat_passed: 0,
            listen_checks: 0,
            listen_passed: 0,
            terima_checks: 0,
            terima_passed: 0,
            lain_checks: 0,
            lain_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Buat => {
                self.buat_checks += 1;
                if lulus {
                    self.buat_passed += 1;
                }
            }
            Group::Listen => {
                self.listen_checks += 1;
                if lulus {
                    self.listen_passed += 1;
                }
            }
            Group::Terima => {
                self.terima_checks += 1;
                if lulus {
                    self.terima_passed += 1;
                }
            }
            Group::Lain => {
                self.lain_checks += 1;
                if lulus {
                    self.lain_passed += 1;
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
/// raw `4 × 2 × 2` = 16; listen_backlog `6`; listen_admisi `2`; shutdown `5`;
/// accept4 `4`; sendmsg_iov `5 × 2` = 10; recvmsg_iov `4`; nama_ukuran `4`;
/// socketpair `2 × 3` = 6 ⇒ 16+6+2+5+4+10+4+4+6 = **57**.
pub const DIFF_EXPECTED: u32 = 57;

/// Nilai ABI sintetis (nilai NYATA diambil dari C++ saat produksi).
const MASK_SINTETIS: u32 = 0xF;
const RAW_SINTETIS: u32 = 3;
const IOV_MAX_SINTETIS: u32 = 4;
const SHUT_RD_SINTETIS: i32 = 0;
const SHUT_WR_SINTETIS: i32 = 1;
const SHUT_RDWR_SINTETIS: i32 = 2;
const AF_LOCAL_SINTETIS: i32 = 1;
const PF_LOCAL_SINTETIS: i32 = 1;
const SOCK_NONBLOCK_SINTETIS: u32 = 0x800;
const SOCK_CLOEXEC_SINTETIS: u32 = 0x400;

fn run_matrix() -> KlatosPanggilanSoketReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_5350_4B53, Group::Buat);
    m.catat(EACCES == 13 && EINVAL == 22 && ENOTSUP == 74, Group::Buat);
    m.catat(AKSI_TIDAK == 0 && AKSI_NONBLOCK == 1 && AKSI_CLOEXEC == 2, Group::Terima);

    // --- soket mentah -----------------------------------------------------
    m.catat(raw_admisi(RAW_SINTETIS, MASK_SINTETIS, RAW_SINTETIS, 1) == 0, Group::Buat);
    m.catat(raw_admisi(RAW_SINTETIS, MASK_SINTETIS, RAW_SINTETIS, 0) == -EACCES, Group::Buat);
    m.catat(raw_admisi(1, MASK_SINTETIS, RAW_SINTETIS, 0) == 0, Group::Buat);
    // bit di luar mask diabaikan: (0x13 & 0xF) == 3 -> tetap mentah.
    m.catat(raw_admisi(0x13, MASK_SINTETIS, RAW_SINTETIS, 0) == -EACCES, Group::Buat);

    // --- backlog ----------------------------------------------------------
    m.catat(listen_backlog(-5) == 1, Group::Listen);
    m.catat(listen_backlog(0) == 1, Group::Listen);
    m.catat(listen_backlog(1) == 1, Group::Listen);
    m.catat(listen_backlog(7) == 7, Group::Listen);
    m.catat(listen_admisi(0) == 0, Group::Listen);
    m.catat(listen_admisi(1) == -EINVAL, Group::Listen);

    // --- shutdown ---------------------------------------------------------
    m.catat(shutdown_admisi(SHUT_RD_SINTETIS, SHUT_RD_SINTETIS, SHUT_WR_SINTETIS, SHUT_RDWR_SINTETIS) == 0, Group::Lain);
    m.catat(shutdown_admisi(SHUT_RDWR_SINTETIS, SHUT_RD_SINTETIS, SHUT_WR_SINTETIS, SHUT_RDWR_SINTETIS) == 0, Group::Lain);
    m.catat(shutdown_admisi(9, SHUT_RD_SINTETIS, SHUT_WR_SINTETIS, SHUT_RDWR_SINTETIS) == -EINVAL, Group::Lain);

    // --- accept4 ----------------------------------------------------------
    m.catat(accept4_terapkan(0, SOCK_NONBLOCK_SINTETIS, SOCK_CLOEXEC_SINTETIS) == AKSI_TIDAK, Group::Terima);
    m.catat(accept4_terapkan(SOCK_NONBLOCK_SINTETIS, SOCK_NONBLOCK_SINTETIS, SOCK_CLOEXEC_SINTETIS) == AKSI_NONBLOCK, Group::Terima);
    m.catat(accept4_terapkan(SOCK_CLOEXEC_SINTETIS, SOCK_NONBLOCK_SINTETIS, SOCK_CLOEXEC_SINTETIS) == AKSI_CLOEXEC, Group::Terima);
    m.catat(accept4_terapkan(SOCK_NONBLOCK_SINTETIS | SOCK_CLOEXEC_SINTETIS, SOCK_NONBLOCK_SINTETIS, SOCK_CLOEXEC_SINTETIS) == (AKSI_NONBLOCK | AKSI_CLOEXEC), Group::Terima);

    // --- iov --------------------------------------------------------------
    m.catat(sendmsg_iov_admisi(0, IOV_MAX_SINTETIS) == 0, Group::Terima);
    m.catat(sendmsg_iov_admisi(IOV_MAX_SINTETIS, IOV_MAX_SINTETIS) == 0, Group::Terima); // batas = OK
    m.catat(sendmsg_iov_admisi(IOV_MAX_SINTETIS + 1, IOV_MAX_SINTETIS) == -EINVAL, Group::Terima);
    m.catat(recvmsg_iov_admisi(1) == 0, Group::Terima);
    m.catat(recvmsg_iov_admisi(0) == -ENOTSUP, Group::Terima);
    m.catat(recvmsg_iov_admisi(2) == -ENOTSUP, Group::Terima);

    // --- nama -------------------------------------------------------------
    m.catat(nama_ukuran_admisi(1) == 0, Group::Terima);
    m.catat(nama_ukuran_admisi(0) == -EINVAL, Group::Terima);
    m.catat(nama_ukuran_admisi(-1) == -EINVAL, Group::Terima);

    // --- socketpair -------------------------------------------------------
    m.catat(socketpair_admisi(AF_LOCAL_SINTETIS, 0, AF_LOCAL_SINTETIS, PF_LOCAL_SINTETIS) == 0, Group::Lain);
    m.catat(socketpair_admisi(AF_LOCAL_SINTETIS, PF_LOCAL_SINTETIS, AF_LOCAL_SINTETIS, PF_LOCAL_SINTETIS) == 0, Group::Lain);
    m.catat(socketpair_admisi(2, 0, AF_LOCAL_SINTETIS, PF_LOCAL_SINTETIS) == -EINVAL, Group::Lain);
    m.catat(socketpair_admisi(AF_LOCAL_SINTETIS, 3, AF_LOCAL_SINTETIS, PF_LOCAL_SINTETIS) == -EINVAL, Group::Lain);

    // --- kontrol NEGATIF: batas sendmsg berdaya ---------------------------
    {
        let benar = sendmsg_iov_admisi(IOV_MAX_SINTETIS, IOV_MAX_SINTETIS);
        let salah = sendmsg_iov_admisi_salah_batas(IOV_MAX_SINTETIS, IOV_MAX_SINTETIS);
        m.catat(benar == 0, Group::Terima);
        m.catat(salah == -EINVAL, Group::Terima);
        m.catat(benar != salah, Group::Terima);
        // Di bawah batas, keduanya HARUS sama.
        m.catat(
            sendmsg_iov_admisi(1, IOV_MAX_SINTETIS) == sendmsg_iov_admisi_salah_batas(1, IOV_MAX_SINTETIS),
            Group::Terima,
        );
    }

    // --- diferensial produksi vs oracle ----------------------------------
    // raw: 4 x 2 x 2 = 16
    for tipe in 0..4u32 {
        for su in 0..2u32 {
            for &raw in &[RAW_SINTETIS, 4u32] {
                if raw_admisi(tipe, MASK_SINTETIS, raw, su) != naif_raw_admisi(tipe, MASK_SINTETIS, raw, su) {
                    beda += 1;
                }
                diukur += 1;
            }
        }
    }
    // listen_backlog: 6
    for &b in &[-3i32, -1, 0, 1, 2, 100] {
        if listen_backlog(b) != naif_listen_backlog(b) {
            beda += 1;
        }
        diukur += 1;
    }
    // listen_admisi: 2
    for t in 0..2u32 {
        if listen_admisi(t) != naif_listen_admisi(t) {
            beda += 1;
        }
        diukur += 1;
    }
    // shutdown: 5
    for how in 0..5i32 {
        if shutdown_admisi(how, SHUT_RD_SINTETIS, SHUT_WR_SINTETIS, SHUT_RDWR_SINTETIS)
            != naif_shutdown_admisi(how, SHUT_RD_SINTETIS, SHUT_WR_SINTETIS, SHUT_RDWR_SINTETIS)
        {
            beda += 1;
        }
        diukur += 1;
    }
    // accept4: 4
    for f in 0..4u32 {
        if accept4_terapkan(f, SOCK_NONBLOCK_SINTETIS, SOCK_CLOEXEC_SINTETIS)
            != naif_accept4_terapkan(f, SOCK_NONBLOCK_SINTETIS, SOCK_CLOEXEC_SINTETIS)
        {
            beda += 1;
        }
        diukur += 1;
    }
    // sendmsg_iov: 5 x 2 = 10
    for n in 0..5u32 {
        for &mx in &[2u32, IOV_MAX_SINTETIS] {
            if sendmsg_iov_admisi(n, mx) != naif_sendmsg_iov_admisi(n, mx) {
                beda += 1;
            }
            diukur += 1;
        }
    }
    // recvmsg_iov: 4
    for n in 0..4u32 {
        if recvmsg_iov_admisi(n) != naif_recvmsg_iov_admisi(n) {
            beda += 1;
        }
        diukur += 1;
    }
    // nama_ukuran: 4
    for &a in &[-1i32, 0, 1, 5] {
        if nama_ukuran_admisi(a) != naif_nama_ukuran_admisi(a) {
            beda += 1;
        }
        diukur += 1;
    }
    // socketpair: 2 x 3 = 6
    for d in 0..2i32 {
        for p in 0..3i32 {
            if socketpair_admisi(d, p, AF_LOCAL_SINTETIS, PF_LOCAL_SINTETIS)
                != naif_socketpair_admisi(d, p, AF_LOCAL_SINTETIS, PF_LOCAL_SINTETIS)
            {
                beda += 1;
            }
            diukur += 1;
        }
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Buat);
    m.catat(beda == 0, Group::Buat);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosPanggilanSoketReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        buat_checks: m.buat_checks,
        buat_passed: m.buat_passed,
        listen_checks: m.listen_checks,
        listen_passed: m.listen_passed,
        terima_checks: m.terima_checks,
        terima_passed: m.terima_passed,
        lain_checks: m.lain_checks,
        lain_passed: m.lain_passed,
        prod_buat: klatos_soketp_counter(Counter::ProdBuat as u32),
        prod_listen: klatos_soketp_counter(Counter::ProdListen as u32),
        prod_terima: klatos_soketp_counter(Counter::ProdTerima as u32),
        prod_lain: klatos_soketp_counter(Counter::ProdLain as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks admisi panggilan sistem soket.
#[no_mangle]
pub extern "C" fn klatos_soketp_selftest() -> KlatosPanggilanSoketReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_5350_4B53);
        assert!(EACCES == 13);
        assert!(EINVAL == 22);
        assert!(ENOTSUP == 74);
        assert!(AKSI_TIDAK == 0);
        assert!(AKSI_NONBLOCK == 1);
        assert!(AKSI_CLOEXEC == 2);
        assert!(Counter::ProdBuat as usize == 0);
        assert!(Counter::ProdListen as usize == 1);
        assert!(Counter::ProdTerima as usize == 2);
        assert!(Counter::ProdLain as usize == 3);
        assert!(N_COUNTER == 4);
        assert!(DIFF_EXPECTED == 57);

        // 8 (tag) + 16*4 (u32) + 4 (status) = 76 -> align 8 -> 80.
        assert!(core::mem::size_of::<KlatosPanggilanSoketReport>() == 80);
        assert!(core::mem::align_of::<KlatosPanggilanSoketReport>() == 8);
    };
}
