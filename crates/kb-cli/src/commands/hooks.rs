use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use kb_core::config;
use kb_core::discovery;

pub fn run(kb_root: Option<&Path>, command: HooksCommand, json: bool, _quiet: bool) -> Result<()> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let _cfg = config::load()?;

    let hooks_dir = root.join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;

    match command {
        HooksCommand::List => run_list(&hooks_dir, json),
        HooksCommand::Add {
            point,
            command: cmd,
            project,
        } => run_add(&hooks_dir, &point, &cmd, project.as_deref(), json),
        HooksCommand::Remove {
            point,
            project,
            index: _,
        } => run_remove(&hooks_dir, &point, project.as_deref(), json),
        HooksCommand::Run { point, project } => {
            run_hook(&hooks_dir, &point, project.as_deref(), json)
        }
    }
}

fn run_list(hooks_dir: &Path, json: bool) -> Result<()> {
    let hooks = load_hooks(hooks_dir)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": { "hooks": hooks }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} {}", "Hooks".bold().blue(), "configured:".bold());
        println!();
        for h in &hooks {
            println!(
                "  {} {} {}",
                h.point.color("cyan"),
                h.project.as_deref().unwrap_or("(global)").dimmed(),
                h.command.dimmed()
            );
        }
        println!();
    }
    Ok(())
}

fn run_add(
    hooks_dir: &Path,
    point: &str,
    command: &str,
    project: Option<&str>,
    json: bool,
) -> Result<()> {
    let valid_points = [
        "pre-sync",
        "post-sync",
        "pre-global-sync",
        "post-global-sync",
        "pre-link",
        "post-link",
    ];
    if !valid_points.contains(&point) {
        anyhow::bail!(
            "Invalid hook point '{}'. Valid: {}",
            point,
            valid_points.join(", ")
        );
    }
    let mut hooks = load_hooks(hooks_dir)?;
    let index = hooks.len();
    hooks.push(HookEntry {
        point: point.to_string(),
        command: command.to_string(),
        project: project.map(|s| s.to_string()),
    });
    save_hooks(hooks_dir, &hooks)?;

    if json {
        let output = serde_json::json!({
            "ok": true,
            "data": { "index": index, "point": point, "command": command }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!(
            "  {} hook to {} at index {}",
            "Added".bold().green(),
            point,
            index
        );
        println!();
    }
    Ok(())
}

fn run_remove(hooks_dir: &Path, point: &str, project: Option<&str>, json: bool) -> Result<()> {
    let mut hooks = load_hooks(hooks_dir)?;
    hooks.retain(|h| h.point != point || h.project.as_deref() != project);
    save_hooks(hooks_dir, &hooks)?;

    if json {
        let output = serde_json::json!({ "ok": true, "data": { "point": point, "removed": true } });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("  {} hooks for {}", "Removed".bold().red(), point);
        println!();
    }
    Ok(())
}

fn run_hook(hooks_dir: &Path, point: &str, project: Option<&str>, _json: bool) -> Result<()> {
    let hooks = load_hooks(hooks_dir)?;
    let matching: Vec<_> = hooks
        .iter()
        .filter(|h| h.point == point && h.project.as_deref() == project)
        .collect();

    if matching.is_empty() {
        println!("  {} No hooks configured for {}", "Info:".dimmed(), point);
        return Ok(());
    }

    for hook in &matching {
        println!("  {} {}", "Running".bold(), hook.command);
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&hook.command)
            .output()?;
        if !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}

fn load_hooks(hooks_dir: &Path) -> Result<Vec<HookEntry>> {
    let path = hooks_dir.join("config.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(&path)?;
    let hooks: Vec<HookEntry> = serde_json::from_str(&data).unwrap_or_default();
    Ok(hooks)
}

fn save_hooks(hooks_dir: &Path, hooks: &[HookEntry]) -> Result<()> {
    let data = serde_json::to_string_pretty(hooks)?;
    std::fs::write(hooks_dir.join("config.json"), data)?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct HookEntry {
    point: String,
    command: String,
    project: Option<String>,
}

#[derive(clap::Subcommand)]
pub enum HooksCommand {
    /// List all configured hooks
    List,

    /// Add a new hook
    Add {
        /// Hook point: pre-sync, post-sync, pre-global-sync, post-global-sync, pre-link, post-link
        point: String,

        /// Command to execute (supports ~ expansion)
        command: String,

        /// Project scope (empty = global)
        #[arg(long)]
        project: Option<String>,
    },

    /// Remove a hook
    Remove {
        /// Hook point
        point: String,

        /// Project scope (empty = global)
        #[arg(long)]
        project: Option<String>,

        /// Index of hook to remove (0-based)
        #[arg(long)]
        index: Option<usize>,
    },

    /// Run a hook point manually (for testing)
    Run {
        /// Hook point
        point: String,

        /// Project scope (empty = global)
        #[arg(long)]
        project: Option<String>,
    },
}
