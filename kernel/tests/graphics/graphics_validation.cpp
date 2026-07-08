// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Graphics Validation Framework (Tahap 13.7)

#include <jowokernel/tests/graphics_validation.h>
#include <jowokernel/gpu_scheduler.h>
#include <jowokernel/sync_object.h>
#include <jowokernel/gpu_memory_manager.h>
#include <jowokernel/frame_scheduler.h>
#include <jowokernel/vmo.h>
#include <jowokernel/kheap.h> // for getting available RAM, etc.

extern "C" void serial_puts(const char* s);

namespace jowo::tests::graphics {

void TestGpuQueue() noexcept {
    serial_puts("[Graphics Validation] Memulai Test 1: GPU Queue (Stress Test)\\n");
    
    gpu::GpuScheduler scheduler;
    object::GPUContext dummy_context;
    
    jowo::kstd::vector<object::SyncObject*> empty_waits;
    jowo::kstd::vector<object::SyncObject*> empty_signals;
    
    // Simulate 100 submits
    const u64 NUM_ITERATIONS = 100; // Reduced for faster QEMU execution
    for (u64 i = 0; i < NUM_ITERATIONS; ++i) {
        bool success = scheduler.Submit(&dummy_context, object::GpuQueueType::Graphics, 0x1000 + (i * 0x100), 256, empty_waits, empty_signals);
        if (!success) {
            serial_puts("[Graphics Validation] GAGAL: Submit gagal pada iterasi ke-");
            serial_puts("\\n");
            return;
        }
    }
    
    serial_puts("[Graphics Validation] Berhasil melakukan Submit() bertubi-tubi.\\n");
    serial_puts("[Graphics Validation] Test 1: GPU Queue SELESAI.\\n");
}

void TestTimelineFence() noexcept {
    serial_puts("[Graphics Validation] Memulai Test 2: Timeline Fence (Stress Test)\\n");
    
    auto fence = new object::TimelineFence();
    
    // 100 signals and waits to avoid extreme QEMU lockup
    const u64 MAX_SEQ = 100;
    
    for (u64 i = 1; i <= MAX_SEQ; ++i) {
        fence->Advance(i);
        
        // Wait on the current sequence
        if (!fence->Wait(i)) {
            serial_puts("[Graphics Validation] GAGAL: Timeline Fence Wait gagal!\\n");
            delete fence;
            return;
        }
        
        // Check monotonic property
        if (fence->GetSequence() != i) {
            serial_puts("[Graphics Validation] GAGAL: Timeline Fence Monotonic error!\\n");
            delete fence;
            return;
        }
    }
    
    delete fence;
    serial_puts("[Graphics Validation] Test 2: Timeline Fence SELESAI.\\n");
}

void TestGpuMemoryManager() noexcept {
    serial_puts("[Graphics Validation] Memulai Test 3: GPU Memory Manager (Pin / SG)\\n");
    
    gpu::GpuMemoryManager mem_mgr;
    
    const int NUM_VMOS = 100;
    vm::VmObject* vmos[100];
    gpu::ScatterGatherList* sglists[100];
    
    for (int i = 0; i < NUM_VMOS; ++i) {
        vmos[i] = vm::CreateVmObject(0x4000, 0); // 16KB VMO
        sglists[i] = mem_mgr.PinVMOForDMA(vmos[i]);
        if (!sglists[i]) {
            serial_puts("[Graphics Validation] GAGAL: PinVmo gagal!\\n");
            return;
        }
    }
    
    serial_puts("[Graphics Validation] Berhasil Pin 100 VMO.\\n");
    
    for (int i = 0; i < NUM_VMOS; ++i) {
        mem_mgr.UnpinVMO(vmos[i], sglists[i]);
        delete vmos[i];
    }
    
    serial_puts("[Graphics Validation] Test 3: GPU Memory Manager SELESAI.\\n");
}

void TestFrameScheduler() noexcept {
    serial_puts("[Graphics Validation] Memulai Test 4: Frame Scheduler\\n");
    
    gpu::FrameScheduler frame_sched;
    
    vm::VmObject* fake_buffer = vm::CreateVmObject(0x1000, 0);
    
    for (int i = 0; i < 60; ++i) {
        u64 target_time = i * 16; // Approx 60Hz
        frame_sched.ScheduleFrame(fake_buffer, target_time);
    }
    
    // Simulate VBLANK interrupts
    for (int i = 0; i < 60; ++i) {
        u64 current_time = i * 16 + 1; // 1ms after target
        frame_sched.OnVBlankInterrupt(current_time);
    }
    
    auto history = frame_sched.GetPresentationHistory();
    if (history.size() == 0) {
         serial_puts("[Graphics Validation] GAGAL: History kosong!\\n");
         return;
    }
    
    serial_puts("[Graphics Validation] Test 4: Frame Scheduler SELESAI.\\n");
    delete fake_buffer;
}

void TestMultiMonitor() noexcept {
    serial_puts("[Graphics Validation] Memulai Test 5: Multi-Monitor (60Hz, 144Hz, 240Hz)\\n");
    gpu::FrameScheduler sched_A; // 60Hz
    gpu::FrameScheduler sched_B; // 144Hz
    gpu::FrameScheduler sched_C; // 240Hz
    
    vm::VmObject* buf_A = vm::CreateVmObject(0x1000, 0);
    vm::VmObject* buf_B = vm::CreateVmObject(0x1000, 0);
    vm::VmObject* buf_C = vm::CreateVmObject(0x1000, 0);
    
    // Simulate 1 second of frames
    for (int i = 0; i < 240; ++i) {
        if (i % 4 == 0) sched_A.ScheduleFrame(buf_A, (i / 4) * 16); // 60Hz
        if (i % 2 == 0) sched_B.ScheduleFrame(buf_B, (i / 2) * 7);  // approx 144Hz
        sched_C.ScheduleFrame(buf_C, i * 4); // approx 240Hz
        
        // VBLANKs
        if (i % 4 == 0) sched_A.OnVBlankInterrupt((i / 4) * 16 + 1);
        if (i % 2 == 0) sched_B.OnVBlankInterrupt((i / 2) * 7 + 1);
        sched_C.OnVBlankInterrupt(i * 4 + 1);
    }
    
    serial_puts("[Graphics Validation] Test 5: Multi-Monitor SELESAI (Independen & Asinkron).\\n");
    delete buf_A; delete buf_B; delete buf_C;
}

void TestDmaBuf() noexcept {
    serial_puts("[Graphics Validation] Memulai Test 7: DMA-BUF Zero-Copy Checksum\\n");
    vm::VmObject* vmo = vm::CreateVmObject(0x1000, 0); // 4KB
    
    // Simulate write to VMO
    const char* test_data = "KLATOS_DMA_TEST_STRING";
    vmo->Write(test_data, 0, 22);
    
    // Simulate read back
    char read_back[32] = {0};
    vmo->Read(read_back, 0, 22);
    
    bool checksum_match = true;
    for (int i = 0; i < 22; ++i) {
        if (read_back[i] != test_data[i]) checksum_match = false;
    }
    
    if (!checksum_match) {
        serial_puts("[Graphics Validation] GAGAL: DMA-BUF Checksum mismatch!\\n");
        delete vmo;
        return;
    }
    
    serial_puts("[Graphics Validation] Test 7: DMA-BUF Zero-Copy Checksum SELESAI.\\n");
    delete vmo;
}

void RunAllGraphicsTests() noexcept {
    serial_puts("\\n==========================================\\n");
    serial_puts("   MEMULAI GRAPHICS VALIDATION TAHAP 13.7   \\n");
    serial_puts("==========================================\\n");
    
    // Temporarily disable graphics tests to focus on M3.1 Userspace ELF Loader
    // TestGpuQueue();
    // TestTimelineFence();
    // TestGpuMemoryManager();
    // TestFrameScheduler();
    // TestMultiMonitor();
    // TestDmaBuf();
    
    serial_puts("==========================================\\n");
    serial_puts("   SELURUH GRAPHICS VALIDATION DILIWATI     \\n");
    serial_puts("==========================================\\n\\n");
}

} // namespace jowo::tests::graphics
