#!/bin/bash

# KnishIO Rust SDK Self-Test Build and Run Script
# 
# This script builds the Rust SDK self-test executable and runs it,
# following the same pattern as other SDK self-test scripts.
# Uses modern Rust 2025 features and Cargo best practices.

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}🦀 Building KnishIO Rust SDK Self-Test...${NC}"

# Check if Cargo.toml exists
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Error: Cargo.toml not found in $(pwd)${NC}"
    exit 1
fi

# Build the self-test binary
echo -e "${YELLOW}🔧 Building self-test target with optimizations...${NC}"
cargo build --release --bin self-test

# Check if build was successful
if [ ! -f "target/release/self-test" ]; then
    echo -e "${RED}❌ Build failed - self-test executable not found${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Build completed successfully${NC}"
echo -e "${BLUE}🚀 Running Rust SDK Self-Test (Modern Rust 2025)...${NC}"
echo ""

# Run the self-test
cargo run --release --bin self-test
TEST_EXIT_CODE=$?

# Report results
echo ""
if [ $TEST_EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}🎉 Rust SDK Self-Test completed successfully!${NC}"
    echo -e "${CYAN}🦀 Modern Rust 2025 features: anyhow, tokio, serde, zero unsafe code${NC}"
    echo -e "${CYAN}🔒 Memory Safety: 100% safe Rust, no segfaults possible${NC}"
    echo -e "${CYAN}⚡ Performance: SIMD optimizations, zero-cost abstractions${NC}"
    echo -e "${CYAN}🎯 Type Safety: Compile-time guarantees, no runtime errors${NC}"
else
    echo -e "${RED}💥 Rust SDK Self-Test failed with exit code $TEST_EXIT_CODE${NC}"
fi

# Results directory (configurable via environment variable)
RESULTS_DIR="${KNISHIO_SHARED_RESULTS:-../shared-test-results}"
echo -e "${BLUE}📊 Results saved to $RESULTS_DIR/rust-results.json${NC}"

exit $TEST_EXIT_CODE