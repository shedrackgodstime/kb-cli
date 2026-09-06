use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use kb_core::{discovery, paths, project};

pub fn run(kb_root: Option<&Path>, project_input: &str, json: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // Resolve project name and repo path
    let (project_name, repo_dir) = resolve_project(project_input)?;

    let result =
        project::unlink(&root, &project_name, &repo_dir).context("failed to unlink project")?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "project_name": result.project_name,
                "scratch_removed": result.scratch_removed,
                "rules_removed": result.rules_removed,
                "kb_rules_removed": result.kb_rules_removed,
                "memory_kept_at": root.join("projects").join(&project_name),
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} {}", "Unlinking".bold().yellow(), project_name.bold());

        if result.scratch_removed {
            println!("  scratch      {}", "removed".red());
        } else {
            println!("  scratch      {}", "not a symlink".dimmed());
        }

        if result.rules_removed {
            println!("  .agent-rules {}", "removed".red());
        } else {
            println!("  .agent-rules {}", "not a symlink".dimmed());
        }

        if result.kb_rules_removed {
            println!("  kb-rules.md  {}", "removed".red());
        } else {
            println!("  kb-rules.md  {}", "not present".dimmed());
        }

        println!();
        println!(
            "  {} Project memory kept at {}",
            "Note:".dimmed(),
            root.join("projects").join(&project_name).display()
        );
        println!();
    }

    Ok(())
}

fn resolve_project(input: &str) -> Result<(String, std::path::PathBuf)> {
    let expanded = paths::expand_home(std::path::Path::new(input))?;

    if expanded.exists() && expanded.is_dir() {
        let name = expanded
            .file_name()
            .context("cannot determine project name from path")?
            .to_string_lossy()
            .to_string();
        return Ok((name, expanded));
    }

    if input.contains('/') || input.contains('\\') {
        anyhow::bail!("project path does not exist: {}", expanded.display());
    }

    let name = input.to_string();
    let repo_dir = paths::default_project_dir(&name)?;
    Ok((name, repo_dir))
}
