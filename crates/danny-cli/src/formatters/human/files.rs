//! File category formatter.

use danny_core::{AnalysisResult, Finding};
use super::format_bytes;
use Finding::*;

pub fn print_files(findings: &[&Finding], result: &AnalysisResult) {
    println!("\n📁 Files ({}):", findings.len());
    let mut unreachable_modules = Vec::new();
    let mut unreachable_files = Vec::new();
    let mut dead_code_modules = Vec::new();
    
    for finding in findings {
        match finding {
            UnreachableModule { .. } => unreachable_modules.push(*finding),
            UnreachableFile { .. } => unreachable_files.push(*finding),
            DeadCodeModule { .. } => dead_code_modules.push(*finding),
            _ => {}
        }
    }
    
    // Print bundle size impact if available
    if let Some(impact) = &result.statistics.bundle_size_impact {
        let total_formatted = format_bytes(impact.total_savings_bytes);
        let safe_formatted = format_bytes(impact.safe_savings_bytes);
        let review_formatted = format_bytes(
            impact.total_savings_bytes - impact.safe_savings_bytes
        );
        println!(
            "  Total potential savings: {} ({} safe, {} review)",
            total_formatted, safe_formatted, review_formatted
        );
        
        let mut safe_modules = Vec::new();
        let mut review_modules = Vec::new();
        for module_info in &impact.by_module {
            if module_info.has_side_effects {
                review_modules.push(module_info);
            } else {
                safe_modules.push(module_info);
            }
        }
        
        for module_info in safe_modules.iter().take(10) {
            let size_formatted = format_bytes(module_info.size_bytes);
            println!(
                "    {} ({}) ✓ Safe to delete",
                module_info.path.display(),
                size_formatted
            );
        }
        if safe_modules.len() > 10 {
            println!("    ... and {} more safe modules", safe_modules.len() - 10);
        }
        
        for module_info in review_modules.iter().take(10) {
            let size_formatted = format_bytes(module_info.size_bytes);
            println!(
                "    {} ({}) ⚠️  Review carefully (has side effects)",
                module_info.path.display(),
                size_formatted
            );
        }
        if review_modules.len() > 10 {
            println!("    ... and {} more modules to review", review_modules.len() - 10);
        }
    } else {
        // Fallback: print unreachable modules
        for finding in unreachable_modules.iter().take(20) {
            if let UnreachableModule { path, size, metadata } = finding {
                let size_formatted = format_bytes(*size);
                let indicator = if metadata.safe_to_delete {
                    "✓ Safe to delete"
                } else if metadata.has_side_effects {
                    "⚠️  Review carefully (has side effects)"
                } else {
                    "⚠️  Review carefully"
                };
                println!("    {} ({}) {}", path.display(), size_formatted, indicator);
            }
        }
        if unreachable_modules.len() > 20 {
            println!("    ... and {} more", unreachable_modules.len() - 20);
        }
    }
    
    // Print unreachable files
    if !unreachable_files.is_empty() {
        println!("\n  Unreachable Files:");
        for finding in unreachable_files.iter().take(20) {
            if let UnreachableFile { path, size, explanation: _ } = finding {
                let size_formatted = format_bytes(*size);
                println!("    {} ({})", path.display(), size_formatted);
            }
        }
        if unreachable_files.len() > 20 {
            println!("    ... and {} more", unreachable_files.len() - 20);
        }
    }
    
    // Print dead code modules
    if !dead_code_modules.is_empty() {
        println!("\n  Dead Code Modules:");
        for finding in dead_code_modules.iter().take(20) {
            if let DeadCodeModule { path, size } = finding {
                let size_formatted = format_bytes(*size);
                println!(
                    "    🗑️  {} ({}) - Strong candidate for removal",
                    path.display(),
                    size_formatted
                );
            }
        }
        if dead_code_modules.len() > 20 {
            println!("    ... and {} more", dead_code_modules.len() - 20);
        }
    }
}

