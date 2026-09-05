use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::sync;

pub fn run(kb_root: Option<&Path>, link: bool, no_link: bool, json: bool) -> Result<()> {
    let do_link = link || !no_link; // default: re-link unless --no-link

    let result = sync::pull(kb_root, do_link)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "git_pull": {
                    "already_up_to_date": result.git_pull.already_up_to_date,
                    "output": result.git_pull.output.trim(),
                },
                "linked_projects": result.linked_projects,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {}", "Pulling knowledge-base...".bold().cyan());
        println!();

        if result.git_pull.already_up_to_date {
            println!("  Git:   {}", "already up-to-date".dimmed());
        } else {
            println!("  Git:   {}", "pulled".green());
            for line in result.git_pull.output.lines().take(5) {
                println!("         {}", line.dimmed());
            }
        }

        if do_link && !result.linked_projects.is_empty() {
            println!();
            println!("  Re-linked {} active project(s):", result.linked_projects.len());
            for name in &result.linked_projects {
                println!("    {} {}", "✓".green(), name.bold());
            }
        }

        println!();
    }

    Ok(())
}
