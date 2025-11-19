//! Cargo ecosystem support

pub mod lockfile;
pub mod parser;
pub mod workspace;

pub use parser::CargoDependencyManager;
