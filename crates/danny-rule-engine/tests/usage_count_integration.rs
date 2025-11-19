//! Integration tests for usage count feature
//!
//! These tests verify real-world scenarios where usage count rules
//! are applied to actual module graphs.

use danny_rule_engine::RuleEngine;
use fob::graph::{Export, ExportKind, Module, ModuleGraph, ModuleId, SourceSpan, SourceType};
use std::path::PathBuf;

fn create_test_module_with_exports(
    path: &str,
    exports: Vec<(String, usize)>, // (name, usage_count)
) -> Module {
    let path_buf = PathBuf::from(path);
    let module_id = ModuleId::new(&path_buf).unwrap();

    let exports_vec: Vec<Export> = exports
        .into_iter()
        .map(|(name, usage_count)| Export {
            name,
            kind: ExportKind::Named,
            span: SourceSpan {
                file: path_buf.clone(),
                start: 0,
                end: 0,
            },
            is_type_only: false,
            is_framework_used: false,
            is_used: false,
            re_exported_from: None,
            came_from_commonjs: false,
            usage_count: Some(usage_count),
        })
        .collect();

    Module::builder(module_id, path_buf, SourceType::JavaScript)
        .exports(exports_vec)
        .build()
}

#[tokio::test]
async fn test_find_rarely_used_exports() {
    // Test finding exports that are used 1-2 times (rarely used)
    let toml = r#"
        [[rules]]
        name = "rarely-used-exports"
        description = "Find exports used 1-2 times"

        [rules.match]
        min_usage_count = 1
        max_usage_count = 2

        [rules.action]
        mark_used = true
        reason = "Rarely used export"
    "#;

    let file: danny_rule_engine::TomlRuleFile = toml::from_str(toml).unwrap();
    let engine = RuleEngine::new(file.rules).unwrap();

    // Create a module graph with various usage counts
    let graph = ModuleGraph::new().await.unwrap();

    let module = create_test_module_with_exports(
        "utils.ts",
        vec![
            ("rarelyUsed1".to_string(), 1),    // Should match
            ("rarelyUsed2".to_string(), 2),    // Should match
            ("popularExport".to_string(), 10), // Should not match
            ("unusedExport".to_string(), 0),   // Should not match
        ],
    );

    graph.add_module(module.clone()).await.unwrap();

    // Collect matching exports
    let modules = graph.modules().await.unwrap();
    let mut matched_exports = Vec::new();

    engine
        .apply_with_callback(&modules, |_module, export_opt, _action| {
            if let Some(export) = export_opt {
                matched_exports.push(export.name.clone());
            }
            Ok(())
        })
        .await
        .unwrap();

    // Verify only rarely used exports matched
    assert_eq!(matched_exports.len(), 2);
    assert!(matched_exports.contains(&"rarelyUsed1".to_string()));
    assert!(matched_exports.contains(&"rarelyUsed2".to_string()));
    assert!(!matched_exports.contains(&"popularExport".to_string()));
    assert!(!matched_exports.contains(&"unusedExport".to_string()));
}

#[tokio::test]
async fn test_find_unused_exports() {
    // Test finding completely unused exports (max = 0)
    let toml = r#"
        [[rules]]
        name = "unused-exports"
        description = "Find completely unused exports"

        [rules.match]
        max_usage_count = 0

        [rules.action]
        mark_used = true
        reason = "Unused export"
    "#;

    let file: danny_rule_engine::TomlRuleFile = toml::from_str(toml).unwrap();
    let engine = RuleEngine::new(file.rules).unwrap();

    // Create a module graph with various usage counts
    let graph = ModuleGraph::new().await.unwrap();

    let module = create_test_module_with_exports(
        "hooks.ts",
        vec![
            ("unusedHook1".to_string(), 0), // Should match
            ("unusedHook2".to_string(), 0), // Should match
            ("usedHook1".to_string(), 1),   // Should not match
            ("usedHook2".to_string(), 5),   // Should not match
        ],
    );

    graph.add_module(module.clone()).await.unwrap();

    // Collect matching exports
    let modules = graph.modules().await.unwrap();
    let mut matched_exports = Vec::new();

    engine
        .apply_with_callback(&modules, |_module, export_opt, _action| {
            if let Some(export) = export_opt {
                matched_exports.push(export.name.clone());
            }
            Ok(())
        })
        .await
        .unwrap();

    // Verify only unused exports matched
    assert_eq!(matched_exports.len(), 2);
    assert!(matched_exports.contains(&"unusedHook1".to_string()));
    assert!(matched_exports.contains(&"unusedHook2".to_string()));
    assert!(!matched_exports.contains(&"usedHook1".to_string()));
    assert!(!matched_exports.contains(&"usedHook2".to_string()));
}

#[tokio::test]
async fn test_find_popular_exports() {
    // Test finding popular exports (min = 10)
    let toml = r#"
        [[rules]]
        name = "popular-exports"
        description = "Find popular exports used at least 10 times"

        [rules.match]
        min_usage_count = 10

        [rules.action]
        mark_used = true
        reason = "Popular export"
    "#;

    let file: danny_rule_engine::TomlRuleFile = toml::from_str(toml).unwrap();
    let engine = RuleEngine::new(file.rules).unwrap();

    // Create a module graph with various usage counts
    let graph = ModuleGraph::new().await.unwrap();

    let module = create_test_module_with_exports(
        "components.ts",
        vec![
            ("popularComponent1".to_string(), 10), // Should match
            ("popularComponent2".to_string(), 15), // Should match
            ("rareComponent".to_string(), 2),      // Should not match
            ("unusedComponent".to_string(), 0),    // Should not match
        ],
    );

    graph.add_module(module.clone()).await.unwrap();

    // Collect matching exports
    let modules = graph.modules().await.unwrap();
    let mut matched_exports = Vec::new();

    engine
        .apply_with_callback(&modules, |_module, export_opt, _action| {
            if let Some(export) = export_opt {
                matched_exports.push(export.name.clone());
            }
            Ok(())
        })
        .await
        .unwrap();

    // Verify only popular exports matched
    assert_eq!(matched_exports.len(), 2);
    assert!(matched_exports.contains(&"popularComponent1".to_string()));
    assert!(matched_exports.contains(&"popularComponent2".to_string()));
    assert!(!matched_exports.contains(&"rareComponent".to_string()));
    assert!(!matched_exports.contains(&"unusedComponent".to_string()));
}
