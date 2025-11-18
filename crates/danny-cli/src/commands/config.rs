use anyhow::{Context, Result};
use clap::Subcommand;
use danny_config::{ConfigManager, Project};
use std::path::PathBuf;
use tokio::runtime::Runtime;

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Initialize config file at ~/.danny/config.toml
    Init,

    /// Add a project to the config
    Add {
        /// Path to project root
        path: PathBuf,

        /// Display name for the project
        #[arg(short, long)]
        name: Option<String>,

        /// Custom project ID (auto-generated if not provided)
        #[arg(long)]
        id: Option<String>,
    },

    /// List all projects
    List {
        /// Show only enabled projects
        #[arg(long)]
        enabled_only: bool,
    },

    /// Enable a project
    Enable {
        /// Project ID to enable
        project_id: String,
    },

    /// Disable a project
    Disable {
        /// Project ID to disable
        project_id: String,
    },

    /// Remove a project from config
    Remove {
        /// Project ID to remove
        project_id: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Show config file path
    Path,

    /// Validate config file
    Validate,
}

pub fn handle_config_command(cmd: ConfigCommand) -> Result<()> {
    let runtime = Runtime::new().context("Failed to create tokio runtime")?;
    
    runtime.block_on(async {
        match cmd {
            ConfigCommand::Init => init_config().await,
            ConfigCommand::Add { path, name, id } => add_project(path, name, id).await,
            ConfigCommand::List { enabled_only } => list_projects(enabled_only).await,
            ConfigCommand::Enable { project_id } => enable_project(project_id).await,
            ConfigCommand::Disable { project_id } => disable_project(project_id).await,
            ConfigCommand::Remove { project_id, yes } => remove_project(project_id, yes).await,
            ConfigCommand::Path => show_config_path(),
            ConfigCommand::Validate => validate_config().await,
        }
    })
}

async fn init_config() -> Result<()> {
    let config_path = ConfigManager::config_path()?;

    if config_path.exists() {
        println!("Config already exists at: {}", config_path.display());
        println!("To reinitialize, please delete the existing config first.");
        return Ok(());
    }

    ConfigManager::init().await?;
    println!("✓ Initialized config at: {}", config_path.display());
    Ok(())
}

async fn add_project(path: PathBuf, name: Option<String>, id: Option<String>) -> Result<()> {
    let mut manager = match ConfigManager::load().await {
        Ok(m) => m,
        Err(_) => {
            println!("Config not found. Initializing...");
            ConfigManager::init().await?
        }
    };

    // Resolve path to absolute
    let abs_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(&path)
    };

    // Auto-generate ID from directory name if not provided
    let project_id = id.unwrap_or_else(|| {
        abs_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_lowercase().replace(|c: char| !c.is_alphanumeric() && c != '-', "-"))
            .unwrap_or_else(|| "project".to_string())
    });

    // Auto-generate name from directory name if not provided
    let project_name = name.unwrap_or_else(|| {
        abs_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Project".to_string())
    });

    let project = Project {
        id: project_id.clone(),
        name: project_name,
        path: abs_path.clone(),
        enabled: true,
        last_checked: None,
        settings: None,
        workspace_members: None,
    };

    manager.add_project(project).await
        .with_context(|| format!("Failed to add project '{}'", project_id))?;

    println!("✓ Added project: {}", project_id);
    println!("  Name: {}", manager.get_project(&project_id).unwrap().name);
    println!("  Path: {}", abs_path.display());
    Ok(())
}

async fn list_projects(enabled_only: bool) -> Result<()> {
    let manager = ConfigManager::load().await
        .context("Config not found. Run 'danny config init' first.")?;

    let projects = if enabled_only {
        manager.list_enabled_projects()
    } else {
        manager.list_projects().iter().collect()
    };

    if projects.is_empty() {
        println!("No projects configured.");
        println!("Add a project with: danny config add <path>");
        return Ok(());
    }

    println!("\nConfigured Projects:");
    println!("{}", "=".repeat(60));

    for project in &projects {
        let status = if project.enabled { "✓" } else { "✗" };
        println!("\n{} {} ({})", status, project.name, project.id);
        println!("  Path: {}", project.path.display());

        if let Some(last_checked) = project.last_checked {
            println!("  Last checked: {}", last_checked.format("%Y-%m-%d %H:%M:%S"));
        }

        if let Some(settings) = &project.settings {
            if settings.fetch_changelogs.is_some() || settings.include_dev_deps.is_some() {
                println!("  Settings: custom overrides");
            }
        }
    }

    println!("\nTotal: {} project(s)", projects.len());
    Ok(())
}

async fn enable_project(project_id: String) -> Result<()> {
    let mut manager = ConfigManager::load().await
        .context("Config not found. Run 'danny config init' first.")?;

    manager.enable_project(&project_id).await
        .with_context(|| format!("Failed to enable project '{}'", project_id))?;

    println!("✓ Enabled project: {}", project_id);
    Ok(())
}

async fn disable_project(project_id: String) -> Result<()> {
    let mut manager = ConfigManager::load().await
        .context("Config not found. Run 'danny config init' first.")?;

    manager.disable_project(&project_id).await
        .with_context(|| format!("Failed to disable project '{}'", project_id))?;

    println!("✓ Disabled project: {}", project_id);
    Ok(())
}

async fn remove_project(project_id: String, skip_confirm: bool) -> Result<()> {
    let mut manager = ConfigManager::load().await
        .context("Config not found. Run 'danny config init' first.")?;

    // Check if project exists
    let project = manager.get_project(&project_id)
        .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project_id))?;

    // Confirm removal unless --yes flag is provided
    if !skip_confirm {
        println!("Remove project '{}'?", project.name);
        println!("  ID: {}", project.id);
        println!("  Path: {}", project.path.display());
        print!("\nContinue? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    manager.remove_project(&project_id).await
        .with_context(|| format!("Failed to remove project '{}'", project_id))?;

    println!("✓ Removed project: {}", project_id);
    Ok(())
}

fn show_config_path() -> Result<()> {
    let config_path = ConfigManager::config_path()?;
    println!("{}", config_path.display());
    Ok(())
}

async fn validate_config() -> Result<()> {
    let manager = ConfigManager::load().await
        .context("Config not found or invalid. Run 'danny config init' first.")?;

    let config = manager.config();

    println!("✓ Config is valid");
    println!("  Version: {}", config.version);
    println!("  Projects: {}", config.projects.len());
    println!("  Enabled: {}", config.projects.iter().filter(|p| p.enabled).count());

    // Validate each project path
    let mut invalid_paths = Vec::new();
    for project in &config.projects {
        if !project.path.exists() {
            invalid_paths.push(&project.id);
        }
    }

    if !invalid_paths.is_empty() {
        println!("\nWarning: Some project paths no longer exist:");
        for id in invalid_paths {
            println!("  - {}", id);
        }
    }

    Ok(())
}
