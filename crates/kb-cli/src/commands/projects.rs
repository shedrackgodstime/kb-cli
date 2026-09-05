use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{config, discovery, project};

pub fn run(kb_root: Option<&Path>, active_only: bool, verbose: bool, json: bool) -> Result<()> {
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

        println!(
            "  {:<20} {:<15} {:<10} {:<8} {}",
            "Name".bold(),
            "Status".bold(),
            "Active".bold(),
            "Handoff".bold(),
            if verbose { "Memory Path" } else { "" }.bold()
        );
        println!("  {}", "─".repeat(80).dimmed());

        for status in &display {
            let is_active = cfg.active_projects.contains(&status.name);
            let status_str = if status.memory_exists { "active" } else { "missing" };
            let handoff_str = status.handoff_age.as_deref().unwrap_or("—");

            let extra = if verbose {
                status.memory_path.display().to_string()
            } else {
                String::new()
            };

            println!(
                "  {:<20} {:<15} {:<10} {:<8} {}",
                status.name.bold(),
                status_str,
                if is_active {
                    "yes".green()
                } else {
                    "no".dimmed()
                },
                handoff_str,
                extra.dimmed(),
            );
        }

        println!();
    }

    Ok(())
}
