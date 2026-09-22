use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use kb_core::{config, discovery, project, refs};

pub fn run(
    kb_root: Option<&Path>,
    active_only: bool,
    verbose: bool,
    json: bool,
    _quiet: bool,
) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let cfg = config::load()?;

    let all_projects = project::list_all(&root)?;

    let display: Vec<_> = if active_only {
        all_projects
            .iter()
            .filter(|p| cfg.active_projects.contains(&p.name))
            .collect()
    } else {
        all_projects.iter().collect()
    };

    if json {
        let items: Vec<_> = display
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "active": cfg.active_projects.contains(&p.name),
                    "memory_exists": p.memory_exists,
                    "handoff_age": p.handoff_age,
                })
            })
            .collect();

        let output = serde_json::json!({
            "ok": true,
            "data": {
                "projects": items,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {}", "Projects".bold().cyan());
        println!();

        if display.is_empty() {
            println!("  {}", "No projects found.".dimmed());
            println!();
            return Ok(());
        }

        // Header with Refs column
        println!(
            "  {:<20} {:<15} {:<10} {:<6} {:<8} {}",
            "Name".bold(),
            "Status".bold(),
            "Active".bold(),
            "Refs".bold(),
            "Handoff".bold(),
            if verbose { "Disk Usage" } else { "" }.bold()
        );
        println!("  {}", "─".repeat(90).dimmed());

        for status in &display {
            let is_active = cfg.active_projects.contains(&status.name);
            let status_str = if status.memory_exists {
                "active"
            } else {
                "missing"
            };
            let handoff_str = status.handoff_age.as_deref().unwrap_or("—");

            // Count refs for this project
            let refs_count = if status.memory_exists {
                refs::check_refs_status(&root, &status.name)
                    .map(|rs| rs.len())
                    .unwrap_or(0)
            } else {
                0
            };

            let extra = if verbose {
                // Compute disk usage for the project memory
                let usage = if status.memory_exists {
                    compute_disk_usage(&status.memory_path)
                } else {
                    "—".to_string()
                };
                format!("{}  {}", status.memory_path.display(), usage)
            } else {
                String::new()
            };

            println!(
                "  {:<20} {:<15} {:<10} {:<6} {:<8} {}",
                status.name.bold(),
                status_str,
                if is_active {
                    "yes".green()
                } else {
                    "no".dimmed()
                },
                refs_count,
                handoff_str,
                extra.dimmed(),
            );
        }

        println!();
    }

    Ok(())
}

/// Compute human-readable disk usage for a directory.
fn compute_disk_usage(path: &Path) -> String {
    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    if total >= 1_000_000_000 {
        format!("{:.1} GB", total as f64 / 1_000_000_000.0)
    } else if total >= 1_000_000 {
        format!("{:.1} MB", total as f64 / 1_000_000.0)
    } else if total >= 1_000 {
        format!("{:.1} KB", total as f64 / 1_000.0)
    } else {
        format!("{} B", total)
    }
}
