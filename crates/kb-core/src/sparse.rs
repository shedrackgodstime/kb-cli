use anyhow::Result;
use std::path::Path;

use crate::git;

/// Whether the working tree at `root` uses sparse-checkout.
pub fn enabled(root: &Path) -> Result<bool> {
    git::ok(root, ["sparse-checkout", "list"])
}

/// Directories the sparse-checkout cone currently materializes.
pub fn dirs(root: &Path) -> Result<Vec<String>> {
    let output = git::run(root, ["sparse-checkout", "list"])?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    Ok(parse_lines(&output.stdout))
}

/// Project names present in the current sparse cone (`projects/<name>/`).
pub fn subscribed_projects(root: &Path) -> Result<Vec<String>> {
    Ok(dirs(root)?
        .into_iter()
        .filter_map(|d| d.strip_prefix("projects/").map(str::to_string))
        .filter(|s| !s.is_empty() && !s.contains('/'))
        .collect())
}

/// Whether `projects/<name>` exists in the KB tree at HEAD.
pub fn project_exists(root: &Path, name: &str) -> Result<bool> {
    git::ok(root, ["cat-file", "-e", &format!("HEAD:projects/{name}")])
}

/// Whether `projects/<name>` exists anywhere on this device: in HEAD or as a
/// (possibly uncommitted) directory in the working tree. The disk fallback
/// lets a project created by `kb link`, which is uncommitted until the next
/// sync, be subscribed immediately.
pub fn memory_exists(root: &Path, name: &str) -> Result<bool> {
    if project_exists(root, name)? {
        return Ok(true);
    }
    Ok(root.join("projects").join(name).is_dir())
}

/// Set the sparse-checkout cone to exactly `cone`, enabling sparse-checkout
/// when the worktree was a full checkout.
pub fn set_cone(root: &Path, cone: &[String]) -> Result<()> {
    anyhow::ensure!(
        !cone.is_empty(),
        "knowledge-base has no directories to keep; \
         commit at least one top-level directory before using selective sync"
    );

    let mut args: Vec<&str> = vec!["sparse-checkout", "set"];
    args.extend(cone.iter().map(String::as_str));
    git::checked(root, &args, "git sparse-checkout set")?;
    Ok(())
}

/// Disable sparse-checkout, materializing the whole knowledge-base again.
/// This is the natural inverse of enabling it on the first subscribe, and is
/// used when the last subscription is removed.
pub fn disable(root: &Path) -> Result<()> {
    let output = git::run(root, ["sparse-checkout", "disable"])?;
    if !output.status.success() {
        anyhow::bail!(
            "git sparse-checkout disable failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Build the desired cone: the always-on top-level directories plus one
/// `projects/<name>/` per subscription. Deterministic — always-on dirs first,
/// then config order.
pub fn cone(root: &Path, subscribed: &[String]) -> Result<Vec<String>> {
    let mut cone = always_on_dirs(root)?;
    for name in subscribed {
        let dir = format!("projects/{name}");
        if !cone.contains(&dir) {
            cone.push(dir);
        }
    }
    Ok(cone)
}

/// The device's always-on directories: every top-level repo directory except
/// the per-project store. Top-level *files* are always present in a cone.
fn always_on_dirs(root: &Path) -> Result<Vec<String>> {
    let out = git::checked(
        root,
        ["ls-tree", "--name-only", "-d", "HEAD"],
        "git ls-tree",
    )?;
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && l != "projects")
        .collect())
}

fn parse_lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_repo_with_memory(names: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-b", "main"])
                .current_dir(dir.path())
                .output()
                .unwrap()
                .status
                .success()
        );

        std::fs::create_dir_all(dir.path().join("agent-rules")).unwrap();
        std::fs::write(dir.path().join("agent-rules/README.md"), "").unwrap();
        for name in names {
            let project = dir.path().join("projects").join(name);
            std::fs::create_dir_all(&project).unwrap();
            std::fs::write(project.join("HANDOFF.md"), "").unwrap();
        }

        assert!(
            Command::new("git")
                .args(["add", "-A"])
                .current_dir(dir.path())
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(
            Command::new("git")
                .args([
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.com",
                    "commit",
                    "-m",
                    "init",
                ])
                .current_dir(dir.path())
                .output()
                .unwrap()
                .status
                .success()
        );

        dir
    }

    fn run(dir: &Path, args: &[&str]) {
        assert!(
            Command::new("git")
                .current_dir(dir)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }

    #[test]
    fn always_on_dirs_excludes_projects() {
        let dir = init_repo_with_memory(&["alpha"]);
        let dirs = always_on_dirs(dir.path()).unwrap();
        assert!(dirs.contains(&"agent-rules".to_string()));
        assert!(!dirs.contains(&"projects".to_string()));
    }

    #[test]
    fn cone_combines_always_on_and_subscriptions() {
        let dir = init_repo_with_memory(&["alpha", "beta"]);
        let cone = cone(dir.path(), &["alpha".to_string(), "beta".to_string()]).unwrap();
        assert!(cone.contains(&"agent-rules".to_string()));
        assert!(cone.contains(&"projects/alpha".to_string()));
        assert!(cone.contains(&"projects/beta".to_string()));
        assert_eq!(cone.len(), 3);
    }

    #[test]
    fn project_and_memory_existence() {
        let dir = init_repo_with_memory(&["alpha"]);
        assert!(project_exists(dir.path(), "alpha").unwrap());
        assert!(!project_exists(dir.path(), "nope").unwrap());

        // Uncommitted memory on disk is still seen by memory_exists.
        std::fs::create_dir_all(dir.path().join("projects/fresh")).unwrap();
        assert!(memory_exists(dir.path(), "fresh").unwrap());
        assert!(!memory_exists(dir.path(), "nope").unwrap());
    }

    #[test]
    fn set_cone_enables_sparse_and_rejects_empty() {
        let dir = init_repo_with_memory(&["alpha"]);
        assert!(!enabled(dir.path()).unwrap());
        assert!(dirs(dir.path()).unwrap().is_empty());

        let err = set_cone(dir.path(), &[]).unwrap_err();
        assert!(err.to_string().contains("has no directories to keep"));

        set_cone(
            dir.path(),
            &["agent-rules".to_string(), "projects/alpha".to_string()],
        )
        .unwrap();
        assert!(enabled(dir.path()).unwrap());
        let dr = dirs(dir.path()).unwrap();
        assert!(dr.contains(&"projects/alpha".to_string()));
        assert!(dr.contains(&"agent-rules".to_string()));

        // set_cone removes paths that are no longer subscribed.
        set_cone(dir.path(), &["agent-rules".to_string()]).unwrap();
        assert!(!dir.path().join("projects/alpha").exists());

        run(dir.path(), &["sparse-checkout", "disable"]);
        assert!(!enabled(dir.path()).unwrap());
    }

    #[test]
    fn subscribed_projects_reads_cone() {
        let dir = init_repo_with_memory(&["alpha", "beta"]);
        assert!(subscribed_projects(dir.path()).unwrap().is_empty());

        set_cone(
            dir.path(),
            &["agent-rules".to_string(), "projects/alpha".to_string()],
        )
        .unwrap();
        let projects = subscribed_projects(dir.path()).unwrap();
        assert_eq!(projects, vec!["alpha".to_string()]);
    }
}
