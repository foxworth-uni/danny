//! Class member analysis - converts Fob's class member data to Danny findings.

use danny_core::{Finding, ClassMemberKind, SymbolSpan};
use fob::graph::{ModuleGraph, UnusedSymbol};
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::PathBuf;

/// Analyzer for class member findings.
pub struct ClassMemberAnalyzer;

impl ClassMemberAnalyzer {
    /// Convert Fob's unused private class members to Danny findings.
    pub async fn convert_private_members<S: BuildHasher>(
        graph: &ModuleGraph,
        unused_by_class: &HashMap<String, Vec<UnusedSymbol>, S>,
    ) -> Result<Vec<Finding>, String> {
        let mut findings = Vec::new();

        for (class_name, unused_symbols) in unused_by_class {
            for unused in unused_symbols {
                // Get module for this symbol
                let module = graph
                    .module(&unused.module_id)
                    .await
                    .map_err(|e| format!("Failed to get module: {}", e))?
                    .ok_or_else(|| format!("Module not found: {:?}", unused.module_id))?;

                // Skip virtual paths
                if Self::is_virtual_path(&module.path) {
                    continue;
                }

                // Skip underscore-prefixed symbols (intentionally unused)
                if unused.symbol.name.starts_with('_') {
                    continue;
                }

                // Convert symbol kind to class member kind
                let member_kind = Self::convert_symbol_kind_to_member_kind(&unused.symbol.kind);

                // Convert span
                let span = Self::convert_symbol_span(&unused.symbol.declaration_span, &module.path);

                findings.push(Finding::UnusedPrivateClassMember {
                    module: module.path.clone(),
                    class_name: class_name.clone(),
                    member_name: unused.symbol.name.clone(),
                    member_kind,
                    span,
                });
            }
        }

        Ok(findings)
    }

    /// Convert Fob's unused public class members to Danny findings.
    pub async fn convert_public_members(
        graph: &ModuleGraph,
        unused_symbols: &[UnusedSymbol],
    ) -> Result<Vec<Finding>, String> {
        let mut findings = Vec::new();

        for unused in unused_symbols {
            // Get module for this symbol
            let module = graph
                .module(&unused.module_id)
                .await
                .map_err(|e| format!("Failed to get module: {}", e))?
                .ok_or_else(|| format!("Module not found: {:?}", unused.module_id))?;

            // Skip virtual paths
            if Self::is_virtual_path(&module.path) {
                continue;
            }

            // Skip underscore-prefixed symbols (intentionally unused)
            if unused.symbol.name.starts_with('_') {
                continue;
            }

            // Extract class name from symbol metadata
            let class_name = Self::extract_class_name(&unused.symbol.metadata)
                .unwrap_or_else(|| "Unknown".to_string());

            // Convert symbol kind to class member kind
            let member_kind = Self::convert_symbol_kind_to_member_kind(&unused.symbol.kind);

            // Convert span
            let span = Self::convert_symbol_span(&unused.symbol.declaration_span, &module.path);

            findings.push(Finding::UnusedPublicClassMember {
                module: module.path.clone(),
                class_name,
                member_name: unused.symbol.name.clone(),
                member_kind,
                span,
            });
        }

        Ok(findings)
    }

    /// Build class member statistics.
    pub fn build_stats(
        private_count: usize,
        public_count: usize,
        total_members: usize,
    ) -> danny_core::ClassMemberStats {
        use danny_core::MemberVisibility;

        let by_visibility = vec![
            (MemberVisibility::Private, private_count),
            (MemberVisibility::Public, public_count),
        ];

        danny_core::ClassMemberStats {
            total_members,
            unused_private: private_count,
            unused_public: public_count,
            by_visibility,
        }
    }

    /// Convert Fob's SymbolKind to Danny's ClassMemberKind.
    fn convert_symbol_kind_to_member_kind(
        kind: &fob::graph::SymbolKind,
    ) -> ClassMemberKind {
        match kind {
            fob::graph::SymbolKind::ClassMethod => ClassMemberKind::Method,
            fob::graph::SymbolKind::ClassProperty => ClassMemberKind::Property,
            fob::graph::SymbolKind::ClassGetter => ClassMemberKind::Getter,
            fob::graph::SymbolKind::ClassSetter => ClassMemberKind::Setter,
            fob::graph::SymbolKind::ClassConstructor => ClassMemberKind::Constructor,
            _ => ClassMemberKind::Method, // Fallback
        }
    }

    /// Convert Fob's SymbolSpan to Danny's SymbolSpan.
    fn convert_symbol_span(
        span: &fob::graph::SymbolSpan,
        file: &PathBuf,
    ) -> SymbolSpan {
        SymbolSpan {
            file: file.clone(),
            line: span.line,
            column: span.column,
            offset: span.offset,
        }
    }

    /// Extract class name from symbol metadata.
    fn extract_class_name(metadata: &fob::graph::SymbolMetadata) -> Option<String> {
        match metadata {
            fob::graph::SymbolMetadata::ClassMember(class_meta) => {
                Some(class_meta.class_name.clone())
            }
            _ => None,
        }
    }

    /// Check if a path is virtual (should be filtered).
    fn is_virtual_path(path: &PathBuf) -> bool {
        path.to_string_lossy().starts_with("virtual:")
    }
}

