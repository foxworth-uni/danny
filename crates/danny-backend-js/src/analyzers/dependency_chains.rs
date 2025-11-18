//! Dependency chain analysis - converts Fob's dependency chain data to Danny findings.

use danny_core::Finding;
use fob::graph::{ModuleGraph, dependency_chain::DependencyChain};
use std::path::PathBuf;

/// Analyzer for dependency chain findings.
pub struct DependencyChainAnalyzer;

impl DependencyChainAnalyzer {
    /// Convert Fob's dependency chains to Danny findings.
    pub async fn convert_chains(
        graph: &ModuleGraph,
        chains: &[DependencyChain],
    ) -> Result<Vec<Finding>, String> {
        let mut findings = Vec::new();

        for chain in chains {
            // Convert module IDs to paths
            let mut path_chain = Vec::new();
            for module_id in &chain.path {
                let module = graph
                    .module(module_id)
                    .await
                    .map_err(|e| format!("Failed to get module: {}", e))?
                    .ok_or_else(|| format!("Module not found: {:?}", module_id))?;

                // Skip virtual paths
                if Self::is_virtual_path(&module.path) {
                    continue;
                }

                path_chain.push(module.path.clone());
            }

            // Only add if we have at least 2 modules in the chain
            if path_chain.len() >= 2 {
                findings.push(Finding::DependencyChain {
                    chain: path_chain,
                    depth: chain.depth,
                });
            }
        }

        Ok(findings)
    }

    /// Check if a path is virtual (should be filtered).
    fn is_virtual_path(path: &PathBuf) -> bool {
        path.to_string_lossy().starts_with("virtual:")
    }
}

