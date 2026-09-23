use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{config, discovery, refs};

#[allow(clippy::too_many_arguments)]
pub fn run(
    kb_root: Option<&Path>,
    project: Option<&str>,
    all: bool,
    shallow: bool,
    full: bool,
    dry_run: bool,
    force: bool,
    json: bool,
    quiet: bool,
) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let cfg = config::load()?;

    // Determine which projects to process
    let projects: Vec<String> = if let Some(name) = project {
        vec![name.to_string()]
    } else if all {
        cfg.active_projects.clone()
    } else if cfg.active_projects.is_empty() {
        anyhow::bail!(
            "no active projects in config.\n\
             Use `kb clone-refs --all` to process all, or `kb clone-refs <project>` for one."
        );
    } else {
        cfg.active_projects.clone()
    };

    if projects.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({"ok": true, "data": {"projects": []}})
            );
        } else if !quiet {
            println!();
            println!("  {}", "No active projects to process.".dimmed());
            println!();
        }
        return Ok(());
    }

    // Determine clone depth per project: CLI flags override config
    let get_depth = |project_name: &str| -> u32 {
        if shallow {
            1
        } else if full {
            0
        } else {
            cfg.projects
                .get(project_name)
                .map(|p| p.clone_depth)
                .unwrap_or(0)
        }
    };

    let mut all_project_outputs = vec![];

    for project_name in &projects {
        let ref_readme = root
            .join("projects")
            .join(project_name)
            .join("ref")
            .join("README.md");

        if !ref_readme.exists() {
            if !json && !quiet {
                println!();
                println!(
                    "  {} {} — no ref/README.md found",
                    "Skipping".dimmed(),
                    project_name.bold()
                );
            }
            continue;
        }

        let statuses = refs::check_refs_status(&root, project_name)?;

        let depth = get_depth(project_name);

        let mut total_cloned = 0;
        let mut total_uptodate = 0;
        let mut total_mismatch = 0;
        let mut total_errors = vec![];
        let mut ref_outputs = vec![];

        if !json && !quiet {
            println!();
            println!("  {} {}", "Refs for".bold().cyan(), project_name.bold());
            println!();
        }

        if statuses.is_empty() {
            if !json && !quiet {
                println!("  {}", "No registered references.".dimmed());
            }
            continue;
        }

        for status in &statuses {
            let ref_status_str = match &status.status {
                refs::RefStatusKind::Missing => "missing",
                refs::RefStatusKind::UpToDate => "up-to-date",
                refs::RefStatusKind::Mismatch { .. } => "mismatch",
                refs::RefStatusKind::Unknown => "unknown",
            };

            ref_outputs.push(serde_json::json!({
                "name": status.entry.name,
                "url": status.entry.url,
                "revision": status.entry.revision,
                "status": ref_status_str,
                "local_path": status.local_path,
            }));

            match &status.status {
                refs::RefStatusKind::Missing => {
                    if dry_run {
                        if !json && !quiet {
                            println!(
                                "  {} {} {}",
                                "?".yellow().bold(),
                                status.entry.name.bold(),
                                "→ would clone".dimmed()
                            );
                        }
                        total_cloned += 1;
                    } else if json {
                        match refs::clone_ref(&status.entry, &status.local_path, depth) {
                            Ok(()) => total_cloned += 1,
                            Err(e) => total_errors.push(format!("{}: {}", status.entry.name, e)),
                        }
                    } else {
                        if !quiet {
                            print!("  {} {} ", "→".cyan(), status.entry.name.bold());
                        }
                        match refs::clone_ref(&status.entry, &status.local_path, depth) {
                            Ok(()) => {
                                println!("{}", "cloned".green());
                                total_cloned += 1;
                            }
                            Err(e) => {
                                println!("{} {}", "failed".red(), e);
                                total_errors
                                    .push(format!("{}: clone failed: {}", status.entry.name, e));
                            }
                        }
                    }
                }
                refs::RefStatusKind::UpToDate => {
                    if !json && !quiet {
                        println!(
                            "  {} {} {}",
                            "⊗".dimmed(),
                            status.entry.name.dimmed(),
                            "→ up-to-date".dimmed()
                        );
                    }
                    total_uptodate += 1;
                }
                refs::RefStatusKind::Mismatch { expected, actual } => {
                    if force {
                        if dry_run {
                            if !json && !quiet {
                                println!(
                                    "  {} {} {}",
                                    "!".yellow().bold(),
                                    status.entry.name.bold(),
                                    format!("→ would re-clone ({} → {})", actual, expected)
                                        .yellow()
                                );
                            }
                        } else if json {
                            match refs::clone_ref(&status.entry, &status.local_path, depth) {
                                Ok(()) => total_cloned += 1,
                                Err(e) => {
                                    total_errors.push(format!("{}: {}", status.entry.name, e))
                                }
                            }
                        } else {
                            if !quiet {
                                print!("  {} {} ", "!".yellow().bold(), status.entry.name.bold());
                            }
                            match refs::clone_ref(&status.entry, &status.local_path, depth) {
                                Ok(()) => {
                                    println!("{}", "re-cloned".green());
                                    total_cloned += 1;
                                }
                                Err(e) => {
                                    println!("{} {}", "failed".red(), e);
                                    total_errors.push(format!(
                                        "{}: re-clone failed: {}",
                                        status.entry.name, e
                                    ));
                                }
                            }
                        }
                    } else if !json && !quiet {
                        println!(
                            "  {} {} {}",
                            "!".yellow().bold(),
                            status.entry.name.bold(),
                            format!("→ mismatch (expected {}, got {})", expected, actual).yellow()
                        );
                    }
                    total_mismatch += 1;
                }
                refs::RefStatusKind::Unknown => {
                    if !json && !quiet {
                        println!(
                            "  {} {} {}",
                            "?".dimmed(),
                            status.entry.name.dimmed(),
                            "→ revision unknown".dimmed()
                        );
                    }
                }
            }
        }

        all_project_outputs.push(serde_json::json!({
            "project": project_name,
            "refs": ref_outputs,
            "summary": {
                "cloned": total_cloned,
                "up_to_date": total_uptodate,
                "mismatch": total_mismatch,
                "errors": total_errors,
            }
        }));

        // Text summary for non-json
        if !json && !quiet {
            let parts = vec![
                if total_cloned > 0 {
                    Some(format!("{} cloned", total_cloned).green().to_string())
                } else {
                    None
                },
                if total_uptodate > 0 {
                    Some(
                        format!("{} up-to-date", total_uptodate)
                            .dimmed()
                            .to_string(),
                    )
                } else {
                    None
                },
                if total_mismatch > 0 {
                    Some(format!("{} mismatch", total_mismatch).yellow().to_string())
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

            if !parts.is_empty() {
                println!();
                println!("  Summary: {}", parts.join(", "));
            }

            if !total_errors.is_empty() {
                println!();
                println!("  {}", "Errors:".red().bold());
                for err in &total_errors {
                    println!("  {} {}", "✗".red(), err);
                }
            }

            println!();
        }
    }

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "projects": all_project_outputs,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    }

    Ok(())
}
