use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{discovery, project, state, sync};

pub fn run(kb_root: Option<&Path>, project_name: &str, json: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // 1. Git sync (pull --rebase)
    let sync_result = sync::sync(kb_root, &[project_name.to_string()])?;

    // 2. Link the project
    let templates_dir = root.join("templates").join("project");
    let repo_dir = default_project_dir(project_name)?;

    if repo_dir.exists() {
        let _ = project::link(&root, project_name, &repo_dir, &templates_dir);
    }

    // 3. Track as in-progress
    state::add_project(project_name)?;

    // 4. Load state for display
    let work_state = state::load()?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "project": project_name,
                "git_synced": !sync_result.git_pull.already_up_to_date || sync_result.rebase_ok,
                "rebase_ok": sync_result.rebase_ok,
                "linked": repo_dir.exists(),
                "in_progress": work_state.active_projects,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} {}", "Working on".bold().cyan(), project_name.bold());
        println!();

        // Git status
        if sync_result.git_pull.already_up_to_date {
            println!("  Git:    {}", "up-to-date".dimmed());
        } else if sync_result.rebase_ok {
            println!("  Git:    {}", "synced".green());
        }

        // Link status
        if repo_dir.exists() {
            println!("  Link:   {} → {}", "✓".green(), repo_dir.display());
        } else {
            println!(
                "  Link:   {} repo not found at {}",
                "!".yellow(),
                repo_dir.display()
            );
        }

        // In-progress list
        if work_state.active_projects.len() == 1 {
            println!("  Active: {}", project_name.dimmed());
        } else {
            println!("  Active:");
            for name in &work_state.active_projects {
                let marker = if name == project_name {
                    format!("{} (just started)", name.bold())
                } else {
                    name.dimmed().to_string()
                };
                println!("    {} {}", "→".cyan(), marker);
            }
        }

        println!();
        println!("  {} When done: kb done", "Tip:".dimmed());
        println!();
    }

    Ok(())
}

fn default_project_dir(name: &str) -> Result<std::path::PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join("Projects").join(name))
}
