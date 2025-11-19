//! Rule engine - applies compiled rules to modules
//!
//! This is the main execution engine that takes TOML rules and applies them
//! to the module graph.

use crate::{CompiledMatcher, Result, RuleAction, TomlRule};
use fob::graph::{Module, ModuleGraph};

/// The rule engine executes compiled rules against modules
#[derive(Clone)]
pub struct RuleEngine {
    pub(crate) rules: Vec<CompiledRule>,
}

/// A compiled rule ready for execution
#[derive(Clone)]
pub(crate) struct CompiledRule {
    pub(crate) name: String,
    pub(crate) matcher: CompiledMatcher,
    pub(crate) action: RuleAction,
}

impl RuleEngine {
    /// Create a new rule engine from TOML rules
    pub fn new(toml_rules: Vec<TomlRule>) -> Result<Self> {
        let rules = toml_rules
            .into_iter()
            .map(|toml_rule| {
                let matcher = CompiledMatcher::from_toml(&toml_rule.matcher)?;
                let action = toml_rule.action.to_action();

                Ok(CompiledRule {
                    name: toml_rule.name,
                    matcher,
                    action,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { rules })
    }

    /// Apply all rules to the module graph
    ///
    /// This mutates the module graph by marking exports as framework-used
    /// according to the rules.
    pub async fn apply(&self, graph: &ModuleGraph) -> Result<RuleStats> {
        let mut stats = RuleStats::default();

        let modules = graph
            .modules()
            .await
            .map_err(|e| crate::RuleError::LoadError {
                path: "module graph".to_string(),
                source: Box::new(e),
            })?;

        for module in &modules {
            for rule in &self.rules {
                // Check if this rule applies to the whole file
                if rule.matcher.is_file_only() {
                    // File-level rule (e.g., skip entire file)
                    if rule.matcher.matches(module, &placeholder_export()) {
                        self.apply_action(module, None, &rule.action, &mut stats)
                            .await?;
                        break; // First matching file-level rule wins
                    }
                } else {
                    // Export-level rule
                    for export in &module.exports {
                        if rule.matcher.matches(module, export) {
                            self.apply_action(module, Some(export), &rule.action, &mut stats)
                                .await?;
                            break; // First matching export-level rule wins
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Apply all rules with a callback for graph mutation
    ///
    /// This is the preferred method for external callers. It separates rule
    /// evaluation from graph mutation, allowing the caller to handle mutable
    /// access to the graph via the callback.
    ///
    /// The callback receives (module, export, action) for each matched rule.
    pub async fn apply_with_callback<F>(
        &self,
        modules: &[Module],
        mut apply_fn: F,
    ) -> Result<RuleStats>
    where
        F: FnMut(&Module, Option<&fob::graph::Export>, &RuleAction) -> fob::Result<()>,
    {
        let mut stats = RuleStats::default();

        for module in modules {
            for rule in &self.rules {
                if rule.matcher.is_file_only() {
                    // File-level rule (e.g., skip entire file)
                    if rule.matcher.matches(module, &placeholder_export()) {
                        #[cfg(debug_assertions)]
                        eprintln!(
                            "Rule '{}' matched file: {}",
                            rule.name,
                            module.path.display()
                        );

                        apply_fn(module, None, &rule.action).map_err(|e| {
                            crate::RuleError::LoadError {
                                path: module.path.display().to_string(),
                                source: Box::new(e),
                            }
                        })?;
                        stats.files_skipped += 1;
                        break; // First matching file-level rule wins
                    }
                } else {
                    // Export-level rule
                    // Check all exports - multiple exports can match the same rule
                    for export in &module.exports {
                        if rule.matcher.matches(module, export) {
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "Rule '{}' matched export '{}' in {}",
                                rule.name,
                                export.name,
                                module.path.display()
                            );

                            apply_fn(module, Some(export), &rule.action).map_err(|e| {
                                crate::RuleError::LoadError {
                                    path: module.path.display().to_string(),
                                    source: Box::new(e),
                                }
                            })?;
                            stats.exports_marked_used += 1;
                            // Continue checking other exports (don't break)
                            // Note: If you want "first matching export wins", move break here
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Apply a rule action to a module/export
    ///
    /// This is a legacy method that doesn't actually mutate the graph.
    /// Use `apply_with_callback` instead for real graph mutation.
    async fn apply_action(
        &self,
        _module: &Module,
        _export: Option<&fob::graph::Export>,
        action: &RuleAction,
        stats: &mut RuleStats,
    ) -> Result<()> {
        match action {
            RuleAction::MarkUsed { reason: _ } => {
                stats.exports_marked_used += 1;
            }
            RuleAction::Skip => {
                stats.files_skipped += 1;
            }
            RuleAction::Warn { message: _ } => {
                // Warning actions are logged but don't affect stats
                // The caller can handle warnings as needed
            }
            RuleAction::SetSeverity { level: _ } => {
                // Severity override actions are handled by the caller
                // They don't affect stats directly
            }
        }

        Ok(())
    }
}

/// Statistics from rule application
#[derive(Debug, Default)]
pub struct RuleStats {
    pub exports_marked_used: usize,
    pub files_skipped: usize,
    pub rules_applied: usize,
}

/// Create a placeholder export for file-only matching
fn placeholder_export() -> fob::graph::Export {
    use std::path::PathBuf;
    fob::graph::Export {
        name: String::new(),
        kind: fob::graph::ExportKind::Named,
        span: fob::graph::SourceSpan {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_engine() {
        let rules = vec![];
        let engine = RuleEngine::new(rules).unwrap();
        assert_eq!(engine.rules.len(), 0);
    }
}
