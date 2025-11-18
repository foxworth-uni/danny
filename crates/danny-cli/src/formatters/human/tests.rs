//! Tests for human formatter category grouping and output.

use super::*;
use danny_core::{
    AnalysisResult, Category, Finding,
};
use danny_core::types::{
    Statistics, ExportKind, SymbolKind, SymbolSpan, ClassMemberKind,
    NpmDependencyType, SourceLocation, CircularDependency as CircularDep,
    CodeSmellType, SmellSeverity, CodeSmellDetails, UnreachableModuleMetadata,
    SafetyAssessment,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Helper to create a basic AnalysisResult for testing
fn create_test_result(findings: Vec<Finding>) -> AnalysisResult {
    AnalysisResult {
        findings,
        ignored_findings: vec![],
        statistics: Statistics {
            total_modules: 10,
            total_dependencies: 20,
            external_dependencies: 5,
            frameworks_detected: vec!["Next.js".to_string()],
            duration_ms: 100,
            ..Default::default()
        },
        errors: vec![],
    }
}

#[test]
fn test_category_grouping() {
    // Create findings from different categories
    let findings = vec![
        Finding::UnreachableFile {
            path: PathBuf::from("file1.ts"),
            size: 100,
            explanation: None,
        },
        Finding::UnusedExport {
            module: PathBuf::from("mod1.ts"),
            export_name: "foo".to_string(),
            kind: ExportKind::Named,
            span: None,
            is_type_only: false,
            explanation: None,
        },
        Finding::UnusedExport {
            module: PathBuf::from("mod2.ts"),
            export_name: "Bar".to_string(),
            kind: ExportKind::Named,
            span: None,
            is_type_only: true,
            explanation: None,
        },
    ];

    let result = create_test_result(findings);

    // Group by category
    let mut findings_by_category: HashMap<Category, Vec<&Finding>> = HashMap::new();
    for finding in &result.findings {
        let category = finding.category();
        findings_by_category.entry(category).or_default().push(finding);
    }

    // Verify grouping
    assert_eq!(findings_by_category.get(&Category::Files).unwrap().len(), 1);
    assert_eq!(findings_by_category.get(&Category::Exports).unwrap().len(), 1);
    assert_eq!(findings_by_category.get(&Category::Types).unwrap().len(), 1);
}

#[test]
fn test_files_category_mapping() {
    let unreachable_file = Finding::UnreachableFile {
        path: PathBuf::from("test.ts"),
        size: 100,
        explanation: None,
    };
    assert_eq!(unreachable_file.category(), Category::Files);

    let unreachable_module = Finding::UnreachableModule {
        path: PathBuf::from("test.ts"),
        size: 100,
        metadata: UnreachableModuleMetadata {
            has_side_effects: false,
            size_bytes: 100,
            safe_to_delete: true,
            safety_assessment: SafetyAssessment::SafeToDelete,
        },
    };
    assert_eq!(unreachable_module.category(), Category::Files);
}

#[test]
fn test_exports_vs_types_separation() {
    // Runtime export should map to Exports
    let runtime_export = Finding::UnusedExport {
        module: PathBuf::from("test.ts"),
        export_name: "foo".to_string(),
        kind: ExportKind::Named,
        span: None,
        is_type_only: false,
        explanation: None,
    };
    assert_eq!(runtime_export.category(), Category::Exports);

    // Type-only export should map to Types
    let type_export = Finding::UnusedExport {
        module: PathBuf::from("test.ts"),
        export_name: "Foo".to_string(),
        kind: ExportKind::Named,
        span: None,
        is_type_only: true,
        explanation: None,
    };
    assert_eq!(type_export.category(), Category::Types);
}

#[test]
fn test_symbols_category_mapping() {
    let unused_symbol = Finding::UnusedSymbol {
        module: PathBuf::from("test.ts"),
        symbol_name: "helperFunction".to_string(),
        kind: SymbolKind::Function,
        span: SymbolSpan {
            file: PathBuf::from("test.ts"),
            line: 10,
            column: 0,
            offset: 150,
        },
        explanation: None,
    };
    assert_eq!(unused_symbol.category(), Category::Symbols);

    let unused_private_member = Finding::UnusedPrivateClassMember {
        module: PathBuf::from("test.ts"),
        class_name: "MyClass".to_string(),
        member_name: "_private".to_string(),
        member_kind: ClassMemberKind::Property,
        span: SymbolSpan {
            file: PathBuf::from("test.ts"),
            line: 5,
            column: 4,
            offset: 80,
        },
    };
    assert_eq!(unused_private_member.category(), Category::Symbols);
}

#[test]
fn test_framework_category_mapping() {
    let framework_export = Finding::FrameworkExport {
        module: PathBuf::from("page.tsx"),
        export_name: "default".to_string(),
        framework: "Next.js".to_string(),
        rule: "pages_router".to_string(),
        explanation: None,
    };
    assert_eq!(framework_export.category(), Category::Framework);
}

#[test]
fn test_empty_category_handling() {
    // Create result with only Files category findings
    let findings = vec![Finding::UnreachableFile {
        path: PathBuf::from("test.ts"),
        size: 100,
        explanation: None,
    }];

    let result = create_test_result(findings);

    // Group by category
    let mut findings_by_category: HashMap<Category, Vec<&Finding>> = HashMap::new();
    for finding in &result.findings {
        let category = finding.category();
        findings_by_category.entry(category).or_default().push(finding);
    }

    // Verify only Files category has findings
    assert!(findings_by_category.contains_key(&Category::Files));
    assert!(!findings_by_category.contains_key(&Category::Exports));
    assert!(!findings_by_category.contains_key(&Category::Symbols));
}

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(500), "500 bytes");
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(2048), "2.0 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    assert_eq!(format_bytes(1024 * 1024 * 2), "2.00 MB");
}

#[test]
fn test_all_categories_have_printers() {
    // Verify that all Category variants have corresponding print functions
    // This test ensures we don't forget to add a printer when adding a new category

    let result = create_test_result(vec![]);

    // Test that print_category_findings handles all categories without panicking
    for category in Category::all() {
        print_category_findings(*category, &[], &result);
    }
}

#[test]
fn test_print_results_does_not_panic_with_empty_result() {
    let result = create_test_result(vec![]);

    // Should not panic with empty findings
    print_results(&result);
}

#[test]
fn test_print_results_does_not_panic_with_all_categories() {
    let findings = vec![
        // Files
        Finding::UnreachableFile {
            path: PathBuf::from("test.ts"),
            size: 100,
            explanation: None,
        },
        // Exports
        Finding::UnusedExport {
            module: PathBuf::from("mod.ts"),
            export_name: "foo".to_string(),
            kind: ExportKind::Named,
            span: None,
            is_type_only: false,
            explanation: None,
        },
        // Types
        Finding::UnusedExport {
            module: PathBuf::from("types.ts"),
            export_name: "Type".to_string(),
            kind: ExportKind::Named,
            span: None,
            is_type_only: true,
            explanation: None,
        },
        // Symbols
        Finding::UnusedSymbol {
            module: PathBuf::from("utils.ts"),
            symbol_name: "helper".to_string(),
            kind: SymbolKind::Function,
            span: SymbolSpan {
                file: PathBuf::from("utils.ts"),
                line: 10,
                column: 0,
                offset: 200,
            },
            explanation: None,
        },
        // Dependencies
        Finding::UnusedNpmDependency {
            package: "lodash".to_string(),
            version: "4.17.21".to_string(),
            dep_type: NpmDependencyType::Production,
        },
        // Imports
        Finding::SideEffectOnlyImport {
            module: PathBuf::from("index.ts"),
            source: "polyfill.js".to_string(),
            resolved_to: None,
            span: SourceLocation {
                file: PathBuf::from("index.ts"),
                start: 0,
                end: 25,
            },
        },
        // Circular
        Finding::CircularDependency(CircularDep {
            cycle: vec![
                PathBuf::from("a.ts"),
                PathBuf::from("b.ts"),
                PathBuf::from("a.ts"),
            ],
            all_unreachable: false,
            total_size: 1024,
        }),
        // Quality
        Finding::CodeSmell {
            smell_type: CodeSmellType::LongFunction,
            location: PathBuf::from("code.ts"),
            symbol_name: Some("longFunction".to_string()),
            line: Some(10),
            column: Some(0),
            details: CodeSmellDetails {
                message: "Function has 150 lines".to_string(),
                recommendation: Some("Consider breaking it into smaller functions".to_string()),
                current_value: Some(150),
                recommended_threshold: Some(50),
                metadata: HashMap::new(),
            },
            severity: SmellSeverity::Warning,
        },
        // Framework
        Finding::FrameworkExport {
            module: PathBuf::from("page.tsx"),
            export_name: "default".to_string(),
            framework: "Next.js".to_string(),
            rule: "pages_router".to_string(),
            explanation: None,
        },
    ];

    let result = create_test_result(findings);

    // Should handle all categories without panicking
    print_results(&result);
}
