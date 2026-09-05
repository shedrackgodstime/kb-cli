use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use kb_core::{discovery, state};

pub fn run(kb_root: Option<&Path>, message: Option<&str>, json: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // 1. Load in-progress projects
    let work_state = state::load()?;

    if work_state.active_projects.is_empty() {
        if json {
            let output = serde_json::json!({
                "ok": true,
                "data": {
                    "nothing_to_do": true,
                    "message": "no projects in progress. Use `kb work <project>` first.",
                }
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!();
            println!("  {}", "Nothing to push.".dimmed());
            println!("  No projects in progress.");
            println!("  Start with: kb work <project>");
            println!();
        }
        return Ok(());
    }

    // 2. Check what's changed
    let status_output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain"])
        .output()
        .context("failed to git status")?;

    let status_str = String::from_utf8_lossy(&status_output.stdout);

    // 3. Stage files for in-progress projects
    let mut files_staged = vec![];

    for line in status_str.lines() {
        let file = line[3..].trim();

        // Always stage shared files
        if file == "INDEX.md"
            || file == "AGENTS.md"
            || file.starts_with("agent-rules/")
            || file.starts_with("templates/")
        {
            files_staged.push(file.to_string());
            continue;
        }

        // Stage files for in-progress projects
        for project_name in &work_state.active_projects {
            let prefix = format!("projects/{}/", project_name);
            if file.starts_with(&prefix) {
                files_staged.push(file.to_string());
                break;
            }
        }
    }

    if files_staged.is_empty() {
        state::clear()?;

        if json {
            let output = serde_json::json!({
                "ok": true,
                "data": {
                    "committed": false,
                    "pushed": false,
                    "projects": work_state.active_projects,
                    "message": "no changes to commit",
                }
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!();
            println!("  {}", "No changes to commit.".dimmed());
            println!("  Projects: {}", work_state.active_projects.join(", "));
            println!();
        }
        return Ok(());
    }

    // 4. Stage files
    for file in &files_staged {
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["add", file])
            .output()
            .context(format!("failed to git add {}", file))?;
    }

    // 5. Commit
    let commit_msg = message.map(|m| m.to_string()).unwrap_or_else(|| {
        let projects = &work_state.active_projects;
        if projects.len() == 1 {
            format!("update {}", projects[0])
        } else {
            format!("update {}", projects.join(", "))
        }
    });

    let commit_output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("failed to git commit")?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        anyhow::bail!("git commit failed: {}", stderr);
    }

    // 6. Push
    let push_output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["push"])
        .output()
        .context("failed to git push")?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        anyhow::bail!("git push failed: {}", stderr);
    }

    // 7. Clear state
    state::clear()?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "committed": true,
                "pushed": true,
                "commit_message": commit_msg,
                "files_changed": files_staged,
                "projects": work_state.active_projects,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!(
            "  {} {}",
            "Done!".bold().green(),
            work_state.active_projects.join(", ").bold()
        );
        println!("  commit: {}", commit_msg);
        println!("  files:  {}", files_staged.len());
        println!("  pushed: {}", "yes".green());
        println!();
    }

    Ok(())
}
