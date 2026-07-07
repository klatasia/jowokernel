// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// libjowo - Renderer Manager
// Handles fallback prioritization for Vulkan, OpenGL, Zink, and llvmpipe

#pragma once

// No string include

namespace jowo {
namespace graphics {

enum class RendererBackend {
    Auto,
    NativeVulkan,
    NativeOpenGL,
    Zink,
    Llvmpipe
};

class RendererManager {
public:
    RendererManager();
    ~RendererManager();

    // Initializes the backend based on KLAT_RENDERER env or probing
    bool Initialize();

    RendererBackend GetActiveBackend() const { return active_backend_; }
    const char* GetBackendName() const;

private:
    RendererBackend active_backend_;

    bool ProbeVulkan();
    bool ProbeOpenGL();
};

} // namespace graphics
} // namespace jowo
