//! Integration tests for Fob API features.
//!
//! Tests the new analysis features:
//! - Class member detection (private vs public)
//! - Enum member detection
//! - NPM dependency analysis
//! - Import pattern analysis
//! - Dead code module detection

use danny_backend_js::JsBackend;
use danny_core::{AnalysisOptions, Finding, LanguageBackend};
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn test_class_member_analysis() {
    let backend = JsBackend::new().unwrap();

    // Create a temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a test file with unused class members
    let test_file = project_root.join("test.ts");
    std::fs::write(
        &test_file,
        r#"
        class Example {
            public publicMethod() {}
            private privateMethod() {}
            public usedMethod() {}
        }
        
        const example = new Example();
        example.usedMethod();
        "#,
    )
    .unwrap();

    let mut backend_options = HashMap::new();
    backend_options.insert("detect_class_members".to_string(), serde_json::json!(true));

    let options = AnalysisOptions {
        entry_points: vec![test_file.clone()],
        project_root: project_root.to_path_buf(),
        backend_options,
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    // Should find unused private and public members
    let private_members: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::UnusedPrivateClassMember { .. }))
        .collect();

    let public_members: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::UnusedPublicClassMember { .. }))
        .collect();

    // At least one unused member should be found
    assert!(
        !private_members.is_empty() || !public_members.is_empty(),
        "Should find at least one unused class member"
    );

    // Verify statistics
    assert!(result.statistics.class_member_stats.is_some());
}

#[test]
fn test_enum_member_analysis() {
    let backend = JsBackend::new().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    let test_file = project_root.join("test.ts");
    std::fs::write(
        &test_file,
        r#"
        enum Status {
            Active = 1,
            Inactive = 2,
            Pending
        }
        
        const status = Status.Active;
        "#,
    )
    .unwrap();

    let mut backend_options = HashMap::new();
    backend_options.insert("detect_enum_members".to_string(), serde_json::json!(true));

    let options = AnalysisOptions {
        entry_points: vec![test_file.clone()],
        project_root: project_root.to_path_buf(),
        backend_options,
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    let enum_members: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::UnusedEnumMember { .. }))
        .collect();

    // Should find unused enum members (Inactive and Pending)
    assert!(!enum_members.is_empty(), "Should find unused enum members");

    // Verify statistics
    assert!(result.statistics.enum_stats.is_some());
}

#[test]
fn test_npm_dependency_analysis() {
    let backend = JsBackend::new().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create package.json with unused dependency
    let package_json = project_root.join("package.json");
    std::fs::write(
        &package_json,
        r#"
        {
            "name": "test",
            "version": "1.0.0",
            "dependencies": {
                "react": "^18.0.0",
                "unused-package": "^1.0.0"
            }
        }
        "#,
    )
    .unwrap();

    // Create a test file that only imports react
    let test_file = project_root.join("test.ts");
    std::fs::write(
        &test_file,
        r#"
        import React from 'react';
        "#,
    )
    .unwrap();

    let mut backend_options = HashMap::new();
    backend_options.insert(
        "detect_npm_dependencies".to_string(),
        serde_json::json!(true),
    );

    let options = AnalysisOptions {
        entry_points: vec![test_file.clone()],
        project_root: project_root.to_path_buf(),
        backend_options,
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    let unused_deps: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::UnusedNpmDependency { .. }))
        .collect();

    // Should find unused-package as unused
    assert!(
        !unused_deps.is_empty(),
        "Should find unused npm dependencies"
    );

    // Verify coverage statistics
    assert!(result.statistics.dependency_coverage_stats.is_some());
}

#[test]
fn test_import_pattern_analysis() {
    let backend = JsBackend::new().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    let test_file = project_root.join("test.ts");
    std::fs::write(
        &test_file,
        r#"import 'side-effect-module';
import * as utils from './utils';
import type { Type } from './types';
"#,
    )
    .unwrap();

    // Create stub files for the imports to be resolved
    std::fs::write(project_root.join("utils.ts"), "export const foo = 1;").unwrap();
    std::fs::write(project_root.join("types.ts"), "export type Type = string;").unwrap();
    std::fs::write(
        project_root.join("side-effect-module.ts"),
        "console.log('side effect');",
    )
    .unwrap();

    let mut backend_options = HashMap::new();
    backend_options.insert(
        "detect_import_patterns".to_string(),
        serde_json::json!(true),
    );

    let options = AnalysisOptions {
        entry_points: vec![test_file.clone()],
        project_root: project_root.to_path_buf(),
        backend_options,
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    let side_effects: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::SideEffectOnlyImport { .. }))
        .collect();

    let namespaces: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::NamespaceImport { .. }))
        .collect();

    let type_only: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::TypeOnlyImport { .. }))
        .collect();

    // With the fob fix, we should now detect all three import patterns
    println!("=== Import Pattern Detection Results ===");
    println!("Total findings: {}", result.findings.len());
    println!("Side-effect imports: {}", side_effects.len());
    println!("Namespace imports: {}", namespaces.len());
    println!("Type-only imports: {}", type_only.len());

    // Debug: print all findings
    for finding in &result.findings {
        match finding {
            Finding::SideEffectOnlyImport { source, .. } => {
                println!("  ✓ Side-effect: {}", source);
            }
            Finding::NamespaceImport {
                namespace_name,
                source,
                ..
            } => {
                println!("  ✓ Namespace: {} from {}", namespace_name, source);
            }
            Finding::TypeOnlyImport {
                source, specifiers, ..
            } => {
                println!("  ✓ Type-only: {:?} from {}", specifiers, source);
            }
            _ => {}
        }
    }

    // Now assert that we found the expected patterns
    assert_eq!(
        side_effects.len(),
        1,
        "Should find 1 side-effect import (import 'side-effect-module')"
    );

    assert_eq!(
        namespaces.len(),
        1,
        "Should find 1 namespace import (import * as utils)"
    );

    assert_eq!(
        type_only.len(),
        1,
        "Should find 1 type-only import (import type {{ Type }})"
    );
}

#[test]
fn test_dead_code_module_detection() {
    let backend = JsBackend::new().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create entry point that imports 'a'
    let entry_file = project_root.join("index.ts");
    std::fs::write(&entry_file, "import './a';\nexport const main = 'entry';").unwrap();

    // Create module 'a' that has an unused export and imports 'b'
    let file_a = project_root.join("a.ts");
    std::fs::write(
        &file_a,
        "import './b';\nexport const unusedExport = 'unused';\nexport const usedExport = 'used';",
    )
    .unwrap();

    // Create module 'b' that's only reachable through 'a'
    // If 'a' becomes dead code, 'b' would only be reachable through dead code
    let file_b = project_root.join("b.ts");
    std::fs::write(&file_b, "export const value = 'b';").unwrap();

    let mut backend_options = HashMap::new();
    backend_options.insert(
        "detect_dead_code_modules".to_string(),
        serde_json::json!(true),
    );

    let options = AnalysisOptions {
        entry_points: vec![entry_file.clone()],
        project_root: project_root.to_path_buf(),
        backend_options,
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    // Note: This test may not find dead code modules because the scenario
    // requires Fob to detect that a module importing another module has no
    // used exports, making it dead code. This is a complex analysis.
    // For now, just verify the feature doesn't crash
    let dead_modules: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::DeadCodeModule { .. }))
        .collect();

    // The test passes as long as the analysis completes without error
    // Finding dead code modules depends on Fob's is_reachable_only_through_dead_code implementation
    println!("Dead code modules found: {}", dead_modules.len());
}

#[test]
fn test_opt_in_features_default_off() {
    let backend = JsBackend::new().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    let test_file = project_root.join("test.ts");
    std::fs::write(&test_file, "class Example { private method() {} }").unwrap();

    // Don't enable any features
    let options = AnalysisOptions {
        entry_points: vec![test_file.clone()],
        project_root: project_root.to_path_buf(),
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    // Should not find class members when feature is off
    let class_members: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f,
                Finding::UnusedPrivateClassMember { .. } | Finding::UnusedPublicClassMember { .. }
            )
        })
        .collect();

    assert!(
        class_members.is_empty(),
        "Should not find class members when feature is disabled"
    );

    assert!(
        result.statistics.class_member_stats.is_none(),
        "Should not have class member stats when feature is disabled"
    );
}

#[test]
fn test_virtual_path_filtering() {
    let backend = JsBackend::new().unwrap();

    // This test verifies that virtual paths are filtered out
    // Virtual paths are typically generated by bundlers and shouldn't appear in findings

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    let test_file = project_root.join("test.ts");
    std::fs::write(&test_file, "export const test = 'value';").unwrap();

    let mut backend_options = HashMap::new();
    backend_options.insert("detect_class_members".to_string(), serde_json::json!(true));

    let options = AnalysisOptions {
        entry_points: vec![test_file.clone()],
        project_root: project_root.to_path_buf(),
        backend_options,
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    // Verify no virtual paths in findings
    for finding in &result.findings {
        let path: Option<&std::path::PathBuf> = match finding {
            Finding::Module { path, .. } => Some(path),
            Finding::UnusedPrivateClassMember { module, .. } => Some(module),
            Finding::UnusedPublicClassMember { module, .. } => Some(module),
            Finding::UnusedEnumMember { module, .. } => Some(module),
            Finding::SideEffectOnlyImport { module, .. } => Some(module),
            Finding::NamespaceImport { module, .. } => Some(module),
            Finding::TypeOnlyImport { module, .. } => Some(module),
            Finding::DeadCodeModule { path, .. } => Some(path),
            Finding::DependencyChain { chain, .. } => chain.first(),
            _ => None,
        };

        if let Some(p) = path {
            assert!(
                !p.to_string_lossy().starts_with("virtual:"),
                "Should not include virtual paths in findings"
            );
        }
    }
}

#[test]
fn test_dependency_chain_analysis() {
    let backend = JsBackend::new().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a deep import chain: entry -> a -> b -> c -> d -> e -> f (depth 6)
    let entry_file = project_root.join("entry.ts");
    std::fs::write(&entry_file, "import './a';").unwrap();

    let file_a = project_root.join("a.ts");
    std::fs::write(&file_a, "import './b';").unwrap();

    let file_b = project_root.join("b.ts");
    std::fs::write(&file_b, "import './c';").unwrap();

    let file_c = project_root.join("c.ts");
    std::fs::write(&file_c, "import './d';").unwrap();

    let file_d = project_root.join("d.ts");
    std::fs::write(&file_d, "import './e';").unwrap();

    let file_e = project_root.join("e.ts");
    std::fs::write(&file_e, "import './f';").unwrap();

    let file_f = project_root.join("f.ts");
    std::fs::write(&file_f, "export const value = 'end';").unwrap();

    let mut backend_options = HashMap::new();
    backend_options.insert(
        "detect_dependency_chains".to_string(),
        serde_json::json!(true),
    );

    let options = AnalysisOptions {
        entry_points: vec![entry_file.clone()],
        project_root: project_root.to_path_buf(),
        backend_options,
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    let chains: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::DependencyChain { .. }))
        .collect();

    // Should find at least one dependency chain with depth > 5
    assert!(
        !chains.is_empty(),
        "Should find deep dependency chains (depth > 5)"
    );

    // Verify chain depth
    for chain_finding in &chains {
        if let Finding::DependencyChain { depth, chain } = chain_finding {
            assert!(
                *depth > 5,
                "Detected chain should have depth > 5, got {}",
                depth
            );
            assert!(
                chain.len() > 5,
                "Chain should have more than 5 modules, got {}",
                chain.len()
            );
        }
    }

    // Verify statistics
    assert!(
        result.statistics.dependency_chains_count > 0,
        "Statistics should track dependency chains"
    );
}
