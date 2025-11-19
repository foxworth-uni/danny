# Changelog Parsing Feature (Phase 1)

## Overview

Added structured changelog parsing with date extraction to `danny-info`. This feature allows you to parse CHANGELOG.md files into structured entries with version numbers, dates, and content.

## What Was Added

### 1. New Types (`src/types.rs`)

- **`ChangelogEntry`**: Represents a single changelog entry with:
  - `version`: Version string (e.g., "1.2.3")
  - `date`: Optional release date (ISO 8601 format)
  - `content`: Markdown content for this version
  - `heading`: Original heading line

- **`ParsedChangelog`**: Container for parsed changelog with:
  - `entries`: Vector of changelog entries (newest first)
  - `other_content`: Optional preamble/header content

### 2. Changelog Parser (`src/changelog_parser.rs`)

New module that parses markdown changelog files into structured data.

**Supported Formats:**
- Keep a Changelog: `## [1.2.3] - 2024-01-15`
- Angular style: `## 1.2.3 (2024-01-15)`
- Timestamp format: `# v1.2.3 / 2024-01-15`
- Natural language: `## [1.2.3] - January 15, 2024`
- Version only: `## [1.2.3]` or `## 1.2.3`

**Features:**
- Extracts version numbers (strips 'v' prefix)
- Parses dates in ISO format (YYYY-MM-DD)
- Preserves natural language dates as-is
- Captures markdown content for each version
- Preserves preamble/header content

### 3. New API Method (`src/lib.rs`)

```rust
pub async fn fetch_parsed_changelog(&self, repo: &RepositoryUrl) -> Result<ParsedChangelog>
```

Fetches and parses a changelog with date extraction. More useful than `fetch_changelog()` if you need to work with specific versions or time ranges.

### 4. Updated Dependencies (`Cargo.toml`)

Added `regex = "1.10"` for pattern matching in changelog parsing.

### 5. Documentation & Examples

- Updated README with changelog parsing examples
- Created new example: `examples/changelog_parser.rs`
- Updated all crate references from `fob-info` to `danny-info`

## Usage Examples

### Basic Parsing

```rust
use danny_info::{InfoClient, RepositoryUrl};

let client = InfoClient::new()?;
let repo = RepositoryUrl::new("facebook", "react", "https://github.com/facebook/react");

// Parse CHANGELOG.md
let parsed = client.fetch_parsed_changelog(&repo).await?;

for entry in parsed.entries {
    println!("Version {} - {}", 
        entry.version,
        entry.date.unwrap_or_else(|| "No date".to_string())
    );
}
```

### Filtering by Date

```rust
// Find entries from 2024
let entries_2024: Vec<_> = parsed.entries.iter()
    .filter(|e| e.date.as_ref().map_or(false, |d| d.starts_with("2024")))
    .collect();
```

### Accessing Content

```rust
for entry in parsed.entries.iter().take(5) {
    println!("Version {}", entry.version);
    println!("Heading: {}", entry.heading);
    println!("Content:\n{}", entry.content);
}
```

## Two Data Sources

Now you can get version/changelog data from two sources:

1. **GitHub Releases** (via `fetch_releases()`) - Already has dates in `published_at` field
2. **CHANGELOG.md** (via `fetch_parsed_changelog()`) - Parses dates from markdown headings

## Tests

Added 6 comprehensive tests in `src/changelog_parser.rs`:
- `test_keep_a_changelog_format` - Keep a Changelog format
- `test_angular_format` - Angular style dates
- `test_version_only` - Versions without dates
- `test_timestamp_format` - Timestamp format with /
- `test_with_preamble` - Changelog with preamble
- `test_empty_changelog` - Empty input handling

All tests pass ✓

## Running Examples

```bash
# Basic usage example
cargo run --package danny-info --example basic

# Changelog parsing example  
cargo run --package danny-info --example changelog_parser
```

## Future Enhancements (Phase 2+)

Potential future improvements:
- Git integration via `git2` crate for tag dates
- Conventional commits parsing
- Better natural language date parsing with `chrono`
- Semantic versioning comparisons
- Changelog generation from git history

## Files Modified

- `src/types.rs` - Added new types
- `src/changelog_parser.rs` - New parser module
- `src/lib.rs` - Added new API method, updated docs
- `Cargo.toml` - Added regex dependency
- `README.md` - Updated with new examples
- `examples/basic.rs` - Fixed crate name
- `examples/changelog_parser.rs` - New example
- `tests/integration.rs` - Fixed crate name

