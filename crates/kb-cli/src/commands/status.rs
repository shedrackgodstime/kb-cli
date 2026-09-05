use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{config, discovery, platform, project};

pub fn run(kb_root: Option<&Path>, all: bool, _refs: bool, json: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let cfg = config::load()?;
    let platform_info = platform::detect_platform();

    let projects = project::list_all(&root)?;

    let display_projects = if all {
        projects
    } else {
        projects
            .iter()
            .filter(|p| cfg.active_projects.contains(&p.name))
            .cloned()
            .collect()
    };

    if json {
        let items: Vec<_> = display_projects
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "active": cfg.active_projects.contains(&p.name),
                    "memory_exists": p.memory_exists,
                    "memory_path": p.memory_path,
                    "repo_path": p.repo_path,
                    "scratch_healthy": p.scratch_healthy,
                    "rules_healthy": p.rules_healthy,
                    "handoff_age": p.handoff_age,
                })
            })
            .collect();

        let output = serde_json::json!({
            "ok": true,
            "data": {
                "kb_root": root,
                "config_path": "~/.kb/config.toml",
                "os": platform_info.os,
                "projects": items,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {}", "Knowledge Base Status".bold().cyan());
        println!();
        println!("  Root:   {}", root.display());
        println!("  Config: ~/.kb/config.toml");
        println!("  OS:     {}", platform_info.os);
        println!();

        if display_projects.is_empty() {
            println!("  {}", "No projects found.".dimmed());
            println!(
                "  {}",
                "Run `kb link <project>` to wire a project.".dimmed()
            );
            println!();
            return Ok(());
        }

        println!("  {}", "Active Projects".bold());
        println!();

        let mut warnings = vec![];

        for status in &display_projects {
            let is_active = cfg.active_projects.contains(&status.name);
            let active_marker = if is_active { "" } else { " (inactive)" };

            println!("  {}{}", status.name.bold(), active_marker.dimmed());

            if status.memory_exists {
                println!(
                    "    memory:       {} {}",
                    "✓".green(),
                    status.memory_path.display()
                );
            } else {
                println!("    memory:       {} missing", "✗".red());
                continue;
            }

            match status.scratch_healthy {
                Some(true) => println!("    scratch:      {} healthy", "✓".green()),
                Some(false) => {
                    println!("    scratch:      {} broken or missing", "✗".red());
                    warnings.push(format!("{}: scratch symlink broken", status.name));
                }
                None => println!("    scratch:      {}", "repo not found".dimmed()),
            }

            match status.rules_healthy {
                Some(true) => println!("    .agent-rules: {} healthy", "✓".green()),
                Some(false) => {
                    println!("    .agent-rules: {} broken or missing", "✗".red());
                    warnings.push(format!("{}: .agent-rules symlink broken", status.name));
                }
                None => println!("    .agent-rules: {}", "repo not found".dimmed()),
            }

            if let Some(age) = &status.handoff_age {
                let stale = match age.as_str() {
                    "today" => false,
                    "1 day ago" => false,
                    other => other
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(|d| d > 14)
                        .unwrap_or(false),
                };

                if stale {
                    println!("    handoff:      {} {} ⚠", age, "(stale)".yellow());
                    warnings.push(format!("{}: handoff is {}", status.name, age));
                } else {
                    println!("    handoff:      {}", age);
                }
            }

            println!();
        }

        if !warnings.is_empty() {
            println!("  {}", "Warnings".bold().yellow());
            for w in &warnings {
                println!("  {} {}", "⚠".yellow(), w);
            }
            println!();
        }
    }

    Ok(())
}
