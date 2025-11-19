//! Changelog markdown parser with date extraction

use crate::types::{ChangelogEntry, ParsedChangelog};
use regex::Regex;

/// Parse a changelog markdown file into structured entries
///
/// Supports common formats:
/// - `## [1.2.3] - 2024-01-15` (Keep a Changelog)
/// - `## 1.2.3 (2024-01-15)` (Angular style)
/// - `# v1.2.3 / 2024-01-15` (Timestamp format)
/// - `## v1.2.3 - January 15, 2024` (Natural language)
pub fn parse_changelog(markdown: &str) -> ParsedChangelog {
    let mut entries = Vec::new();
    let mut current_entry: Option<(String, String, String, Option<String>)> = None;
    let mut preamble = String::new();
    let mut in_preamble = true;
    
    for line in markdown.lines() {
        // Check if this is a version heading
        if let Some((heading, version, date)) = parse_version_heading(line) {
            in_preamble = false;
            
            // Save previous entry if exists
            if let Some((h, v, content, d)) = current_entry.take() {
                entries.push(ChangelogEntry {
                    version: v,
                    date: d,
                    content: content.trim().to_string(),
                    heading: h,
                });
            }
            
            // Start new entry
            current_entry = Some((heading, version, String::new(), date));
        } else {
            // Append to current content
            if let Some((_, _, ref mut content, _)) = current_entry {
                content.push_str(line);
                content.push('\n');
            } else if in_preamble {
                preamble.push_str(line);
                preamble.push('\n');
            }
        }
    }
    
    // Save last entry
    if let Some((h, v, content, d)) = current_entry {
        entries.push(ChangelogEntry {
            version: v,
            date: d,
            content: content.trim().to_string(),
            heading: h,
        });
    }
    
    ParsedChangelog {
        entries,
        other_content: if preamble.trim().is_empty() {
            None
        } else {
            Some(preamble.trim().to_string())
        },
    }
}

/// Try to parse a version heading line
///
/// Returns: (original_heading, version, optional_date)
fn parse_version_heading(line: &str) -> Option<(String, String, Option<String>)> {
    let line = line.trim();
    
    // Skip if not a heading
    if !line.starts_with('#') {
        return None;
    }
    
    // Remove heading markers
    let content = line.trim_start_matches('#').trim();
    
    // Try various patterns
    let patterns = [
        // ## [1.2.3] - 2024-01-15 (Keep a Changelog)
        r"^\[([v]?[\d.]+(?:-[\w.]+)?)\]\s*-\s*(\d{4}-\d{2}-\d{2})",
        // ## 1.2.3 (2024-01-15) (Angular)
        r"^([v]?[\d.]+(?:-[\w.]+)?)\s*\((\d{4}-\d{2}-\d{2})\)",
        // ## v1.2.3 / 2024-01-15 (Timestamp)
        r"^([v]?[\d.]+(?:-[\w.]+)?)\s*/\s*(\d{4}-\d{2}-\d{2})",
        // ## [1.2.3] - January 15, 2024 (Natural language)
        r"^\[([v]?[\d.]+(?:-[\w.]+)?)\]\s*-\s*([A-Za-z]+\s+\d{1,2},\s+\d{4})",
        // ## 1.2.3 - Jan 15 2024
        r"^([v]?[\d.]+(?:-[\w.]+)?)\s*-\s*([A-Za-z]+\s+\d{1,2}\s+\d{4})",
    ];
    
    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(content) {
                let version = caps.get(1)?.as_str().trim_start_matches('v').to_string();
                let date = caps.get(2).map(|m| normalize_date(m.as_str()));
                return Some((line.to_string(), version, date));
            }
        }
    }
    
    // Version only patterns (no date)
    let version_only_patterns = [
        // ## [1.2.3]
        r"^\[([v]?[\d.]+(?:-[\w.]+)?)\]$",
        // ## 1.2.3
        r"^([v]?[\d.]+(?:-[\w.]+)?)$",
        // ## v1.2.3
        r"^v([\d.]+(?:-[\w.]+)?)$",
    ];
    
    for pattern in &version_only_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(content) {
                let version = caps.get(1)?.as_str().trim_start_matches('v').to_string();
                return Some((line.to_string(), version, None));
            }
        }
    }
    
    None
}

/// Normalize various date formats to ISO 8601 (YYYY-MM-DD)
fn normalize_date(date_str: &str) -> String {
    // If already in ISO format, return as-is
    if date_str.len() == 10 && date_str.chars().filter(|c| *c == '-').count() == 2 {
        return date_str.to_string();
    }
    
    // Try to parse natural language dates
    // For Phase 1, we'll keep it simple and just return the original
    // Phase 2 could add chrono for proper parsing
    date_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keep_a_changelog_format() {
        let md = r#"# Changelog

## [1.2.3] - 2024-01-15
### Added
- New feature

## [1.2.2] - 2024-01-10
### Fixed
- Bug fix
"#;
        
        let parsed = parse_changelog(md);
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].version, "1.2.3");
        assert_eq!(parsed.entries[0].date, Some("2024-01-15".to_string()));
        assert_eq!(parsed.entries[1].version, "1.2.2");
        assert_eq!(parsed.entries[1].date, Some("2024-01-10".to_string()));
    }
    
    #[test]
    fn test_angular_format() {
        let md = r#"## 1.2.3 (2024-01-15)
- Feature

## 1.2.2 (2024-01-10)
- Fix
"#;
        
        let parsed = parse_changelog(md);
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].version, "1.2.3");
        assert_eq!(parsed.entries[0].date, Some("2024-01-15".to_string()));
    }
    
    #[test]
    fn test_version_only() {
        let md = r#"## [1.2.3]
- Feature

## 1.2.2
- Fix
"#;
        
        let parsed = parse_changelog(md);
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].version, "1.2.3");
        assert_eq!(parsed.entries[0].date, None);
        assert_eq!(parsed.entries[1].version, "1.2.2");
        assert_eq!(parsed.entries[1].date, None);
    }
    
    #[test]
    fn test_timestamp_format() {
        let md = r#"# v1.2.3 / 2024-01-15
- Feature

# v1.2.2 / 2024-01-10
- Fix
"#;
        
        let parsed = parse_changelog(md);
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].version, "1.2.3");
        assert_eq!(parsed.entries[0].date, Some("2024-01-15".to_string()));
    }
    
    #[test]
    fn test_with_preamble() {
        let md = r#"# Changelog
All notable changes to this project will be documented in this file.

## [1.2.3] - 2024-01-15
- Feature
"#;
        
        let parsed = parse_changelog(md);
        assert_eq!(parsed.entries.len(), 1);
        assert!(parsed.other_content.is_some());
        assert!(parsed.other_content.unwrap().contains("notable changes"));
    }
    
    #[test]
    fn test_empty_changelog() {
        let md = "";
        let parsed = parse_changelog(md);
        assert_eq!(parsed.entries.len(), 0);
        assert_eq!(parsed.other_content, None);
    }
}

