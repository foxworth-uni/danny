//! Circular dependencies category formatter.

use danny_core::Finding;
use super::format_bytes;
use Finding::*;

pub fn print_circular(findings: &[&Finding]) {
    println!("\n🔄 Circular Dependencies ({}):", findings.len());
    for (idx, finding) in findings.iter().enumerate() {
        if let CircularDependency(circ) = finding {
            let size_formatted = format_bytes(circ.total_size);
            let unreachable_indicator = if circ.all_unreachable {
                " (all unreachable)"
            } else {
                ""
            };
            println!(
                "  ⚠️  Cycle {} ({} modules, {}){}:",
                idx + 1,
                circ.cycle.len(),
                size_formatted,
                unreachable_indicator
            );
            let cycle_str = circ
                .cycle
                .iter()
                .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" → ");
            println!("     {}", cycle_str);
        }
    }
}

