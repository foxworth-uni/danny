//! Enum member analysis - converts Fob's enum member data to Danny findings.

use danny_core::{Finding, EnumValue, SymbolSpan};
use fob::graph::{ModuleGraph, UnusedSymbol};
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::PathBuf;

/// Analyzer for enum member findings.
pub struct EnumMemberAnalyzer;

impl EnumMemberAnalyzer {
    /// Convert Fob's unused enum members to Danny findings.
    pub async fn convert_unused_members<S: BuildHasher>(
        graph: &ModuleGraph,
        unused_by_enum: &HashMap<String, Vec<UnusedSymbol>, S>,
    ) -> Result<Vec<Finding>, String> {
        let mut findings = Vec::new();

        for (enum_name, unused_symbols) in unused_by_enum {
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

                // Extract enum value from symbol metadata
                let value = Self::extract_enum_value(&unused.symbol.metadata);

                // Convert span
                let span = Self::convert_symbol_span(&unused.symbol.declaration_span, &module.path);

                findings.push(Finding::UnusedEnumMember {
                    module: module.path.clone(),
                    enum_name: enum_name.clone(),
                    member_name: unused.symbol.name.clone(),
                    value,
                    span,
                });
            }
        }

        Ok(findings)
    }

    /// Build enum statistics.
    pub fn build_stats(
        total_enums: usize,
        total_members: usize,
        unused_members: usize,
    ) -> danny_core::EnumStats {
        danny_core::EnumStats {
            total_enums,
            total_members,
            unused_members,
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

    /// Extract enum value from symbol metadata.
    fn extract_enum_value(metadata: &fob::graph::SymbolMetadata) -> Option<EnumValue> {
        match metadata {
            fob::graph::SymbolMetadata::EnumMember(enum_meta) => {
                enum_meta.value.as_ref().map(|v| match v {
                    fob::graph::EnumMemberValue::Number(n) => EnumValue::Number(*n),
                    fob::graph::EnumMemberValue::String(s) => EnumValue::String(s.clone()),
                    fob::graph::EnumMemberValue::Computed => EnumValue::Computed,
                })
            }
            _ => None,
        }
    }

    /// Check if a path is virtual (should be filtered).
    fn is_virtual_path(path: &PathBuf) -> bool {
        path.to_string_lossy().starts_with("virtual:")
    }
}

