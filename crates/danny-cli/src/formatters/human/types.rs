//! Types category formatter.

use danny_core::Finding;
use std::collections::HashMap;
use Finding::*;

pub fn print_types(findings: &[&Finding]) {
    println!("\n📝 Types ({}):", findings.len());
    let mut type_exports = Vec::new();

    for finding in findings {
        if let UnusedExport {
            module,
            export_name,
            ..
        } = finding
        {
            type_exports.push((module, export_name));
        }
    }

    if !type_exports.is_empty() {
        let mut by_module: HashMap<_, Vec<_>> = HashMap::new();
        for (module, name) in type_exports.iter().take(50) {
            by_module.entry(*module).or_default().push(*name);
        }

        println!("  Type-Only Exports (0 runtime impact):");
        for (module, exports) in by_module.iter().take(10) {
            let exports_str: Vec<String> = exports.iter().map(|s| (*s).clone()).collect();
            println!("    {}: {}", module.display(), exports_str.join(", "));
        }
        if by_module.len() > 10 {
            println!("    ... and {} more modules", by_module.len() - 10);
        }
    }
}
