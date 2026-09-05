use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::sync;

pub fn run(
    kb_root: Option<&Path>,
    projects: &[String],
    message: Option<&str>,
    json: bool,
) -> Result<()> {
    if projects.is_empty() {
        anyhow::bail!(
            "no projects specified.\n\
             Usage: kb push --project dioxus-auth\n\
             Or:    kb push --project dioxus-auth --project kb"
        );
    }

    let result = sync::push(kb_root, projects, message)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "committed": result.committed,
                "commit_message": result.commit_message,
                "pushed": result.pushed,
                "files_changed": result.files_changed,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();

        if !result.committed {
            println!("  {}", "Nothing to push.".dimmed());
            println!();
            return Ok(());
        }

        println!(
            "  {} {}",
            "Pushed".bold().green(),
            projects.join(", ").bold()
        );
        println!(
            "  commit: {}",
            result.commit_message.unwrap_or_default()
        );
        println!("  files:  {}", result.files_changed.len());
        println!();
    }

    Ok(())
}
