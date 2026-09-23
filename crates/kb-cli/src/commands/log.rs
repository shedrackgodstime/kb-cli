use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{log, paths};

pub fn run(
    kb_root: Option<&Path>,
    limit: usize,
    stat: bool,
    projects: &[String],
    json: bool,
    quiet: bool,
) -> Result<()> {
    let opts = log::LogOptions {
        limit,
        stat,
        projects: projects.to_vec(),
    };
    let result = log::log(kb_root, &opts)?;

    if json {
        let entries: Vec<_> = result
            .entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "hash": e.hash,
                    "subject": e.subject,
                })
            })
            .collect();

        let output = serde_json::json!({
            "ok": true,
            "data": {
                "root": paths::normalize_display(&result.root),
                "count": entries.len(),
                "limit": opts.limit,
                "projects": opts.projects,
                "stat": result.stat,
                "entries": entries,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if result.entries.is_empty() {
        if !quiet {
            println!();
            println!("  {}", "No commits found.".dimmed());
            println!();
        }
        return Ok(());
    }

    let scope = if projects.is_empty() {
        "knowledge-base".to_string()
    } else {
        format!("project(s) {}", projects.join(", "))
    };

    if !quiet {
        println!();
        println!("  {} {}", "Recent commits in".dimmed(), scope.bold());
        println!();

        if result.stat {
            for line in result.raw.lines() {
                println!("  {line}");
            }
        } else {
            for entry in &result.entries {
                println!("  {} {}", entry.hash.yellow().bold(), entry.subject);
            }
        }
        println!();
    }

    Ok(())
}
