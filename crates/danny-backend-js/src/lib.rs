//! JavaScript/TypeScript analysis backend using Fob.
//!
//! This crate provides a [`LanguageBackend`] implementation that delegates
//! JavaScript and TypeScript analysis to Fob's graph infrastructure.
//!
//! # Example
//!
//! ```no_run
//! use danny_backend_js::JsBackend;
//! use danny_core::{AnalysisOptions, LanguageBackend};
//! use std::path::PathBuf;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let backend = JsBackend::new()?;
//!
//! let options = AnalysisOptions {
//!     entry_points: vec![PathBuf::from("src/index.ts")],
//!     project_root: PathBuf::from("."),
//!     ..Default::default()
//! };
//!
//! let result = backend.analyze(options)?;
//! println!("Analyzed {} modules", result.statistics.total_modules);
//! # Ok(())
//! # }
//! ```

pub mod analyzers;
pub mod backend;
pub mod file_discovery;
pub mod toml_config;

pub use backend::JsBackend;
pub use toml_config::DannyConfig;
