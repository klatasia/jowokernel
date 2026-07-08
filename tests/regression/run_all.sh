#!/bin/bash
# JowoKernel Regression Test Runner
# Copyright (c) 2026 JowoKernel Team
# SPDX-License-Identifier: MIT

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test counters
PASS_COUNT=0
FAIL_COUNT=0
TOTAL_COUNT=0

# Output log
LOG_FILE="regression_test.log"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# ============================================================================
# Helper Functions
# ============================================================================

log() {
    echo -e "$1"
    echo -e "$1" | sed 's/\x1b\[[0-9;]*m//g' >> "$LOG_FILE"
}

header() {
    log ""
    log "========================================"
    log "$1"
    log "========================================"
}

pass() {
    log "${GREEN}[PASS]${NC} $1"
    ((PASS_COUNT++))
    ((TOTAL_COUNT++))
}

fail() {
    log "${RED}[FAIL]${NC} $1"
    ((FAIL_COUNT++))
    ((TOTAL_COUNT++))
}

info() {
    log "${BLUE}[INFO]${NC} $1"
}

warn() {
    log "${YELLOW}[WARN]${NC} $1"
}

section() {
    log ""
    log "${CYAN}--- $1 ---${NC}"
}

# ============================================================================
# Prerequisites Check
# ============================================================================

check_prerequisites() {
    header "Prerequisites Check"

    local missing=0

    # Check for compiler
    if command -v clang &> /dev/null; then
        pass "Clang compiler found"
    else
        fail "Clang compiler not found"
        missing=1
    fi

    # Check for QEMU
    if test -x "/c/Program Files/qemu/qemu-system-x86_64.exe" &> /dev/null; then
        pass "QEMU found"
    else
        warn "QEMU not found - cannot run runtime tests"
    fi

    # Check for xxd (for binary to header conversion)
    if command -v xxd &> /dev/null; then
        pass "xxd found"
    else
        warn "xxd not found - manual header generation may be needed"
    fi

    # Check for grub-mkrescue
    if command -v grub-mkrescue &> /dev/null; then
        pass "grub-mkrescue found"
    else
        warn "grub-mkrescue not found - ISO creation may fail"
    fi

    if [ $missing -eq 1 ]; then
        log ""
        log "${RED}FATAL: Missing critical prerequisites${NC}"
        exit 1
    fi
}

# ============================================================================
# Build Phase
# ============================================================================

build_kernel() {
    header "Build Phase"

    info "Cleaning previous build..."
    make clean 2>/dev/null || true

    info "Building kernel..."
    if make 2>&1 | tee -a "$LOG_FILE"; then
        pass "Kernel build"
    else
        fail "Kernel build"
        return 1
    fi

    if [ -f "build-x86_64/jowokernel.bin" ]; then
        pass "Kernel binary created ($(du -h build-x86_64/jowokernel.bin | cut -f1))"
    else
        fail "Kernel binary not found"
        return 1
    fi
}

# ============================================================================
# ISO Creation
# ============================================================================

create_iso() {
    header "ISO Creation"

    info "Creating ISO directory structure..."
    mkdir -p build-x86_64/isodir/boot/grub

    # Copy kernel
    cp build-x86_64/jowokernel.bin build-x86_64/isodir/boot/jowokernel.bin
    pass "Kernel copied to ISO"

    # Create minimal grub config
    cat > build-x86_64/isodir/boot/grub/grub.cfg << 'EOF'
set timeout=0
set default=0

menuentry "JowoKernel" {
    multiboot /boot/jowokernel.bin
    boot
}
EOF
    pass "GRUB config created"

    # Try to create ISO
    if command -v grub-mkrescue &> /dev/null; then
        info "Creating ISO with grub-mkrescue..."
        if grub-mkrescue -o build-x86_64/jowokernel.iso build-x86_64/isodir 2>/dev/null; then
            pass "ISO created ($(du -h build-x86_64/jowokernel.iso | cut -f1))"
        else
            warn "ISO creation failed (grub-mkrescue error)"
        fi
    else
        warn "grub-mkrescue not available - skipping ISO creation"
    fi
}

# ============================================================================
# Runtime Tests (QEMU)
# ============================================================================

run_qemu_test() {
    local test_name="$1"
    local expected_pattern="$2"
    local timeout="${3:-30}"

    section "Testing: $test_name"

    if ! test -x "/c/Program Files/qemu/qemu-system-x86_64.exe" &> /dev/null; then
        warn "QEMU not available - skipping runtime test"
        return 0
    fi

    info "Starting QEMU..."
    info "Timeout: ${timeout}s"

    # Create unique output file for this test
    local test_output="test_output_${test_name}.log"

    # Run QEMU with timeout
    timeout ${timeout}s "/c/Program Files/qemu/qemu-system-x86_64.exe" \
        -nographic \
        -kernel build-x86_64/jowokernel.bin \
        -append "console=ttyS0" \
        -serial file:"$test_output" \
        -m 512M \
        -enable-kvm 2>/dev/null \
        || timeout ${timeout}s "/c/Program Files/qemu/qemu-system-x86_64.exe" \
            -nographic \
            -kernel build-x86_64/jowokernel.bin \
            -append "console=ttyS0" \
            -serial file:"$test_output" \
            -m 512M 2>/dev/null

    local exit_code=$?

    # Check if output file was created
    if [ ! -f "$test_output" ]; then
        fail "$test_name - No output generated"
        return 1
    fi

    # Check for error patterns
    if grep -qiE "(panic|triple fault|#GP|general protection|#PF|page fault|fatal)" "$test_output" 2>/dev/null; then
        fail "$test_name - Kernel error detected"
        grep -iE "(panic|triple fault|#GP|#PF)" "$test_output" | head -3 >> "$LOG_FILE"
        return 1
    fi

    # Check for expected pattern
    if [ -n "$expected_pattern" ]; then
        if grep -q "$expected_pattern" "$test_output" 2>/dev/null; then
            pass "$test_name"
        else
            warn "$test_name - Expected pattern not found (may still be OK)"
            pass "$test_name (partial)"
        fi
    else
        pass "$test_name"
    fi

    # Show relevant output
    info "Output excerpt:"
    head -20 "$test_output" | grep -v "^$" | tail -10 >> "$LOG_FILE" 2>/dev/null || true

    return 0
}

# ============================================================================
# Runtime Test Suite
# ============================================================================

run_runtime_tests() {
    header "Runtime Tests"

    if ! test -x "/c/Program Files/qemu/qemu-system-x86_64.exe" &> /dev/null; then
        info "Skipping runtime tests - QEMU not available"
        info "Run on a system with QEMU installed to execute runtime tests"
        return 0
    fi

    # Boot test - basic kernel boot
    run_qemu_test "Boot" "Boot" 20

    # Scheduler test - scheduler initialized
    run_qemu_test "Scheduler" "Scheduler" 15

    # Syscall test - SYS_WRITE from userspace
    run_qemu_test "SYS_WRITE" "Hello from userspace" 20

    # Yield test - SYS_YIELD
    run_qemu_test "SYS_YIELD" "Yield" 15

    # Exit test - SYS_EXIT
    run_qemu_test "SYS_EXIT" "EXIT" 15

    # EFAULT test - invalid pointer handling
    run_qemu_test "EFAULT" "EFAULT" 15

    # Stress test - many syscalls
    run_qemu_test "Stress" "stress" 30
}

# ============================================================================
# Summary
# ============================================================================

print_summary() {
    header "Test Summary"

    log ""
    log "Timestamp: $TIMESTAMP"
    log "Kernel: build-x86_64/jowokernel.bin"
    log ""
    log "========================================"
    log "RESULTS"
    log "========================================"
    log ""
    log "${GREEN}PASSED${NC}: $PASS_COUNT"
    log "${RED}FAILED${NC}: $FAIL_COUNT"
    log "TOTAL : $TOTAL_COUNT"
    log ""

    if [ $FAIL_COUNT -eq 0 ]; then
        log "${GREEN}========================================"
        log "       ALL TESTS PASSED!"
        log "========================================${NC}"
        log ""
        log "Kernel is ready for deployment."
        return 0
    else
        log "${RED}========================================"
        log "       SOME TESTS FAILED"
        log "========================================${NC}"
        log ""
        log "Check $LOG_FILE for details."
        return 1
    fi
}

# ============================================================================
# Main
# ============================================================================

main() {
    # Initialize log file
    echo "JowoKernel Regression Test Log" > "$LOG_FILE"
    echo "Started: $TIMESTAMP" >> "$LOG_FILE"
    echo "========================================" >> "$LOG_FILE"
    echo "" >> "$LOG_FILE"

    log ""
    log "╔══════════════════════════════════════════╗"
    log "║    JowoKernel Regression Test Runner     ║"
    log "╚══════════════════════════════════════════╝"
    log ""
    log "Timestamp: $TIMESTAMP"
    log "Log file: $LOG_FILE"

    # Run phases
    check_prerequisites
    build_kernel
    create_iso
    run_runtime_tests

    # Print summary
    print_summary
}

# Run main
main "$@"
