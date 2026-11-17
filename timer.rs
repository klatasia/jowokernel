// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas **tick timer produksi** Jowo (P1.06) — MODEL B.



use core::sync::atomic::{AtomicU32, Ordering};

// ---------------------------------------------------------------------------
// Konstanta — cermin C++ yang mengikat
// ---------------------------------------------------------------------------

/// Basis vektor IRQ generik. Cermin `IRQ_VECTOR_BASE`
/// (`Jowo/Arsitektur/x86_64/CPU.h:21`).
pub const IRQ_VECTOR_BASE: u32 = 0x50;

/// Nomor IRQ timer Local APIC. Cermin `IRQ_APIC_TIMER`
/// (`Jowo/Arsitektur/x86_64/Interrupts/APIC.cpp:30` = `0xfc - IRQ_VECTOR_BASE`).
pub const IRQ_APIC_TIMER: u32 = 0xfc - IRQ_VECTOR_BASE;

/// Vektor IDT timer Local APIC. Cermin `APIC_LVT(IRQ_APIC_TIMER +
/// IRQ_VECTOR_BASE, 0)` (`APIC.cpp:626`). Inilah vektor yang BENAR-BENAR
/// datang pada sistem ini (x86_64).
pub const VEKTOR_APIC_TIMER: u32 = IRQ_APIC_TIMER + IRQ_VECTOR_BASE;

/// Nomor interupsi (**INTID**) timer ARMv8 EL1 virtual pada GIC (aarch64).
///
/// Turunan, bukan tebakan (lihat
/// `Build/audit/MIGRASI-20261018/AARCH64-RUNTIME-BOOT-FINDING.md`):
///
/// * Node `timer` pada DeviceTree mesin QEMU `virt` — dan pada Raspberry Pi —
///   berbunyi `interrupts = <1 13 …>, <1 14 …>, <1 11 …>, <1 10 …>`.
/// * KLATOS memakai indeks 2 = `DeviceTreeTimerInterruptIndex::EL1Virtual`
///   (`KlatKernel/Efek/Arsitektur/aarch64/Time/ARMv8Timer.cpp`).
/// * `GICv2::translate_interrupt_specifier_to_interrupt_number`
///   (`KlatKernel/Efek/Arsitektur/aarch64/Interrupts/GICv2.cpp:138`)
///   memetakan PPI nomor 11 menjadi `11 + 16 = 27`.
#[cfg(target_arch = "aarch64")]
pub const VEKTOR_ARM_V8_TIMER: u32 = 27;

/// Vektor timer yang **terdaftar** pada arsitektur tempat modul ini dibangun.
///
/// * `x86_64`: vektor IDT timer Local APIC (`0xfc`) — **nilai, tipe, dan
///   perilaku tidak berubah** dari P1.06 semula; ini bidang ABI.
/// * `aarch64`: INTID GIC timer ARMv8 EL1 virtual (`27`).
#[cfg(target_arch = "x86_64")]
pub const VEKTOR_TIMER_TERDAFTAR: u32 = VEKTOR_APIC_TIMER;
#[cfg(target_arch = "aarch64")]
pub const VEKTOR_TIMER_TERDAFTAR: u32 = VEKTOR_ARM_V8_TIMER;

/// Nomor IRQ timer yang **terdaftar** pada arsitektur ini.
///
/// * `x86_64`: `IRQ_APIC_TIMER` (`0xac`) — bidang ABI yang dibekukan.
/// * `aarch64`: INTID GIC itu sendiri (`27`). Pada aarch64 INTID **adalah**
///   nomor interupsi: tidak ada pemisahan "IRQ" (nomor LVT APIC) dan
///   "vektor" (entri IDT) seperti pada x86_64.
#[cfg(target_arch = "x86_64")]
pub const IRQ_TIMER_TERDAFTAR: u32 = IRQ_APIC_TIMER;
#[cfg(target_arch = "aarch64")]
pub const IRQ_TIMER_TERDAFTAR: u32 = VEKTOR_ARM_V8_TIMER;

/// Nomor IRQ IPI antar-CPU APIC. Cermin `IRQ_APIC_IPI` (`APIC.cpp:31`).
pub const IRQ_APIC_IPI: u32 = 0xfd - IRQ_VECTOR_BASE;

/// Nomor IRQ galat APIC. Cermin `IRQ_APIC_ERR` (`APIC.cpp:32`).
pub const IRQ_APIC_ERR: u32 = 0xfe - IRQ_VECTOR_BASE;

/// Nomor IRQ spurious APIC. Cermin `IRQ_APIC_SPURIOUS` (`APIC.cpp:33`).
pub const IRQ_APIC_SPURIOUS: u32 = 0xff - IRQ_VECTOR_BASE;

/// Nomor IRQ PIC sah terakhir. IRQ PIC berada di `0..=15`; di atas itu bukan
/// PIC. Diturunkan dari `pic_disabled_vector_end - pic_disabled_vector_base`
/// (`Interrupts/PIC.h:18-19`).
pub const IRQ_PIC_END: u32 = 15;

/// Laju tick optimal sistem. Cermin `OPTIMAL_TICKS_PER_SECOND_RATE`
/// (`KlatKernel/Efek/Inti/TimeManagement.h:27`), yang juga tercetak di log boot sebagai
/// `APICTimer: Ticks per second: 250`.
pub const TIK_PER_DETIK_OPTIMAL: u32 = 250;

// Kelas tick. Nilainya masuk ke laporan C ABI, jadi TIDAK boleh berubah.
/// Tick diterima pada bootstrap processor.
pub const KELAS_TIK_BSP: u32 = 0;
/// Tick diterima pada application processor.
pub const KELAS_TIK_AP: u32 = 1;
/// Tick ditolak: vektornya bukan vektor timer yang terdaftar.
pub const KELAS_TIK_DITOLAK: u32 = 2;

// Kelas EOI. Sumbu yang BERBEDA dari kelas tick.
/// EOI untuk timer sistem itu sendiri.
pub const KELAS_EOI_TIMER: u32 = 0;
/// EOI untuk vektor lokal APIC lain (IPI, galat, spurious).
pub const KELAS_EOI_APIC_LAIN: u32 = 1;
/// EOI untuk IRQ PIC.
pub const KELAS_EOI_PIC: u32 = 2;
/// EOI untuk IRQ di luar kedua blok yang dikenal.
pub const KELAS_EOI_ASING: u32 = 3;

// Pengendali EOI. Nilainya masuk ke C ABI.
/// Tidak ada EOI.
pub const EOI_TIDAK: u32 = 0;
/// `APIC::the().eoi()`.
pub const EOI_APIC: u32 = 1;
/// EOI PIC.
pub const EOI_PIC: u32 = 2;

// Kode alasan. Hanya bermakna bila keputusan tidak diterima.
/// Keputusan diterima.
pub const ALASAN_OK: u32 = 0;
/// Vektor timer yang diserahkan C++ bukan vektor timer yang terdaftar. Ini
/// penjaga identitas: bila objek timer berpindah vektor tanpa gerbang ini
/// mengetahuinya, kebijakan tick akan diterapkan pada perangkat lain.
pub const ALASAN_BUKAN_VEKTOR_TIMER: u32 = 1;

/// Tag laporan. BERBEDA dari tag laporan gate lain (`KLATKRNL`, `KLATTKID`,
/// `KLATSCHD`, `KLATIPCH`, `KLATCID`, `KLATFPUU`, `KLATINTD`) dengan sengaja:
/// C++ memeriksa tag berbeda per gate sehingga laporan tidak kehilangan daya
/// pisah.
pub const REPORT_TAG: u64 = 0x4B4C_4154_544D_5244; // "KLATTMRD"

// ---------------------------------------------------------------------------
// Keputusan — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Hasil keputusan kebijakan tick timer.
///
/// Cermin `KlatosTimerVerdict` di `Jowo/Kontrak/KlatKernelBridge.h`. Sembilan
/// bidang `u32`, tanpa padding: `sizeof == 36`, `alignof == 4`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosTimerVerdict {
    /// Vektor timer yang diserahkan C++, dikembalikan apa adanya (jejak audit).
    pub vektor: u32,
    /// [`KELAS_TIK_BSP`], [`KELAS_TIK_AP`], atau [`KELAS_TIK_DITOLAK`].
    pub kelas: u32,
    /// IRQ timer yang terdaftar. Tidak bermakna bila ditolak.
    pub irq: u32,
    /// `1` = tick ini dilayani; `0` = ditolak (C++ akan `PANIC`).
    pub diterima: u32,
    /// `1` = C++ wajib memanggil `update_time()`.
    pub perbarui_waktu: u32,
    /// `1` = C++ wajib memanggil `TimerQueue::the().fire()`.
    pub bakar_antrean: u32,
    /// `1` = C++ wajib memanggil `Scheduler::timer_tick()`.
    pub jadwalkan: u32,
    /// `1` = C++ wajib memanggil `handler->eoi()`.
    pub eoi_diminta: u32,
    /// [`ALASAN_OK`] bila diterima, selain itu kode alasan penolakan.
    pub alasan: u32,
}

impl KlatosTimerVerdict {
    /// `true` bila tick ini dilayani.
    pub const fn diterima(&self) -> bool {
        self.diterima == 1
    }

    /// `true` bila kebijakan ini memajukan waktu dinding.
    pub const fn memajukan_waktu(&self) -> bool {
        self.perbarui_waktu == 1
    }
}

/// Hasil keputusan EOI objek timer.
///
/// Cermin `KlatosTimerEoiVerdict` di `Jowo/Kontrak/KlatKernelBridge.h`. Empat
/// bidang `u32`: `sizeof == 16`, `alignof == 4`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosTimerEoiVerdict {
    /// IRQ yang diserahkan C++, dikembalikan apa adanya (jejak audit).
    pub irq: u32,
    /// [`KELAS_EOI_TIMER`], [`KELAS_EOI_APIC_LAIN`], [`KELAS_EOI_PIC`], atau
    /// [`KELAS_EOI_ASING`].
    pub kelas: u32,
    /// [`EOI_APIC`], [`EOI_PIC`], atau [`EOI_TIDAK`].
    pub eoi_jenis: u32,
    /// `1` = EOI harus benar-benar dijalankan; `0` = tidak.
    pub perlu_eoi: u32,
}

impl KlatosTimerEoiVerdict {
    /// `true` bila EOI harus dijalankan.
    pub const fn perlu(&self) -> bool {
        self.perlu_eoi == 1
    }
}

/// Menolak tick pada `vektor` dengan `alasan`.
const fn tolak_tik(vektor: u32, alasan: u32) -> KlatosTimerVerdict {
    KlatosTimerVerdict {
        vektor,
        kelas: KELAS_TIK_DITOLAK,
        irq: 0,
        diterima: 0,
        perbarui_waktu: 0,
        bakar_antrean: 0,
        jadwalkan: 0,
        eoi_diminta: 0,
        alasan,
    }
}

/// Keputusan kebijakan tick untuk satu tick timer. **Murni dan total.**
///
/// `vektor_timer` adalah vektor yang **diklaim** C++ sebagai vektor timer
/// terdaftar (`APICTimer::interrupt_number() + IRQ_VECTOR_BASE`). `bsp` adalah
/// `1` bila CPU ini bootstrap processor.
///
/// Total berarti: untuk SETIAP `(vektor_timer, bsp)` ia mengembalikan keputusan,
/// termasuk penolakan beralasan. Tidak ada panik dan tidak ada efek samping —
/// penghitung audit di-bump oleh [`klatos_timer_tick`], bukan di sini.
///
/// Urutan keputusan pada input yang sama SAMA PERSIS dengan keputusan
/// `system_timer_callback()` C++ sebelum P1.06.
#[inline]
pub const fn putuskan_tik(vektor_timer: u32, bsp: u32) -> KlatosTimerVerdict {
    // Penjaga identitas: hanya vektor timer yang TERDAFTAR PADA ARSITEKTUR INI
    // yang dilayani. Sentinel `0xffffffff` (tipe timer tak dikenal) tidak
    // pernah lolos. Pada x86_64 nilai terdaftar tetap `0xfc`.
    if vektor_timer != VEKTOR_TIMER_TERDAFTAR {
        return tolak_tik(vektor_timer, ALASAN_BUKAN_VEKTOR_TIMER);
    }

    let kelas = if bsp == 1 { KELAS_TIK_BSP } else { KELAS_TIK_AP };

    KlatosTimerVerdict {
        vektor: vektor_timer,
        kelas,
        irq: IRQ_TIMER_TERDAFTAR,
        diterima: 1,
        // Kebijakan BSP: hanya BSP yang memajukan waktu dinding.
        // Cermin `if (Processor::is_bootstrap_processor()) update_time();`
        // (`TimeManagement.cpp:509-514`).
        perbarui_waktu: if bsp == 1 { 1 } else { 0 },
        // Antrean timer dan tick penjadwal berjalan di SETIAP CPU.
        // Cermin `system_timer_tick()` (`TimeManagement.cpp:518-522`).
        bakar_antrean: 1,
        jadwalkan: 1,
        // Cermin `handle_interrupt`: `eoi_diminta` selalu 1 untuk vektor
        // generik, dan timer adalah vektor generik.
        eoi_diminta: 1,
        alasan: ALASAN_OK,
    }
}

/// Keputusan EOI untuk objek timer pada `irq`. **Murni dan total.**
///
/// Aturan yang dikodekan (bukan konstanta hiasan):
///
/// * IRQ timer APIC -> EOI APIC.
/// * IRQ **spurious** APIC -> **TIDAK** ada EOI. Ini aturan APIC yang nyata:
///   interupsi spurious tidak boleh di-EOI.
/// * IRQ lokal APIC lain (IPI, galat) -> EOI APIC.
/// * IRQ PIC (`< 16`) -> EOI PIC.
/// * di luar kedua blok -> APIC. `HardwareTimer<GenericInterruptHandler>`
///   hanya dipakai [`IRQ_APIC_TIMER`] (dipin oleh audit), jadi fallback ini
///   tidak dapat menyembunyikan pemakai kedua; ia hanya menjaga totalitas.
#[inline]
pub const fn putuskan_eoi(irq: u32) -> KlatosTimerEoiVerdict {
    if irq == IRQ_APIC_TIMER {
        return KlatosTimerEoiVerdict {
            irq,
            kelas: KELAS_EOI_TIMER,
            eoi_jenis: EOI_APIC,
            perlu_eoi: 1,
        };
    }
    if irq == IRQ_APIC_SPURIOUS {
        // Spurious: dilaporkan, TIDAK di-EOI.
        return KlatosTimerEoiVerdict {
            irq,
            kelas: KELAS_EOI_APIC_LAIN,
            eoi_jenis: EOI_TIDAK,
            perlu_eoi: 0,
        };
    }
    if irq > IRQ_APIC_TIMER {
        // Blok vektor lokal APIC lain (IPI = 0xad, galat = 0xae).
        return KlatosTimerEoiVerdict {
            irq,
            kelas: KELAS_EOI_APIC_LAIN,
            eoi_jenis: EOI_APIC,
            perlu_eoi: 1,
        };
    }
    if irq <= IRQ_PIC_END {
        return KlatosTimerEoiVerdict {
            irq,
            kelas: KELAS_EOI_PIC,
            eoi_jenis: EOI_PIC,
            perlu_eoi: 1,
        };
    }
    KlatosTimerEoiVerdict {
        irq,
        kelas: KELAS_EOI_ASING,
        eoi_jenis: EOI_APIC,
        perlu_eoi: 1,
    }
}

// ---------------------------------------------------------------------------
// Penghitung audit
// ---------------------------------------------------------------------------

/// Penghitung untuk audit. Indeksnya masuk ke C ABI, jadi tidak boleh berubah.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah tick yang melewati [`klatos_timer_tick`] dari jalur produksi.
    TikMasuk = 0,
    /// Di antaranya: kelas BSP.
    TikBsp = 1,
    /// Di antaranya: kelas AP.
    TikAp = 2,
    /// Di antaranya: ditolak (pada kernel yang hidup, ini harus 0 — penolakan
    /// di jalur produksi berakhir `PANIC`).
    TikDitolak = 3,
    /// Jumlah keputusan EOI yang memilih APIC.
    EoiApic = 4,
    /// Jumlah keputusan EOI yang memilih PIC.
    EoiPic = 5,
    /// Jumlah keputusan EOI yang memilih "tidak ada EOI".
    EoiTidak = 6,
}

const N_COUNTER: usize = 7;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

fn read_counter(counter: Counter) -> u32 {
    COUNTERS[counter as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// **Pintu produksi.** Dipanggil dari `TimeManagement::system_timer_callback()`
/// (`KlatKernel/Efek/Inti/TimeManagement.cpp`), yaitu jalur yang benar-benar dijalankan
/// timer APIC pada 250 Hz.
///
/// Menghitung peristiwa yang sebenarnya, lalu mengembalikan keputusan.
#[no_mangle]
pub extern "C" fn klatos_timer_tick(vektor_timer: u32, bsp: u32) -> KlatosTimerVerdict {
    let verdict = putuskan_tik(vektor_timer, bsp);
    bump(Counter::TikMasuk);
    match verdict.kelas {
        KELAS_TIK_BSP => bump(Counter::TikBsp),
        KELAS_TIK_AP => bump(Counter::TikAp),
        _ => bump(Counter::TikDitolak),
    }
    verdict
}

/// **Pintu produksi EOI.** Dipanggil dari
/// `HardwareTimer<GenericInterruptHandler>::eoi()`
/// (`Jowo/Arsitektur/x86_64/Interrupts/APIC.cpp`).
#[no_mangle]
pub extern "C" fn klatos_timer_eoi(irq: u32) -> KlatosTimerEoiVerdict {
    let verdict = putuskan_eoi(irq);
    match verdict.eoi_jenis {
        EOI_APIC => bump(Counter::EoiApic),
        EOI_PIC => bump(Counter::EoiPic),
        _ => bump(Counter::EoiTidak),
    }
    verdict
}

/// Keputusan tick yang sama TANPA menyentuh penghitung produksi. Dipakai audit
/// dan harness host supaya "keputusan" dan "pemakaian produksi" dapat
/// dibedakan.
#[no_mangle]
pub extern "C" fn klatos_timer_putuskan_tik(vektor_timer: u32, bsp: u32) -> KlatosTimerVerdict {
    putuskan_tik(vektor_timer, bsp)
}

/// Keputusan EOI yang sama TANPA menyentuh penghitung produksi.
#[no_mangle]
pub extern "C" fn klatos_timer_putuskan_eoi(irq: u32) -> KlatosTimerEoiVerdict {
    putuskan_eoi(irq)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_timer_counter(which: u32) -> u32 {
    match which {
        0 => read_counter(Counter::TikMasuk),
        1 => read_counter(Counter::TikBsp),
        2 => read_counter(Counter::TikAp),
        3 => read_counter(Counter::TikDitolak),
        4 => read_counter(Counter::EoiApic),
        5 => read_counter(Counter::EoiPic),
        6 => read_counter(Counter::EoiTidak),
        _ => 0,
    }
}

/// Mengembalikan semua penghitung ke 0. TIDAK dipanggil dari C++ (matriks
/// tidak menyentuh produksi); disediakan untuk konsistensi dengan modul
/// otoritas lain dan untuk harness host.
#[no_mangle]
pub extern "C" fn klatos_timer_reset() -> u32 {
    for counter in COUNTERS.iter() {
        counter.store(0, Ordering::Release);
    }
    TIK_PER_DETIK_OPTIMAL
}

/// Tag laporan; cermin [`REPORT_TAG`].
#[no_mangle]
pub extern "C" fn klatos_timer_tag() -> u64 {
    REPORT_TAG
}

/// Vektor timer yang terdaftar pada arsitektur ini; cermin
/// [`VEKTOR_TIMER_TERDAFTAR`] (`0xfc` pada x86_64, `27` pada aarch64).
#[no_mangle]
pub extern "C" fn klatos_timer_vektor() -> u32 {
    VEKTOR_TIMER_TERDAFTAR
}

/// IRQ timer yang terdaftar pada arsitektur ini; cermin
/// [`IRQ_TIMER_TERDAFTAR`] (`0xac` pada x86_64, `27` pada aarch64).
#[no_mangle]
pub extern "C" fn klatos_timer_irq() -> u32 {
    IRQ_TIMER_TERDAFTAR
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosTimerReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
///
/// `tag: u64` @0, lalu 27 `u32`, lalu `status: i32` ->
/// `sizeof == 120`, `alignof == 8`. Tata letaknya dijaga `static_assert` di
/// sisi C++ dan diuji harness host.
#[repr(C)]
pub struct KlatosTimerReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub tik_checks: u32,
    pub tik_passed: u32,
    pub eoi_checks: u32,
    pub eoi_passed: u32,
    pub konsisten_checks: u32,
    pub konsisten_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub tik_masuk: u32,
    pub tik_bsp: u32,
    pub tik_ap: u32,
    pub tik_ditolak: u32,
    pub eoi_apic: u32,
    pub eoi_pic: u32,
    pub eoi_tidak: u32,
    pub vektor_timer: u32,
    pub irq_timer: u32,
    pub tik_per_detik: u32,
    pub kelas_bsp: u32,
    pub kelas_ap: u32,
    pub kelas_tolak: u32,
    pub eoi_jenis_apic: u32,
    pub eoi_jenis_pic: u32,
    pub eoi_jenis_tidak: u32,
    pub alasan_bukan_vektor: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Tik,
    Eoi,
    Konsisten,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    tik_checks: u32,
    tik_passed: u32,
    eoi_checks: u32,
    eoi_passed: u32,
    konsisten_checks: u32,
    konsisten_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            tik_checks: 0,
            tik_passed: 0,
            eoi_checks: 0,
            eoi_passed: 0,
            konsisten_checks: 0,
            konsisten_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }

    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Tik => {
                self.tik_checks += 1;
                if lulus {
                    self.tik_passed += 1;
                }
            }
            Group::Eoi => {
                self.eoi_checks += 1;
                if lulus {
                    self.eoi_passed += 1;
                }
            }
            Group::Konsisten => {
                self.konsisten_checks += 1;
                if lulus {
                    self.konsisten_passed += 1;
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

/// Membandingkan keputusan tick dengan kelas dan kebijakan yang diharapkan.
const fn cocok_tik(vektor: u32, bsp: u32, kelas: u32, waktu: u32, diterima: u32, alasan: u32) -> bool {
    let v = putuskan_tik(vektor, bsp);
    v.kelas == kelas
        && v.perbarui_waktu == waktu
        && v.diterima == diterima
        && v.alasan == alasan
        && v.vektor == vektor
}

/// Membandingkan keputusan EOI dengan kelas dan jenis yang diharapkan.
const fn cocok_eoi(irq: u32, kelas: u32, jenis: u32, perlu: u32) -> bool {
    let v = putuskan_eoi(irq);
    v.kelas == kelas && v.eoi_jenis == jenis && v.perlu_eoi == perlu && v.irq == irq
}

/// Matriks otoritas timer. Berjalan pada input **SINTETIS**; ia memanggil
/// [`putuskan_tik`]/[`putuskan_eoi`] (murni) dan **tidak pernah**
/// [`klatos_timer_tick`]/[`klatos_timer_eoi`], sehingga tidak menggeser
/// penghitung produksi. Pemakaian produksi dilihat terpisah oleh `init_stage2`
/// lewat [`klatos_timer_counter`].
fn run_matrix() -> KlatosTimerReport {
    let mut m = Matrix::new();

    // --- kelompok TIK: vektor timer yang terdaftar -------------------------
    // BSP: memajukan waktu.
    m.catat(
        cocok_tik(VEKTOR_TIMER_TERDAFTAR, 1, KELAS_TIK_BSP, 1, 1, ALASAN_OK),
        Group::Tik,
    );
    // AP: TIDAK memajukan waktu, tetapi tetap menerima tick.
    m.catat(
        cocok_tik(VEKTOR_TIMER_TERDAFTAR, 0, KELAS_TIK_AP, 0, 1, ALASAN_OK),
        Group::Tik,
    );
    // Kebijakan yang tidak bergantung pada CPU: dua-duanya menerima dan
    // dua-duanya meminta EOI.
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).eoi_diminta == 1, Group::Tik);
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 0).eoi_diminta == 1, Group::Tik);
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).bakar_antrean == 1, Group::Tik);
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 0).bakar_antrean == 1, Group::Tik);
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).jadwalkan == 1, Group::Tik);
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 0).jadwalkan == 1, Group::Tik);
    // IRQ yang dilaporkan adalah IRQ timer, bukan vektornya.
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).irq == IRQ_TIMER_TERDAFTAR, Group::Tik);
    // Predikat turunan harus sepadan dengan bidang mentahnya — bukan konstanta
    // yang kebetulan selalu benar.
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).memajukan_waktu(), Group::Tik);
    m.catat(!putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 0).memajukan_waktu(), Group::Tik);

    // --- kelompok TIK: penolakan identitas ---------------------------------
    // Vektor timer yang SALAH harus ditolak, dengan alasan yang dapat
    // dibedakan. Ini penjaga identitas timer.
    m.catat(
        cocok_tik(0x50, 1, KELAS_TIK_DITOLAK, 0, 0, ALASAN_BUKAN_VEKTOR_TIMER),
        Group::Tik,
    );
    m.catat(
        cocok_tik(0x00, 1, KELAS_TIK_DITOLAK, 0, 0, ALASAN_BUKAN_VEKTOR_TIMER),
        Group::Tik,
    );
    // Tetangga langsung timer (IPI, galat, spurious) BUKAN timer.
    m.catat(
        cocok_tik(0xfd, 1, KELAS_TIK_DITOLAK, 0, 0, ALASAN_BUKAN_VEKTOR_TIMER),
        Group::Tik,
    );
    m.catat(
        cocok_tik(0xfe, 1, KELAS_TIK_DITOLAK, 0, 0, ALASAN_BUKAN_VEKTOR_TIMER),
        Group::Tik,
    );
    m.catat(
        cocok_tik(0xff, 1, KELAS_TIK_DITOLAK, 0, 0, ALASAN_BUKAN_VEKTOR_TIMER),
        Group::Tik,
    );

    // --- kelompok EOI: pemetaan pengendali ---------------------------------
    m.catat(
        cocok_eoi(IRQ_APIC_TIMER, KELAS_EOI_TIMER, EOI_APIC, 1),
        Group::Eoi,
    );
    // Predikat turunan sepadan dengan bidang mentahnya.
    m.catat(putuskan_eoi(IRQ_APIC_TIMER).perlu(), Group::Eoi);
    m.catat(!putuskan_eoi(IRQ_APIC_SPURIOUS).perlu(), Group::Eoi);
    m.catat(
        cocok_eoi(IRQ_APIC_IPI, KELAS_EOI_APIC_LAIN, EOI_APIC, 1),
        Group::Eoi,
    );
    m.catat(
        cocok_eoi(IRQ_APIC_ERR, KELAS_EOI_APIC_LAIN, EOI_APIC, 1),
        Group::Eoi,
    );
    // Spurious: aturan APIC yang nyata — TIDAK di-EOI.
    m.catat(
        cocok_eoi(IRQ_APIC_SPURIOUS, KELAS_EOI_APIC_LAIN, EOI_TIDAK, 0),
        Group::Eoi,
    );
    // PIC.
    m.catat(cocok_eoi(0, KELAS_EOI_PIC, EOI_PIC, 1), Group::Eoi);
    m.catat(cocok_eoi(7, KELAS_EOI_PIC, EOI_PIC, 1), Group::Eoi);
    m.catat(cocok_eoi(15, KELAS_EOI_PIC, EOI_PIC, 1), Group::Eoi);
    // Di luar kedua blok.
    m.catat(cocok_eoi(16, KELAS_EOI_ASING, EOI_APIC, 1), Group::Eoi);
    m.catat(cocok_eoi(0xab, KELAS_EOI_ASING, EOI_APIC, 1), Group::Eoi);

    // --- kelompok KONSISTEN: invarian lintas ruang input -------------------
    // Sapuan seluruh vektor 0x00..0x200: diterima IFF vektor == timer.
    let mut sapuan_benar = true;
    let mut sapuan_diterima = 0u32;
    let mut sapuan_ditolak = 0u32;
    let mut v = 0u32;
    while v < 0x200 {
        let t = putuskan_tik(v, 1);
        let harus_diterima = v == VEKTOR_TIMER_TERDAFTAR;
        if t.diterima() != harus_diterima {
            sapuan_benar = false;
        }
        if t.diterima() {
            sapuan_diterima += 1;
            // Kebijakan hanya bermakna bila diterima.
            if t.bakar_antrean != 1 || t.jadwalkan != 1 || t.eoi_diminta != 1 {
                sapuan_benar = false;
            }
        } else {
            sapuan_ditolak += 1;
            if t.perbarui_waktu != 0
                || t.bakar_antrean != 0
                || t.jadwalkan != 0
                || t.eoi_diminta != 0
                || t.irq != 0
            {
                // Penolakan tidak boleh menyisakan kebijakan separuh jalan.
                sapuan_benar = false;
            }
        }
        // Kelas dan alasan saling konsisten.
        let kelas_konsisten = (t.kelas == KELAS_TIK_DITOLAK) == !t.diterima();
        let alasan_konsisten = if t.diterima() {
            t.alasan == ALASAN_OK
        } else {
            t.alasan != ALASAN_OK
        };
        if !kelas_konsisten || !alasan_konsisten {
            sapuan_benar = false;
        }
        v += 1;
    }
    m.catat(sapuan_benar, Group::Konsisten);
    // Tepat SATU vektor diterima dari 512 yang disapu.
    m.catat(sapuan_diterima == 1, Group::Konsisten);
    m.catat(sapuan_ditolak == 0x1ff, Group::Konsisten);
    m.catat(sapuan_diterima + sapuan_ditolak == 0x200, Group::Konsisten);

    // Sapuan IRQ 0..0x200: jumlah EOI dan jumlah "tidak ada EOI" harus persis.
    let mut eoi_apic = 0u32;
    let mut eoi_pic = 0u32;
    let mut eoi_tidak = 0u32;
    let mut eoi_sepadan = true;
    let mut irq = 0u32;
    while irq < 0x200 {
        let e = putuskan_eoi(irq);
        match e.eoi_jenis {
            EOI_APIC => eoi_apic += 1,
            EOI_PIC => eoi_pic += 1,
            _ => eoi_tidak += 1,
        }
        // `perlu_eoi` harus sepadan dengan jenis: satu-satunya jenis tanpa EOI
        // adalah `EOI_TIDAK`, dan sebaliknya.
        if (e.eoi_jenis == EOI_TIDAK) != (e.perlu_eoi == 0) {
            eoi_sepadan = false;
        }
        irq += 1;
    }
    m.catat(eoi_sepadan, Group::Konsisten);
    // Dihitung dari aturan, bukan disalin: PIC = 16 (0..=15);
    // "tidak ada EOI" = 1 (hanya spurious = 0xaf).
    m.catat(eoi_pic == 16, Group::Konsisten);
    m.catat(eoi_tidak == 1, Group::Konsisten);
    m.catat(eoi_apic == 0x200 - 16 - 1, Group::Konsisten);
    m.catat(eoi_apic + eoi_pic + eoi_tidak == 0x200, Group::Konsisten);

    // Idempoten: 1000 keputusan berturut-turut identik.
    let mut last_tik: Option<KlatosTimerVerdict> = None;
    let mut last_eoi: Option<KlatosTimerEoiVerdict> = None;
    let mut idempoten = true;
    let mut i = 0;
    while i < 1000 {
        let t = putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1);
        let e = putuskan_eoi(IRQ_APIC_TIMER);
        match (last_tik, last_eoi) {
            (None, None) => {
                last_tik = Some(t);
                last_eoi = Some(e);
            }
            (Some(pt), Some(pe)) => {
                if pt != t || pe != e {
                    idempoten = false;
                    break;
                }
            }
            _ => {
                idempoten = false;
                break;
            }
        }
        i += 1;
    }
    m.catat(idempoten, Group::Konsisten);

    // --- kelompok NEGATIF: harus BISA gagal --------------------------------
    m.catat(IRQ_VECTOR_BASE == 0x50, Group::Neg);
    m.catat(IRQ_APIC_TIMER == 0xac, Group::Neg);
    m.catat(VEKTOR_APIC_TIMER == 0xfc, Group::Neg);
    // Konstanta TERDAFTAR harus bernilai benar per arsitektur — inilah
    // "harus BISA gagal" untuk port aarch64: kalau nilainya salah, matriks
    // ini gagal, bukan diam-diam menerima.
    #[cfg(target_arch = "x86_64")]
    m.catat(
        VEKTOR_TIMER_TERDAFTAR == 0xfc && IRQ_TIMER_TERDAFTAR == 0xac,
        Group::Neg,
    );
    #[cfg(target_arch = "aarch64")]
    m.catat(
        VEKTOR_TIMER_TERDAFTAR == 27 && IRQ_TIMER_TERDAFTAR == 27,
        Group::Neg,
    );
    m.catat(IRQ_APIC_IPI == 0xad, Group::Neg);
    m.catat(IRQ_APIC_ERR == 0xae, Group::Neg);
    m.catat(IRQ_APIC_SPURIOUS == 0xaf, Group::Neg);
    m.catat(IRQ_PIC_END == 15, Group::Neg);
    m.catat(TIK_PER_DETIK_OPTIMAL == 250, Group::Neg);
    m.catat(REPORT_TAG == 0x4B4C_4154_544D_5244, Group::Neg);
    m.catat(
        KELAS_TIK_BSP == 0 && KELAS_TIK_AP == 1 && KELAS_TIK_DITOLAK == 2,
        Group::Neg,
    );
    m.catat(
        KELAS_EOI_TIMER == 0
            && KELAS_EOI_APIC_LAIN == 1
            && KELAS_EOI_PIC == 2
            && KELAS_EOI_ASING == 3,
        Group::Neg,
    );
    m.catat(EOI_TIDAK == 0 && EOI_APIC == 1 && EOI_PIC == 2, Group::Neg);
    m.catat(ALASAN_OK == 0 && ALASAN_BUKAN_VEKTOR_TIMER == 1, Group::Neg);
    // Predikat yang SAMA, dua input, dua nilai berbeda: membuktikan bahwa
    // `diterima` benar-benar memisahkan, bukan konstanta.
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).diterima == 1, Group::Neg);
    m.catat(putuskan_tik(0xfd, 1).diterima == 0, Group::Neg);
    // `perbarui_waktu` benar-benar memisahkan BSP dari AP.
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).perbarui_waktu == 1, Group::Neg);
    m.catat(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 0).perbarui_waktu == 0, Group::Neg);
    // Penolakan benar-benar terpicu di dalam matriks ini.
    m.catat(putuskan_tik(0xfd, 1).alasan != ALASAN_OK, Group::Neg);
    m.catat(putuskan_eoi(IRQ_APIC_SPURIOUS).perlu_eoi == 0, Group::Neg);
    m.catat(putuskan_eoi(IRQ_APIC_TIMER).perlu_eoi == 1, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosTimerReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        tik_checks: m.tik_checks,
        tik_passed: m.tik_passed,
        eoi_checks: m.eoi_checks,
        eoi_passed: m.eoi_passed,
        konsisten_checks: m.konsisten_checks,
        konsisten_passed: m.konsisten_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        tik_masuk: klatos_timer_counter(Counter::TikMasuk as u32),
        tik_bsp: klatos_timer_counter(Counter::TikBsp as u32),
        tik_ap: klatos_timer_counter(Counter::TikAp as u32),
        tik_ditolak: klatos_timer_counter(Counter::TikDitolak as u32),
        eoi_apic: klatos_timer_counter(Counter::EoiApic as u32),
        eoi_pic: klatos_timer_counter(Counter::EoiPic as u32),
        eoi_tidak: klatos_timer_counter(Counter::EoiTidak as u32),
        vektor_timer: VEKTOR_TIMER_TERDAFTAR,
        irq_timer: IRQ_TIMER_TERDAFTAR,
        tik_per_detik: TIK_PER_DETIK_OPTIMAL,
        kelas_bsp: KELAS_TIK_BSP,
        kelas_ap: KELAS_TIK_AP,
        kelas_tolak: KELAS_TIK_DITOLAK,
        eoi_jenis_apic: EOI_APIC,
        eoi_jenis_pic: EOI_PIC,
        eoi_jenis_tidak: EOI_TIDAK,
        alasan_bukan_vektor: ALASAN_BUKAN_VEKTOR_TIMER,
        status,
    }
}

/// Menjalankan matriks otoritas tick timer.
#[no_mangle]
pub extern "C" fn klatos_timer_selftest() -> KlatosTimerReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        // Konstanta dasar.
        assert!(IRQ_VECTOR_BASE == 0x50);
        assert!(IRQ_APIC_TIMER == 0xac);
        assert!(VEKTOR_APIC_TIMER == 0xfc);
        // Vektor / IRQ timer TERDAFTAR harus sesuai arsitektur. Ditulis tanpa
        // atribut `cfg` pada pernyataan supaya diperiksa pada KEDUA arsitektur.
        assert!(
            VEKTOR_TIMER_TERDAFTAR == if cfg!(target_arch = "x86_64") { 0xfc } else { 27 }
        );
        assert!(
            IRQ_TIMER_TERDAFTAR
                == if cfg!(target_arch = "x86_64") { 0xac } else { 27 }
        );
        assert!(IRQ_APIC_IPI == 0xad);
        assert!(IRQ_APIC_ERR == 0xae);
        assert!(IRQ_APIC_SPURIOUS == 0xaf);
        assert!(IRQ_PIC_END == 15);
        assert!(TIK_PER_DETIK_OPTIMAL == 250);

        // Tag laporan konstan.
        assert!(REPORT_TAG == 0x4B4C_4154_544D_5244);

        // Kode kelas, jenis EOI, dan alasan tidak boleh bergeser: ia masuk ke
        // C ABI.
        assert!(KELAS_TIK_BSP == 0);
        assert!(KELAS_TIK_AP == 1);
        assert!(KELAS_TIK_DITOLAK == 2);
        assert!(KELAS_EOI_TIMER == 0);
        assert!(KELAS_EOI_APIC_LAIN == 1);
        assert!(KELAS_EOI_PIC == 2);
        assert!(KELAS_EOI_ASING == 3);
        assert!(EOI_TIDAK == 0);
        assert!(EOI_APIC == 1);
        assert!(EOI_PIC == 2);
        assert!(ALASAN_OK == 0);
        assert!(ALASAN_BUKAN_VEKTOR_TIMER == 1);

        // Urutan penghitung audit stabil.
        assert!(Counter::TikMasuk as usize == 0);
        assert!(Counter::TikBsp as usize == 1);
        assert!(Counter::TikAp as usize == 2);
        assert!(Counter::TikDitolak as usize == 3);
        assert!(Counter::EoiApic as usize == 4);
        assert!(Counter::EoiPic as usize == 5);
        assert!(Counter::EoiTidak as usize == 6);

        // Byte-identik dengan C++ historis pada jalur produksi.
        assert!(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).diterima == 1);
        assert!(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).perbarui_waktu == 1);
        assert!(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 0).diterima == 1);
        assert!(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 0).perbarui_waktu == 0);
        assert!(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).bakar_antrean == 1);
        assert!(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).jadwalkan == 1);
        assert!(putuskan_tik(VEKTOR_TIMER_TERDAFTAR, 1).eoi_diminta == 1);
        // Penolakan identitas.
        assert!(putuskan_tik(0xfd, 1).kelas == KELAS_TIK_DITOLAK);
        assert!(putuskan_tik(0xfd, 1).alasan == ALASAN_BUKAN_VEKTOR_TIMER);
        // EOI.
        assert!(putuskan_eoi(IRQ_APIC_TIMER).eoi_jenis == EOI_APIC);
        assert!(putuskan_eoi(IRQ_APIC_TIMER).perlu_eoi == 1);
        assert!(putuskan_eoi(IRQ_APIC_SPURIOUS).eoi_jenis == EOI_TIDAK);
        assert!(putuskan_eoi(IRQ_APIC_SPURIOUS).perlu_eoi == 0);
        assert!(putuskan_eoi(7).eoi_jenis == EOI_PIC);

        // Ukuran tipe yang melintasi C ABI.
        assert!(core::mem::size_of::<KlatosTimerVerdict>() == 36);
        assert!(core::mem::align_of::<KlatosTimerVerdict>() == 4);
        assert!(core::mem::size_of::<KlatosTimerEoiVerdict>() == 16);
        assert!(core::mem::align_of::<KlatosTimerEoiVerdict>() == 4);
        // 8 (tag) + 27 * 4 + 4 (status) = 120 byte; alignof = 8.
        assert!(core::mem::size_of::<KlatosTimerReport>() == 120);
        assert!(core::mem::align_of::<KlatosTimerReport>() == 8);
    };
}
