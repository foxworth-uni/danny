//! Security and performance constants for the rule engine
//!
//! These constants define limits to prevent various attack vectors:
//! - ReDoS (Regular Expression Denial of Service)
//! - Memory exhaustion
//! - Path traversal
//! - Excessive recursion

/// Maximum size for TOML rule files (1MB)
///
/// Rationale: TOML files should be small configuration files.
/// Larger files may indicate malicious content or misconfiguration.
/// This prevents memory exhaustion when loading rules.
pub const MAX_TOML_FILE_SIZE: u64 = 1_048_576; // 1MB

/// Maximum file size for content pattern matching (10MB)
///
/// Rationale: Content pattern matching requires reading entire files
/// into memory. This limit prevents DoS attacks via extremely large files.
/// Most source files are well under this limit.
pub const MAX_CONTENT_SIZE: u64 = 10_485_760; // 10MB

/// Maximum regex pattern length (500 characters)
///
/// Rationale: Extremely long regex patterns are often a sign of
/// malicious input or poor design. This limit prevents ReDoS attacks
/// and keeps patterns maintainable.
pub const MAX_REGEX_LENGTH: usize = 500;

/// Compiled regex size limit (10MB)
///
/// Rationale: Limits memory usage of compiled regex patterns.
/// Prevents memory exhaustion from pathological patterns.
/// Applied during regex compilation via RegexBuilder.
pub const REGEX_SIZE_LIMIT: usize = 10_000_000; // 10MB

/// Regex DFA size limit (2MB)
///
/// Rationale: Limits the size of the deterministic finite automaton
/// used by the regex engine. Prevents excessive memory usage during
/// pattern matching operations.
pub const REGEX_DFA_SIZE_LIMIT: usize = 2_000_000; // 2MB

/// Maximum directory traversal depth (10 levels)
///
/// Rationale: Prevents stack overflow and excessive recursion when
/// loading rules from nested directories. Most real projects don't
/// need more than 10 levels of nesting.
pub const MAX_DIRECTORY_DEPTH: usize = 10;

