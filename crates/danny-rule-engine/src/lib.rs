//! Danny Rule Engine - TOML-based framework and custom rules
//!
//! This crate provides a flexible, declarative rule system for marking exports
//! as framework-used without requiring Rust code or recompilation.
//!
//! # Architecture
//!
//! - **TOML Rules** (95%): Declarative pattern matching for framework detection
//! - **Compiled Matching**: Regex patterns compiled once and cached
//! - **Multi-Source Loading**: Built-in rules + user rules + project rules
//!
//! # Example
//!
//! ```toml
//! # rules/react.toml
//! [[rules]]
//! name = "react-hooks"
//! [rules.match]
//! import_from = "react"
//! export_pattern = "^use[A-Z]\\w+"
//! export_type = "function"
//! [rules.action]
//! mark_used = true
//! reason = "React hook pattern"
//! ```

pub mod bridge;
pub mod built_in;
pub mod constants;
pub mod detection;
pub mod engine;
pub mod entry_points;
pub mod loader;
pub mod matcher;
pub mod toml_rule;

// Re-export core types
pub use bridge::TomlFrameworkRule;
pub use built_in::{load_built_in_entry_points, load_built_in_rules};
pub use constants::*;
pub use detection::{DetectionEvidence, DetectionResult, FrameworkDetector};
pub use engine::RuleEngine;
pub use entry_points::extract_entry_points;
pub use loader::RuleLoader;
pub use matcher::CompiledMatcher;
pub use toml_rule::{
    DetectionRule, DetectionType, EntryPointPattern, ExportType, FrameworkMetadata, RuleAction,
    RuleMatcher, Severity, TomlRule, TomlRuleFile,
};

/// Result type for rule operations
pub type Result<T> = std::result::Result<T, RuleError>;

/// Error types for rule engine
#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),

    #[error("Failed to load rules from {path}: {source}")]
    LoadError {
        path: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Invalid TOML: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}
