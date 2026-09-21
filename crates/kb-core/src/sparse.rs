use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config;
use crate::discovery;
use crate::paths;

/// Result of a `kb subscribe` operation.
#[derive(Debug)]
pub struct SubscribeResult {
    pub project: String,
    /// Full subscription list (from config) after the change.
    pub subscribed: Vec<String>,
    /// Whether the working tree uses sparse-checkout after the change.
    pub sparse_enabled: bool,
}

/// Result of a `kb unsubscribe` operation.
#[derive(Debug)]
pub struct UnsubscribeResult {
    pub project: String,
    pub subscribed: Vec<String>,
    pub sparse_enabled: bool,
}

/// Subscribe this device to a project's memory.
///
/// Adds the project to `active_projects` (config is the source of truth) and
/// rebuilds the sparse-checkout cone so this device keeps the always-on top
/// level plus exactly its subscribed `projects/<name>/` directories. Enables
/// sparse-checkout on a full checkout that was never sparse.
pub fn subscribe(kb_root: Option<&Path>, name: &str) -> Result<SubscribeResult> {
    paths::validate_project_name(name)?;
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    if !is_git_repo(&root) {
        anyhow::bail!(
            "{} is not a git repository.\n\
             The knowledge-base must be a git repo to subscribe to project memory.",
            paths::normalize_display(&root)
        );
    }

    if !project_exists(&root, name)? {
        anyhow::bail!(
            "no project memory for '{}' in the knowledge-base.\n\
             There is no projects/{}/ in the repo yet.\n\
             Hint: `kb link {}` on another device (or this one) creates it.",
            name,
            name,
            name
        );
    }

    config::update(|cfg| config::ensure_active_project(cfg, name))?;
    let subscribed = config::load()?.active_projects;
    let sparse_enabled = rebuild_cone(&root, &subscribed)?;

    Ok(SubscribeResult {
        project: name.to_string(),
        subscribed,
        sparse_enabled,
    })
}

/// Unsubscribe this device from a project's memory.
///
/// Removes the project from `active_projects` and rebuilds the sparse cone.
/// Idempotent: the project just needs to be a valid name.
pub fn unsubscribe(kb_root: Option<&Path>, name: &str) -> Result<UnsubscribeResult> {
    paths::validate_project_name(name)?;
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    if !is_git_repo(&root) {
        anyhow::bail!(
            "{} is not a git repository.\n\
             The knowledge-base must be a git repo to manage subscriptions.",
            paths::normalize_display(&root)
        );
    }

    config::update(|cfg| config::remove_active_project(cfg, name))?;
    let subscribed = config::load()?.active_projects;
    let sparse_enabled = rebuild_cone(&root, &subscribed)?;

    Ok(UnsubscribeResult {
        project: name.to_string(),
        subscribed,
        sparse_enabled,
    })
}

/// Whether the working tree at `root` uses sparse-checkout.
pub fn sparse_enabled(root: &Path) -> Result<bool> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["sparse-checkout", "list"])
        .output()
        .context("failed to inspect sparse-checkout state")?;
    Ok(out.status.success())
}

/// Directories the sparse-checkout cone currently materializes.
pub fn sparse_dirs(root: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["sparse-checkout", "list"])
        .output()
        .context("failed to list sparse-checkout dirs")?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let dirs = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    Ok(dirs)
}

/// Project names present in the current sparse cone (`projects/<name>/`).
pub fn subscribed_sparse_projects(root: &Path) -> Result<Vec<String>> {
    Ok(sparse_dirs(root)?
        .into_iter()
        .filter_map(|d| d.strip_prefix("projects/").map(|s| s.to_string()))
        .filter(|s| !s.is_empty() && !s.contains('/'))
        .collect())
}

/// The device's always-on directories: every top-level repo directory except
/// the per-project store. Top-level *files* are always present in a cone.
fn always_on_dirs(root: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-tree", "--name-only", "-d", "HEAD"])
        .output()
        .context("failed to list knowledge-base layout")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("git ls-tree failed: {}", stderr.trim());
    }

    let dirs: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && l != "projects")
        .collect();
    Ok(dirs)
}

/// Build the cone: always-on top-level dirs + one `projects/<name>/` per
/// subscription. Deterministic (always-on dirs first, then config order).
fn build_cone(root: &Path, subscribed: &[String]) -> Result<Vec<String>> {
    let mut cone = always_on_dirs(root)?;
    for name in subscribed {
        let dir = format!("projects/{name}");
        if !cone.contains(&dir) {
            cone.push(dir);
        }
    }
    Ok(cone)
}

/// Set the sparse-checkout cone to exactly `cone` (enables sparse-checkout
/// when the worktree was a full checkout). Returns whether sparse is on.
fn rebuild_cone(root: &Path, subscribed: &[String]) -> Result<bool> {
    let cone = build_cone(root, subscribed)?;
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("sparse-checkout")
        .arg("set")
        .args(&cone)
        .output()
        .context("failed to configure sparse-checkout")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("git sparse-checkout failed: {}", stderr.trim());
    }
    sparse_enabled(root)
}

/// Whether `projects/<name>` exists in the KB tree at HEAD.
fn project_exists(root: &Path, name: &str) -> Result<bool> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "-e", &format!("HEAD:projects/{name}")])
        .output()
        .context("failed to check project memory in knowledge-base")?;
    Ok(out.status.success())
}

/// Whether `root` is inside a git work tree.
fn is_git_repo(root: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn init_repo(dir: &Path) -> std::process::Output {
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(dir)
            .output()
            .unwrap()
    }

    fn commit_all(dir: &Path, msg: &str) {
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap()
        };
        assert!(git(&["add", "-A"]).status.success());
        let out = git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            msg,
        ]);
        assert!(out.status.success(), "git commit failed: {:?}", out);
    }

    #[test]
    fn always_on_dirs_excludes_projects() {
        let dir = TempDir::new().unwrap();
        init_repo(dir.path());
        fs::create_dir_all(dir.path().join("agent-rules")).unwrap();
        fs::create_dir_all(dir.path().join("templates")).unwrap();
        fs::create_dir_all(dir.path().join("projects/alpha")).unwrap();
        fs::write(dir.path().join("agent-rules/README.md"), "").unwrap();
        fs::write(dir.path().join("templates/README.md"), "").unwrap();
        fs::write(dir.path().join("projects/alpha/HANDOFF.md"), "").unwrap();
        commit_all(dir.path(), "init");

        let dirs = always_on_dirs(dir.path()).unwrap();
        assert!(dirs.contains(&"agent-rules".to_string()));
        assert!(dirs.contains(&"templates".to_string()));
        assert!(!dirs.contains(&"projects".to_string()));
    }

    #[test]
    fn build_cone_combines_always_on_and_subscriptions() {
        let dir = TempDir::new().unwrap();
        init_repo(dir.path());
        fs::create_dir_all(dir.path().join("agent-rules")).unwrap();
        fs::create_dir_all(dir.path().join("projects/alpha")).unwrap();
        fs::create_dir_all(dir.path().join("projects/beta")).unwrap();
        fs::write(dir.path().join("agent-rules/README.md"), "").unwrap();
        fs::write(dir.path().join("projects/alpha/HANDOFF.md"), "").unwrap();
        fs::write(dir.path().join("projects/beta/HANDOFF.md"), "").unwrap();
        commit_all(dir.path(), "init");

        let cone = build_cone(dir.path(), &["alpha".to_string(), "beta".to_string()]).unwrap();
        assert!(cone.contains(&"agent-rules".to_string()));
        assert!(cone.contains(&"projects/alpha".to_string()));
        assert!(cone.contains(&"projects/beta".to_string()));
        assert_eq!(cone.len(), 3);
    }

    #[test]
    fn project_exists_checks_tree() {
        let dir = TempDir::new().unwrap();
        init_repo(dir.path());
        fs::create_dir_all(dir.path().join("projects/alpha")).unwrap();
        fs::write(dir.path().join("projects/alpha/HANDOFF.md"), "").unwrap();
        commit_all(dir.path(), "init");

        assert!(project_exists(dir.path(), "alpha").unwrap());
        assert!(!project_exists(dir.path(), "nope").unwrap());
    }

    #[test]
    fn sparse_state_is_detected() {
        let dir = TempDir::new().unwrap();
        init_repo(dir.path());
        fs::create_dir_all(dir.path().join("agent-rules")).unwrap();
        fs::create_dir_all(dir.path().join("projects/alpha")).unwrap();
        fs::write(dir.path().join("agent-rules/README.md"), "").unwrap();
        fs::write(dir.path().join("projects/alpha/HANDOFF.md"), "").unwrap();
        commit_all(dir.path(), "init");

        assert!(!sparse_enabled(dir.path()).unwrap());
        assert!(sparse_dirs(dir.path()).unwrap().is_empty());

        rebuild_cone(dir.path(), &["alpha".to_string()]).unwrap();
        assert!(sparse_enabled(dir.path()).unwrap());
        let cone = sparse_dirs(dir.path()).unwrap();
        assert!(cone.contains(&"projects/alpha".to_string()));
        assert!(cone.contains(&"agent-rules".to_string()));
    }
}
