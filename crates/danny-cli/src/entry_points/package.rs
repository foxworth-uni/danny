use anyhow::{Context, Result};
use danny_config::Framework;
use danny_core::Error;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Extract entry points from package.json with security validation
pub fn extract_entry_points(package_json: &Path) -> Result<Vec<PathBuf>> {
    // TOCTOU fix: Open file immediately instead of checking existence first
    use std::fs::File;
    let _file = File::open(package_json)
        .context("Failed to open package.json")?;
    
    let content = std::fs::read_to_string(package_json)
        .context("Failed to read package.json")?;

    let package: serde_json::Value = serde_json::from_str(&content)
        .context("Failed to parse package.json")?;

    let root = package_json
        .parent()
        .ok_or_else(|| Error::InvalidPath {
            path: package_json.to_path_buf(),
            reason: "package.json has no parent directory".to_string(),
        })?;
    
    let mut entry_points = Vec::new();

    // Check "main" field with validation
    if let Some(main) = package.get("main").and_then(|v| v.as_str()) {
        if let Ok(path) = super::security::validate_entry_point(main, root) {
            entry_points.push(path);
        }
    }

    // Check "module" field with validation
    if let Some(module) = package.get("module").and_then(|v| v.as_str()) {
        if let Ok(path) = super::security::validate_entry_point(module, root) {
            if !entry_points.contains(&path) {
                entry_points.push(path);
            }
        }
    }

    // If no entry points found, try framework-specific detection
    if entry_points.is_empty() {
        entry_points = detect_framework_entry_points(root)?;
    }

    Ok(entry_points)
}

/// Detect framework from package.json dependencies
pub fn detect_framework(root: &Path) -> Result<Option<Framework>> {
    let package_json = root.join("package.json");

    // TOCTOU fix: Try to open file instead of checking existence
    use std::fs::File;
    match File::open(&package_json) {
        Ok(_) => {
            // File exists, proceed to read
        }
        Err(_) => {
            return Ok(None);
        }
    }

    let content = std::fs::read_to_string(&package_json)
        .context("Failed to read package.json")?;

    // Simple detection based on dependencies
    if content.contains("\"next\"") {
        Ok(Some(Framework::NextJs))
    } else if content.contains("\"@vue/") || content.contains("\"vue\"") {
        Ok(Some(Framework::Vue))
    } else if content.contains("\"svelte\"") {
        Ok(Some(Framework::Svelte))
    } else if content.contains("\"react\"") {
        Ok(Some(Framework::React))
    } else {
        Ok(None)
    }
}

/// Detect framework-specific entry points using TOML patterns
fn detect_framework_entry_points(root: &Path) -> Result<Vec<PathBuf>> {
    // Load entry point patterns from built-in TOML files
    let patterns = danny_rule_engine::load_built_in_entry_points()
        .context("Failed to load built-in entry point patterns")?;

    let mut discovered = HashSet::new();

    for pattern_group in patterns {
        for pattern_str in &pattern_group.patterns {
            // Expand brace patterns since glob crate doesn't support {a,b,c} syntax
            let expanded_patterns = expand_braces(pattern_str);

            for expanded in &expanded_patterns {
                // Convert pattern to absolute by joining with root
                // For patterns starting with **, remove the leading ** and join
                let pattern_for_glob = if expanded.starts_with("**/") {
                    format!("{}/{}", root.display(), &expanded[3..])
                } else if expanded.starts_with("**") {
                    format!("{}/{}", root.display(), &expanded[2..])
                } else {
                    format!("{}/{}", root.display(), expanded)
                };

                match glob::glob(&pattern_for_glob) {
                    Ok(paths) => {
                        for path_result in paths {
                            match path_result {
                                Ok(path) => {
                                    // Path is already absolute from our glob pattern
                                    if path.is_file() {
                                        match path.canonicalize() {
                                            Ok(canonical) => {
                                                // Security: Validate path is within project root
                                                if super::security::validate_path_within_root(&canonical, root).is_ok() {
                                                    discovered.insert(canonical);
                                                }
                                            }
                                            Err(_e) => {
                                                // Silently skip files that can't be canonicalized
                                            }
                                        }
                                    }
                                }
                                Err(_e) => {
                                    // Silently skip glob path errors
                                }
                            }
                        }
                    }
                    Err(_e) => {
                        // Silently skip invalid glob patterns
                    }
                }
            }
        }
    }

    let mut result: Vec<PathBuf> = discovered.into_iter().collect();
    result.sort();
    Ok(result)
}

/// Expand brace patterns like "file.{ts,tsx,js}" into ["file.ts", "file.tsx", "file.js"]
///
/// The `glob` crate doesn't support brace expansion, but our TOML patterns use it extensively.
/// This function manually expands {a,b,c} patterns before passing to glob.
///
/// Handles nested braces recursively: "file.{ts,js}.{map,bak}" → 4 combinations
fn expand_braces(pattern: &str) -> Vec<String> {
    // Find first brace pattern
    if let Some(start) = pattern.find('{') {
        if let Some(end) = pattern[start..].find('}') {
            let end = start + end;
            let prefix = &pattern[..start];
            let suffix = &pattern[end + 1..];
            let options = &pattern[start + 1..end];

            // Split by comma and expand
            let mut results = Vec::new();
            for option in options.split(',') {
                let expanded = format!("{}{}{}", prefix, option, suffix);
                // Recursively expand any remaining braces
                results.extend(expand_braces(&expanded));
            }
            return results;
        }
    }

    // No braces found, return as-is
    vec![pattern.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_braces_simple() {
        assert_eq!(
            expand_braces("file.{ts,js}"),
            vec!["file.ts", "file.js"]
        );
    }

    #[test]
    fn test_expand_braces_multiple_extensions() {
        let result = expand_braces("page.{ts,tsx,js,jsx}");
        assert_eq!(result, vec!["page.ts", "page.tsx", "page.js", "page.jsx"]);
    }

    #[test]
    fn test_expand_braces_with_path() {
        let result = expand_braces("**/app/**/page.{ts,tsx}");
        assert_eq!(result, vec!["**/app/**/page.ts", "**/app/**/page.tsx"]);
    }

    #[test]
    fn test_expand_braces_no_braces() {
        assert_eq!(expand_braces("file.ts"), vec!["file.ts"]);
    }

    #[test]
    fn test_expand_braces_nested() {
        let result = expand_braces("file.{ts,js}.{map,bak}");
        assert_eq!(
            result,
            vec!["file.ts.map", "file.ts.bak", "file.js.map", "file.js.bak"]
        );
    }

    #[test]
    fn test_expand_braces_complex() {
        let result = expand_braces("**/src/app/**/page.{ts,tsx,js,jsx}");
        assert_eq!(
            result,
            vec![
                "**/src/app/**/page.ts",
                "**/src/app/**/page.tsx",
                "**/src/app/**/page.js",
                "**/src/app/**/page.jsx"
            ]
        );
    }
}

