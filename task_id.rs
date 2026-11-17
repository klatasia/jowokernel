// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Otoritas identitas task (P1.00) — ruang id TUNGGAL untuk PID dan TID.


use core::cell::Cell;
use core::sync::atomic::{AtomicU32, Ordering};

/// Id yang berarti "tidak ada task". Sama dengan `task::TASK_NONE`.
pub const TASK_ID_NONE: u32 = 0;

/// Batas atas id. `pid_t` adalah `i32` bertanda, jadi nilai negatif tidak
/// pernah sah dan bit tanda tidak boleh dipakai.
pub const TASK_ID_MAX: u32 = 0x7FFF_FFFF;

/// Kapasitas himpunan hidup yang dilacak (bit). 8192 bit = 1 KiB.
///
/// Id di atas kapasitas ini tetap SAH dan tetap diberikan; ia hanya tidak
/// dilacak keunikannya. Pada KLATOS jumlah task hidup tidak pernah mendekati
/// angka ini (diukur: puluhan), dan batas ini dinyatakan terbuka, bukan
/// disembunyikan.
pub const LIVE_CAPACITY: u32 = 8192;

const LIVE_WORDS: usize = (LIVE_CAPACITY as usize) / 32;
const LIVE_BIT: u32 = 31;
const N_COUNTER: usize = 6;

/// Tag laporan. BERBEDA dari `frame::REPORT_TAG` ("KLATKRNL") dengan sengaja:
/// laporan identitas task dan laporan frame adalah dua laporan yang berbeda,
/// dan C++ memeriksanya lewat konstanta yang berbeda pula
/// (`task_id_tag_expected` vs `kernel_tag_expected`). Menyamakan tag akan
/// membuat pemeriksaan tag kehilangan daya pisahnya.
const REPORT_TAG: u64 = 0x4B4C_4154_544B_4944; // "KLATTKID"

/// Galat otoritas identitas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    /// Id 0, atau di atas [`TASK_ID_MAX`].
    TidakSah,
    /// Penghitung id habis (overflow). Ditolak, bukan UB.
    Habis,
    /// Id sudah hidup; tidak boleh diberikan ulang.
    SudahHidup,
}

impl IdError {
    /// Kode C ABI: negatif, sehingga `>= 0` berarti id yang sah.
    pub const fn kode(&self) -> i32 {
        match self {
            IdError::TidakSah => -1,
            IdError::Habis => -2,
            IdError::SudahHidup => -3,
        }
    }
}

/// Penghitung untuk audit (§21). Urutannya dipakai oleh
/// `klatos_task_id_counter`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    Alloc = 0,
    Reserve = 1,
    Release = 2,
    Skip = 3,
    Overflow = 4,
    Err = 5,
}

/// `true` bila id ini dilacak keunikannya.
pub const fn tracked(id: u32) -> bool {
    id < LIVE_CAPACITY
}

fn word_of(id: u32) -> usize {
    (id / 32) as usize
}

fn bit_of(id: u32) -> u32 {
    1u32 << (id & LIVE_BIT)
}

/// Penyimpanan otoritas identitas. Algoritmanya tidak ada di sini — ia ada di
/// [`alloc_in`]/[`reserve_in`]/[`release_in`].
pub trait IdStore {
    fn next_load(&self) -> u32;
    fn next_cas(&self, cur: u32, new: u32) -> bool;
    fn live_test(&self, id: u32) -> bool;
    /// `true` bila bit SUDAH menyala sebelum panggilan ini.
    fn live_set(&self, id: u32) -> bool;
    /// `true` bila bit TADI menyala.
    fn live_clear(&self, id: u32) -> bool;
    fn bump(&self, counter: Counter);
}

// ---------------------------------------------------------------------------
// Algoritma — SATU salinan, dipakai produksi dan matriks
// ---------------------------------------------------------------------------

/// Mengambil id berikutnya.
///
/// Urutan: penghitung naik satu; 0 dilewati; id yang masih hidup dilewati
/// (jaring pengaman — tidak pernah terjadi pada kebijakan monoton); id di luar
/// kapasitas pelacakan diberikan tanpa dilacak; overflow ditolak.
pub fn alloc_in<S: IdStore>(s: &S) -> Result<u32, IdError> {
    let mut tries: u32 = 0;
    loop {
        let cur = s.next_load();
        if cur > TASK_ID_MAX {
            s.bump(Counter::Overflow);
            s.bump(Counter::Err);
            return Err(IdError::Habis);
        }
        if !s.next_cas(cur, cur + 1) {
            // Balapan dengan pengambil lain; baca ulang.
            continue;
        }
        let id = cur;
        if id == TASK_ID_NONE {
            continue;
        }
        if !tracked(id) {
            s.bump(Counter::Alloc);
            return Ok(id);
        }
        if s.live_set(id) {
            s.bump(Counter::Skip);
            tries += 1;
            if tries > LIVE_CAPACITY {
                s.bump(Counter::Err);
                return Err(IdError::Habis);
            }
            continue;
        }
        s.bump(Counter::Alloc);
        return Ok(id);
    }
}

/// Meminta id TERTENTU (kasus thread-pertama: TID == PID).
pub fn reserve_in<S: IdStore>(s: &S, id: u32) -> Result<(), IdError> {
    if id == TASK_ID_NONE || id > TASK_ID_MAX {
        s.bump(Counter::Err);
        return Err(IdError::TidakSah);
    }
    if s.live_set(id) {
        s.bump(Counter::Err);
        return Err(IdError::SudahHidup);
    }
    s.bump(Counter::Reserve);
    Ok(())
}

/// Melepas id. **Idempoten**: melepas id yang sudah bebas bukan galat, karena
/// jalur kematian task (thread dan proses) dapat tumpang tindih pada id yang
/// sama untuk thread pertama.
pub fn release_in<S: IdStore>(s: &S, id: u32) -> Result<(), IdError> {
    if id == TASK_ID_NONE || id > TASK_ID_MAX {
        s.bump(Counter::Err);
        return Err(IdError::TidakSah);
    }
    s.live_clear(id);
    s.bump(Counter::Release);
    Ok(())
}

/// Apakah id ini task yang hidup?
pub fn is_live_in<S: IdStore>(s: &S, id: u32) -> bool {
    id != TASK_ID_NONE && id <= TASK_ID_MAX && s.live_test(id)
}

// ---------------------------------------------------------------------------
// Penyimpanan PRODUKSI (statik, atomik, tanpa kunci)
// ---------------------------------------------------------------------------

struct AtomicStore;

static NEXT_ID: AtomicU32 = AtomicU32::new(0);
static LIVE: [AtomicU32; LIVE_WORDS] = [const { AtomicU32::new(0) }; LIVE_WORDS];
static COUNTERS: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];

impl IdStore for AtomicStore {
    fn next_load(&self) -> u32 {
        NEXT_ID.load(Ordering::Acquire)
    }
    fn next_cas(&self, cur: u32, new: u32) -> bool {
        NEXT_ID
            .compare_exchange(cur, new, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    fn live_test(&self, id: u32) -> bool {
        if !tracked(id) {
            return false;
        }
        LIVE[word_of(id)].load(Ordering::Acquire) & bit_of(id) != 0
    }
    fn live_set(&self, id: u32) -> bool {
        if !tracked(id) {
            return false;
        }
        let w = word_of(id);
        let b = bit_of(id);
        loop {
            let old = LIVE[w].load(Ordering::Acquire);
            if old & b != 0 {
                return true;
            }
            if LIVE[w]
                .compare_exchange(old, old | b, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return false;
            }
        }
    }
    fn live_clear(&self, id: u32) -> bool {
        if !tracked(id) {
            return false;
        }
        let w = word_of(id);
        let b = bit_of(id);
        loop {
            let old = LIVE[w].load(Ordering::Acquire);
            if old & b == 0 {
                return false;
            }
            if LIVE[w]
                .compare_exchange(old, old & !b, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return true;
            }
        }
    }
    fn bump(&self, counter: Counter) {
        COUNTERS[counter as usize].fetch_add(1, Ordering::Relaxed);
    }
}

static PROD: AtomicStore = AtomicStore;

// ---------------------------------------------------------------------------
// Penyimpanan SINTETIS (stack, tanpa atomik) — hanya untuk matriks
// ---------------------------------------------------------------------------

/// Ruang id sintetis. Dipakai matriks supaya matriks TIDAK menyentuh ruang id
/// produksi; lihat kontrol `prod_next` di [`run_matrix`].
pub struct PlainStore {
    next: Cell<u32>,
    live: [Cell<u32>; LIVE_WORDS],
    counters: [Cell<u32>; N_COUNTER],
}

impl PlainStore {
    pub const fn new() -> Self {
        PlainStore {
            next: Cell::new(0),
            live: [const { Cell::new(0) }; LIVE_WORDS],
            counters: [const { Cell::new(0) }; N_COUNTER],
        }
    }
    /// Menaruh penghitung pada nilai tertentu, supaya batas (overflow) dapat
    /// diuji tanpa mengulang miliaran kali.
    pub fn seed_next(&self, value: u32) {
        self.next.set(value);
    }
    pub fn next_value(&self) -> u32 {
        self.next.get()
    }
    pub fn counter(&self, counter: Counter) -> u32 {
        self.counters[counter as usize].get()
    }
}

impl IdStore for PlainStore {
    fn next_load(&self) -> u32 {
        self.next.get()
    }
    fn next_cas(&self, cur: u32, new: u32) -> bool {
        if self.next.get() == cur {
            self.next.set(new);
            true
        } else {
            false
        }
    }
    fn live_test(&self, id: u32) -> bool {
        if !tracked(id) {
            return false;
        }
        self.live[word_of(id)].get() & bit_of(id) != 0
    }
    fn live_set(&self, id: u32) -> bool {
        if !tracked(id) {
            return false;
        }
        let w = word_of(id);
        let old = self.live[w].get();
        self.live[w].set(old | bit_of(id));
        old & bit_of(id) != 0
    }
    fn live_clear(&self, id: u32) -> bool {
        if !tracked(id) {
            return false;
        }
        let w = word_of(id);
        let old = self.live[w].get();
        self.live[w].set(old & !bit_of(id));
        old & bit_of(id) != 0
    }
    fn bump(&self, counter: Counter) {
        let i = counter as usize;
        self.counters[i].set(self.counters[i].get() + 1);
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Mengembalikan penghitung ke 0 dan mengosongkan himpunan hidup. Dipanggil
/// dari `Process::initialize()` — menggantikan `next_pid.store(0)`.
#[no_mangle]
pub extern "C" fn klatos_task_id_reset() -> u32 {
    NEXT_ID.store(0, Ordering::Release);
    for word in LIVE.iter() {
        word.store(0, Ordering::Release);
    }
    for counter in COUNTERS.iter() {
        counter.store(0, Ordering::Release);
    }
    TASK_ID_NONE
}

/// Id berikutnya, atau kode galat negatif.
#[no_mangle]
pub extern "C" fn klatos_task_id_alloc() -> i32 {
    match alloc_in(&PROD) {
        Ok(id) => id as i32,
        Err(e) => e.kode(),
    }
}

/// Meminta id tertentu. `0` berarti berhasil.
#[no_mangle]
pub extern "C" fn klatos_task_id_reserve(id: u32) -> i32 {
    match reserve_in(&PROD, id) {
        Ok(()) => 0,
        Err(e) => e.kode(),
    }
}

/// Melepas id. `0` berarti berhasil (idempoten).
#[no_mangle]
pub extern "C" fn klatos_task_id_release(id: u32) -> i32 {
    match release_in(&PROD, id) {
        Ok(()) => 0,
        Err(e) => e.kode(),
    }
}

/// `1` bila id hidup, `0` bila tidak.
#[no_mangle]
pub extern "C" fn klatos_task_id_is_live(id: u32) -> u32 {
    u32::from(is_live_in(&PROD, id))
}

/// Penghitung id berikutnya. Dipakai laporan dan audit.
#[no_mangle]
pub extern "C" fn klatos_task_id_next() -> u32 {
    PROD.next_load()
}

/// Jumlah id hidup. Dihitung dari bitmap (bukan penghitung terpelihara),
/// supaya angka ini adalah kebenaran langsung, bukan salinan yang bisa
/// menyimpang.
#[no_mangle]
pub extern "C" fn klatos_task_id_count() -> u32 {
    let mut n = 0u32;
    for word in LIVE.iter() {
        n += word.load(Ordering::Relaxed).count_ones();
    }
    n
}

/// Nilai satu penghitung audit. Lihat [`Counter`].
#[no_mangle]
pub extern "C" fn klatos_task_id_counter(which: u32) -> u32 {
    if (which as usize) >= N_COUNTER {
        return 0;
    }
    COUNTERS[which as usize].load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Laporan + matriks
// ---------------------------------------------------------------------------

/// Cermin `KlatosTaskIdReport` di `Jowo/Kontrak/KlatKernelBridge.h`.
/// Tata letaknya dijaga `static_assert` di sisi C++ dan diuji harness host.
#[repr(C)]
pub struct KlatosTaskIdReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub alloc_checks: u32,
    pub alloc_passed: u32,
    pub reserve_checks: u32,
    pub reserve_passed: u32,
    pub free_checks: u32,
    pub free_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub live_count: u32,
    pub prod_next: u32,
    pub alloc_skip: u32,
    pub overflow: u32,
    pub err: u32,
    pub status: i32,
}

struct Matrix {
    checks: u32,
    passed: u32,
    group_checks: [u32; 4],
    group_passed: [u32; 4],
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            group_checks: [0; 4],
            group_passed: [0; 4],
        }
    }
    fn catat(&mut self, lulus: bool, group: usize) {
        self.checks += 1;
        self.group_checks[group] += 1;
        if lulus {
            self.passed += 1;
            self.group_passed[group] += 1;
        }
    }
}

/// Matriks otoritas identitas. Berjalan pada ruang id SINTETIS.
fn run_matrix() -> KlatosTaskIdReport {
    let mut m = Matrix::new();

    // Kontrol: matriks tidak boleh menggeser ruang id produksi.
    let prod_next_before = PROD.next_load();
    let prod_live_before = klatos_task_id_count();

    // --- kelompok 0: alokasi ------------------------------------------------
    let s = PlainStore::new();
    m.catat(alloc_in(&s) == Ok(1), 0); // 0 dilewati
    m.catat(alloc_in(&s) == Ok(2), 0);
    m.catat(alloc_in(&s) == Ok(3), 0);
    m.catat(
        is_live_in(&s, 1) && is_live_in(&s, 2) && is_live_in(&s, 3),
        0,
    );
    m.catat(!is_live_in(&s, 4), 0);
    m.catat(!is_live_in(&s, TASK_ID_NONE), 0);
    m.catat(s.next_value() == 4, 0);
    m.catat(s.counter(Counter::Alloc) == 3, 0);
    m.catat(s.counter(Counter::Skip) == 0, 0);

    // --- kelompok 1: reservasi ---------------------------------------------
    m.catat(reserve_in(&s, 100) == Ok(()), 1);
    m.catat(is_live_in(&s, 100), 1);
    m.catat(reserve_in(&s, 100) == Err(IdError::SudahHidup), 1);
    m.catat(s.counter(Counter::Reserve) == 1, 1);
    m.catat(reserve_in(&s, 2) == Err(IdError::SudahHidup), 1);
    m.catat(reserve_in(&s, 1) == Err(IdError::SudahHidup), 1);

    // --- kelompok 2: pelepasan ---------------------------------------------
    m.catat(release_in(&s, 100) == Ok(()), 2);
    m.catat(!is_live_in(&s, 100), 2);
    m.catat(release_in(&s, 100) == Ok(()), 2); // idempoten
    m.catat(s.counter(Counter::Release) == 2, 2);
    m.catat(is_live_in(&s, 2), 2); // pelepasan tidak menyentuh id lain

    // --- kelompok 3: negatif (harus BISA gagal) ----------------------------
    // (a) 0 haram; di atas batas ditolak.
    m.catat(reserve_in(&s, TASK_ID_NONE) == Err(IdError::TidakSah), 3);
    m.catat(reserve_in(&s, TASK_ID_MAX + 1) == Err(IdError::TidakSah), 3);
    m.catat(release_in(&s, TASK_ID_NONE) == Err(IdError::TidakSah), 3);
    m.catat(release_in(&s, TASK_ID_MAX + 1) == Err(IdError::TidakSah), 3);
    m.catat(!is_live_in(&s, TASK_ID_MAX + 1), 3);

    // (b) overflow DITOLAK, bukan UB.
    //     Penghitung ditaruh TEPAT di TASK_ID_MAX: alokasi berikutnya
    //     mengembalikan TASK_ID_MAX, lalu yang sesudahnya WAJIB menolak.
    let ov = PlainStore::new();
    ov.seed_next(TASK_ID_MAX);
    m.catat(alloc_in(&ov) == Ok(TASK_ID_MAX), 3);
    m.catat(alloc_in(&ov) == Err(IdError::Habis), 3);
    m.catat(ov.counter(Counter::Overflow) >= 1, 3);
    m.catat(alloc_in(&ov) == Err(IdError::Habis), 3); // tetap menolak

    // (c) id hidup TIDAK PERNAH diberikan ulang.
    let dup = PlainStore::new();
    m.catat(reserve_in(&dup, 5) == Ok(()), 3);
    dup.seed_next(5);
    m.catat(alloc_in(&dup) == Ok(6), 3); // 5 dilewati
    m.catat(dup.counter(Counter::Skip) == 1, 3);
    m.catat(is_live_in(&dup, 6), 3);
    m.catat(is_live_in(&dup, 5), 3); // 5 tetap hidup
    dup.seed_next(6);
    m.catat(alloc_in(&dup) == Ok(7), 3); // 6 juga dilewati
    m.catat(dup.counter(Counter::Skip) == 2, 3);

    // (d) di luar kapasitas pelacakan: tetap sah, tidak dilacak. Batas ini
    //     dinyatakan, bukan disembunyikan.
    let big = PlainStore::new();
    let big_id = LIVE_CAPACITY + 10;
    big.seed_next(big_id);
    m.catat(alloc_in(&big) == Ok(big_id), 3);
    m.catat(!tracked(big_id), 3);
    m.catat(!is_live_in(&big, big_id), 3);
    m.catat(reserve_in(&big, big_id + 1) == Ok(()), 3);

    // (e) KONTROL matriks: ruang id produksi tidak bergeser.
    m.catat(PROD.next_load() == prod_next_before, 3);
    m.catat(klatos_task_id_count() == prod_live_before, 3);

    // Penghitung yang dilaporkan adalah penghitung ruang SINTETIS, bukan
    // penghitung produksi: laporan ini menggambarkan apa yang MATRIKS lakukan.
    // Penghitung produksi tersedia terpisah lewat `klatos_task_id_counter`.
    // (Kesalahan pertama gate ini justru membaca penghitung produksi di sini,
    // sehingga `alloc_skip`/`overflow` selalu 0 dan tiga kasus negatif gagal —
    // kontrol yang tidak dapat gagal.)
    let synth_skip = dup.counter(Counter::Skip);
    let synth_overflow = ov.counter(Counter::Overflow);
    let synth_err = s.counter(Counter::Err)
        + ov.counter(Counter::Err)
        + dup.counter(Counter::Err)
        + big.counter(Counter::Err);

    let mut report = KlatosTaskIdReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        alloc_checks: m.group_checks[0],
        alloc_passed: m.group_passed[0],
        reserve_checks: m.group_checks[1],
        reserve_passed: m.group_passed[1],
        free_checks: m.group_checks[2],
        free_passed: m.group_passed[2],
        neg_checks: m.group_checks[3],
        neg_passed: m.group_passed[3],
        live_count: klatos_task_id_count(),
        prod_next: PROD.next_load(),
        alloc_skip: synth_skip,
        overflow: synth_overflow,
        err: synth_err,
        status: 0,
    };
    if report.checks != report.passed {
        report.status = -1;
    }
    report
}

/// Menjalankan matriks otoritas identitas.
#[no_mangle]
pub extern "C" fn klatos_task_id_selftest() -> KlatosTaskIdReport {
    run_matrix()
}

/// Konformansi waktu-kompilasi.
pub mod conformance {
    use super::*;

    const _: () = {
        // Batas-batas dasar.
        assert!(TASK_ID_NONE == 0);
        assert!(TASK_ID_MAX == 0x7FFF_FFFF);
        assert!(LIVE_WORDS == 256);
        assert!(LIVE_CAPACITY == 8192);

        // `tracked` tepat pada batas.
        assert!(tracked(0));
        assert!(tracked(LIVE_CAPACITY - 1));
        assert!(!tracked(LIVE_CAPACITY));

        // 0 tidak pernah dilacak sebagai "hidup" walau bit-nya ada.
        // (Bit 0 dipakai, tetapi `is_live_in` menolak id 0 lebih awal.)

        // Ukuran laporan: 8 + 16*4 = 72 byte. Sama dengan cermin C++.
        assert!(core::mem::size_of::<KlatosTaskIdReport>() == 72);

        // Kode galat selalu negatif, sehingga `>= 0` berarti id sah.
        assert!(IdError::TidakSah.kode() < 0);
        assert!(IdError::Habis.kode() < 0);
        assert!(IdError::SudahHidup.kode() < 0);

        // Urutan penghitung audit stabil (dipakai `klatos_task_id_counter`).
        assert!(Counter::Alloc as usize == 0);
        assert!(Counter::Reserve as usize == 1);
        assert!(Counter::Release as usize == 2);
        assert!(Counter::Skip as usize == 3);
        assert!(Counter::Overflow as usize == 4);
        assert!(Counter::Err as usize == 5);
    };
}
