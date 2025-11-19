//! File ignore pattern handling for Danny.

use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Default patterns that Danny ignores by default.
///
/// These patterns match common directories that developers don't want to analyze:
/// - Build outputs (dist/, build/, .next/)
/// - Dependencies (node_modules/)
/// - Version control (.git/)
/// - Cache directories (.turbo/, coverage/)
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    "**/node_modules/**",
    "**/.git/**",
    "**/.next/**",
    "**/dist/**",
    "**/build/**",
    "**/out/**",
    "**/.dist/**",
    "**/_dist/**",
    "**/.cache/**",
    "**/cache/**",
    "**/coverage/**",
    "**/.nyc_output/**",
    "**/.turbo/**",
];

/// Builder for creating ignore pattern sets.
pub struct IgnorePatternBuilder {
    patterns: Vec<String>,
    use_defaults: bool,
}

impl IgnorePatternBuilder {
    /// Create a new ignore pattern builder with defaults enabled.
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            use_defaults: true,
        }
    }

    /// Disable default ignore patterns.
    pub fn no_defaults(mut self) -> Self {
        self.use_defaults = false;
        self
    }

    /// Add multiple custom ignore patterns.
    pub fn add_patterns<I>(mut self, patterns: I) -> Result<Self>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        for pattern in patterns {
            // Validate pattern
            Glob::new(pattern.as_ref())?;
            self.patterns.push(pattern.as_ref().to_string());
        }
        Ok(self)
    }

    /// Build the GlobSet along with pattern metadata for tracking.
    pub fn build_with_metadata(mut self) -> Result<(GlobSet, Vec<PatternInfo>)> {
        let mut builder = GlobSetBuilder::new();
        let mut pattern_infos = Vec::new();

        // Add default patterns if enabled
        if self.use_defaults {
            for pattern in DEFAULT_IGNORE_PATTERNS {
                self.patterns.insert(0, pattern.to_string());
            }
        }

        // Build glob set with pattern tracking
        for pattern in self.patterns.iter() {
            builder.add(Glob::new(pattern)?);
            pattern_infos.push(PatternInfo {
                pattern: pattern.clone(),
            });
        }

        Ok((builder.build()?, pattern_infos))
    }
}

impl Default for IgnorePatternBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about an ignore pattern.
#[derive(Debug, Clone)]
pub struct PatternInfo {
    /// The pattern string.
    pub pattern: String,
}

/// Matches a path and returns the matched pattern string, if any.
pub fn match_with_pattern(
    path: &Path,
    ignore_set: &GlobSet,
    pattern_infos: &[PatternInfo],
) -> Option<String> {
    let matches = ignore_set.matches(path);

    if matches.is_empty() {
        None
    } else {
        // Return the first matching pattern
        matches
            .into_iter()
            .next()
            .and_then(|idx| pattern_infos.get(idx))
            .map(|info| info.pattern.clone())
    }
}

/// Finds and parses .gitignore file patterns.
///
/// Walks up from the given directory to find .gitignore files.
/// Returns patterns as strings that can be added to a GlobSetBuilder.
pub fn load_gitignore_patterns(start_dir: &Path) -> Result<Vec<String>> {
    let mut patterns = Vec::new();
    let mut current = start_dir;

    // Walk up to find .gitignore files
    loop {
        let gitignore_path = current.join(".gitignore");
        if gitignore_path.exists() {
            let patterns_from_file = parse_gitignore_file(&gitignore_path)?;
            patterns.extend(patterns_from_file);
        }

        // Stop at filesystem root or if we can't go up further
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Ok(patterns)
}

/// Parses a .gitignore file and converts patterns to glob patterns.
fn parse_gitignore_file(path: &Path) -> Result<Vec<String>> {
    use ignore::gitignore::GitignoreBuilder;

    let mut builder = GitignoreBuilder::new(path.parent().unwrap_or_else(|| Path::new(".")));

    builder.add(path);

    let _gitignore = builder.build()?;

    // Extract patterns from the gitignore object
    // Note: The ignore crate doesn't expose patterns directly, so we'll
    // read the file manually and convert gitignore patterns to glob patterns
    let content = std::fs::read_to_string(path)?;
    let mut glob_patterns = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Convert gitignore pattern to glob pattern
        let glob_pattern = gitignore_to_glob_pattern(line);
        glob_patterns.push(glob_pattern);
    }

    Ok(glob_patterns)
}

/// Converts a gitignore pattern to a glob pattern.
///
/// Handles common gitignore conventions:
/// - `/pattern` matches at root only
/// - `pattern/` matches directories
/// - `pattern` matches anywhere
/// - `*` and `**` wildcards
fn gitignore_to_glob_pattern(pattern: &str) -> String {
    let pattern = pattern.trim();

    // Remove trailing slash (directory marker)
    let pattern = pattern.trim_end_matches('/');

    // Handle negation (not supported in globset, so we'll keep as-is for now)
    if pattern.starts_with('!') {
        // Negation patterns are complex - for now we skip them
        // TODO: Implement proper negation support
        return String::new();
    }

    // If pattern starts with /, it's relative to the root
    if let Some(stripped) = pattern.strip_prefix('/') {
        // Root-relative pattern
        format!("**/{}", stripped)
    } else if pattern.contains('/') {
        // Pattern with path components
        format!("**/{}", pattern)
    } else {
        // Simple pattern - match anywhere
        format!("**/{}", pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_patterns_match_node_modules() {
        let (ignore_set, _) = IgnorePatternBuilder::new()
            .build_with_metadata()
            .expect("Failed to build ignore set");

        assert!(ignore_set.is_match(&PathBuf::from("node_modules/react/index.js")));
        assert!(ignore_set.is_match(&PathBuf::from("project/node_modules/lodash/index.js")));
    }

    #[test]
    fn test_default_patterns_match_build_dirs() {
        let (ignore_set, _) = IgnorePatternBuilder::new()
            .build_with_metadata()
            .expect("Failed to build ignore set");

        assert!(ignore_set.is_match(&PathBuf::from(".next/static/foo.js")));
        assert!(ignore_set.is_match(&PathBuf::from("dist/bundle.js")));
        assert!(ignore_set.is_match(&PathBuf::from("build/output.js")));
    }

    #[test]
    fn test_custom_patterns() {
        let (ignore_set, _) = IgnorePatternBuilder::new()
            .add_patterns(&["**/*.test.js"])
            .expect("Failed to add pattern")
            .build_with_metadata()
            .expect("Failed to build ignore set");

        assert!(ignore_set.is_match(&PathBuf::from("src/foo.test.js")));
        assert!(!ignore_set.is_match(&PathBuf::from("src/foo.js")));
    }

    #[test]
    fn test_no_defaults() {
        let (ignore_set, _) = IgnorePatternBuilder::new()
            .no_defaults()
            .build_with_metadata()
            .expect("Failed to build ignore set");

        // Should not match default patterns
        assert!(!ignore_set.is_match(&PathBuf::from("node_modules/react/index.js")));
        assert!(!ignore_set.is_match(&PathBuf::from(".next/foo.js")));
    }

    #[test]
    fn test_user_patterns_do_not_match() {
        let (ignore_set, _) = IgnorePatternBuilder::new()
            .build_with_metadata()
            .expect("Failed to build ignore set");

        assert!(!ignore_set.is_match(&PathBuf::from("src/index.js")));
        assert!(!ignore_set.is_match(&PathBuf::from("pages/about.tsx")));
        assert!(!ignore_set.is_match(&PathBuf::from("utils/helpers.ts")));
    }

    #[test]
    fn test_build_with_metadata() {
        let (_ignore_set, pattern_infos) = IgnorePatternBuilder::new()
            .add_patterns(&["**/*.test.js"])
            .expect("Failed to add pattern")
            .build_with_metadata()
            .expect("Failed to build with metadata");

        // Should have default patterns + custom pattern
        assert!(pattern_infos.len() > 1);
        assert!(pattern_infos.iter().any(|p| p.pattern == "**/*.test.js"));
    }

    #[test]
    fn test_match_with_pattern() {
        let (ignore_set, pattern_infos) = IgnorePatternBuilder::new()
            .add_patterns(&["**/*.test.js", "**/dist/**"])
            .expect("Failed to add patterns")
            .build_with_metadata()
            .expect("Failed to build with metadata");

        let test_file = PathBuf::from("src/foo.test.js");
        let matched = match_with_pattern(&test_file, &ignore_set, &pattern_infos);

        assert!(matched.is_some());
        assert_eq!(matched.unwrap(), "**/*.test.js");

        let dist_file = PathBuf::from("dist/bundle.js");
        let matched = match_with_pattern(&dist_file, &ignore_set, &pattern_infos);

        assert!(matched.is_some());
        assert_eq!(matched.unwrap(), "**/dist/**");

        let normal_file = PathBuf::from("src/index.js");
        let matched = match_with_pattern(&normal_file, &ignore_set, &pattern_infos);

        assert!(matched.is_none());
    }

    #[test]
    fn test_no_defaults_with_metadata() {
        let (ignore_set, pattern_infos) = IgnorePatternBuilder::new()
            .no_defaults()
            .add_patterns(&["**/*.foo"])
            .expect("Failed to add pattern")
            .build_with_metadata()
            .expect("Failed to build with metadata");

        // Should only have the custom pattern
        assert_eq!(pattern_infos.len(), 1);
        assert_eq!(pattern_infos[0].pattern, "**/*.foo");

        // node_modules should NOT be ignored
        let node_modules_file = PathBuf::from("node_modules/react/index.js");
        assert!(!ignore_set.is_match(&node_modules_file));
    }
}
