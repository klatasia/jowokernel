// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.39 — OTORITAS ANTREAN PERISTIWA PERANGKAT (Rust).


use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// Tag laporan: "KLATAPRK".
pub const REPORT_TAG: u64 = 0x4B4C_4154_4150_524B;

/// Kapasitas ring, sama dengan `CircularQueue<DeviceEvent, 100>`.
pub const KAPASITAS: usize = 100;
const N_COUNTER: usize = 4;
const RENTANG_DIFERENSIAL: u32 = 100_000;

/// Cermin `DeviceEvent` (`Jowo/Kontrak/DeviceEvent.h`), `#[repr(C)]` 16 byte.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KlatDeviceEvent {
    pub state: i32,
    pub blok: i32,
    pub mayor: u32,
    pub minor: u32,
}

/// Sentinel "tidak ada peristiwa". `DeviceEvent::State` nyata bernilai 1..4.
pub const KOSONG: KlatDeviceEvent = KlatDeviceEvent {
    state: 0,
    blok: 0,
    mayor: 0,
    minor: 0,
};

// ---------------------------------------------------------------------------
// Oracle historis — transkripsi `Paket::CircularQueue`
// ---------------------------------------------------------------------------

/// Model C++ `CircularQueue<DeviceEvent, 100>`, tanpa atomik. Dipakai sebagai
/// oracle diferensial. `masuk` mengembalikan `true` bila slot lama dibuang.
pub struct Model {
    pub head: usize,
    pub size: usize,
    pub slot: [KlatDeviceEvent; KAPASITAS],
}

impl Model {
    pub const fn baru() -> Self {
        Model {
            head: 0,
            size: 0,
            slot: [KOSONG; KAPASITAS],
        }
    }

    pub fn kosong(&self) -> bool {
        self.size == 0
    }

    pub fn masuk(&mut self, e: KlatDeviceEvent) -> bool {
        let penuh = self.size == KAPASITAS;
        let idx = (self.head + self.size) % KAPASITAS;
        self.slot[idx] = e;
        if penuh {
            self.head = (self.head + 1) % KAPASITAS;
        } else {
            self.size += 1;
        }
        penuh
    }

    pub fn keluar(&mut self) -> KlatDeviceEvent {
        if self.size == 0 {
            return KOSONG;
        }
        let e = self.slot[self.head];
        self.head = (self.head + 1) % KAPASITAS;
        self.size -= 1;
        e
    }
}

/// Alias agar nama oracle eksplisit di matriks.
pub fn kebijakan_lama(m: &mut Model, e: KlatDeviceEvent) -> bool {
    m.masuk(e)
}

// ---------------------------------------------------------------------------
// Algoritma TUNGGAL — antrean produksi
// ---------------------------------------------------------------------------

/// Antrean peristiwa perangkat. Satu algoritma dipakai produksi DAN matriks;
/// perbedaannya hanya pada instans (`static PROD` vs instans lokal).
pub struct Antrean<const N: usize> {
    kunci: AtomicBool,
    /// Status interupsi pemegang kunci; hanya pemegang yang menulis.
    kunci_flags: AtomicU64,
    head: AtomicU32,
    size: AtomicU32,
    kata_a: [AtomicU64; N],
    kata_b: [AtomicU64; N],
}

impl<const N: usize> Antrean<N> {
    pub const fn baru() -> Self {
        Antrean {
            kunci: AtomicBool::new(false),
            kunci_flags: AtomicU64::new(0),
            head: AtomicU32::new(0),
            size: AtomicU32::new(0),
            kata_a: [const { AtomicU64::new(0) }; N],
            kata_b: [const { AtomicU64::new(0) }; N],
        }
    }

    fn ambil_kunci(&self) {
        // Matikan interupsi SEBELUM mengambil kunci; lihat arch::simpan_dan_matikan_interupsi.
        let flags = crate::arch::simpan_dan_matikan_interupsi();
        while self
            .kunci
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        self.kunci_flags.store(flags, Ordering::Relaxed);
    }

    fn lepas_kunci(&self) {
        let flags = self.kunci_flags.load(Ordering::Relaxed);
        self.kunci.store(false, Ordering::Release);
        crate::arch::pulihkan_interupsi(flags);
    }

    pub fn kosong(&self) -> bool {
        self.size.load(Ordering::Acquire) == 0
    }

    pub fn jumlah(&self) -> usize {
        self.size.load(Ordering::Acquire) as usize
    }

    /// Menambah peristiwa. Mengembalikan `true` bila slot **tertua** dibuang
    /// (ring penuh), sama dengan perilaku `CircularQueue::enqueue`.
    pub fn masuk(&self, e: KlatDeviceEvent) -> bool {
        self.ambil_kunci();
        let head = self.head.load(Ordering::Relaxed) as usize;
        let size = self.size.load(Ordering::Relaxed) as usize;
        let penuh = size == N;
        let idx = (head + size) % N;
        self.simpan(idx, e);
        if penuh {
            self.head
                .store(((head + 1) % N) as u32, Ordering::Release);
        } else {
            self.size.store((size + 1) as u32, Ordering::Release);
        }
        self.lepas_kunci();
        penuh
    }

    /// Mengambil peristiwa tertua. Mengembalikan [`KOSONG`] bila antrean kosong.
    pub fn keluar(&self) -> KlatDeviceEvent {
        self.ambil_kunci();
        let head = self.head.load(Ordering::Relaxed) as usize;
        let size = self.size.load(Ordering::Relaxed) as usize;
        let hasil = if size == 0 {
            KOSONG
        } else {
            let e = self.muat(head);
            self.head
                .store(((head + 1) % N) as u32, Ordering::Release);
            self.size.store((size - 1) as u32, Ordering::Release);
            e
        };
        self.lepas_kunci();
        hasil
    }

    fn simpan(&self, i: usize, e: KlatDeviceEvent) {
        let a = (e.state as u32 as u64) | ((e.blok as u32 as u64) << 32);
        let b = (e.mayor as u64) | ((e.minor as u64) << 32);
        self.kata_a[i].store(a, Ordering::Relaxed);
        self.kata_b[i].store(b, Ordering::Relaxed);
    }

    fn muat(&self, i: usize) -> KlatDeviceEvent {
        let a = self.kata_a[i].load(Ordering::Relaxed);
        let b = self.kata_b[i].load(Ordering::Relaxed);
        KlatDeviceEvent {
            state: a as u32 as i32,
            blok: (a >> 32) as u32 as i32,
            mayor: b as u32,
            minor: (b >> 32) as u32,
        }
    }
}

// ---------------------------------------------------------------------------
// Penyimpanan PRODUKSI
// ---------------------------------------------------------------------------

/// Antrean peristiwa perangkat global. Menggantikan
/// `AllDevicesDetails::event_queue`.
static PROD: Antrean<KAPASITAS> = Antrean::baru();

/// `0` = masuk, `1` = keluar, `2` = luapan (drop tertua), `3` = keluar-kosong.
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

// ---------------------------------------------------------------------------
// Permukaan C ABI — PRODUKSI
// ---------------------------------------------------------------------------

/// Menambah peristiwa perangkat. Mengembalikan jumlah peristiwa setelahnya.
#[no_mangle]
pub extern "C" fn klatos_antrean_perangkat_masuk(
    state: u32,
    blok: u32,
    mayor: u32,
    minor: u32,
) -> u32 {
    let e = KlatDeviceEvent {
        state: state as i32,
        blok: blok as i32,
        mayor,
        minor,
    };
    let penuh = PROD.masuk(e);
    COUNTERS[0].fetch_add(1, Ordering::Relaxed);
    if penuh {
        COUNTERS[2].fetch_add(1, Ordering::Relaxed);
    }
    PROD.jumlah() as u32
}

/// Mengambil peristiwa tertua. `state == 0` berarti antrean kosong.
#[no_mangle]
pub extern "C" fn klatos_antrean_perangkat_keluar() -> KlatDeviceEvent {
    let e = PROD.keluar();
    COUNTERS[1].fetch_add(1, Ordering::Relaxed);
    if e.state == 0 {
        COUNTERS[3].fetch_add(1, Ordering::Relaxed);
    }
    e
}

/// Jumlah peristiwa dalam antrean produksi.
#[no_mangle]
pub extern "C" fn klatos_antrean_perangkat_jumlah() -> u32 {
    PROD.jumlah() as u32
}

/// `1` bila antrean produksi kosong, `0` bila tidak.
#[no_mangle]
pub extern "C" fn klatos_antrean_perangkat_kosong() -> u32 {
    if PROD.kosong() {
        1
    } else {
        0
    }
}

/// Nilai satu penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_antrean_perangkat_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosAntreanReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
#[repr(C)]
pub struct KlatosAntreanReport {
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

struct Matrix {
    checks: u32,
    passed: u32,
    grup_checks: [u32; 3],
    grup_passed: [u32; 3],
}

impl Matrix {
    const fn baru() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            grup_checks: [0; 3],
            grup_passed: [0; 3],
        }
    }
    fn catat(&mut self, lulus: bool, grup: usize) {
        self.checks += 1;
        self.grup_checks[grup] += 1;
        if lulus {
            self.passed += 1;
            self.grup_passed[grup] += 1;
        }
    }
}

fn ev(state: i32, blok: i32, mayor: u32, minor: u32) -> KlatDeviceEvent {
    KlatDeviceEvent {
        state,
        blok,
        mayor,
        minor,
    }
}

fn run_matrix() -> KlatosAntreanReport {
    let mut m = Matrix::baru();

    let prod_jumlah_awal = PROD.jumlah();
    let prod_counter_awal = [
        klatos_antrean_perangkat_counter(0),
        klatos_antrean_perangkat_counter(1),
        klatos_antrean_perangkat_counter(2),
        klatos_antrean_perangkat_counter(3),
    ];

    // --- kelompok 0: dasar + kapasitas + luapan + bungkus -------------------
    {
        let a = Antrean::<KAPASITAS>::baru();
        m.catat(a.kosong() && a.jumlah() == 0, 0);

        // masuk/keluar satu peristiwa
        m.catat(!a.masuk(ev(2, 0, 1, 3)), 0); // tidak penuh
        m.catat(!a.kosong() && a.jumlah() == 1, 0);
        m.catat(a.keluar() == ev(2, 0, 1, 3), 0);
        m.catat(a.kosong() && a.jumlah() == 0, 0);

        // FIFO
        let b = Antrean::<KAPASITAS>::baru();
        (void_masuk(&b, ev(1, 0, 8, 1)));
        (void_masuk(&b, ev(2, 1, 8, 2)));
        (void_masuk(&b, ev(1, 0, 8, 3)));
        m.catat(
            b.keluar() == ev(1, 0, 8, 1)
                && b.keluar() == ev(2, 1, 8, 2)
                && b.keluar() == ev(1, 0, 8, 3),
            0,
        );
        m.catat(b.kosong(), 0);

        // kapasitas tepat 100: tidak ada luapan
        let c = Antrean::<KAPASITAS>::baru();
        let mut i = 0u32;
        let mut ada_luapan = false;
        while i < KAPASITAS as u32 {
            if c.masuk(ev(2, (i & 1) as i32, i, i ^ 0xAAAA)) {
                ada_luapan = true;
            }
            i += 1;
        }
        m.catat(!ada_luapan && c.jumlah() == KAPASITAS, 0);
        // peristiwa ke-101 membuang yang TERTUA
        m.catat(c.masuk(ev(2, 0, 0xFFFF_FFFF, 0xFFFF_FFFF)), 0);
        m.catat(c.jumlah() == KAPASITAS, 0);
        m.catat(c.keluar() == ev(2, 1, 1, 1 ^ 0xAAAA), 0); // #0 dibuang, #1 tersisa

        // 150 masuk: sisa #50..#149
        let d = Antrean::<KAPASITAS>::baru();
        let mut k = 0u32;
        while k < 150 {
            (void_masuk(&d, ev(2, 0, k, k)));
            k += 1;
        }
        m.catat(d.jumlah() == KAPASITAS, 0);
        let pertama = d.keluar();
        m.catat(pertama == ev(2, 0, 50, 50), 0);
        let mut terakhir = pertama;
        let mut n = 1u32;
        while n < KAPASITAS as u32 {
            terakhir = d.keluar();
            n += 1;
        }
        m.catat(terakhir == ev(2, 0, 149, 149), 0);
        m.catat(d.kosong(), 0);

        // bungkus setelah kosong
        let mut p = 0u32;
        while p < 5 {
            (void_masuk(&d, ev(1, 1, p, p + 1)));
            p += 1;
        }
        m.catat(
            d.keluar() == ev(1, 1, 0, 1)
                && d.keluar() == ev(1, 1, 1, 2)
                && d.keluar() == ev(1, 1, 2, 3),
            0,
        );
    }

    // --- kelompok 1: diferensial terhadap oracle historis -------------------
    let mut diukur = 0u32;
    let mut beda = 0u32;
    {
        let uji = Antrean::<KAPASITAS>::baru();
        let mut model = Model::baru();
        let mut x = 0x1234_5678u32;
        let mut i = 0u32;
        while i < RENTANG_DIFERENSIAL {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let op = (x >> 16) & 1;
            if op == 1 || model.kosong() {
                let e = ev(
                    ((x >> 8) & 0x7) as i32,
                    (x & 1) as i32,
                    x,
                    x ^ 0x5A5A_5A5A,
                );
                let p1 = uji.masuk(e);
                let p2 = kebijakan_lama(&mut model, e);
                if p1 != p2 {
                    beda += 1;
                }
            } else {
                let a1 = uji.keluar();
                let a2 = model.keluar();
                if a1 != a2 {
                    beda += 1;
                }
            }
            if uji.jumlah() != model.size || uji.kosong() != model.kosong() {
                beda += 1;
            }
            diukur += 1;
            i += 1;
        }
        m.catat(beda == 0, 1);
        m.catat(diukur == RENTANG_DIFERENSIAL, 1);
    }

    // --- kelompok 2: negatif + kontrol -------------------------------------
    {
        // keluar dari antrean kosong -> KOSONG
        let z = Antrean::<KAPASITAS>::baru();
        m.catat(z.keluar() == KOSONG, 2);
        m.catat(z.jumlah() == 0, 2);

        // pemak/unpak byte-identik untuk nilai ekstrem
        let e = Antrean::<KAPASITAS>::baru();
        let ekstrem = ev(-1, 0x7FFF_FFFF, 0xFFFF_FFFF, 0);
        (void_masuk(&e, ekstrem));
        m.catat(e.keluar() == ekstrem, 2);
        let ekstrem2 = ev(i32::MIN, i32::MIN, 0, 0xFFFF_FFFF);
        (void_masuk(&e, ekstrem2));
        m.catat(e.keluar() == ekstrem2, 2);

        // luapan membuang TERTUA, bukan terbaru
        let o = Antrean::<KAPASITAS>::baru();
        let mut i = 0u32;
        while i < KAPASITAS as u32 {
            (void_masuk(&o, ev(2, 0, i, 0)));
            i += 1;
        }
        (void_masuk(&o, ev(2, 0, 100, 0)));
        m.catat(o.keluar() == ev(2, 0, 1, 0), 2);

        // KONTROL: keadaan PRODUKSI tidak bergeser oleh matriks.
        m.catat(PROD.jumlah() == prod_jumlah_awal, 2);
        m.catat(
            klatos_antrean_perangkat_counter(0) == prod_counter_awal[0]
                && klatos_antrean_perangkat_counter(1) == prod_counter_awal[1]
                && klatos_antrean_perangkat_counter(2) == prod_counter_awal[2]
                && klatos_antrean_perangkat_counter(3) == prod_counter_awal[3],
            2,
        );
    }

    let mut report = KlatosAntreanReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.grup_checks[1],
        peta_passed: m.grup_passed[1],
        neg_checks: m.grup_checks[2],
        neg_passed: m.grup_passed[2],
        prod_a: klatos_antrean_perangkat_counter(0),
        prod_b: klatos_antrean_perangkat_counter(1),
        prod_c: klatos_antrean_perangkat_counter(2),
        diferensial_diukur: diukur,
        diferensial_beda: beda,
        status: 0,
    };
    if report.checks != report.passed {
        report.status = -1;
    }
    report
}

/// Pembantu: `masuk` yang hasilnya diabaikan (menghindari `must_use`).
fn void_masuk(a: &Antrean<KAPASITAS>, e: KlatDeviceEvent) {
    let _ = a.masuk(e);
}

/// Menjalankan matriks otoritas antrean peristiwa perangkat.
#[no_mangle]
pub extern "C" fn klatos_antrean_perangkat_selftest() -> KlatosAntreanReport {
    run_matrix()
}

/// Konformansi waktu-kompilasi.
pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_4150_524B); // "KLATAPRK"
        assert!(KAPASITAS == 100);
        assert!(N_COUNTER == 4);
        assert!(core::mem::size_of::<KlatDeviceEvent>() == 16);
        assert!(core::mem::size_of::<KlatosAntreanReport>() == 56);
        assert!(KOSONG.state == 0);
        // Oracle: luapan membuang tertua (cabang `head`), bukan terbaru.
        // Model kapasitas penuh: setelah 100 masuk lalu 1 lagi, head maju.
        // (Diuji runtime di matriks; di sini hanya invarian lebar.)
        assert!(core::mem::size_of::<Model>() == KAPASITAS * 16 + 16);
    };
}
