use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::paths;

/// Machine-local config stored at `~/.kb/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Absolute path to the knowledge-base repo on this machine.
    pub kb_root: Option<PathBuf>,

    /// Project names this machine cares about.
    #[serde(default)]
    pub active_projects: Vec<String>,

    /// Per-project overrides.
    #[serde(default)]
    pub projects: HashMap<String, ProjectConfig>,
}

/// Per-project configuration overrides.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    /// Override default repo location (`~/Projects/<name>`).
    pub repo_path: Option<PathBuf>,

    /// Clone depth for refs (1 = shallow, 0 = full). Default 0.
    #[serde(default)]
    pub clone_depth: u32,
}

/// Config file path: `~/.kb/config.toml`.
pub fn config_path() -> Result<PathBuf> {
    let home = paths::home_dir()?;
    Ok(home.join(".kb").join("config.toml"))
}

/// Ensure `~/.kb/` directory exists with restrictive permissions.
///
/// On Unix, creates with 0o700 (owner-only access).
pub fn ensure_kb_dir() -> Result<PathBuf> {
    let home = paths::home_dir()?;
    let kb_dir = home.join(".kb");

    if !kb_dir.exists() {
        fs::create_dir_all(&kb_dir)
            .context(format!("failed to create directory {}", kb_dir.display()))?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&kb_dir, fs::Permissions::from_mode(0o700))
                .context("failed to set permissions on ~/.kb/")?;
        }
    }

    Ok(kb_dir)
}

/// Load config from disk. Returns default config if file doesn't exist.
pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&path)
        .context(format!("failed to read config at {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .context(format!("failed to parse config at {}", path.display()))?;
    Ok(config)
}

/// Save config to disk atomically. Creates `~/.kb/` directory if needed.
///
/// Writes to a temp file first, then renames. If the process crashes
/// mid-write, the original config is preserved.
pub fn save(config: &Config) -> Result<()> {
    let kb_dir = ensure_kb_dir()?;
    let path = kb_dir.join("config.toml");

    let content = toml::to_string_pretty(config).context("failed to serialize config")?;

    // Atomic write: write to temp file, then rename
    let temp_path = kb_dir.join("config.toml.tmp");
    fs::write(&temp_path, &content)
        .context(format!("failed to write config to {}", temp_path.display()))?;
    fs::rename(&temp_path, &path).context(format!(
        "failed to rename {} -> {}",
        temp_path.display(),
        path.display()
    ))?;

    // Set restrictive permissions on config file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .context("failed to set permissions on config file")?;
    }

    Ok(())
}

/// Update config in place: load, apply closure, save.
pub fn update(f: impl FnOnce(&mut Config)) -> Result<()> {
    let mut config = load()?;
    f(&mut config);
    save(&config)?;
    Ok(())
}

/// Add a project to active_projects if not already present.
pub fn ensure_active_project(config: &mut Config, name: &str) {
    if !config.active_projects.iter().any(|p| p == name) {
        config.active_projects.push(name.to_string());
    }
}

/// Remove a project from active_projects.
pub fn remove_active_project(config: &mut Config, name: &str) {
    config.active_projects.retain(|p| p != name);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty() {
        let config = Config::default();
        assert!(config.kb_root.is_none());
        assert!(config.active_projects.is_empty());
        assert!(config.projects.is_empty());
    }

    #[test]
    fn roundtrip_config() {
        let config = Config {
            kb_root: Some(PathBuf::from("/tmp/kb")),
            active_projects: vec!["irosh".to_string()],
            ..Default::default()
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.kb_root, Some(PathBuf::from("/tmp/kb")));
        assert_eq!(parsed.active_projects, vec!["irosh".to_string()]);
    }

    #[test]
    fn ensure_active_project_deduplicates() {
        let mut config = Config::default();
        ensure_active_project(&mut config, "irosh");
        ensure_active_project(&mut config, "irosh");
        assert_eq!(config.active_projects.len(), 1);
    }

    #[test]
    fn test_remove_active_project() {
        let mut config = Config {
            active_projects: vec!["irosh".into(), "dioxus-auth".into()],
            ..Default::default()
        };
        super::remove_active_project(&mut config, "irosh");
        assert_eq!(config.active_projects, vec!["dioxus-auth".to_string()]);
    }
}
