use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::sync;

pub fn run(kb_root: Option<&Path>, project: &str, output: Option<&Path>, json: bool) -> Result<()> {
    let dest = sync::export_project(kb_root, project, output)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "project": project,
                "tarball": dest,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} {}", "Exported".bold().green(), project.bold());
        println!("  {}", dest.display());
        println!();
        println!(
            "  {} Copy to another machine and run: kb import {}",
            "To sync:".dimmed(),
            dest.display()
        );
        println!();
    }

    Ok(())
}
