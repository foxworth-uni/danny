use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Args, Subcommand};
use danny_config::ConfigManager;
use danny_feed::{FeedConfig, FeedGenerationEvent, FeedGenerator, FeedView};
use tokio_stream::StreamExt;
use std::io;

#[derive(Args, Debug)]
pub struct FeedCommand {
    /// Project ID to generate feed for (omit to use --all)
    project_id: Option<String>,

    /// Generate feed for all enabled projects
    #[arg(long, conflicts_with = "project_id")]
    all: bool,

    /// Enable streaming output (real-time progress)
    #[arg(long, short = 's')]
    stream: bool,

    #[command(subcommand)]
    view: Option<FeedViewCommand>,
}

#[derive(Subcommand, Debug)]
enum FeedViewCommand {
    /// Show all packages with updates available
    Updates,

    /// Chronological changelog feed (like a social feed)
    Changelogs,

    /// Only packages with breaking changes (major updates)
    Breaking,

    /// Packages sorted by freshness (most recently updated)
    Freshness,

    /// Packages sorted by release velocity
    Velocity,

    /// Only prerelease versions
    Prereleases,

    /// Only minor version updates (new features)
    Minor,

    /// Only patch updates (bug fixes)
    Patches,

    /// All tracked packages (default)
    All,
}

impl FeedViewCommand {
    fn to_view(&self) -> FeedView {
        match self {
            Self::Updates => FeedView::Updates,
            Self::Changelogs => FeedView::ChangelogRiver,
            Self::Breaking => FeedView::BreakingChanges,
            Self::Freshness => FeedView::Freshness,
            Self::Velocity => FeedView::Velocity,
            Self::Prereleases => FeedView::Prereleases,
            Self::Minor => FeedView::MinorUpdates,
            Self::Patches => FeedView::PatchParadise,
            Self::All => FeedView::All,
        }
    }
}

pub fn handle_feed_command(cmd: FeedCommand) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    runtime.block_on(async_handle_feed_command(cmd))
}

async fn async_handle_feed_command(cmd: FeedCommand) -> Result<()> {
    let mut manager = ConfigManager::load().await
        .context("Config not found. Run 'danny config init' first.")?;

    let projects = if cmd.all {
        manager.list_enabled_projects().iter().map(|p| (*p).clone()).collect()
    } else if let Some(project_id) = &cmd.project_id {
        let project = manager
            .get_project(project_id)
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project_id))?
            .clone();

        if !project.enabled {
            eprintln!("Warning: Project '{}' is disabled", project_id);
        }

        vec![project]
    } else {
        anyhow::bail!("Please specify a project ID or use --all");
    };

    if projects.is_empty() {
        println!("No enabled projects found.");
        println!("Add a project with: danny config add <path>");
        return Ok(());
    }

    let view = cmd.view.as_ref().map(|v| v.to_view()).unwrap_or(FeedView::Updates);

    // Generate feeds for each project
    for project in &projects {
        println!("\n{} ({})", project.name, project.id);
        println!("  Path: {}", project.path.display());

        let config = FeedConfig {
            fetch_changelogs: project
                .settings
                .as_ref()
                .and_then(|s| s.fetch_changelogs)
                .unwrap_or(manager.config().settings.fetch_changelogs),
            include_dev_deps: project
                .settings
                .as_ref()
                .and_then(|s| s.include_dev_deps)
                .unwrap_or(manager.config().settings.include_dev_deps),
            registry_only: project
                .settings
                .as_ref()
                .and_then(|s| s.registry_only)
                .unwrap_or(manager.config().settings.registry_only),
            include_workspaces: true,
            max_recent_updates: 10,
            updates_only: false,
            workspace_member_config: project.workspace_members.clone(),
        };

        let generator = FeedGenerator::with_config(config)
            .context("Failed to create feed generator")?;

        if cmd.stream {
            // Streaming mode - real-time progress
            handle_streaming(&generator, &project.path, view).await?;
        } else {
            // Non-streaming mode - wait for completion
            match generator.generate(&project.path).await {
                Ok(feed) => {
                    if let Err(e) = manager.update_last_checked(&project.id, Utc::now()).await {
                        eprintln!("  Warning: Failed to update last_checked: {}", e);
                    }

                    print_feed(&project.name, &feed, view);
                }
                Err(e) => {
                    eprintln!("  Error: {}", e);
                    continue;
                }
            }
        }
    }

    Ok(())
}

async fn handle_streaming(
    generator: &FeedGenerator,
    project_path: &std::path::Path,
    view: FeedView,
) -> Result<()> {
    let mut stream = generator.generate_stream(project_path).await?;
    let mut final_feed = None;

    while let Some(event) = stream.next().await {
        match event {
            FeedGenerationEvent::Started { total_packages } => {
                println!("  Starting analysis of {} packages...", total_packages);
            }
            FeedGenerationEvent::PackageResolved { item, index, total } => {
                let status = if item.has_update {
                    format!("UPDATE: {} -> {}", item.current_version, item.latest_version)
                } else {
                    "up to date".to_string()
                };
                println!("  [{:3}/{}] {} - {}", index + 1, total, item.name, status);
            }
            FeedGenerationEvent::Progress { completed, total, elapsed_ms } => {
                println!("  Progress: {}/{} packages ({}ms)", completed, total, elapsed_ms);
            }
            FeedGenerationEvent::PackageFailed { name, error } => {
                eprintln!("  Failed: {} - {}", name, error);
            }
            FeedGenerationEvent::WorkspaceMemberStarted { name, index, total } => {
                println!("  Workspace member [{}/{}]: {}", index + 1, total, name);
            }
            FeedGenerationEvent::WorkspaceMemberCompleted { name, package_count, .. } => {
                println!("  Completed {} ({} packages)", name, package_count);
            }
            FeedGenerationEvent::Completed { feed, duration_ms } => {
                println!("  Completed in {}ms", duration_ms);
                final_feed = Some(feed);
            }
        }
    }

    if let Some(feed) = final_feed {
        println!("\nFeed Summary:");
        print_feed_summary(&feed, view);
    }

    Ok(())
}

fn print_feed(project_name: &str, feed: &danny_feed::PackageFeed, view: FeedView) {
    // Check if this is a monorepo (has workspace members)
    if !feed.workspace_members.is_empty() {
        // Use monorepo display
        println!();
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let config = crate::display::MonorepoDisplayConfig::default();

        if let Err(e) = crate::display::print_monorepo_feed(&mut handle, feed, &config) {
            eprintln!("  Error displaying monorepo feed: {}", e);
        }
    } else {
        // Use legacy single-project display
        println!("  Generated feed for {}", project_name);
        println!("  Ecosystem: {:?}", feed.ecosystem);
        print_feed_summary(feed, view);
    }
}

fn print_feed_summary(feed: &danny_feed::PackageFeed, view: FeedView) {
    match view {
        FeedView::ChangelogRiver => {
            let items = feed.changelog_river();
            println!("  Changelog entries: {}", items.len());
            println!("\n  Recent Changes:");
            println!("  {}", "-".repeat(70));

            for item in items.iter().take(15) {
                println!("\n  {} v{}", item.package_name, item.version);
                if let Some(date) = item.date {
                    println!("    Released: {}", date.format("%Y-%m-%d"));
                }
                if let Some(summary) = &item.summary {
                    let preview: String = summary.chars().take(120).collect();
                    println!("    {}", preview);
                }
            }
        }
        FeedView::Velocity => {
            let items = feed.velocity_feed();
            println!("  Packages by velocity:");
            println!("  {}", "-".repeat(70));

            for item in items.iter().take(20) {
                println!("  {} - {} ({} releases/year)",
                    item.package_name,
                    item.velocity.description(),
                    item.releases_last_year
                );
            }
        }
        FeedView::Freshness => {
            let items = feed.freshness_feed();
            println!("  Recently updated packages:");
            println!("  {}", "-".repeat(70));

            for item in items.iter().take(20) {
                println!("  {} v{} - {} days ago",
                    item.package_name,
                    item.latest_version,
                    item.days_since_release
                );
            }
        }
        _ => {
            // Standard package list views
            let packages: Vec<_> = feed.view(view).collect();
            println!("  Packages: {}", packages.len());

            if packages.is_empty() {
                println!("  No packages match this view.");
                return;
            }

            println!("\n  Package Updates:");
            println!("  {}", "-".repeat(70));

            for item in packages.iter().take(20) {
                println!("\n  {} {}",
                    if item.has_update { "📦" } else { "✓" },
                    item.name
                );

                if item.has_update {
                    println!("     {} -> {}", item.current_version, item.latest_version);

                    if let Some(update_type) = &item.update_type {
                        println!("     Type: {}", update_type.description());
                    }

                    if let Some(update) = item.latest_update() {
                        if let Some(date) = update.date {
                            println!("     Released: {}", date.format("%Y-%m-%d"));
                        }
                        if let Some(summary) = &update.changelog_summary {
                            let preview: String = summary.chars().take(100).collect();
                            println!("     {}", preview);
                        }
                    }
                } else {
                    println!("     Current: {}", item.current_version);
                }
            }

            if packages.len() > 20 {
                println!("\n  ... and {} more", packages.len() - 20);
            }
        }
    }
}
