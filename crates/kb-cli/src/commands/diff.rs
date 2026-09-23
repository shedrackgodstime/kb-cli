use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::config;
use kb_core::discovery;
use kb_core::git;

pub fn run(kb_root: Option<&Path>, since: Option<&str>, json: bool, _quiet: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let _cfg = config::load()?;
    let since_ref = since.unwrap_or("HEAD~1");
    let stat = git::checked(&root, ["diff", "--stat", since_ref, "--"], "diff: get stat")?;
    let log_output = git::checked(
        &root,
        ["log", "--oneline", since_ref, "..HEAD"],
        "diff: get log",
    )?;

    let commits: Vec<&str> = log_output.lines().collect();

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "since": since_ref,
                "commits": commits.len(),
                "stat": stat,
                "log": commits,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} {}", "Changes since".bold(), since_ref.bold());
        println!("  {} commits", commits.len());
        println!("  {}", stat);
        println!();
    }

    Ok(())
}
