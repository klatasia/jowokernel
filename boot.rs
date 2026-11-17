// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Prototipe boot inti kernel Rust (P0.97.3).
//!

use crate::ffi::{KlatosKernelBootInfo, KlatosKernelBootReport};
use crate::{arch, interrupt, ipc, ksm, memory, scheduler, serial};
use crate::kprintln;

/// Jumlah pemeriksaan yang harus lulus sebelum laporan dinyatakan lulus.
pub const BOOT_STEPS: u32 = 6;

/// Titik masuk inti kernel Rust.
///
/// Menjalankan urutan inisialisasi, menulis jejak deterministik ke serial, dan
/// **mengembalikan** laporan ke pemanggil C++.
pub fn entry(info: KlatosKernelBootInfo) -> KlatosKernelBootReport {
    let mut report = KlatosKernelBootReport::empty();
    let mut completed = 0u32;
    let mut failure = 0i32;

    // --- titik masuk ---------------------------------------------------------
    serial::init();
    kprintln!(
        "P097-RUST-ENTRY tag=0x{:016x} arch={} ptr_bits={} steps={}",
        report.tag,
        arch::ARCH_NAME,
        arch::POINTER_BITS,
        BOOT_STEPS
    );
    kprintln!(
        "P097-RUST-BOOTINFO arch_id={} cpus={} boot_method={} cmdline_len={} page_size={} flags=0x{:x}",
        info.arch_id,
        info.cpu_count,
        info.boot_method,
        info.cmdline_length,
        info.page_size,
        info.flags
    );

    // --- 1. serial -----------------------------------------------------------
    let serial_ok = serial::is_initialized();
    report.serial_ok = serial_ok as u32;
    if serial_ok {
        completed += 1;
    } else {
        failure = 1;
    }
    #[cfg(target_arch = "x86_64")]
    kprintln!("P097-RUST-SERIAL com1=0x{:x} initialized={} ok={}", arch::COM1_PORT, serial_ok, serial_ok);
    #[cfg(target_arch = "aarch64")]
    {
        let pl011 = serial::PL011_BASE.load(core::sync::atomic::Ordering::SeqCst);
        kprintln!("P097-RUST-SERIAL pl011=0x{:x} initialized={} ok={}", pl011, serial_ok, serial_ok);
    }

    // --- 2. interrupt --------------------------------------------------------
    let irq = interrupt::probe();
    let interrupt_ok = irq.ok();
    report.interrupt_ok = interrupt_ok as u32;
    report.interrupts_enabled_before = irq.enabled_before as u32;
    if interrupt_ok {
        completed += 1;
    } else if failure == 0 {
        failure = 2;
    }
    kprintln!(
        "P097-RUST-INTERRUPT vectors={} irq_lines={} if_before={} disable_verified={} enable_attempted={} enable_verified={} ok={}",
        irq.vectors,
        irq.irq_lines,
        irq.enabled_before,
        irq.disable_verified,
        irq.enable_attempted,
        irq.enable_verified,
        interrupt_ok
    );

    // --- 3. memory -----------------------------------------------------------
    let mem = memory::probe();
    let memory_ok = mem.ok();
    report.memory_ok = memory_ok as u32;
    report.memory_checksum = mem.checksum;
    if memory_ok {
        completed += 1;
    } else if failure == 0 {
        failure = 3;
    }
    kprintln!(
        "P097-RUST-MEMORY page_size={} ptr_bits={} allocator={} words={} checksum=0x{:016x} round_trip={} ok={}",
        mem.page_size,
        mem.pointer_bits,
        mem.allocator,
        mem.stack_words,
        mem.checksum,
        mem.round_trip_ok,
        memory_ok
    );

    // --- 4. scheduler --------------------------------------------------------
    let sched = scheduler::probe(0, 0);
    let scheduler_ok = sched.ok();
    report.scheduler_ok = scheduler_ok as u32;
    if scheduler_ok {
        completed += 1;
    } else if failure == 0 {
        failure = 4;
    }
    kprintln!(
        "P097-RUST-SCHEDULER task={} thread={} task_present={} preemption={} runnable={} handoff={} ok={}",
        sched.current_task,
        sched.current_thread,
        sched.task_present(),
        sched.preemption,
        sched.runnable_queue,
        sched.handoff_to_cpp,
        scheduler_ok
    );

    // --- 5. ipc --------------------------------------------------------------
    let entries = ipc::table_entries();
    let ipc_ok = entries == ipc::DISPATCH_ENTRY_COUNT_EXPECTED;
    report.ipc_ok = ipc_ok as u32;
    report.dispatch_table_entries = entries;
    if ipc_ok {
        completed += 1;
    } else if failure == 0 {
        failure = 5;
    }
    kprintln!(
        "P097-RUST-IPC dispatch_entries={} expected={} ok={}",
        entries,
        ipc::DISPATCH_ENTRY_COUNT_EXPECTED,
        ipc_ok
    );

    // --- 6. ksm --------------------------------------------------------------
    let ksm_foundation = ksm::probe();
    let ksm_ok = ksm_foundation.ok();
    report.ksm_ok = ksm_ok as u32;
    if ksm_ok {
        completed += 1;
    } else if failure == 0 {
        failure = 6;
    }
    kprintln!(
        "P097-RUST-KSM tag=0x{:016x} entries={} checks={}/{} ok={}",
        ksm_foundation.mechanism_tag,
        ksm_foundation.table_entries,
        ksm_foundation.checks_passed,
        ksm_foundation.checks,
        ksm_ok
    );

    report.steps_completed = completed;
    report.status = failure;

    if report.ok() {
        kprintln!("P097-RUST-RESULT PASS {}/{} langkah", completed, BOOT_STEPS);
    } else {
        kprintln!(
            "P097-RUST-RESULT FAIL selesai={}/{} langkah_gagal_pertama={}",
            completed,
            BOOT_STEPS,
            failure
        );
    }
    kprintln!("P097-RUST-HANDOFF kendali kembali ke kernel C++");

    report
}
