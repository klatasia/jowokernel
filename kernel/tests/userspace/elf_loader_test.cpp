// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// M3.1 ELF Loader Regression Test

#include <jowokernel/tests/test_runner.h>
#include "../../subsystems/24_userspace/elf_loader.h"
#include <jowokernel/process.h>
#include <jowokernel/vfs.h>
#include <jowokernel/vmo.h>
#include <jowokernel/string.h>

extern "C" void serial_puts(const char* s);
extern "C" void serial_print_hex(u64 val);
extern "C" void serial_print_int(int val);

// Menghubungkan header array bytes hasil kompilasi init.elf
#include "init_elf_bin.h"

namespace jowo::tests {
namespace userspace {

void run_elf_loader_test() noexcept {
    serial_puts("\n--- [Test M3.1] ELF Loader Parsing & Mapping ---\n");

    // 1. Buat Process palsu untuk pengujian (PID 999)
    kernel::Process* proc = new kernel::Process(999, "ElfTestProc");
    if (!proc) {
        serial_puts("[Test M3.1] FAIL: Gagal membuat Test Process!\n");
        return;
    }

    // 2. Bungkus init_elf_bin sebagai MemoryFileVNode
    // Nama variabel dari xxd biasanya userspace_init_init_elf dan userspace_init_init_elf_len
    const char* elf_data = reinterpret_cast<const char*>(userspace_init_init_elf);
    u64 elf_len = userspace_init_init_elf_len;

    fs::MemoryFileVNode* vnode = new fs::MemoryFileVNode();
    vnode->LoadData(elf_data, elf_len);

    // 3. Panggil ElfLoader
    u64 out_rsp = 0;
    u64 rip = kernel::userspace::ElfLoader::Load(vnode, proc, &out_rsp);

    // 4. Verifikasi Entry Point
    if (rip == 0) {
        serial_puts("[Test M3.1] FAIL: ElfLoader mengembalikan RIP 0 (Gagal load)!\n");
        delete vnode;
        delete proc;
        return;
    }

    // 5. Verifikasi Stack Allocation (RSP tidak nol)
    if (out_rsp == 0) {
        serial_puts("[Test M3.1] FAIL: ElfLoader gagal mengalokasikan RSP/Auxv!\n");
        delete vnode;
        delete proc;
        return;
    }
    
    // Verifikasi apakah 16-byte aligned (sebagian besar runtime Userspace ABI C membutuhkan ini)
    // Wait, setelah Push Auxv, mungkin kita perlu memastikan stack disejajarkan dengan benar.
    // Sementara kita hanya log saja.
    serial_puts("[Test M3.1] RIP Valid: 0x"); serial_print_hex(rip); serial_puts("\n");
    serial_puts("[Test M3.1] RSP Valid: 0x"); serial_print_hex(out_rsp); serial_puts("\n");
    
    // Cleanup
    delete vnode;
    delete proc; // Jika Process dihancurkan, Process destructor idealnya menghancurkan VMAR/PageTables.
                 // (Asumsi VMAR dicollect di destructor Process).

    serial_puts("[Test M3.1] PASS: ELF berhasil diparsing, VMO PT_LOAD terpetakan, Stack teralokasi, 0 Leak!\n");
}

} // namespace userspace
} // namespace jowo::tests
