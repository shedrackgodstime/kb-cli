use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

use kb_core::{config, discovery, platform};

pub fn run(kb_root: Option<&Path>, json: bool) -> Result<()> {
    // 1. Discover or confirm KB root
    let (root, _source) = discovery::discover_kb_root(kb_root)?;

    // 2. Write config
    config::update(|cfg| {
        cfg.kb_root = Some(root.clone());
    })?;

    // 3. Setup global gitignore
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let gitignore_global = home.join(".gitignore_global");
    let mut gitignore_updated = false;

    if !gitignore_global.exists() {
        fs::write(
            &gitignore_global,
            "# Knowledge base symlinks\nscratch/\n.agent-rules/\n.kb/\n",
        )?;
        gitignore_updated = true;
    } else {
        let content = fs::read_to_string(&gitignore_global)?;
        let mut missing = vec![];
        for pattern in &["scratch/", ".agent-rules/", ".kb/"] {
            if !content.contains(pattern) {
                missing.push(*pattern);
            }
        }
        if !missing.is_empty() {
            let mut new_content = content;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(&format!("# Knowledge base\n{}\n", missing.join("\n")));
            fs::write(&gitignore_global, &new_content)?;
            gitignore_updated = true;
        }
    }

    // 4. Check git config for global excludes
    let git_config_ok = std::process::Command::new("git")
        .args(["config", "--global", "core.excludesFile"])
        .output()
        .map(|o| {
            let val = String::from_utf8_lossy(&o.stdout).trim().to_string();
            !val.is_empty()
        })
        .unwrap_or(false);

    let platform_info = platform::detect_platform();

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "kb_root": root,
                "config_path": "~/.kb/config.toml",
                "gitignore_global": gitignore_global,
                "gitignore_updated": gitignore_updated,
                "global_rules_configured": git_config_ok,
                "os": platform_info.os,
                "symlink_support": format!("{:?}", platform_info.symlink_support),
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} kb — knowledge-base manager", "KB".bold().cyan());
        println!();
        println!("  {}: {}", "KB root".bold(), root.display());
        println!("  {}: ~/.kb/config.toml", "Config".bold());

        if gitignore_updated {
            println!(
                "  {}: {} (updated)",
                "Gitignore".bold(),
                gitignore_global.display()
            );
        } else {
            println!("  {}: {} ✓", "Gitignore".bold(), gitignore_global.display());
        }

        if git_config_ok {
            println!("  {}: git config core.excludesFile ✓", "Global rules".bold());
        } else {
            println!(
                "  {}: git config core.excludesFile {}",
                "Global rules".bold(),
                "not set".yellow()
            );
            println!(
                "    Fix: {}",
                "git config --global core.excludesFile ~/.gitignore_global".dimmed()
            );
        }

        println!();
        println!(
            "  {}",
            "Run `kb link <project>` to wire a project.".dimmed()
        );
        println!();
    }

    Ok(())
}
