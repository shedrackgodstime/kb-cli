use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::sparse;

pub fn run(kb_root: Option<&Path>, project: &str, json: bool) -> Result<()> {
    let result = sparse::subscribe(kb_root, project)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "project": result.project,
                "subscribed": result.subscribed,
                "sparse_enabled": result.sparse_enabled,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!();
    println!("  {} {}", "Subscribed to".dimmed(), result.project.bold());
    if result.sparse_enabled {
        println!(
            "  {}",
            "This device keeps only its subscribed project memory (sparse-checkout).".dimmed()
        );
    }
    if result.subscribed.len() == 1 {
        println!("  {} subscription\n", 1.to_string().cyan());
    } else {
        println!(
            "  {} subscriptions\n",
            result.subscribed.len().to_string().cyan()
        );
    }
    Ok(())
}
