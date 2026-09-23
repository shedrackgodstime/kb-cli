use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::discovery;
use kb_core::git;

pub fn run(kb_root: Option<&Path>, message: Option<&str>, json: bool, _quiet: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    let output_dir = root.join("archive");
    std::fs::create_dir_all(&output_dir)?;

    let timestamp = chrono::Local::now();
    let msg = message
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("KB snapshot {}", timestamp.format("%Y-%m-%d %H:%M:%S")));

    git::run(&root, ["commit", "--allow-empty", "-m", &msg])?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "commit": git::checked(&root, ["rev-parse", "HEAD"], "snapshot: get new HEAD")?,
                "message": msg,
                "timestamp": timestamp.to_rfc3339(),
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} {}", "Snapshot".bold().green(), "created.".bold());
        println!("  {}", msg);
        println!();
    }

    Ok(())
}
