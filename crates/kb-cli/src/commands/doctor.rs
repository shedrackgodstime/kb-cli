use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{discovery, doctor};

pub fn run(kb_root: Option<&Path>, _fix: bool, json: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let report = doctor::run_all(&root)?;

    if json {
        let checks: Vec<_> = report
            .checks
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "severity": format!("{:?}", c.severity).to_lowercase(),
                    "message": c.message,
                    "fix": c.fix,
                })
            })
            .collect();

        let output = serde_json::json!({
            "ok": true,
            "data": {
                "checks": checks,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {}", "Doctor Results".bold().cyan());
        println!();

        let mut pass_count = 0;
        let mut warn_count = 0;
        let mut error_count = 0;

        for check in &report.checks {
            match check.severity {
                doctor::Severity::Pass => {
                    println!("  {} {}", "✓".green().bold(), check.name);
                    pass_count += 1;
                }
                doctor::Severity::Warn => {
                    println!("  {} {}", "⚠".yellow().bold(), check.name);
                    println!("    └ {}", check.message);
                    if let Some(fix) = &check.fix {
                        println!("      fix: {}", fix.dimmed());
                    }
                    warn_count += 1;
                }
                doctor::Severity::Error => {
                    println!("  {} {}", "✗".red().bold(), check.name);
                    println!("    └ {}", check.message);
                    if let Some(fix) = &check.fix {
                        println!("      fix: {}", fix.dimmed());
                    }
                    error_count += 1;
                }
            }
        }

        println!();
        println!(
            "  Summary: {} passed, {} warnings, {} errors",
            pass_count.to_string().green(),
            warn_count.to_string().yellow(),
            error_count.to_string().red(),
        );
        println!();
    }

    Ok(())
}
