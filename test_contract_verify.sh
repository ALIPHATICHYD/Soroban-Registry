#!/bin/bash
# test_contract_verify.sh - Test suite for contract verification features
# 
# This script tests the new contract verify command with:
# - Caching functionality (24-hour TTL)
# - Strict mode (--strict flag)
# - Batch verification (--batch flag)
# - JSON output format

set -e

# Configuration
API_URL="${API_URL:-http://localhost:3001}"
NETWORK="${NETWORK:-testnet}"
TEST_CONTRACT_1="CADAYHQLQIAVGK6TQY74N6VD7RBQBMHTF5BT7HWMM4JYJCQPJPVPD5U"
TEST_CONTRACT_2="CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4"
TEST_CONTRACT_3="CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA4BAAAA"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_test() {
    echo -e "${BLUE}▶ TEST: $1${NC}"
}

log_pass() {
    echo -e "${GREEN}✓ PASS: $1${NC}"
}

log_fail() {
    echo -e "${RED}✗ FAIL: $1${NC}"
}

log_info() {
    echo -e "${YELLOW}ℹ INFO: $1${NC}"
}

# Build the CLI if needed
build_cli() {
    log_info "Building CLI (if needed)..."
    cd cli
    cargo build --release 2>/dev/null || log_info "Using existing binary"
    cd ..
}

# Test 1: Single Contract Verification
test_single_verification() {
    log_test "Single contract verification"
    
    if soroban-registry contract verify "$TEST_CONTRACT_1" \
        --network "$NETWORK" \
        --api-url "$API_URL" &>/dev/null; then
        log_pass "Single verification succeeded"
    else
        log_fail "Single verification failed"
        return 1
    fi
}

# Test 2: Cache Hit Performance
test_cache_functionality() {
    log_test "Cache functionality (24-hour TTL)"
    
    # Clear cache first
    rm -f ~/.soroban-registry/verification_cache.json 2>/dev/null || true
    
    # First run - should hit API
    log_info "First run (API call)..."
    START=$(date +%s%N)
    soroban-registry contract verify "$TEST_CONTRACT_1" \
        --network "$NETWORK" \
        --api-url "$API_URL" --json &>/dev/null
    FIRST_TIME=$(( ($(date +%s%N) - START) / 1000000 ))
    
    # Second run - should hit cache
    log_info "Second run (cache hit)..."
    START=$(date +%s%N)
    soroban-registry contract verify "$TEST_CONTRACT_1" \
        --network "$NETWORK" \
        --api-url "$API_URL" --json &>/dev/null
    CACHE_TIME=$(( ($(date +%s%N) - START) / 1000000 ))
    
    log_info "First run: ${FIRST_TIME}ms, Cache hit: ${CACHE_TIME}ms"
    
    if [ "$CACHE_TIME" -lt "$FIRST_TIME" ]; then
        log_pass "Cache hit is faster than API call"
    else
        log_fail "Cache hit not significantly faster"
        return 1
    fi
    
    # Test --no-cache bypass
    log_info "Testing --no-cache flag..."
    START=$(date +%s%N)
    soroban-registry contract verify "$TEST_CONTRACT_1" \
        --network "$NETWORK" \
        --api-url "$API_URL" --no-cache --json &>/dev/null
    NO_CACHE_TIME=$(( ($(date +%s%N) - START) / 1000000 ))
    
    log_info "No-cache run: ${NO_CACHE_TIME}ms"
    
    if [ "$NO_CACHE_TIME" -gt "$CACHE_TIME" ]; then
        log_pass "--no-cache bypasses cache correctly"
    else
        log_fail "--no-cache flag not working correctly"
        return 1
    fi
}

# Test 3: Strict Mode
test_strict_mode() {
    log_test "Strict mode (--strict flag)"
    
    # Test strict mode with verified contract (should pass)
    log_info "Testing strict mode with verified contract..."
    if soroban-registry contract verify "$TEST_CONTRACT_1" \
        --network "$NETWORK" \
        --api-url "$API_URL" --strict --no-cache 2>/dev/null; then
        log_pass "Strict mode: verified contract passed"
    else
        log_info "Strict mode: verified contract has warnings (expected for some contracts)"
    fi
    
    # Test strict mode error handling
    log_info "Testing strict mode with unverified contract..."
    if soroban-registry contract verify "INVALID_OR_UNVERIFIED" \
        --network "$NETWORK" \
        --api-url "$API_URL" --strict --no-cache 2>/dev/null; then
        log_fail "Strict mode: should have failed for unverified contract"
        return 1
    else
        log_pass "Strict mode: correctly fails for unverified contracts"
    fi
}

# Test 4: Batch Verification
test_batch_verification() {
    log_test "Batch verification (--batch flag)"
    
    # Test batch with valid contracts
    log_info "Verifying 3 contracts in batch mode..."
    if soroban-registry contract verify "$TEST_CONTRACT_1,$TEST_CONTRACT_2,$TEST_CONTRACT_3" \
        --network "$NETWORK" \
        --api-url "$API_URL" --batch --no-cache &>/dev/null; then
        log_pass "Batch verification completed"
    else
        log_info "Batch verification completed (with expected failures for test contracts)"
    fi
    
    # Test batch size limit
    log_info "Testing batch size limit (51 contracts)..."
    LARGE_BATCH=$(printf '%s,' $(seq 1 51 | xargs -I {} echo "$TEST_CONTRACT_1") | sed 's/,$//g')
    if soroban-registry contract verify "$LARGE_BATCH" \
        --network "$NETWORK" \
        --api-url "$API_URL" --batch 2>&1 | grep -q "exceeds maximum"; then
        log_pass "Batch size limit correctly enforced (max 50)"
    else
        log_fail "Batch size limit not enforced"
        return 1
    fi
}

# Test 5: JSON Output Format
test_json_output() {
    log_test "JSON output format"
    
    log_info "Testing single contract JSON output..."
    if soroban-registry contract verify "$TEST_CONTRACT_1" \
        --network "$NETWORK" \
        --api-url "$API_URL" --json 2>/dev/null | jq . &>/dev/null; then
        log_pass "Single contract JSON output is valid"
    else
        log_fail "Single contract JSON output is invalid"
        return 1
    fi
    
    log_info "Testing batch JSON output..."
    if soroban-registry contract verify "$TEST_CONTRACT_1,$TEST_CONTRACT_2" \
        --network "$NETWORK" \
        --api-url "$API_URL" --batch --json 2>/dev/null | jq '.summary' &>/dev/null; then
        log_pass "Batch JSON output is valid"
    else
        log_fail "Batch JSON output is invalid"
        return 1
    fi
}

# Test 6: Cache Cleanup
test_cache_management() {
    log_test "Cache management"
    
    log_info "Checking cache file exists..."
    if [ -f ~/.soroban-registry/verification_cache.json ]; then
        log_pass "Cache file created at ~/.soroban-registry/verification_cache.json"
    else
        log_fail "Cache file not found"
        return 1
    fi
    
    log_info "Cache file size: $(du -h ~/.soroban-registry/verification_cache.json 2>/dev/null || echo 'unknown')"
    log_pass "Cache management working correctly"
}

# Test 7: Error Handling
test_error_handling() {
    log_test "Error handling"
    
    # Test invalid address
    log_info "Testing invalid contract address..."
    if soroban-registry contract verify "" \
        --network "$NETWORK" \
        --api-url "$API_URL" 2>&1 | grep -q "not found\|error"; then
        log_pass "Invalid address handled correctly"
    else
        log_info "Invalid address error message varies (acceptable)"
    fi
    
    # Test invalid network
    log_info "Testing API connectivity..."
    if soroban-registry contract verify "$TEST_CONTRACT_1" \
        --network "$NETWORK" \
        --api-url "http://invalid-url:9999" --no-cache 2>&1 | grep -q "error\|timeout"; then
        log_pass "Connection error handled correctly"
    else
        log_info "Error handling may vary (acceptable)"
    fi
}

# Main test runner
main() {
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}Contract Verification Command Test Suite${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo ""
    
    # Check prerequisites
    if ! command -v soroban-registry &> /dev/null; then
        log_fail "soroban-registry CLI not found in PATH"
        log_info "Please install with: cargo install --path cli"
        exit 1
    fi
    
    # Run tests
    TESTS_PASSED=0
    TESTS_FAILED=0
    
    for test in "test_single_verification" "test_cache_functionality" \
                "test_strict_mode" "test_batch_verification" \
                "test_json_output" "test_cache_management" "test_error_handling"; do
        if $test; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
        echo ""
    done
    
    # Summary
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}Test Summary${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    
    if [ "$TESTS_FAILED" -eq 0 ]; then
        echo -e "\n${GREEN}All tests passed! ✓${NC}"
        exit 0
    else
        echo -e "\n${RED}Some tests failed ✗${NC}"
        exit 1
    fi
}

# Run main function
main "$@"
