// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Virtual Memory Object (VMO) API

#pragma once

#include <jowokernel/types.h>

#ifdef __cplusplus
extern "C" {
#endif

// Represents a handle to a Virtual Memory Object
typedef int vmo_handle_t;

// VMO creation options
#define VMO_OPT_DEFAULT 0
#define VMO_OPT_RESIZABLE 1

#ifdef __cplusplus
}
#endif
