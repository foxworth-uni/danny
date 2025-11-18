//! Framework detection engine
//!
//! This module provides TOML-driven framework detection, replacing hardcoded
//! detection logic with declarative rules.

use crate::{Result, RuleError, TomlRuleFile, DetectionRule, DetectionType};
use regex::Regex;
use globset::{Glob, GlobMatcher};
use std::collections::HashMap;
use std::path::Path;

/// Maximum regex pattern length (to prevent ReDoS attacks)
const MAX_REGEX_LEN: usize = 500;

/// Maximum DFA size for regex compilation (2MB) - protects against ReDoS
const MAX_DFA_SIZE: usize = 2 * 1024 * 1024;

/// Compiled detection rule with cached regex/glob matchers
#[derive(Debug, Clone)]
pub struct CompiledDetectionRule {
    /// Original rule
    pub rule: DetectionRule,
    /// Compiled regex (for import/export patterns)
    pub regex: Option<Regex>,
    /// Compiled glob matcher (for file paths)
    pub glob: Option<GlobMatcher>,
}

impl CompiledDetectionRule {
    /// Compile a detection rule with security limits
    pub fn compile(rule: DetectionRule) -> Result<Self> {
        // Validate pattern length
        if rule.pattern.len() > MAX_REGEX_LEN {
            return Err(RuleError::InvalidPattern(format!(
                "Pattern too long (max {} chars): {}",
                MAX_REGEX_LEN,
                rule.pattern.len()
            )));
        }

        let (regex, glob) = match rule.rule_type {
            DetectionType::Import | DetectionType::ExportPattern => {
                // Compile regex with DFA size limits to prevent ReDoS
                use regex::RegexBuilder;
                let regex = RegexBuilder::new(&rule.pattern)
                    .dfa_size_limit(MAX_DFA_SIZE)
                    .build()
                    .map_err(|e| RuleError::RegexError(e))?;

                (Some(regex), None)
            }
            DetectionType::FilePath => {
                // Compile glob pattern
                let glob = Glob::new(&rule.pattern)
                    .map_err(|e| RuleError::InvalidPattern(format!("Invalid glob pattern: {}", e)))?
                    .compile_matcher();
                (None, Some(glob))
            }
            DetectionType::FileExtension => {
                // File extension matching (simple string comparison)
                (None, None)
            }
            DetectionType::PackageDependency | DetectionType::PackageScript => {
                // Package.json matching (simple string comparison)
                (None, None)
            }
        };

        Ok(Self {
            rule,
            regex,
            glob,
        })
    }
}

/// Evidence for why a framework was detected
#[derive(Debug, Clone)]
pub struct DetectionEvidence {
    /// Framework name
    pub framework: String,
    /// Detection rule that matched
    pub rule: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Additional context
    pub context: Option<String>,
}

/// Detection result for a single framework
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// Framework name
    pub framework: String,
    /// Total confidence score
    pub confidence: f32,
    /// List of evidence
    pub evidence: Vec<DetectionEvidence>,
}

/// Framework detector using TOML-based rules
pub struct FrameworkDetector {
    /// Compiled detection rules by framework name
    frameworks: HashMap<String, Vec<CompiledDetectionRule>>,
    /// Framework priorities (higher = evaluated first)
    priorities: HashMap<String, u32>,
    /// Framework suppression rules (framework -> suppressed frameworks)
    suppresses: HashMap<String, Vec<String>>,
}

impl FrameworkDetector {
    /// Create a new detector from TOML rule files
    pub fn from_toml_files(toml_files: Vec<(String, &str)>) -> Result<Self> {
        let mut frameworks = HashMap::new();
        let mut priorities = HashMap::new();
        let mut suppresses = HashMap::new();

        for (_name, toml_content) in toml_files {
            let file: TomlRuleFile = toml::from_str(toml_content)
                .map_err(|e| RuleError::TomlError(e))?;

            if let Some(metadata) = file.framework {
                let framework_name = metadata.name.clone();
                let priority = metadata.priority.unwrap_or(50);
                priorities.insert(framework_name.clone(), priority);

                if !metadata.suppresses.is_empty() {
                    suppresses.insert(framework_name.clone(), metadata.suppresses);
                }

                // Compile detection rules
                let mut compiled_rules = Vec::new();
                for rule in metadata.detection {
                    // Validate weight if present
                    if let Some(weight) = rule.weight {
                        if !weight.is_finite() || weight < 0.0 {
                            return Err(RuleError::InvalidPattern(format!(
                                "Invalid weight {}: must be finite and non-negative",
                                weight
                            )));
                        }
                    }

                    let compiled = CompiledDetectionRule::compile(rule)?;
                    compiled_rules.push(compiled);
                }

                if !compiled_rules.is_empty() {
                    frameworks.insert(framework_name, compiled_rules);
                }
            }
        }

        Ok(Self {
            frameworks,
            priorities,
            suppresses,
        })
    }

    /// Detect frameworks from module imports
    pub fn detect_from_imports(&self, imports: &[String]) -> Vec<DetectionResult> {
        let mut results: HashMap<String, DetectionResult> = HashMap::new();

        for (framework_name, rules) in &self.frameworks {
            for rule in rules {
                if rule.rule.rule_type != DetectionType::Import {
                    continue;
                }

                let weight = rule.rule.weight.unwrap_or(1.0);

                for import in imports {
                    // All import patterns are compiled as regex
                    let matched = if let Some(ref regex) = rule.regex {
                        regex.is_match(import)
                    } else {
                        // This branch should never be hit for Import type
                        // (all Import patterns are compiled as regex in compile())
                        false
                    };

                    if matched {
                        let confidence = weight;
                        let evidence = DetectionEvidence {
                            framework: framework_name.clone(),
                            rule: format!("import:{}", rule.rule.pattern),
                            confidence,
                            context: Some(format!("import: {}", import)),
                        };

                        results
                            .entry(framework_name.clone())
                            .or_insert_with(|| DetectionResult {
                                framework: framework_name.clone(),
                                confidence: 0.0,
                                evidence: Vec::new(),
                            })
                            .evidence.push(evidence);
                    }
                }
            }
        }

        self.finalize_results(results)
    }

    /// Detect frameworks from file paths
    pub fn detect_from_path(&self, path: &Path) -> Vec<DetectionResult> {
        let mut results: HashMap<String, DetectionResult> = HashMap::new();
        let path_str = path.to_string_lossy();

        for (framework_name, rules) in &self.frameworks {
            for rule in rules {
                let weight = rule.rule.weight.unwrap_or(1.0);
                let mut matched = false;

                match rule.rule.rule_type {
                    DetectionType::FilePath => {
                        if let Some(ref glob) = rule.glob {
                            matched = glob.is_match(path);
                        }
                    }
                    DetectionType::FileExtension => {
                        if let Some(ext) = path.extension() {
                            matched = ext.to_string_lossy() == rule.rule.pattern.trim_start_matches('.');
                        }
                    }
                    _ => continue,
                }

                if matched {
                    let evidence = DetectionEvidence {
                        framework: framework_name.clone(),
                        rule: format!("{}:{}", format!("{:?}", rule.rule.rule_type).to_lowercase(), rule.rule.pattern),
                        confidence: weight,
                        context: Some(format!("path: {}", path_str)),
                    };

                    results
                        .entry(framework_name.clone())
                        .or_insert_with(|| DetectionResult {
                            framework: framework_name.clone(),
                            confidence: 0.0,
                            evidence: Vec::new(),
                        })
                        .evidence.push(evidence);
                }
            }
        }

        self.finalize_results(results)
    }

    /// Detect frameworks from package.json data
    pub fn detect_from_package_json(
        &self,
        dependencies: &HashMap<String, String>,
        scripts: &HashMap<String, String>,
    ) -> Vec<DetectionResult> {
        let mut results: HashMap<String, DetectionResult> = HashMap::new();

        for (framework_name, rules) in &self.frameworks {
            for rule in rules {
                let weight = rule.rule.weight.unwrap_or(1.0);
                let matched = match rule.rule.rule_type {
                    DetectionType::PackageDependency => {
                        dependencies.contains_key(&rule.rule.pattern)
                    }
                    DetectionType::PackageScript => {
                        scripts.values().any(|script| script.contains(&rule.rule.pattern))
                    }
                    _ => continue,
                };

                if matched {
                    let evidence = DetectionEvidence {
                        framework: framework_name.clone(),
                        rule: format!("{}:{}", format!("{:?}", rule.rule.rule_type).to_lowercase(), rule.rule.pattern),
                        confidence: weight,
                        context: Some(format!("package.json: {}", rule.rule.pattern)),
                    };

                    results
                        .entry(framework_name.clone())
                        .or_insert_with(|| DetectionResult {
                            framework: framework_name.clone(),
                            confidence: 0.0,
                            evidence: Vec::new(),
                        })
                        .evidence.push(evidence);
                }
            }
        }

        self.finalize_results(results)
    }

    /// Finalize detection results: calculate confidence, apply suppression, and sort
    fn finalize_results(&self, mut results: HashMap<String, DetectionResult>) -> Vec<DetectionResult> {
        // Calculate total confidence scores
        for result in results.values_mut() {
            result.confidence = result
                .evidence
                .iter()
                .map(|e| e.confidence)
                .sum::<f32>()
                .min(1.0);
        }

        let mut final_results: Vec<DetectionResult> = results.into_values().collect();
        self.apply_suppression(&mut final_results);

        // Sort by priority (higher first), then by confidence
        final_results.sort_by(|a, b| {
            let priority_a = self.priorities.get(&a.framework).copied().unwrap_or(0);
            let priority_b = self.priorities.get(&b.framework).copied().unwrap_or(0);
            priority_b.cmp(&priority_a).then_with(|| {
                // Use total_cmp to handle NaN properly
                b.confidence.total_cmp(&a.confidence)
            })
        });

        final_results
    }

    /// Apply suppression rules (e.g., Next.js suppresses React)
    fn apply_suppression(&self, results: &mut Vec<DetectionResult>) {
        use std::collections::HashSet;

        // Build set of frameworks that should be suppressed
        let suppressed_frameworks: HashSet<&str> = results
            .iter()
            .filter_map(|r| self.suppresses.get(&r.framework))
            .flatten()
            .map(String::as_str)
            .collect();

        // Keep only frameworks that are NOT in the suppressed set
        results.retain(|r| !suppressed_frameworks.contains(r.framework.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_from_imports() {
        let toml = r#"
            [framework]
            name = "React"
            priority = 50

            [[framework.detection]]
            type = "import"
            pattern = "^react$"
            weight = 1.0
        "#;

        let detector = FrameworkDetector::from_toml_files(vec![
            ("React".to_string(), toml),
        ]).unwrap();

        let imports = vec!["react".to_string(), "react-dom".to_string()];
        let results = detector.detect_from_imports(&imports);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].framework, "React");
        assert!(results[0].confidence > 0.0);
    }

    #[test]
    fn test_suppression() {
        let nextjs_toml = r#"
            [framework]
            name = "Next.js"
            priority = 100
            suppresses = ["React"]

            [[framework.detection]]
            type = "import"
            pattern = "^next$"
            weight = 1.0
        "#;

        let react_toml = r#"
            [framework]
            name = "React"
            priority = 50

            [[framework.detection]]
            type = "import"
            pattern = "^react$"
            weight = 1.0
        "#;

        let detector = FrameworkDetector::from_toml_files(vec![
            ("Next.js".to_string(), nextjs_toml),
            ("React".to_string(), react_toml),
        ]).unwrap();

        let imports = vec!["next".to_string(), "react".to_string()];
        let results = detector.detect_from_imports(&imports);

        // Next.js should suppress React
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].framework, "Next.js");
    }

    #[test]
    fn test_regex_size_limit() {
        let long_pattern = "a".repeat(MAX_REGEX_LEN + 1);
        let rule = DetectionRule {
            rule_type: DetectionType::Import,
            pattern: long_pattern,
            weight: None,
        };

        let result = CompiledDetectionRule::compile(rule);
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_pattern_matching() {
        // Test regex pattern with ^react/ (matches react/* packages)
        let toml = r#"
            [framework]
            name = "React"
            priority = 50

            [[framework.detection]]
            type = "import"
            pattern = "^react/"
            weight = 1.0
        "#;

        let detector = FrameworkDetector::from_toml_files(vec![
            ("React".to_string(), toml),
        ]).unwrap();

        // Should match "react/hooks", "react/dom"
        let imports = vec!["react/hooks".to_string(), "react/dom".to_string()];
        let results = detector.detect_from_imports(&imports);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].framework, "React");

        // Should NOT match just "react" (no slash)
        let imports = vec!["react".to_string()];
        let results = detector.detect_from_imports(&imports);
        assert_eq!(results.len(), 0, "Pattern ^react/ should not match 'react'");
    }

    #[test]
    fn test_exact_regex_pattern() {
        // Test exact match with ^react$ regex
        let toml = r#"
            [framework]
            name = "React"
            priority = 50

            [[framework.detection]]
            type = "import"
            pattern = "^react$"
            weight = 1.0
        "#;

        let detector = FrameworkDetector::from_toml_files(vec![
            ("React".to_string(), toml),
        ]).unwrap();

        // Should match only exact "react"
        let imports = vec!["react".to_string()];
        let results = detector.detect_from_imports(&imports);
        assert_eq!(results.len(), 1);

        // Should NOT match "react-dom" or "react/hooks"
        let imports = vec!["react-dom".to_string(), "react/hooks".to_string()];
        let results = detector.detect_from_imports(&imports);
        assert_eq!(results.len(), 0, "Pattern ^react$ should only match exact 'react'");
    }

    #[test]
    fn test_invalid_weight_validation() {
        let toml_nan = r#"
            [framework]
            name = "Test"

            [[framework.detection]]
            type = "import"
            pattern = "test"
            weight = "NaN"
        "#;

        // Should fail to parse invalid weight
        let result = FrameworkDetector::from_toml_files(vec![
            ("Test".to_string(), toml_nan),
        ]);
        assert!(result.is_err(), "Should reject NaN weights");
    }

    #[test]
    fn test_suppression_no_duplicates() {
        // Test that suppression works correctly with multiple frameworks
        let nextjs_toml = r#"
            [framework]
            name = "Next.js"
            priority = 100
            suppresses = ["React"]

            [[framework.detection]]
            type = "import"
            pattern = "next"
            weight = 1.0
        "#;

        let react_toml = r#"
            [framework]
            name = "React"
            priority = 50

            [[framework.detection]]
            type = "import"
            pattern = "react"
            weight = 1.0
        "#;

        let vue_toml = r#"
            [framework]
            name = "Vue"
            priority = 50

            [[framework.detection]]
            type = "import"
            pattern = "vue"
            weight = 1.0
        "#;

        let detector = FrameworkDetector::from_toml_files(vec![
            ("Next.js".to_string(), nextjs_toml),
            ("React".to_string(), react_toml),
            ("Vue".to_string(), vue_toml),
        ]).unwrap();

        // All three detected, but React should be suppressed
        let imports = vec!["next".to_string(), "react".to_string(), "vue".to_string()];
        let results = detector.detect_from_imports(&imports);

        assert_eq!(results.len(), 2, "Should have Next.js and Vue, React suppressed");
        assert!(results.iter().any(|r| r.framework == "Next.js"));
        assert!(results.iter().any(|r| r.framework == "Vue"));
        assert!(!results.iter().any(|r| r.framework == "React"), "React should be suppressed");
    }

    #[test]
    fn test_confidence_sorting() {
        let toml1 = r#"
            [framework]
            name = "High"
            priority = 100

            [[framework.detection]]
            type = "import"
            pattern = "high"
            weight = 0.9
        "#;

        let toml2 = r#"
            [framework]
            name = "Low"
            priority = 50

            [[framework.detection]]
            type = "import"
            pattern = "low"
            weight = 1.0
        "#;

        let detector = FrameworkDetector::from_toml_files(vec![
            ("High".to_string(), toml1),
            ("Low".to_string(), toml2),
        ]).unwrap();

        let imports = vec!["high".to_string(), "low".to_string()];
        let results = detector.detect_from_imports(&imports);

        // High priority should come first despite lower confidence
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].framework, "High", "Higher priority should be first");
        assert_eq!(results[1].framework, "Low");
    }
}

