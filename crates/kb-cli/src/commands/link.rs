use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use kb_core::{discovery, project};

use super::resolve_project;

pub fn run(kb_root: Option<&Path>, project_input: &str, json: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let templates_dir = root.join("templates").join("project");

    // Resolve project name and repo path
    let (project_name, repo_dir) = resolve_project(project_input)?;

    let result = project::link(&root, &project_name, &repo_dir, &templates_dir)
        .context("failed to link project")?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "project_name": result.project_name,
                "memory_dir": result.memory_dir,
                "scratch_link": result.scratch_link,
                "rules_link": result.rules_link,
                "global_gitignore_updated": result.global_gitignore_updated,
                "kb_rules_written": result.kb_rules_written,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} {}", "Linking".bold().green(), project_name.bold());
        println!("  Project repo:   {}", repo_dir.display());
        println!("  Project memory: {}", result.memory_dir.display());
        println!();
        println!(
            "  scratch      {} {}",
            "→".dimmed(),
            result.memory_dir.display()
        );
        println!(
            "  .agent-rules {} {}",
            "→".dimmed(),
            root.join("agent-rules").display()
        );

        if result.global_gitignore_updated {
            println!("  ~/.gitignore  {} updated", "✓".green());
        } else {
            println!("  ~/.gitignore  {} already configured", "✓".green());
        }

        if result.kb_rules_written {
            println!(
                "  kb-rules.md   {} written (attach it in prompts)",
                "✓".green()
            );
        } else {
            println!("  kb-rules.md   {} already present", "✓".green());
        }

        println!();
        println!(
            "  {} Shared files (.gitignore, AGENTS.md) were NOT modified.",
            "ℹ".blue()
        );
        println!("  Symlinks are ignored via your global ~/.gitignore.");

        // Check if registered in INDEX.md
        let index_path = root.join("INDEX.md");
        if index_path.exists() {
            let index_content = std::fs::read_to_string(&index_path)?;
            if !index_content.contains(&format!("`{}`", project_name)) {
                println!();
                println!(
                    "  {} Remember to register '{}' in INDEX.md under ## Projects.",
                    "!".yellow().bold(),
                    project_name
                );
            }
        }

        println!();
    }

    Ok(())
}
