//! Language backend trait and registry.

use crate::error::Result;
use crate::types::{AnalysisOptions, AnalysisResult};
use std::fmt;

/// Trait for language-specific analysis backends.
///
/// Backends are responsible for:
/// - Loading language-specific configuration
/// - Performing module resolution and dependency analysis
/// - Detecting patterns and frameworks
/// - Converting results to Danny's common types
///
/// # Thread Safety
///
/// Implementations must be Send + Sync to allow parallel analysis.
///
/// # Examples
///
/// ```no_run
/// use danny_core::{LanguageBackend, AnalysisOptions};
///
/// fn analyze_with_backend(backend: &dyn LanguageBackend, options: AnalysisOptions) {
///     println!("Using {} backend", backend.name());
///
///     match backend.analyze(options) {
///         Ok(result) => println!("Found {} modules", result.statistics.total_modules),
///         Err(e) => eprintln!("Analysis failed: {}", e),
///     }
/// }
/// ```
pub trait LanguageBackend: Send + Sync + fmt::Debug {
    /// Returns the backend name (e.g., "JavaScript", "Python").
    fn name(&self) -> &str;

    /// Returns the file extensions this backend handles.
    ///
    /// Extensions should include the dot (e.g., ".js", ".ts").
    fn supported_extensions(&self) -> &[&str];

    /// Performs analysis on the given project.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration is invalid
    /// - Entry points don't exist
    /// - Analysis encounters a fatal error
    ///
    /// Non-fatal errors (e.g., parse errors in individual files) should be
    /// included in `AnalysisResult.errors` rather than failing the entire analysis.
    fn analyze(&self, options: AnalysisOptions) -> Result<AnalysisResult>;

    /// Validates that this backend can analyze the given project.
    ///
    /// This is called before `analyze()` to provide early feedback.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required tools are missing (e.g., Node.js not installed)
    /// - Project structure is invalid
    /// - Configuration is missing or malformed
    fn validate(&self, options: &AnalysisOptions) -> Result<()> {
        // Default implementation: check that entry points exist
        for entry in &options.entry_points {
            if !entry.exists() {
                return Err(crate::error::Error::EntryPointNotFound {
                    path: entry.clone(),
                });
            }
        }
        Ok(())
    }

    /// Returns the default configuration for this backend.
    ///
    /// Used when no configuration file is present.
    fn default_config(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

/// Registry for language backends.
///
/// Allows dynamic backend discovery and selection.
#[derive(Default)]
pub struct BackendRegistry {
    backends: Vec<Box<dyn LanguageBackend>>,
}

impl BackendRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a backend.
    pub fn register(&mut self, backend: Box<dyn LanguageBackend>) {
        self.backends.push(backend);
    }

    /// Finds a backend that supports the given file extension.
    ///
    /// Returns the first matching backend, or None if no backend supports the extension.
    pub fn find_by_extension(&self, extension: &str) -> Option<&dyn LanguageBackend> {
        self.backends.iter().find_map(|backend| {
            if backend.supported_extensions().contains(&extension) {
                Some(backend.as_ref())
            } else {
                None
            }
        })
    }

    /// Finds a backend by name.
    pub fn find_by_name(&self, name: &str) -> Option<&dyn LanguageBackend> {
        self.backends.iter().find_map(|backend| {
            if backend.name().eq_ignore_ascii_case(name) {
                Some(backend.as_ref())
            } else {
                None
            }
        })
    }

    /// Returns all registered backends.
    pub fn all(&self) -> &[Box<dyn LanguageBackend>] {
        &self.backends
    }
}

impl fmt::Debug for BackendRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackendRegistry")
            .field("backends", &self.backends.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Statistics;
    use std::path::PathBuf;

    // Mock backend for testing
    #[derive(Debug)]
    struct MockBackend {
        name: String,
        extensions: Vec<&'static str>,
    }

    impl LanguageBackend for MockBackend {
        fn name(&self) -> &str {
            &self.name
        }

        fn supported_extensions(&self) -> &[&str] {
            &self.extensions
        }

        fn analyze(&self, _options: AnalysisOptions) -> Result<AnalysisResult> {
            Ok(AnalysisResult {
                findings: vec![],
                statistics: Statistics::default(),
                errors: vec![],
                ignored_findings: vec![],
            })
        }
    }

    #[test]
    fn test_registry_find_by_extension() {
        let mut registry = BackendRegistry::new();

        registry.register(Box::new(MockBackend {
            name: "JavaScript".to_string(),
            extensions: vec![".js", ".ts"],
        }));

        registry.register(Box::new(MockBackend {
            name: "Python".to_string(),
            extensions: vec![".py"],
        }));

        assert!(registry.find_by_extension(".js").is_some());
        assert!(registry.find_by_extension(".ts").is_some());
        assert!(registry.find_by_extension(".py").is_some());
        assert!(registry.find_by_extension(".rs").is_none());
    }

    #[test]
    fn test_registry_find_by_name() {
        let mut registry = BackendRegistry::new();

        registry.register(Box::new(MockBackend {
            name: "JavaScript".to_string(),
            extensions: vec![".js"],
        }));

        assert!(registry.find_by_name("JavaScript").is_some());
        assert!(registry.find_by_name("javascript").is_some()); // Case insensitive
        assert!(registry.find_by_name("Python").is_none());
    }

    #[test]
    fn test_validate_missing_entry_point() {
        let backend = MockBackend {
            name: "Test".to_string(),
            extensions: vec![],
        };

        let options = AnalysisOptions {
            entry_points: vec![PathBuf::from("/non/existent/file.js")],
            ..Default::default()
        };

        let result = backend.validate(&options);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::Error::EntryPointNotFound { .. }
        ));
    }
}
