// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// M3.2 Process Address Space & Ring 3 Transition Test

#include <jowokernel/tests/test_runner.h>
#include "../../../subsystems/24_userspace/elf_loader.h"
#include <jowokernel/process.h>
#include <jowokernel/vfs.h>
#include <jowokernel/vmo.h>
#include <jowokernel/string.h>
#include <jowokernel/thread.h>

extern "C" void serial_puts(const char* s);
extern "C" void serial_print_hex(u64 val);

#include "test_elf_bin.h"

namespace jowo::tests {
namespace userspace {

void run_m32_ring3_test() noexcept {
    serial_puts("\n--- [Test M3.2] Ring 3 Transition & Address Space ---\n");

    kernel::Process* proc = new kernel::Process(1001, "M32TestProc");
    if (!proc) {
        serial_puts("[Test M3.2] FAIL: Gagal membuat Test Process!\n");
        return;
    }

    const char* elf_data = reinterpret_cast<const char*>(kernel_tests_userspace_m32_test_test_elf);
    u64 elf_len = kernel_tests_userspace_m32_test_test_elf_len;

    fs::MemoryFileVNode* vnode = new fs::MemoryFileVNode();
    vnode->LoadData(elf_data, elf_len);

    u64 out_rsp = 0;
    u64 rip = kernel::userspace::ElfLoader::Load(vnode, proc, &out_rsp);

    if (rip == 0 || out_rsp == 0) {
        serial_puts("[Test M3.2] FAIL: ElfLoader gagal memuat m32_test!\n");
        delete vnode;
        delete proc;
        return;
    }

    serial_puts("[Test M3.2] ELF dimuat. Memulai User Thread...\n");
    
    kernel::Thread* thread = thread_create_user(proc, rip, out_rsp, 0);
    if (!thread) {
        serial_puts("[Test M3.2] FAIL: Gagal membuat User Thread!\n");
        delete vnode;
        delete proc;
        return;
    }
    
    serial_puts("[Test M3.2] Menambahkan User Thread ke Scheduler. (Expected: Page Fault by User Thread, dan kembali ke test)\n");
    sched_wake(thread);
    
    // We don't delete vnode and proc immediately because the thread needs it.
    // The scheduler will switch to it, cause a #PF, catch it, and mark it terminated.
}

}
}
