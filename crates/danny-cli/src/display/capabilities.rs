use colored::*;
use danny_config::AnalysisTarget;
use danny_core::{AnalysisMode, Category, UnavailableReason};
use std::io::{self, Write};

pub struct CapabilityDisplay {
    target: AnalysisTarget,
    auto_confirm: bool,
}

impl CapabilityDisplay {
    pub fn new(target: AnalysisTarget, auto_confirm: bool) -> Self {
        Self {
            target,
            auto_confirm,
        }
    }

    /// Display all available and unavailable categories
    pub fn display_list_categories(&self) {
        let caps = self.target.capabilities();

        println!("{}", self.mode_header());
        println!();

        // Show available categories
        println!("{}", "Available categories:".bold());
        for cat in caps.available_categories() {
            println!(
                "  {} {:12} {}",
                "✓".green(),
                cat.cli_name().cyan(),
                cat.description()
            );
        }

        // Show unavailable categories if any
        if !caps.unavailable_categories().is_empty() {
            println!();
            println!("{}", "Unavailable categories:".bold());
            for uc in caps.unavailable_categories() {
                let reason = self.format_reason(&uc.reason);
                println!(
                    "  {} {:12} {}",
                    "✗".red(),
                    uc.category.cli_name().bright_black(),
                    reason.bright_black()
                );
            }
        }

        // Show suggestion if available
        if let Some(suggestion) = self.get_package_suggestion() {
            println!();
            println!("{}", suggestion);
        }
    }

    /// Show warning for partial availability
    pub fn show_partial_warning(&self, available: &[Category], unavailable: &[Category]) {
        eprintln!();
        eprintln!(
            "{} {}",
            "⚠️ ".yellow(),
            "Warning: Some categories unavailable".yellow().bold()
        );
        eprintln!();

        eprintln!("{}", "Will analyze:".bold());
        for cat in available {
            eprintln!("  {} {}", "✓".green(), cat.cli_name());
        }

        eprintln!();
        eprintln!("{}", "Unavailable:".bold());
        for cat in unavailable {
            if let Some(reason) = self.get_unavailable_reason(*cat) {
                eprintln!("  {} {} - {}", "✗".red(), cat.cli_name(), reason);
            }
        }

        if let Some(suggestion) = self.get_package_suggestion() {
            eprintln!();
            eprintln!("{}", suggestion);
        }
        eprintln!();
    }

    /// Show error when no categories are available
    pub fn show_none_available(&self, requested: &[Category], _unavailable: &[Category]) {
        eprintln!();
        eprintln!(
            "{} {}",
            "❌".red(),
            "Error: No requested categories are available".red().bold()
        );
        eprintln!();

        eprintln!("{}", "Requested:".bold());
        for cat in requested {
            if let Some(reason) = self.get_unavailable_reason(*cat) {
                eprintln!("  {} {} - {}", "✗".red(), cat.cli_name(), reason);
            }
        }

        if let Some(suggestion) = self.get_package_suggestion() {
            eprintln!();
            eprintln!("{}", suggestion);
        }
        eprintln!();
    }

    /// Show default categories being used
    pub fn show_using_defaults(&self, categories: &[Category]) {
        println!("{}", "Using default categories:".bright_blue());
        for cat in categories {
            println!("  {} {}", "•".cyan(), cat.cli_name());
        }
        println!();
    }

    /// Ask user for confirmation
    pub fn confirm_continue(&self) -> io::Result<bool> {
        if self.auto_confirm {
            return Ok(true);
        }

        print!("Continue with available categories? [Y/n] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let response = input.trim().to_lowercase();
        Ok(response.is_empty() || response == "y" || response == "yes")
    }

    fn mode_header(&self) -> String {
        match self.target.mode() {
            AnalysisMode::Package => "📦 Package Mode Analysis".bright_blue().bold().to_string(),
            AnalysisMode::Files => "📄 File-Level Analysis Mode"
                .bright_yellow()
                .bold()
                .to_string(),
        }
    }

    fn format_reason(&self, reason: &UnavailableReason) -> &'static str {
        match reason {
            UnavailableReason::RequiresFullGraph => "requires full dependency graph",
            UnavailableReason::RequiresPackageJson => "requires package.json context",
            UnavailableReason::RequiresFrameworkDetection => "no framework detected",
            UnavailableReason::RequiresNodeModules => "node_modules not found (run npm install)",
        }
    }

    fn get_unavailable_reason(&self, category: Category) -> Option<&'static str> {
        let caps = self.target.capabilities();
        caps.unavailable_categories()
            .iter()
            .find(|uc| uc.category == category)
            .map(|uc| self.format_reason(&uc.reason))
    }

    fn get_package_suggestion(&self) -> Option<String> {
        match &self.target {
            AnalysisTarget::Files(ft) => ft.suggest_package_mode().map(|sug| {
                format!(
                    "{}\n  {}",
                    "To enable all categories, run:".bright_blue(),
                    sug.command.cyan()
                )
            }),
            _ => None,
        }
    }
}
