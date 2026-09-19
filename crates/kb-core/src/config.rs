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

/// A parsed config key path for `kb config get|set|unset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigKey {
    KbRoot,
    ActiveProjects,
    /// `projects.<name>.repo_path`
    ProjectRepoPath(String),
    /// `projects.<name>.clone_depth`
    ProjectCloneDepth(String),
}

/// The resolved value of a config key, mirrored for display/round-tripping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    /// An optional path value (kb_root, projects.<name>.repo_path).
    Path(Option<PathBuf>),
    /// An array of project names (active_projects).
    Strings(Vec<String>),
    /// clone_depth for a project.
    U32(u32),
}

/// Parse a dotted config key, erroring with the valid key list on bad input.
pub fn parse_config_key(input: &str) -> Result<ConfigKey> {
    let parts: Vec<&str> = input.split('.').collect();
    let key = match parts.as_slice() {
        ["kb_root"] => ConfigKey::KbRoot,
        ["active_projects"] => ConfigKey::ActiveProjects,
        ["projects", name, "repo_path"] if !name.is_empty() => {
            ConfigKey::ProjectRepoPath(name.to_string())
        }
        ["projects", name, "clone_depth"] if !name.is_empty() => {
            ConfigKey::ProjectCloneDepth(name.to_string())
        }
        _ => {
            return Err(anyhow::anyhow!(
                "invalid config key '{}': {}",
                input,
                valid_keys_hint()
            ));
        }
    };
    Ok(key)
}

/// Read a key's value from config without touching disk.
pub fn get_key(config: &Config, key: &ConfigKey) -> ConfigValue {
    match key {
        ConfigKey::KbRoot => ConfigValue::Path(config.kb_root.clone()),
        ConfigKey::ActiveProjects => ConfigValue::Strings(config.active_projects.clone()),
        ConfigKey::ProjectRepoPath(name) => ConfigValue::Path(
            config
                .projects
                .get(name.as_str())
                .and_then(|p| p.repo_path.clone()),
        ),
        ConfigKey::ProjectCloneDepth(name) => ConfigValue::U32(
            config
                .projects
                .get(name.as_str())
                .map(|p| p.clone_depth)
                .unwrap_or(0),
        ),
    }
}

/// Set a key on the config from the raw CLI value. Paths are `~`-expanded;
/// `active_projects` accepts a JSON array, comma-separated list, or single name.
pub fn set_key(config: &mut Config, key: &ConfigKey, raw: &str) -> Result<()> {
    match key {
        ConfigKey::KbRoot => {
            config.kb_root = Some(paths::expand_home(std::path::Path::new(raw))?);
        }
        ConfigKey::ActiveProjects => {
            let items = parse_string_list(raw)?;
            for name in &items {
                paths::validate_project_name(name)?;
            }
            config.active_projects = items;
        }
        ConfigKey::ProjectRepoPath(name) => {
            let entry = config.projects.entry(name.clone()).or_default();
            entry.repo_path = Some(paths::expand_home(std::path::Path::new(raw))?);
        }
        ConfigKey::ProjectCloneDepth(name) => {
            let depth: u32 = raw
                .trim()
                .parse()
                .context("clone_depth must be an integer (0 = full clone)")?;
            config.projects.entry(name.clone()).or_default().clone_depth = depth;
        }
    }
    Ok(())
}

/// Reset a key to its default. Empty per-project entries are pruned.
pub fn unset_key(config: &mut Config, key: &ConfigKey) {
    match key {
        ConfigKey::KbRoot => config.kb_root = None,
        ConfigKey::ActiveProjects => config.active_projects.clear(),
        ConfigKey::ProjectRepoPath(name) => {
            if let Some(entry) = config.projects.get_mut(name.as_str()) {
                entry.repo_path = None;
            }
            prune_empty_project(config, name);
        }
        ConfigKey::ProjectCloneDepth(name) => {
            if let Some(entry) = config.projects.get_mut(name.as_str()) {
                entry.clone_depth = 0;
            }
            prune_empty_project(config, name);
        }
    }
}

fn prune_empty_project(config: &mut Config, name: &str) {
    let is_empty = config
        .projects
        .get(name)
        .map(|p| p.repo_path.is_none() && p.clone_depth == 0)
        .unwrap_or(false);
    if is_empty {
        config.projects.remove(name);
    }
}

/// Parse an `active_projects` value: JSON array, comma-separated, or a single
/// project name.
fn parse_string_list(raw: &str) -> Result<Vec<String>> {
    let trimmed = raw.trim();
    if trimmed.starts_with('[') {
        let list: Vec<String> = serde_json::from_str(trimmed)
            .context("active_projects expects a JSON array of strings, e.g. [\"a\",\"b\"]")?;
        return Ok(list);
    }
    let items: Vec<String> = trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        anyhow::bail!("active_projects requires at least one project name");
    }
    Ok(items)
}

/// Human-readable list of valid config keys.
pub fn valid_keys_hint() -> &'static str {
    "valid keys: kb_root, active_projects, projects.<name>.repo_path, projects.<name>.clone_depth"
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

    #[test]
    fn parse_key_accepts_valid_paths() {
        assert_eq!(parse_config_key("kb_root").unwrap(), ConfigKey::KbRoot);
        assert_eq!(
            parse_config_key("active_projects").unwrap(),
            ConfigKey::ActiveProjects
        );
        assert_eq!(
            parse_config_key("projects.irosh.repo_path").unwrap(),
            ConfigKey::ProjectRepoPath("irosh".into())
        );
        assert_eq!(
            parse_config_key("projects.irosh.clone_depth").unwrap(),
            ConfigKey::ProjectCloneDepth("irosh".into())
        );
    }

    #[test]
    fn parse_key_rejects_invalid_paths() {
        for bad in [
            "bogus",
            "projects",
            "projects..repo_path",
            "projects.irosh",
            "kb.root",
        ] {
            let err = parse_config_key(bad).unwrap_err();
            assert!(err.to_string().contains("valid keys"), "err: {err}");
        }
    }

    #[test]
    fn set_and_get_roundtrip_all_keys() {
        let mut config = Config::default();

        set_key(&mut config, &ConfigKey::KbRoot, "/kb/path").unwrap();
        set_key(
            &mut config,
            &ConfigKey::ActiveProjects,
            r#"["alpha","beta"]"#,
        )
        .unwrap();
        set_key(
            &mut config,
            &ConfigKey::ProjectRepoPath("alpha".into()),
            "/repos/alpha",
        )
        .unwrap();
        set_key(
            &mut config,
            &ConfigKey::ProjectCloneDepth("alpha".into()),
            "1",
        )
        .unwrap();

        assert_eq!(
            get_key(&config, &ConfigKey::KbRoot),
            ConfigValue::Path(Some(PathBuf::from("/kb/path")))
        );
        assert_eq!(
            get_key(&config, &ConfigKey::ActiveProjects),
            ConfigValue::Strings(vec!["alpha".into(), "beta".into()])
        );
        assert_eq!(
            get_key(&config, &ConfigKey::ProjectRepoPath("alpha".into())),
            ConfigValue::Path(Some(PathBuf::from("/repos/alpha")))
        );
        assert_eq!(
            get_key(&config, &ConfigKey::ProjectCloneDepth("alpha".into())),
            ConfigValue::U32(1)
        );
    }

    #[test]
    fn active_projects_accepts_comma_list_and_single() {
        let mut config = Config::default();
        set_key(&mut config, &ConfigKey::ActiveProjects, "a, b, c").unwrap();
        assert_eq!(
            config.active_projects,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );

        set_key(&mut config, &ConfigKey::ActiveProjects, "solo").unwrap();
        assert_eq!(config.active_projects, vec!["solo".to_string()]);
    }

    #[test]
    fn unset_resets_and_prunes_empty_projects() {
        let mut config = Config {
            kb_root: Some(PathBuf::from("/kb")),
            active_projects: vec!["alpha".into()],
            projects: {
                let mut m = HashMap::new();
                m.insert(
                    "alpha".into(),
                    ProjectConfig {
                        repo_path: Some(PathBuf::from("/repos/alpha")),
                        clone_depth: 1,
                    },
                );
                m
            },
        };

        unset_key(&mut config, &ConfigKey::KbRoot);
        assert_eq!(config.kb_root, None);

        unset_key(&mut config, &ConfigKey::ProjectRepoPath("alpha".into()));
        assert!(config.projects.contains_key("alpha"));
        unset_key(&mut config, &ConfigKey::ProjectCloneDepth("alpha".into()));
        assert!(!config.projects.contains_key("alpha"));
    }

    #[test]
    fn unset_clears_active_projects() {
        let mut config = Config {
            active_projects: vec!["alpha".into(), "beta".into()],
            ..Default::default()
        };
        unset_key(&mut config, &ConfigKey::ActiveProjects);
        assert!(config.active_projects.is_empty());
    }
}
