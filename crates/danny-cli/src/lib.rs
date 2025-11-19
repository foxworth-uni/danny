//! Danny CLI library components.
//!
//! This crate provides the command-line interface for Danny's analysis tools.
//! The main binary is in `main.rs`.

// Module declarations
pub mod commands;
pub mod display;
pub mod entry_points;
pub mod formatters;
pub mod ignore;

// Re-export core types for convenience
pub use danny_core::{AnalysisOptions, AnalysisResult, Finding};
