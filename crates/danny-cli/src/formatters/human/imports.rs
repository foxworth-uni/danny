//! Imports category formatter.

use danny_core::Finding;
use Finding::*;

pub fn print_imports(findings: &[&Finding]) {
    println!("\n📥 Imports ({}):", findings.len());
    let mut dynamic_imports = Vec::new();
    let mut side_effect_imports = Vec::new();
    let mut namespace_imports = Vec::new();
    let mut type_only_imports = Vec::new();
    let mut dependency_chains = Vec::new();
    
    for finding in findings {
        match finding {
            DynamicImport(_) => dynamic_imports.push(*finding),
            SideEffectOnlyImport { .. } => side_effect_imports.push(*finding),
            NamespaceImport { .. } => namespace_imports.push(*finding),
            TypeOnlyImport { .. } => type_only_imports.push(*finding),
            DependencyChain { .. } => dependency_chains.push(*finding),
            _ => {}
        }
    }
    
    if !dynamic_imports.is_empty() {
        println!("  Dynamic Imports (Code-Split Points):");
        for finding in dynamic_imports.iter().take(20) {
            if let DynamicImport(info) = finding {
                let chunk_indicator = if info.creates_chunk {
                    "(creates chunk)"
                } else {
                    "(external)"
                };
                println!(
                    "    {} → {} {}",
                    info.from.display(),
                    info.to.display(),
                    chunk_indicator
                );
            }
        }
        if dynamic_imports.len() > 20 {
            println!("    ... and {} more", dynamic_imports.len() - 20);
        }
    }
    
    if !side_effect_imports.is_empty() {
        println!("\n  Side-Effect-Only Imports (Cannot be tree-shaken):");
        for finding in side_effect_imports.iter().take(20) {
            if let SideEffectOnlyImport { module, source, .. } = finding {
                println!("    ⚡ import '{}' in {}", source, module.display());
            }
        }
        if side_effect_imports.len() > 20 {
            println!("    ... and {} more", side_effect_imports.len() - 20);
        }
    }
    
    if !namespace_imports.is_empty() {
        println!("\n  Namespace Imports (import * as X):");
        for finding in namespace_imports.iter().take(20) {
            if let NamespaceImport {
                module,
                namespace_name,
                source,
                ..
            } = finding
            {
                println!(
                    "    📦 import * as {} from '{}' in {}",
                    namespace_name,
                    source,
                    module.display()
                );
            }
        }
        if namespace_imports.len() > 20 {
            println!("    ... and {} more", namespace_imports.len() - 20);
        }
    }
    
    if !type_only_imports.is_empty() {
        println!("\n  Type-Only Imports (TypeScript import type):");
        for finding in type_only_imports.iter().take(20) {
            if let TypeOnlyImport {
                module,
                source,
                specifiers,
                ..
            } = finding
            {
                let specifiers_str = if specifiers.is_empty() {
                    "".to_string()
                } else {
                    format!(" {{ {} }}", specifiers.join(", "))
                };
                println!(
                    "    📝 import type{}{} from '{}' in {}",
                    specifiers_str,
                    if specifiers.is_empty() { "" } else { "" },
                    source,
                    module.display()
                );
            }
        }
        if type_only_imports.len() > 20 {
            println!("    ... and {} more", type_only_imports.len() - 20);
        }
    }
    
    if !dependency_chains.is_empty() {
        println!("\n  Dependency Chains (Import Path Analysis):");
        for finding in dependency_chains.iter().take(10) {
            if let DependencyChain { chain, depth } = finding {
                let chain_str = chain
                    .iter()
                    .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join(" → ");
                println!("    🔗 {} (depth: {})", chain_str, depth);
            }
        }
        if dependency_chains.len() > 10 {
            println!("    ... and {} more", dependency_chains.len() - 10);
        }
    }
}

