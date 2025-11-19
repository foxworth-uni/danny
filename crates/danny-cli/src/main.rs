//! Danny CLI - Dead code analyzer for JavaScript/TypeScript.

mod cli;
mod commands;
mod display;
mod entry_points;
mod formatters;
mod ignore;

use anyhow::Result;
use clap::Parser;
use danny_core::Category;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "danny")]
#[command(about = "Dead code analyzer for JavaScript/TypeScript and package feed generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Paths to analyze (files, directories, or package roots)
    ///
    /// Examples:
    ///   danny .                    # Full package analysis
    ///   danny src/                 # Full package analysis (finds package.json)
    ///   danny src/foo.ts           # File-level analysis (limited categories)
    ///   danny src/*.ts             # File-level analysis
    #[arg(value_name = "PATHS", default_values = ["."])]
    paths: Vec<PathBuf>,

    /// Categories to analyze
    ///
    /// Available categories depend on analysis mode:
    /// - Package mode (danny .): All categories
    /// - Files mode (danny file.ts): symbols, quality, imports, types only
    ///
    /// Use --list-categories to see available categories for your command.
    #[arg(short, long, value_name = "CATEGORY")]
    category: Vec<String>,

    /// List available categories for the given paths and exit
    #[arg(long)]
    list_categories: bool,

    /// Force package mode even when analyzing specific files
    ///
    /// This enables all categories but may be slower.
    #[arg(long)]
    force_package_mode: bool,

    /// Skip confirmation prompts (assume yes)
    #[arg(short = 'y', long)]
    yes: bool,

    /// Output format
    #[arg(short, long = "output", value_enum, default_value = "human")]
    format: OutputFormat,

    /// Configuration file path
    #[arg(long)]
    config: Option<PathBuf>,

    /// Follow external (npm) dependencies
    #[arg(long)]
    follow_external: bool,

    /// Maximum depth for dependency traversal
    #[arg(long)]
    max_depth: Option<usize>,

    /// Disable all ignore patterns (including defaults and .gitignore)
    #[arg(long)]
    no_ignore: bool,

    /// Disable .gitignore respect (still uses default patterns)
    #[arg(long)]
    no_gitignore: bool,

    /// Additional ignore patterns (can be specified multiple times)
    #[arg(long = "ignore", value_name = "PATTERN")]
    ignore_patterns: Vec<String>,

    /// Verbose output
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    verbose: u8,

    /// Output JSON format (alias for --output json)
    #[arg(long)]
    json: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Manage danny configuration
    Config {
        #[command(subcommand)]
        command: commands::ConfigCommand,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle subcommands
    match cli.command {
        Some(Command::Config { command }) => commands::handle_config_command(command),
        None => {
            // Parse category strings to Category enums
            let categories: Vec<Category> = cli
                .category
                .iter()
                .filter_map(|s| Category::from_cli_name(s))
                .collect();

            // Default to analysis with new category system
            cli::analysis::run_analysis(&cli::analysis::AnalysisRunOptions {
                paths: cli.paths.clone(),
                categories,
                list_categories: cli.list_categories,
                force_package_mode: cli.force_package_mode,
                yes: cli.yes,
                config: cli.config.clone(),
                follow_external: cli.follow_external,
                max_depth: cli.max_depth,
                no_ignore: cli.no_ignore,
                no_gitignore: cli.no_gitignore,
                ignore_patterns: cli.ignore_patterns.clone(),
                verbose: cli.verbose,
                json: cli.json,
                format: match cli.format {
                    OutputFormat::Human => cli::analysis::OutputFormat::Human,
                    OutputFormat::Json => cli::analysis::OutputFormat::Json,
                },
            })
        }
    }
}
