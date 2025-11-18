//! Compiled rule matchers with regex caching for performance
//!
//! This module compiles TOML rule matchers into fast, executable forms.
//! Regex patterns are compiled once and cached to avoid runtime overhead.

use crate::constants::{
    MAX_CONTENT_SIZE, MAX_REGEX_LENGTH, REGEX_DFA_SIZE_LIMIT, REGEX_SIZE_LIMIT,
};
use crate::{RuleError, RuleMatcher, Result};
use fob::graph::{Export, Module};
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;

/// Compile a regex with size limits to prevent ReDoS attacks
///
/// This function adds defensive limits to regex compilation:
/// - Pattern length limit (500 chars)
/// - Compiled regex size limit (10MB)
/// - DFA size limit (2MB)
///
/// These limits prevent malicious patterns from causing excessive memory
/// usage or CPU consumption.
fn compile_regex_safe(pattern: &str) -> Result<Regex> {
    if pattern.len() > MAX_REGEX_LENGTH {
        return Err(RuleError::InvalidPattern(format!(
            "Pattern exceeds maximum length of {} characters",
            MAX_REGEX_LENGTH
        )));
    }

    RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_DFA_SIZE_LIMIT)
        .build()
        .map_err(|e| RuleError::InvalidPattern(e.to_string()))
}

/// A compiled matcher ready for fast execution
///
/// Note: Regex doesn't implement Clone, so we store patterns and recompile on clone
#[derive(Debug)]
pub struct CompiledMatcher {
    /// Pre-compiled regex for export name matching
    export_regex: Option<Regex>,
    export_regex_pattern: Option<String>,

    /// Pre-compiled regex for path matching
    path_regex: Option<Regex>,
    path_regex_pattern: Option<String>,

    /// Fast hash set for exact export name matching
    export_names: Option<HashSet<String>>,

    /// Required import sources (any of these)
    import_from: Option<Vec<String>>,

    /// Pre-compiled regex for import source pattern matching
    import_from_regex: Option<Regex>,
    import_from_regex_pattern: Option<String>,

    /// Required import specifiers (named imports)
    import_specifiers: Option<HashSet<String>>,

    /// Must import default export
    import_default: Option<bool>,

    /// Must use namespace import
    import_namespace: Option<bool>,

    /// Required export type
    export_type: Option<crate::ExportType>,

    /// Path must start with one of these
    path_starts_with: Option<Vec<String>>,

    /// Path must end with one of these
    path_ends_with: Option<Vec<String>>,

    // Negation matchers
    /// Pre-compiled regex for NOT export name matching
    not_export_regex: Option<Regex>,
    not_export_regex_pattern: Option<String>,

    /// Fast hash set for NOT export name matching
    not_export_names: Option<HashSet<String>>,

    /// Pre-compiled regex for NOT path matching
    not_path_regex: Option<Regex>,
    not_path_regex_pattern: Option<String>,

    /// Must NOT import from these sources
    not_import_from: Option<Vec<String>>,

    /// Pre-compiled regex for file content matching
    content_regex: Option<Regex>,
    content_regex_pattern: Option<String>,

    /// Minimum usage count threshold
    min_usage_count: Option<usize>,

    /// Maximum usage count threshold
    max_usage_count: Option<usize>,
}

impl Clone for CompiledMatcher {
    fn clone(&self) -> Self {
        Self {
            export_regex: self
                .export_regex_pattern
                .as_ref()
                .and_then(|p| compile_regex_safe(p).ok()),
            export_regex_pattern: self.export_regex_pattern.clone(),
            path_regex: self
                .path_regex_pattern
                .as_ref()
                .and_then(|p| compile_regex_safe(p).ok()),
            path_regex_pattern: self.path_regex_pattern.clone(),
            export_names: self.export_names.clone(),
            import_from: self.import_from.clone(),
            import_from_regex: self
                .import_from_regex_pattern
                .as_ref()
                .and_then(|p| compile_regex_safe(p).ok()),
            import_from_regex_pattern: self.import_from_regex_pattern.clone(),
            import_specifiers: self.import_specifiers.clone(),
            import_default: self.import_default,
            import_namespace: self.import_namespace,
            export_type: self.export_type,
            path_starts_with: self.path_starts_with.clone(),
            path_ends_with: self.path_ends_with.clone(),
            not_export_regex: self
                .not_export_regex_pattern
                .as_ref()
                .and_then(|p| compile_regex_safe(p).ok()),
            not_export_regex_pattern: self.not_export_regex_pattern.clone(),
            not_export_names: self.not_export_names.clone(),
            not_path_regex: self
                .not_path_regex_pattern
                .as_ref()
                .and_then(|p| compile_regex_safe(p).ok()),
            not_path_regex_pattern: self.not_path_regex_pattern.clone(),
            not_import_from: self.not_import_from.clone(),
            content_regex: self
                .content_regex_pattern
                .as_ref()
                .and_then(|p| compile_regex_safe(p).ok()),
            content_regex_pattern: self.content_regex_pattern.clone(),
            min_usage_count: self.min_usage_count,
            max_usage_count: self.max_usage_count,
        }
    }
}

impl CompiledMatcher {
    /// Compile a RuleMatcher into an efficient executable form
    pub fn from_toml(matcher: &RuleMatcher) -> Result<Self> {
        // Validate usage count range (min must be <= max)
        if let (Some(min), Some(max)) = (matcher.min_usage_count, matcher.max_usage_count) {
            if min > max {
                return Err(RuleError::InvalidPattern(format!(
                    "Invalid usage count range: min ({}) > max ({})",
                    min, max
                )));
            }
        }

        // Compile export pattern regex if present
        let export_regex = matcher
            .export_pattern
            .as_ref()
            .map(|pattern| compile_regex_safe(pattern))
            .transpose()?;

        // Compile path pattern regex if present
        let path_regex = matcher
            .path_pattern
            .as_ref()
            .map(|pattern| compile_regex_safe(pattern))
            .transpose()?;

        // Compile import_from pattern regex if present
        let import_from_regex = matcher
            .import_from_pattern
            .as_ref()
            .map(|pattern| compile_regex_safe(pattern))
            .transpose()?;

        // Compile NOT export pattern regex if present
        let not_export_regex = matcher
            .not_export_pattern
            .as_ref()
            .map(|pattern| compile_regex_safe(pattern))
            .transpose()?;

        // Compile NOT path pattern regex if present
        let not_path_regex = matcher
            .not_path_pattern
            .as_ref()
            .map(|pattern| compile_regex_safe(pattern))
            .transpose()?;

        // Convert export names to HashSet for O(1) lookup
        let export_names = matcher
            .export_name
            .as_ref()
            .map(|names| names.iter().cloned().collect());

        // Convert NOT export names to HashSet for O(1) lookup
        let not_export_names = matcher
            .not_export_name
            .as_ref()
            .map(|names| names.iter().cloned().collect());

        // Convert import specifiers to HashSet for O(1) lookup
        let import_specifiers = matcher
            .import_specifiers
            .as_ref()
            .map(|specs| specs.iter().cloned().collect());

        // Compile content pattern regex if present
        let content_regex = matcher
            .content_pattern
            .as_ref()
            .map(|pattern| compile_regex_safe(pattern))
            .transpose()?;

        Ok(Self {
            export_regex,
            export_regex_pattern: matcher.export_pattern.clone(),
            path_regex,
            path_regex_pattern: matcher.path_pattern.clone(),
            export_names,
            import_from: matcher.import_from.clone(),
            import_from_regex,
            import_from_regex_pattern: matcher.import_from_pattern.clone(),
            import_specifiers,
            import_default: matcher.import_default,
            import_namespace: matcher.import_namespace,
            export_type: matcher.export_type,
            path_starts_with: matcher.path_starts_with.clone(),
            path_ends_with: matcher.path_ends_with.clone(),
            not_export_regex,
            not_export_regex_pattern: matcher.not_export_pattern.clone(),
            not_export_names,
            not_path_regex,
            not_path_regex_pattern: matcher.not_path_pattern.clone(),
            not_import_from: matcher.not_import_from.clone(),
            content_regex,
            content_regex_pattern: matcher.content_pattern.clone(),
            min_usage_count: matcher.min_usage_count,
            max_usage_count: matcher.max_usage_count,
        })
    }

    /// Check if a module and export match this rule
    ///
    /// Returns `true` if ALL conditions match (AND logic).
    /// Short-circuits on first non-match for performance.
    /// Negation checks are performed after positive checks.
    pub fn matches(&self, module: &Module, export: &Export) -> bool {
        self.check_import_conditions(module)
            && self.check_export_conditions(export)
            && self.check_path_conditions(module)
            && self.check_content_conditions(module)
            && self.check_negation_conditions(module, export)
            && self.check_usage_count_conditions(export)
    }

    /// Check all import-related conditions
    fn check_import_conditions(&self, module: &Module) -> bool {
        // Check import_from (requires module inspection)
        // Support multiple sources: match if ANY source matches
        if let Some(ref required_imports) = self.import_from {
            if !required_imports
                .iter()
                .any(|source| module_imports_from(module, source))
            {
                return false;
            }
        }

        // Check import_from_pattern (regex matching)
        if let Some(ref regex) = self.import_from_regex {
            if !module
                .imports
                .iter()
                .any(|import| regex.is_match(&import.source))
            {
                return false;
            }
        }

        // Check import specifiers, default, and namespace
        // We need to check if ANY import matches the source requirements AND specifier requirements
        if (self.import_specifiers.is_some()
            || self.import_default.is_some()
            || self.import_namespace.is_some())
            && !self.check_import_specifier_conditions(module)
        {
            return false;
        }

        true
    }

    /// Check if any import matches source requirements AND specifier requirements
    fn check_import_specifier_conditions(&self, module: &Module) -> bool {
        module.imports.iter().any(|import| {
            // Check if source matches import_from or import_from_pattern
            let source_match = if let Some(ref required_imports) = self.import_from {
                required_imports.contains(&import.source)
            } else if let Some(ref regex) = self.import_from_regex {
                regex.is_match(&import.source)
            } else {
                true // No source requirement
            };

            if !source_match {
                return false;
            }

            // Check specifiers
            self.check_required_specifiers(import)
        })
    }

    /// Check if an import matches all required specifier conditions
    fn check_required_specifiers(&self, import: &fob::graph::Import) -> bool {
        // Check named specifiers
        if let Some(ref required_specs) = self.import_specifiers {
            let import_spec_names: HashSet<String> = import
                .specifiers
                .iter()
                .map(|spec| match spec {
                    fob::graph::ImportSpecifier::Named(name) => name.clone(),
                    fob::graph::ImportSpecifier::Default => "default".to_string(),
                    fob::graph::ImportSpecifier::Namespace(name) => name.clone(),
                })
                .collect();
            if !required_specs.iter().all(|req| import_spec_names.contains(req)) {
                return false;
            }
        }

        // Check default import
        if let Some(requires_default) = self.import_default {
            let has_default = import
                .specifiers
                .iter()
                .any(|s| matches!(s, fob::graph::ImportSpecifier::Default));
            if requires_default != has_default {
                return false;
            }
        }

        // Check namespace import
        if let Some(requires_namespace) = self.import_namespace {
            let is_namespace = import
                .specifiers
                .iter()
                .any(|s| matches!(s, fob::graph::ImportSpecifier::Namespace(_)));
            if requires_namespace != is_namespace {
                return false;
            }
        }

        true
    }

    /// Check all export-related conditions
    fn check_export_conditions(&self, export: &Export) -> bool {
        // Check export name (hash lookup: O(1))
        if let Some(ref names) = self.export_names {
            if !names.contains(&export.name) {
                return false;
            }
        }

        // Check export pattern (compiled regex)
        if let Some(ref regex) = self.export_regex {
            if !regex.is_match(&export.name) {
                return false;
            }
        }

        // Check export type
        if let Some(ref typ) = self.export_type {
            if !typ.matches_fob_export(export) {
                return false;
            }
        }

        true
    }

    /// Check all path-related conditions
    fn check_path_conditions(&self, module: &Module) -> bool {
        let path_str = module.path.to_string_lossy();

        if let Some(ref prefixes) = self.path_starts_with {
            if !prefixes.iter().any(|prefix| path_str.starts_with(prefix)) {
                return false;
            }
        }

        if let Some(ref suffixes) = self.path_ends_with {
            if !suffixes.iter().any(|suffix| path_str.ends_with(suffix)) {
                return false;
            }
        }

        if let Some(ref regex) = self.path_regex {
            if !regex.is_match(&path_str) {
                return false;
            }
        }

        true
    }

    /// Check file content patterns (expensive I/O operation)
    ///
    /// Note: Content regex matching is not supported in WASM builds
    /// due to filesystem limitations. On WASM, this check is skipped.
    fn check_content_conditions(&self, module: &Module) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref regex) = self.content_regex {
                // Check file size before reading to prevent DoS via large files
                if let Ok(metadata) = std::fs::metadata(&module.path) {
                    if metadata.len() > MAX_CONTENT_SIZE {
                        // File too large - skip content pattern check (fail the rule)
                        // This prevents memory exhaustion attacks
                        return false;
                    }

                    // Read file content and check pattern
                    // Note: This is relatively expensive, so we do it after other checks
                    if let Ok(content) = std::fs::read_to_string(&module.path) {
                        if !regex.is_match(&content) {
                            return false;
                        }
                    } else {
                        // File can't be read - fail the rule (conservative approach)
                        return false;
                    }
                } else {
                    // Can't get metadata - fail the rule
                    return false;
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // On WASM, content regex is not supported (requires FileSystem integration)
            // For now, we skip the check (treat as pass)
            // TODO: Add async FileSystem-based content checking
            let _ = module; // Suppress unused warning
        }

        true
    }

    /// Check all negation conditions
    fn check_negation_conditions(&self, module: &Module, export: &Export) -> bool {
        let path_str = module.path.to_string_lossy();

        // Check NOT import_from
        if let Some(ref not_imports) = self.not_import_from {
            if not_imports
                .iter()
                .any(|source| module_imports_from(module, source))
            {
                return false;
            }
        }

        // Check NOT export name
        if let Some(ref not_names) = self.not_export_names {
            if not_names.contains(&export.name) {
                return false;
            }
        }

        // Check NOT export pattern
        if let Some(ref regex) = self.not_export_regex {
            if regex.is_match(&export.name) {
                return false;
            }
        }

        // Check NOT path pattern
        if let Some(ref regex) = self.not_path_regex {
            if regex.is_match(&path_str) {
                return false;
            }
        }

        true
    }

    /// Check usage count thresholds
    fn check_usage_count_conditions(&self, export: &Export) -> bool {
        // Usage count matching uses a conservative approach:
        // - If usage_count is None (not computed), we reject the match to avoid false positives
        // - If usage_count is Some(n), we check against min/max thresholds
        if self.min_usage_count.is_some() || self.max_usage_count.is_some() {
            match export.usage_count {
                None => {
                    // Usage count not available - fail conservatively
                    // This prevents false positives when usage data hasn't been computed
                    return false;
                }
                Some(count) => {
                    // Check minimum usage count
                    if let Some(min) = self.min_usage_count {
                        if count < min {
                            return false;
                        }
                    }

                    // Check maximum usage count
                    if let Some(max) = self.max_usage_count {
                        if count > max {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    /// Check if this matcher only applies to files (no export checking needed)
    pub fn is_file_only(&self) -> bool {
        // If no export-specific conditions, it's file-only
        // Usage count conditions are export-specific, so they make it export-level
        self.export_regex.is_none()
            && self.export_names.is_none()
            && self.export_type.is_none()
            && self.not_export_regex.is_none()
            && self.not_export_names.is_none()
            && self.min_usage_count.is_none()
            && self.max_usage_count.is_none()
    }
}

/// Check if a module imports from a specific source
fn module_imports_from(module: &Module, source: &str) -> bool {
    module
        .imports
        .iter()
        .any(|import| import.source == source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fob::graph::{ExportKind, Import, ImportKind, ModuleId, SourceSpan, SourceType};
    use std::path::PathBuf;

    fn create_test_module(path: &str, imports: Vec<&str>) -> Module {
        let path_buf = PathBuf::from(path);
        let module_id = ModuleId::new(&path_buf).unwrap();

        let imports_vec = imports
            .into_iter()
            .map(|source| Import {
                source: source.to_string(),
                specifiers: vec![],
                kind: ImportKind::Static,
                resolved_to: None,
                span: SourceSpan {
                    file: path_buf.clone(),
                    start: 0,
                    end: 0,
                },
            })
            .collect();

        Module::builder(module_id, path_buf, SourceType::JavaScript)
            .imports(imports_vec)
            .build()
    }

    fn create_test_export(name: &str) -> Export {
        Export {
            name: name.to_string(),
            kind: ExportKind::Named,
            span: SourceSpan {
                file: PathBuf::new(),
                start: 0,
                end: 0,
            },
            is_type_only: false,
            is_framework_used: false,
            is_used: false,
            re_exported_from: None,
            came_from_commonjs: false,
            usage_count: None,
        }
    }

    fn create_test_export_with_usage(name: &str, usage_count: usize) -> Export {
        Export {
            name: name.to_string(),
            kind: ExportKind::Named,
            span: SourceSpan {
                file: PathBuf::new(),
                start: 0,
                end: 0,
            },
            is_type_only: false,
            is_framework_used: false,
            is_used: false,
            re_exported_from: None,
            came_from_commonjs: false,
            usage_count: Some(usage_count),
        }
    }

    #[test]
    fn test_export_pattern_matching() {
        let matcher = RuleMatcher {
            export_pattern: Some("^use[A-Z]\\w+".to_string()),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        assert!(compiled.matches(&module, &create_test_export("useState")));
        assert!(compiled.matches(&module, &create_test_export("useEffect")));
        assert!(!compiled.matches(&module, &create_test_export("useeffect"))); // lowercase
        assert!(!compiled.matches(&module, &create_test_export("Component")));
    }

    #[test]
    fn test_export_name_matching() {
        let matcher = RuleMatcher {
            export_name: Some(vec![
                "getStaticProps".to_string(),
                "getServerSideProps".to_string(),
            ]),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        assert!(compiled.matches(&module, &create_test_export("getStaticProps")));
        assert!(compiled.matches(&module, &create_test_export("getServerSideProps")));
        assert!(!compiled.matches(&module, &create_test_export("getStaticPaths")));
    }

    #[test]
    fn test_import_from_matching() {
        let matcher = RuleMatcher {
            import_from: Some(vec!["react".to_string()]),
            export_pattern: Some("^use[A-Z]\\w+".to_string()),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        let react_module = create_test_module("component.tsx", vec!["react"]);
        let plain_module = create_test_module("utils.ts", vec![]);

        // Matches: imports react AND name matches pattern
        assert!(compiled.matches(&react_module, &create_test_export("useState")));

        // Doesn't match: no react import
        assert!(!compiled.matches(&plain_module, &create_test_export("useState")));
    }

    #[test]
    fn test_path_pattern_matching() {
        let matcher = RuleMatcher {
            path_starts_with: Some(vec!["pages/".to_string()]),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        let page_module = create_test_module("pages/index.tsx", vec![]);
        let component_module = create_test_module("components/Button.tsx", vec![]);

        assert!(compiled.matches(&page_module, &create_test_export("default")));
        assert!(!compiled.matches(&component_module, &create_test_export("default")));
    }

    #[test]
    fn test_combined_conditions() {
        // Matches: imports react AND hook pattern AND in components/
        let matcher = RuleMatcher {
            import_from: Some(vec!["react".to_string()]),
            export_pattern: Some("^use[A-Z]\\w+".to_string()),
            path_starts_with: Some(vec!["components/".to_string()]),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        let matching_module = create_test_module("components/hooks.ts", vec!["react"]);
        let wrong_path = create_test_module("utils/hooks.ts", vec!["react"]);
        let no_import = create_test_module("components/hooks.ts", vec![]);

        // All conditions met
        assert!(compiled.matches(&matching_module, &create_test_export("useState")));

        // Wrong path
        assert!(!compiled.matches(&wrong_path, &create_test_export("useState")));

        // No react import
        assert!(!compiled.matches(&no_import, &create_test_export("useState")));
    }

    #[test]
    fn test_usage_count_min_threshold() {
        let matcher = RuleMatcher {
            min_usage_count: Some(5),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        // Should match: usage count >= min
        assert!(compiled.matches(
            &module,
            &create_test_export_with_usage("export1", 5)
        ));
        assert!(compiled.matches(
            &module,
            &create_test_export_with_usage("export2", 10)
        ));

        // Should not match: usage count < min
        assert!(!compiled.matches(
            &module,
            &create_test_export_with_usage("export3", 4)
        ));
        assert!(!compiled.matches(
            &module,
            &create_test_export_with_usage("export4", 0)
        ));
    }

    #[test]
    fn test_usage_count_max_threshold() {
        let matcher = RuleMatcher {
            max_usage_count: Some(3),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        // Should match: usage count <= max
        assert!(compiled.matches(
            &module,
            &create_test_export_with_usage("export1", 3)
        ));
        assert!(compiled.matches(
            &module,
            &create_test_export_with_usage("export2", 0)
        ));

        // Should not match: usage count > max
        assert!(!compiled.matches(
            &module,
            &create_test_export_with_usage("export3", 4)
        ));
        assert!(!compiled.matches(
            &module,
            &create_test_export_with_usage("export4", 10)
        ));
    }

    #[test]
    fn test_usage_count_range() {
        let matcher = RuleMatcher {
            min_usage_count: Some(2),
            max_usage_count: Some(5),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        // Should match: usage count within range [2, 5]
        assert!(compiled.matches(
            &module,
            &create_test_export_with_usage("export1", 2)
        ));
        assert!(compiled.matches(
            &module,
            &create_test_export_with_usage("export2", 3)
        ));
        assert!(compiled.matches(
            &module,
            &create_test_export_with_usage("export3", 5)
        ));

        // Should not match: usage count outside range
        assert!(!compiled.matches(
            &module,
            &create_test_export_with_usage("export4", 1)
        ));
        assert!(!compiled.matches(
            &module,
            &create_test_export_with_usage("export5", 6)
        ));
    }

    #[test]
    fn test_usage_count_zero() {
        let matcher = RuleMatcher {
            max_usage_count: Some(0),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        // Should match: unused exports (count = 0)
        assert!(compiled.matches(
            &module,
            &create_test_export_with_usage("unused1", 0)
        ));

        // Should not match: any usage
        assert!(!compiled.matches(
            &module,
            &create_test_export_with_usage("used1", 1)
        ));
    }

    #[test]
    fn test_usage_count_conservative_none() {
        let matcher = RuleMatcher {
            min_usage_count: Some(1),
            ..Default::default()
        };

        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        // Should not match: None usage_count fails conservatively
        // This prevents false positives when usage data hasn't been computed
        assert!(!compiled.matches(&module, &create_test_export("export1")));

        // Also test with max_usage_count
        let matcher_max = RuleMatcher {
            max_usage_count: Some(5),
            ..Default::default()
        };
        let compiled_max = CompiledMatcher::from_toml(&matcher_max).unwrap();
        assert!(!compiled_max.matches(&module, &create_test_export("export2")));
    }

    #[test]
    fn test_usage_count_invalid_range() {
        let matcher = RuleMatcher {
            min_usage_count: Some(10),
            max_usage_count: Some(5), // Invalid: min > max
            ..Default::default()
        };

        // Should reject invalid range
        let result = CompiledMatcher::from_toml(&matcher);
        assert!(result.is_err());
        if let Err(RuleError::InvalidPattern(msg)) = result {
            assert!(msg.contains("Invalid usage count range"));
            assert!(msg.contains("min (10) > max (5)"));
        } else {
            panic!("Expected InvalidPattern error");
        }
    }

    // ============================================================================
    // Phase 3A: Feature Tests for New Matchers
    // ============================================================================

    // 3.1 Negation Matcher Tests
    #[test]
    fn test_not_export_pattern() {
        let matcher = RuleMatcher {
            not_export_pattern: Some("^_.*".to_string()),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        // Should NOT match exports starting with underscore
        assert!(!compiled.matches(&module, &create_test_export("_private")));
        assert!(!compiled.matches(&module, &create_test_export("_internal")));

        // Should match exports not starting with underscore
        assert!(compiled.matches(&module, &create_test_export("public")));
        assert!(compiled.matches(&module, &create_test_export("exported")));
    }

    #[test]
    fn test_not_export_name() {
        let matcher = RuleMatcher {
            not_export_name: Some(vec!["private".to_string(), "internal".to_string()]),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("test.ts", vec![]);

        // Should NOT match excluded names
        assert!(!compiled.matches(&module, &create_test_export("private")));
        assert!(!compiled.matches(&module, &create_test_export("internal")));

        // Should match other names
        assert!(compiled.matches(&module, &create_test_export("public")));
        assert!(compiled.matches(&module, &create_test_export("exported")));
    }

    #[test]
    fn test_not_path_pattern() {
        let matcher = RuleMatcher {
            not_path_pattern: Some(".*\\.test\\..*".to_string()),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        // Should NOT match test files
        assert!(!compiled.matches(
            &create_test_module("src/utils.test.ts", vec![]),
            &create_test_export("test")
        ));
        assert!(!compiled.matches(
            &create_test_module("test/unit.test.ts", vec![]),
            &create_test_export("test")
        ));

        // Should match non-test files
        assert!(compiled.matches(
            &create_test_module("src/utils.ts", vec![]),
            &create_test_export("test")
        ));
    }

    #[test]
    fn test_not_import_from() {
        let matcher = RuleMatcher {
            not_import_from: Some(vec!["react".to_string()]),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        // Should NOT match modules importing from react
        assert!(!compiled.matches(
            &create_test_module("component.tsx", vec!["react"]),
            &create_test_export("test")
        ));

        // Should match modules not importing from react
        assert!(compiled.matches(
            &create_test_module("utils.ts", vec![]),
            &create_test_export("test")
        ));
        assert!(compiled.matches(
            &create_test_module("lib.ts", vec!["lodash"]),
            &create_test_export("test")
        ));
    }

    #[test]
    fn test_combined_positive_and_negative() {
        // Matches: hook pattern AND NOT starting with underscore
        let matcher = RuleMatcher {
            export_pattern: Some("^use[A-Z]\\w+".to_string()),
            not_export_pattern: Some("^_.*".to_string()),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module("hooks.ts", vec![]);

        // Should match: hook pattern and not private
        assert!(compiled.matches(&module, &create_test_export("useState")));
        assert!(compiled.matches(&module, &create_test_export("useEffect")));

        // Should NOT match: private hook
        assert!(!compiled.matches(&module, &create_test_export("_useInternal")));

        // Should NOT match: not a hook
        assert!(!compiled.matches(&module, &create_test_export("Component")));
    }

    // 3.2 Import Specifier Tests
    fn create_module_with_specifiers(
        path: &str,
        source: &str,
        specifiers: Vec<fob::graph::ImportSpecifier>,
    ) -> Module {
        let path_buf = PathBuf::from(path);
        let module_id = ModuleId::new(&path_buf).unwrap();

        let imports_vec = vec![Import {
            source: source.to_string(),
            specifiers,
            kind: ImportKind::Static,
            resolved_to: None,
            span: SourceSpan {
                file: path_buf.clone(),
                start: 0,
                end: 0,
            },
        }];

        Module::builder(module_id, path_buf, SourceType::JavaScript)
            .imports(imports_vec)
            .build()
    }

    #[test]
    fn test_import_specifiers_named() {
        let matcher = RuleMatcher {
            import_from: Some(vec!["react".to_string()]),
            import_specifiers: Some(vec!["useState".to_string(), "useEffect".to_string()]),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        // Should match: imports both required specifiers
        let module_with_both = create_module_with_specifiers(
            "component.tsx",
            "react",
            vec![
                fob::graph::ImportSpecifier::Named("useState".to_string()),
                fob::graph::ImportSpecifier::Named("useEffect".to_string()),
            ],
        );
        assert!(compiled.matches(&module_with_both, &create_test_export("test")));

        // Should NOT match: missing one specifier
        let module_with_one = create_module_with_specifiers(
            "component.tsx",
            "react",
            vec![fob::graph::ImportSpecifier::Named("useState".to_string())],
        );
        assert!(!compiled.matches(&module_with_one, &create_test_export("test")));

        // Should NOT match: wrong source
        let module_wrong_source = create_module_with_specifiers(
            "component.tsx",
            "vue",
            vec![
                fob::graph::ImportSpecifier::Named("useState".to_string()),
                fob::graph::ImportSpecifier::Named("useEffect".to_string()),
            ],
        );
        assert!(!compiled.matches(&module_wrong_source, &create_test_export("test")));
    }

    #[test]
    fn test_import_default() {
        let matcher = RuleMatcher {
            import_from: Some(vec!["react".to_string()]),
            import_default: Some(true),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        // Should match: has default import
        let module_with_default = create_module_with_specifiers(
            "component.tsx",
            "react",
            vec![fob::graph::ImportSpecifier::Default],
        );
        assert!(compiled.matches(&module_with_default, &create_test_export("test")));

        // Should NOT match: no default import
        let module_no_default = create_module_with_specifiers(
            "component.tsx",
            "react",
            vec![fob::graph::ImportSpecifier::Named("useState".to_string())],
        );
        assert!(!compiled.matches(&module_no_default, &create_test_export("test")));
    }

    #[test]
    fn test_import_namespace() {
        let matcher = RuleMatcher {
            import_from: Some(vec!["react".to_string()]),
            import_namespace: Some(true),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        // Should match: has namespace import
        let module_with_namespace = create_module_with_specifiers(
            "component.tsx",
            "react",
            vec![fob::graph::ImportSpecifier::Namespace("React".to_string())],
        );
        assert!(compiled.matches(&module_with_namespace, &create_test_export("test")));

        // Should NOT match: no namespace import
        let module_no_namespace = create_module_with_specifiers(
            "component.tsx",
            "react",
            vec![fob::graph::ImportSpecifier::Named("useState".to_string())],
        );
        assert!(!compiled.matches(&module_no_namespace, &create_test_export("test")));
    }

    // 3.3 Content Pattern Tests
    #[test]
    fn test_content_pattern_matching() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.ts");

        // Create file with @public tag
        let content = "// @public\nexport const test = 123;";
        std::fs::write(&file_path, content).unwrap();

        let matcher = RuleMatcher {
            content_pattern: Some("@public".to_string()),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
        let module = create_test_module(file_path.to_str().unwrap(), vec![]);
        let export = create_test_export("test");

        // Should match: file contains @public tag
        assert!(compiled.matches(&module, &export));

        // Test without tag
        let content_no_tag = "export const test = 123;";
        std::fs::write(&file_path, content_no_tag).unwrap();
        assert!(!compiled.matches(&module, &export));
    }

    // 3.4 Multiple Import Sources
    #[test]
    fn test_multiple_import_sources_or_logic() {
        // Should match if importing from ANY of the sources
        let matcher = RuleMatcher {
            import_from: Some(vec!["react".to_string(), "preact".to_string()]),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        // Should match: imports from react
        assert!(compiled.matches(
            &create_test_module("component.tsx", vec!["react"]),
            &create_test_export("test")
        ));

        // Should match: imports from preact
        assert!(compiled.matches(
            &create_test_module("component.tsx", vec!["preact"]),
            &create_test_export("test")
        ));

        // Should NOT match: imports from neither
        assert!(!compiled.matches(
            &create_test_module("utils.ts", vec![]),
            &create_test_export("test")
        ));

        // Should NOT match: imports from different source
        assert!(!compiled.matches(
            &create_test_module("component.tsx", vec!["vue"]),
            &create_test_export("test")
        ));
    }

    // 3.5 Import Pattern Matching
    #[test]
    fn test_import_from_pattern() {
        // Test scoped package matching: @scope/*
        let matcher = RuleMatcher {
            import_from_pattern: Some("@scope/.*".to_string()),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        // Should match: scoped packages
        assert!(compiled.matches(
            &create_test_module("component.tsx", vec!["@scope/package"]),
            &create_test_export("test")
        ));
        assert!(compiled.matches(
            &create_test_module("component.tsx", vec!["@scope/another"]),
            &create_test_export("test")
        ));

        // Should NOT match: non-scoped packages
        assert!(!compiled.matches(
            &create_test_module("component.tsx", vec!["react"]),
            &create_test_export("test")
        ));
    }

    #[test]
    fn test_import_from_pattern_with_specifiers() {
        // Combined matching: pattern AND specifiers
        let matcher = RuleMatcher {
            import_from_pattern: Some("@scope/.*".to_string()),
            import_specifiers: Some(vec!["Component".to_string()]),
            ..Default::default()
        };
        let compiled = CompiledMatcher::from_toml(&matcher).unwrap();

        // Should match: scoped package with required specifier
        let module_with_spec = create_module_with_specifiers(
            "component.tsx",
            "@scope/ui",
            vec![fob::graph::ImportSpecifier::Named("Component".to_string())],
        );
        assert!(compiled.matches(&module_with_spec, &create_test_export("test")));

        // Should NOT match: wrong specifier
        let module_wrong_spec = create_module_with_specifiers(
            "component.tsx",
            "@scope/ui",
            vec![fob::graph::ImportSpecifier::Named("Button".to_string())],
        );
        assert!(!compiled.matches(&module_wrong_spec, &create_test_export("test")));

        // Should NOT match: wrong source pattern
        let module_wrong_source = create_module_with_specifiers(
            "component.tsx",
            "react",
            vec![fob::graph::ImportSpecifier::Named("Component".to_string())],
        );
        assert!(!compiled.matches(&module_wrong_source, &create_test_export("test")));
    }
}
