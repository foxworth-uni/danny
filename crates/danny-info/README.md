# danny-info

Package metadata fetcher for npm, crates.io, JSR, and GitHub.

## Features

- Fetch package information from npm registry
- Fetch crate information from crates.io
- Fetch package information from JSR (JavaScript Registry)
- Fetch GitHub releases and changelogs
- **Parse changelogs by date and version** - Extract structured entries from CHANGELOG.md files
- Parse repository URLs from package metadata
- **Built-in rate limiting** to comply with registry requirements
- Simple, async API
- Type-safe error handling

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
danny-info = "0.1.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

### Basic Example

```rust
use danny_info::InfoClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = InfoClient::new()?;

    // Fetch npm package
    let react = client.fetch_npm("react").await?;
    println!("React v{}", react.version);

    // Fetch Rust crate
    let serde = client.fetch_crates_io("serde").await?;
    println!("Serde v{}", serde.version);

    // Fetch GitHub releases if repository is available
    if let Some(repo) = react.repository {
        let releases = client.fetch_releases(&repo).await?;
        println!("Latest release: {}", releases[0].tag_name);

        // Fetch changelog
        let changelog = client.fetch_changelog(&repo).await?;
        println!("Changelog: {} bytes", changelog.len());
    }

    Ok(())
}
```

### Supported Registries

#### npm

```rust
let info = client.fetch_npm("react").await?;
let scoped = client.fetch_npm("@types/node").await?;
```

#### crates.io

```rust
let info = client.fetch_crates_io("serde").await?;
```

#### JSR (JavaScript Registry)

```rust
let info = client.fetch_jsr("@std/path").await?;
```

### GitHub Integration

#### Releases (with dates)

```rust
// Get GitHub releases - already includes dates!
let releases = client.fetch_releases(&repo).await?;
for release in releases {
    println!("{} - {}", 
        release.tag_name, 
        release.published_at.unwrap_or_default()
    );
}
```

#### Changelog (raw markdown)

```rust
// Tries common filenames: CHANGELOG.md, CHANGES.md, etc.
let changelog = client.fetch_changelog(&repo).await?;
```

#### Parsed Changelog (structured with dates)

```rust
// Parse CHANGELOG.md into structured entries with version and date
let parsed = client.fetch_parsed_changelog(&repo).await?;

// Iterate over changelog entries
for entry in parsed.entries {
    println!("Version {} - Released: {}", 
        entry.version,
        entry.date.unwrap_or_else(|| "Unknown".to_string())
    );
    println!("{}\n", entry.content);
}

// Access preamble/header content
if let Some(preamble) = parsed.other_content {
    println!("Preamble: {}", preamble);
}
```

**Supported Changelog Formats:**
- Keep a Changelog: `## [1.2.3] - 2024-01-15`
- Angular style: `## 1.2.3 (2024-01-15)`
- Timestamp format: `# v1.2.3 / 2024-01-15`
- Version only: `## [1.2.3]` or `## 1.2.3`

### Repository URL Parsing

```rust
let repo = InfoClient::parse_repository_url("https://github.com/facebook/react")?;
println!("Owner: {}, Repo: {}", repo.owner, repo.repo);

// Supports various formats:
// - https://github.com/owner/repo
// - https://github.com/owner/repo.git
// - git+https://github.com/owner/repo.git
// - git@github.com:owner/repo.git
```

## Rate Limiting

**danny-info** respects rate limits to comply with registry requirements:

### Default Behavior

By default, `InfoClient::new()` enables rate limiting:

- **npm**: 1 request/second (conservative, npm allows ~5M requests/month)
- **crates.io**: 1 request/second (**required by crates.io**)
- **JSR**: 1 request/second (conservative)
- **GitHub**: 60 requests/hour (unauthenticated), 5000 requests/hour (authenticated with `GITHUB_TOKEN`)

### Customization

```rust
// With rate limiting (recommended, default)
let client = InfoClient::new()?;

// Without rate limiting (use with caution!)
// Note: crates.io will block requests exceeding 1 req/sec
let client = InfoClient::without_rate_limiting()?;
```

### Registry-Specific Requirements

#### crates.io
- **Mandatory**: Maximum 1 request per second
- **User-Agent**: Must include identifying information (automatically set by fob-info)
- Responds with HTTP 429 when rate limit exceeded

#### npm
- ~5 million requests per month considered acceptable
- Responds with HTTP 429 when rate limit exceeded
- Higher rates allowed for authenticated users

#### JSR
- No official public rate limits documented
- Publishing: 1000 attempts per 7 days
- Conservative 1 req/sec used by default

#### GitHub
- **Unauthenticated**: 60 requests/hour
- **Authenticated**: 5,000 requests/hour
- Set `GITHUB_TOKEN` environment variable for higher limits

## Environment Variables

- `GITHUB_TOKEN` - Optional GitHub personal access token for higher API rate limits (5000/hour vs 60/hour)

## Testing

Run unit tests (no network required):

```bash
cargo test --package danny-info
```

Run integration tests (requires network):

```bash
cargo test --package danny-info -- --ignored
```

Run examples:

```bash
# Basic usage example
cargo run --package danny-info --example basic

# Changelog parsing example
cargo run --package danny-info --example changelog_parser
```

## License

MIT
