// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
#include <jowokernel/process.h>
#include <jowokernel/pmm.h>
#include <jowokernel/thread.h>
#include <jowokernel/handle.h>
#include <jowokernel/kheap.h>
#include <jowokernel/string.h>

namespace arch { namespace x86 {
    extern void CopyPageTable(u64 dst_pml4_phys, u64 src_pml4_phys);
}}

extern "C" void serial_puts(const char* s);
extern "C" void serial_print_hex(u64 val);

namespace kernel {

void Process::AddThread(Thread* t) {
    if (t) {
        threads_.push_back(t);
        t->SetProcess(this);
    }
}

Process::Process(int id, const char* name) : id_(id), state_(ProcessState::INITIAL) {
    // Copy process name
    int i = 0;
    while (name[i] != '\0' && i < 31) {
        name_[i] = name[i];
        i++;
    }
    name_[i] = '\0';

    // Allocate a new root page table (PML4) for this process to ensure isolation.
    // Each process gets its own virtual address space mapping.
    page_table_physical_base_ = reinterpret_cast<u64>(vm::PhysicalMemoryManager::Get().AllocPage());
    
    // Nol-kan dulu seluruh PML4
    u64* pml4 = reinterpret_cast<u64*>(page_table_physical_base_);
    for(int i = 0; i < 512; i++) pml4[i] = 0;
    
    // Salin pemetaan kernel (Identity map 1GB pertama dan Kernel Space) dari CR3 yang sedang aktif
    u64 active_cr3;
    asm volatile("mov %%cr3, %0" : "=r"(active_cr3));
    u64* active_pml4 = reinterpret_cast<u64*>(active_cr3 & ~0xFFFULL);
    
    // Salin bagian atas (Kernel space, mulai indeks 256 ke atas)
    for (int i = 256; i < 512; i++) {
        pml4[i] = active_pml4[i];
    }
    
    // SANGAT PENTING: Kernel JowoKernel saat ini masih menggunakan Identity Mapping
    // di 512GB pertama (PML4 Index 0). Jadi kita juga harus membagikan PML4[0] ke setiap proses.
    // Jika tidak, saat proses berjalan, ia tidak bisa memanggil syscall atau mengakses Framebuffer/MMIO
    // dan CPU akan page fault saat berpindah kembali ke kernel mode!
    pml4[0] = active_pml4[0];

    // Register ke ProcessTable
    ProcessTable::Get().Register(this);
}

Process* Process::Fork() {
    // Buat proses anak dengan nama sama (atau ditambah suffix)
    Process* child = new Process(ProcessTable::Get().AllocatePid(), name_);
    if (!child) return nullptr;

    // Deep copy userspace memory (PML4 indices 1-255)
    arch::x86::CopyPageTable(child->GetPageTable(), this->GetPageTable());

    // Copy file handles (Handle duplication)
    for (u64 i = 0; i < handle_table_.size(); i++) {
        if (handle_table_[i]) {
            // Sederhanakan: buat handle baru yang menunjuk ke objek yang sama
            object::Handle* h = new object::Handle(-1, handle_table_[i]->GetObject(), handle_table_[i]->GetRights());
            child->AddHandle(h);
        }
    }

    // Add to hierarchy
    this->AddChild(child);

    return child;
}

Process::~Process() {
    state_ = ProcessState::DEAD;
    
    // Unregister dari ProcessTable
    ProcessTable::Get().Unregister(this);

    // Free the root page table
    if (page_table_physical_base_) {
        vm::PhysicalMemoryManager::Get().FreePage(reinterpret_cast<void*>(page_table_physical_base_));
    }
    
    // Clean up handles (simplified for now, ideally calls handle's destructors)
    for (u64 i = 0; i < handle_table_.size(); i++) {
        if (handle_table_[i]) {
            // handle_table_[i]->Close(); // To be implemented
        }
    }
}

void Process::Exit(int status) {
    state_ = ProcessState::ZOMBIE;
    exit_code_ = status;

    // Wake parent if waiting
    wait_queue_.WakeAll();
    
    // Close all file descriptors
    for (int i = 0; i < 256; i++) {
        if (fd_table_[i]) {
            FreeFileDescriptor(i);
        }
    }
    
    // Terminate all threads belonging to this process
    for (auto* t : threads_) {
        if (t) {
            t->SetState(ThreadState::TERMINATED); // Will be properly cleaned up by Idle thread or scheduler
        }
    }
}

int Process::AddHandle(object::Handle* h) {
    // Find an empty slot first
    for (u64 i = 0; i < handle_table_.size(); i++) {
        if (handle_table_[i] == nullptr) {
            handle_table_[i] = h;
            return static_cast<int>(i);
        }
    }
    // No empty slot found, push back a new one
    handle_table_.push_back(h);
    return static_cast<int>(handle_table_.size() - 1);
}

object::Handle* Process::GetHandle(int handle_id) const {
    if (handle_id < 0 || static_cast<u64>(handle_id) >= handle_table_.size()) {
        return nullptr;
    }
    return handle_table_[handle_id];
}

object::Handle* Process::RemoveHandle(int handle_id) {
    if (handle_id < 0 || static_cast<u64>(handle_id) >= handle_table_.size()) {
        return nullptr;
    }
    object::Handle* h = handle_table_[handle_id];
    handle_table_[handle_id] = nullptr;
    return h;
}

int Process::AllocateFileDescriptor(::fs::VNode* node) {
    if (!node) return -1;
    for (int i = 0; i < 256; i++) {
        if (fd_table_[i] == nullptr) {
            fd_table_[i] = node;
            return i;
        }
    }
    return -1; // ENFILE
}

::fs::VNode* Process::GetFileDescriptor(int fd) const {
    if (fd < 0 || fd >= 256) return nullptr;
    return fd_table_[fd];
}

void Process::FreeFileDescriptor(int fd) {
    if (fd >= 0 && fd < 256) {
        fd_table_[fd] = nullptr; // Note: In a real VFS, we should decrease refcount/close
    }
}

// ============================================================
// Tahap 2: Process Hierarchy
// ============================================================

void Process::AddChild(Process* child) {
    if (child) {
        children_.push_back(child);
        child->parent_ = this;
    }
}

void Process::RemoveChild(Process* child) {
    if (!child) return;
    for (u64 i = 0; i < children_.size(); i++) {
        if (children_[i] == child) {
            // Geser elemen terakhir ke posisi ini
            children_[i] = children_[children_.size() - 1];
            // Pop terakhir (jika vector mendukung)
            // Untuk sekarang, set nullptr
            children_[i] = nullptr;
            break;
        }
    }
}

// ============================================================
// Process Table — Global registry
// ============================================================

ProcessTable& ProcessTable::Get() {
    static ProcessTable g_process_table;
    return g_process_table;
}

int ProcessTable::AllocatePid() {
    AutoSpinlock guard(lock_);
    return next_pid_++;
}

void ProcessTable::Register(Process* proc) {
    AutoSpinlock guard(lock_);
    if (!proc) return;
    int pid = proc->GetId();
    if (pid >= 0 && pid < MAX_PROCESSES) {
        table_[pid] = proc;
    }
}

void ProcessTable::Unregister(Process* proc) {
    AutoSpinlock guard(lock_);
    if (!proc) return;
    int pid = proc->GetId();
    if (pid >= 0 && pid < MAX_PROCESSES) {
        table_[pid] = nullptr;
    }
}

Process* ProcessTable::FindByPid(int pid) {
    AutoSpinlock guard(lock_);
    if (pid < 0 || pid >= MAX_PROCESSES) return nullptr;
    return table_[pid];
}

} // namespace kernel
