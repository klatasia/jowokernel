#!/bin/bash
# KLAT OS Boot Test Script
# Validates M1: QEMU Bring-up

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOG_FILE="$SCRIPT_DIR/boot_test.log"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

log() {
    echo "[$(date '+%H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

pass() {
    echo -e "${GREEN}✅ PASS${NC}"
    log "PASS: $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED}❌ FAIL${NC}"
    log "FAIL: $1"
    ((TESTS_FAILED++))
}

info() {
    echo -e "${YELLOW}ℹ INFO${NC}"
    log "INFO: $1"
}

echo "=========================================="
echo "   KLAT OS Boot Test (M1)"
echo "=========================================="
echo ""

# Clear log
> "$LOG_FILE"

# Check if kernel exists
if [ ! -f "$PROJECT_DIR/build-x86_64/kernel.bin" ]; then
    info "Kernel not found. Building..."
    cd "$PROJECT_DIR"
    make kernel
fi

# Create initramfs
if [ ! -f "$PROJECT_DIR/build-x86_64/init.elf" ]; then
    info "Creating initramfs..."
    cd "$PROJECT_DIR/userspace"
    make clean && make
    cd "$PROJECT_DIR"
fi

# Test cases
TESTS=(
    "test_kernel_banner:Kernel banner appears"
    "test_init_runs:Init process runs"
    "test_shell_prompt:Shell prompt appears"
    "test_syscall_read:Syscall read works"
    "test_syscall_write:Syscall write works"
    "test_syscall_open:Syscall open works"
)

info "Running QEMU boot test..."
info "Log file: $LOG_FILE"

# Run QEMU with timeout and capture output
timeout 60 qemu-system-x86_64 \
    -kernel "$PROJECT_DIR/build-x86_64/kernel.bin" \
    -initrd "$PROJECT_DIR/build-x86_64/initramfs.cpio.gz" \
    -m 512M \
    -nographic \
    -display none \
    -serial file:"$LOG_FILE" \
    -append "console=ttyS0" \
    2>&1 || true

# Wait for log to be written
sleep 2

echo ""
echo "=========================================="
echo "   Test Results"
echo "=========================================="

# Test 1: Kernel banner
if grep -q "JowoKernel" "$LOG_FILE"; then
    pass "Kernel banner appears"
else
    fail "Kernel banner NOT found"
fi

# Test 2: Init runs
if grep -q "Init" "$LOG_FILE"; then
    pass "Init process runs"
else
    fail "Init NOT found in log"
fi

# Test 3: Shell prompt
if grep -q "klatos" "$LOG_FILE"; then
    pass "Shell prompt appears"
else
    fail "Shell prompt NOT found"
fi

# Test 4: Syscall working
if grep -q "Syscall" "$LOG_FILE"; then
    pass "Syscall handler invoked"
else
    fail "Syscall NOT invoked"
fi

# Test 5: VFS working
if grep -q "VFS" "$LOG_FILE" || grep -q "Filesystem" "$LOG_FILE"; then
    pass "VFS initialized"
else
    fail "VFS NOT initialized"
fi

# Test 6: Memory manager
if grep -q "Memory" "$LOG_FILE" || grep -q "PMM" "$LOG_FILE"; then
    pass "Memory manager initialized"
else
    fail "Memory manager NOT initialized"
fi

echo ""
echo "=========================================="
echo "   Summary"
echo "=========================================="
echo "Passed: $TESTS_PASSED"
echo "Failed: $TESTS_FAILED"
echo "Log: $LOG_FILE"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL TESTS PASSED${NC}"
    exit 0
else
    echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    echo ""
    echo "Recent log output:"
    tail -20 "$LOG_FILE"
    exit 1
fi
