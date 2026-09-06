use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::sync;

pub fn run(
    kb_root: Option<&Path>,
    message: Option<&str>,
    link: bool,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    let result = sync::global_sync(kb_root, message, link, dry_run)?;

    if let Some(conflict) = &result.conflict {
        if json {
            let output = serde_json::json!({
                "ok": false,
                "error": "diverged",
                "message": conflict,
                "data": {
                    "pulled": false,
                    "committed": false,
                    "pushed": false,
                }
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!();
            println!(
                "  {} {}",
                "sync skipped".red().bold(),
                "— local and remote diverged".bold()
            );
            println!();
            println!("  {}", conflict.replace("\n", "\n  "));
            println!();
        }
        return Ok(());
    }

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "dry_run": result.dry_run,
                "pulled": result.pulled,
                "already_up_to_date": result.already_up_to_date,
                "committed": result.committed,
                "commit_message": result.commit_message,
                "pushed": result.pushed,
                "files_changed": result.files_changed,
                "linked_projects": result.linked_projects,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!(
            "  {} {}",
            "Global sync".bold().cyan(),
            "- pull if safe, push everything".dimmed()
        );
        println!();

        if result.dry_run {
            println!("  {} (preview — nothing changed)", "dry-run".yellow());
        }

        if result.pulled {
            println!("  Pull:   {}", "fast-forwarded".green());
        } else if result.already_up_to_date {
            println!("  Pull:   {}", "already up-to-date".dimmed());
        }

        if result.committed {
            println!(
                "  Commit: {}",
                result.commit_message.as_deref().unwrap_or("").bold()
            );
            println!("  Files:  {}", result.files_changed.len());
        } else if result.dry_run && !result.files_changed.is_empty() {
            println!(
                "  Commit: {} (would commit {})",
                "pending".yellow(),
                result.files_changed.len()
            );
        } else {
            println!("  Commit: {}", "nothing to commit".dimmed());
        }

        if result.pushed {
            println!("  Push:   {}", "✓ done".green());
        } else if result.dry_run && !result.files_changed.is_empty() {
            println!("  Push:   {}", "pending".yellow());
        }

        if !result.linked_projects.is_empty() {
            println!();
            println!("  Re-linked {} project(s):", result.linked_projects.len());
            for name in &result.linked_projects {
                println!("    {} {}", "✓".green(), name.bold());
            }
        }

        println!();
    }

    Ok(())
}
