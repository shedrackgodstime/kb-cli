use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::{discovery, search};

pub fn run(
    kb_root: Option<&Path>,
    query: &str,
    projects: &[String],
    glob: Option<&str>,
    files_only: bool,
    case_sensitive: bool,
    json: bool,
    quiet: bool,
) -> Result<bool> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    let opts = search::SearchOptions {
        projects: projects.to_vec(),
        files_only,
        case_sensitive,
        glob: glob.map(|g| g.to_string()),
        ..Default::default()
    };

    let hits = search::search(&root, query, &opts)?;
    let found = !hits.is_empty();

    if json {
        let matches: Vec<_> = hits
            .iter()
            .map(|h| {
                serde_json::json!({
                    "project": h.project,
                    "path": h.path,
                    "line": h.line,
                    "text": h.text,
                })
            })
            .collect();

        let output = serde_json::json!({
            "ok": true,
            "data": {
                "query": query,
                "count": hits.len(),
                "matches": matches,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(found);
    }

    if !found {
        println!();
        println!(
            "  {} {} {}",
            "No matches for".dimmed(),
            format!("'{}'", query).bold(),
            format!("in {}.", kb_core::paths::normalize_display(&root)).dimmed()
        );
        println!();
        return Ok(false);
    }

    println!();
    if files_only {
        let mut seen = std::collections::HashSet::new();
        for h in &hits {
            if seen.insert(h.path.clone()) {
                println!("  {}", h.path.bold());
            }
        }
    } else {
        for h in &hits {
            let line = h.line.to_string().yellow().bold();
            let text = h.text.trim_end();
            println!("  {}:{}: {}", h.path.bold(), line, text);
        }
    }
    println!();

    if hits.len() == 1 {
        println!("  {} match\n", 1.to_string().cyan());
    } else {
        println!("  {} matches\n", hits.len().to_string().cyan());
    }

    Ok(true)
}
