//! Quality category formatter.

use danny_core::types::{CodeSmellType, SmellSeverity};
use danny_core::Finding;
use Finding::*;

pub fn print_quality(findings: &[&Finding]) {
    println!("\n⚡ Quality ({}):", findings.len());
    let mut high_impact = Vec::new();
    let mut code_quality = Vec::new();

    for finding in findings {
        if let CodeSmell {
            severity,
            smell_type,
            ..
        } = finding
        {
            match (severity, smell_type) {
                (
                    SmellSeverity::Error | SmellSeverity::Warning,
                    CodeSmellType::LongFunction
                    | CodeSmellType::TooManyParameters
                    | CodeSmellType::LargeClass,
                ) => {
                    high_impact.push(*finding);
                }
                _ => {
                    code_quality.push(*finding);
                }
            }
        }
    }

    if !high_impact.is_empty() {
        println!("  🔥 High Impact Issues (fix these first):");

        let mut long_functions = Vec::new();
        let mut too_many_params = Vec::new();
        let mut large_classes = Vec::new();

        for finding in &high_impact {
            if let CodeSmell { smell_type, .. } = finding {
                match smell_type {
                    CodeSmellType::LongFunction => long_functions.push(*finding),
                    CodeSmellType::TooManyParameters => too_many_params.push(*finding),
                    CodeSmellType::LargeClass => large_classes.push(*finding),
                    _ => {}
                }
            }
        }

        if !long_functions.is_empty() {
            println!("\n    Large Functions ({}):", long_functions.len());
            for finding in long_functions.iter().take(10) {
                if let CodeSmell {
                    location,
                    symbol_name,
                    line,
                    details,
                    ..
                } = finding
                {
                    let symbol_display = symbol_name
                        .as_ref()
                        .map(|s| format!("{}()", s))
                        .unwrap_or_else(|| "unknown".to_string());
                    let line_display = line.map(|l| format!(":{}", l)).unwrap_or_default();
                    println!(
                        "      ⚠️  {}{} - {}",
                        location.display(),
                        line_display,
                        symbol_display
                    );
                    if let Some(rec) = &details.recommendation {
                        println!("         💡 {}", rec);
                    }
                }
            }
            if long_functions.len() > 10 {
                println!("      ... and {} more", long_functions.len() - 10);
            }
        }

        if !too_many_params.is_empty() {
            println!("\n    Too Many Parameters ({}):", too_many_params.len());
            for finding in too_many_params.iter().take(10) {
                if let CodeSmell {
                    location,
                    symbol_name,
                    line,
                    details,
                    ..
                } = finding
                {
                    let symbol_display = symbol_name
                        .as_ref()
                        .map(|s| format!("{}()", s))
                        .unwrap_or_else(|| "unknown".to_string());
                    let line_display = line.map(|l| format!(":{}", l)).unwrap_or_default();
                    println!(
                        "      ⚠️  {}{} - {}",
                        location.display(),
                        line_display,
                        symbol_display
                    );
                    if let Some(rec) = &details.recommendation {
                        println!("         💡 {}", rec);
                    }
                }
            }
            if too_many_params.len() > 10 {
                println!("      ... and {} more", too_many_params.len() - 10);
            }
        }

        if !large_classes.is_empty() {
            println!("\n    Large Classes ({}):", large_classes.len());
            for finding in large_classes.iter().take(10) {
                if let CodeSmell {
                    location,
                    symbol_name,
                    line,
                    details,
                    ..
                } = finding
                {
                    let symbol_display =
                        symbol_name.clone().unwrap_or_else(|| "unknown".to_string());
                    let line_display = line.map(|l| format!(":{}", l)).unwrap_or_default();
                    println!(
                        "      ⚠️  {}{} - {}",
                        location.display(),
                        line_display,
                        symbol_display
                    );
                    if let Some(rec) = &details.recommendation {
                        println!("         💡 {}", rec);
                    }
                }
            }
            if large_classes.len() > 10 {
                println!("      ... and {} more", large_classes.len() - 10);
            }
        }
    }

    if !code_quality.is_empty() {
        println!("\n  Code Quality Issues ({}):", code_quality.len());
        for finding in code_quality.iter().take(20) {
            if let CodeSmell {
                location,
                symbol_name,
                line,
                details,
                smell_type,
                ..
            } = finding
            {
                let type_name = match smell_type {
                    CodeSmellType::MagicNumber => "Magic Number",
                    CodeSmellType::MessageChain => "Message Chain",
                    CodeSmellType::ComplexConditional => "Complex Conditional",
                    CodeSmellType::DeepNesting => "Deep Nesting",
                    CodeSmellType::MultipleReturns => "Multiple Returns",
                    CodeSmellType::EmptyCatchBlock => "Empty Catch Block",
                    CodeSmellType::DuplicatedCode => "Duplicated Code",
                    CodeSmellType::LongParameterList => "Long Parameter List",
                    CodeSmellType::TooManyMethods => "Too Many Methods",
                    CodeSmellType::TooManyFields => "Too Many Fields",
                    CodeSmellType::LowCohesion => "Low Cohesion",
                    _ => "Code Smell",
                };
                let symbol_display = symbol_name
                    .as_ref()
                    .map(|s| format!(" - {}", s))
                    .unwrap_or_default();
                let line_display = line.map(|l| format!(":{}", l)).unwrap_or_default();
                println!(
                    "    💡 {}{}{} ({})",
                    location.display(),
                    line_display,
                    symbol_display,
                    type_name
                );
                if let Some(rec) = &details.recommendation {
                    println!("       {}", rec);
                }
            }
        }
        if code_quality.len() > 20 {
            println!("    ... and {} more", code_quality.len() - 20);
        }
    }
}
