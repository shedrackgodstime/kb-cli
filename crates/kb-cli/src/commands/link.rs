use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use kb_core::{discovery, paths, project};

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
                "gitignore_updated": result.gitignore_updated,
                "agents_md_created": result.agents_md_created,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!(
            "  {} {}",
            "Linking".bold().green(),
            project_name.bold()
        );
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

        if result.gitignore_updated {
            println!("  .gitignore   {} (updated)", "✓".green());
        } else {
            println!("  .gitignore   {} protected", "✓".green());
        }

        if result.agents_md_created {
            println!("  AGENTS.md    {} created", "✓".green());
        } else {
            println!("  AGENTS.md    {} exists", "✓".green());
        }

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

fn resolve_project(input: &str) -> Result<(String, std::path::PathBuf)> {
    let expanded = paths::expand_home(std::path::Path::new(input))?;

    if expanded.exists() && expanded.is_dir() {
        let name = expanded
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| input.to_string());

        // Handle "." and ".." — resolve to actual directory name
        let name = if name == "." || name == ".." {
            std::env::current_dir()
                .context("cannot determine current directory")?
                .file_name()
                .context("cannot determine current directory name")?
                .to_string_lossy()
                .to_string()
        } else {
            name
        };

        return Ok((name, expanded));
    }

    if input.contains('/') || input.contains('\\') {
        if expanded.exists() {
            let name = expanded
                .file_name()
                .context("cannot determine project name from path")?
                .to_string_lossy()
                .to_string();
            return Ok((name, expanded));
        }
        anyhow::bail!("project path does not exist: {}", expanded.display());
    }

    let name = input.to_string();
    let repo_dir = paths::default_project_dir(&name)?;
    if !repo_dir.exists() {
        anyhow::bail!(
            "project repo not found at {}.\n\
             Pass the full path instead: kb link /path/to/{}",
            repo_dir.display(),
            name
        );
    }
    Ok((name, repo_dir))
}
