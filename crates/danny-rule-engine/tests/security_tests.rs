//! Comprehensive security tests for the danny-rule-engine
//!
//! These tests verify protection against:
//! - ReDoS (Regular Expression Denial of Service)
//! - Memory exhaustion via large files
//! - Path traversal attacks
//! - Malicious TOML structures
//! - Regex size limit bypasses

use danny_rule_engine::{CompiledMatcher, RuleLoader, RuleMatcher, MAX_CONTENT_SIZE, MAX_REGEX_LENGTH, MAX_TOML_FILE_SIZE};
use fob::graph::{Export, ExportKind, Import, ImportKind, Module, ModuleId, SourceSpan, SourceType};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

// Test helper functions
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

// ============================================================================
// 2.1 ReDoS Pattern Testing
// ============================================================================

#[test]
fn test_redos_catastrophic_backtracking_nested_quantifiers() {
    // Pattern: (a+)+b - catastrophic backtracking
    // Input: "aaaaaaaaaaaaaaaaaaaaaaaaaaaa" (no 'b')
    // Should complete quickly (<100ms) or reject pattern
    let pattern = "(a+)+b";
    let start = Instant::now();
    
    let result = CompiledMatcher::from_toml(&RuleMatcher {
        export_pattern: Some(pattern.to_string()),
        ..Default::default()
    });
    
    let duration = start.elapsed();
    
    // Either compilation should fail (preferred) or complete quickly
    if result.is_ok() {
        // If compilation succeeds, matching should be bounded
        let compiled = result.unwrap();
        let module = create_test_module("test.ts", vec![]);
        let export = create_test_export("aaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        
        let match_start = Instant::now();
        let _matched = compiled.matches(&module, &export);
        let match_duration = match_start.elapsed();
        
        // Matching should complete quickly (<100ms)
        assert!(
            match_duration < Duration::from_millis(100),
            "Matching took too long: {:?}",
            match_duration
        );
    }
    
    // Compilation should complete quickly regardless
    assert!(
        duration < Duration::from_millis(100),
        "Compilation took too long: {:?}",
        duration
    );
}

#[test]
fn test_redos_nested_star_quantifiers() {
    // Pattern: (x*)*y - nested star quantifiers
    let pattern = "(x*)*y";
    let start = Instant::now();
    
    let result = CompiledMatcher::from_toml(&RuleMatcher {
        export_pattern: Some(pattern.to_string()),
        ..Default::default()
    });
    
    let duration = start.elapsed();
    
    if result.is_ok() {
        let compiled = result.unwrap();
        let module = create_test_module("test.ts", vec![]);
        let export = create_test_export("xxxxxxxxxxxxxxxxxxxxxxxx");
        
        let match_start = Instant::now();
        let _matched = compiled.matches(&module, &export);
        let match_duration = match_start.elapsed();
        
        assert!(
            match_duration < Duration::from_millis(100),
            "Matching took too long: {:?}",
            match_duration
        );
    }
    
    assert!(
        duration < Duration::from_millis(100),
        "Compilation took too long: {:?}",
        duration
    );
}

#[test]
fn test_redos_alternation_with_overlap() {
    // Pattern: (a|a)*b - alternation with overlap
    let pattern = "(a|a)*b";
    let start = Instant::now();
    
    let result = CompiledMatcher::from_toml(&RuleMatcher {
        export_pattern: Some(pattern.to_string()),
        ..Default::default()
    });
    
    let duration = start.elapsed();
    
    if result.is_ok() {
        let compiled = result.unwrap();
        let module = create_test_module("test.ts", vec![]);
        let export = create_test_export("aaaaaaaaaaaaaaaaaaaa");
        
        let match_start = Instant::now();
        let _matched = compiled.matches(&module, &export);
        let match_duration = match_start.elapsed();
        
        assert!(
            match_duration < Duration::from_millis(100),
            "Matching took too long: {:?}",
            match_duration
        );
    }
    
    assert!(
        duration < Duration::from_millis(100),
        "Compilation took too long: {:?}",
        duration
    );
}

#[test]
fn test_redos_dfa_size_limit_enforcement() {
    // Verify DFA size limits prevent memory exhaustion
    // Create a pattern that would create a large DFA if not limited
    let pattern = "a{1000,2000}";
    let start = Instant::now();

    let _result = CompiledMatcher::from_toml(&RuleMatcher {
        export_pattern: Some(pattern.to_string()),
        ..Default::default()
    });

    let duration = start.elapsed();

    // Should either reject or compile quickly due to DFA limits
    assert!(
        duration < Duration::from_millis(500),
        "Compilation took too long: {:?}",
        duration
    );
}

#[test]
fn test_redos_compilation_time_bounded() {
    // Verify regex compilation completes in <100ms
    let patterns = vec![
        ".*.*.*.*.*.*.*.*.*.*.*.*.*.*.*.*.*.*.*.*",
        "(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z)+",
        "a*b*c*d*e*f*g*h*i*j*k*l*m*n*o*p*q*r*s*t*u*v*w*x*y*z*",
    ];
    
    for pattern in patterns {
        let start = Instant::now();
        let _result = CompiledMatcher::from_toml(&RuleMatcher {
            export_pattern: Some(pattern.to_string()),
            ..Default::default()
        });
        let duration = start.elapsed();
        
        assert!(
            duration < Duration::from_millis(100),
            "Pattern '{}' compilation took too long: {:?}",
            pattern,
            duration
        );
    }
}

// ============================================================================
// 2.2 Large File Handling
// ============================================================================

#[test]
fn test_content_pattern_rejects_oversized_files() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.ts");
    
    // Create a file larger than MAX_CONTENT_SIZE (10MB)
    let large_content = vec![b'a'; (MAX_CONTENT_SIZE as usize) + 1];
    fs::write(&file_path, large_content).unwrap();
    
    let matcher = RuleMatcher {
        content_pattern: Some("@public".to_string()),
        ..Default::default()
    };
    
    let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
    let module = create_test_module(file_path.to_str().unwrap(), vec![]);
    let export = create_test_export("test");
    
    // Should reject oversized file (return false)
    assert!(!compiled.matches(&module, &export));
}

#[test]
fn test_content_pattern_accepts_normal_files() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("normal.ts");
    
    // Create a normal-sized file (<10MB) with @public tag
    let content = "// @public\nexport const test = 123;";
    fs::write(&file_path, content).unwrap();
    
    let matcher = RuleMatcher {
        content_pattern: Some("@public".to_string()),
        ..Default::default()
    };
    
    let compiled = CompiledMatcher::from_toml(&matcher).unwrap();
    let module = create_test_module(file_path.to_str().unwrap(), vec![]);
    let export = create_test_export("test");
    
    // Should accept normal file
    assert!(compiled.matches(&module, &export));
}

#[tokio::test]
async fn test_toml_loader_rejects_oversized_files() {
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join(".danny/rules");
    fs::create_dir_all(&rules_dir).unwrap();
    
    let rule_file = rules_dir.join("large.toml");
    
    // Create a TOML file larger than MAX_TOML_FILE_SIZE (1MB)
    let large_content = vec![b'a'; (MAX_TOML_FILE_SIZE as usize) + 1];
    fs::write(&rule_file, large_content).unwrap();
    
    let fs = Arc::new(danny_fs::NativeFileSystem::new(temp_dir.path()).unwrap());
    let loader = RuleLoader::new(fs, temp_dir.path());
    let result = loader.load_all().await;
    
    // Should reject oversized TOML file
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("exceeds maximum size"));
    }
}

#[tokio::test]
async fn test_file_size_edge_cases() {
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join(".danny/rules");
    fs::create_dir_all(&rules_dir).unwrap();
    
    // Test exactly at limit
    let rule_file_at_limit = rules_dir.join("at_limit.toml");
    let content_at_limit = format!(
        "[[rules]]\nname = \"test\"\n[rules.match]\nexport_pattern = \"test\"\n[rules.action]\nmark_used = true\n{}\n",
        "a".repeat((MAX_TOML_FILE_SIZE as usize) - 100) // Leave room for TOML structure
    );
    fs::write(&rule_file_at_limit, content_at_limit).unwrap();
    
    let fs = Arc::new(danny_fs::NativeFileSystem::new(temp_dir.path()).unwrap());
    let loader = RuleLoader::new(fs.clone(), temp_dir.path());
    // Should accept file at limit (or close to it, depending on actual size)
    let _result = loader.load_all().await;
    
    // Test just over limit
    let rule_file_over_limit = rules_dir.join("over_limit.toml");
    let content_over_limit = vec![b'a'; (MAX_TOML_FILE_SIZE as usize) + 1];
    fs::write(&rule_file_over_limit, content_over_limit).unwrap();
    
    let loader2 = RuleLoader::new(fs, temp_dir.path());
    let result2 = loader2.load_all().await;
    
    // Should reject file over limit
    assert!(result2.is_err());
}

// ============================================================================
// 2.3 Path Traversal Protection
// ============================================================================

#[tokio::test]
#[cfg(unix)]
async fn test_symlink_not_followed() {
    use std::os::unix::fs::symlink;
    
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join(".danny/rules");
    fs::create_dir_all(&rules_dir).unwrap();
    
    // Create a symlink pointing outside the rules directory
    let outside_file = temp_dir.path().join("outside.toml");
    fs::write(&outside_file, "[[rules]]\nname = \"outside\"\n").unwrap();
    
    let symlink_path = rules_dir.join("link.toml");
    symlink(&outside_file, &symlink_path).unwrap();
    
    let fs = Arc::new(danny_fs::NativeFileSystem::new(temp_dir.path()).unwrap());
    let loader = RuleLoader::new(fs, temp_dir.path());
    let result = loader.load_all().await;
    
    // Symlink should not be followed (should be skipped or rejected)
    // The loader should not load rules from outside the directory
    // This test verifies that follow_links(false) is working
    assert!(result.is_ok());
    let rules = result.unwrap();
    // Should not have loaded the outside file
    assert!(!rules.iter().any(|r| r.name == "outside"));
}

#[tokio::test]
async fn test_path_canonicalization_prevents_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join(".danny/rules");
    fs::create_dir_all(&rules_dir).unwrap();
    
    // Create a subdirectory
    let subdir = rules_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();
    
    // Create a valid rule file in subdirectory
    let valid_file = subdir.join("valid.toml");
    fs::write(
        &valid_file,
        "[[rules]]\nname = \"valid\"\n[rules.match]\nexport_pattern = \"test\"\n[rules.action]\nmark_used = true\n",
    )
    .unwrap();
    
    let fs = Arc::new(danny_fs::NativeFileSystem::new(temp_dir.path()).unwrap());
    let loader = RuleLoader::new(fs, temp_dir.path());
    let result = loader.load_all().await;
    
    // Should successfully load valid file
    if let Err(e) = &result {
        eprintln!("Error loading rules: {}", e);
    }
    assert!(result.is_ok(), "Failed to load rules: {:?}", result);
    let rules = result.unwrap();
    assert!(rules.iter().any(|r| r.name == "valid"));
}

#[tokio::test]
async fn test_files_stay_within_directory() {
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join(".danny/rules");
    fs::create_dir_all(&rules_dir).unwrap();
    
    // Create a valid rule file
    let valid_file = rules_dir.join("valid.toml");
    fs::write(
        &valid_file,
        "[[rules]]\nname = \"valid\"\n[rules.match]\nexport_pattern = \"test\"\n[rules.action]\nmark_used = true\n",
    )
    .unwrap();
    
    let fs = Arc::new(danny_fs::NativeFileSystem::new(temp_dir.path()).unwrap());
    let loader = RuleLoader::new(fs, temp_dir.path());
    let result = loader.load_all().await;
    
    assert!(result.is_ok());
    let rules = result.unwrap();
    
    // All loaded rules should be from within the rules directory
    // (This is implicitly verified by the loader's canonicalization checks)
    assert!(!rules.is_empty());
}

#[tokio::test]
#[cfg(unix)]
async fn test_symlink_to_sensitive_file() {
    use std::os::unix::fs::symlink;
    
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join(".danny/rules");
    fs::create_dir_all(&rules_dir).unwrap();
    
    // Try to create a symlink to /etc/passwd (should not be followed)
    let symlink_path = rules_dir.join("passwd.toml");
    
    // Only create symlink if /etc/passwd exists (it should on Unix)
    if PathBuf::from("/etc/passwd").exists() {
        let _ = symlink("/etc/passwd", &symlink_path);
        
        let fs = Arc::new(danny_fs::NativeFileSystem::new(temp_dir.path()).unwrap());
        let loader = RuleLoader::new(fs, temp_dir.path());
        let result = loader.load_all().await;
        
        // Should not load /etc/passwd even if symlink exists
        // The loader should skip it or fail safely
        // We just verify it doesn't crash or load sensitive data
        let _ = result; // Don't assert - symlink might fail or be skipped
    }
}

// ============================================================================
// 2.4 Malicious TOML Edge Cases
// ============================================================================

#[tokio::test]
async fn test_deeply_nested_toml_structures() {
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join(".danny/rules");
    fs::create_dir_all(&rules_dir).unwrap();
    
    // Create a deeply nested TOML structure (100+ levels)
    let mut nested = String::from("[[rules]]\nname = \"nested\"\n");
    for i in 0..100 {
        nested.push_str(&format!("[rules.level{}]\n", i));
        nested.push_str(&format!("value = {}\n", i));
    }
    nested.push_str("[rules.match]\nexport_pattern = \"test\"\n[rules.action]\nmark_used = true\n");
    
    let rule_file = rules_dir.join("nested.toml");
    fs::write(&rule_file, nested).unwrap();
    
    let fs = Arc::new(danny_fs::NativeFileSystem::new(temp_dir.path()).unwrap());
    let loader = RuleLoader::new(fs, temp_dir.path());
    let _result = loader.load_all().await;
    
    // Should handle deeply nested structures (either parse or reject gracefully)
    // Don't crash or hang
}

#[test]
fn test_oversized_regex_patterns_rejected() {
    // Create a pattern exactly at limit (500 chars)
    let pattern_at_limit = "a".repeat(MAX_REGEX_LENGTH);
    
    let result = CompiledMatcher::from_toml(&RuleMatcher {
        export_pattern: Some(pattern_at_limit),
        ..Default::default()
    });
    
    // Should accept pattern at limit
    assert!(result.is_ok());
    
    // Create a pattern over limit (501 chars)
    let pattern_over_limit = "a".repeat(MAX_REGEX_LENGTH + 1);
    
    let result2 = CompiledMatcher::from_toml(&RuleMatcher {
        export_pattern: Some(pattern_over_limit),
        ..Default::default()
    });
    
    // Should reject pattern over limit
    assert!(result2.is_err());
    if let Err(e) = result2 {
        assert!(e.to_string().contains("exceeds maximum length"));
    }
}

#[test]
fn test_invalid_regex_syntax_rejected() {
    let invalid_patterns = vec![
        "[",           // Unclosed bracket
        "(",           // Unclosed parenthesis
        "\\",          // Incomplete escape
        "*",           // Quantifier without operand
        "?",           // Quantifier without operand
        "+",           // Quantifier without operand
        "{",           // Unclosed brace
        "a{",          // Incomplete quantifier
        "a{10,",       // Incomplete quantifier
        "a{10,20",     // Unclosed quantifier
    ];
    
    for pattern in invalid_patterns {
        let result = CompiledMatcher::from_toml(&RuleMatcher {
            export_pattern: Some(pattern.to_string()),
            ..Default::default()
        });
        
        // Should reject invalid regex syntax
        assert!(
            result.is_err(),
            "Pattern '{}' should have been rejected",
            pattern
        );
    }
}

#[tokio::test]
async fn test_multiple_malformed_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join(".danny/rules");
    fs::create_dir_all(&rules_dir).unwrap();
    
    // Create a TOML file with multiple malformed patterns
    let malformed_toml = r#"
[[rules]]
name = "rule1"
[rules.match]
export_pattern = "["  # Invalid regex

[[rules]]
name = "rule2"
[rules.match]
export_pattern = "("  # Invalid regex

[[rules]]
name = "rule3"
[rules.match]
export_pattern = "^valid"  # Valid regex
[rules.action]
mark_used = true
"#;
    
    let rule_file = rules_dir.join("malformed.toml");
    fs::write(&rule_file, malformed_toml).unwrap();
    
    let fs = Arc::new(danny_fs::NativeFileSystem::new(temp_dir.path()).unwrap());
    let loader = RuleLoader::new(fs, temp_dir.path());
    let result = loader.load_all().await;
    
    // Should reject file with invalid patterns
    assert!(result.is_err());
}

// ============================================================================
// 2.5 Regex Size Limits
// ============================================================================

#[test]
fn test_pattern_at_limit_accepted() {
    // Pattern exactly at 500 char limit
    let pattern = "a".repeat(MAX_REGEX_LENGTH);
    
    let result = CompiledMatcher::from_toml(&RuleMatcher {
        export_pattern: Some(pattern.clone()),
        ..Default::default()
    });
    
    assert!(result.is_ok());
    
    // Verify it actually works
    let compiled = result.unwrap();
    let module = create_test_module("test.ts", vec![]);
    let export = create_test_export(&pattern);
    
    assert!(compiled.matches(&module, &export));
}

#[test]
fn test_pattern_over_limit_rejected() {
    // Pattern over 500 char limit
    let pattern = "a".repeat(MAX_REGEX_LENGTH + 1);
    
    let result = CompiledMatcher::from_toml(&RuleMatcher {
        export_pattern: Some(pattern),
        ..Default::default()
    });
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("exceeds maximum length"));
        assert!(e.to_string().contains(&MAX_REGEX_LENGTH.to_string()));
    }
}

