//! Symbols category formatter.

use danny_core::Finding;
use Finding::*;

pub fn print_symbols(findings: &[&Finding]) {
    println!("\n🔍 Symbols ({}):", findings.len());
    let mut unused_symbols = Vec::new();
    let mut unused_private_members = Vec::new();
    let mut unused_public_members = Vec::new();
    let mut unused_enum_members = Vec::new();
    
    for finding in findings {
        match finding {
            UnusedSymbol { .. } => unused_symbols.push(*finding),
            UnusedPrivateClassMember { .. } => unused_private_members.push(*finding),
            UnusedPublicClassMember { .. } => unused_public_members.push(*finding),
            UnusedEnumMember { .. } => unused_enum_members.push(*finding),
            _ => {}
        }
    }
    
    if !unused_symbols.is_empty() {
        println!("  Unused Symbols:");
        for finding in unused_symbols.iter().take(20) {
            if let UnusedSymbol { module, symbol_name, kind, span, explanation: _ } = finding {
                use danny_core::types::SymbolKind;
                let kind_str = match kind {
                    SymbolKind::Function => "function",
                    SymbolKind::Variable => "variable",
                    SymbolKind::Class => "class",
                    SymbolKind::Parameter => "parameter",
                    SymbolKind::TypeAlias => "type",
                    SymbolKind::Interface => "interface",
                    SymbolKind::Enum => "enum",
                };
                println!(
                    "    {} '{}' in {} (line {})",
                    kind_str,
                    symbol_name,
                    module.display(),
                    span.line
                );
            }
        }
        if unused_symbols.len() > 20 {
            println!("    ... and {} more", unused_symbols.len() - 20);
        }
    }
    
    if !unused_private_members.is_empty() {
        println!("\n  Unused Private Class Members (Safe to Remove):");
        for finding in unused_private_members.iter().take(20) {
            if let UnusedPrivateClassMember {
                module,
                class_name,
                member_name,
                member_kind,
                ..
            } = finding
            {
                let kind_str = format!("{:?}", member_kind).to_lowercase();
                println!(
                    "    ✓ {}::{} ({}) in {}",
                    class_name,
                    member_name,
                    kind_str,
                    module.display()
                );
            }
        }
        if unused_private_members.len() > 20 {
            println!("    ... and {} more", unused_private_members.len() - 20);
        }
    }
    
    if !unused_public_members.is_empty() {
        println!("\n  Unused Public Class Members (Warning: May be used externally):");
        for finding in unused_public_members.iter().take(20) {
            if let UnusedPublicClassMember {
                module,
                class_name,
                member_name,
                member_kind,
                ..
            } = finding
            {
                let kind_str = format!("{:?}", member_kind).to_lowercase();
                println!(
                    "    ⚠️  {}::{} ({}) in {}",
                    class_name,
                    member_name,
                    kind_str,
                    module.display()
                );
            }
        }
        if unused_public_members.len() > 20 {
            println!("    ... and {} more", unused_public_members.len() - 20);
        }
    }
    
    if !unused_enum_members.is_empty() {
        println!("\n  Unused Enum Members:");
        for finding in unused_enum_members.iter().take(20) {
            if let UnusedEnumMember {
                module,
                enum_name,
                member_name,
                value,
                ..
            } = finding
            {
                let value_str = value.as_ref().map(|v| match v {
                    danny_core::EnumValue::Number(n) => format!(" = {}", n),
                    danny_core::EnumValue::String(s) => format!(" = \"{}\"", s),
                    danny_core::EnumValue::Computed => " (computed)".to_string(),
                }).unwrap_or_default();
                println!(
                    "    → {}::{}{} in {}",
                    enum_name,
                    member_name,
                    value_str,
                    module.display()
                );
            }
        }
        if unused_enum_members.len() > 20 {
            println!("    ... and {} more", unused_enum_members.len() - 20);
        }
    }
}

