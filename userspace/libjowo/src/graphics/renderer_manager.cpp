// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT

#include <jowo/graphics/renderer_manager.h>
#include <stdlib.h>
#include <string.h>

namespace jowo {
namespace graphics {

RendererManager::RendererManager() : active_backend_(RendererBackend::Auto) {}
RendererManager::~RendererManager() {}

bool RendererManager::Initialize() {
    // Auto Fallback Logic
    // Native Vulkan -> Native OpenGL -> Zink -> llvmpipe
    if (ProbeVulkan()) {
        active_backend_ = RendererBackend::NativeVulkan;
    } else if (ProbeOpenGL()) {
        active_backend_ = RendererBackend::NativeOpenGL;
    } else {
        // Fallback to llvmpipe if no hardware support
        active_backend_ = RendererBackend::Llvmpipe;
    }
    
    return true;
}

const char* RendererManager::GetBackendName() const {
    switch (active_backend_) {
        case RendererBackend::NativeVulkan: return "Vulkan (Hardware)";
        case RendererBackend::NativeOpenGL: return "OpenGL (Hardware)";
        case RendererBackend::Zink:         return "Zink (OpenGL on Vulkan)";
        case RendererBackend::Llvmpipe:     return "llvmpipe (Software)";
        default: return "Unknown";
    }
}

bool RendererManager::ProbeVulkan() {
    // Stub: In reality we would dynamically load libvulkan.so and call vkEnumeratePhysicalDevices
    return false; // Assuming no vulkan stub ready yet
}

bool RendererManager::ProbeOpenGL() {
    // Stub: In reality we would load libEGL.so / libGL.so
    return false; // Assuming no opengl stub ready yet
}

} // namespace graphics
} // namespace jowo
