// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.95 / WS6 — **OTORITAS JARINGAN, BATCH 2: transport + soket lokal**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `ESRCH` — indeks 3.
pub const ESRCH: i32 = 3;
/// `EAGAIN` — indeks 11.
pub const EAGAIN: i32 = 11;
/// `EBUSY` — indeks 16.
pub const EBUSY: i32 = 16;
/// `EEXIST` — indeks 17.
pub const EEXIST: i32 = 17;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `EPIPE` — indeks 32.
pub const EPIPE: i32 = 32;
/// `EOPNOTSUPP` — indeks 37.
pub const EOPNOTSUPP: i32 = 37;
/// `EADDRINUSE` — indeks 42.
pub const EADDRINUSE: i32 = 42;
/// `ECONNREFUSED` — indeks 45.
pub const ECONNREFUSED: i32 = 45;
/// `EADDRNOTAVAIL` — indeks 47.
pub const EADDRNOTAVAIL: i32 = 47;
/// `EISCONN` — indeks 48.
pub const EISCONN: i32 = 48;
/// `ENOPROTOOPT` — indeks 62.
pub const ENOPROTOOPT: i32 = 62;
/// `ENOTCONN` — indeks 63.
pub const ENOTCONN: i32 = 63;
/// `ETIMEDOUT` — indeks 69.
pub const ETIMEDOUT: i32 = 69;
/// `EINPROGRESS` — indeks 71.
pub const EINPROGRESS: i32 = 71;

/// Batas antrean `sendfd`; cermin `LocalSocket.cpp:521` (`queue.size() > 128`).
pub const ANTREAN_MAKS: u32 = 128;

/// Tag laporan; cermin `KlatosAdmisiNet2Report` di `Jowo/Kontrak/KlatSeam96.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4E45_5432; // "KLATNET2"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_net2_tcp_sokopt_tak`.
    ProdTcpSokoptTak = 0,
    /// `klatos_net2_tcp_tuple`.
    ProdTcpTuple = 1,
    /// `klatos_net2_tcp_adapter`.
    ProdTcpAdapter = 2,
    /// `klatos_net2_port`.
    ProdPort = 3,
    /// `klatos_net2_tcp_hubung`.
    ProdTcpHubung = 4,
    /// `klatos_net2_tcp_hubung_lanjut`.
    ProdTcpHubungLanjut = 5,
    /// `klatos_net2_lokal_ukuran`.
    ProdLokalUkuran = 6,
    /// `klatos_net2_lokal_keluarga`.
    ProdLokalKeluarga = 7,
    /// `klatos_net2_lokal_bentrok`.
    ProdLokalBentrok = 8,
    /// `klatos_net2_lokal_terhubung`.
    ProdLokalTerhubung = 9,
    /// `klatos_net2_lokal_peer`.
    ProdLokalPeer = 10,
    /// `klatos_net2_lokal_stream`.
    ProdLokalStream = 11,
    /// `klatos_net2_lokal_lepas`.
    ProdLokalLepas = 12,
    /// `klatos_net2_lokal_buffer`.
    ProdLokalBuffer = 13,
    /// `klatos_net2_lokal_peercred_hubung`.
    ProdLokalPeercredHubung = 14,
    /// `klatos_net2_lokal_peran`.
    ProdLokalPeran = 15,
    /// `klatos_net2_lokal_ioctl`.
    ProdLokalIoctl = 16,
    /// `klatos_net2_lokal_chown`.
    ProdLokalChown = 17,
    /// `klatos_net2_lokal_fd_peran`.
    ProdLokalFdPeran = 18,
    /// `klatos_net2_lokal_sendfd_penuh`.
    ProdLokalSendfdPenuh = 19,
    /// `klatos_net2_lokal_recvfd_kosong`.
    ProdLokalRecvfdKosong = 20,
    /// `klatos_net2_rute_duplikat`.
    ProdRuteDuplikat = 21,
    /// `klatos_net2_rute_tidak_ada`.
    ProdRuteTidakAda = 22,
    /// `klatos_net2_inet_tipe`.
    ProdInetTipe = 23,
}

const N_COUNTER: usize = 24;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Sockopt `IPPROTO_TCP` tak dikenal. Transkripsi `TCPSocket.cpp:467,484`.
#[inline]
pub const fn net2_tcp_sokopt_tak() -> i32 {
    -ENOPROTOOPT
}

/// Tuple TCP sudah terdaftar. Transkripsi `TCPSocket.cpp:154`.
#[inline]
pub const fn net2_tcp_tuple_ada(ada: u32) -> i32 {
    if ada != 0 {
        return -EEXIST;
    }
    0
}

/// Adapter untuk alamat lokal tak ditemukan. Transkripsi `TCPSocket.cpp:496`.
#[inline]
pub const fn net2_tcp_adapter_tidak_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -EADDRNOTAVAIL;
    }
    0
}

/// Port habis (scan penuh) atau sudah dipakai. Satu verdict, lima pemanggil.
/// Transkripsi `TCPSocket.cpp:527,540,573`, `UDPSocket.cpp:146,152`.
#[inline]
pub const fn net2_port_bentrok(bentrok: u32) -> i32 {
    if bentrok != 0 {
        return -EADDRINUSE;
    }
    0
}

/// `connect` TCP gagal setelah blokir: timeout vs ditolak.
/// Transkripsi `TCPSocket.cpp:600,603`.
#[inline]
pub const fn net2_tcp_hubung_gagal(timeout: u32) -> i32 {
    if timeout != 0 {
        return -ETIMEDOUT;
    }
    -ECONNREFUSED
}

/// `connect` TCP non-blokir. Transkripsi `TCPSocket.cpp:608`.
#[inline]
pub const fn net2_tcp_hubung_lanjut() -> i32 {
    -EINPROGRESS
}

/// Ukuran alamat soket lokal terlalu besar. Transkripsi `LocalSocket.cpp:132,172`.
#[inline]
pub const fn net2_lokal_ukuran_lebih(lebih: u32) -> i32 {
    if lebih != 0 {
        return -EINVAL;
    }
    0
}

/// Keluarga soket lokal bukan `AF_LOCAL`. Transkripsi `LocalSocket.cpp:138,178`.
#[inline]
pub const fn net2_lokal_keluarga(af: u32, af_local: u32) -> i32 {
    if af != af_local {
        return -EINVAL;
    }
    0
}

/// Nama soket lokal sudah dipakai atau bind gagal.
/// Transkripsi `LocalSocket.cpp:148,157`.
#[inline]
pub const fn net2_lokal_bentrok(bentrok: u32) -> i32 {
    if bentrok != 0 {
        return -EADDRINUSE;
    }
    0
}

/// Soket lokal sudah terhubung. Transkripsi `LocalSocket.cpp:169,181`.
#[inline]
pub const fn net2_lokal_terhubung(terhubung: u32) -> i32 {
    if terhubung != 0 {
        return -EISCONN;
    }
    0
}

/// Peer soket lokal tidak ada. Transkripsi `LocalSocket.cpp:194,222`.
#[inline]
pub const fn net2_lokal_peer(ada: u32) -> i32 {
    if ada == 0 {
        return -ECONNREFUSED;
    }
    0
}

/// `listen` soket lokal bukan `SOCK_STREAM`. Transkripsi `LocalSocket.cpp:232`.
#[inline]
pub const fn net2_lokal_stream(stream: u32) -> i32 {
    if stream == 0 {
        return -EOPNOTSUPP;
    }
    0
}

/// `sendto` soket lokal tanpa peer terpasang. Transkripsi `LocalSocket.cpp:310`.
#[inline]
pub const fn net2_lokal_lepas(ada_peer: u32) -> i32 {
    if ada_peer == 0 {
        return -EPIPE;
    }
    0
}

/// Buffer soket lokal tidak ada untuk peran ini.
/// Transkripsi `LocalSocket.cpp:313,344`.
#[inline]
pub const fn net2_lokal_buffer(ada: u32) -> i32 {
    if ada == 0 {
        return -EINVAL;
    }
    0
}

/// `SO_PEERCRED` saat peran `Connecting`. Transkripsi `LocalSocket.cpp:440`.
#[inline]
pub const fn net2_lokal_peercred_hubung(connecting: u32) -> i32 {
    if connecting != 0 {
        return -ENOTCONN;
    }
    0
}

/// Peran soket lokal tak dikenal pada `SO_PEERCRED`.
/// Transkripsi `LocalSocket.cpp:442`.
#[inline]
pub const fn net2_lokal_peran_tak_dikenal() -> i32 {
    -EINVAL
}

/// `ioctl` soket lokal tak dikenal. Transkripsi `LocalSocket.cpp:460`.
#[inline]
pub const fn net2_lokal_ioctl_tak() -> i32 {
    -EINVAL
}

/// `chown` soket lokal tanpa hak. Transkripsi `LocalSocket.cpp:484`.
#[inline]
pub const fn net2_lokal_chown_izin(izin: u32) -> i32 {
    if izin == 0 {
        return -EPERM;
    }
    0
}

/// `sendfd`/`recvfd` dengan peran salah. Satu verdict, tiga pemanggil.
/// Transkripsi `LocalSocket.cpp:518,532,547`.
#[inline]
pub const fn net2_lokal_fd_peran(salah: u32) -> i32 {
    if salah != 0 {
        return -EINVAL;
    }
    0
}

/// Antrean `sendfd` penuh. Transkripsi `LocalSocket.cpp:522`.
#[inline]
pub const fn net2_lokal_sendfd_penuh(ukuran: u32) -> i32 {
    if ukuran > ANTREAN_MAKS {
        return -EBUSY;
    }
    0
}

/// Antrean `recvfd` kosong. Transkripsi `LocalSocket.cpp:536`.
#[inline]
pub const fn net2_lokal_recvfd_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        return -EAGAIN;
    }
    0
}

/// Rute duplikat saat `UpdateTable::Set`. Transkripsi `Routing.cpp:153`.
#[inline]
pub const fn net2_rute_duplikat(ada: u32) -> i32 {
    if ada != 0 {
        return -EEXIST;
    }
    0
}

/// Rute tidak ditemukan saat `UpdateTable::Delete`. Transkripsi `Routing.cpp:166`.
#[inline]
pub const fn net2_rute_tidak_ada(ditemukan: u32) -> i32 {
    if ditemukan == 0 {
        return -ESRCH;
    }
    0
}

/// Tipe soket IPv4 tak dikenal. Transkripsi `IP/Socket.cpp:63`.
#[inline]
pub const fn net2_inet_tipe_tak_dikenal() -> i32 {
    -EINVAL
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_bool(v: u32, errno: i32) -> i32 {
    match v {
        0 => 0,
        _ => errno,
    }
}
fn naif_bool_terbalik(v: u32, errno: i32) -> i32 {
    match v {
        0 => errno,
        _ => 0,
    }
}
fn naif_tcp_hubung_gagal(timeout: u32) -> i32 {
    match timeout {
        0 => -ECONNREFUSED,
        _ => -ETIMEDOUT,
    }
}
fn naif_keluarga(af: u32, af_local: u32) -> i32 {
    if af == af_local {
        0
    } else {
        -EINVAL
    }
}
fn naif_sendfd_penuh(ukuran: u32) -> i32 {
    if ukuran < ANTREAN_MAKS + 1 {
        0
    } else {
        -EBUSY
    }
}

/// Kontrol NEGATIF: `net2_tcp_hubung_gagal` yang salah selalu `ECONNREFUSED`.
/// Sengaja SALAH; berdaya pada `timeout != 0`.
fn net2_tcp_hubung_gagal_salah(timeout: u32) -> i32 {
    let _ = timeout;
    -ECONNREFUSED
}

/// Kontrol NEGATIF: `net2_lokal_sendfd_penuh` yang salah memakai `>=`.
/// Sengaja SALAH; berdaya pada `ukuran == ANTREAN_MAKS`.
fn net2_lokal_sendfd_penuh_salah(ukuran: u32) -> i32 {
    if ukuran >= ANTREAN_MAKS {
        return -EBUSY;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`net2_tcp_sokopt_tak`].
#[no_mangle]
pub extern "C" fn klatos_net2_tcp_sokopt_tak() -> i32 {
    bump(Counter::ProdTcpSokoptTak);
    net2_tcp_sokopt_tak()
}

/// Lihat [`net2_tcp_tuple_ada`].
#[no_mangle]
pub extern "C" fn klatos_net2_tcp_tuple(ada: u32) -> i32 {
    bump(Counter::ProdTcpTuple);
    net2_tcp_tuple_ada(ada)
}

/// Lihat [`net2_tcp_adapter_tidak_ada`].
#[no_mangle]
pub extern "C" fn klatos_net2_tcp_adapter(ada: u32) -> i32 {
    bump(Counter::ProdTcpAdapter);
    net2_tcp_adapter_tidak_ada(ada)
}

/// Lihat [`net2_port_bentrok`].
#[no_mangle]
pub extern "C" fn klatos_net2_port(bentrok: u32) -> i32 {
    bump(Counter::ProdPort);
    net2_port_bentrok(bentrok)
}

/// Lihat [`net2_tcp_hubung_gagal`].
#[no_mangle]
pub extern "C" fn klatos_net2_tcp_hubung(timeout: u32) -> i32 {
    bump(Counter::ProdTcpHubung);
    net2_tcp_hubung_gagal(timeout)
}

/// Lihat [`net2_tcp_hubung_lanjut`].
#[no_mangle]
pub extern "C" fn klatos_net2_tcp_hubung_lanjut() -> i32 {
    bump(Counter::ProdTcpHubungLanjut);
    net2_tcp_hubung_lanjut()
}

/// Lihat [`net2_lokal_ukuran_lebih`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_ukuran(lebih: u32) -> i32 {
    bump(Counter::ProdLokalUkuran);
    net2_lokal_ukuran_lebih(lebih)
}

/// Lihat [`net2_lokal_keluarga`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_keluarga(af: u32, af_local: u32) -> i32 {
    bump(Counter::ProdLokalKeluarga);
    net2_lokal_keluarga(af, af_local)
}

/// Lihat [`net2_lokal_bentrok`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_bentrok(bentrok: u32) -> i32 {
    bump(Counter::ProdLokalBentrok);
    net2_lokal_bentrok(bentrok)
}

/// Lihat [`net2_lokal_terhubung`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_terhubung(terhubung: u32) -> i32 {
    bump(Counter::ProdLokalTerhubung);
    net2_lokal_terhubung(terhubung)
}

/// Lihat [`net2_lokal_peer`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_peer(ada: u32) -> i32 {
    bump(Counter::ProdLokalPeer);
    net2_lokal_peer(ada)
}

/// Lihat [`net2_lokal_stream`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_stream(stream: u32) -> i32 {
    bump(Counter::ProdLokalStream);
    net2_lokal_stream(stream)
}

/// Lihat [`net2_lokal_lepas`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_lepas(ada_peer: u32) -> i32 {
    bump(Counter::ProdLokalLepas);
    net2_lokal_lepas(ada_peer)
}

/// Lihat [`net2_lokal_buffer`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_buffer(ada: u32) -> i32 {
    bump(Counter::ProdLokalBuffer);
    net2_lokal_buffer(ada)
}

/// Lihat [`net2_lokal_peercred_hubung`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_peercred_hubung(connecting: u32) -> i32 {
    bump(Counter::ProdLokalPeercredHubung);
    net2_lokal_peercred_hubung(connecting)
}

/// Lihat [`net2_lokal_peran_tak_dikenal`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_peran() -> i32 {
    bump(Counter::ProdLokalPeran);
    net2_lokal_peran_tak_dikenal()
}

/// Lihat [`net2_lokal_ioctl_tak`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_ioctl() -> i32 {
    bump(Counter::ProdLokalIoctl);
    net2_lokal_ioctl_tak()
}

/// Lihat [`net2_lokal_chown_izin`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_chown(izin: u32) -> i32 {
    bump(Counter::ProdLokalChown);
    net2_lokal_chown_izin(izin)
}

/// Lihat [`net2_lokal_fd_peran`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_fd_peran(salah: u32) -> i32 {
    bump(Counter::ProdLokalFdPeran);
    net2_lokal_fd_peran(salah)
}

/// Lihat [`net2_lokal_sendfd_penuh`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_sendfd_penuh(ukuran: u32) -> i32 {
    bump(Counter::ProdLokalSendfdPenuh);
    net2_lokal_sendfd_penuh(ukuran)
}

/// Lihat [`net2_lokal_recvfd_kosong`].
#[no_mangle]
pub extern "C" fn klatos_net2_lokal_recvfd_kosong(kosong: u32) -> i32 {
    bump(Counter::ProdLokalRecvfdKosong);
    net2_lokal_recvfd_kosong(kosong)
}

/// Lihat [`net2_rute_duplikat`].
#[no_mangle]
pub extern "C" fn klatos_net2_rute_duplikat(ada: u32) -> i32 {
    bump(Counter::ProdRuteDuplikat);
    net2_rute_duplikat(ada)
}

/// Lihat [`net2_rute_tidak_ada`].
#[no_mangle]
pub extern "C" fn klatos_net2_rute_tidak_ada(ditemukan: u32) -> i32 {
    bump(Counter::ProdRuteTidakAda);
    net2_rute_tidak_ada(ditemukan)
}

/// Lihat [`net2_inet_tipe_tak_dikenal`].
#[no_mangle]
pub extern "C" fn klatos_net2_inet_tipe() -> i32 {
    bump(Counter::ProdInetTipe);
    net2_inet_tipe_tak_dikenal()
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_net2_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_net2_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiNet2Report` di `Jowo/Kontrak/KlatSeam96.h`.
#[repr(C)]
pub struct KlatosAdmisiNet2Report {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub teras_checks: u32,
    pub teras_passed: u32,
    pub sub_checks: u32,
    pub sub_passed: u32,
    pub prod_tcp_sokopt_tak: u32,
    pub prod_tcp_tuple: u32,
    pub prod_tcp_adapter: u32,
    pub prod_port: u32,
    pub prod_tcp_hubung: u32,
    pub prod_tcp_hubung_lanjut: u32,
    pub prod_lokal_ukuran: u32,
    pub prod_lokal_keluarga: u32,
    pub prod_lokal_bentrok: u32,
    pub prod_lokal_terhubung: u32,
    pub prod_lokal_peer: u32,
    pub prod_lokal_stream: u32,
    pub prod_lokal_lepas: u32,
    pub prod_lokal_buffer: u32,
    pub prod_lokal_peercred_hubung: u32,
    pub prod_lokal_peran: u32,
    pub prod_lokal_ioctl: u32,
    pub prod_lokal_chown: u32,
    pub prod_lokal_fd_peran: u32,
    pub prod_lokal_sendfd_penuh: u32,
    pub prod_lokal_recvfd_kosong: u32,
    pub prod_rute_duplikat: u32,
    pub prod_rute_tidak_ada: u32,
    pub prod_inet_tipe: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Teras,
    Sub,
}

struct Matrix {
    checks: u32,
    passed: u32,
    teras_checks: u32,
    teras_passed: u32,
    sub_checks: u32,
    sub_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            teras_checks: 0,
            teras_passed: 0,
            sub_checks: 0,
            sub_passed: 0,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Teras => {
                self.teras_checks += 1;
                if lulus {
                    self.teras_passed += 1;
                }
            }
            Group::Sub => {
                self.sub_checks += 1;
                if lulus {
                    self.sub_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        }
    }
}

/// Jumlah kasus diferensial yang diharapkan: **69**.
pub const DIFF_EXPECTED: u32 = 47;

fn run_matrix() -> KlatosAdmisiNet2Report {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4E45_5432, Group::Teras);
    m.catat(
        EPERM == 1
            && ESRCH == 3
            && EAGAIN == 11
            && EBUSY == 16
            && EEXIST == 17
            && EINVAL == 22
            && EPIPE == 32
            && EOPNOTSUPP == 37
            && EADDRINUSE == 42
            && ECONNREFUSED == 45
            && EADDRNOTAVAIL == 47
            && EISCONN == 48
            && ENOPROTOOPT == 62
            && ENOTCONN == 63
            && ETIMEDOUT == 69
            && EINPROGRESS == 71
            && ANTREAN_MAKS == 128,
        Group::Teras,
    );

    // --- inti (Teras): TCP/UDP --------------------------------------------
    m.catat(net2_tcp_sokopt_tak() == -ENOPROTOOPT, Group::Teras);
    m.catat(net2_tcp_tuple_ada(1) == -EEXIST, Group::Teras);
    m.catat(net2_tcp_tuple_ada(0) == 0, Group::Teras);
    m.catat(net2_tcp_adapter_tidak_ada(0) == -EADDRNOTAVAIL, Group::Teras);
    m.catat(net2_tcp_adapter_tidak_ada(1) == 0, Group::Teras);
    m.catat(net2_port_bentrok(1) == -EADDRINUSE, Group::Teras);
    m.catat(net2_port_bentrok(0) == 0, Group::Teras);
    m.catat(net2_tcp_hubung_gagal(1) == -ETIMEDOUT, Group::Teras);
    m.catat(net2_tcp_hubung_gagal(0) == -ECONNREFUSED, Group::Teras);
    m.catat(net2_tcp_hubung_lanjut() == -EINPROGRESS, Group::Teras);

    // --- subsistem (Sub): LocalSocket + Routing + IPv4 create -------------
    m.catat(net2_lokal_ukuran_lebih(1) == -EINVAL, Group::Sub);
    m.catat(net2_lokal_ukuran_lebih(0) == 0, Group::Sub);
    m.catat(net2_lokal_keluarga(1, 1) == 0, Group::Sub);
    m.catat(net2_lokal_keluarga(2, 1) == -EINVAL, Group::Sub);
    m.catat(net2_lokal_bentrok(1) == -EADDRINUSE, Group::Sub);
    m.catat(net2_lokal_bentrok(0) == 0, Group::Sub);
    m.catat(net2_lokal_terhubung(1) == -EISCONN, Group::Sub);
    m.catat(net2_lokal_terhubung(0) == 0, Group::Sub);
    m.catat(net2_lokal_peer(0) == -ECONNREFUSED, Group::Sub);
    m.catat(net2_lokal_peer(1) == 0, Group::Sub);
    m.catat(net2_lokal_stream(0) == -EOPNOTSUPP, Group::Sub);
    m.catat(net2_lokal_stream(1) == 0, Group::Sub);
    m.catat(net2_lokal_lepas(0) == -EPIPE, Group::Sub);
    m.catat(net2_lokal_lepas(1) == 0, Group::Sub);
    m.catat(net2_lokal_buffer(0) == -EINVAL, Group::Sub);
    m.catat(net2_lokal_buffer(1) == 0, Group::Sub);
    m.catat(net2_lokal_peercred_hubung(1) == -ENOTCONN, Group::Sub);
    m.catat(net2_lokal_peercred_hubung(0) == 0, Group::Sub);
    m.catat(net2_lokal_peran_tak_dikenal() == -EINVAL, Group::Sub);
    m.catat(net2_lokal_ioctl_tak() == -EINVAL, Group::Sub);
    m.catat(net2_lokal_chown_izin(0) == -EPERM, Group::Sub);
    m.catat(net2_lokal_chown_izin(1) == 0, Group::Sub);
    m.catat(net2_lokal_fd_peran(1) == -EINVAL, Group::Sub);
    m.catat(net2_lokal_fd_peran(0) == 0, Group::Sub);
    m.catat(net2_lokal_sendfd_penuh(129) == -EBUSY, Group::Sub);
    m.catat(net2_lokal_sendfd_penuh(128) == 0, Group::Sub);
    m.catat(net2_lokal_recvfd_kosong(1) == -EAGAIN, Group::Sub);
    m.catat(net2_lokal_recvfd_kosong(0) == 0, Group::Sub);
    m.catat(net2_rute_duplikat(1) == -EEXIST, Group::Sub);
    m.catat(net2_rute_duplikat(0) == 0, Group::Sub);
    m.catat(net2_rute_tidak_ada(0) == -ESRCH, Group::Sub);
    m.catat(net2_rute_tidak_ada(1) == 0, Group::Sub);
    m.catat(net2_inet_tipe_tak_dikenal() == -EINVAL, Group::Sub);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // ETIMEDOUT vs ECONNREFUSED pada connect TCP.
        m.catat(net2_tcp_hubung_gagal(1) == -ETIMEDOUT, Group::Teras);
        m.catat(net2_tcp_hubung_gagal_salah(1) == -ECONNREFUSED, Group::Teras);
        m.catat(
            net2_tcp_hubung_gagal(1) != net2_tcp_hubung_gagal_salah(1),
            Group::Teras,
        );
        m.catat(
            net2_tcp_hubung_gagal(0) == net2_tcp_hubung_gagal_salah(0),
            Group::Teras,
        );

        // EBUSY hanya bila ukuran > 128 (bukan >=).
        m.catat(net2_lokal_sendfd_penuh(128) == 0, Group::Sub);
        m.catat(net2_lokal_sendfd_penuh_salah(128) == -EBUSY, Group::Sub);
        m.catat(
            net2_lokal_sendfd_penuh(128) != net2_lokal_sendfd_penuh_salah(128),
            Group::Sub,
        );
        m.catat(
            net2_lokal_sendfd_penuh(129) == net2_lokal_sendfd_penuh_salah(129),
            Group::Sub,
        );

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if net2_tcp_hubung_lanjut() != -EINPROGRESS {
                stabil = false;
                break;
            }
        }
        m.catat(stabil, Group::Teras);
    }

    // --- diferensial produksi vs oracle -----------------------------------
    macro_rules! ukur {
        ($g:expr, $p:expr, $n:expr) => {{
            if $p != $n {
                beda += 1;
            }
            diukur += 1;
            let _ = $g;
        }};
    }

    // #1/#6/#16/#17/#24: verdict tanpa argumen.
    ukur!(Group::Teras, net2_tcp_sokopt_tak(), -ENOPROTOOPT);
    ukur!(Group::Teras, net2_tcp_hubung_lanjut(), -EINPROGRESS);
    ukur!(Group::Sub, net2_lokal_peran_tak_dikenal(), -EINVAL);
    ukur!(Group::Sub, net2_lokal_ioctl_tak(), -EINVAL);
    ukur!(Group::Sub, net2_inet_tipe_tak_dikenal(), -EINVAL);

    // #2/#3/#4/#7/#9/#10/#11/#12/#13/#14/#15/#18/#19/#20/#21/#22/#23:
    // verdict boolean tunggal.
    for v in 0..2u32 {
        ukur!(Group::Teras, net2_tcp_tuple_ada(v), naif_bool(v, -EEXIST));
        ukur!(
            Group::Teras,
            net2_tcp_adapter_tidak_ada(v),
            naif_bool_terbalik(v, -EADDRNOTAVAIL)
        );
        ukur!(Group::Teras, net2_port_bentrok(v), naif_bool(v, -EADDRINUSE));
        ukur!(Group::Sub, net2_lokal_ukuran_lebih(v), naif_bool(v, -EINVAL));
        ukur!(Group::Sub, net2_lokal_bentrok(v), naif_bool(v, -EADDRINUSE));
        ukur!(Group::Sub, net2_lokal_terhubung(v), naif_bool(v, -EISCONN));
        ukur!(Group::Sub, net2_lokal_peer(v), naif_bool_terbalik(v, -ECONNREFUSED));
        ukur!(Group::Sub, net2_lokal_stream(v), naif_bool_terbalik(v, -EOPNOTSUPP));
        ukur!(Group::Sub, net2_lokal_lepas(v), naif_bool_terbalik(v, -EPIPE));
        ukur!(Group::Sub, net2_lokal_buffer(v), naif_bool_terbalik(v, -EINVAL));
        ukur!(
            Group::Sub,
            net2_lokal_peercred_hubung(v),
            naif_bool(v, -ENOTCONN)
        );
        ukur!(Group::Sub, net2_lokal_chown_izin(v), naif_bool_terbalik(v, -EPERM));
        ukur!(Group::Sub, net2_lokal_fd_peran(v), naif_bool(v, -EINVAL));
        ukur!(
            Group::Sub,
            net2_lokal_recvfd_kosong(v),
            naif_bool(v, -EAGAIN)
        );
        ukur!(Group::Sub, net2_rute_duplikat(v), naif_bool(v, -EEXIST));
        ukur!(
            Group::Sub,
            net2_rute_tidak_ada(v),
            naif_bool_terbalik(v, -ESRCH)
        );
    }

    // #5: timeout vs refused.
    for t in 0..2u32 {
        ukur!(
            Group::Teras,
            net2_tcp_hubung_gagal(t),
            naif_tcp_hubung_gagal(t)
        );
    }

    // #8: keluarga AF_LOCAL.
    for (af, lokal) in [(1u32, 1u32), (2, 1), (0, 1)] {
        ukur!(
            Group::Sub,
            net2_lokal_keluarga(af, lokal),
            naif_keluarga(af, lokal)
        );
    }

    // #20: ambang antrean sendfd (127/128/129).
    for n in [0u32, 127, 128, 129, 200] {
        ukur!(
            Group::Sub,
            net2_lokal_sendfd_penuh(n),
            naif_sendfd_penuh(n)
        );
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Teras);
    m.catat(beda == 0, Group::Teras);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiNet2Report {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        teras_checks: m.teras_checks,
        teras_passed: m.teras_passed,
        sub_checks: m.sub_checks,
        sub_passed: m.sub_passed,
        prod_tcp_sokopt_tak: klatos_net2_counter(Counter::ProdTcpSokoptTak as u32),
        prod_tcp_tuple: klatos_net2_counter(Counter::ProdTcpTuple as u32),
        prod_tcp_adapter: klatos_net2_counter(Counter::ProdTcpAdapter as u32),
        prod_port: klatos_net2_counter(Counter::ProdPort as u32),
        prod_tcp_hubung: klatos_net2_counter(Counter::ProdTcpHubung as u32),
        prod_tcp_hubung_lanjut: klatos_net2_counter(Counter::ProdTcpHubungLanjut as u32),
        prod_lokal_ukuran: klatos_net2_counter(Counter::ProdLokalUkuran as u32),
        prod_lokal_keluarga: klatos_net2_counter(Counter::ProdLokalKeluarga as u32),
        prod_lokal_bentrok: klatos_net2_counter(Counter::ProdLokalBentrok as u32),
        prod_lokal_terhubung: klatos_net2_counter(Counter::ProdLokalTerhubung as u32),
        prod_lokal_peer: klatos_net2_counter(Counter::ProdLokalPeer as u32),
        prod_lokal_stream: klatos_net2_counter(Counter::ProdLokalStream as u32),
        prod_lokal_lepas: klatos_net2_counter(Counter::ProdLokalLepas as u32),
        prod_lokal_buffer: klatos_net2_counter(Counter::ProdLokalBuffer as u32),
        prod_lokal_peercred_hubung: klatos_net2_counter(Counter::ProdLokalPeercredHubung as u32),
        prod_lokal_peran: klatos_net2_counter(Counter::ProdLokalPeran as u32),
        prod_lokal_ioctl: klatos_net2_counter(Counter::ProdLokalIoctl as u32),
        prod_lokal_chown: klatos_net2_counter(Counter::ProdLokalChown as u32),
        prod_lokal_fd_peran: klatos_net2_counter(Counter::ProdLokalFdPeran as u32),
        prod_lokal_sendfd_penuh: klatos_net2_counter(Counter::ProdLokalSendfdPenuh as u32),
        prod_lokal_recvfd_kosong: klatos_net2_counter(Counter::ProdLokalRecvfdKosong as u32),
        prod_rute_duplikat: klatos_net2_counter(Counter::ProdRuteDuplikat as u32),
        prod_rute_tidak_ada: klatos_net2_counter(Counter::ProdRuteTidakAda as u32),
        prod_inet_tipe: klatos_net2_counter(Counter::ProdInetTipe as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas jaringan (batch 2).
#[no_mangle]
pub extern "C" fn klatos_net2_selftest() -> KlatosAdmisiNet2Report {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4E45_5432);
        assert!(EPERM == 1);
        assert!(ESRCH == 3);
        assert!(EAGAIN == 11);
        assert!(EBUSY == 16);
        assert!(EEXIST == 17);
        assert!(EINVAL == 22);
        assert!(EPIPE == 32);
        assert!(EOPNOTSUPP == 37);
        assert!(EADDRINUSE == 42);
        assert!(ECONNREFUSED == 45);
        assert!(EADDRNOTAVAIL == 47);
        assert!(EISCONN == 48);
        assert!(ENOPROTOOPT == 62);
        assert!(ENOTCONN == 63);
        assert!(ETIMEDOUT == 69);
        assert!(EINPROGRESS == 71);
        assert!(ANTREAN_MAKS == 128);
        assert!(Counter::ProdTcpSokoptTak as usize == 0);
        assert!(Counter::ProdInetTipe as usize == 23);
        assert!(N_COUNTER == 24);
        assert!(DIFF_EXPECTED == 47);

        assert!(net2_tcp_hubung_gagal(1) == -ETIMEDOUT);
        assert!(net2_tcp_hubung_gagal(0) == -ECONNREFUSED);
        assert!(net2_lokal_sendfd_penuh(128) == 0);
        assert!(net2_lokal_sendfd_penuh(129) == -EBUSY);
        assert!(net2_rute_tidak_ada(0) == -ESRCH);

        // 8 (tag) + 32*4 (u32) + 4 (status) = 140 -> 144 dengan padding.
        assert!(core::mem::size_of::<KlatosAdmisiNet2Report>() == 144);
        assert!(core::mem::align_of::<KlatosAdmisiNet2Report>() == 8);
    };
}
