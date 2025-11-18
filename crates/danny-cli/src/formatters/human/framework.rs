//! Framework category formatter.

use danny_core::Finding;
use Finding::*;

pub fn print_framework(findings: &[&Finding]) {
    println!("\n🎯 Framework ({}):", findings.len());
    let mut framework_exports = Vec::new();
    
    for finding in findings {
        if let FrameworkExport { module, export_name, framework, rule, explanation: _ } = finding {
            framework_exports.push((module, export_name, framework, rule));
        }
    }
    
    if !framework_exports.is_empty() {
        println!("  Framework-Used Exports:");
        for (module, export_name, framework, rule) in framework_exports.iter().take(20) {
            println!(
                "    '{}' in {} (used by {} via {})",
                export_name,
                module.display(),
                framework,
                rule
            );
        }
        if framework_exports.len() > 20 {
            println!("    ... and {} more", framework_exports.len() - 20);
        }
    }
}

