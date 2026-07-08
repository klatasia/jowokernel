// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// ELF 64-bit Loader Implementation

#include "elf_loader.h"
#include <jowokernel/elf.h>
#include <jowokernel/vmo.h>
#include <jowokernel/string.h>

extern "C" void serial_puts(const char* s);
extern "C" void serial_print_hex(u64 val);

namespace kernel {
namespace userspace {

u64 ElfLoader::Load(fs::VNode* file, kernel::Process* proc, u64* out_rsp) {
    if (!file || !proc || file->IsDirectory()) {
        serial_puts("[ElfLoader] GAGAL: File atau Proses tidak valid!\n");
        return 0;
    }

    // Untuk purwarupa VFS, kita asumsikan ini adalah MemoryFileVNode
    fs::MemoryFileVNode* mem_file = static_cast<fs::MemoryFileVNode*>(file);
    const char* file_data = mem_file->GetData();
    if (!file_data) {
        serial_puts("[ElfLoader] GAGAL: Tidak dapat membaca data VNode!\n");
        return 0;
    }

    jowo::elf::Elf64_Ehdr* ehdr = reinterpret_cast<jowo::elf::Elf64_Ehdr*>(const_cast<char*>(file_data));

    // Validasi Magic Number ELF
    if (ehdr->e_ident[0] != ELFMAG0 || ehdr->e_ident[1] != ELFMAG1 || 
        ehdr->e_ident[2] != ELFMAG2 || ehdr->e_ident[3] != ELFMAG3) {
        serial_puts("[ElfLoader] GAGAL: Berkas bukan format ELF!\n");
        return 0;
    }

    if (ehdr->e_machine != EM_X86_64) {
        serial_puts("[ElfLoader] GAGAL: ELF bukan untuk arsitektur x86_64!\n");
        return 0;
    }

    serial_puts("[ElfLoader] Memuat berkas ELF ke dalam Proses: ");
    serial_puts(proc->GetId() == 1 ? "InitUserProcess\n" : "Unknown\n");

    jowo::elf::Elf64_Phdr* phdr = reinterpret_cast<jowo::elf::Elf64_Phdr*>(const_cast<char*>(file_data + ehdr->e_phoff));
    
    // Langkah 1: Cari rentang Virtual Address seluruh Segmen PT_LOAD
    u64 min_vaddr = (u64)-1;
    u64 max_vaddr = 0;
    
    for (int i = 0; i < ehdr->e_phnum; i++) {
        if (phdr[i].p_type == PT_LOAD) {
            if (phdr[i].p_vaddr < min_vaddr) min_vaddr = phdr[i].p_vaddr;
            u64 end_addr = phdr[i].p_vaddr + phdr[i].p_memsz;
            if (end_addr > max_vaddr) max_vaddr = end_addr;
        }
    }
    
    if (min_vaddr == (u64)-1) {
        serial_puts("[ElfLoader] GAGAL: Tidak menemukan segmen PT_LOAD!\n");
        return 0;
    }
    
    // Page-align alamat dasar
    u64 aligned_vaddr = min_vaddr & ~4095ULL;
    
    // Langkah 2: Iterasi setiap segmen PT_LOAD dan buat VMO terpisah
    for (int i = 0; i < ehdr->e_phnum; i++) {
        if (phdr[i].p_type == PT_LOAD) {
            u64 vaddr = phdr[i].p_vaddr;
            u64 memsz = phdr[i].p_memsz;
            u64 filesz = phdr[i].p_filesz;
            u64 flags = phdr[i].p_flags;
            
            u64 seg_aligned_vaddr = vaddr & ~4095ULL;
            u64 offset_in_page = vaddr - seg_aligned_vaddr;
            u64 total_memsz = (memsz + offset_in_page + 4095) & ~4095ULL;
            
            // Konversi ELF flags (R=4, W=2, X=1) menjadi page protection kernel
            u32 map_flags = 0x05; // User, RO, Present (Read/Execute)
            if (flags & PF_W) { // Write flag (PF_W = 2)
                map_flags = 0x07; // User, RW, Present (Read/Write)
            }
            
            vm::VmObject* seg_vmo = vm::CreateVmObject(total_memsz, 0);
            if (!seg_vmo) {
                serial_puts("[ElfLoader] GAGAL: Tidak dapat membuat VMO untuk segmen PT_LOAD!\n");
                return 0;
            }
            
            if (!seg_vmo->MapToProcess(proc, seg_aligned_vaddr, total_memsz, map_flags)) {
                serial_puts("[ElfLoader] GAGAL: Tidak dapat memetakan VMO ke Proses!\n");
                return 0;
            }
            
            void* seg_base = seg_vmo->GetPhysicalBase();
            kl::memset(seg_base, 0, total_memsz);
            
            if (filesz > 0) {
                kl::memcpy(reinterpret_cast<char*>(seg_base) + offset_in_page, file_data + phdr[i].p_offset, filesz);
            }
        }
    }

    // Langkah 3.5: Periksa PT_INTERP untuk Dynamic Linker
    const char* interp_path = nullptr;
    for (int i = 0; i < ehdr->e_phnum; i++) {
        if (phdr[i].p_type == PT_INTERP) {
            interp_path = file_data + phdr[i].p_offset;
            break;
        }
    }

    u64 target_entry = ehdr->e_entry;
    u64 interp_base = 0;

    if (interp_path) {
        serial_puts("[ElfLoader] Mendeteksi Interpreter (Dynamic Linker): ");
        serial_puts(interp_path);
        serial_puts("\n");

        fs::VNode* interp_file = fs::VirtualFileSystem::Open(interp_path);
        if (interp_file) {
            fs::MemoryFileVNode* interp_mem = static_cast<fs::MemoryFileVNode*>(interp_file);
            const char* interp_data = interp_mem->GetData();
            jowo::elf::Elf64_Ehdr* interp_ehdr = reinterpret_cast<jowo::elf::Elf64_Ehdr*>(const_cast<char*>(interp_data));
            jowo::elf::Elf64_Phdr* interp_phdr = reinterpret_cast<jowo::elf::Elf64_Phdr*>(const_cast<char*>(interp_data + interp_ehdr->e_phoff));
            
            interp_base = 0x4000000000; // Load bias untuk interpreter

            u64 interp_min_vaddr = (u64)-1;
            u64 interp_max_vaddr = 0;
            for (int i = 0; i < interp_ehdr->e_phnum; i++) {
                if (interp_phdr[i].p_type == PT_LOAD) {
                    if (interp_phdr[i].p_vaddr < interp_min_vaddr) interp_min_vaddr = interp_phdr[i].p_vaddr;
                    u64 end_addr = interp_phdr[i].p_vaddr + interp_phdr[i].p_memsz;
                    if (end_addr > interp_max_vaddr) interp_max_vaddr = end_addr;
                }
            }
            
            u64 interp_aligned_vaddr = interp_min_vaddr & ~4095ULL;
            u64 interp_total_memsz = ((interp_max_vaddr - interp_aligned_vaddr) + 4095) & ~4095ULL;
            
            vm::VmObject* interp_vmo = vm::CreateVmObject(interp_total_memsz, 0);
            if (interp_vmo && interp_vmo->MapToProcess(proc, interp_base + interp_aligned_vaddr, interp_total_memsz, 0x07)) {
                void* interp_vmo_base = interp_vmo->GetPhysicalBase();
                kl::memset(interp_vmo_base, 0, interp_total_memsz);
                
                for (int i = 0; i < interp_ehdr->e_phnum; i++) {
                    if (interp_phdr[i].p_type == PT_LOAD) {
                        u64 offset_in_vmo = interp_phdr[i].p_vaddr - interp_aligned_vaddr;
                        char* dest = reinterpret_cast<char*>(interp_vmo_base) + offset_in_vmo;
                        if (interp_phdr[i].p_filesz > 0) {
                            kl::memcpy(dest, interp_data + interp_phdr[i].p_offset, interp_phdr[i].p_filesz);
                        }
                    }
                }
                
                target_entry = interp_base + interp_ehdr->e_entry;
            }
        }
    }

    // Langkah 4: Alokasikan Stack dan bangun Auxiliary Vector
    if (out_rsp) {
        u64 stack_size = 4096 * 4; // 16KB stack
        vm::VmObject* stack_vmo = vm::CreateVmObject(stack_size, 0);
        u64 stack_vaddr = 0x9000000000;
        
        if (stack_vmo && stack_vmo->MapToProcess(proc, stack_vaddr, stack_size, 0x07)) {
            u64* sp = reinterpret_cast<u64*>(reinterpret_cast<u8*>(stack_vmo->GetPhysicalBase()) + stack_size);
            
            auto push = [&](u64 val) {
                sp--;
                *sp = val;
            };
            
            // Auxv END
            push(0);
            push(AT_NULL);
            
            if (interp_base != 0) {
                push(interp_base);
                push(AT_BASE);
            }
            
            // PAGESZ
            push(4096);
            push(AT_PAGESZ);
            
            // PHNUM
            push(ehdr->e_phnum);
            push(AT_PHNUM);
            
            // PHENT
            push(ehdr->e_phentsize);
            push(AT_PHENT);
            
            // PHDR
            push(aligned_vaddr + ehdr->e_phoff);
            push(AT_PHDR);
            
            // ENTRY
            push(ehdr->e_entry);
            push(AT_ENTRY);
            
            // envp END
            push(0);
            
            // argv END
            push(0);
            
            // argc
            push(0);
            
            u64 offset_from_top = (reinterpret_cast<u8*>(stack_vmo->GetPhysicalBase()) + stack_size) - reinterpret_cast<u8*>(sp);
            *out_rsp = (stack_vaddr + stack_size) - offset_from_top;
        } else {
            *out_rsp = 0;
        }
    }

    serial_puts("[ElfLoader] Sukses memuat ELF! Target Entry Point: 0x");
    serial_print_hex(target_entry);
    serial_puts("\n");

    return target_entry;
}

} // namespace userspace
} // namespace kernel
