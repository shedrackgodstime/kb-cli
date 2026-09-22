use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use kb_core::{archive, discovery};

pub fn run(
    kb_root: Option<&Path>,
    project_input: &str,
    restore: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let _ = quiet;
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // Archive takes the project *name*: unlike link/unlink it must work for
    // memory-only projects whose repo is not on this machine.
    let project_name = project_input.to_string();

    let result = archive::archive_project(&root, &project_name, restore).context(if restore {
        "failed to restore project"
    } else {
        "failed to archive project"
    })?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "project": result.project_name,
                "restored": result.restored,
                "memory_from": result.memory_from,
                "memory_to": result.memory_to,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if result.restored {
        println!();
        println!(
            "  {} {}",
            "Restored".bold().green(),
            result.project_name.bold()
        );
        println!(
            "  {} Project memory back at {}",
            "Note:".dimmed(),
            result.memory_to.display()
        );
        println!(
            "  {} Run `kb link {}` to wire it up.",
            "Next:".dimmed(),
            result.project_name
        );
        println!();
    } else {
        println!();
        println!(
            "  {} {}",
            "Archived".bold().yellow(),
            result.project_name.bold()
        );
        println!(
            "  {} Project memory moved to {}",
            "Note:".dimmed(),
            result.memory_to.display()
        );

        if let Some(unlinked) = &result.unlinked {
            if unlinked.scratch_removed {
                println!("  scratch      {}", "removed".red());
            } else {
                println!("  scratch      {}", "not a symlink".dimmed());
            }
            if unlinked.rules_removed {
                println!("  .agent-rules {}", "removed".red());
            } else {
                println!("  .agent-rules {}", "not a symlink".dimmed());
            }
            if unlinked.kb_rules_removed {
                println!("  kb-rules.md  {}", "removed".red());
            } else {
                println!("  kb-rules.md  {}", "not present".dimmed());
            }
        }

        println!(
            "  {} Restore with `kb archive --restore {}`.",
            "Undo:".dimmed(),
            result.project_name
        );
        println!();
    }

    Ok(())
}
