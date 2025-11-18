//! Built-in framework rules embedded in the binary
//!
//! This module provides the default framework detection rules for popular
//! frameworks like React, Next.js, Vue, and Svelte. These rules are embedded
//! at compile time via `include_str!()` for zero-config defaults.

use fob::graph::FrameworkRule;
use crate::{TomlFrameworkRule, Result, EntryPointPattern};

/// React framework rules (hooks, components)
pub const REACT_RULES: &str = include_str!("built_in/react.toml");

/// Next.js framework rules (data fetching, API routes, middleware)
pub const NEXTJS_RULES: &str = include_str!("built_in/nextjs.toml");

/// Vue framework rules (composables, SFC defaults)
pub const VUE_RULES: &str = include_str!("built_in/vue.toml");

/// Svelte framework rules (stores, reactive patterns)
pub const SVELTE_RULES: &str = include_str!("built_in/svelte.toml");

/// Load all built-in framework rules
///
/// This function loads the 4 embedded framework rule sets and returns them
/// as trait objects that can be passed to Fob's analysis pipeline.
///
/// # Example
///
/// ```no_run
/// use danny_rule_engine::load_built_in_rules;
///
/// let rules = load_built_in_rules().expect("Failed to load built-in rules");
/// assert_eq!(rules.len(), 4); // React, Next.js, Vue, Svelte
/// ```
pub fn load_built_in_rules() -> Result<Vec<Box<dyn FrameworkRule>>> {
    let frameworks = [
        ("React", REACT_RULES),
        ("Next.js", NEXTJS_RULES),
        ("Vue", VUE_RULES),
        ("Svelte", SVELTE_RULES),
    ];

    frameworks
        .iter()
        .map(|(name, toml_str)| {
            TomlFrameworkRule::from_toml_str((*name).to_string(), toml_str)
                .map(|rule| Box::new(rule) as Box<dyn FrameworkRule>)
        })
        .collect()
}

/// Load all built-in entry point patterns from TOML files
///
/// This function extracts entry point patterns from all embedded framework TOML files.
/// Entry points are used for file discovery BEFORE analysis to seed the dependency graph.
///
/// # Example
///
/// ```no_run
/// use danny_rule_engine::load_built_in_entry_points;
///
/// let entry_points = load_built_in_entry_points().expect("Failed to load entry points");
/// // Use entry_points to discover files using glob patterns
/// ```
pub fn load_built_in_entry_points() -> Result<Vec<EntryPointPattern>> {
    let frameworks = [
        REACT_RULES,
        NEXTJS_RULES,
        VUE_RULES,
        SVELTE_RULES,
    ];

    let mut all_entry_points = Vec::new();

    for toml_str in frameworks.iter() {
        match crate::extract_entry_points(toml_str) {
            Ok(mut entry_points) => {
                all_entry_points.append(&mut entry_points);
            }
            Err(e) => {
                // Log but don't fail - some TOML files may not have entry_points section
                eprintln!("Warning: Failed to extract entry points: {}", e);
            }
        }
    }

    // Sort by priority (higher priority first), then by name for stable ordering
    all_entry_points.sort_by(|a, b| {
        b.priority
            .unwrap_or(0)
            .cmp(&a.priority.unwrap_or(0))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(all_entry_points)
}

/// Load all built-in TOML files for framework detection
///
/// Returns a vector of (framework_name, toml_content) tuples that can be used
/// with FrameworkDetector::from_toml_files().
pub fn load_built_in_toml_files() -> Vec<(String, &'static str)> {
    vec![
        ("React".to_string(), REACT_RULES),
        ("Next.js".to_string(), NEXTJS_RULES),
        ("Vue".to_string(), VUE_RULES),
        ("Svelte".to_string(), SVELTE_RULES),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_built_in_rules() {
        let rules = load_built_in_rules().expect("Failed to load built-in rules");
        assert_eq!(rules.len(), 4, "Should load all 4 framework rules");

        let names: Vec<&str> = rules.iter().map(|r| r.name()).collect();
        assert!(names.contains(&"React"), "Should include React rules");
        assert!(names.contains(&"Next.js"), "Should include Next.js rules");
        assert!(names.contains(&"Vue"), "Should include Vue rules");
        assert!(names.contains(&"Svelte"), "Should include Svelte rules");
    }

    #[test]
    fn test_react_rules_parse() {
        let rule = TomlFrameworkRule::from_toml_str("React".to_string(), REACT_RULES)
            .expect("Failed to parse React rules");
        assert_eq!(rule.name(), "React");
        assert!(rule.description().contains("React"));
        assert!(rule.is_default());
    }

    #[test]
    fn test_nextjs_rules_parse() {
        let rule = TomlFrameworkRule::from_toml_str("Next.js".to_string(), NEXTJS_RULES)
            .expect("Failed to parse Next.js rules");
        assert_eq!(rule.name(), "Next.js");
    }

    #[test]
    fn test_vue_rules_parse() {
        let rule = TomlFrameworkRule::from_toml_str("Vue".to_string(), VUE_RULES)
            .expect("Failed to parse Vue rules");
        assert_eq!(rule.name(), "Vue");
    }

    #[test]
    fn test_svelte_rules_parse() {
        let rule = TomlFrameworkRule::from_toml_str("Svelte".to_string(), SVELTE_RULES)
            .expect("Failed to parse Svelte rules");
        assert_eq!(rule.name(), "Svelte");
    }

    #[test]
    fn test_all_rules_have_descriptions() {
        let rules = load_built_in_rules().expect("Failed to load rules");
        for rule in rules {
            assert!(
                !rule.description().is_empty(),
                "Rule {} should have a description",
                rule.name()
            );
        }
    }

    #[test]
    fn test_all_rules_are_default() {
        let rules = load_built_in_rules().expect("Failed to load rules");
        for rule in rules {
            assert!(
                rule.is_default(),
                "Built-in rule {} should be enabled by default",
                rule.name()
            );
        }
    }
}
