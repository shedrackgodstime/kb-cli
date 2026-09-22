use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{open, paths};

pub fn run(kb_root: Option<&Path>, project: Option<&str>, json: bool, quiet: bool) -> Result<()> {
    let dir = open::open(kb_root, project)?;
    let display = paths::normalize_display(&dir);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": {
                    "path": display,
                }
            }))?
        );
        return Ok(());
    }

    println!("  {}", format!("Opened {display}").dimmed());
    Ok(())
}
