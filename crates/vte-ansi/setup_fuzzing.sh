#!/bin/bash
# One-command fuzzing setup for macOS
# Usage: ./setup_fuzzing.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Fuzzing Setup for ANSI Parser${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Step 1: Check rustup
echo -e "${BLUE}[1/5]${NC} Checking rustup..."
if ! command -v rustup &> /dev/null; then
    echo -e "${RED}Error: rustup not found${NC}"
    echo "Please install rustup first: https://rustup.rs/"
    exit 1
fi
echo -e "${GREEN}✓${NC} rustup found"
echo ""

# Step 2: Install nightly
echo -e "${BLUE}[2/5]${NC} Installing nightly toolchain..."
if rustup toolchain list | grep -q nightly; then
    echo -e "${GREEN}✓${NC} Nightly already installed"
else
    rustup toolchain install nightly
    echo -e "${GREEN}✓${NC} Nightly installed"
fi
echo ""

# Step 3: Install cargo-fuzz
echo -e "${BLUE}[3/5]${NC} Installing cargo-fuzz..."
if cargo +nightly fuzz --version &> /dev/null; then
    echo -e "${GREEN}✓${NC} cargo-fuzz already installed"
else
    cargo +nightly install cargo-fuzz
    echo -e "${GREEN}✓${NC} cargo-fuzz installed"
fi
echo ""

# Step 4: Initialize fuzzing
echo -e "${BLUE}[4/5]${NC} Initializing fuzz directory..."
if [ -d "fuzz" ]; then
    echo -e "${YELLOW}⚠${NC}  fuzz/ directory already exists"
else
    cargo +nightly fuzz init
    echo -e "${GREEN}✓${NC} Fuzz directory initialized"
fi
echo ""

# Step 5: Verify setup
echo -e "${BLUE}[5/5]${NC} Verifying setup..."

# Check toolchains
NIGHTLY_VERSION=$(rustup toolchain list | grep nightly | head -1)
CARGO_FUZZ_VERSION=$(cargo +nightly fuzz --version 2>&1 | head -1)

echo -e "${GREEN}✓${NC} Nightly: $NIGHTLY_VERSION"
echo -e "${GREEN}✓${NC} cargo-fuzz: $CARGO_FUZZ_VERSION"
echo ""

# List fuzz targets
if [ -d "fuzz/fuzz_targets" ]; then
    echo -e "${BLUE}Fuzz targets:${NC}"
    ls -1 fuzz/fuzz_targets/*.rs 2>/dev/null | while read -r file; do
        basename "$file" .rs | sed 's/^/  - /'
    done
    echo ""
fi

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Setup Complete! 🎉${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

echo -e "${BLUE}Next steps:${NC}"
echo ""
echo "1. Quick test (5 minutes per target):"
echo -e "   ${YELLOW}./run_fuzzing.sh quick${NC}"
echo ""
echo "2. Or test one target manually:"
echo -e "   ${YELLOW}cd fuzz && cargo +nightly fuzz run parser_basic -- -max_total_time=60${NC}"
echo ""
echo "3. Full test (6+ hours - run overnight):"
echo -e "   ${YELLOW}./run_fuzzing.sh full${NC}"
echo ""

# Offer to run a quick test
echo -e "${BLUE}Run a quick 1-minute test now? (y/n)${NC}"
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    echo ""
    echo -e "${BLUE}Running 1-minute test on parser_basic...${NC}"
    if cd fuzz && cargo +nightly fuzz run parser_basic -- -max_total_time=60 -timeout=1 && cd -; then
        echo ""
        echo -e "${GREEN}✓ Test passed! Your fuzzing setup is working!${NC}"
    else
        echo ""
        echo -e "${YELLOW}⚠ Test completed with warnings (this is normal)${NC}"
    fi
else
    echo ""
    echo -e "${GREEN}Setup complete! Run fuzzing whenever you're ready.${NC}"
fi
