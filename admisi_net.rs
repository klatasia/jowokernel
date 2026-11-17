// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.94 / WS6 — **OTORITAS JARINGAN, BATCH 1: IPv4 socket control plane**.


#![allow(unsafe_code)] // rustc mengklasifikasi #[no_mangle] sebagai unsafe_code

use core::sync::atomic::{AtomicU32, Ordering};

/// `EPERM` — cermin `Jowo/Kontrak/POSIX/errno.h` (indeks 1).
pub const EPERM: i32 = 1;
/// `EACCES` — indeks 13.
pub const EACCES: i32 = 13;
/// `EFAULT` — indeks 14.
pub const EFAULT: i32 = 14;
/// `ENODEV` — indeks 19.
pub const ENODEV: i32 = 19;
/// `EINVAL` — indeks 22.
pub const EINVAL: i32 = 22;
/// `EPIPE` — indeks 32.
pub const EPIPE: i32 = 32;
/// `EAFNOSUPPORT` — indeks 40.
pub const EAFNOSUPPORT: i32 = 40;
/// `EISCONN` — indeks 48.
pub const EISCONN: i32 = 48;
/// `ENOPROTOOPT` — indeks 62.
pub const ENOPROTOOPT: i32 = 62;
/// `ENOTSUP` — indeks 74.
pub const ENOTSUP: i32 = 74;

/// Batas nilai `IP_TTL`/`IP_TOS`; cermin `IP/Socket.cpp:528,538`.
pub const TTL_TOS_MAKS: i32 = 255;
/// Port istimewa; cermin `IP/Socket.cpp:131` (`< 1024`).
pub const PORT_ISTIMEWA: u32 = 1024;

/// Tag laporan; cermin `KlatosAdmisiNetReport` di `Jowo/Kontrak/KlatSeam95.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_4E45_5441; // "KLATNETA"

/// Penghitung audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// `klatos_net_sokopt_tak`.
    ProdSokoptTak = 0,
    /// `klatos_net_ukuran_persis`.
    ProdUkuranPersis = 1,
    /// `klatos_net_ukuran_minimum`.
    ProdUkuranMinimum = 2,
    /// `klatos_net_bindtodevice_ada`.
    ProdBindtodeviceAda = 3,
    /// `klatos_net_timestamp`.
    ProdTimestamp = 4,
    /// `klatos_net_bindtodevice_kosong`.
    ProdBindtodeviceKosong = 5,
    /// `klatos_net_ttl_tos`.
    ProdTtlTos = 6,
    /// `klatos_net_loop`.
    ProdLoop = 7,
    /// `klatos_net_interface_any`.
    ProdInterfaceAny = 8,
    /// `klatos_net_ioctl_izin`.
    ProdIoctlIzin = 9,
    /// `klatos_net_ioctl_keluarga`.
    ProdIoctlKeluarga = 10,
    /// `klatos_net_ioctl_rt`.
    ProdIoctlRt = 11,
    /// `klatos_net_ioctl_perangkat`.
    ProdIoctlPerangkat = 12,
    /// `klatos_net_ioctl_indeks`.
    ProdIoctlIndeks = 13,
    /// `klatos_net_ioctl_tak`.
    ProdIoctlTak = 14,
    /// `klatos_net_bind_sudah`.
    ProdBindSudah = 15,
    /// `klatos_net_bind_keluarga`.
    ProdBindKeluarga = 16,
    /// `klatos_net_hubung_sudah`.
    ProdHubungSudah = 17,
    /// `klatos_net_hubung_keluarga`.
    ProdHubungKeluarga = 18,
    /// `klatos_net_port`.
    ProdPort = 19,
    /// `klatos_net_kirim_keluarga`.
    ProdKirimKeluarga = 20,
    /// `klatos_net_kirim_belum`.
    ProdKirimBelum = 21,
}

const N_COUNTER: usize = 22;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Algoritma murni — satu salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Level/opsi sockopt tidak dikenal. Satu verdict, enam pemanggil.
/// Transkripsi `Socket.cpp:103,162,176,283` dan `IP/Socket.cpp:577,616`.
#[inline]
pub const fn net_sokopt_tak() -> i32 {
    -ENOPROTOOPT
}

/// Panjang nilai sockopt harus persis sama. Satu verdict, sebelas pemanggil.
/// Transkripsi `Socket.cpp:108,113,118,137,147,156`,
/// `IP/Socket.cpp:525,535,545,555,567`.
#[inline]
pub const fn net_ukuran_persis(ukuran: u64, harap: u64) -> i32 {
    if ukuran != harap {
        return -EINVAL;
    }
    0
}

/// Panjang nilai sockopt minimal. Satu verdict, empat belas pemanggil.
/// Transkripsi `Socket.cpp:182,191,200,214,232,238,245,252,260,268,276`,
/// `IP/Socket.cpp:594,602,610`.
#[inline]
pub const fn net_ukuran_minimum(ukuran: u64, harap: u64) -> i32 {
    if ukuran < harap {
        return -EINVAL;
    }
    0
}

/// `SO_BINDTODEVICE` perangkat tidak ditemukan. Transkripsi `Socket.cpp:123`.
#[inline]
pub const fn net_bindtodevice_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -ENODEV;
    }
    0
}

/// `SO_TIMESTAMP` diminta untuk domain/tipe yang tak didukung.
/// Transkripsi `Socket.cpp:142`.
#[inline]
pub const fn net_timestamp_dukung(nilai_bukan_nol: u32, domain_inet: u32, tipe_stream: u32) -> i32 {
    if nilai_bukan_nol != 0 && (domain_inet == 0 || tipe_stream != 0) {
        return -ENOTSUP;
    }
    0
}

/// `getsockopt(SO_BINDTODEVICE)` tanpa perangkat terikat.
/// Transkripsi `Socket.cpp:227`.
#[inline]
pub const fn net_bindtodevice_kosong(kosong: u32) -> i32 {
    if kosong != 0 {
        return -EFAULT;
    }
    0
}

/// `IP_TTL`/`IP_TOS` di luar rentang 0..255. Transkripsi `IP/Socket.cpp:529,539`.
#[inline]
pub const fn net_ttl_tos(nilai: i32) -> i32 {
    if nilai < 0 || nilai > TTL_TOS_MAKS {
        return -EINVAL;
    }
    0
}

/// `IP_MULTICAST_LOOP` bukan 0/1. Transkripsi `IP/Socket.cpp:549`.
#[inline]
pub const fn net_loop(nilai: i32) -> i32 {
    if nilai != 0 && nilai != 1 {
        return -EINVAL;
    }
    0
}

/// `IP_ADD_MEMBERSHIP`/`IP_DROP_MEMBERSHIP` antarmuka bukan `INADDR_ANY`.
/// Transkripsi `IP/Socket.cpp:559,571`.
#[inline]
pub const fn net_interface_any(bukan_any: u32) -> i32 {
    if bukan_any != 0 {
        return -ENOTSUP;
    }
    0
}

/// `ioctl` rute/ARP/antarmuka butuh superuser. Satu verdict, enam pemanggil.
/// Transkripsi `IP/Socket.cpp:641,656,680,688,761,774`.
#[inline]
pub const fn net_ioctl_izin(izin: u32) -> i32 {
    if izin == 0 {
        return -EPERM;
    }
    0
}

/// `ioctl` keluarga alamat bukan `AF_INET`. Satu verdict, enam pemanggil.
/// Transkripsi `IP/Socket.cpp:642,658,682,690,769,776`.
#[inline]
pub const fn net_ioctl_keluarga(af: u32, af_inet: u32) -> i32 {
    if af != af_inet {
        return -EAFNOSUPPORT;
    }
    0
}

/// `SIOCADDRT` tanpa bendera `RTF_UP`. Transkripsi `IP/Socket.cpp:645`.
#[inline]
pub const fn net_ioctl_rt(bendera_up: u32) -> i32 {
    if bendera_up == 0 {
        return -EINVAL;
    }
    0
}

/// `ioctl` perangkat tidak ditemukan. Satu verdict, empat pemanggil.
/// Transkripsi `IP/Socket.cpp:635,726,749,754`.
#[inline]
pub const fn net_ioctl_perangkat_ada(ada: u32) -> i32 {
    if ada == 0 {
        return -ENODEV;
    }
    0
}

/// `SIOCGIFNAME` dengan indeks 0. Transkripsi `IP/Socket.cpp:706`.
#[inline]
pub const fn net_ioctl_indeks(indeks: u32) -> i32 {
    if indeks == 0 {
        return -EINVAL;
    }
    0
}

/// Permintaan `ioctl` tidak dikenal atau stub. Satu verdict, lima pemanggil.
/// Transkripsi `IP/Socket.cpp:667,695,843,847,886`.
#[inline]
pub const fn net_ioctl_tak() -> i32 {
    -EINVAL
}

/// `bind` pada soket yang sudah terikat. Transkripsi `IP/Socket.cpp:117`.
#[inline]
pub const fn net_bind_sudah(bound: u32) -> i32 {
    if bound != 0 {
        return -EINVAL;
    }
    0
}

/// `bind` keluarga alamat bukan `AF_INET`. Transkripsi `IP/Socket.cpp:127`.
#[inline]
pub const fn net_bind_keluarga(af: u32, af_inet: u32) -> i32 {
    if af != af_inet {
        return -EINVAL;
    }
    0
}

/// `connect` pada soket yang sudah terhubung. Transkripsi `IP/Socket.cpp:168`.
#[inline]
pub const fn net_hubung_sudah(terhubung: u32) -> i32 {
    if terhubung != 0 {
        return -EISCONN;
    }
    0
}

/// `connect` keluarga alamat bukan `AF_INET`. Transkripsi `IP/Socket.cpp:166`.
#[inline]
pub const fn net_hubung_keluarga(af: u32, af_inet: u32) -> i32 {
    if af != af_inet {
        return -EINVAL;
    }
    0
}

/// `bind` port istimewa tanpa hak superuser. Transkripsi `IP/Socket.cpp:133`.
#[inline]
pub const fn net_port(port: u32, superuser: u32) -> i32 {
    if superuser == 0 && port > 0 && port < PORT_ISTIMEWA {
        return -EACCES;
    }
    0
}

/// `sendto` keluarga alamat bukan `AF_INET`. Transkripsi `IP/Socket.cpp:208`.
#[inline]
pub const fn net_kirim_keluarga(af: u32, af_inet: u32) -> i32 {
    if af != af_inet {
        return -EAFNOSUPPORT;
    }
    0
}

/// `sendto` tanpa tujuan dan belum terhubung. Transkripsi `IP/Socket.cpp:218`.
#[inline]
pub const fn net_kirim_belum(belum_terhubung: u32, peer_nol: u32) -> i32 {
    if belum_terhubung != 0 && peer_nol != 0 {
        return -EPIPE;
    }
    0
}

// ---------------------------------------------------------------------------
// Oracle pembanding deterministik — formulasi BERBEDA dari produksi
// ---------------------------------------------------------------------------

fn naif_ukuran_persis(ukuran: u64, harap: u64) -> i32 {
    match ukuran {
        u if u == harap => 0,
        _ => -EINVAL,
    }
}
fn naif_ukuran_minimum(ukuran: u64, harap: u64) -> i32 {
    match ukuran {
        u if u < harap => -EINVAL,
        _ => 0,
    }
}
fn naif_bindtodevice_ada(ada: u32) -> i32 {
    match ada {
        0 => -ENODEV,
        _ => 0,
    }
}
fn naif_timestamp(nilai_bukan_nol: u32, domain_inet: u32, tipe_stream: u32) -> i32 {
    if nilai_bukan_nol == 0 {
        0
    } else if domain_inet == 1 && tipe_stream == 0 {
        0
    } else {
        -ENOTSUP
    }
}
fn naif_bindtodevice_kosong(kosong: u32) -> i32 {
    match kosong {
        0 => 0,
        _ => -EFAULT,
    }
}
fn naif_ttl_tos(nilai: i32) -> i32 {
    if (0..=TTL_TOS_MAKS).contains(&nilai) {
        0
    } else {
        -EINVAL
    }
}
fn naif_loop(nilai: i32) -> i32 {
    match nilai {
        0 | 1 => 0,
        _ => -EINVAL,
    }
}
fn naif_interface_any(bukan_any: u32) -> i32 {
    match bukan_any {
        0 => 0,
        _ => -ENOTSUP,
    }
}
fn naif_ioctl_izin(izin: u32) -> i32 {
    match izin {
        0 => -EPERM,
        _ => 0,
    }
}
fn naif_keluarga(af: u32, af_inet: u32, errno: i32) -> i32 {
    if af == af_inet {
        0
    } else {
        errno
    }
}
fn naif_ioctl_rt(bendera_up: u32) -> i32 {
    match bendera_up {
        0 => -EINVAL,
        _ => 0,
    }
}
fn naif_ioctl_perangkat_ada(ada: u32) -> i32 {
    match ada {
        0 => -ENODEV,
        _ => 0,
    }
}
fn naif_ioctl_indeks(indeks: u32) -> i32 {
    match indeks {
        0 => -EINVAL,
        _ => 0,
    }
}
fn naif_bind_sudah(bound: u32) -> i32 {
    match bound {
        0 => 0,
        _ => -EINVAL,
    }
}
fn naif_hubung_sudah(terhubung: u32) -> i32 {
    match terhubung {
        0 => 0,
        _ => -EISCONN,
    }
}
fn naif_port(port: u32, superuser: u32) -> i32 {
    if superuser == 1 || port == 0 || port >= PORT_ISTIMEWA {
        0
    } else {
        -EACCES
    }
}
fn naif_kirim_belum(belum_terhubung: u32, peer_nol: u32) -> i32 {
    if belum_terhubung == 1 && peer_nol == 1 {
        -EPIPE
    } else {
        0
    }
}

/// Kontrol NEGATIF: `net_port` yang salah memakai `<= 1024` (bukan `<`).
/// Sengaja SALAH; berdaya pada `port == PORT_ISTIMEWA`.
fn net_port_salah(port: u32, superuser: u32) -> i32 {
    if superuser == 0 && port > 0 && port <= PORT_ISTIMEWA {
        return -EACCES;
    }
    0
}

/// Kontrol NEGATIF: `net_ioctl_keluarga` yang salah mengembalikan `EINVAL`
/// (bukan `EAFNOSUPPORT`). Sengaja SALAH.
fn net_ioctl_keluarga_salah(af: u32, af_inet: u32) -> i32 {
    if af != af_inet {
        return -EINVAL;
    }
    0
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Lihat [`net_sokopt_tak`].
#[no_mangle]
pub extern "C" fn klatos_net_sokopt_tak() -> i32 {
    bump(Counter::ProdSokoptTak);
    net_sokopt_tak()
}

/// Lihat [`net_ukuran_persis`].
#[no_mangle]
pub extern "C" fn klatos_net_ukuran_persis(ukuran: u64, harap: u64) -> i32 {
    bump(Counter::ProdUkuranPersis);
    net_ukuran_persis(ukuran, harap)
}

/// Lihat [`net_ukuran_minimum`].
#[no_mangle]
pub extern "C" fn klatos_net_ukuran_minimum(ukuran: u64, harap: u64) -> i32 {
    bump(Counter::ProdUkuranMinimum);
    net_ukuran_minimum(ukuran, harap)
}

/// Lihat [`net_bindtodevice_ada`].
#[no_mangle]
pub extern "C" fn klatos_net_bindtodevice_ada(ada: u32) -> i32 {
    bump(Counter::ProdBindtodeviceAda);
    net_bindtodevice_ada(ada)
}

/// Lihat [`net_timestamp_dukung`].
#[no_mangle]
pub extern "C" fn klatos_net_timestamp(nilai_bukan_nol: u32, domain_inet: u32, tipe_stream: u32) -> i32 {
    bump(Counter::ProdTimestamp);
    net_timestamp_dukung(nilai_bukan_nol, domain_inet, tipe_stream)
}

/// Lihat [`net_bindtodevice_kosong`].
#[no_mangle]
pub extern "C" fn klatos_net_bindtodevice_kosong(kosong: u32) -> i32 {
    bump(Counter::ProdBindtodeviceKosong);
    net_bindtodevice_kosong(kosong)
}

/// Lihat [`net_ttl_tos`].
#[no_mangle]
pub extern "C" fn klatos_net_ttl_tos(nilai: i32) -> i32 {
    bump(Counter::ProdTtlTos);
    net_ttl_tos(nilai)
}

/// Lihat [`net_loop`].
#[no_mangle]
pub extern "C" fn klatos_net_loop(nilai: i32) -> i32 {
    bump(Counter::ProdLoop);
    net_loop(nilai)
}

/// Lihat [`net_interface_any`].
#[no_mangle]
pub extern "C" fn klatos_net_interface_any(bukan_any: u32) -> i32 {
    bump(Counter::ProdInterfaceAny);
    net_interface_any(bukan_any)
}

/// Lihat [`net_ioctl_izin`].
#[no_mangle]
pub extern "C" fn klatos_net_ioctl_izin(izin: u32) -> i32 {
    bump(Counter::ProdIoctlIzin);
    net_ioctl_izin(izin)
}

/// Lihat [`net_ioctl_keluarga`].
#[no_mangle]
pub extern "C" fn klatos_net_ioctl_keluarga(af: u32, af_inet: u32) -> i32 {
    bump(Counter::ProdIoctlKeluarga);
    net_ioctl_keluarga(af, af_inet)
}

/// Lihat [`net_ioctl_rt`].
#[no_mangle]
pub extern "C" fn klatos_net_ioctl_rt(bendera_up: u32) -> i32 {
    bump(Counter::ProdIoctlRt);
    net_ioctl_rt(bendera_up)
}

/// Lihat [`net_ioctl_perangkat_ada`].
#[no_mangle]
pub extern "C" fn klatos_net_ioctl_perangkat(ada: u32) -> i32 {
    bump(Counter::ProdIoctlPerangkat);
    net_ioctl_perangkat_ada(ada)
}

/// Lihat [`net_ioctl_indeks`].
#[no_mangle]
pub extern "C" fn klatos_net_ioctl_indeks(indeks: u32) -> i32 {
    bump(Counter::ProdIoctlIndeks);
    net_ioctl_indeks(indeks)
}

/// Lihat [`net_ioctl_tak`].
#[no_mangle]
pub extern "C" fn klatos_net_ioctl_tak() -> i32 {
    bump(Counter::ProdIoctlTak);
    net_ioctl_tak()
}

/// Lihat [`net_bind_sudah`].
#[no_mangle]
pub extern "C" fn klatos_net_bind_sudah(bound: u32) -> i32 {
    bump(Counter::ProdBindSudah);
    net_bind_sudah(bound)
}

/// Lihat [`net_bind_keluarga`].
#[no_mangle]
pub extern "C" fn klatos_net_bind_keluarga(af: u32, af_inet: u32) -> i32 {
    bump(Counter::ProdBindKeluarga);
    net_bind_keluarga(af, af_inet)
}

/// Lihat [`net_hubung_sudah`].
#[no_mangle]
pub extern "C" fn klatos_net_hubung_sudah(terhubung: u32) -> i32 {
    bump(Counter::ProdHubungSudah);
    net_hubung_sudah(terhubung)
}

/// Lihat [`net_hubung_keluarga`].
#[no_mangle]
pub extern "C" fn klatos_net_hubung_keluarga(af: u32, af_inet: u32) -> i32 {
    bump(Counter::ProdHubungKeluarga);
    net_hubung_keluarga(af, af_inet)
}

/// Lihat [`net_port`].
#[no_mangle]
pub extern "C" fn klatos_net_port(port: u32, superuser: u32) -> i32 {
    bump(Counter::ProdPort);
    net_port(port, superuser)
}

/// Lihat [`net_kirim_keluarga`].
#[no_mangle]
pub extern "C" fn klatos_net_kirim_keluarga(af: u32, af_inet: u32) -> i32 {
    bump(Counter::ProdKirimKeluarga);
    net_kirim_keluarga(af, af_inet)
}

/// Lihat [`net_kirim_belum`].
#[no_mangle]
pub extern "C" fn klatos_net_kirim_belum(belum_terhubung: u32, peer_nol: u32) -> i32 {
    bump(Counter::ProdKirimBelum);
    net_kirim_belum(belum_terhubung, peer_nol)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_net_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

/// Reset penghitung ke 0. TIDAK dipanggil dari produksi.
#[no_mangle]
pub extern "C" fn klatos_net_reset() -> u32 {
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAdmisiNetReport` di `Jowo/Kontrak/KlatSeam95.h`.
#[repr(C)]
pub struct KlatosAdmisiNetReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub teras_checks: u32,
    pub teras_passed: u32,
    pub sub_checks: u32,
    pub sub_passed: u32,
    pub prod_sokopt_tak: u32,
    pub prod_ukuran_persis: u32,
    pub prod_ukuran_minimum: u32,
    pub prod_bindtodevice_ada: u32,
    pub prod_timestamp: u32,
    pub prod_bindtodevice_kosong: u32,
    pub prod_ttl_tos: u32,
    pub prod_loop: u32,
    pub prod_interface_any: u32,
    pub prod_ioctl_izin: u32,
    pub prod_ioctl_keluarga: u32,
    pub prod_ioctl_rt: u32,
    pub prod_ioctl_perangkat: u32,
    pub prod_ioctl_indeks: u32,
    pub prod_ioctl_tak: u32,
    pub prod_bind_sudah: u32,
    pub prod_bind_keluarga: u32,
    pub prod_hubung_sudah: u32,
    pub prod_hubung_keluarga: u32,
    pub prod_port: u32,
    pub prod_kirim_keluarga: u32,
    pub prod_kirim_belum: u32,
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

/// Jumlah kasus diferensial yang diharapkan: **74**.
pub const DIFF_EXPECTED: u32 = 74;

fn run_matrix() -> KlatosAdmisiNetReport {
    let mut m = Matrix::new();
    let mut beda: u32 = 0;
    let mut diukur: u32 = 0;

    // --- konstanta --------------------------------------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_4E45_5441, Group::Teras);
    m.catat(
        EPERM == 1
            && EACCES == 13
            && EFAULT == 14
            && ENODEV == 19
            && EINVAL == 22
            && EPIPE == 32
            && EAFNOSUPPORT == 40
            && EISCONN == 48
            && ENOPROTOOPT == 62
            && ENOTSUP == 74
            && TTL_TOS_MAKS == 255
            && PORT_ISTIMEWA == 1024,
        Group::Teras,
    );

    // --- inti (Teras): sockopt --------------------------------------------
    m.catat(net_sokopt_tak() == -ENOPROTOOPT, Group::Teras);
    m.catat(net_ukuran_persis(16, 16) == 0, Group::Teras);
    m.catat(net_ukuran_persis(4, 16) == -EINVAL, Group::Teras);
    m.catat(net_ukuran_minimum(16, 16) == 0, Group::Teras);
    m.catat(net_ukuran_minimum(15, 16) == -EINVAL, Group::Teras);
    m.catat(net_bindtodevice_ada(0) == -ENODEV, Group::Teras);
    m.catat(net_bindtodevice_ada(1) == 0, Group::Teras);
    m.catat(net_timestamp_dukung(0, 0, 1) == 0, Group::Teras);
    m.catat(net_timestamp_dukung(1, 1, 0) == 0, Group::Teras);
    m.catat(net_timestamp_dukung(1, 0, 0) == -ENOTSUP, Group::Teras);
    m.catat(net_timestamp_dukung(1, 1, 1) == -ENOTSUP, Group::Teras);
    m.catat(net_bindtodevice_kosong(1) == -EFAULT, Group::Teras);
    m.catat(net_bindtodevice_kosong(0) == 0, Group::Teras);
    m.catat(net_ttl_tos(0) == 0, Group::Teras);
    m.catat(net_ttl_tos(255) == 0, Group::Teras);
    m.catat(net_ttl_tos(256) == -EINVAL, Group::Teras);
    m.catat(net_ttl_tos(-1) == -EINVAL, Group::Teras);
    m.catat(net_loop(0) == 0, Group::Teras);
    m.catat(net_loop(1) == 0, Group::Teras);
    m.catat(net_loop(2) == -EINVAL, Group::Teras);
    m.catat(net_interface_any(1) == -ENOTSUP, Group::Teras);
    m.catat(net_interface_any(0) == 0, Group::Teras);

    // --- subsistem (Sub): ioctl + alamat ----------------------------------
    m.catat(net_ioctl_izin(0) == -EPERM, Group::Sub);
    m.catat(net_ioctl_izin(1) == 0, Group::Sub);
    m.catat(net_ioctl_keluarga(2, 2) == 0, Group::Sub);
    m.catat(net_ioctl_keluarga(10, 2) == -EAFNOSUPPORT, Group::Sub);
    m.catat(net_ioctl_rt(0) == -EINVAL, Group::Sub);
    m.catat(net_ioctl_rt(1) == 0, Group::Sub);
    m.catat(net_ioctl_perangkat_ada(0) == -ENODEV, Group::Sub);
    m.catat(net_ioctl_perangkat_ada(1) == 0, Group::Sub);
    m.catat(net_ioctl_indeks(0) == -EINVAL, Group::Sub);
    m.catat(net_ioctl_indeks(1) == 0, Group::Sub);
    m.catat(net_ioctl_tak() == -EINVAL, Group::Sub);
    m.catat(net_bind_sudah(1) == -EINVAL, Group::Sub);
    m.catat(net_bind_sudah(0) == 0, Group::Sub);
    m.catat(net_bind_keluarga(2, 2) == 0, Group::Sub);
    m.catat(net_bind_keluarga(10, 2) == -EINVAL, Group::Sub);
    m.catat(net_hubung_sudah(1) == -EISCONN, Group::Sub);
    m.catat(net_hubung_sudah(0) == 0, Group::Sub);
    m.catat(net_hubung_keluarga(2, 2) == 0, Group::Sub);
    m.catat(net_hubung_keluarga(10, 2) == -EINVAL, Group::Sub);
    m.catat(net_port(80, 0) == -EACCES, Group::Sub);
    m.catat(net_port(80, 1) == 0, Group::Sub);
    m.catat(net_port(1024, 0) == 0, Group::Sub);
    m.catat(net_port(0, 0) == 0, Group::Sub);
    m.catat(net_kirim_keluarga(2, 2) == 0, Group::Sub);
    m.catat(net_kirim_keluarga(10, 2) == -EAFNOSUPPORT, Group::Sub);
    m.catat(net_kirim_belum(1, 1) == -EPIPE, Group::Sub);
    m.catat(net_kirim_belum(1, 0) == 0, Group::Sub);
    m.catat(net_kirim_belum(0, 1) == 0, Group::Sub);

    // --- kontrol NEGATIF --------------------------------------------------
    {
        // EACCES pada port == 1024 harus TIDAK terjadi (`<`, bukan `<=`).
        m.catat(net_port(1024, 0) == 0, Group::Sub);
        m.catat(net_port_salah(1024, 0) == -EACCES, Group::Sub);
        m.catat(net_port(1024, 0) != net_port_salah(1024, 0), Group::Sub);
        m.catat(net_port(80, 0) == net_port_salah(80, 0), Group::Sub);

        // EAFNOSUPPORT (bukan EINVAL) pada keluarga ioctl yang salah.
        m.catat(net_ioctl_keluarga(10, 2) == -EAFNOSUPPORT, Group::Sub);
        m.catat(net_ioctl_keluarga_salah(10, 2) == -EINVAL, Group::Sub);
        m.catat(
            net_ioctl_keluarga(10, 2) != net_ioctl_keluarga_salah(10, 2),
            Group::Sub,
        );
        m.catat(
            net_ioctl_keluarga(2, 2) == net_ioctl_keluarga_salah(2, 2),
            Group::Sub,
        );

        // Determinisme.
        let mut stabil = true;
        for _ in 0..1000 {
            if net_ioctl_tak() != -EINVAL {
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

    // #1/#15: verdict tanpa argumen.
    ukur!(Group::Teras, net_sokopt_tak(), -ENOPROTOOPT);
    ukur!(Group::Sub, net_ioctl_tak(), -EINVAL);

    // #2/#3: pasangan ukuran.
    for (u, h) in [(0u64, 0u64), (4, 4), (4, 16), (16, 16), (15, 16), (17, 16)] {
        ukur!(Group::Teras, net_ukuran_persis(u, h), naif_ukuran_persis(u, h));
        ukur!(
            Group::Teras,
            net_ukuran_minimum(u, h),
            naif_ukuran_minimum(u, h)
        );
    }

    // #4/#6/#9/#13/#14/#16/#18: verdict boolean tunggal.
    for v in 0..2u32 {
        ukur!(
            Group::Teras,
            net_bindtodevice_ada(v),
            naif_bindtodevice_ada(v)
        );
        ukur!(
            Group::Teras,
            net_bindtodevice_kosong(v),
            naif_bindtodevice_kosong(v)
        );
        ukur!(Group::Teras, net_interface_any(v), naif_interface_any(v));
        ukur!(Group::Sub, net_ioctl_izin(v), naif_ioctl_izin(v));
        ukur!(Group::Sub, net_ioctl_rt(v), naif_ioctl_rt(v));
        ukur!(
            Group::Sub,
            net_ioctl_perangkat_ada(v),
            naif_ioctl_perangkat_ada(v)
        );
        ukur!(Group::Sub, net_ioctl_indeks(v), naif_ioctl_indeks(v));
        ukur!(Group::Sub, net_bind_sudah(v), naif_bind_sudah(v));
        ukur!(Group::Sub, net_hubung_sudah(v), naif_hubung_sudah(v));
        for w in 0..2u32 {
            ukur!(
                Group::Sub,
                net_kirim_belum(v, w),
                naif_kirim_belum(v, w)
            );
        }
    }

    // #5: tiga masukan boolean.
    for a in 0..2u32 {
        for b in 0..2u32 {
            for c in 0..2u32 {
                ukur!(
                    Group::Teras,
                    net_timestamp_dukung(a, b, c),
                    naif_timestamp(a, b, c)
                );
            }
        }
    }

    // #7/#8: nilai bertanda.
    for v in [-1i32, 0, 1, 2, 255, 256] {
        ukur!(Group::Teras, net_ttl_tos(v), naif_ttl_tos(v));
        ukur!(Group::Teras, net_loop(v), naif_loop(v));
    }

    // #11/#17/#19/#21: keluarga alamat (dua errno berbeda).
    for (af, inet, e) in [
        (2u32, 2u32, 0i32),
        (10, 2, -EAFNOSUPPORT),
        (0, 2, -EAFNOSUPPORT),
    ] {
        ukur!(
            Group::Sub,
            net_ioctl_keluarga(af, inet),
            naif_keluarga(af, inet, e)
        );
        ukur!(
            Group::Sub,
            net_kirim_keluarga(af, inet),
            naif_keluarga(af, inet, e)
        );
    }
    for (af, inet) in [(2u32, 2u32), (10, 2), (0, 2)] {
        ukur!(
            Group::Sub,
            net_bind_keluarga(af, inet),
            naif_keluarga(af, inet, -EINVAL)
        );
        ukur!(
            Group::Sub,
            net_hubung_keluarga(af, inet),
            naif_keluarga(af, inet, -EINVAL)
        );
    }

    // #20: port istimewa.
    for (p, su) in [(0u32, 0u32), (80, 0), (1023, 0), (1024, 0), (8080, 0), (80, 1)] {
        ukur!(Group::Sub, net_port(p, su), naif_port(p, su));
    }

    m.catat(diukur == DIFF_EXPECTED, Group::Teras);
    m.catat(beda == 0, Group::Teras);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosAdmisiNetReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        teras_checks: m.teras_checks,
        teras_passed: m.teras_passed,
        sub_checks: m.sub_checks,
        sub_passed: m.sub_passed,
        prod_sokopt_tak: klatos_net_counter(Counter::ProdSokoptTak as u32),
        prod_ukuran_persis: klatos_net_counter(Counter::ProdUkuranPersis as u32),
        prod_ukuran_minimum: klatos_net_counter(Counter::ProdUkuranMinimum as u32),
        prod_bindtodevice_ada: klatos_net_counter(Counter::ProdBindtodeviceAda as u32),
        prod_timestamp: klatos_net_counter(Counter::ProdTimestamp as u32),
        prod_bindtodevice_kosong: klatos_net_counter(Counter::ProdBindtodeviceKosong as u32),
        prod_ttl_tos: klatos_net_counter(Counter::ProdTtlTos as u32),
        prod_loop: klatos_net_counter(Counter::ProdLoop as u32),
        prod_interface_any: klatos_net_counter(Counter::ProdInterfaceAny as u32),
        prod_ioctl_izin: klatos_net_counter(Counter::ProdIoctlIzin as u32),
        prod_ioctl_keluarga: klatos_net_counter(Counter::ProdIoctlKeluarga as u32),
        prod_ioctl_rt: klatos_net_counter(Counter::ProdIoctlRt as u32),
        prod_ioctl_perangkat: klatos_net_counter(Counter::ProdIoctlPerangkat as u32),
        prod_ioctl_indeks: klatos_net_counter(Counter::ProdIoctlIndeks as u32),
        prod_ioctl_tak: klatos_net_counter(Counter::ProdIoctlTak as u32),
        prod_bind_sudah: klatos_net_counter(Counter::ProdBindSudah as u32),
        prod_bind_keluarga: klatos_net_counter(Counter::ProdBindKeluarga as u32),
        prod_hubung_sudah: klatos_net_counter(Counter::ProdHubungSudah as u32),
        prod_hubung_keluarga: klatos_net_counter(Counter::ProdHubungKeluarga as u32),
        prod_port: klatos_net_counter(Counter::ProdPort as u32),
        prod_kirim_keluarga: klatos_net_counter(Counter::ProdKirimKeluarga as u32),
        prod_kirim_belum: klatos_net_counter(Counter::ProdKirimBelum as u32),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status,
    }
}

/// Menjalankan matriks otoritas jaringan (batch 1).
#[no_mangle]
pub extern "C" fn klatos_net_selftest() -> KlatosAdmisiNetReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4E45_5441);
        assert!(EPERM == 1);
        assert!(EACCES == 13);
        assert!(EFAULT == 14);
        assert!(ENODEV == 19);
        assert!(EINVAL == 22);
        assert!(EPIPE == 32);
        assert!(EAFNOSUPPORT == 40);
        assert!(EISCONN == 48);
        assert!(ENOPROTOOPT == 62);
        assert!(ENOTSUP == 74);
        assert!(TTL_TOS_MAKS == 255);
        assert!(PORT_ISTIMEWA == 1024);
        assert!(Counter::ProdSokoptTak as usize == 0);
        assert!(Counter::ProdKirimBelum as usize == 21);
        assert!(N_COUNTER == 22);
        assert!(DIFF_EXPECTED == 74);

        assert!(net_sokopt_tak() == -ENOPROTOOPT);
        assert!(net_ukuran_persis(4, 16) == -EINVAL);
        assert!(net_ttl_tos(256) == -EINVAL);
        assert!(net_port(1024, 0) == 0);
        assert!(net_ioctl_keluarga(10, 2) == -EAFNOSUPPORT);

        // 8 (tag) + 30*4 (u32) + 4 (status) = 132 -> 136 dengan padding.
        assert!(core::mem::size_of::<KlatosAdmisiNetReport>() == 136);
        assert!(core::mem::align_of::<KlatosAdmisiNetReport>() == 8);
    };
}
