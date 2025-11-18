use danny_feed::types::{InternalDependency, PackageFeed, SharedDependency, WorkspaceMemberFeed};
use std::collections::HashMap;
use std::io::{self, Write};

/// Box-drawing characters for tree display
mod box_chars {
    pub const VERTICAL: &str = "│";
    pub const BRANCH: &str = "├─";
    pub const LAST_BRANCH: &str = "└─";
    pub const SPACE: &str = "  ";
}

/// Configuration for monorepo display
#[derive(Debug, Clone)]
pub struct MonorepoDisplayConfig {
    pub show_shared_deps: bool,
    pub show_internal_deps: bool,
    pub show_member_details: bool,
    pub compact_mode: bool,
}

impl Default for MonorepoDisplayConfig {
    fn default() -> Self {
        Self {
            show_shared_deps: true,
            show_internal_deps: true,
            show_member_details: true,
            compact_mode: false,
        }
    }
}

/// Print a monorepo feed to the terminal with tree-style formatting
pub fn print_monorepo_feed<W: Write>(
    writer: &mut W,
    feed: &PackageFeed,
    config: &MonorepoDisplayConfig,
) -> io::Result<()> {
    // Header
    print_header(writer, feed)?;
    writeln!(writer)?;

    // Workspace Summary
    print_workspace_summary(writer, feed)?;
    writeln!(writer)?;

    // Shared Dependencies Section
    if config.show_shared_deps && !feed.shared_dependencies.is_empty() {
        print_shared_dependencies(writer, &feed.shared_dependencies)?;
        writeln!(writer)?;
    }

    // Internal Dependencies Section
    if config.show_internal_deps && !feed.internal_dependencies.is_empty() {
        print_internal_dependencies(writer, &feed.internal_dependencies)?;
        writeln!(writer)?;
    }

    // Workspace Members
    if config.show_member_details {
        print_workspace_members(writer, &feed.workspace_members, config.compact_mode)?;
    }

    Ok(())
}

fn print_header<W: Write>(writer: &mut W, feed: &PackageFeed) -> io::Result<()> {
    writeln!(
        writer,
        "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    )?;
    writeln!(writer, "┃ 📦 {} (Monorepo)", feed.project_name)?;
    writeln!(
        writer,
        "┃ Ecosystem: {:?} │ Members: {} │ Updated: {}",
        feed.ecosystem,
        feed.workspace_members.len(),
        feed.generated_at.format("%Y-%m-%d %H:%M UTC")
    )?;
    writeln!(
        writer,
        "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    )?;

    Ok(())
}

fn print_workspace_summary<W: Write>(writer: &mut W, feed: &PackageFeed) -> io::Result<()> {
    writeln!(writer, "╔═══ Workspace Summary ═══╗")?;
    writeln!(
        writer,
        "║ Members:              {} ║",
        pad_number(feed.workspace_members.len())
    )?;
    writeln!(
        writer,
        "║ Shared Dependencies:  {} ║",
        pad_number(feed.shared_dependencies.len())
    )?;
    writeln!(
        writer,
        "║ Internal Dependencies: {} ║",
        pad_number(feed.internal_dependencies.len())
    )?;
    writeln!(writer, "╚═════════════════════════╝")?;

    Ok(())
}

fn print_shared_dependencies<W: Write>(
    writer: &mut W,
    dependencies: &[SharedDependency],
) -> io::Result<()> {
    writeln!(
        writer,
        "🔗 Shared Dependencies ({}):",
        dependencies.len()
    )?;
    writeln!(writer)?;

    for (idx, dep) in dependencies.iter().enumerate() {
        let is_last = idx == dependencies.len() - 1;
        let prefix = if is_last {
            box_chars::LAST_BRANCH
        } else {
            box_chars::BRANCH
        };
        let continuation = if is_last {
            box_chars::SPACE
        } else {
            box_chars::VERTICAL
        };

        let usage_count = dep.used_by_members.len();
        let update_indicator = if dep.package.current_version != dep.package.latest_version {
            "📤"
        } else {
            "✓"
        };

        writeln!(
            writer,
            "{} {} {} (used by {} members)",
            prefix,
            update_indicator,
            sanitize_terminal_string(&dep.package.name),
            usage_count
        )?;

        writeln!(
            writer,
            "{}   Current: {} → Latest: {}",
            continuation,
            sanitize_terminal_string(&dep.package.current_version),
            sanitize_terminal_string(&dep.package.latest_version)
        )?;

        // Show which members use this dependency
        let members_display = if usage_count <= 5 {
            dep.used_by_members
                .iter()
                .map(|m| sanitize_terminal_string(m))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            format!(
                "{}, ... and {} more",
                dep.used_by_members[..3]
                    .iter()
                    .map(|m| sanitize_terminal_string(m))
                    .collect::<Vec<_>>()
                    .join(", "),
                usage_count - 3
            )
        };

        writeln!(writer, "{}   Used by: {}", continuation, members_display)?;

        if !is_last {
            writeln!(writer, "{}", continuation)?;
        }
    }

    Ok(())
}

fn print_internal_dependencies<W: Write>(
    writer: &mut W,
    dependencies: &[InternalDependency],
) -> io::Result<()> {
    writeln!(
        writer,
        "🔄 Internal Dependencies ({}):",
        dependencies.len()
    )?;
    writeln!(writer)?;

    // Group by from_member
    let mut grouped: HashMap<String, Vec<&InternalDependency>> = HashMap::new();

    for dep in dependencies {
        grouped
            .entry(dep.from_member.clone())
            .or_default()
            .push(dep);
    }

    let mut sorted_members: Vec<_> = grouped.keys().cloned().collect();
    sorted_members.sort();

    for (member_idx, from_member) in sorted_members.iter().enumerate() {
        let is_last_member = member_idx == sorted_members.len() - 1;
        let member_prefix = if is_last_member {
            box_chars::LAST_BRANCH
        } else {
            box_chars::BRANCH
        };
        let member_continuation = if is_last_member {
            box_chars::SPACE
        } else {
            box_chars::VERTICAL
        };

        writeln!(
            writer,
            "{} 📄 {}",
            member_prefix,
            sanitize_terminal_string(from_member)
        )?;

        let deps = grouped.get(from_member).unwrap();
        for (dep_idx, dep) in deps.iter().enumerate() {
            let is_last_dep = dep_idx == deps.len() - 1;
            let dep_prefix = if is_last_dep {
                box_chars::LAST_BRANCH
            } else {
                box_chars::BRANCH
            };

            writeln!(
                writer,
                "{}  {} → {} ({})",
                member_continuation,
                dep_prefix,
                sanitize_terminal_string(&dep.to_member),
                sanitize_terminal_string(&dep.version_spec)
            )?;
        }

        if !is_last_member {
            writeln!(writer, "{}", box_chars::VERTICAL)?;
        }
    }

    Ok(())
}

fn print_workspace_members<W: Write>(
    writer: &mut W,
    members: &[WorkspaceMemberFeed],
    compact_mode: bool,
) -> io::Result<()> {
    writeln!(writer, "📁 Workspace Members ({}):", members.len())?;
    writeln!(writer)?;

    for (idx, member) in members.iter().enumerate() {
        let is_last = idx == members.len() - 1;
        print_workspace_member(writer, member, is_last, compact_mode)?;

        if !is_last && !compact_mode {
            writeln!(writer)?;
        }
    }

    Ok(())
}

fn print_workspace_member<W: Write>(
    writer: &mut W,
    member: &WorkspaceMemberFeed,
    is_last: bool,
    compact_mode: bool,
) -> io::Result<()> {
    let prefix = if is_last {
        box_chars::LAST_BRANCH
    } else {
        box_chars::BRANCH
    };
    let continuation = if is_last {
        box_chars::SPACE
    } else {
        box_chars::VERTICAL
    };

    writeln!(
        writer,
        "{} 📄 {} ({} dependencies)",
        prefix,
        sanitize_terminal_string(&member.name),
        member.feed.packages.len()
    )?;

    if compact_mode {
        return Ok(());
    }

    if member.feed.packages.is_empty() {
        writeln!(writer, "{}   (no external dependencies)", continuation)?;
        return Ok(());
    }

    for (pkg_idx, package) in member.feed.packages.iter().enumerate() {
        let is_last_pkg = pkg_idx == member.feed.packages.len() - 1;
        let pkg_prefix = if is_last_pkg {
            box_chars::LAST_BRANCH
        } else {
            box_chars::BRANCH
        };

        let update_indicator = if package.current_version != package.latest_version {
            "📤"
        } else {
            "✓"
        };

        writeln!(
            writer,
            "{}  {} {} {} → {}",
            continuation,
            pkg_prefix,
            update_indicator,
            sanitize_terminal_string(&package.name),
            sanitize_terminal_string(&package.latest_version)
        )?;

        if package.current_version != package.latest_version {
            let pkg_continuation = if is_last_pkg {
                box_chars::SPACE
            } else {
                box_chars::VERTICAL
            };
            writeln!(
                writer,
                "{}  {}   (current: {})",
                continuation,
                pkg_continuation,
                sanitize_terminal_string(&package.current_version)
            )?;
        }
    }

    Ok(())
}

/// Sanitize strings for terminal output (prevent control character injection)
fn sanitize_terminal_string(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// Pad numbers for alignment in summary box
fn pad_number(num: usize) -> String {
    format!("{:>3}", num)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use danny_deps::Ecosystem;
    use danny_feed::types::PackageFeedItem;

    fn create_empty_monorepo() -> PackageFeed {
        PackageFeed {
            project_name: "test-monorepo".to_string(),
            ecosystem: Ecosystem::Rust,
            packages: vec![],
            generated_at: Utc::now(),
            workspace_members: vec![],
            shared_dependencies: vec![],
            internal_dependencies: vec![],
        }
    }

    fn create_test_package(name: &str, current: &str, latest: &str) -> PackageFeedItem {
        PackageFeedItem {
            name: name.to_string(),
            ecosystem: Ecosystem::Rust,
            current_version: current.to_string(),
            latest_version: latest.to_string(),
            recent_updates: vec![],
            has_update: current != latest,
            update_type: None,
            changelog_url: None,
            repository_url: None,
        }
    }

    #[test]
    fn test_print_empty_monorepo() {
        let feed = create_empty_monorepo();
        let mut output = Vec::new();
        let config = MonorepoDisplayConfig::default();

        print_monorepo_feed(&mut output, &feed, &config).unwrap();
        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("📦"));
        assert!(output_str.contains("(Monorepo)"));
        assert!(output_str.contains("Members: 0"));
    }

    #[test]
    fn test_sanitize_terminal_string() {
        let malicious = "normal\x1b[31mtext";
        let sanitized = sanitize_terminal_string(malicious);
        assert!(!sanitized.contains('\x1b'));
        assert_eq!(sanitized, "normal[31mtext");
    }

    #[test]
    fn test_pad_number() {
        assert_eq!(pad_number(5), "  5");
        assert_eq!(pad_number(42), " 42");
        assert_eq!(pad_number(123), "123");
    }
}
