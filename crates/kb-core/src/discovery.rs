use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config;

/// Strategy for finding the knowledge-base root.
#[derive(Debug, Clone)]
pub enum DiscoverySource {
    /// From `--kb-root` CLI flag.
    CliFlag(PathBuf),
    /// From `KB_ROOT` environment variable.
    EnvVar(PathBuf),
    /// From `kb_root` in `~/.kb/config.toml`.
    ConfigFile(PathBuf),
    /// Walked up from cwd and found AGENTS.md + INDEX.md.
    GitHeuristic(PathBuf),
}

/// Discover the knowledge-base root using the priority chain:
/// 1. CLI flag
/// 2. KB_ROOT env var
/// 3. config file
/// 4. heuristic (walk up looking for AGENTS.md + INDEX.md)
pub fn discover_kb_root(cli_flag: Option<&Path>) -> Result<(PathBuf, DiscoverySource)> {
    // 1. CLI flag
    if let Some(path) = cli_flag {
        let resolved = path.canonicalize()
            .context(format!("--kb-root path does not exist: {}", path.display()))?;
        validate_kb_root(&resolved)?;
        return Ok((resolved.clone(), DiscoverySource::CliFlag(resolved)));
    }

    // 2. Environment variable
    if let Ok(val) = std::env::var("KB_ROOT") {
        let path = PathBuf::from(&val);
        let resolved = path.canonicalize()
            .context(format!("KB_ROOT path does not exist: {}", path.display()))?;
        validate_kb_root(&resolved)?;
        return Ok((resolved.clone(), DiscoverySource::EnvVar(resolved)));
    }

    // 3. Config file
    if let Ok(cfg) = config::load()
        && let Some(root) = cfg.kb_root
        && root.exists()
    {
        validate_kb_root(&root)?;
        return Ok((root.clone(), DiscoverySource::ConfigFile(root)));
    }

    // 4. Heuristic: walk up from cwd
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    if let Some(root) = walk_up_find_kb(&cwd) {
        return Ok((root.clone(), DiscoverySource::GitHeuristic(root)));
    }

    anyhow::bail!(
        "knowledge-base root not found.\n\n\
         Searched:\n\
         1. --kb-root CLI flag (not provided)\n\
         2. KB_ROOT environment variable (not set)\n\
         3. ~/.kb/config.toml (kb_root not configured)\n\
         4. Walked up from {} (no AGENTS.md + INDEX.md found)\n\n\
         Fix: run `kb init` to set up config, or pass --kb-root <PATH>",
        cwd.display()
    )
}

/// Walk up from `start` looking for a directory containing both
/// `AGENTS.md` and `INDEX.md`.
fn walk_up_find_kb(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let agents = current.join("AGENTS.md");
        let index = current.join("INDEX.md");
        if agents.exists() && index.exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Validate that a path looks like a knowledge-base root
/// (has AGENTS.md and INDEX.md).
fn validate_kb_root(path: &Path) -> Result<()> {
    let agents = path.join("AGENTS.md");
    let index = path.join("INDEX.md");
    if !agents.exists() || !index.exists() {
        anyhow::bail!(
            "{} does not look like a knowledge-base root.\n\
             Expected AGENTS.md and INDEX.md to exist.\n\
             Got: AGENTS.md={}, INDEX.md={}",
            path.display(),
            agents.exists(),
            index.exists()
        );
    }
    Ok(())
}

/// Check if a valid KB root is configured (without erroring).
pub fn is_configured() -> bool {
    config::load()
        .ok()
        .and_then(|c| c.kb_root)
        .map(|r| r.exists() && r.join("AGENTS.md").exists() && r.join("INDEX.md").exists())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn validate_kb_root_passes_with_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "").unwrap();
        fs::write(dir.path().join("INDEX.md"), "").unwrap();
        assert!(validate_kb_root(dir.path()).is_ok());
    }

    #[test]
    fn validate_kb_root_fails_without_files() {
        let dir = TempDir::new().unwrap();
        assert!(validate_kb_root(dir.path()).is_err());
    }

    #[test]
    fn walk_up_finds_kb_root() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "").unwrap();
        fs::write(dir.path().join("INDEX.md"), "").unwrap();
        let sub = dir.path().join("a").join("b");
        fs::create_dir_all(&sub).unwrap();
        assert_eq!(walk_up_find_kb(&sub), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn walk_up_returns_none_when_not_found() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("a");
        fs::create_dir_all(&sub).unwrap();
        assert_eq!(walk_up_find_kb(&sub), None);
    }
}
