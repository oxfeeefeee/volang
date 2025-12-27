#!/bin/bash
# Vo Integration Test Runner
#
# Usage:
#   ./test.sh              # Run all tests (both VM and JIT modes)
#   ./test.sh vm           # Run only VM mode tests
#   ./test.sh jit          # Run only JIT mode tests
#   ./test.sh -v           # Verbose mode (show all output)
#   ./test.sh <file.vo>   # Run a single test file
#
# Configuration: test_data/_skip.yaml (lists tests to skip)
# Tests NOT in _skip.yaml run in both vm and jit modes.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/.."

TEST_DIR="test_data"
SKIP_CONFIG="$TEST_DIR/_skip.yaml"
BIN="target/debug/vo"
MODE="${1:-both}"
VERBOSE=false

# Helper: check if file should skip a mode
should_skip() {
    local file="$1"
    local mode="$2"
    
    if [[ ! -f "$SKIP_CONFIG" ]]; then
        return 1
    fi
    
    # Find the file entry and check its skip modes
    local in_entry=false
    local found_file=false
    while IFS= read -r line; do
        if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*file:[[:space:]]*(.+)$ ]]; then
            if $found_file; then
                break
            fi
            local entry_file="${BASH_REMATCH[1]}"
            if [[ "$entry_file" == "$file" ]]; then
                found_file=true
            fi
        elif $found_file && [[ "$line" =~ ^[[:space:]]*skip:[[:space:]]*\[(.+)\]$ ]]; then
            local modes="${BASH_REMATCH[1]}"
            if [[ "$modes" == *"$mode"* ]]; then
                return 0
            fi
        fi
    done < "$SKIP_CONFIG"
    return 1
}

# Helper: check if file expects error
expects_error() {
    local file="$1"
    
    if [[ ! -f "$SKIP_CONFIG" ]]; then
        return 1
    fi
    
    local found_file=false
    while IFS= read -r line; do
        if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*file:[[:space:]]*(.+)$ ]]; then
            if $found_file; then
                break
            fi
            local entry_file="${BASH_REMATCH[1]}"
            if [[ "$entry_file" == "$file" ]]; then
                found_file=true
            fi
        elif $found_file && [[ "$line" =~ ^[[:space:]]*expect_error:[[:space:]]*true ]]; then
            return 0
        fi
    done < "$SKIP_CONFIG"
    return 1
}

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# Parse arguments
if [[ "$1" == "-v" || "$1" == "--verbose" ]]; then
    VERBOSE=true
    MODE="${2:-both}"
elif [[ "$1" == "-h" || "$1" == "--help" ]]; then
    echo "Vo Integration Test Runner"
    echo ""
    echo "Usage:"
    echo "  ./test.sh              # Run all tests (both VM and JIT modes)"
    echo "  ./test.sh vm           # Run only VM mode tests"
    echo "  ./test.sh jit          # Run only JIT mode tests"
    echo "  ./test.sh -v           # Verbose mode"
    echo "  ./test.sh <file.vo>   # Run a single test file"
    exit 0
elif [[ -f "$1" ]]; then
    # Single file mode
    echo "Running single test: $1"
    cargo build -q -p vo-cli || exit 1
    "$BIN" run "$1" --mode=vm
    exit $?
fi

# Build CLI
echo -e "${DIM}Building vo-cli...${NC}"
cargo build -q -p vo-cli || exit 1

# Counters
vm_passed=0
vm_failed=0
jit_passed=0
jit_failed=0
skipped=0

passed_list=""
failed_list=""

run_test() {
    local file="$1"
    local mode="$2"
    local expect_error="$3"
    
    local path="$TEST_DIR/$file"
    
    # Check if path exists
    if [[ ! -e "$path" ]]; then
        echo -e "  ${YELLOW}⊘${NC} $file [not found]"
        return
    fi
    
    # Run the test
    local output
    local exit_code
    
    if $VERBOSE; then
        echo -e "${DIM}Running: $BIN run $path --mode=$mode${NC}"
    fi
    
    output=$("$BIN" run "$path" --mode="$mode" 2>&1) || true
    
    # Check output for result tags
    local has_error=false
    if echo "$output" | grep -q "\[VO:PANIC:\|\[VO:ERROR:"; then
        has_error=true
    fi
    
    # Also check for Rust panics (internal errors)
    local has_rust_panic=false
    if echo "$output" | grep -q "panicked at"; then
        has_rust_panic=true
    fi
    
    # Handle expect_error tests (negative tests)
    if [[ "$expect_error" == "true" ]]; then
        if $has_error; then
            # Expected error occurred - test passes
            if [[ "$mode" == "vm" ]]; then
                vm_passed=$((vm_passed + 1))
            else
                jit_passed=$((jit_passed + 1))
            fi
            passed_list="$passed_list  ${GREEN}✓${NC} $file [$mode] (expected error)\n"
        else
            # Expected error but got success - test fails
            if [[ "$mode" == "vm" ]]; then
                vm_failed=$((vm_failed + 1))
            else
                jit_failed=$((jit_failed + 1))
            fi
            failed_list="$failed_list  ${RED}✗${NC} $file [$mode] (expected error but passed)\n"
        fi
        return
    fi
    
    # Normal test handling
    if echo "$output" | grep -q "\[VO:OK\]"; then
        if [[ "$mode" == "vm" ]]; then
            vm_passed=$((vm_passed + 1))
        else
            jit_passed=$((jit_passed + 1))
        fi
        passed_list="$passed_list  ${GREEN}✓${NC} $file [$mode]\n"
    elif echo "$output" | grep -q "\[VO:PANIC:"; then
        if [[ "$mode" == "vm" ]]; then
            vm_failed=$((vm_failed + 1))
        else
            jit_failed=$((jit_failed + 1))
        fi
        local panic_msg=$(echo "$output" | grep -o "\[VO:PANIC:[^]]*\]" | head -1)
        failed_list="$failed_list  ${RED}✗${NC} $file [$mode] $panic_msg\n"
    elif echo "$output" | grep -q "\[VO:ERROR:"; then
        if [[ "$mode" == "vm" ]]; then
            vm_failed=$((vm_failed + 1))
        else
            jit_failed=$((jit_failed + 1))
        fi
        local error_msg=$(echo "$output" | grep -o "\[VO:ERROR:[^]]*\]" | head -1)
        failed_list="$failed_list  ${RED}✗${NC} $file [$mode] $error_msg\n"
    elif $has_rust_panic; then
        # Rust panic (internal compiler/runtime error)
        if [[ "$mode" == "vm" ]]; then
            vm_failed=$((vm_failed + 1))
        else
            jit_failed=$((jit_failed + 1))
        fi
        local panic_line=$(echo "$output" | grep "panicked at" | head -1 | sed 's/.*panicked at //' | cut -c1-60)
        failed_list="$failed_list  ${RED}✗${NC} $file [$mode] [RUST PANIC: $panic_line]\n"
    elif echo "$output" | grep -q "^error:"; then
        # CLI error output (e.g., "error: analysis error: type check failed")
        if [[ "$mode" == "vm" ]]; then
            vm_failed=$((vm_failed + 1))
        else
            jit_failed=$((jit_failed + 1))
        fi
        local error_line=$(echo "$output" | grep "^error:" | head -1 | cut -c1-60)
        failed_list="$failed_list  ${RED}✗${NC} $file [$mode] [$error_line]\n"
    else
        # No [VO:OK] tag and no error - consider it a failure (missing success marker)
        if [[ "$mode" == "vm" ]]; then
            vm_failed=$((vm_failed + 1))
        else
            jit_failed=$((jit_failed + 1))
        fi
        failed_list="$failed_list  ${RED}✗${NC} $file [$mode] [no [VO:OK] marker]\n"
    fi
    
    if $VERBOSE; then
        echo "$output"
    fi
}

echo -e "${BOLD}Running Vo integration tests...${NC}\n"

# Find and run all .vo test files (recursive)
while IFS= read -r path; do
    # Get relative path from test_data/
    file="${path#$TEST_DIR/}"
    
    # Skip files in proj_* directories (handled as multi-file projects)
    if [[ "$file" == proj_*/* ]]; then
        continue
    fi
    
    # Check if expect_error
    is_expect_error=""
    if expects_error "$file"; then
        is_expect_error="true"
    fi
    
    # Run VM mode
    if [[ "$MODE" == "vm" || "$MODE" == "both" ]]; then
        if should_skip "$file" "vm"; then
            skipped=$((skipped + 1))
            if $VERBOSE; then
                echo -e "  ${YELLOW}⊘${NC} $file [vm skipped]"
            fi
        else
            run_test "$file" "vm" "$is_expect_error"
        fi
    fi
    
    # Run JIT mode
    if [[ "$MODE" == "jit" || "$MODE" == "both" ]]; then
        if should_skip "$file" "jit"; then
            skipped=$((skipped + 1))
            if $VERBOSE; then
                echo -e "  ${YELLOW}⊘${NC} $file [jit skipped]"
            fi
        else
            run_test "$file" "jit" "$is_expect_error"
        fi
    fi
done < <(find "$TEST_DIR" -name "*.vo" -type f | sort)

# Run tests for project directories
while IFS= read -r dir_path; do
    [[ -z "$dir_path" ]] && continue
    dir="${dir_path#$TEST_DIR/}/"
    
    # Check if expect_error
    is_expect_error=""
    if expects_error "$dir"; then
        is_expect_error="true"
    fi
    
    # Run VM mode
    if [[ "$MODE" == "vm" || "$MODE" == "both" ]]; then
        if should_skip "$dir" "vm"; then
            skipped=$((skipped + 1))
            if $VERBOSE; then
                echo -e "  ${YELLOW}⊘${NC} $dir [vm skipped]"
            fi
        else
            run_test "$dir" "vm" "$is_expect_error"
        fi
    fi
    
    # Run JIT mode
    if [[ "$MODE" == "jit" || "$MODE" == "both" ]]; then
        if should_skip "$dir" "jit"; then
            skipped=$((skipped + 1))
            if $VERBOSE; then
                echo -e "  ${YELLOW}⊘${NC} $dir [jit skipped]"
            fi
        else
            run_test "$dir" "jit" "$is_expect_error"
        fi
    fi
done < <(find "$TEST_DIR" -maxdepth 1 -type d -name "proj_*" | sort)

total_passed=$((vm_passed + jit_passed))
total_failed=$((vm_failed + jit_failed))

# Print results
echo ""

if [[ -n "$failed_list" ]]; then
    echo -e "${RED}${BOLD}Failed:${NC}"
    echo -e "$failed_list"
fi

echo -e "${CYAN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}${BOLD}                   Vo Test Results                        ${NC}${CYAN}║${NC}"
echo -e "${CYAN}╠══════════════════════════════════════════════════════════╣${NC}"
printf "${CYAN}║${NC}  VM:  ${GREEN}%3d passed${NC}  ${RED}%3d failed${NC}                             ${CYAN}║${NC}\n" "$vm_passed" "$vm_failed"
printf "${CYAN}║${NC}  JIT: ${GREEN}%3d passed${NC}  ${RED}%3d failed${NC}                             ${CYAN}║${NC}\n" "$jit_passed" "$jit_failed"
if [[ $skipped -gt 0 ]]; then
    printf "${CYAN}║${NC}  Skipped: ${YELLOW}%3d${NC}                                            ${CYAN}║${NC}\n" "$skipped"
fi
echo -e "${CYAN}╠══════════════════════════════════════════════════════════╣${NC}"
printf "${CYAN}║${NC}  Total: ${GREEN}%3d passed${NC}  ${RED}%3d failed${NC}                           ${CYAN}║${NC}\n" "$total_passed" "$total_failed"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════╝${NC}"

[[ "$total_failed" -gt 0 ]] && exit 1 || exit 0
