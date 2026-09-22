use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{discovery, doctor};

pub fn run(kb_root: Option<&Path>, fix: bool, json: bool, quiet: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    let before = doctor::run_all(&root)?;

    if !fix {
        print_report(&before.checks, json)?;
        return Ok(());
    }

    let fixed = doctor::fix_all(&root)?;
    let after = doctor::run_all(&root)?;

    if json {
        let fixed_json: Vec<_> = fixed
            .iter()
            .map(|f| {
                serde_json::json!({
                    "name": f.name,
                    "ok": f.ok,
                    "action": f.action,
                })
            })
            .collect();
        let checks_json: Vec<_> = after
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
                "fixed": fixed_json,
                "checks_after": checks_json,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {}", "Doctor Results (before fixes)".bold().cyan());
        print_issues(&before.checks);
        println!();
        println!("  {}", "Fixes applied".bold().cyan());
        let mut fixed_count = 0;
        for f in &fixed {
            if f.ok {
                println!("  {} {}: {}", "✓".green().bold(), f.name, f.action.dimmed());
                fixed_count += 1;
            } else {
                println!("  {} {}: {}", "✗".red().bold(), f.name, f.action);
            }
        }
        println!();
        println!(
            "  {} applied {} of {} repairs",
            "Summary:".bold(),
            fixed_count.to_string().green(),
            fixed.len(),
        );
        println!();
        println!("  {}", "Doctor Results (after fixes)".bold().cyan());
        print_issues(&after.checks);
        print_summary(&after.checks);
        println!();
    }

    Ok(())
}

/// Pretty-print a full check report (all severities).
fn print_report(checks: &[doctor::Check], json: bool) -> Result<()> {
    if json {
        let checks_json: Vec<_> = checks
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
                "checks": checks_json,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {}", "Doctor Results".bold().cyan());
        print_issues(checks);
        print_summary(checks);
        println!();
    }
    Ok(())
}

/// Pretty-print only the failing (Warn/Error) checks.
fn print_issues(checks: &[doctor::Check]) {
    for check in checks {
        if check.severity == doctor::Severity::Pass {
            continue;
        }
        let (symbol, color) = if check.severity == doctor::Severity::Error {
            ("✗", "red")
        } else {
            ("⚠", "yellow")
        };
        match color {
            "red" => println!("  {} {}", symbol.red().bold(), check.name),
            _ => println!("  {} {}", symbol.yellow().bold(), check.name),
        }
        println!("    └ {}", check.message);
        if let Some(fix) = &check.fix {
            println!("      fix: {}", fix.dimmed());
        }
    }
}

/// Pretty-print the pass/warn/error summary line.
fn print_summary(checks: &[doctor::Check]) {
    let pass_count = checks
        .iter()
        .filter(|c| c.severity == doctor::Severity::Pass)
        .count();
    let warn_count = checks
        .iter()
        .filter(|c| c.severity == doctor::Severity::Warn)
        .count();
    let error_count = checks
        .iter()
        .filter(|c| c.severity == doctor::Severity::Error)
        .count();
    println!(
        "  Summary: {} passed, {} warnings, {} errors",
        pass_count.to_string().green(),
        warn_count.to_string().yellow(),
        error_count.to_string().red(),
    );
}
