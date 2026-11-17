// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Fondasi penjadwal (P0.97.1 `scheduler`).


/// Hasil probe fondasi penjadwal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchedulerFoundation {
    /// Identitas task saat ini seperti dilaporkan bridge (`0` = belum ada).
    pub current_task: u32,
    /// Identitas thread saat ini seperti dilaporkan bridge (`0` = belum ada).
    pub current_thread: u32,
    /// `true` bila model ini mem-preempt. Selalu `false` pada gate ini.
    pub preemption: bool,
    /// Panjang antrian runnable yang dimiliki model ini. Selalu `0`.
    pub runnable_queue: u32,
    /// `true` bila kendali dikembalikan ke penjadwal C++.
    pub handoff_to_cpp: bool,
}

impl SchedulerFoundation {
    /// Lulus bila model ini menyerahkan kendali dan tidak mem-preempt.
    pub const fn ok(&self) -> bool {
        self.handoff_to_cpp && !self.preemption && self.runnable_queue == 0
    }

    /// Apakah bridge melaporkan adanya task yang sedang berjalan.
    pub const fn task_present(&self) -> bool {
        self.current_task != 0 || self.current_thread != 0
    }
}

/// Membentuk model fondasi penjadwal dari identitas yang dilaporkan bridge.
pub fn probe(current_task: u32, current_thread: u32) -> SchedulerFoundation {
    SchedulerFoundation {
        current_task,
        current_thread,
        preemption: false,
        runnable_queue: 0,
        handoff_to_cpp: true,
    }
}

// ===========================================================================
// P0.98.4 — ANTARMUKA PENJADWAL
// ===========================================================================
//
// Lima operasi yang diminta: `spawn`, `block`, `wake`, `yield`, `exit`.
//
// # Ini BUKAN penulisan ulang penjadwal
//
// Tidak ada antrian runnable milik Rust, tidak ada pemilihan berikutnya, tidak
// ada peralihan konteks, tidak ada preemption. Yang ada hanyalah **transisi
// keadaan task** pada model [`TaskTable`] — persis "Rust model + runtime proof
// + C++ bridge boundary" yang diminta P0.98. Penjadwal yang benar-benar
// memilih task berikutnya tetap `KlatKernel/Efek/Eksekusi/Scheduler` milik C++.
//
// # Tidak ada dua penjadwal aktif
//
// `preemption` tetap `false` dan `handoff_to_cpp` tetap `true` di
// [`SchedulerFoundation`]; modul ini tidak pernah mengambil alih pemilihan.
//
// # Nama
//
// `yield` adalah kata kunci yang dipesan di Rust, jadi operasinya bernama
// [`yield_task`]. `spawn`, `block`, `wake`, dan `exit` memakai nama persis
// yang diminta.

use crate::abi2::object_kind;
use crate::dispatch;
use crate::ksm::Izin;
use crate::task::{KlatTask, TaskId, TaskKind, TaskState, TaskTable, TASK_NONE};

/// Galat antarmuka penjadwal. Semua negatif gagal-tertutup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    /// Token izin tidak ada atau bukan untuk operasi ini.
    IzinTidakSah,
    /// `id` tidak menunjuk task yang ada.
    TaskTidakDikenal,
    /// Transisi tidak sah untuk keadaan task sekarang.
    TransisiTidakSah,
    /// Tabel task penuh.
    Penuh,
    /// `id` tidak sah (mis. [`TASK_NONE`]).
    IdTidakSah,
}

/// Memeriksa token: wajib ada dan wajib untuk `(TASK, opcode)`.
fn izin_ok(izin: Option<Izin>, opcode: u8) -> Result<Izin, SchedulerError> {
    match izin {
        Some(izin) if izin.untuk(object_kind::TASK, opcode) => Ok(izin),
        _ => Err(SchedulerError::IzinTidakSah),
    }
}

/// `spawn` — membuat task baru dalam keadaan [`TaskState::Created`].
///
/// Memerlukan token `(TASK, op_task::SETPGID)` — satu-satunya operasi TASK yang
/// mengubah keadaan di tabel dispatch, jadi ia yang mewakili "boleh mengubah
/// task".
pub fn spawn(
    table: &mut TaskTable,
    id: TaskId,
    kind: TaskKind,
    parent: TaskId,
    izin: Option<Izin>,
) -> Result<(), SchedulerError> {
    izin_ok(izin, dispatch::op_task::SETPGID)?;
    if id == TASK_NONE {
        return Err(SchedulerError::IdTidakSah);
    }
    match table.sisipkan(id, kind, parent, parent) {
        Some(_) => Ok(()),
        None => {
            // Bedakan "sudah ada" dari "penuh": keduanya `None`, tetapi
            // pemanggil berhak tahu yang mana.
            if table.get(id).is_some() {
                Err(SchedulerError::TransisiTidakSah)
            } else {
                Err(SchedulerError::Penuh)
            }
        }
    }
}

/// `run` — `Created`/`Runnable` → `Running`.
pub fn run(table: &mut TaskTable, id: TaskId, izin: Option<Izin>) -> Result<(), SchedulerError> {
    izin_ok(izin, dispatch::op_task::SETPGID)?;
    let task = table.get(id).ok_or(SchedulerError::TaskTidakDikenal)?;
    match task.state() {
        TaskState::Created | TaskState::Runnable => {
            if table.set_state(id, TaskState::Running) {
                Ok(())
            } else {
                Err(SchedulerError::TaskTidakDikenal)
            }
        }
        _ => Err(SchedulerError::TransisiTidakSah),
    }
}

/// `block` — `Running` → `Blocked`.
pub fn block(table: &mut TaskTable, id: TaskId, izin: Option<Izin>) -> Result<(), SchedulerError> {
    izin_ok(izin, dispatch::op_task::SETPGID)?;
    let task = table.get(id).ok_or(SchedulerError::TaskTidakDikenal)?;
    match task.state() {
        TaskState::Running => {
            if table.set_state(id, TaskState::Blocked) {
                Ok(())
            } else {
                Err(SchedulerError::TaskTidakDikenal)
            }
        }
        _ => Err(SchedulerError::TransisiTidakSah),
    }
}

/// `wake` — `Blocked` → `Runnable`.
pub fn wake(table: &mut TaskTable, id: TaskId, izin: Option<Izin>) -> Result<(), SchedulerError> {
    izin_ok(izin, dispatch::op_task::SETPGID)?;
    let task = table.get(id).ok_or(SchedulerError::TaskTidakDikenal)?;
    match task.state() {
        TaskState::Blocked => {
            if table.set_state(id, TaskState::Runnable) {
                Ok(())
            } else {
                Err(SchedulerError::TaskTidakDikenal)
            }
        }
        _ => Err(SchedulerError::TransisiTidakSah),
    }
}

/// `yield` — `Running` → `Runnable`. Dinamai `yield_task` karena `yield`
/// dipesan di Rust.
pub fn yield_task(table: &mut TaskTable, id: TaskId, izin: Option<Izin>) -> Result<(), SchedulerError> {
    izin_ok(izin, dispatch::op_task::SETPGID)?;
    let task = table.get(id).ok_or(SchedulerError::TaskTidakDikenal)?;
    match task.state() {
        TaskState::Running => {
            if table.set_state(id, TaskState::Runnable) {
                Ok(())
            } else {
                Err(SchedulerError::TaskTidakDikenal)
            }
        }
        _ => Err(SchedulerError::TransisiTidakSah),
    }
}

/// `exit` — keadaan apa pun → `Exited`, lalu slot dilepas.
pub fn exit(table: &mut TaskTable, id: TaskId, izin: Option<Izin>) -> Result<(), SchedulerError> {
    izin_ok(izin, dispatch::op_task::SETPGID)?;
    let task = table.get(id).ok_or(SchedulerError::TaskTidakDikenal)?;
    if matches!(task.state(), TaskState::Exited) {
        return Err(SchedulerError::TransisiTidakSah);
    }
    if table.set_state(id, TaskState::Exited) && table.lepas(id) {
        Ok(())
    } else {
        Err(SchedulerError::TaskTidakDikenal)
    }
}

/// Membaca keadaan task. Memerlukan token `(TASK, op_task::INFO)`.
pub fn state_of(table: &TaskTable, id: TaskId, izin: Option<Izin>) -> Result<TaskState, SchedulerError> {
    izin_ok(izin, dispatch::op_task::INFO)?;
    table.get(id).map(KlatTask::state).ok_or(SchedulerError::TaskTidakDikenal)
}
