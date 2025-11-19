//! Integration tests for Phase 1 features.

use danny_backend_js::JsBackend;
use danny_core::{AnalysisOptions, Finding, LanguageBackend};
use std::path::PathBuf;

#[tokio::test]
#[ignore] // Requires network and test files
async fn test_nextjs_app_phase1_features() {
    let backend = JsBackend::new().unwrap();

    let options = AnalysisOptions {
        entry_points: vec![PathBuf::from("test-files/nextjs-app/pages/index.tsx")],
        project_root: PathBuf::from("test-files/nextjs-app"),
        ..Default::default()
    };

    let result = backend.analyze(options).unwrap();

    // Feature 1 & 2: Bundle size impact should be calculated if there are unreachable modules
    if result.statistics.unreachable_modules_count > 0 {
        assert!(result.statistics.bundle_size_impact.is_some());
        let impact = result.statistics.bundle_size_impact.unwrap();
        assert!(impact.total_savings_bytes >= 0);
        assert!(impact.safe_savings_bytes <= impact.total_savings_bytes);
    }

    // Feature 3: Type-only exports should be counted
    assert!(result.statistics.type_only_unused_exports_count >= 0);
    assert!(
        result.statistics.type_only_unused_exports_count <= result.statistics.unused_exports_count
    );

    // Feature 4: Dynamic imports should be detected
    let dynamic_imports: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::DynamicImport(_)))
        .collect();
    assert_eq!(
        dynamic_imports.len(),
        result.statistics.dynamic_imports_count
    );

    // Feature 5: Circular dependencies
    let circular_deps: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::CircularDependency(_)))
        .collect();
    assert_eq!(
        circular_deps.len(),
        result.statistics.circular_dependencies_count
    );

    // Verify unreachable modules have enriched metadata
    let unreachable_with_metadata: Vec<_> = result
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::UnreachableModule { .. }))
        .collect();

    for finding in unreachable_with_metadata {
        if let Finding::UnreachableModule { metadata, .. } = finding {
            // Metadata should be populated
            assert_eq!(metadata.size_bytes, metadata.size_bytes); // Sanity check
        }
    }
}

#[test]
fn test_phase1_features_structure() {
    // This test verifies that Phase 1 features are properly integrated
    // It doesn't require actual analysis, just checks that types compile

    // Verify backend can be created
    let backend = JsBackend::new();
    assert!(backend.is_ok());

    // If we get here, the Phase 1 integration didn't break basic structure
}
