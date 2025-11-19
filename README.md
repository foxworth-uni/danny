# Danny

Smart bundler analyzer for modern web applications.

## Installation

### Homebrew (macOS)

```bash
brew install foxworth-uni/danny
```

### From Source

```bash
cargo install --path crates/danny-cli
```

### Shell Script (macOS)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/foxworth-uni/danny/releases/latest/download/danny-installer.sh | sh
```

## Usage

```bash
danny --help
```

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```

### Running Locally

```bash
cargo run -p danny-cli -- [ARGS]
```

## Releasing

This project uses [cargo-dist](https://github.com/axodotdev/cargo-dist) for automated releases.

### Prerequisites

1. Create a GitHub Personal Access Token with `repo` permissions
2. Add it as a repository secret named `HOMEBREW_TAP_TOKEN`
3. Create the `foxworth-uni/homebrew-tap` repository on GitHub

### Release Process

1. Update the version in `Cargo.toml`:
   ```toml
   [workspace.package]
   version = "0.2.0"
   ```

2. Commit the version change:
   ```bash
   git add Cargo.toml Cargo.lock
   git commit -m "chore: bump version to 0.2.0"
   ```

3. Create and push a git tag:
   ```bash
   git tag v0.2.0
   git push origin main --tags
   ```

4. GitHub Actions will automatically:
   - Build binaries for macOS (Intel and ARM)
   - Create a GitHub Release with artifacts
   - Generate and publish the Homebrew formula to `foxworth-uni/homebrew-tap`
   - Create shell installers

### What Gets Released

- **macOS Intel** (`x86_64-apple-darwin`)
- **macOS ARM** (`aarch64-apple-darwin`)
- **Homebrew Formula** (published to `foxworth-uni/homebrew-tap`)
- **Shell Installer** (for quick installation without Homebrew)

### Updating Release Configuration

The release configuration is in `dist-workspace.toml`. To regenerate the GitHub Actions workflow after changes:

```bash
dist generate
```

## License

MIT

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
