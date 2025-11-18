//! Bridge between TOML rules and Fob's FrameworkRule trait
//!
//! This module provides `TomlFrameworkRule`, which wraps our TOML-based
//! RuleEngine and implements Fob's `FrameworkRule` trait, allowing TOML
//! rules to integrate seamlessly with Fob's analysis pipeline.

use async_trait::async_trait;
use fob::graph::{Export, FrameworkRule, Module, ModuleGraph};
use std::sync::OnceLock;
use crate::{RuleAction, RuleEngine, TomlRule, TomlRuleFile};

/// A framework rule implemented via TOML configuration
///
/// This struct bridges TOML-based rules to Fob's FrameworkRule trait,
/// allowing declarative pattern matching without Rust code.
pub struct TomlFrameworkRule {
    name: String,
    description: String,
    engine: RuleEngine,
    /// Cached 'static reference to name (leaked exactly once)
    leaked_name: OnceLock<&'static str>,
    /// Cached 'static reference to description (leaked exactly once)
    leaked_description: OnceLock<&'static str>,
}

impl TomlFrameworkRule {
    /// Create a new TOML framework rule from parsed rules
    pub fn new(
        name: String,
        description: String,
        rules: Vec<TomlRule>,
    ) -> crate::Result<Self> {
        let engine = RuleEngine::new(rules)?;
        Ok(Self {
            name,
            description,
            engine,
            leaked_name: OnceLock::new(),
            leaked_description: OnceLock::new(),
        })
    }

    /// Create a new TOML framework rule from a TOML string
    pub fn from_toml_str(name: String, toml_content: &str) -> crate::Result<Self> {
        let rule_file: TomlRuleFile = toml::from_str(toml_content)?;
        let description = rule_file
            .framework
            .and_then(|f| f.description)
            .unwrap_or_default();
        Self::new(name, description, rule_file.rules)
    }
}

#[async_trait]
impl FrameworkRule for TomlFrameworkRule {
    async fn apply(&self, graph: &ModuleGraph) -> fob::Result<()> {
        let modules = graph.modules().await.map_err(|e| {
            fob::Error::InvalidConfig(format!("Failed to get modules: {}", e))
        })?;

        // Apply rules using callback pattern for graph mutation
        self.engine
            .apply_with_callback(&modules, |module, export, action| {
                // Create a blocking context for async graph operations
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        apply_action_to_graph(graph, module, export, action).await
                    })
                })
            })
            .await
            .map_err(|e| fob::Error::InvalidConfig(e.to_string()))?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        // Leak the string exactly once using OnceLock
        // This prevents memory leaks from repeated calls
        self.leaked_name.get_or_init(|| {
            Box::leak(self.name.clone().into_boxed_str())
        })
    }

    fn description(&self) -> &'static str {
        // Leak the string exactly once using OnceLock
        // This prevents memory leaks from repeated calls
        self.leaked_description.get_or_init(|| {
            Box::leak(self.description.clone().into_boxed_str())
        })
    }

    fn is_default(&self) -> bool {
        // All built-in TOML rules are enabled by default
        true
    }

    fn clone_box(&self) -> Box<dyn FrameworkRule> {
        // Reconstruct from the engine's rules
        // This is necessary because FrameworkRule is a trait object
        Box::new(Self {
            name: self.name.clone(),
            description: self.description.clone(),
            engine: self.engine.clone(),
            leaked_name: OnceLock::new(),
            leaked_description: OnceLock::new(),
        })
    }
}

/// Apply a rule action to the module graph
///
/// This is called by the rule engine for each matched module/export.
/// It handles the async graph mutation while maintaining sync callback semantics.
async fn apply_action_to_graph(
    graph: &ModuleGraph,
    module: &Module,
    export: Option<&Export>,
    action: &RuleAction,
) -> fob::Result<()> {
    match action {
        RuleAction::MarkUsed { reason: _ } => {
            if let Some(export_to_mark) = export {
                // Clone module, mark export, update graph
                let mut updated = module.clone();
                if let Some(exp) = updated
                    .exports
                    .iter_mut()
                    .find(|e| e.name == export_to_mark.name)
                {
                    exp.mark_framework_used();
                }
                graph.add_module(updated).await?;
            }
        }
        RuleAction::Skip => {
            // Skip means don't analyze - no action needed on graph
        }
        RuleAction::Warn { message: _ } => {
            // Warning actions are logged but don't mutate the graph
            // The caller can handle warnings as needed
        }
        RuleAction::SetSeverity { level: _ } => {
            // Severity override actions are handled by the caller
            // They don't mutate the graph directly
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fob::graph::{ExportKind, ModuleId, SourceSpan, SourceType};
    use std::path::PathBuf;

    #[test]
    fn test_create_toml_framework_rule() {
        let toml = r#"
[framework]
name = "Test"
description = "Test framework"

[[rules]]
name = "test-rule"
[rules.match]
export_pattern = "^test"
[rules.action]
mark_used = true
"#;

        let rule = TomlFrameworkRule::from_toml_str("Test".to_string(), toml)
            .expect("Failed to parse TOML");

        assert_eq!(rule.name(), "Test");
        assert_eq!(rule.description(), "Test framework");
        assert!(rule.is_default());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_apply_marks_export() {
        let toml = r#"
[[rules]]
name = "mark-test-exports"
[rules.match]
export_pattern = "^test"
[rules.action]
mark_used = true
"#;

        let rule = TomlFrameworkRule::from_toml_str("Test".to_string(), toml)
            .expect("Failed to parse TOML");

        // Create test graph with a module containing "testFunction"
        let graph = ModuleGraph::new().await.expect("Failed to create graph");

        let path = PathBuf::from("test.ts");
        let module_id = ModuleId::new(&path).unwrap();

        let module = Module::builder(module_id.clone(), path.clone(), SourceType::JavaScript)
            .exports(vec![Export {
                name: "testFunction".to_string(),
                kind: ExportKind::Named,
                span: SourceSpan {
                    file: path,
                    start: 0,
                    end: 0,
                },
                is_type_only: false,
                is_framework_used: false,
                is_used: false,
                re_exported_from: None,
                came_from_commonjs: false,
                usage_count: None,
            }])
            .build();

        graph.add_module(module.clone()).await.unwrap();

        // Apply rule
        rule.apply(&graph).await.expect("Failed to apply rule");

        // Verify export was marked
        let updated = graph
            .module(&module_id)
            .await
            .unwrap()
            .expect("Module not found");

        let test_export = updated
            .exports
            .iter()
            .find(|e| e.name == "testFunction")
            .expect("Export not found");

        assert!(
            test_export.is_framework_used,
            "Export should be marked as framework-used"
        );
    }
}
