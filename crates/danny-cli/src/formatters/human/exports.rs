//! Export category formatter.

use danny_core::Finding;
use std::collections::HashMap;
use Finding::*;

pub fn print_exports(findings: &[&Finding]) {
    println!("\n📤 Exports ({}):", findings.len());
    let mut runtime_exports = Vec::new();

    for finding in findings {
        if let UnusedExport {
            module,
            export_name,
            span,
            is_type_only,
            ..
        } = finding
        {
            if !*is_type_only {
                runtime_exports.push((module, export_name, span));
            }
        }
    }

    if !runtime_exports.is_empty() {
        let mut by_module: HashMap<_, Vec<_>> = HashMap::new();
        for (module, name, _) in runtime_exports.iter().take(50) {
            by_module.entry(*module).or_default().push(*name);
        }

        for (module, exports) in by_module.iter().take(10) {
            let exports_str: Vec<String> = exports.iter().map(|s| (*s).clone()).collect();
            println!("    {}: {}", module.display(), exports_str.join(", "));
        }
        if by_module.len() > 10 {
            println!("    ... and {} more modules", by_module.len() - 10);
        }
    }
}
