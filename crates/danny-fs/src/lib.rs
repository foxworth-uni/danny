//! Platform-agnostic filesystem abstraction for Danny.
//!
//! This crate provides a `FileSystem` trait that works on both native platforms
//! (using `std::fs`) and WASM (using in-memory storage).
//!
//! # Example
//!
//! ```no_run
//! use danny_fs::{FileSystem, NativeFileSystem};
//! use std::sync::Arc;
//! use std::path::Path;
//!
//! # #[tokio::main]
//! # async fn main() -> std::io::Result<()> {
//! let fs = Arc::new(NativeFileSystem::new(".")?);
//! let contents = fs.read_to_string(Path::new("README.md")).await?;
//! println!("{}", contents);
//! # Ok(())
//! # }
//! ```

mod file_system;
pub use file_system::{DiscoveryOptions, FileMetadata, FileSystem};

#[cfg(feature = "native")]
pub mod native;
#[cfg(feature = "native")]
pub use native::NativeFileSystem;

#[cfg(feature = "wasm")]
pub mod wasm;
#[cfg(feature = "wasm")]
pub use wasm::WasmFileSystem;

#[cfg(feature = "native")]
pub use NativeFileSystem as DefaultFileSystem;

#[cfg(all(not(feature = "native"), feature = "wasm"))]
pub use WasmFileSystem as DefaultFileSystem;
