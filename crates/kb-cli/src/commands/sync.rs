use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::sync;

pub fn run(kb_root: Option<&Path>, projects: &[String], json: bool) -> Result<()> {
    let result = sync::sync(kb_root, projects)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "git_pull": {
                    "already_up_to_date": result.git_pull.already_up_to_date,
                    "output": result.git_pull.output.trim(),
                },
                "rebase_ok": result.rebase_ok,
                "linked_projects": result.linked_projects,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {}", "Syncing knowledge-base...".bold().cyan());
        println!();

        if result.git_pull.already_up_to_date {
            println!("  Git:   {}", "already up-to-date".dimmed());
        } else if result.rebase_ok {
            println!("  Git:   {}", "pulled + rebased".green());
        }

        if !result.linked_projects.is_empty() {
            println!();
            println!(
                "  Re-linked {} project(s):",
                result.linked_projects.len()
            );
            for name in &result.linked_projects {
                println!("    {} {}", "✓".green(), name.bold());
            }
        }

        println!();
    }

    Ok(())
}
