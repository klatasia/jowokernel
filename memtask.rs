// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P0.98.6 — MATRIKS RUNTIME MEMORI + TASK.


use crate::abi2::{object_kind, right};
use crate::dispatch;
use crate::ksm::{self, Ditolak, Keputusan};
use crate::memory::{self, AddressSpace, Frame, MemoryError, Page, VirtualAddress, MAX_MAPPINGS};
use crate::scheduler::{self, SchedulerError};
use crate::task::{self, TaskId, TaskKind, TaskState, TaskTable, TASK_NONE};

/// Jumlah pemeriksaan maksimum yang dapat dilaporkan.
pub const MAX_CASES: usize = 40;

/// Ringkasan matriks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemTaskReport {
    /// Penanda mekanisme (hanya dapat berasal dari Rust).
    pub tag: u64,
    /// Total pemeriksaan.
    pub checks: u32,
    /// Pemeriksaan yang lulus.
    pub passed: u32,
    /// Subtotal: model memori.
    pub mem_checks: u32,
    pub mem_passed: u32,
    /// Subtotal: model task.
    pub task_checks: u32,
    pub task_passed: u32,
    /// Subtotal: pintu izin KSM.
    pub auth_checks: u32,
    pub auth_passed: u32,
    /// Subtotal: jalur negatif (wajib gagal-tertutup).
    pub neg_checks: u32,
    pub neg_passed: u32,
    /// `0` = lulus; selain itu jumlah kegagalan.
    pub status: i32,
}

impl MemTaskReport {
    pub const fn ok(&self) -> bool {
        self.status == 0 && self.checks == self.passed && self.checks > 0
    }

    pub const fn failed(&self) -> u32 {
        self.checks - self.passed
    }
}

/// Kelompok pemeriksaan, supaya subtotalnya dapat dilaporkan terpisah.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kelompok {
    Memori,
    Task,
    Izin,
    Negatif,
}

/// Penghitung tetap, tanpa alokasi.
struct Hitung {
    checks: u32,
    passed: u32,
    mem_c: u32,
    mem_p: u32,
    task_c: u32,
    task_p: u32,
    auth_c: u32,
    auth_p: u32,
    neg_c: u32,
    neg_p: u32,
    dicetak: u32,
}

impl Hitung {
    const fn baru() -> Self {
        Hitung {
            checks: 0,
            passed: 0,
            mem_c: 0,
            mem_p: 0,
            task_c: 0,
            task_p: 0,
            auth_c: 0,
            auth_p: 0,
            neg_c: 0,
            neg_p: 0,
            dicetak: 0,
        }
    }

    fn catat(&mut self, kelompok: Kelompok, nama: &str, lulus: bool) {
        self.checks += 1;
        if lulus {
            self.passed += 1;
        }
        match kelompok {
            Kelompok::Memori => {
                self.mem_c += 1;
                if lulus {
                    self.mem_p += 1;
                }
            }
            Kelompok::Task => {
                self.task_c += 1;
                if lulus {
                    self.task_p += 1;
                }
            }
            Kelompok::Izin => {
                self.auth_c += 1;
                if lulus {
                    self.auth_p += 1;
                }
            }
            Kelompok::Negatif => {
                self.neg_c += 1;
                if lulus {
                    self.neg_p += 1;
                }
            }
        }
        if self.dicetak < MAX_CASES as u32 {
            self.dicetak += 1;
            cetak(core::format_args!(
                "P098-KASUS {} {} {}",
                if lulus { "LULUS" } else { "GAGAL" },
                self.checks,
                nama
            ));
        }
    }
}

/// Menulis satu baris ke serial.
///
/// Lewat [`crate::serial`] langsung (bukan makro `kprintln!`) supaya modul ini
/// dapat diuji di host dengan stub serial — modelnya tidak bergantung pada
/// port I/O, jadi menguji modelnya tidak boleh memerlukan perangkat keras.
fn cetak(args: core::fmt::Arguments) {
    crate::serial::write_fmt(args);
    crate::serial::write_byte(b'\n');
}

/// Alamat virtual halaman `n` — dibangun lewat konstruktor yang memeriksa.
fn va(page_number: u64) -> VirtualAddress {
    match Page::new(page_number).and_then(Page::start) {
        Some(address) => address,
        None => VirtualAddress::new(0).expect("alamat 0 selalu sah"),
    }
}

/// Menjalankan matriks P0.98.6.
///
/// `hak_memori` dan `hak_task` adalah hak yang **dimiliki pemanggil menurut KSM
/// C++ yang nyata** (`KlatIpc/Kanal/OtoritasEndpoint.h`). Rust tidak menambah,
/// mengurangi, atau menyimpan nilai itu.
pub fn jalankan(hak_memori: u32, hak_task: u32) -> MemTaskReport {
    let mut h = Hitung::baru();

    // Token dari KSM. `None` bila KSM akan menolak — dan itulah yang dipakai
    // jalur negatif di bawah.
    let izin_mem = memory::izin_memori(hak_memori);
    let izin_task_ubah = task::izin_task_ubah(hak_task);
    let izin_task_baca = task::izin_task_baca(hak_task);

    // =====================================================================
    // A. PINTU IZIN KSM (P0.98.2)
    // =====================================================================
    cetak(core::format_args!(
        "P098-IZIN hak_memori=0x{:x} hak_task=0x{:x} mem={} task_ubah={} task_baca={}",
        hak_memori,
        hak_task,
        izin_mem.is_some(),
        izin_task_ubah.is_some(),
        izin_task_baca.is_some()
    ));

    h.catat(Kelompok::Izin, "izin_memori_dari_ksm", izin_mem.is_some());
    h.catat(Kelompok::Izin, "izin_task_ubah_dari_ksm", izin_task_ubah.is_some());
    h.catat(Kelompok::Izin, "izin_task_baca_dari_ksm", izin_task_baca.is_some());

    // Keputusan mentah dari tabel yang sama, dibaca langsung.
    h.catat(
        Kelompok::Izin,
        "tabel_memori_modify_kendali",
        ksm::operation_rights(object_kind::MEMORY, dispatch::op_mem::MODIFY)
            == ksm::FOUND | right::CONTROL as u64,
    );
    h.catat(
        Kelompok::Izin,
        "tabel_task_setpgid_kendali",
        ksm::operation_rights(object_kind::TASK, dispatch::op_task::SETPGID)
            == ksm::FOUND | right::CONTROL as u64,
    );

    // =====================================================================
    // B. MODEL MEMORI (P0.98.1)
    // =====================================================================
    let mut ruang = AddressSpace::new();
    let halaman = Page::new(1).expect("halaman 1 sah");
    let bingkai = Frame::new(2).expect("frame 2 sah");

    h.catat(Kelompok::Memori, "mem_ruang_awal_kosong", ruang.is_empty());

    // Alamat: konstruktor menolak bentuk yang tidak sah.
    h.catat(
        Kelompok::Memori,
        "alamat_fisik_selaras",
        memory::PhysicalAddress::new(4096).is_some(),
    );
    h.catat(
        Kelompok::Negatif,
        "alamat_fisik_tak_selaras_ditolak",
        memory::PhysicalAddress::new(4097).is_none(),
    );
    h.catat(
        Kelompok::Negatif,
        "alamat_virtual_di_luar_batas_ditolak",
        VirtualAddress::new(1u64 << 49).is_none(),
    );
    h.catat(
        Kelompok::Memori,
        "alamat_virtual_offset",
        va(1).offset() == 0 && va(1).page() == halaman,
    );

    // map / translate / fault
    h.catat(
        Kelompok::Memori,
        "mem_peta",
        ruang.map(halaman, bingkai, right::READ, izin_mem).is_ok(),
    );
    h.catat(Kelompok::Memori, "mem_panjang_satu", ruang.len() == 1);
    h.catat(
        Kelompok::Memori,
        "mem_terjemah",
        ruang.translate(halaman) == Some(bingkai),
    );
    h.catat(
        Kelompok::Memori,
        "mem_fault_hak_cukup",
        ruang.fault(va(1), right::READ) == memory::FaultKind::TidakBermasalah,
    );
    h.catat(
        Kelompok::Negatif,
        "mem_fault_hak_kurang",
        ruang.fault(va(1), right::WRITE) == memory::FaultKind::HakKurang,
    );

    // unmap
    h.catat(
        Kelompok::Memori,
        "mem_unmap",
        ruang.unmap(halaman, izin_mem) == Ok(bingkai),
    );
    h.catat(Kelompok::Memori, "mem_kosong_lagi", ruang.is_empty());
    h.catat(
        Kelompok::Negatif,
        "mem_fault_setelah_unmap",
        ruang.fault(va(1), right::READ) == memory::FaultKind::TidakDipetakan,
    );

    // =====================================================================
    // C. JALUR NEGATIF MEMORI — semua harus gagal-tertutup
    // =====================================================================
    h.catat(
        Kelompok::Negatif,
        "mem_peta_tanpa_izin_ditolak",
        ruang.map(halaman, bingkai, right::READ, None) == Err(MemoryError::IzinTidakSah),
    );
    // Token TASK dipakai untuk operasi MEMORY — lintas otoritas.
    // Kasus ini WAJIB memastikan token-nya benar-benar ada: kalau token-nya
    // `None`, penolakannya berasal dari "tidak ada izin", bukan dari "izin
    // untuk otoritas lain" — dan itu akan lulus karena alasan yang salah.
    h.catat(
        Kelompok::Negatif,
        "mem_peta_token_lintas_otoritas_ditolak",
        izin_task_ubah.is_some()
            && ruang.map(halaman, bingkai, right::READ, izin_task_ubah)
                == Err(MemoryError::IzinTidakSah),
    );
    // Hak yang tidak sah untuk kind MEMORY (SEND milik endpoint) ditolak.
    h.catat(
        Kelompok::Negatif,
        "mem_peta_hak_bukan_memori_ditolak",
        ruang.map(halaman, bingkai, right::SEND, izin_mem) == Err(MemoryError::IzinTidakSah),
    );
    h.catat(
        Kelompok::Negatif,
        "mem_unmap_tanpa_izin_ditolak",
        ruang.unmap(halaman, None) == Err(MemoryError::IzinTidakSah),
    );
    h.catat(
        Kelompok::Negatif,
        "mem_unmap_belum_dipetakan_ditolak",
        ruang.unmap(halaman, izin_mem) == Err(MemoryError::TidakDipetakan),
    );

    // Peta ganda + ruang penuh.
    let mut ruang2 = AddressSpace::new();
    let _ = ruang2.map(halaman, bingkai, right::READ, izin_mem);
    h.catat(
        Kelompok::Negatif,
        "mem_peta_ganda_ditolak",
        ruang2.map(halaman, bingkai, right::READ, izin_mem) == Err(MemoryError::SudahDipetakan),
    );
    let mut penuh = AddressSpace::new();
    let mut terisi = 0usize;
    for nomor in 0..MAX_MAPPINGS as u64 {
        let p = Page::new(nomor).expect("halaman dalam batas");
        let f = Frame::new(nomor).expect("frame dalam batas");
        if penuh.map(p, f, right::READ, izin_mem).is_ok() {
            terisi += 1;
        }
    }
    let p_lebih = Page::new(MAX_MAPPINGS as u64).expect("halaman dalam batas");
    let f_lebih = Frame::new(MAX_MAPPINGS as u64).expect("frame dalam batas");
    h.catat(
        Kelompok::Memori,
        "mem_ruang_terisi_penuh",
        terisi == MAX_MAPPINGS,
    );
    h.catat(
        Kelompok::Negatif,
        "mem_ruang_penuh_ditolak",
        penuh.map(p_lebih, f_lebih, right::READ, izin_mem) == Err(MemoryError::Penuh),
    );

    // =====================================================================
    // D. MODEL TASK (P0.98.3) + ANTARMUKA PENJADWAL (P0.98.4)
    // =====================================================================
    let mut tabel = TaskTable::new();
    let proses: TaskId = 1;
    let thread: TaskId = 2;

    h.catat(Kelompok::Task, "task_tabel_awal_kosong", tabel.is_empty());
    h.catat(
        Kelompok::Task,
        "task_spawn_proses",
        scheduler::spawn(&mut tabel, proses, TaskKind::Process, TASK_NONE, izin_task_ubah).is_ok(),
    );
    h.catat(
        Kelompok::Task,
        "task_keadaan_created",
        scheduler::state_of(&tabel, proses, izin_task_baca) == Ok(TaskState::Created),
    );
    h.catat(
        Kelompok::Task,
        "task_run",
        scheduler::run(&mut tabel, proses, izin_task_ubah).is_ok()
            && scheduler::state_of(&tabel, proses, izin_task_baca) == Ok(TaskState::Running),
    );
    h.catat(
        Kelompok::Task,
        "task_block",
        scheduler::block(&mut tabel, proses, izin_task_ubah).is_ok()
            && scheduler::state_of(&tabel, proses, izin_task_baca) == Ok(TaskState::Blocked),
    );
    h.catat(
        Kelompok::Task,
        "task_wake",
        scheduler::wake(&mut tabel, proses, izin_task_ubah).is_ok()
            && scheduler::state_of(&tabel, proses, izin_task_baca) == Ok(TaskState::Runnable),
    );
    h.catat(
        Kelompok::Task,
        "task_run_dan_yield",
        scheduler::run(&mut tabel, proses, izin_task_ubah).is_ok()
            && scheduler::yield_task(&mut tabel, proses, izin_task_ubah).is_ok()
            && scheduler::state_of(&tabel, proses, izin_task_baca) == Ok(TaskState::Runnable),
    );

    // Proses dan thread pada tabel yang sama.
    h.catat(
        Kelompok::Task,
        "task_spawn_thread",
        scheduler::spawn(&mut tabel, thread, TaskKind::Thread, proses, izin_task_ubah).is_ok(),
    );
    h.catat(Kelompok::Task, "task_tabel_dua", tabel.len() == 2);
    // Tanpa `expect`: selftest kernel TIDAK BOLEH panic — panic di sini akan
    // menghentikan kernel, bukan melaporkan kegagalan. Setiap pencarian tabel
    // karena itu mengembalikan `false` alih-alih panik.
    h.catat(
        Kelompok::Task,
        "task_proses_id",
        tabel
            .get(proses)
            .and_then(|task| task::KlatProcess::new(*task))
            .map(|p| p.pid())
            == Some(proses),
    );
    h.catat(
        Kelompok::Task,
        "task_thread_id",
        tabel
            .get(thread)
            .and_then(|task| task::KlatThread::new(*task))
            .map(|t| t.tid())
            == Some(thread),
    );
    h.catat(
        Kelompok::Negatif,
        "task_proses_menolak_thread",
        tabel
            .get(thread)
            .map(|task| task::KlatProcess::new(*task).is_none())
            .unwrap_or(false),
    );

    // exit
    h.catat(
        Kelompok::Task,
        "task_exit",
        scheduler::exit(&mut tabel, proses, izin_task_ubah).is_ok(),
    );
    h.catat(Kelompok::Task, "task_tabel_satu", tabel.len() == 1);
    h.catat(
        Kelompok::Negatif,
        "task_exit_lagi_ditolak",
        scheduler::exit(&mut tabel, proses, izin_task_ubah) == Err(SchedulerError::TaskTidakDikenal),
    );

    // =====================================================================
    // E. JALUR NEGATIF TASK — semua harus gagal-tertutup
    // =====================================================================
    h.catat(
        Kelompok::Negatif,
        "task_tidak_dikenal_ditolak",
        scheduler::run(&mut tabel, 99, izin_task_ubah) == Err(SchedulerError::TaskTidakDikenal),
    );
    h.catat(
        Kelompok::Negatif,
        "task_id_tidak_sah_ditolak",
        scheduler::spawn(&mut tabel, TASK_NONE, TaskKind::Process, TASK_NONE, izin_task_ubah)
            == Err(SchedulerError::IdTidakSah),
    );
    h.catat(
        Kelompok::Negatif,
        "task_spawn_tanpa_izin_ditolak",
        scheduler::spawn(&mut tabel, 3, TaskKind::Process, TASK_NONE, None)
            == Err(SchedulerError::IzinTidakSah),
    );
    // Token MEMORY dipakai untuk operasi TASK — lintas otoritas.
    // Sama seperti di atas: token-nya wajib benar-benar ada.
    h.catat(
        Kelompok::Negatif,
        "task_spawn_token_lintas_otoritas_ditolak",
        izin_mem.is_some()
            && scheduler::spawn(&mut tabel, 4, TaskKind::Process, TASK_NONE, izin_mem)
                == Err(SchedulerError::IzinTidakSah),
    );
    h.catat(
        Kelompok::Negatif,
        "task_transisi_tidak_sah_ditolak",
        scheduler::wake(&mut tabel, thread, izin_task_ubah) == Err(SchedulerError::TransisiTidakSah),
    );
    h.catat(
        Kelompok::Negatif,
        "task_baca_tanpa_izin_ditolak",
        scheduler::state_of(&tabel, thread, None) == Err(SchedulerError::IzinTidakSah),
    );
    h.catat(
        Kelompok::Negatif,
        "task_baca_token_ubah_ditolak",
        izin_task_ubah.is_some()
            && scheduler::state_of(&tabel, thread, izin_task_ubah)
                == Err(SchedulerError::IzinTidakSah),
    );

    // Tabel penuh.
    let mut kecil = TaskTable::new();
    let mut dibuat = 0u32;
    for id in 1..=(crate::task::MAX_TASKS as u16 + 1) {
        if scheduler::spawn(&mut kecil, id, TaskKind::Task, TASK_NONE, izin_task_ubah).is_ok() {
            dibuat += 1;
        }
    }
    h.catat(
        Kelompok::Task,
        "task_tabel_terisi_penuh",
        dibuat == crate::task::MAX_TASKS as u32,
    );
    h.catat(
        Kelompok::Negatif,
        "task_tabel_penuh_ditolak",
        scheduler::spawn(
            &mut kecil,
            crate::task::MAX_TASKS as u16 + 1,
            TaskKind::Task,
            TASK_NONE,
            izin_task_ubah,
        ) == Err(SchedulerError::Penuh),
    );

    // =====================================================================
    // F. KEPUTUSAN MENTAH — kontrol positif/negatif pintu izin
    // =====================================================================
    h.catat(
        Kelompok::Izin,
        "keputusan_izin_sah",
        ksm::periksa_operasi(object_kind::MEMORY, dispatch::op_mem::MODIFY, hak_memori).ok(),
    );
    h.catat(
        Kelompok::Negatif,
        "keputusan_hak_kurang",
        ksm::periksa_operasi(object_kind::MEMORY, dispatch::op_mem::MODIFY, right::READ)
            == Keputusan::Ditolak(Ditolak::HakKurang),
    );
    h.catat(
        Kelompok::Negatif,
        "keputusan_lintas_otoritas",
        ksm::periksa_operasi(object_kind::MEMORY, dispatch::op_mem::MODIFY, right::SEND)
            == Keputusan::Ditolak(Ditolak::HakTidakSahUntukKind),
    );
    h.catat(
        Kelompok::Negatif,
        "keputusan_kind_tidak_dikenal",
        ksm::periksa_operasi(999, 0, hak_memori) == Keputusan::Ditolak(Ditolak::KindTidakDikenal),
    );
    h.catat(
        Kelompok::Negatif,
        "keputusan_operasi_tidak_dikenal",
        ksm::periksa_operasi(object_kind::MEMORY, 200, hak_memori)
            == Keputusan::Ditolak(Ditolak::OperasiTidakDikenal),
    );

    // =====================================================================
    // G. INVARIAN AKHIR
    // =====================================================================
    // Model ini tidak boleh meninggalkan sisa: tabel task harus kembali kosong
    // setelah semua task di-exit, dan tidak ada hak yang tersimpan di mana pun.
    let mut bersih = TaskTable::new();
    let _ = scheduler::spawn(&mut bersih, 1, TaskKind::Task, TASK_NONE, izin_task_ubah);
    let _ = scheduler::exit(&mut bersih, 1, izin_task_ubah);
    h.catat(Kelompok::Task, "task_tabel_bersih", bersih.is_empty());
    h.catat(
        Kelompok::Memori,
        "mem_ruang_bersih",
        ruang.is_empty() && ruang2.len() == 1,
    );

    let status = if h.passed == h.checks {
        0
    } else {
        (h.checks - h.passed) as i32
    };

    MemTaskReport {
        tag: ksm::MECHANISM_TAG,
        checks: h.checks,
        passed: h.passed,
        mem_checks: h.mem_c,
        mem_passed: h.mem_p,
        task_checks: h.task_c,
        task_passed: h.task_p,
        auth_checks: h.auth_c,
        auth_passed: h.auth_p,
        neg_checks: h.neg_c,
        neg_passed: h.neg_p,
        status,
    }
}
