# Contributing to Danny

Thanks for your interest in contributing to Danny! This guide will help you get started.

## Development Setup

### Prerequisites

Before you begin, make sure you have the following installed:

- **Rust 1.83+** - [Install via rustup](https://rustup.rs)
- **Node.js 18+** - [Download from nodejs.org](https://nodejs.org)
- **just** - Command runner for development tasks
  - macOS: `brew install just`
  - Other platforms: [Install just](https://github.com/casey/just#installation)
- **gum** (optional, but recommended for better UI)
  - macOS: `brew install gum`
  - Other platforms: [Install gum](https://github.com/charmbracelet/gum#installation)

### Quick Start

1. **Clone the repository:**
   ```bash
   git clone https://github.com/nine-labs/danny
   cd danny
   ```

2. **Run the setup script:**
   ```bash
   just setup
   ```
   
   This will:
   - ✓ Check all required dependencies
   - ✓ Install Rust and Node.js dependencies
   - ✓ Build the entire workspace
   - ✓ Verify everything is working

3. **Try running a test fixture:**
   ```bash
   just fixtures
   ```
   
   This opens an interactive menu to run test fixtures.

### Common Development Commands

Run `just` or `just --list` to see all available commands. Here are the most common ones:

#### Building
- `just dev` - Build in development mode (faster, includes debug symbols)
- `just build` - Build in release mode (optimized)
- `just build-cli` - Build only the CLI (faster iteration)

#### Testing
- `just test` - Run all tests (Rust + Node.js)
- `just test-rust` - Run only Rust tests
- `just test-rust-verbose` - Run tests with full output
- `just fixtures` - Run test fixtures interactively
- `just fixture-all` - Run all fixtures sequentially

#### Code Quality
- `just fmt` - Format code
- `just lint` - Run clippy linter
- `just check` - Run formatting check, linting, and tests
- `just fmt-check` - Check formatting without modifying files

#### Development Workflow
- `just watch` - Watch and rebuild on changes (requires `cargo-watch`)
- `just watch-test` - Watch and run tests on changes

#### Other Useful Commands
- `just clean` - Clean all build artifacts
- `just stats` - Show project statistics
- `just docs` - Generate and open documentation
- `just install` - Install the CLI locally

## Project Structure

```
danny/
├── crates/                    # Rust workspace
│   ├── danny-cli/            # Command-line interface
│   ├── danny-core/           # Core analysis engine
│   ├── danny-backend-js/     # JavaScript/TypeScript backend
│   ├── danny-rule-engine/    # Rule engine for customization
│   ├── danny-config/         # Configuration management
│   └── danny-fs/             # File system utilities
├── packages/                  # Node.js packages
│   └── benchmark/            # Benchmarking suite
├── test-files/               # Test fixtures
└── justfile                  # Development commands
```

## Testing Your Changes

### Running Fixtures

Fixtures are test files that demonstrate various dead code patterns. Use them to verify your changes:

```bash
# Interactive menu
just fixtures

# Run specific fixture
just fixture-example1        # JavaScript examples
just fixture-example2        # TypeScript examples
just fixture-nextjs          # Full Next.js app

# Run with different options
just fixture-quality example1.js   # With quality analysis
just fixture-json example1.js      # JSON output
```

### Running Tests

```bash
# Run all tests
just test

# Run specific test suite
cargo test --package danny-backend-js
cargo test --package danny-core

# Run specific test
cargo test test_name

# Run tests with output
just test-rust-verbose
```

### Benchmarking

Compare Danny's performance against other tools:

```bash
just benchmark              # Run benchmarks
just benchmark-compare      # Compare with Knip
just benchmark-findings     # Compare detection accuracy
```

## Code Style

- Run `just fmt` before committing to format your code
- Run `just lint` to catch common issues
- We use `cargo clippy` with standard settings
- Keep source files under 500 lines when possible

## Making Changes

### Workflow

1. Create a new branch for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes and test them:
   ```bash
   just check    # Run all checks
   ```

3. Commit your changes with clear commit messages:
   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

4. Push and create a pull request:
   ```bash
   git push origin feature/your-feature-name
   ```

### Commit Messages

We follow conventional commits:

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `test:` - Test changes
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Build/tooling changes

## Adding New Features

### Adding a New Analysis Rule

1. Add your rule logic in `crates/danny-backend-js/src/analyzers/`
2. Update tests in the corresponding test file
3. Add test fixtures in `test-files/`
4. Update documentation

### Adding Support for a New Language

1. Create a new crate: `crates/danny-backend-{language}/`
2. Implement the `Backend` trait from `danny-core`
3. Add tests and fixtures
4. Register the backend in the CLI

## Getting Help

- **Questions?** Open a discussion on GitHub
- **Found a bug?** Open an issue with reproduction steps
- **Want to propose a feature?** Open an issue for discussion first

## License

By contributing to Danny, you agree that your contributions will be licensed under the MIT License.

