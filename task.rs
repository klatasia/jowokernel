// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Model task Rust (P0.98.3) — `KlatTask`, `KlatProcess`, `KlatThread`.


use crate::abi2::object_kind;
use crate::dispatch;
use crate::ksm::{Izin, Keputusan};

/// Jumlah slot task maksimum. Tetap, tanpa alokasi.
pub const MAX_TASKS: usize = 8;

/// Identitas task. `0` adalah "tidak ada task" — sama seperti
/// `Abi2::ObjectId::NONE` untuk `index == 0`.
pub type TaskId = u16;

/// Task id yang berarti "tidak ada".
pub const TASK_NONE: TaskId = 0;

/// Keadaan task, persis lima yang diminta P0.98.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Created,
    Runnable,
    Running,
    Blocked,
    Exited,
}

impl TaskState {
    /// `true` bila task masih boleh menempati slot.
    pub const fn hidup(&self) -> bool {
        !matches!(self, TaskState::Exited)
    }
}

/// Kelas task. Ketiganya memakai [`KlatTask`] yang sama — satu model, bukan
/// tiga.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    /// Task kernel murni.
    Task,
    /// Task yang memimpin sebuah proses.
    Process,
    /// Task yang berbagi ruang alamat dengan prosesnya.
    Thread,
}

/// Satu task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatTask {
    id: TaskId,
    kind: TaskKind,
    state: TaskState,
    parent: TaskId,
    pgid: TaskId,
}

impl KlatTask {
    /// Slot kosong. `id == TASK_NONE` menandai slot yang tidak dipakai.
    pub const KOSONG: KlatTask = KlatTask {
        id: TASK_NONE,
        kind: TaskKind::Task,
        state: TaskState::Exited,
        parent: TASK_NONE,
        pgid: TASK_NONE,
    };

    pub const fn id(&self) -> TaskId {
        self.id
    }
    pub const fn kind(&self) -> TaskKind {
        self.kind
    }
    pub const fn state(&self) -> TaskState {
        self.state
    }
    pub const fn parent(&self) -> TaskId {
        self.parent
    }
    pub const fn pgid(&self) -> TaskId {
        self.pgid
    }
    /// `true` bila slot ini benar-benar dipakai.
    pub const fn terpakai(&self) -> bool {
        self.id != TASK_NONE
    }
}

/// Task yang memimpin proses. `pid` adalah `id` task-nya.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatProcess {
    task: KlatTask,
}

impl KlatProcess {
    pub const fn new(task: KlatTask) -> Option<Self> {
        match task.kind {
            TaskKind::Process => Some(KlatProcess { task }),
            _ => None,
        }
    }
    pub const fn pid(&self) -> TaskId {
        self.task.id
    }
    pub const fn state(&self) -> TaskState {
        self.task.state
    }
    pub const fn task(&self) -> &KlatTask {
        &self.task
    }
}

/// Task yang berbagi ruang alamat dengan prosesnya. `tid` adalah `id`-nya.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KlatThread {
    task: KlatTask,
}

impl KlatThread {
    pub const fn new(task: KlatTask) -> Option<Self> {
        match task.kind {
            TaskKind::Thread => Some(KlatThread { task }),
            _ => None,
        }
    }
    pub const fn tid(&self) -> TaskId {
        self.task.id
    }
    pub const fn state(&self) -> TaskState {
        self.task.state
    }
    pub const fn task(&self) -> &KlatTask {
        &self.task
    }
}

/// Tabel task: himpunan slot tetap, tanpa alokasi, tanpa hak.
#[derive(Clone, Copy, Debug)]
pub struct TaskTable {
    slots: [KlatTask; MAX_TASKS],
}

impl TaskTable {
    /// Tabel kosong.
    pub const fn new() -> Self {
        TaskTable { slots: [KlatTask::KOSONG; MAX_TASKS] }
    }

    /// Jumlah slot terpakai.
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|task| task.terpakai()).count()
    }

    /// `true` bila belum ada task.
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|task| !task.terpakai())
    }

    /// Mencari task berdasarkan id.
    pub fn get(&self, id: TaskId) -> Option<&KlatTask> {
        self.slots.iter().find(|task| task.terpakai() && task.id == id)
    }

    /// Mencari slot yang dapat diubah.
    fn slot_mut(&mut self, id: TaskId) -> Option<&mut KlatTask> {
        self.slots.iter_mut().find(|task| task.terpakai() && task.id == id)
    }

    /// Menempatkan task baru pada slot bebas. **Tidak** memeriksa izin —
    /// pemanggil di [`crate::scheduler`] yang melakukannya, dan itu memang
    /// satu-satunya pemanggil.
    pub(crate) fn sisipkan(
        &mut self,
        id: TaskId,
        kind: TaskKind,
        parent: TaskId,
        pgid: TaskId,
    ) -> Option<&KlatTask> {
        if id == TASK_NONE {
            return None;
        }
        if self.slots.iter().any(|task| task.terpakai() && task.id == id) {
            return None;
        }
        let slot = self.slots.iter_mut().find(|task| !task.terpakai())?;
        *slot = KlatTask { id, kind, state: TaskState::Created, parent, pgid };
        self.get(id)
    }

    /// Mengubah keadaan sebuah task.
    pub(crate) fn set_state(&mut self, id: TaskId, state: TaskState) -> bool {
        match self.slot_mut(id) {
            Some(task) => {
                task.state = state;
                true
            }
            None => false,
        }
    }

    /// Melepas slot task (sesudah `Exited`).
    pub(crate) fn lepas(&mut self, id: TaskId) -> bool {
        match self.slots.iter_mut().find(|task| task.terpakai() && task.id == id) {
            Some(task) => {
                *task = KlatTask::KOSONG;
                true
            }
            None => false,
        }
    }
}

impl Default for TaskTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Izin untuk **mengubah** task, atau `None` bila KSM menolak.
pub const fn izin_task_ubah(hak_dimiliki: u32) -> Option<Izin> {
    match crate::ksm::periksa_operasi(
        object_kind::TASK,
        dispatch::op_task::SETPGID,
        hak_dimiliki,
    ) {
        Keputusan::Diizinkan(izin) => Some(izin),
        Keputusan::Ditolak(_) => None,
    }
}

/// Izin untuk **membaca** metadata task, atau `None` bila KSM menolak.
pub const fn izin_task_baca(hak_dimiliki: u32) -> Option<Izin> {
    match crate::ksm::periksa_operasi(object_kind::TASK, dispatch::op_task::INFO, hak_dimiliki) {
        Keputusan::Diizinkan(izin) => Some(izin),
        Keputusan::Ditolak(_) => None,
    }
}

/// Konformansi waktu-kompilasi model task.
pub mod conformance {
    use super::*;
    use crate::abi2;

    const _: () = {
        // Lima keadaan, dan hanya `Exited` yang bukan "hidup".
        assert!(TaskState::Created.hidup());
        assert!(TaskState::Runnable.hidup());
        assert!(TaskState::Running.hidup());
        assert!(TaskState::Blocked.hidup());
        assert!(!TaskState::Exited.hidup());

        // Slot kosong tidak terpakai.
        assert!(!KlatTask::KOSONG.terpakai());
        assert!(KlatTask::KOSONG.id() == TASK_NONE);

        // `KlatProcess`/`KlatThread` menolak kind yang salah.
        assert!(KlatProcess::new(KlatTask::KOSONG).is_none());
        assert!(KlatThread::new(KlatTask::KOSONG).is_none());

        // Pintu izin task. Hak yang dipakai harus SAH untuk kind TASK
        // (`common | read | write`); `right::ALL` memuat bit yang tidak
        // bermakna untuk TASK dan karena itu ditolak lebih awal.
        assert!(izin_task_ubah(abi2::right::CONTROL | abi2::right::READ).is_some());
        assert!(izin_task_baca(abi2::right::READ).is_some());
        // Negatif: hak kosong tidak memuat CONTROL.
        assert!(izin_task_ubah(abi2::right::NONE).is_none());
        // Negatif lintas otoritas: hak endpoint (SEND) tidak sah untuk TASK.
        assert!(izin_task_ubah(abi2::right::SEND).is_none());
        // Negatif lintas otoritas: hak memori (MAP) tidak sah untuk TASK.
        assert!(izin_task_baca(abi2::right::MAP).is_none());
    };
}
