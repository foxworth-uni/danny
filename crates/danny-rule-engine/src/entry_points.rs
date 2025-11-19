//! Entry point pattern extraction from TOML files
//!
//! This module extracts entry point patterns from TOML rule files.
//! The actual file discovery using glob patterns is handled by the CLI.

use crate::{Result, RuleError, TomlRuleFile};

/// Extract entry point patterns from a TOML rule file
///
/// Returns a vector of entry point patterns sorted by priority (higher first).
pub fn extract_entry_points(
    toml_content: &str,
) -> Result<Vec<crate::toml_rule::EntryPointPattern>> {
    let file: TomlRuleFile = toml::from_str(toml_content).map_err(RuleError::TomlError)?;

    let mut entry_points = file.entry_points;

    // Sort by priority (higher priority first), then by name for stable ordering
    entry_points.sort_by(|a, b| {
        b.priority
            .unwrap_or(0)
            .cmp(&a.priority.unwrap_or(0))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(entry_points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_entry_points() {
        let toml = r#"
            [[entry_points]]
            name = "app-router-pages"
            patterns = ["**/app/**/page.{ts,tsx,js,jsx}"]
            priority = 100

            [[entry_points]]
            name = "pages-router"
            patterns = ["pages/**/*.{ts,tsx,js,jsx}"]
            priority = 90
        "#;

        let entry_points = extract_entry_points(toml).unwrap();
        assert_eq!(entry_points.len(), 2);
        assert_eq!(entry_points[0].name, "app-router-pages");
        assert_eq!(entry_points[0].priority, Some(100));
        assert_eq!(entry_points[1].name, "pages-router");
        assert_eq!(entry_points[1].priority, Some(90));
    }

    #[test]
    fn test_extract_empty_entry_points() {
        let toml = r#"
            [[rules]]
            name = "test-rule"
            [rules.match]
            export_name = ["default"]
            [rules.action]
            mark_used = true
        "#;

        let entry_points = extract_entry_points(toml).unwrap();
        assert_eq!(entry_points.len(), 0);
    }
}
