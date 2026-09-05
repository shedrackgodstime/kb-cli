use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Command;

use kb_core::{config, discovery, platform};

pub fn run(kb_root: Option<&Path>, json: bool) -> Result<()> {
    // Try to discover existing KB root
    let discovery = discovery::discover_kb_root(kb_root);

    match discovery {
        Ok((root, source)) => {
            // KB already exists — just configure it
            configure_existing(&root, source, json)
        }
        Err(_) => {
            // No KB found — offer to clone or create
            setup_new_machine(kb_root, json)
        }
    }
}

/// Configure an existing knowledge-base.
fn configure_existing(
    root: &Path,
    source: discovery::DiscoverySource,
    json: bool,
) -> Result<()> {
    // 1. Write config
    config::update(|cfg| {
        cfg.kb_root = Some(root.to_path_buf());
    })?;

    // 2. Setup global gitignore
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let gitignore_global = home.join(".gitignore_global");
    let gitignore_updated = setup_gitignore(&gitignore_global)?;

    // 3. Check git config for global excludes
    let git_config_ok = check_global_excludes();

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

/// Setup on a fresh machine — clone or create the knowledge-base.
fn setup_new_machine(kb_root: Option<&Path>, json: bool) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let default_path = home.join("knowledge-base");

    let target_path = kb_root.unwrap_or(&default_path);

    if json {
        let output = serde_json::json!({
            "ok": false,
            "error": "knowledge_base_not_found",
            "message": format!("Knowledge base not found at {}", target_path.display()),
            "suggestions": [
                format!("git clone <your-repo> {}", target_path.display()),
                format!("kb init --kb-root /path/to/existing/knowledge-base"),
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
            println!(
                "    kb init --kb-root /path/to/your/knowledge-base"
            );
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
            println!(
                "    {}",
                format!("cd {}", target_path.display()).dimmed()
            );
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

/// Setup global gitignore file.
fn setup_gitignore(gitignore_global: &Path) -> Result<bool> {
    let mut updated = false;

    if !gitignore_global.exists() {
        fs::write(
            gitignore_global,
            "# Knowledge base symlinks\nscratch/\n.agent-rules/\n.kb/\n",
        )?;
        updated = true;
    } else {
        let content = fs::read_to_string(gitignore_global)?;
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
            fs::write(gitignore_global, &new_content)?;
            updated = true;
        }
    }

    Ok(updated)
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
