use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::sync;

pub fn run(kb_root: Option<&Path>, tarball: &Path, name: Option<&str>, json: bool) -> Result<()> {
    let imported_name = sync::import_project(kb_root, tarball, name)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "project": imported_name,
                "tarball": tarball,
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} {}", "Imported".bold().green(), imported_name.bold());
        println!(
            "  {} Run `kb link {}` to wire it up.",
            "Next:".dimmed(),
            imported_name
        );
        println!();
    }

    Ok(())
}
