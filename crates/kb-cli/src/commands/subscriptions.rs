use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::subscription::{self, SubscriptionResult};

pub fn subscribe(
    kb_root: Option<&Path>,
    project: &str,
    dry_run: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    render(
        subscription::subscribe(kb_root, project, dry_run)?,
        "Subscribed to",
        json,
        quiet,
    )
}

pub fn unsubscribe(
    kb_root: Option<&Path>,
    project: &str,
    dry_run: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    render(
        subscription::unsubscribe(kb_root, project, dry_run)?,
        "Unsubscribed from",
        json,
        quiet,
    )
}

fn render(result: SubscriptionResult, heading: &str, json: bool, quiet: bool) -> Result<()> {
    if json {
        return print_json(&result);
    }

    println!();
    println!("  {} {}", heading.dimmed(), result.project.bold());

    if result.dry_run {
        println!("  {}", "(dry run) nothing changed".dimmed());
    } else if result.conversion {
        println!(
            "  {}",
            "Sparse-checkout enabled: this device now keeps only subscribed projects.".dimmed()
        );
    }

    if !result.dropped.is_empty() {
        println!(
            "  {}",
            format!("Dropped from working tree: {}", result.dropped.join(", ")).dimmed()
        );
    }

    if result.subscribed.is_empty() {
        println!(
            "  {}",
            "No project subscriptions left on this device.".dimmed()
        );
    } else if result.subscribed.len() == 1 {
        println!("  {} subscription\n", 1.to_string().cyan());
    } else {
        println!(
            "  {} subscriptions\n",
            result.subscribed.len().to_string().cyan()
        );
    }

    Ok(())
}

fn print_json(result: &SubscriptionResult) -> Result<()> {
    let output = serde_json::json!({
        "ok": true,
        "data": {
            "project": result.project,
            "subscribed": result.subscribed,
            "sparse_enabled": result.sparse_enabled,
            "dropped": result.dropped,
            "conversion": result.conversion,
            "dry_run": result.dry_run,
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
