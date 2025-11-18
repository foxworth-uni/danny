#!/bin/bash

# Danny Benchmark Setup Script
# This script sets up everything needed to run benchmarks

set -e

echo "🚀 Setting up Danny Benchmark Suite"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "package.json" ]; then
    echo -e "${RED}Error: Must run from packages/benchmark directory${NC}"
    exit 1
fi

# Step 1: Build Danny
echo -e "${BLUE}Step 1: Building Danny...${NC}"
cd ../..
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Cannot find Cargo.toml${NC}"
    exit 1
fi

if cargo build --release; then
    echo -e "${GREEN}✓ Danny built successfully${NC}"
else
    echo -e "${RED}✗ Failed to build Danny${NC}"
    exit 1
fi

# Check if danny binary exists
if [ ! -f "target/release/danny" ]; then
    echo -e "${RED}✗ Danny binary not found at target/release/danny${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Danny binary found${NC}"
echo ""

# Step 2: Install benchmark dependencies
echo -e "${BLUE}Step 2: Installing benchmark dependencies...${NC}"
cd packages/benchmark

if pnpm install; then
    echo -e "${GREEN}✓ Benchmark dependencies installed${NC}"
else
    echo -e "${YELLOW}⚠ Failed to install with pnpm, trying npm...${NC}"
    if npm install; then
        echo -e "${GREEN}✓ Benchmark dependencies installed with npm${NC}"
    else
        echo -e "${RED}✗ Failed to install dependencies${NC}"
        exit 1
    fi
fi
echo ""

# Step 3: Install test app dependencies
echo -e "${BLUE}Step 3: Installing test app dependencies...${NC}"
cd ../../test-files/nextjs-app

if pnpm install; then
    echo -e "${GREEN}✓ Test app dependencies installed${NC}"
else
    echo -e "${YELLOW}⚠ Failed to install with pnpm, trying npm...${NC}"
    if npm install; then
        echo -e "${GREEN}✓ Test app dependencies installed with npm${NC}"
    else
        echo -e "${YELLOW}⚠ Failed to install test app dependencies${NC}"
        echo -e "${YELLOW}  Knip benchmarks may not work properly${NC}"
    fi
fi
echo ""

# Step 4: Create results directory
echo -e "${BLUE}Step 4: Creating results directory...${NC}"
cd ../../packages/benchmark
mkdir -p results
echo -e "${GREEN}✓ Results directory created${NC}"
echo ""

# Step 5: Run a quick test
echo -e "${BLUE}Step 5: Running quick test...${NC}"
echo ""

echo -e "${YELLOW}Testing Danny...${NC}"
if ../../target/release/danny ../../test-files/nextjs-app > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Danny test successful${NC}"
else
    echo -e "${RED}✗ Danny test failed${NC}"
    echo -e "${YELLOW}  This might be okay if Danny exits with non-zero on findings${NC}"
fi

echo -e "${YELLOW}Testing Knip...${NC}"
cd ../../test-files/nextjs-app
if npx knip --version > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Knip is available${NC}"
else
    echo -e "${YELLOW}⚠ Knip test failed${NC}"
    echo -e "${YELLOW}  You may need to install it: npm install -g knip${NC}"
fi

cd ../../packages/benchmark
echo ""

# Done!
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Setup complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo ""
echo -e "  ${YELLOW}Run a benchmark:${NC}"
echo -e "    pnpm benchmark"
echo ""
echo -e "  ${YELLOW}Generate HTML report:${NC}"
echo -e "    pnpm benchmark --format html"
echo ""
echo -e "  ${YELLOW}Compare results:${NC}"
echo -e "    pnpm benchmark:compare"
echo ""
echo -e "  ${YELLOW}View help:${NC}"
echo -e "    pnpm benchmark --help"
echo ""
echo -e "${BLUE}For more information, see BENCHMARK_GUIDE.md${NC}"
echo ""

