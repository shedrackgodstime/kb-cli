use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

use kb_core::project::{self, KbRulesAction};
use kb_core::{config, discovery};

use super::resolve_project;

pub fn run(
    kb_root: Option<&Path>,
    project_input: Option<&str>,
    all: bool,
    dry_run: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let templates_dir = root.join("templates").join("project");

    let mut results = Vec::new();
    let mut errors = Vec::new();

    if all {
        let cfg = config::load()?;
        let mut names: Vec<String> = cfg.projects.keys().cloned().collect();
        names.sort();
        for name in names {
            match project::rules_for(&root, &name, None, &templates_dir, dry_run) {
                Ok(result) => results.push(result),
                Err(e) => errors.push(format!("{}: {:#}", name, e)),
            }
        }
    } else {
        let (name, repo_dir) = match project_input {
            Some(input) => resolve_project(input)?,
            None => current_project()?,
        };
        results.push(project::rules_for(
            &root,
            &name,
            Some(&repo_dir),
            &templates_dir,
            dry_run,
        )?);
    }

    for error in &errors {
        eprintln!("  {} {}", "!".yellow().bold(), error);
    }

    if json {
        let data: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "project": r.project_name,
                    "action": action_str(r.action),
                    "repo_dir": r.repo_dir,
                    "file": r.dest,
                })
            })
            .collect();
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "dry_run": dry_run,
                "kb_root": root,
                "results": data,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!();
    if dry_run {
        if !quiet {
            println!("  {} kb-rules.md map (dry run)", "Rules".bold().green());
        }
    } else {
        if !quiet {
            println!("  {} kb-rules.md map", "Rules".bold().green());
        }
    }
    println!();

    for result in &results {
        let (mark, label) = match result.action {
            KbRulesAction::Created => ("✓".green(), "written".green()),
            KbRulesAction::UpToDate => ("✓".green(), "up to date".green()),
            KbRulesAction::Skipped => ("!".yellow().bold(), "skipped (keep local edits)".yellow()),
            KbRulesAction::NotFound => ("!".red().bold(), "repo not found".red()),
        };
        let dest = result.dest.display();
        if !quiet {
            println!("  {} {}  {}", mark, result.project_name.bold(), label);
            println!("  {}  {}", "  └".dimmed(), dest);
        }
    }

    if !quiet {
        println!();
        let created = results
            .iter()
            .filter(|r| r.action == KbRulesAction::Created)
            .count();
        let up_to_date = results
            .iter()
            .filter(|r| r.action == KbRulesAction::UpToDate)
            .count();
        let skipped = results
            .iter()
            .filter(|r| r.action == KbRulesAction::Skipped)
            .count();
        let not_found = results
            .iter()
            .filter(|r| r.action == KbRulesAction::NotFound)
            .count();
        let written_label = if dry_run {
            "would be written"
        } else {
            "written"
        };
        println!(
            "  {} {} · {} up to date · {} skipped · {} not found",
            created, written_label, up_to_date, skipped, not_found,
        );
        if created > 0 {
            println!();
            println!(
                "  {} Attach kb-rules.md in prompts (e.g. @kb-rules.md) so agents load KB context.",
                "ℹ".blue()
            );
        }
        println!();
    }

    Ok(())
}

fn action_str(action: KbRulesAction) -> &'static str {
    match action {
        KbRulesAction::Created => "created",
        KbRulesAction::UpToDate => "up-to-date",
        KbRulesAction::Skipped => "skipped",
        KbRulesAction::NotFound => "not-found",
    }
}

fn current_project() -> Result<(String, PathBuf)> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    let name = cwd
        .file_name()
        .context("cannot determine project name from current directory")?
        .to_string_lossy()
        .to_string();
    Ok((name, cwd))
}
