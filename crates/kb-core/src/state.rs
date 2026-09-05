use anyhow::{Context, Result};
use std::path::PathBuf;
use std::fs;

/// Track which projects are currently "in progress" (being worked on).
///
/// State is stored in `~/.kb/state.toml` alongside the config.

#[derive(Debug, Default, Clone)]
pub struct WorkState {
    pub active_projects: Vec<String>,
}

/// Path to the state file.
fn state_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".kb").join("state.toml"))
}

/// Load the current work state.
pub fn load() -> Result<WorkState> {
    let path = state_path()?;

    if !path.exists() {
        return Ok(WorkState::default());
    }

    let content = fs::read_to_string(&path)
        .context(format!("failed to read state file at {}", path.display()))?;

    let mut active_projects = vec![];

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix("active = ") {
            let name = name.trim_matches('"').trim_matches('\'').to_string();
            if !name.is_empty() {
                active_projects.push(name);
            }
        }
    }

    Ok(WorkState { active_projects })
}

/// Save the work state.
pub fn save(state: &WorkState) -> Result<()> {
    let path = state_path()?;

    // Ensure ~/.kb/ directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .context(format!("failed to create directory {}", parent.display()))?;
    }

    let mut content = String::new();
    content.push_str("# kb work state — auto-generated\n");
    content.push_str("# Which projects are currently in progress\n\n");

    for project in &state.active_projects {
        content.push_str(&format!("active = \"{}\"\n", project));
    }

    // Atomic write: temp file + rename
    let tmp_path = path.with_extension("toml.tmp");
    fs::write(&tmp_path, &content)
        .context(format!("failed to write state to {}", tmp_path.display()))?;

    fs::rename(&tmp_path, &path).context(format!(
        "failed to rename {} to {}",
        tmp_path.display(),
        path.display()
    ))?;

    Ok(())
}

/// Add a project to the in-progress list.
pub fn add_project(name: &str) -> Result<()> {
    let mut state = load()?;
    if !state.active_projects.contains(&name.to_string()) {
        state.active_projects.push(name.to_string());
        save(&state)?;
    }
    Ok(())
}

/// Remove a project from the in-progress list.
pub fn remove_project(name: &str) -> Result<()> {
    let mut state = load()?;
    state.active_projects.retain(|p| p != name);
    save(&state)?;
    Ok(())
}

/// Clear all in-progress projects.
pub fn clear() -> Result<()> {
    save(&WorkState {
        active_projects: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_state_is_empty() {
        let state = WorkState::default();
        assert!(state.active_projects.is_empty());
    }

    #[test]
    fn roundtrip_state() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("state.toml");

        let state = WorkState {
            active_projects: vec!["dioxus-auth".into(), "kb".into()],
        };

        let mut content = String::new();
        for project in &state.active_projects {
            content.push_str(&format!("active = \"{}\"\n", project));
        }
        fs::write(&path, &content).unwrap();

        let loaded = fs::read_to_string(&path).unwrap();
        assert!(loaded.contains("dioxus-auth"));
        assert!(loaded.contains("kb"));
    }
}
