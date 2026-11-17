// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.60 — **OTORITAS ADMISI & KLASIFIKASI PAGE FAULT**.


use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan: "KLATPFAD".
pub const REPORT_TAG: u64 = 0x4B4C_4154_5046_4144;

// --- status `klatos_pf_admisi` ---------------------------------------------

/// Fault diadmisikan; lanjut ke klasifikasi jenis.
pub const ADM_OK: i32 = 0;
/// Fault user di Region non-user.
pub const ADM_USER_NONUSER: i32 = 1;
/// Fault kernel di Region user dengan SMAP aktif.
pub const ADM_KERNEL_SMAP: i32 = 2;
/// Instruction fetch di Region non-eksekutabel.
pub const ADM_NONEXEC: i32 = 3;
/// Baca di Region non-baca.
pub const ADM_NONREAD: i32 = 4;
/// Tulis di Region non-tulis.
pub const ADM_NONWRITE: i32 = 5;
/// Argumen tidak sah (bukan 0/1, atau `is_user` dan `is_kernel` keduanya 1).
pub const ADM_ARG: i32 = 6;

// --- jenis fault `klatos_pf_jenis` -----------------------------------------

/// Zero page (tulis ke halaman nol/lazy).
pub const JN_ZERO: i32 = 0;
/// Copy-on-write.
pub const JN_COW: i32 = 1;
/// Tulis ke Region inode.
pub const JN_INODE_WRITE: i32 = 2;
/// Baca dari Region inode.
pub const JN_INODE_READ: i32 = 3;
/// Tidak ada alasan (C++ memeriksa PTE, lalu mungkin `PANIC`).
pub const JN_NONE: i32 = 4;
/// Argumen tidak sah.
pub const JN_ARG: i32 = 5;

// --- aksi `klatos_pf_sinyal` -----------------------------------------------

/// Lanjutkan eksekusi (response `Continue`).
pub const AKSI_LANJUT: u32 = 0;
/// Kirim sinyal ke benang saat ini.
pub const AKSI_SINYAL: u32 = 1;
/// Crash (panggil `handle_crash`).
pub const AKSI_CRASH: u32 = 2;
/// Argumen tidak sah.
pub const AKSI_ARG: u32 = 3;

// --- sinyal & si_code ------------------------------------------------------

/// `SIGBUS`.
pub const SINYAL_SIGBUS: u32 = 7;
/// `SIGSEGV`.
pub const SINYAL_SIGSEGV: u32 = 11;
/// `SEGV_MAPERR`.
pub const SEGV_MAPERR: i32 = 1;
/// `SEGV_ACCERR`.
pub const SEGV_ACCERR: i32 = 2;
/// `BUS_ADRERR`.
pub const BUS_ADRERR: i32 = 2;

// --- respons page fault (cermin `PageFaultResponse`) -----------------------

/// `PageFaultResponse::ShouldCrash`.
pub const RESP_CRASH: u32 = 0;
/// `PageFaultResponse::BusError`.
pub const RESP_BUS: u32 = 1;
/// `PageFaultResponse::OutOfMemory`.
pub const RESP_OOM: u32 = 2;
/// `PageFaultResponse::Continue`.
pub const RESP_CONTINUE: u32 = 3;

// --- tipe fault (cermin `PageFault::Type`) ---------------------------------

/// `PageFault::Type::PageNotPresent`.
pub const FT_NOTPRESENT: u32 = 0;
/// `PageFault::Type::ProtectionViolation`.
pub const FT_PROTVIOL: u32 = 1;
/// `PageFault::Type::Unknown`.
pub const FT_UNKNOWN: u32 = 2;

// --- akses coredump --------------------------------------------------------

/// `fault_access` = "Execute".
pub const AKSES_EXECUTE: u32 = 0;
/// `fault_access` = "Read".
pub const AKSES_READ: u32 = 1;
/// `fault_access` = "Write".
pub const AKSES_WRITE: u32 = 2;
/// Argumen tidak sah.
pub const AKSES_ARG: u32 = 0xFFFF_FFFF;

/// Penghitung audit produksi.
#[repr(u32)]
pub enum Counter {
    /// Fault diadmisikan.
    AdmisiOk = 0,
    /// Fault ditolak oleh gerbang admisi.
    AdmisiTolak = 1,
    /// Klasifikasi: zero page.
    JenisZero = 2,
    /// Klasifikasi: copy-on-write.
    JenisCow = 3,
    /// Klasifikasi: inode (baca atau tulis).
    JenisInode = 4,
    /// Klasifikasi: tanpa alasan.
    JenisNone = 5,
    /// Sinyal dikirim.
    Sinyal = 6,
    /// Crash dipilih.
    Crash = 7,
    /// Argumen tidak sah.
    ArgSalah = 8,
}

/// Jumlah penghitung.
pub const N_COUNTER: usize = 9;

static COUNTER: [AtomicU32; N_COUNTER] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

#[inline]
fn bump(c: Counter) {
    COUNTER[c as usize].fetch_add(1, Ordering::Relaxed);
}

/// Hasil `klatos_pf_sinyal` (16 byte).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosPfSinyal {
    /// `AKSI_*`.
    pub aksi: u32,
    /// Nomor sinyal (hanya bila `aksi == AKSI_SINYAL` atau `AKSI_CRASH`).
    pub sinyal: u32,
    /// `si_code` (hanya bila `aksi == AKSI_SINYAL`).
    pub si_code: i32,
    /// 1 bila respons adalah `OutOfMemory` (dipakai `handle_crash`).
    pub oom: u32,
}

/// Laporan selftest (72 byte).
#[repr(C)]
pub struct KlatosPfReport {
    /// `REPORT_TAG`.
    pub tag: u64,
    /// Jumlah pemeriksaan struktural.
    pub checks: u32,
    /// Pemeriksaan struktural yang lulus.
    pub passed: u32,
    /// Jumlah iterasi diferensial.
    pub peta_checks: u32,
    /// Iterasi diferensial yang cocok dengan oracle.
    pub peta_passed: u32,
    /// Jumlah kasus negatif.
    pub neg_checks: u32,
    /// Kasus negatif yang lulus.
    pub neg_passed: u32,
    /// Produksi: admisi OK.
    pub prod_admisi_ok: u32,
    /// Produksi: admisi ditolak.
    pub prod_admisi_tolak: u32,
    /// Produksi: total klasifikasi jenis.
    pub prod_jenis: u32,
    /// Produksi: total sinyal.
    pub prod_sinyal: u32,
    /// Produksi: total crash.
    pub prod_crash: u32,
    /// Produksi: total argumen salah.
    pub prod_arg_salah: u32,
    /// Diferensial: jumlah kasus.
    pub diferensial_ukur: u32,
    /// Diferensial: jumlah ketidakcocokan.
    pub diferensial_beda: u32,
    /// 0 = lulus.
    pub status: i32,
}

#[inline]
fn b(x: u32) -> bool {
    x != 0
}

#[inline]
fn sah(x: u32) -> bool {
    x <= 1
}

/// Gerbang admisi `Region::handle_fault` (batas + izin).
///
/// Mengembalikan [`ADM_OK`] bila fault boleh diproses lebih lanjut, atau kode
/// alasan (`ADM_USER_NONUSER`..`ADM_NONWRITE`) yang di C++ berarti
/// `PageFaultResponse::ShouldCrash`. [`ADM_ARG`] untuk argumen tidak sah.
#[inline]
pub fn admisi(
    fault_is_user: u32,
    fault_is_kernel: u32,
    was_smap_disabled: u32,
    region_is_user: u32,
    is_instruction_fetch: u32,
    region_executable: u32,
    is_read: u32,
    region_readable: u32,
    is_write: u32,
    region_writable: u32,
) -> i32 {
    if !sah(fault_is_user)
        || !sah(fault_is_kernel)
        || !sah(was_smap_disabled)
        || !sah(region_is_user)
        || !sah(is_instruction_fetch)
        || !sah(region_executable)
        || !sah(is_read)
        || !sah(region_readable)
        || !sah(is_write)
        || !sah(region_writable)
    {
        return ADM_ARG;
    }
    if b(fault_is_user) && b(fault_is_kernel) {
        return ADM_ARG;
    }

    if b(fault_is_user) && !b(region_is_user) {
        return ADM_USER_NONUSER;
    }
    if b(fault_is_kernel) && b(region_is_user) && !b(was_smap_disabled) {
        return ADM_KERNEL_SMAP;
    }
    // CATATAN: instruction fetch juga "read" di x86-64; cek ini harus lebih dulu.
    if b(is_instruction_fetch) && !b(region_executable) {
        return ADM_NONEXEC;
    }
    if b(is_read) && !b(region_readable) {
        return ADM_NONREAD;
    }
    if b(is_write) && !b(region_writable) {
        return ADM_NONWRITE;
    }
    ADM_OK
}

/// Klasifikasi jenis fault `Region::handle_fault`.
#[inline]
pub fn jenis(
    is_write: u32,
    page_zero_or_lazy: u32,
    should_cow: u32,
    vmobject_is_inode: u32,
    is_read: u32,
) -> i32 {
    if !sah(is_write)
        || !sah(page_zero_or_lazy)
        || !sah(should_cow)
        || !sah(vmobject_is_inode)
        || !sah(is_read)
    {
        return JN_ARG;
    }

    if b(is_write) {
        if b(page_zero_or_lazy) {
            return JN_ZERO;
        }
        if b(should_cow) {
            return JN_COW;
        }
        if b(vmobject_is_inode) {
            return JN_INODE_WRITE;
        }
    }
    if b(is_read) && b(vmobject_is_inode) {
        return JN_INODE_READ;
    }
    JN_NONE
}

/// Klasifikasi sinyal `PageFault::handle`.
#[inline]
pub fn sinyal(
    response: u32,
    has_thread: u32,
    has_sigbus: u32,
    has_sigsegv: u32,
    fault_type: u32,
) -> KlatosPfSinyal {
    if !sah(has_thread)
        || !sah(has_sigbus)
        || !sah(has_sigsegv)
        || response > RESP_CONTINUE
        || fault_type > FT_UNKNOWN
    {
        return KlatosPfSinyal { aksi: AKSI_ARG, sinyal: 0, si_code: 0, oom: 0 };
    }

    if response == RESP_CONTINUE {
        return KlatosPfSinyal { aksi: AKSI_LANJUT, sinyal: 0, si_code: 0, oom: 0 };
    }

    if response == RESP_BUS && b(has_thread) && b(has_sigbus) {
        return KlatosPfSinyal { aksi: AKSI_SINYAL, sinyal: SINYAL_SIGBUS, si_code: BUS_ADRERR, oom: 0 };
    }

    // Bug-for-bug: cabang ini TIDAK mengecualikan BusError.
    if response != RESP_OOM && b(has_thread) && b(has_sigsegv) {
        let code = if fault_type == FT_PROTVIOL { SEGV_ACCERR } else { SEGV_MAPERR };
        return KlatosPfSinyal { aksi: AKSI_SINYAL, sinyal: SINYAL_SIGSEGV, si_code: code, oom: 0 };
    }

    let sig = if response == RESP_BUS { SINYAL_SIGBUS } else { SINYAL_SIGSEGV };
    let oom = if response == RESP_OOM { 1 } else { 0 };
    KlatosPfSinyal { aksi: AKSI_CRASH, sinyal: sig, si_code: 0, oom }
}

/// Properti coredump `fault_access`.
#[inline]
pub fn akses(is_instruction_fetch: u32, is_write: u32) -> u32 {
    if !sah(is_instruction_fetch) || !sah(is_write) {
        return AKSES_ARG;
    }
    if b(is_instruction_fetch) {
        AKSES_EXECUTE
    } else if b(is_write) {
        AKSES_WRITE
    } else {
        AKSES_READ
    }
}

// ---------------------------------------------------------------------------
// Permukaan C-ABI
// ---------------------------------------------------------------------------

/// C-ABI [`admisi`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_pf_admisi(
    fault_is_user: u32,
    fault_is_kernel: u32,
    was_smap_disabled: u32,
    region_is_user: u32,
    is_instruction_fetch: u32,
    region_executable: u32,
    is_read: u32,
    region_readable: u32,
    is_write: u32,
    region_writable: u32,
) -> i32 {
    let r = admisi(
        fault_is_user,
        fault_is_kernel,
        was_smap_disabled,
        region_is_user,
        is_instruction_fetch,
        region_executable,
        is_read,
        region_readable,
        is_write,
        region_writable,
    );
    if r == ADM_OK {
        bump(Counter::AdmisiOk);
    } else if r == ADM_ARG {
        bump(Counter::ArgSalah);
    } else {
        bump(Counter::AdmisiTolak);
    }
    r
}

/// C-ABI [`jenis`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_pf_jenis(
    is_write: u32,
    page_zero_or_lazy: u32,
    should_cow: u32,
    vmobject_is_inode: u32,
    is_read: u32,
) -> i32 {
    let r = jenis(is_write, page_zero_or_lazy, should_cow, vmobject_is_inode, is_read);
    match r {
        JN_ZERO => bump(Counter::JenisZero),
        JN_COW => bump(Counter::JenisCow),
        JN_INODE_WRITE | JN_INODE_READ => bump(Counter::JenisInode),
        JN_NONE => bump(Counter::JenisNone),
        _ => bump(Counter::ArgSalah),
    }
    r
}

/// C-ABI [`sinyal`] + pencatatan produksi.
#[no_mangle]
pub extern "C" fn klatos_pf_sinyal(
    response: u32,
    has_thread: u32,
    has_sigbus: u32,
    has_sigsegv: u32,
    fault_type: u32,
) -> KlatosPfSinyal {
    let r = sinyal(response, has_thread, has_sigbus, has_sigsegv, fault_type);
    if r.aksi == AKSI_SINYAL {
        bump(Counter::Sinyal);
    } else if r.aksi == AKSI_CRASH {
        bump(Counter::Crash);
    } else if r.aksi == AKSI_ARG {
        bump(Counter::ArgSalah);
    }
    r
}

/// C-ABI [`akses`].
#[no_mangle]
pub extern "C" fn klatos_pf_akses(is_instruction_fetch: u32, is_write: u32) -> u32 {
    akses(is_instruction_fetch, is_write)
}

/// Baca penghitung audit produksi.
#[no_mangle]
pub extern "C" fn klatos_pf_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTER[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Nolkan seluruh penghitung; mengembalikan jumlah yang dinolkan.
#[no_mangle]
pub extern "C" fn klatos_pf_reset() -> u32 {
    let mut n = 0u32;
    let mut i = 0usize;
    while i < N_COUNTER {
        COUNTER[i].store(0, Ordering::Relaxed);
        n += 1;
        i += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// Selftest / matriks
// ---------------------------------------------------------------------------

struct Matriks {
    checks: u32,
    passed: u32,
    peta_checks: u32,
    peta_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
    ukur: u32,
    beda: u32,
}

impl Matriks {
    const fn new() -> Self {
        Self { checks: 0, passed: 0, peta_checks: 0, peta_passed: 0, neg_checks: 0, neg_passed: 0, ukur: 0, beda: 0 }
    }
    #[inline]
    fn cek(&mut self, ok: bool) {
        self.checks += 1;
        if ok {
            self.passed += 1;
        }
    }
    #[inline]
    fn cek_neg(&mut self, ok: bool) {
        self.neg_checks += 1;
        if ok {
            self.neg_passed += 1;
        }
    }
    #[inline]
    fn diferensial(&mut self, ok: bool) {
        self.ukur += 1;
        self.peta_checks += 1;
        if ok {
            self.peta_passed += 1;
        } else {
            self.beda += 1;
        }
    }
}

/// Oracle independen: gerbang admisi (implementasi kedua).
fn o_admisi(
    fu: u32,
    fk: u32,
    smap: u32,
    ru: u32,
    ife: u32,
    re: u32,
    ir: u32,
    rr: u32,
    iw: u32,
    rw: u32,
) -> i32 {
    if fu > 1 || fk > 1 || smap > 1 || ru > 1 || ife > 1 || re > 1 || ir > 1 || rr > 1 || iw > 1 || rw > 1 {
        return ADM_ARG;
    }
    if fu == 1 && fk == 1 {
        return ADM_ARG;
    }
    if fu == 1 && ru == 0 {
        return ADM_USER_NONUSER;
    }
    if fk == 1 && ru == 1 && smap == 0 {
        return ADM_KERNEL_SMAP;
    }
    if ife == 1 && re == 0 {
        return ADM_NONEXEC;
    }
    if ir == 1 && rr == 0 {
        return ADM_NONREAD;
    }
    if iw == 1 && rw == 0 {
        return ADM_NONWRITE;
    }
    ADM_OK
}

/// Oracle independen: jenis fault (implementasi kedua).
fn o_jenis(iw: u32, zl: u32, cow: u32, ino: u32, ir: u32) -> i32 {
    if iw > 1 || zl > 1 || cow > 1 || ino > 1 || ir > 1 {
        return JN_ARG;
    }
    if iw == 1 && zl == 1 {
        return JN_ZERO;
    }
    if iw == 1 && cow == 1 {
        return JN_COW;
    }
    if iw == 1 && ino == 1 {
        return JN_INODE_WRITE;
    }
    if ir == 1 && ino == 1 {
        return JN_INODE_READ;
    }
    JN_NONE
}

/// Oracle independen: sinyal (implementasi kedua, tanpa `struct`).
fn o_sinyal(resp: u32, th: u32, sb: u32, sg: u32, ft: u32) -> (u32, u32, i32, u32) {
    if th > 1 || sb > 1 || sg > 1 || resp > 3 || ft > 2 {
        return (AKSI_ARG, 0, 0, 0);
    }
    if resp == 3 {
        return (AKSI_LANJUT, 0, 0, 0);
    }
    if resp == 1 && th == 1 && sb == 1 {
        return (AKSI_SINYAL, SINYAL_SIGBUS, BUS_ADRERR, 0);
    }
    if resp != 2 && th == 1 && sg == 1 {
        let c = if ft == 1 { SEGV_ACCERR } else { SEGV_MAPERR };
        return (AKSI_SINYAL, SINYAL_SIGSEGV, c, 0);
    }
    let s = if resp == 1 { SINYAL_SIGBUS } else { SINYAL_SIGSEGV };
    let o = if resp == 2 { 1 } else { 0 };
    (AKSI_CRASH, s, 0, o)
}

/// Oracle independen: akses.
fn o_akses(ife: u32, iw: u32) -> u32 {
    if ife > 1 || iw > 1 {
        return AKSES_ARG;
    }
    if ife == 1 {
        AKSES_EXECUTE
    } else if iw == 1 {
        AKSES_WRITE
    } else {
        AKSES_READ
    }
}

const PETA_ADMISI: [([u32; 10], i32); 12] = [
    ([1, 0, 0, 1, 0, 1, 0, 1, 1, 1], ADM_OK),
    ([1, 0, 0, 0, 0, 1, 0, 1, 1, 1], ADM_USER_NONUSER),
    ([0, 1, 0, 1, 0, 1, 0, 1, 1, 1], ADM_KERNEL_SMAP),
    ([0, 1, 1, 1, 0, 1, 0, 1, 1, 1], ADM_OK),
    ([1, 0, 0, 1, 1, 0, 1, 1, 0, 0], ADM_NONEXEC),
    ([1, 0, 0, 1, 0, 1, 1, 0, 0, 0], ADM_NONREAD),
    ([1, 0, 0, 1, 0, 1, 0, 1, 1, 0], ADM_NONWRITE),
    ([1, 0, 0, 1, 1, 0, 1, 0, 1, 0], ADM_NONEXEC),
    ([1, 1, 0, 1, 0, 1, 0, 1, 0, 0], ADM_ARG),
    ([2, 0, 0, 1, 0, 1, 0, 1, 0, 0], ADM_ARG),
    ([0, 0, 0, 0, 0, 0, 0, 0, 0, 0], ADM_OK),
    ([0, 0, 1, 0, 0, 0, 1, 0, 1, 0], ADM_NONREAD),
];

const PETA_JENIS: [([u32; 5], i32); 8] = [
    ([1, 1, 0, 0, 0], JN_ZERO),
    ([1, 0, 1, 0, 0], JN_COW),
    ([1, 0, 0, 1, 0], JN_INODE_WRITE),
    ([0, 0, 0, 1, 1], JN_INODE_READ),
    ([0, 0, 0, 0, 1], JN_NONE),
    ([0, 0, 0, 0, 0], JN_NONE),
    ([1, 0, 0, 0, 0], JN_NONE),
    ([1, 1, 1, 1, 1], JN_ZERO),
];

const PETA_SINYAL: [([u32; 5], u32, u32, i32, u32); 8] = [
    ([3, 1, 0, 0, 0], AKSI_LANJUT, 0, 0, 0),
    ([1, 1, 1, 0, 0], AKSI_SINYAL, SINYAL_SIGBUS, BUS_ADRERR, 0),
    ([1, 1, 0, 1, 0], AKSI_SINYAL, SINYAL_SIGSEGV, SEGV_MAPERR, 0),
    ([1, 1, 0, 1, 1], AKSI_SINYAL, SINYAL_SIGSEGV, SEGV_ACCERR, 0),
    ([0, 1, 0, 1, 1], AKSI_SINYAL, SINYAL_SIGSEGV, SEGV_ACCERR, 0),
    ([0, 1, 0, 0, 0], AKSI_CRASH, SINYAL_SIGSEGV, 0, 0),
    ([1, 1, 0, 0, 0], AKSI_CRASH, SINYAL_SIGBUS, 0, 0),
    ([2, 1, 1, 1, 1], AKSI_CRASH, SINYAL_SIGSEGV, 0, 1),
];

const PETA_AKSES: [([u32; 2], u32); 4] = [
    ([1, 1], AKSES_EXECUTE),
    ([0, 1], AKSES_WRITE),
    ([0, 0], AKSES_READ),
    ([1, 0], AKSES_EXECUTE),
];

/// Jalankan matriks lengkap; mengembalikan laporan.
pub fn jalankan_matriks() -> KlatosPfReport {
    let mut m = Matriks::new();

    // G1 — konstanta.
    m.cek(ADM_OK != ADM_USER_NONUSER && ADM_USER_NONUSER != ADM_KERNEL_SMAP);
    m.cek(ADM_NONEXEC != ADM_NONREAD && ADM_NONREAD != ADM_NONWRITE && ADM_NONWRITE != ADM_ARG);
    m.cek(JN_ZERO != JN_COW && JN_COW != JN_INODE_WRITE && JN_INODE_WRITE != JN_INODE_READ);
    m.cek(JN_INODE_READ != JN_NONE && JN_NONE != JN_ARG);
    m.cek(AKSI_LANJUT != AKSI_SINYAL && AKSI_SINYAL != AKSI_CRASH && AKSI_CRASH != AKSI_ARG);
    m.cek(SINYAL_SIGBUS == 7 && SINYAL_SIGSEGV == 11);
    m.cek(SEGV_MAPERR == 1 && SEGV_ACCERR == 2 && BUS_ADRERR == 2);
    m.cek(RESP_CRASH == 0 && RESP_BUS == 1 && RESP_OOM == 2 && RESP_CONTINUE == 3);
    m.cek(FT_NOTPRESENT == 0 && FT_PROTVIOL == 1 && FT_UNKNOWN == 2);
    m.cek(N_COUNTER == 9);

    // G2 — admisi: presedensi urutan.
    // user+nonuser menang atas nonexec.
    m.cek(admisi(1, 0, 0, 0, 1, 0, 1, 0, 1, 0) == ADM_USER_NONUSER);
    // SMAP menang atas nonexec (region user, kernel fault, smap aktif).
    m.cek(admisi(0, 1, 0, 1, 1, 0, 1, 0, 0, 0) == ADM_KERNEL_SMAP);
    // instruction fetch menang atas read (keduanya 1, region tak-eksekutabel & tak-baca).
    m.cek(admisi(1, 0, 0, 1, 1, 0, 1, 0, 0, 0) == ADM_NONEXEC);
    // read menang atas write.
    m.cek(admisi(1, 0, 0, 1, 0, 1, 1, 0, 1, 0) == ADM_NONREAD);

    // G3 — jenis: urutan zero > cow > inode.
    m.cek(jenis(1, 1, 1, 1, 1) == JN_ZERO);
    m.cek(jenis(1, 0, 1, 1, 1) == JN_COW);
    m.cek(jenis(1, 0, 0, 1, 1) == JN_INODE_WRITE);
    m.cek(jenis(0, 0, 0, 1, 1) == JN_INODE_READ);
    m.cek(jenis(0, 0, 0, 0, 1) == JN_NONE);
    m.cek(jenis(1, 0, 0, 0, 0) == JN_NONE);

    // G4 — sinyal: termasuk jebakan BusError tanpa SIGBUS tapi ada SIGSEGV.
    let s_bus_segv = sinyal(1, 1, 0, 1, 0);
    m.cek(s_bus_segv.aksi == AKSI_SINYAL && s_bus_segv.sinyal == SINYAL_SIGSEGV);
    let s_bus_none = sinyal(1, 1, 0, 0, 0);
    m.cek(s_bus_none.aksi == AKSI_CRASH && s_bus_none.sinyal == SINYAL_SIGBUS);
    let s_oom = sinyal(2, 1, 1, 1, 0);
    m.cek(s_oom.aksi == AKSI_CRASH && s_oom.sinyal == SINYAL_SIGSEGV && s_oom.oom == 1);
    let s_cont = sinyal(3, 1, 1, 1, 1);
    m.cek(s_cont.aksi == AKSI_LANJUT);
    let s_no_thread = sinyal(0, 0, 1, 1, 0);
    m.cek(s_no_thread.aksi == AKSI_CRASH);

    // G5 — akses.
    m.cek(akses(1, 1) == AKSES_EXECUTE);
    m.cek(akses(0, 1) == AKSES_WRITE);
    m.cek(akses(0, 0) == AKSES_READ);

    // G6 — peta tetap.
    for (i, (arg, exp)) in PETA_ADMISI.iter().enumerate() {
        let _ = i;
        let got = admisi(arg[0], arg[1], arg[2], arg[3], arg[4], arg[5], arg[6], arg[7], arg[8], arg[9]);
        m.cek(got == *exp);
    }
    for (arg, exp) in PETA_JENIS.iter() {
        m.cek(jenis(arg[0], arg[1], arg[2], arg[3], arg[4]) == *exp);
    }
    for (arg, a, s, c, o) in PETA_SINYAL.iter() {
        let got = sinyal(arg[0], arg[1], arg[2], arg[3], arg[4]);
        m.cek(got.aksi == *a && got.sinyal == *s && got.si_code == *c && got.oom == *o);
    }
    for (arg, exp) in PETA_AKSES.iter() {
        m.cek(akses(arg[0], arg[1]) == *exp);
    }

    // G7 — diferensial acak vs oracle (LCG).
    let mut state: u64 = 0x5DEE_CE66_D1CE_4E5B;
    let n: u32 = 60_000;
    let mut i = 0u32;
    while i < n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = (state >> 33) as u32;
        let fu = r & 1;
        let fk = (r >> 1) & 1;
        let smap = (r >> 2) & 1;
        let ru = (r >> 3) & 1;
        let ife = (r >> 4) & 1;
        let re = (r >> 5) & 1;
        let ir = (r >> 6) & 1;
        let rr = (r >> 7) & 1;
        let iw = (r >> 8) & 1;
        let rw = (r >> 9) & 1;
        m.diferensial(admisi(fu, fk, smap, ru, ife, re, ir, rr, iw, rw) == o_admisi(fu, fk, smap, ru, ife, re, ir, rr, iw, rw));

        let zl = (r >> 10) & 1;
        let cow = (r >> 11) & 1;
        let ino = (r >> 12) & 1;
        let ir2 = (r >> 13) & 1;
        m.diferensial(jenis(iw, zl, cow, ino, ir2) == o_jenis(iw, zl, cow, ino, ir2));

        let resp = (r >> 14) % 4;
        let th = (r >> 16) & 1;
        let sb = (r >> 17) & 1;
        let sg = (r >> 18) & 1;
        let ft = (r >> 19) % 3;
        let gs = sinyal(resp, th, sb, sg, ft);
        let (oa, os_, oc, oo) = o_sinyal(resp, th, sb, sg, ft);
        m.diferensial(gs.aksi == oa && gs.sinyal == os_ && gs.si_code == oc && gs.oom == oo);

        m.diferensial(akses(ife, iw) == o_akses(ife, iw));

        i += 1;
    }

    // G8 — determinisme: oracle yang sama dua kali pada seed tetap.
    let mut state2: u64 = 0x1234_5678_9ABC_DEF0;
    let mut sama = true;
    let mut j = 0u32;
    while j < 4096 {
        state2 = state2.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = (state2 >> 33) as u32;
        let a = admisi(r & 1, (r >> 1) & 1, (r >> 2) & 1, (r >> 3) & 1, (r >> 4) & 1, (r >> 5) & 1, (r >> 6) & 1, (r >> 7) & 1, (r >> 8) & 1, (r >> 9) & 1);
        let b2 = admisi(r & 1, (r >> 1) & 1, (r >> 2) & 1, (r >> 3) & 1, (r >> 4) & 1, (r >> 5) & 1, (r >> 6) & 1, (r >> 7) & 1, (r >> 8) & 1, (r >> 9) & 1);
        if a != b2 {
            sama = false;
        }
        j += 1;
    }
    m.cek(sama);

    // G9 — negatif.
    m.cek_neg(admisi(2, 0, 0, 1, 0, 1, 0, 1, 0, 0) == ADM_ARG);
    m.cek_neg(admisi(0, 2, 0, 1, 0, 1, 0, 1, 0, 0) == ADM_ARG);
    m.cek_neg(admisi(1, 1, 0, 1, 0, 1, 0, 1, 0, 0) == ADM_ARG);
    m.cek_neg(admisi(0, 0, 0, 0, 0, 0, 0, 0, 3, 0) == ADM_ARG);
    m.cek_neg(jenis(2, 0, 0, 0, 0) == JN_ARG);
    m.cek_neg(sinyal(4, 1, 0, 0, 0).aksi == AKSI_ARG);
    m.cek_neg(sinyal(0, 1, 0, 0, 3).aksi == AKSI_ARG);
    m.cek_neg(sinyal(0, 2, 0, 0, 0).aksi == AKSI_ARG);
    m.cek_neg(akses(0, 2) == AKSES_ARG);

    // Produksi (dibaca ulang oleh init.cpp lewat klatos_pf_counter).
    let prod_admisi_ok = COUNTER[Counter::AdmisiOk as usize].load(Ordering::Relaxed);
    let prod_admisi_tolak = COUNTER[Counter::AdmisiTolak as usize].load(Ordering::Relaxed);
    let prod_jenis = COUNTER[Counter::JenisZero as usize].load(Ordering::Relaxed)
        + COUNTER[Counter::JenisCow as usize].load(Ordering::Relaxed)
        + COUNTER[Counter::JenisInode as usize].load(Ordering::Relaxed)
        + COUNTER[Counter::JenisNone as usize].load(Ordering::Relaxed);
    let prod_sinyal = COUNTER[Counter::Sinyal as usize].load(Ordering::Relaxed);
    let prod_crash = COUNTER[Counter::Crash as usize].load(Ordering::Relaxed);
    let prod_arg_salah = COUNTER[Counter::ArgSalah as usize].load(Ordering::Relaxed);

    let status = if m.passed == m.checks && m.peta_passed == m.peta_checks && m.neg_passed == m.neg_checks && m.beda == 0 {
        0
    } else {
        1
    };

    KlatosPfReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        prod_admisi_ok,
        prod_admisi_tolak,
        prod_jenis,
        prod_sinyal,
        prod_crash,
        prod_arg_salah,
        diferensial_ukur: m.ukur,
        diferensial_beda: m.beda,
        status,
    }
}

/// C-ABI selftest.
#[no_mangle]
pub extern "C" fn klatos_pf_selftest() -> KlatosPfReport {
    jalankan_matriks()
}

/// Pemeriksaan ukuran tipe pada waktu kompilasi.
pub mod conformance {
    use super::*;
    const _: () = {
        assert!(core::mem::size_of::<KlatosPfSinyal>() == 16);
        assert!(core::mem::size_of::<KlatosPfReport>() == 72);
    };
}
