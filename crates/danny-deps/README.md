# danny-deps

Local dependency file parsing and management for Rust and JavaScript projects.

This crate provides functionality to:
- Parse dependency files (Cargo.toml, package.json)
- Parse lockfiles (Cargo.lock, package-lock.json, pnpm-lock.yaml, yarn.lock)
- Compare versions using semver and npm-style versioning
- Safely update dependency files while preserving formatting and comments
- Support monorepo/workspace scenarios (Cargo workspaces, pnpm/npm workspaces)
- Verify lockfile integrity (checksums)
- **Integration with danny-info** for unified dependency management

## Quick Start

### Parse Dependencies

```rust
use danny_deps::{CargoDependencyManager, DependencyManager};
use std::path::Path;

let manager = CargoDependencyManager::new();
let manifest = manager.parse(Path::new("Cargo.toml"))?;

for dep in manifest.all_dependencies() {
    println!("{}: {}", dep.name, dep.version_req.raw);
}
```

### Check for Updates (Unified API)

```rust
use danny_deps::{UnifiedDependencyManager, Ecosystem};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manager = UnifiedDependencyManager::new()?;
    let updates = manager.check_updates(Path::new("Cargo.toml"), Ecosystem::Rust).await?;

    for update in updates {
        println!("{}: {} -> {}", 
            update.package, 
            update.current_req.raw, 
            update.latest_version
        );
    }
    Ok(())
}
```

### Update Dependencies

```rust
use danny_deps::{CargoDependencyManager, DependencyManager, DependencyType, DependencyUpdate};

let manager = CargoDependencyManager::new();
let updates = vec![
    DependencyUpdate {
        package: "serde".to_string(),
        new_version: "1.0.210".to_string(),
        dep_type: DependencyType::Runtime,
    }
];

// Dry run first
let result = manager.update(Path::new("Cargo.toml"), &updates, true)?;

// Apply updates
let result = manager.update(Path::new("Cargo.toml"), &updates, false)?;
```

## Architecture

The crate follows a trait-based architecture:
- Core traits for extensibility (`DependencyManager`, `LockfileParser`)
- Ecosystem-specific implementations (Cargo, npm/pnpm/yarn)
- Integration with `danny-info` for fetching remote package data

## Supported Ecosystems

### Rust (Cargo)
- ✅ Cargo.toml parsing (preserves comments/formatting)
- ✅ Cargo.lock parsing
- ✅ Workspace detection
- ✅ Dependency updates

### JavaScript (npm/pnpm/yarn)
- ✅ package.json parsing
- ✅ package-lock.json parsing
- ✅ pnpm-lock.yaml parsing
- ✅ Workspace detection (npm, pnpm)
- ✅ Dependency updates

## Examples

See the `examples/` directory:
- `check_updates.rs` - Check for dependency updates using unified API
- `parse_and_update.rs` - Parse and update dependencies
- `workspace_demo.rs` - Workspace detection demo

Run examples with:
```bash
cargo run --package danny-deps --example check_updates
```

## Integration with danny-info

`danny-deps` integrates seamlessly with `danny-info`:

- **danny-info**: Fetches remote package data (registries, changelogs, releases)
- **danny-deps**: Parses local dependency files and manages updates
- **UnifiedDependencyManager**: Combines both for complete dependency management

```rust
use danny_deps::UnifiedDependencyManager;

let manager = UnifiedDependencyManager::new()?;
let recommendations = manager.check_updates("Cargo.toml", Ecosystem::Rust).await?;

// Each recommendation includes:
// - Current version requirement
// - Latest available version
// - Update type (major/minor/patch)
// - Changelog entries (if available)
// - GitHub releases (if available)
```

## Features

### Format Preservation
- Uses `toml_edit` (not `toml`) to preserve comments and formatting in Cargo.toml
- Pretty-prints JSON with standard 2-space indentation

### Safe Updates
- Atomic file writes (temp file → rename)
- Dry-run support
- Validation before applying changes

### Workspace Support
- Detects Cargo workspaces
- Detects npm/pnpm workspaces
- Finds workspace members automatically

### Version Comparison
- Semver for Rust
- npm-style (^, ~, *) for JavaScript
- Determines update type (major/minor/patch)

## Testing

Run all tests:
```bash
cargo test --package danny-deps
```

Run integration tests (requires network):
```bash
cargo test --package danny-deps --test integration -- --ignored
```

Run property-based tests:
```bash
cargo test --package danny-deps --features property-tests
```

## License

MIT OR Apache-2.0

