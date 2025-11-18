//! Dependencies category formatter.

use danny_core::Finding;
use std::collections::HashMap;
use Finding::*;

pub fn print_dependencies(findings: &[&Finding]) {
    println!("\n📦 Dependencies ({}):", findings.len());
    let mut by_type: HashMap<_, Vec<_>> = HashMap::new();
    
    for finding in findings {
        if let UnusedNpmDependency {
            package,
            version,
            dep_type,
        } = finding
        {
            let type_str = match dep_type {
                danny_core::NpmDependencyType::Production => "dependencies",
                danny_core::NpmDependencyType::Development => "devDependencies",
                danny_core::NpmDependencyType::Peer => "peerDependencies",
                danny_core::NpmDependencyType::Optional => "optionalDependencies",
            };
            by_type.entry(type_str).or_default().push((package, version));
        }
    }
    
    for (dep_type, deps) in by_type.iter() {
        println!("  {} ({}):", dep_type, deps.len());
        for (package, version) in deps.iter().take(10) {
            println!("    💡 {}@{} - Suggestion: Remove from package.json", package, version);
        }
        if deps.len() > 10 {
            println!("    ... and {} more", deps.len() - 10);
        }
    }
}

