// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas **dispatch interupsi** Jowo (P1.05) — MODEL B.


use core::sync::atomic::{AtomicU32, Ordering};

// ---------------------------------------------------------------------------
// Konstanta — cermin C++ yang mengikat
// ---------------------------------------------------------------------------

/// Basis vektor IRQ generik. Cermin `IRQ_VECTOR_BASE`
/// (`Jowo/Arsitektur/x86_64/CPU.h:21`).
pub const IRQ_VECTOR_BASE: u32 = 0x50;

/// Jumlah slot tabel handler generik. Cermin
/// `GENERIC_INTERRUPT_HANDLERS_COUNT` (`CPU.h:22` = `256 - IRQ_VECTOR_BASE`),
/// yang sekaligus panjang `s_interrupt_handler[]`
/// (`Interrupts.cpp:60`).
pub const GENERIC_HANDLER_COUNT: u32 = 256 - IRQ_VECTOR_BASE;

/// Vektor pertama yang dipakai saat PIC dinonaktifkan. Cermin
/// `pic_disabled_vector_base` (`Interrupts/PIC.h:18`).
pub const PIC_DISABLED_VECTOR_BASE: u32 = 0x20;

/// Vektor terakhir yang dipakai saat PIC dinonaktifkan. Cermin
/// `pic_disabled_vector_end` (`Interrupts/PIC.h:19`).
pub const PIC_DISABLED_VECTOR_END: u32 = 0x2f;

/// Panjang `s_disabled_interrupt_handler[]` (`Interrupts.cpp:61`).
pub const DISABLED_SLOT_COUNT: u32 = 2;

/// Jumlah vektor IDT x86_64. Batas keras `isr_number` yang dapat dicapai.
pub const IDT_VECTORS: u32 = 256;

// Kelas vektor. Nilainya masuk ke laporan C ABI, jadi TIDAK boleh berubah.
/// Vektor generik (`isr - IRQ_VECTOR_BASE` mengindeks `s_interrupt_handler[]`).
pub const KELAS_GENERIK: u32 = 0;
/// Vektor PIC-disabled (`isr - 0x20` adalah nomor IRQ PIC).
pub const KELAS_PIC_DISABLED: u32 = 1;
/// Vektor ditolak: tidak dilayani, C++ akan `PANIC`.
pub const KELAS_DITOLAK: u32 = 2;

// Kode alasan. Hanya bermakna bila `kelas == KELAS_DITOLAK`.
/// Keputusan diterima.
pub const ALASAN_OK: u32 = 0;
/// Kelas PIC-disabled, tetapi IRQ bukan 7 dan bukan 15: C++ lama meninggalkan
/// `handler == nullptr` lalu `VERIFY(handler)` gagal.
pub const ALASAN_PIC_BUKAN_7_ATAU_15: u32 = 1;
/// Di bawah `IRQ_VECTOR_BASE` dan di luar rentang PIC-disabled: C++ lama
/// `VERIFY` gagal. Termasuk celah 0x30..0x4f yang stub ASM-nya NYATA.
pub const ALASAN_DI_BAWAH_BASE: u32 = 2;
/// Di atas `IRQ_VECTOR_BASE + GENERIC_HANDLER_COUNT - 1`: indeks akan keluar
/// dari `s_interrupt_handler[]`.
pub const ALASAN_DI_ATAS_BATAS: u32 = 3;

/// Tag laporan. BERBEDA dari tag laporan gate lain (`KLATKRNL`, `KLATTKID`,
/// `KLATSCHD`, `KLATIPCH`, `KLATCID`, `KLATFPUU`) dengan sengaja: C++ memeriksa
/// tag berbeda per gate sehingga laporan tidak kehilangan daya pisah.
pub const REPORT_TAG: u64 = 0x4B4C_4154_494E_5444; // "KLATINTD"

// ---------------------------------------------------------------------------
// Keputusan — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Hasil keputusan dispatch untuk satu vektor.
///
/// Cermin `KlatosIntVerdict` di `Jowo/Kontrak/KlatKernelBridge.h`. Delapan
/// bidang `u32`, tanpa padding: `sizeof == 32`, `alignof == 4`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatosIntVerdict {
    /// Vektor yang diminta, dikembalikan apa adanya (jejak audit).
    pub isr_number: u32,
    /// [`KELAS_GENERIK`], [`KELAS_PIC_DISABLED`], atau [`KELAS_DITOLAK`].
    pub kelas: u32,
    /// IRQ di dalam kelas (`isr - base`). Tidak bermakna bila ditolak.
    pub irq: u32,
    /// Indeks slot yang harus dipakai C++. Tidak bermakna bila ditolak.
    pub slot: u32,
    /// `1` = C++ memakai `s_disabled_interrupt_handler[]`; `0` = memakai
    /// `s_interrupt_handler[]`.
    pub pakai_disabled: u32,
    /// `1` = C++ wajib memanggil `handler->eoi()` setelah handler.
    pub eoi_diminta: u32,
    /// `1` = vektor ini dilayani; `0` = ditolak (C++ akan `PANIC`).
    pub diterima: u32,
    /// [`ALASAN_OK`] bila diterima, selain itu kode alasan penolakan.
    pub alasan: u32,
}

impl KlatosIntVerdict {
    /// `true` bila vektor ini dilayani.
    pub const fn diterima(&self) -> bool {
        self.diterima == 1
    }

    /// Cermin dari predikat C++ lama: apakah vektor ini akan mencapai
    /// `VERIFY(handler)` dengan slot yang terisi.
    pub const fn pakai_tabel_disabled(&self) -> bool {
        self.pakai_disabled == 1
    }
}

/// Menolak `isr_number` dengan `alasan`.
const fn tolak(isr_number: u32, alasan: u32) -> KlatosIntVerdict {
    KlatosIntVerdict {
        isr_number,
        kelas: KELAS_DITOLAK,
        irq: 0,
        slot: 0,
        pakai_disabled: 0,
        eoi_diminta: 0,
        diterima: 0,
        alasan,
    }
}

/// Keputusan dispatch untuk satu vektor IDT. **Murni dan total.**
///
/// Total berarti: untuk SETIAP `u32` ia mengembalikan keputusan, termasuk
/// penolakan beralasan. Tidak ada panik, tidak ada pembacaan di luar batas,
/// tidak ada efek samping — penghitung audit di-bump oleh
/// [`klatos_int_dispatch`], bukan di sini.
///
/// Urutan byte keputusan pada input yang sama SAMA PERSIS dengan urutan
/// keputusan `handle_interrupt` C++ sebelum P1.05, untuk seluruh vektor yang
/// dapat dicapai dari IDT.
#[inline]
pub const fn decide(isr_number: u32) -> KlatosIntVerdict {
    // Kelas 1: rentang PIC-disabled. Diperiksa LEBIH DULU, sama seperti C++:
    // rentang ini berada di bawah IRQ_VECTOR_BASE, jadi urutannya penting.
    if isr_number >= PIC_DISABLED_VECTOR_BASE && isr_number <= PIC_DISABLED_VECTOR_END {
        let irq = isr_number - PIC_DISABLED_VECTOR_BASE;
        // Pemetaan slot C++ historis — BERSILANGAN dengan
        // `register_disabled_interrupt_handler`, dan sengaja direproduksi
        // apa adanya supaya tidak ada perilaku produksi yang berubah.
        if irq == 7 {
            return KlatosIntVerdict {
                isr_number,
                kelas: KELAS_PIC_DISABLED,
                irq,
                slot: 0,
                pakai_disabled: 1,
                eoi_diminta: 1,
                diterima: 1,
                alasan: ALASAN_OK,
            };
        }
        if irq == 15 {
            return KlatosIntVerdict {
                isr_number,
                kelas: KELAS_PIC_DISABLED,
                irq,
                slot: 1,
                pakai_disabled: 1,
                eoi_diminta: 1,
                diterima: 1,
                alasan: ALASAN_OK,
            };
        }
        // C++ lama: `handler` tetap nullptr -> `VERIFY(handler)` -> PANIC.
        return tolak(isr_number, ALASAN_PIC_BUKAN_7_ATAU_15);
    }

    // Kelas 0: rentang generik. Batas atasnya `IRQ_VECTOR_BASE +
    // GENERIC_HANDLER_COUNT - 1` = 0xff, yaitu indeks sah TERAKHIR
    // `s_interrupt_handler[]`.
    if isr_number >= IRQ_VECTOR_BASE && isr_number < IRQ_VECTOR_BASE + GENERIC_HANDLER_COUNT {
        let irq = isr_number - IRQ_VECTOR_BASE;
        return KlatosIntVerdict {
            isr_number,
            kelas: KELAS_GENERIK,
            irq,
            slot: irq,
            pakai_disabled: 0,
            eoi_diminta: 1,
            diterima: 1,
            alasan: ALASAN_OK,
        };
    }

    // Di luar keduanya. C++ lama: `VERIFY` gagal (bawah) atau pembacaan di
    // luar batas (0x100).
    if isr_number < IRQ_VECTOR_BASE {
        tolak(isr_number, ALASAN_DI_BAWAH_BASE)
    } else {
        tolak(isr_number, ALASAN_DI_ATAS_BATAS)
    }
}

// ---------------------------------------------------------------------------
// Penghitung audit + peta vektor
// ---------------------------------------------------------------------------

/// Penghitung untuk audit. Indeksnya masuk ke C ABI, jadi tidak boleh berubah.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah pemanggilan [`klatos_int_dispatch`] dari jalur produksi.
    Masuk = 0,
    /// Di antaranya: kelas generik.
    Generik = 1,
    /// Di antaranya: kelas PIC-disabled.
    PicDisabled = 2,
    /// Di antaranya: ditolak (pada kernel yang hidup, ini harus 0 — penolakan
    /// di jalur produksi berakhir `PANIC`).
    Ditolak = 3,
}

const N_COUNTER: usize = 4;

static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

fn bump(counter: Counter) {
    COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
}

fn read_counter(counter: Counter) -> u32 {
    COUNTERS[counter as usize].load(Ordering::Relaxed)
}

/// Jumlah kata bitmap vektor. Cakupan bitmap `0x20..=0xff` = 224 bit.
const VECTOR_WORDS: usize = 7;

static VECTOR_SEEN: [AtomicU32; VECTOR_WORDS] = [const { AtomicU32::new(0) }; VECTOR_WORDS];

/// Mencatat bahwa `isr_number` benar-benar dilayani pada boot ini.
fn mark_vector(isr_number: u32) {
    if isr_number < PIC_DISABLED_VECTOR_BASE {
        return;
    }
    let bit = isr_number - PIC_DISABLED_VECTOR_BASE;
    let word = (bit / 32) as usize;
    if word >= VECTOR_WORDS {
        return;
    }
    VECTOR_SEEN[word].fetch_or(1u32 << (bit % 32), Ordering::Relaxed);
}

/// Jumlah vektor BERBEDA yang pernah dilayani sejak boot.
#[no_mangle]
pub extern "C" fn klatos_int_vector_used() -> u32 {
    let mut total = 0;
    let mut index = 0;
    while index < VECTOR_WORDS {
        total += VECTOR_SEEN[index].load(Ordering::Relaxed).count_ones();
        index += 1;
    }
    total
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// **Pintu produksi.** Dipanggil dari `handle_interrupt`
/// (`Jowo/Arsitektur/x86_64/Interrupts.cpp`), di dalam penjaga FPU.
///
/// Menghitung peristiwa yang sebenarnya, lalu mengembalikan keputusan.
#[no_mangle]
pub extern "C" fn klatos_int_dispatch(isr_number: u32) -> KlatosIntVerdict {
    let verdict = decide(isr_number);
    bump(Counter::Masuk);
    match verdict.kelas {
        KELAS_GENERIK => bump(Counter::Generik),
        KELAS_PIC_DISABLED => bump(Counter::PicDisabled),
        _ => bump(Counter::Ditolak),
    }
    if verdict.diterima() {
        mark_vector(isr_number);
    }
    verdict
}

/// Keputusan yang sama TANPA menyentuh penghitung produksi. Dipakai audit dan
/// harness host supaya "keputusan" dan "pemakaian produksi" dapat dibedakan.
#[no_mangle]
pub extern "C" fn klatos_int_decide(isr_number: u32) -> KlatosIntVerdict {
    decide(isr_number)
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_int_counter(which: u32) -> u32 {
    match which {
        0 => read_counter(Counter::Masuk),
        1 => read_counter(Counter::Generik),
        2 => read_counter(Counter::PicDisabled),
        3 => read_counter(Counter::Ditolak),
        _ => 0,
    }
}

/// Mengembalikan semua penghitung dan bitmap vektor ke 0. TIDAK dipanggil dari
/// C++ (matriks tidak menyentuh produksi); disediakan untuk konsistensi dengan
/// modul otoritas lain dan untuk harness host.
#[no_mangle]
pub extern "C" fn klatos_int_reset() -> u32 {
    for counter in COUNTERS.iter() {
        counter.store(0, Ordering::Release);
    }
    for word in VECTOR_SEEN.iter() {
        word.store(0, Ordering::Release);
    }
    GENERIC_HANDLER_COUNT
}

/// Apakah Rust memiliki IDT. **Selalu `0`.**
///
/// Ini pernyataan arsitektur MODEL B, bukan angka hiasan: Rust tidak memasang
/// IDT, tidak memanggil `lidt`, dan tidak memiliki `iretq`. C++ memeriksanya di
/// `init_stage2` sehingga klaim itu dapat diaudit dari log serial.
#[no_mangle]
pub extern "C" fn klatos_int_owns_idt() -> u32 {
    0
}

/// Tag laporan; cermin [`REPORT_TAG`].
#[no_mangle]
pub extern "C" fn klatos_int_tag() -> u64 {
    REPORT_TAG
}

/// Basis vektor generik; cermin [`IRQ_VECTOR_BASE`].
#[no_mangle]
pub extern "C" fn klatos_int_irq_vector_base() -> u32 {
    IRQ_VECTOR_BASE
}

/// Panjang tabel handler generik; cermin [`GENERIC_HANDLER_COUNT`].
#[no_mangle]
pub extern "C" fn klatos_int_generic_count() -> u32 {
    GENERIC_HANDLER_COUNT
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosIntReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
///
/// `tag: u64` @0 lalu 26 `u32` -> `sizeof == 112`, `alignof == 8`. Tata letaknya
/// dijaga `static_assert` di sisi C++ dan diuji harness host.
#[repr(C)]
pub struct KlatosIntReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub kelas_checks: u32,
    pub kelas_passed: u32,
    pub tolak_checks: u32,
    pub tolak_passed: u32,
    pub konsisten_checks: u32,
    pub konsisten_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub masuk: u32,
    pub ditolak: u32,
    pub generik: u32,
    pub pic_disabled: u32,
    pub vektor_terpakai: u32,
    pub irq_vector_base: u32,
    pub generic_count: u32,
    pub pic_base: u32,
    pub pic_end: u32,
    pub kelas_generik: u32,
    pub kelas_pic: u32,
    pub kelas_tolak: u32,
    pub alasan_pic: u32,
    pub alasan_bawah: u32,
    pub alasan_atas: u32,
    pub status: i32,
}

#[derive(Clone, Copy)]
enum Group {
    Kelas,
    Tolak,
    Konsisten,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    kelas_checks: u32,
    kelas_passed: u32,
    tolak_checks: u32,
    tolak_passed: u32,
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
            kelas_checks: 0,
            kelas_passed: 0,
            tolak_checks: 0,
            tolak_passed: 0,
            konsisten_checks: 0,
            konsisten_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
        }
    }

    fn catat(&mut self, lulus: bool, group: Group) {
        self.checks += 1;
        match group {
            Group::Kelas => {
                self.kelas_checks += 1;
                if lulus {
                    self.kelas_passed += 1;
                }
            }
            Group::Tolak => {
                self.tolak_checks += 1;
                if lulus {
                    self.tolak_passed += 1;
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

/// Membandingkan keputusan `isr` dengan kelas/slot/alasan yang diharapkan.
const fn cocok(isr: u32, kelas: u32, slot: u32, diterima: u32, alasan: u32) -> bool {
    let v = decide(isr);
    v.kelas == kelas
        && v.slot == slot
        && v.diterima == diterima
        && v.alasan == alasan
        && v.isr_number == isr
}

/// Matriks otoritas dispatch. Berjalan pada vektor **SINTETIS**; ia memanggil
/// [`decide`] (murni) dan **tidak pernah** [`klatos_int_dispatch`], sehingga
/// tidak menggeser penghitung produksi maupun bitmap vektor. Pemakaian
/// produksi dilihat terpisah oleh `init_stage2` lewat [`klatos_int_counter`].
fn run_matrix() -> KlatosIntReport {
    let mut m = Matrix::new();

    // --- kelompok KELAS: vektor yang dapat dicapai -------------------------
    // Kelas PIC-disabled: hanya 7 dan 15 yang punya slot.
    m.catat(cocok(0x27, KELAS_PIC_DISABLED, 0, 1, ALASAN_OK), Group::Kelas);
    m.catat(cocok(0x2f, KELAS_PIC_DISABLED, 1, 1, ALASAN_OK), Group::Kelas);
    // Kelas PIC-disabled yang TIDAK punya slot: ditolak, bukan PANIC anonim.
    m.catat(
        cocok(0x20, KELAS_DITOLAK, 0, 0, ALASAN_PIC_BUKAN_7_ATAU_15),
        Group::Kelas,
    );
    m.catat(
        cocok(0x26, KELAS_DITOLAK, 0, 0, ALASAN_PIC_BUKAN_7_ATAU_15),
        Group::Kelas,
    );
    m.catat(
        cocok(0x28, KELAS_DITOLAK, 0, 0, ALASAN_PIC_BUKAN_7_ATAU_15),
        Group::Kelas,
    );
    m.catat(
        cocok(0x2e, KELAS_DITOLAK, 0, 0, ALASAN_PIC_BUKAN_7_ATAU_15),
        Group::Kelas,
    );
    // Kelas generik: slot == irq, termasuk batas bawah dan batas atas.
    m.catat(cocok(0x50, KELAS_GENERIK, 0, 1, ALASAN_OK), Group::Kelas);
    m.catat(cocok(0x51, KELAS_GENERIK, 1, 1, ALASAN_OK), Group::Kelas);
    m.catat(cocok(0x60, KELAS_GENERIK, 16, 1, ALASAN_OK), Group::Kelas);
    // Vektor APIC yang benar-benar dipakai sistem ini.
    m.catat(cocok(0xfc, KELAS_GENERIK, 172, 1, ALASAN_OK), Group::Kelas);
    m.catat(cocok(0xfd, KELAS_GENERIK, 173, 1, ALASAN_OK), Group::Kelas);
    m.catat(cocok(0xfe, KELAS_GENERIK, 174, 1, ALASAN_OK), Group::Kelas);
    m.catat(cocok(0xff, KELAS_GENERIK, 175, 1, ALASAN_OK), Group::Kelas);

    // --- kelompok TOLAK: batas yang harus ditolak --------------------------
    m.catat(cocok(0x00, KELAS_DITOLAK, 0, 0, ALASAN_DI_BAWAH_BASE), Group::Tolak);
    m.catat(cocok(0x1f, KELAS_DITOLAK, 0, 0, ALASAN_DI_BAWAH_BASE), Group::Tolak);
    // Celah 0x30..0x4f: stub ASM-nya NYATA, jadi vektor ini benar-benar bisa
    // mencapai handle_interrupt. Sebelum P1.05 itu berarti VERIFY -> PANIC.
    m.catat(cocok(0x30, KELAS_DITOLAK, 0, 0, ALASAN_DI_BAWAH_BASE), Group::Tolak);
    m.catat(cocok(0x40, KELAS_DITOLAK, 0, 0, ALASAN_DI_BAWAH_BASE), Group::Tolak);
    m.catat(cocok(0x4f, KELAS_DITOLAK, 0, 0, ALASAN_DI_BAWAH_BASE), Group::Tolak);
    // Batas atas: C++ lama membaca di luar batas di sini.
    m.catat(cocok(0x100, KELAS_DITOLAK, 0, 0, ALASAN_DI_ATAS_BATAS), Group::Tolak);
    m.catat(cocok(0x101, KELAS_DITOLAK, 0, 0, ALASAN_DI_ATAS_BATAS), Group::Tolak);
    m.catat(cocok(0x1000, KELAS_DITOLAK, 0, 0, ALASAN_DI_ATAS_BATAS), Group::Tolak);
    m.catat(cocok(0xffff, KELAS_DITOLAK, 0, 0, ALASAN_DI_ATAS_BATAS), Group::Tolak);

    // --- kelompok KONSISTEN: invarian lintas seluruh ruang input -----------
    // Sapuan menyeluruh 0x00..0x200: diterima IFF (PIC-disabled dengan irq 7
    // atau 15) ATAU (0x50..0xff). Tidak ada input yang lolos tanpa alasan.
    let mut sapuan_benar = true;
    let mut sapuan_diterima = 0u32;
    let mut sapuan_ditolak = 0u32;
    let mut sapuan_slot_benar = true;
    let mut isr = 0u32;
    while isr < 0x200 {
        let v = decide(isr);
        let di_pic = isr >= PIC_DISABLED_VECTOR_BASE && isr <= PIC_DISABLED_VECTOR_END;
        let pic_punya_slot = di_pic && (isr - PIC_DISABLED_VECTOR_BASE == 7 || isr - PIC_DISABLED_VECTOR_BASE == 15);
        let di_generik = isr >= IRQ_VECTOR_BASE && isr < IRQ_VECTOR_BASE + GENERIC_HANDLER_COUNT;
        let harus_diterima = pic_punya_slot || di_generik;
        if v.diterima() != harus_diterima {
            sapuan_benar = false;
        }
        if v.diterima() {
            sapuan_diterima += 1;
        } else {
            sapuan_ditolak += 1;
        }
        // Slot: generik -> slot == irq; PIC -> slot < DISABLED_SLOT_COUNT.
        if v.diterima() {
            let slot_ok = if v.kelas == KELAS_GENERIK {
                v.slot == v.irq
            } else {
                v.slot < DISABLED_SLOT_COUNT
            };
            if !slot_ok {
                sapuan_slot_benar = false;
            }
        }
        // Kelas dan alasan saling konsisten.
        let kelas_konsisten = (v.kelas == KELAS_DITOLAK) == !v.diterima();
        let alasan_konsisten = if v.diterima() {
            v.alasan == ALASAN_OK
        } else {
            v.alasan != ALASAN_OK
        };
        let eoi_konsisten = v.eoi_diminta == v.diterima;
        let tabel_konsisten = v.pakai_disabled == u32::from(v.kelas == KELAS_PIC_DISABLED);
        if !kelas_konsisten || !alasan_konsisten || !eoi_konsisten || !tabel_konsisten {
            sapuan_benar = false;
        }
        isr += 1;
    }
    m.catat(sapuan_benar, Group::Konsisten);
    m.catat(sapuan_slot_benar, Group::Konsisten);
    // Sapuan 0x00..0x1ff = 512 nilai. Diterima = 176 generik + 2 PIC (7, 15)
    // = 178; sisanya 334 ditolak. Angka ini DITURUNKAN dari rentang di atas,
    // bukan disalin dari tempat lain.
    m.catat(sapuan_diterima == 178, Group::Konsisten);
    m.catat(sapuan_ditolak == 334, Group::Konsisten);
    m.catat(sapuan_diterima + sapuan_ditolak == 0x200, Group::Konsisten);
    // Kelas dan alasan tidak pernah tercampur.
    m.catat(decide(0x50).kelas != decide(0x27).kelas, Group::Konsisten);
    m.catat(decide(0x50).pakai_disabled == 0, Group::Konsisten);
    m.catat(decide(0x27).pakai_disabled == 1, Group::Konsisten);
    m.catat(decide(0x27).eoi_diminta == 1, Group::Konsisten);
    m.catat(decide(0x30).eoi_diminta == 0, Group::Konsisten);
    // Idempoten: 1000 keputusan berturut-turut identik.
    let mut last: Option<KlatosIntVerdict> = None;
    let mut idempoten = true;
    let mut i = 0;
    while i < 1000 {
        let v = decide(0xfc);
        match last {
            None => last = Some(v),
            Some(prev) => {
                if prev != v {
                    idempoten = false;
                    break;
                }
            }
        }
        i += 1;
    }
    m.catat(idempoten, Group::Konsisten);

    // --- kelompok NEGATIF: harus BISA gagal --------------------------------
    m.catat(IRQ_VECTOR_BASE == 0x50, Group::Neg);
    m.catat(GENERIC_HANDLER_COUNT == 176, Group::Neg);
    m.catat(PIC_DISABLED_VECTOR_BASE == 0x20, Group::Neg);
    m.catat(PIC_DISABLED_VECTOR_END == 0x2f, Group::Neg);
    m.catat(DISABLED_SLOT_COUNT == 2, Group::Neg);
    m.catat(IDT_VECTORS == 256, Group::Neg);
    m.catat(REPORT_TAG == 0x4B4C_4154_494E_5444, Group::Neg);
    m.catat(KELAS_GENERIK == 0 && KELAS_PIC_DISABLED == 1 && KELAS_DITOLAK == 2, Group::Neg);
    m.catat(
        ALASAN_OK == 0
            && ALASAN_PIC_BUKAN_7_ATAU_15 == 1
            && ALASAN_DI_BAWAH_BASE == 2
            && ALASAN_DI_ATAS_BATAS == 3,
        Group::Neg,
    );
    // Predikat yang SAMA, dua input, dua nilai berbeda: membuktikan bahwa
    // `diterima` benar-benar memisahkan, bukan konstanta.
    m.catat(decide(0x27).diterima == 1, Group::Neg);
    m.catat(decide(0x26).diterima == 0, Group::Neg);
    m.catat(decide(0xff).diterima == 1, Group::Neg);
    m.catat(decide(0x100).diterima == 0, Group::Neg);
    // Rust tidak memiliki IDT — klaim arsitektur MODEL B.
    m.catat(klatos_int_owns_idt() == 0, Group::Neg);
    // Penolakan benar-benar terpicu di dalam matriks ini: himpunan tolak tidak
    // kosong. Tanpa ini, "kelompok tolak lulus" bisa lulus secara hampa.
    m.catat(decide(0x30).alasan != ALASAN_OK, Group::Neg);
    m.catat(decide(0x20).alasan != ALASAN_OK, Group::Neg);
    m.catat(decide(0x100).alasan != ALASAN_OK, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosIntReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        kelas_checks: m.kelas_checks,
        kelas_passed: m.kelas_passed,
        tolak_checks: m.tolak_checks,
        tolak_passed: m.tolak_passed,
        konsisten_checks: m.konsisten_checks,
        konsisten_passed: m.konsisten_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        masuk: klatos_int_counter(Counter::Masuk as u32),
        ditolak: klatos_int_counter(Counter::Ditolak as u32),
        generik: klatos_int_counter(Counter::Generik as u32),
        pic_disabled: klatos_int_counter(Counter::PicDisabled as u32),
        vektor_terpakai: klatos_int_vector_used(),
        irq_vector_base: IRQ_VECTOR_BASE,
        generic_count: GENERIC_HANDLER_COUNT,
        pic_base: PIC_DISABLED_VECTOR_BASE,
        pic_end: PIC_DISABLED_VECTOR_END,
        kelas_generik: KELAS_GENERIK,
        kelas_pic: KELAS_PIC_DISABLED,
        kelas_tolak: KELAS_DITOLAK,
        alasan_pic: ALASAN_PIC_BUKAN_7_ATAU_15,
        alasan_bawah: ALASAN_DI_BAWAH_BASE,
        alasan_atas: ALASAN_DI_ATAS_BATAS,
        status,
    }
}

/// Menjalankan matriks otoritas dispatch interupsi.
#[no_mangle]
pub extern "C" fn klatos_int_selftest() -> KlatosIntReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// P1.05-GIC — otoritas dispatch interupsi untuk aarch64 (MODEL B)
// ---------------------------------------------------------------------------
//
// # Kenapa fungsi TERPISAH, bukan `decide()` yang sama
//
// Pada x86_64 `isr_number` adalah **vektor IDT** (`0x00..=0xff`): keputusan
// memetakannya ke `s_interrupt_handler[isr - 0x50]`, dan rentang `0x20..=0x2f`
// adalah kelas PIC-disabled dengan pemetaan slot BERSILANGAN (`7 -> 0`,
// `15 -> 1`). Pada aarch64 `isr_number` adalah **INTID GIC** (`0..=1019`) dan
// tabel `s_interrupt_handlers[1020]` diindeks **langsung** oleh INTID.
//
// Semantiknya berbeda, jadi keputusannya pun fungsi berbeda. Modul ini TIDAK
// memakai satu pun konstanta APIC/IDT (`IRQ_VECTOR_BASE`, `PIC_DISABLED_*`,
// `GENERIC_HANDLER_COUNT`) di jalur GIC, dan TIDAK menganggap INTID tertentu
// sebagai timer: seluruh INTID sah diperlakukan sama.
//
// # KEPUTUSAN yang dipindahkan (sama bentuknya dengan x86_64)
//
// 1. **Penerimaan** — INTID `0..=1019` diterima; `1020..=1023` (ID khusus GIC:
//    `1020`/`1022` "tidak ada interupsi", `1023` yang di-*filter*
//    `GICv2::pending_interrupt()`) dan seluruh nilai di atasnya **ditolak
//    beralasan**. Sebelum P1.05-GIC dispatcher aarch64 menulis
//    `s_interrupt_handlers[intid]` TANPA batas — untuk INTID `>= 1020` itu
//    pembacaan di luar batas array 1020 elemen.
// 2. **Slot handler** — pada GIC slot = INTID (identitas); tabel tunggal, jadi
//    `pakai_disabled` selalu 0.
// 3. **EOI** — selalu diminta untuk INTID yang diterima. EOI-nya sendiri tetap
//    milik C++ (`GICv2::eoi()` lewat `IRQHandler::eoi()`), TERPISAH dari EOI
//    APIC x86_64.
// 4. **Kelas** — `KELAS_GIC` (nilai `0`, sama dengan `KELAS_GENERIK` supaya
//    arti "diterima pada tabel umum" tetap satu bahasa) atau `KELAS_DITOLAK`.
//
// # Yang TIDAK berpindah
//
// Tabel `s_interrupt_handlers[1020]`, penyimpanan objek handler, pemanggilan
// `increment_call_count`/`handle_interrupt`/`eoi`, GIC distributor/CPU-interface,
// pendaftaran handler, `enter_trap`/`exit_trap`, dan stub ASM GIC.

/// INTID GICv2 sah terbesar **+ 1**. Cermin `Array<GenericInterruptHandler*,
/// 1020>` (`KlatKernel/Efek/Arsitektur/aarch64/Interrupts.cpp`) dan
/// `SHARED_PERIPHERAL_INTERRUPT_RANGE_END`
/// (`KlatKernel/Efek/Arsitektur/aarch64/Interrupts/GICv3.cpp:28`).
pub const GIC_MAX_INTIDS: u32 = 1020;

/// Kelas "INTID GIC sah": nilai `0`, sama dengan [`KELAS_GENERIK`].
pub const KELAS_GIC: u32 = 0;

/// Alasan penolakan untuk GIC: INTID `>= 1020` (ID khusus/spurious) sehingga
/// tidak ada slot tabel yang sah.
pub const ALASAN_INTID_LUAR_BATAS: u32 = 4;

/// Tag laporan GIC. **BERBEDA** dari [`REPORT_TAG`] dengan sengaja: C++ memeriksa
/// tag masing-masing gate sehingga laporan tidak kehilangan daya pisah.
pub const REPORT_TAG_GIC: u64 = 0x4B4C_4154_4749_4331; // "KLATGIC1"

/// Keputusan dispatch untuk satu INTID GIC. **Murni dan total.**
///
/// Total berarti: untuk SETIAP `u32` ia mengembalikan keputusan, termasuk
/// penolakan beralasan — cermin sifat `decide()` pada x86_64. Tidak ada panik,
/// tidak ada pembacaan di luar batas, tidak ada efek samping; penghitung audit
/// dan bitmap di-bump oleh [`klatos_int_dispatch_gic`], bukan di sini.
#[inline]
pub const fn decide_gic(intid: u32) -> KlatosIntVerdict {
    if intid < GIC_MAX_INTIDS {
        KlatosIntVerdict {
            isr_number: intid,
            kelas: KELAS_GIC,
            // Pada GIC nomor IRQ **identitas** dengan INTID.
            irq: intid,
            slot: intid,
            pakai_disabled: 0,
            eoi_diminta: 1,
            diterima: 1,
            alasan: ALASAN_OK,
        }
    } else {
        // Cakupan 1020 bit = 32 kata `u32`; indeks kata di sini selalu sah.
        tolak(intid, ALASAN_INTID_LUAR_BATAS)
    }
}

/// Jumlah kata bitmap INTID GIC. Cakupan `0..=1019` = 1020 bit = 32 kata.
const GIC_VECTOR_WORDS: usize = 32;

/// Bitmap INTID yang benar-benar dilayani pada boot ini (bukan "ada di tabel").
/// TERPISAH dari `VECTOR_SEEN` x86_64 (`0x20..=0xff`): ruang INTID GIC berbeda.
static GIC_VECTOR_SEEN: [AtomicU32; GIC_VECTOR_WORDS] = [const { AtomicU32::new(0) }; GIC_VECTOR_WORDS];

/// Mencatat bahwa `intid` benar-benar dilayani pada boot ini.
fn mark_vector_gic(intid: u32) {
    if intid >= GIC_MAX_INTIDS {
        return;
    }
    let word = (intid / 32) as usize;
    GIC_VECTOR_SEEN[word].fetch_or(1u32 << (intid % 32), Ordering::Relaxed);
}

/// Jumlah INTID BERBEDA yang pernah dilayani sejak boot.
#[no_mangle]
pub extern "C" fn klatos_int_gic_vector_used() -> u32 {
    let mut total = 0;
    let mut index = 0;
    while index < GIC_VECTOR_WORDS {
        total += GIC_VECTOR_SEEN[index].load(Ordering::Relaxed).count_ones();
        index += 1;
    }
    total
}

/// **Pintu produksi aarch64.** Dipanggil dari `handle_interrupt`
/// (`KlatKernel/Efek/Arsitektur/aarch64/Interrupts.cpp`), di dalam penjaga FPU.
///
/// Menghitung peristiwa yang sebenarnya, lalu mengembalikan keputusan. Memakai
/// penghitung audit yang SAMA dengan x86_64 ([`Counter::Masuk`] dan
/// [`Counter::Ditolak`]) supaya invarian tepat-sekali `masuk == ditolak +
/// ditangani` berlaku pada kedua arsitektur. `Counter::Generik` di sini berarti
/// "INTID GIC diterima"; `Counter::PicDisabled` tetap `0` (tidak ada PIC di
/// aarch64).
#[no_mangle]
pub extern "C" fn klatos_int_dispatch_gic(intid: u32) -> KlatosIntVerdict {
    let verdict = decide_gic(intid);
    bump(Counter::Masuk);
    if verdict.diterima() {
        bump(Counter::Generik);
        mark_vector_gic(intid);
    } else {
        bump(Counter::Ditolak);
    }
    verdict
}

/// Keputusan GIC yang sama TANPA menyentuh penghitung produksi. Dipakai audit
/// dan harness supaya "keputusan" dan "pemakaian produksi" dapat dibedakan.
#[no_mangle]
pub extern "C" fn klatos_int_decide_gic(intid: u32) -> KlatosIntVerdict {
    decide_gic(intid)
}

/// INTID GIC sah terbesar + 1; cermin [`GIC_MAX_INTIDS`].
#[no_mangle]
pub extern "C" fn klatos_int_gic_max_intids() -> u32 {
    GIC_MAX_INTIDS
}

/// Tag laporan GIC; cermin [`REPORT_TAG_GIC`].
#[no_mangle]
pub extern "C" fn klatos_int_gic_tag() -> u64 {
    REPORT_TAG_GIC
}

/// Cermin `KlatosIntReportGic` di `KlatKernel/Efek/Kontrak/KlatKernelBridge.h`.
///
/// `tag: u64` @0 lalu 19 `u32` + `status: i32` -> `sizeof == 88`,
/// `alignof == 8`. Tata letaknya dijaga `static_assert` di sisi C++.
#[repr(C)]
pub struct KlatosIntReportGic {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub kelas_checks: u32,
    pub kelas_passed: u32,
    pub tolak_checks: u32,
    pub tolak_passed: u32,
    pub konsisten_checks: u32,
    pub konsisten_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub masuk: u32,
    pub ditolak: u32,
    pub diterima: u32,
    pub intid_terpakai: u32,
    pub max_intids: u32,
    pub kelas_gic: u32,
    pub kelas_tolak: u32,
    pub alasan_luar_batas: u32,
    pub alasan_ok: u32,
    pub status: i32,
}

/// Membandingkan keputusan `intid` dengan kelas/slot/penerimaan/alasan/EOI yang
/// diharapkan.
const fn cocok_gic(
    intid: u32,
    kelas: u32,
    slot: u32,
    diterima: u32,
    alasan: u32,
    eoi: u32,
) -> bool {
    let v = decide_gic(intid);
    v.kelas == kelas
        && v.slot == slot
        && v.diterima == diterima
        && v.alasan == alasan
        && v.eoi_diminta == eoi
        && v.irq == slot
        && v.pakai_disabled == 0
        && v.isr_number == intid
}

/// Matriks otoritas dispatch GIC. Berjalan pada INTID **SINTETIS**; ia memanggil
/// [`decide_gic`] (murni) dan **tidak pernah** [`klatos_int_dispatch_gic`],
/// sehingga tidak menggeser penghitung produksi maupun bitmap.
fn run_matrix_gic() -> KlatosIntReportGic {
    let mut m = Matrix::new();

    // --- kelompok KELAS: INTID yang dapat dicapai --------------------------
    // SGI.
    m.catat(cocok_gic(0, KELAS_GIC, 0, 1, ALASAN_OK, 1), Group::Kelas);
    // PPI: 16 (EL1 physical timer), 27 (EL1 virtual timer — INTID yang dipakai
    // P1.06 pada mesin `virt`), 30 (EL1 physical, non-secure).
    m.catat(cocok_gic(16, KELAS_GIC, 16, 1, ALASAN_OK, 1), Group::Kelas);
    m.catat(cocok_gic(27, KELAS_GIC, 27, 1, ALASAN_OK, 1), Group::Kelas);
    m.catat(cocok_gic(30, KELAS_GIC, 30, 1, ALASAN_OK, 1), Group::Kelas);
    // SPI pertama dan INTID sah terakhir.
    m.catat(cocok_gic(32, KELAS_GIC, 32, 1, ALASAN_OK, 1), Group::Kelas);
    m.catat(cocok_gic(1019, KELAS_GIC, 1019, 1, ALASAN_OK, 1), Group::Kelas);

    // --- kelompok TOLAK: ID khusus GIC dan di atasnya ----------------------
    // 1020 dan 1022: "tidak ada interupsi" pada GICv2/GICv3; 1023: ID spurious
    // yang di-*filter* `GICv2::pending_interrupt()`. Semuanya TIDAK punya slot
    // tabel 1020 entri — dulu pembacaan di luar batas.
    m.catat(cocok_gic(1020, KELAS_DITOLAK, 0, 0, ALASAN_INTID_LUAR_BATAS, 0), Group::Tolak);
    m.catat(cocok_gic(1021, KELAS_DITOLAK, 0, 0, ALASAN_INTID_LUAR_BATAS, 0), Group::Tolak);
    m.catat(cocok_gic(1022, KELAS_DITOLAK, 0, 0, ALASAN_INTID_LUAR_BATAS, 0), Group::Tolak);
    m.catat(cocok_gic(1023, KELAS_DITOLAK, 0, 0, ALASAN_INTID_LUAR_BATAS, 0), Group::Tolak);
    m.catat(cocok_gic(0x1000, KELAS_DITOLAK, 0, 0, ALASAN_INTID_LUAR_BATAS, 0), Group::Tolak);
    m.catat(cocok_gic(0xffff_ffff, KELAS_DITOLAK, 0, 0, ALASAN_INTID_LUAR_BATAS, 0), Group::Tolak);

    // --- kelompok KONSISTEN: invarian lintas seluruh ruang INTID 10 bit ----
    // Sapuan 0..0x400 (1024): diterima IFF `intid < 1020`. Tidak ada input yang
    // lolos tanpa alasan, dan penolakan tidak menamai handler.
    let mut sapuan_benar = true;
    let mut sapuan_diterima = 0u32;
    let mut sapuan_ditolak = 0u32;
    let mut intid = 0u32;
    while intid < 0x400 {
        let v = decide_gic(intid);
        if v.diterima() != (intid < GIC_MAX_INTIDS) {
            sapuan_benar = false;
        }
        if v.diterima() {
            sapuan_diterima += 1;
            if v.slot != intid || v.irq != intid || v.eoi_diminta != 1 || v.kelas != KELAS_GIC {
                sapuan_benar = false;
            }
        } else {
            sapuan_ditolak += 1;
            if v.slot != 0 || v.irq != 0 || v.eoi_diminta != 0 || v.kelas != KELAS_DITOLAK {
                sapuan_benar = false;
            }
        }
        // Kelas dan alasan selalu konsisten dengan `diterima`.
        let kelas_konsisten = (v.kelas == KELAS_DITOLAK) == !v.diterima();
        let alasan_konsisten = if v.diterima() {
            v.alasan == ALASAN_OK
        } else {
            v.alasan != ALASAN_OK
        };
        if !kelas_konsisten || !alasan_konsisten {
            sapuan_benar = false;
        }
        intid += 1;
    }
    m.catat(sapuan_benar, Group::Konsisten);
    // Sapuan 0..0x400 = 1024 nilai. Diterima = 1020 INTID; ditolak = 4.
    m.catat(sapuan_diterima == 1020, Group::Konsisten);
    m.catat(sapuan_ditolak == 4, Group::Konsisten);
    m.catat(sapuan_diterima + sapuan_ditolak == 0x400, Group::Konsisten);
    // Idempoten: 1000 keputusan berturut-turut identik.
    let mut last: Option<KlatosIntVerdict> = None;
    let mut idempoten = true;
    let mut i = 0;
    while i < 1000 {
        let v = decide_gic(27);
        match last {
            None => last = Some(v),
            Some(prev) => {
                if prev != v {
                    idempoten = false;
                    break;
                }
            }
        }
        i += 1;
    }
    m.catat(idempoten, Group::Konsisten);

    // --- kelompok NEGATIF: harus BISA gagal --------------------------------
    m.catat(GIC_MAX_INTIDS == 1020, Group::Neg);
    m.catat(REPORT_TAG_GIC == 0x4B4C_4154_4749_4331, Group::Neg);
    m.catat(KELAS_GIC == 0, Group::Neg);
    m.catat(ALASAN_INTID_LUAR_BATAS != ALASAN_OK, Group::Neg);
    // Predikat yang SAMA, dua input, dua nilai berbeda: membuktikan `diterima`
    // benar-benar memisahkan pada GIC, bukan konstanta.
    m.catat(decide_gic(1019).diterima == 1, Group::Neg);
    m.catat(decide_gic(1020).diterima == 0, Group::Neg);
    m.catat(decide_gic(0).diterima == 1, Group::Neg);
    m.catat(decide_gic(27).alasan == ALASAN_OK, Group::Neg);
    // Penolakan benar-benar terpicu di dalam matriks ini.
    m.catat(decide_gic(1023).alasan != ALASAN_OK, Group::Neg);
    m.catat(decide_gic(0x1000).alasan != ALASAN_OK, Group::Neg);

    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosIntReportGic {
        tag: REPORT_TAG_GIC,
        checks: m.checks,
        passed: m.passed,
        kelas_checks: m.kelas_checks,
        kelas_passed: m.kelas_passed,
        tolak_checks: m.tolak_checks,
        tolak_passed: m.tolak_passed,
        konsisten_checks: m.konsisten_checks,
        konsisten_passed: m.konsisten_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        masuk: read_counter(Counter::Masuk),
        ditolak: read_counter(Counter::Ditolak),
        diterima: read_counter(Counter::Generik),
        intid_terpakai: klatos_int_gic_vector_used(),
        max_intids: GIC_MAX_INTIDS,
        kelas_gic: KELAS_GIC,
        kelas_tolak: KELAS_DITOLAK,
        alasan_luar_batas: ALASAN_INTID_LUAR_BATAS,
        alasan_ok: ALASAN_OK,
        status,
    }
}

/// Menjalankan matriks otoritas dispatch GIC (aarch64).
#[no_mangle]
pub extern "C" fn klatos_int_selftest_gic() -> KlatosIntReportGic {
    run_matrix_gic()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        // Konstanta dasar.
        assert!(IRQ_VECTOR_BASE == 0x50);
        assert!(GENERIC_HANDLER_COUNT == 176);
        assert!(PIC_DISABLED_VECTOR_BASE == 0x20);
        assert!(PIC_DISABLED_VECTOR_END == 0x2f);
        assert!(DISABLED_SLOT_COUNT == 2);
        assert!(IDT_VECTORS == 256);

        // Tag laporan konstan.
        assert!(REPORT_TAG == 0x4B4C_4154_494E_5444);

        // Kode kelas dan alasan tidak boleh bergeser: ia masuk ke C ABI.
        assert!(KELAS_GENERIK == 0);
        assert!(KELAS_PIC_DISABLED == 1);
        assert!(KELAS_DITOLAK == 2);
        assert!(ALASAN_OK == 0);
        assert!(ALASAN_PIC_BUKAN_7_ATAU_15 == 1);
        assert!(ALASAN_DI_BAWAH_BASE == 2);
        assert!(ALASAN_DI_ATAS_BATAS == 3);

        // Urutan penghitung audit stabil.
        assert!(Counter::Masuk as usize == 0);
        assert!(Counter::Generik as usize == 1);
        assert!(Counter::PicDisabled as usize == 2);
        assert!(Counter::Ditolak as usize == 3);

        // Byte-identik dengan C++ historis pada seluruh vektor yang dapat
        // dicapai dari IDT (0x00..=0xff).
        assert!(decide(0x27).diterima == 1);
        assert!(decide(0x27).slot == 0);
        assert!(decide(0x2f).diterima == 1);
        assert!(decide(0x2f).slot == 1);
        assert!(decide(0x50).diterima == 1);
        assert!(decide(0x50).slot == 0);
        assert!(decide(0xff).diterima == 1);
        assert!(decide(0xff).slot == 175);
        assert!(decide(0xfc).slot == 172);
        // Penolakan.
        assert!(decide(0x30).kelas == KELAS_DITOLAK);
        assert!(decide(0x30).alasan == ALASAN_DI_BAWAH_BASE);
        assert!(decide(0x20).kelas == KELAS_DITOLAK);
        assert!(decide(0x20).alasan == ALASAN_PIC_BUKAN_7_ATAU_15);
        assert!(decide(0x100).kelas == KELAS_DITOLAK);
        assert!(decide(0x100).alasan == ALASAN_DI_ATAS_BATAS);

        // Ukuran tipe yang melintasi C ABI.
        assert!(core::mem::size_of::<KlatosIntVerdict>() == 32);
        assert!(core::mem::align_of::<KlatosIntVerdict>() == 4);
        // 8 (tag) + 26 * 4 = 112 byte; alignof = 8 (tag u64), tanpa padding.
        assert!(core::mem::size_of::<KlatosIntReport>() == 112);
        assert!(core::mem::align_of::<KlatosIntReport>() == 8);

        // --- P1.05-GIC (aarch64) ------------------------------------------
        // Konstanta dan tag wajib stabil: nilainya masuk ke C ABI.
        assert!(GIC_MAX_INTIDS == 1020);
        assert!(KELAS_GIC == 0);
        assert!(ALASAN_INTID_LUAR_BATAS == 4);
        assert!(REPORT_TAG_GIC == 0x4B4C_4154_4749_4331);
        // INTID sah: slot identitas, EOI diminta.
        assert!(decide_gic(0).diterima == 1);
        assert!(decide_gic(0).slot == 0);
        assert!(decide_gic(16).slot == 16);
        assert!(decide_gic(27).diterima == 1);
        assert!(decide_gic(27).slot == 27);
        assert!(decide_gic(27).eoi_diminta == 1);
        assert!(decide_gic(1019).diterima == 1);
        assert!(decide_gic(1019).slot == 1019);
        // ID khusus GIC dan di atasnya: ditolak beralasan, tanpa EOI.
        assert!(decide_gic(1020).kelas == KELAS_DITOLAK);
        assert!(decide_gic(1020).alasan == ALASAN_INTID_LUAR_BATAS);
        assert!(decide_gic(1020).slot == 0);
        assert!(decide_gic(1023).diterima == 0);
        assert!(decide_gic(1023).eoi_diminta == 0);
        assert!(decide_gic(0xffff_ffff).diterima == 0);
        // Ukuran tipe laporan GIC yang melintasi C ABI.
        assert!(core::mem::size_of::<KlatosIntReportGic>() == 88);
        assert!(core::mem::align_of::<KlatosIntReportGic>() == 8);
    };
}
