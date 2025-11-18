# Default recipe shows available commands
default:
    @just --list

# ============================================================================
# Setup and Installation
# ============================================================================

# Setup repository for development (run this first!)
setup:
    @echo "🚀 Setting up danny development environment..."
    @echo ""
    @just _check-dependencies
    @echo ""
    @just _install-rust-deps
    @echo ""
    @just _install-node-deps
    @echo ""
    @just _build-all
    @echo ""
    @echo "✅ Setup complete! You're ready to develop."
    @echo ""
    @echo "Quick start:"
    @echo "  just fixtures    - Run test fixtures interactively"
    @echo "  just dev         - Build and run in development mode"
    @echo "  just test        - Run all tests"

# Check if required dependencies are installed
_check-dependencies:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "📋 Checking dependencies..."
    
    # Check Rust
    if ! command -v rustc &> /dev/null; then
        echo "❌ Rust not found. Install from: https://rustup.rs"
        exit 1
    fi
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    echo "✓ Rust $RUST_VERSION"
    
    # Check Cargo
    if ! command -v cargo &> /dev/null; then
        echo "❌ Cargo not found."
        exit 1
    fi
    echo "✓ Cargo $(cargo --version | cut -d' ' -f2)"
    
    # Check Node.js
    if ! command -v node &> /dev/null; then
        echo "❌ Node.js not found. Install Node.js 18+ from: https://nodejs.org"
        exit 1
    fi
    NODE_VERSION=$(node --version)
    echo "✓ Node.js $NODE_VERSION"
    
    # Check pnpm
    if ! command -v pnpm &> /dev/null; then
        echo "⚠️  pnpm not found. Installing..."
        npm install -g pnpm@10.22.0
    fi
    echo "✓ pnpm $(pnpm --version)"
    
    # Check gum (optional but recommended)
    if ! command -v gum &> /dev/null; then
        echo "⚠️  gum not found (optional, for better UI)"
        echo "   Install with: brew install gum"
        echo "   Or visit: https://github.com/charmbracelet/gum"
    else
        echo "✓ gum $(gum --version | head -1)"
    fi
    
    # Check just itself
    if ! command -v just &> /dev/null; then
        echo "✓ just (you're running it!)"
    else
        echo "✓ just $(just --version)"
    fi

# Install Rust dependencies
_install-rust-deps:
    @echo "📦 Installing Rust dependencies..."
    cargo fetch

# Install Node.js dependencies
_install-node-deps:
    @echo "📦 Installing Node.js dependencies..."
    pnpm install

# Build everything
_build-all:
    @echo "🔨 Building Rust workspace..."
    cargo build --workspace
    @echo "🔨 Building Node.js packages..."
    pnpm -r build

# ============================================================================
# Development Commands
# ============================================================================

# Build in development mode
dev:
    cargo build --workspace

# Build in release mode
build:
    cargo build --workspace --release

# Build the CLI only (faster)
build-cli:
    cargo build --bin danny --release

# Run all tests (Rust + Node.js)
test:
    @echo "🧪 Running Rust tests..."
    cargo test --workspace
    @echo "🧪 Running Node.js tests..."
    pnpm test

# Run Rust tests only
test-rust:
    cargo test --workspace

# Run Rust tests with output
test-rust-verbose:
    cargo test --workspace -- --nocapture

# Run linting
lint:
    cargo clippy --workspace --all-targets --all-features

# Format code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all --check

# Run all checks (fmt, lint, test)
check: fmt-check lint test

# Clean build artifacts
clean:
    cargo clean
    pnpm -r exec rm -rf dist node_modules
    rm -rf target

# ============================================================================
# Fixture Testing
# ============================================================================

# List and select a fixture to run interactively
fixtures:
    #!/usr/bin/env bash
    if command -v gum &> /dev/null; then
        FIXTURE=$(gum choose \
            "example1.js - JavaScript dead code examples" \
            "example2.ts - TypeScript dead code examples" \
            "example3.jsx - JSX dead code examples" \
            "test-combined-features.ts - Exported vs internal" \
            "test-namespace-container.ts - Namespace containers" \
            "test-enum-container.ts - Enum containers" \
            "nextjs-app - Full Next.js application" \
            "all - Run all fixtures")
        
        case "$FIXTURE" in
            example1.js*) just fixture-example1 ;;
            example2.ts*) just fixture-example2 ;;
            example3.jsx*) just fixture-example3 ;;
            test-combined*) just fixture-combined ;;
            test-namespace*) just fixture-namespace ;;
            test-enum*) just fixture-enum ;;
            nextjs-app*) just fixture-nextjs ;;
            all*) just fixture-all ;;
        esac
    else
        echo "📝 Select a fixture to run:"
        echo ""
        echo "1) example1.js - JavaScript dead code examples"
        echo "2) example2.ts - TypeScript dead code examples"
        echo "3) example3.jsx - JSX dead code examples"
        echo "4) test-combined-features.ts - Exported vs internal"
        echo "5) test-namespace-container.ts - Namespace containers"
        echo "6) test-enum-container.ts - Enum containers"
        echo "7) nextjs-app - Full Next.js application"
        echo "8) all - Run all fixtures"
        echo ""
        read -p "Enter number (1-8): " choice
        case $choice in
            1) just fixture-example1 ;;
            2) just fixture-example2 ;;
            3) just fixture-example3 ;;
            4) just fixture-combined ;;
            5) just fixture-namespace ;;
            6) just fixture-enum ;;
            7) just fixture-nextjs ;;
            8) just fixture-all ;;
            *) echo "Invalid selection" ;;
        esac
    fi

# Run example1.js fixture - basic JavaScript dead code
fixture-example1:
    @echo "🧪 Running fixture: example1.js"
    cargo run --release --bin danny -- test-files/example1.js --detect-unused-symbols

# Run example2.ts fixture - TypeScript dead code
fixture-example2:
    @echo "🧪 Running fixture: example2.ts"
    cargo run --release --bin danny -- test-files/example2.ts --detect-unused-symbols

# Run example3.jsx fixture - JSX dead code
fixture-example3:
    @echo "🧪 Running fixture: example3.jsx"
    cargo run --release --bin danny -- test-files/example3.jsx --detect-unused-symbols

# Run test-combined-features.ts fixture - exported vs internal
fixture-combined:
    @echo "🧪 Running fixture: test-combined-features.ts"
    cargo run --release --bin danny -- test-files/test-combined-features.ts --detect-unused-symbols

# Run test-namespace-container.ts fixture
fixture-namespace:
    @echo "🧪 Running fixture: test-namespace-container.ts"
    cargo run --release --bin danny -- test-files/test-namespace-container.ts --detect-unused-symbols

# Run test-enum-container.ts fixture
fixture-enum:
    @echo "🧪 Running fixture: test-enum-container.ts"
    cargo run --release --bin danny -- test-files/test-enum-container.ts --detect-unused-symbols

# Run nextjs-app fixture - full application analysis
fixture-nextjs:
    @echo "🧪 Running fixture: nextjs-app"
    cargo run --release --bin danny -- test-files/nextjs-app --detect-unused-symbols

# Run all fixtures
fixture-all:
    @echo "🧪 Running all fixtures..."
    @echo ""
    @just fixture-example1
    @echo ""
    @just fixture-example2
    @echo ""
    @just fixture-example3
    @echo ""
    @just fixture-combined
    @echo ""
    @just fixture-namespace
    @echo ""
    @just fixture-enum
    @echo ""
    @just fixture-nextjs

# Run fixture with quality analysis enabled
fixture-quality FIXTURE:
    @echo "🧪 Running fixture with quality analysis: {{FIXTURE}}"
    cargo run --release --bin danny -- test-files/{{FIXTURE}} --detect-unused-symbols --quality

# Run fixture with JSON output
fixture-json FIXTURE:
    @echo "🧪 Running fixture with JSON output: {{FIXTURE}}"
    cargo run --release --bin danny -- test-files/{{FIXTURE}} --detect-unused-symbols --format json

# ============================================================================
# Benchmarking
# ============================================================================

# Run benchmarks
benchmark:
    cd packages/benchmark && pnpm benchmark

# Run benchmarks comparing Danny vs Knip
benchmark-compare:
    cd packages/benchmark && pnpm benchmark:compare

# Compare findings between tools
benchmark-findings:
    cd packages/benchmark && pnpm compare:findings

# ============================================================================
# Release & Distribution
# ============================================================================

# Build release binaries
release:
    cargo build --workspace --release
    @echo ""
    @echo "✅ Release binaries built:"
    @echo "   • danny CLI: target/release/danny"
    @echo "   • danny desktop: target/release/danny-desktop"

# Install danny CLI locally
install:
    cargo install --path crates/danny-cli --force

# ============================================================================
# Documentation
# ============================================================================

# Generate and open Rust documentation
docs:
    cargo doc --workspace --no-deps --open

# Generate documentation without opening
docs-build:
    cargo doc --workspace --no-deps

# ============================================================================
# Utility Commands
# ============================================================================

# Show project statistics
stats:
    @echo "📊 Project Statistics"
    @echo ""
    @echo "Rust code:"
    @find crates -name "*.rs" | xargs wc -l | tail -1
    @echo ""
    @echo "JavaScript/TypeScript code:"
    @find packages -name "*.js" -o -name "*.ts" | xargs wc -l 2>/dev/null | tail -1 || echo "  No JS/TS files found"
    @echo ""
    @echo "Test fixtures:"
    @find test-files -type f | xargs wc -l 2>/dev/null | tail -1

# Watch and rebuild on changes (requires cargo-watch)
watch:
    cargo watch -x "build --workspace"

# Watch and run tests on changes (requires cargo-watch)
watch-test:
    cargo watch -x "test --workspace"

