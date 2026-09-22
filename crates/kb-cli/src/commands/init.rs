use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use kb_core::{config, discovery, paths, platform, project};

pub fn run(kb_root: Option<&Path>, json: bool, _quiet: bool) -> Result<()> {
    // Try to discover existing KB root
    let discovery = discovery::discover_kb_root(kb_root);

    match discovery {
        Ok((root, source)) => {
            // KB already exists — just configure it
            configure_existing(&root, source, json, _quiet)
        }
        Err(_) => {
            // No KB found — offer to clone or create
            setup_new_machine(kb_root, json, _quiet)
        }
    }
}

/// Configure an existing knowledge-base.
fn configure_existing(
    root: &Path,
    source: discovery::DiscoverySource,
    json: bool,
    _quiet: bool,
) -> Result<()> {
    // 1. Write config
    config::update(|cfg| {
        cfg.kb_root = Some(root.to_path_buf());
    })?;

    // 2. Setup global gitignore (~/.gitignore)
    let gitignore_updated = project::ensure_global_gitignore()?;

    // 3. Check git config for global excludes
    let git_config_ok = check_global_excludes();

    let platform_info = platform::detect_platform();
    let gitignore = paths::home_dir()?.join(".gitignore");

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "kb_root": root,
                "config_path": "~/.kb/config.toml",
                "gitignore": gitignore,
                "gitignore_updated": gitignore_updated,
                "global_rules_configured": git_config_ok,
                "os": platform_info.os,
                "symlink_support": format!("{:?}", platform_info.symlink_support),
                "source": format!("{:?}", source),
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
                gitignore.display()
            );
        } else {
            println!("  {}: {} ✓", "Gitignore".bold(), gitignore.display());
        }

        if git_config_ok {
            println!(
                "  {}: git config core.excludesFile ✓",
                "Global rules".bold()
            );
        } else {
            println!(
                "  {}: git config core.excludesFile {}",
                "Global rules".bold(),
                "not set".yellow()
            );
            println!(
                "    Fix: {}",
                "git config --global core.excludesFile ~/.gitignore".dimmed()
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

/// Setup on a fresh machine — clone or create the knowledge-base.
fn setup_new_machine(kb_root: Option<&Path>, json: bool, _quiet: bool) -> Result<()> {
    let home = paths::home_dir()?;
    let default_path = home.join("knowledge-base");

    let target_path = kb_root.unwrap_or(&default_path);

    if json {
        let output = serde_json::json!({
            "ok": false,
            "error": "knowledge_base_not_found",
            "message": format!("Knowledge base not found at {}", target_path.display()),
        "suggestions": [
                    format!("git clone <your-repo> {}", target_path.display()),
                    "kb init --kb-root /path/to/existing/knowledge-base",
                ]
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} kb — knowledge-base manager", "KB".bold().cyan());
        println!();
        println!("  {}", "First time setup".bold());
        println!();

        if target_path.exists() {
            // Path exists but isn't a valid KB
            println!(
                "  {} exists but doesn't look like a knowledge-base.",
                target_path.display()
            );
            println!("  Expected AGENTS.md and INDEX.md in the root.");
            println!();
            println!("  If you have a knowledge-base repo elsewhere:");
            println!("    kb init --kb-root /path/to/your/knowledge-base");
        } else {
            println!("  No knowledge-base found at {}", target_path.display());
            println!();
            println!("  To get started, clone your knowledge-base repo:");
            println!(
                "    {}",
                format!("git clone <your-repo> {}", target_path.display()).dimmed()
            );
            println!();
            println!("  Or if you're starting fresh:");
            println!(
                "    {}",
                format!("mkdir -p {}", target_path.display()).dimmed()
            );
            println!("    {}", format!("cd {}", target_path.display()).dimmed());
            println!("    git init");
            println!("    # Create AGENTS.md and INDEX.md");
            println!();
            println!("  Then run:");
            println!("    kb init");
        }

        println!();
    }

    Ok(())
}

/// Check if git global excludesFile is configured.
fn check_global_excludes() -> bool {
    Command::new("git")
        .args(["config", "--global", "core.excludesFile"])
        .output()
        .map(|o| {
            let val = String::from_utf8_lossy(&o.stdout).trim().to_string();
            !val.is_empty()
        })
        .unwrap_or(false)
}
