// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Kernel Object Handle System

namespace object {

// A Handle is the user-space reference to a kernel object.
class Handle {
public:
    Handle(int id) : id_(id) {}
    ~Handle() {}

    int GetId() const { return id_; }

private:
    int id_;
};

class HandleTable {
public:
    HandleTable() {}

    Handle* CreateHandle() {
        // Create and register a new handle
        return new Handle(next_id_++);
    }

private:
    int next_id_ = 1;
};

} // namespace object
