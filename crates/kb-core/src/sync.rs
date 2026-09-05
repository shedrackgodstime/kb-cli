use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config;
use crate::discovery;
use crate::project;

/// Result of a pull operation.
#[derive(Debug)]
pub struct PullResult {
    pub git_pull: GitPullResult,
    pub linked_projects: Vec<String>,
}

#[derive(Debug)]
pub struct GitPullResult {
    pub already_up_to_date: bool,
    pub output: String,
}

/// Result of a sync operation.
#[derive(Debug)]
pub struct SyncResult {
    pub git_pull: GitPullResult,
    pub rebase_ok: bool,
    pub linked_projects: Vec<String>,
}

/// Result of a git push operation.
#[derive(Debug)]
pub struct PushResult {
    pub committed: bool,
    pub commit_message: Option<String>,
    pub pushed: bool,
    pub files_changed: Vec<String>,
}

/// Pull the knowledge-base repo and re-link active projects.
///
/// If `only_projects` is non-empty, only re-link those specific projects.
/// If empty, re-link all active projects from config.
pub fn pull(
    kb_root: Option<&Path>,
    link: bool,
    only_projects: &[String],
) -> Result<PullResult> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // 1. Git pull
    let git_pull = git_pull_ff(&root)?;

    // 2. Re-link projects if requested
    let mut linked_projects = vec![];

    if link {
        let cfg = config::load()?;
        let templates_dir = root.join("templates").join("project");

        let projects_to_link: Vec<String> = if only_projects.is_empty() {
            cfg.active_projects.clone()
        } else {
            only_projects.to_vec()
        };

        for project_name in &projects_to_link {
            let repo_dir = default_project_dir(project_name)?;
            if repo_dir.exists() {
                let _ = project::link(&root, project_name, &repo_dir, &templates_dir);
                linked_projects.push(project_name.clone());
            }
        }
    }

    Ok(PullResult {
        git_pull,
        linked_projects,
    })
}

/// Sync: git pull --rebase + re-link. Handles diverged branches.
pub fn sync(
    kb_root: Option<&Path>,
    only_projects: &[String],
) -> Result<SyncResult> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // 1. Fetch first
    Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["fetch", "origin"])
        .output()
        .context("failed to git fetch")?;

    // 2. Try rebase
    let rebase_output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["pull", "--rebase"])
        .output()
        .context("failed to git pull --rebase")?;

    let stdout = String::from_utf8_lossy(&rebase_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&rebase_output.stderr).to_string();
    let output = if stderr.is_empty() { &stdout } else { &stderr };

    let already_up_to_date = output.contains("Already up to date")
        || output.contains("Current branch master is up to date");

    let rebase_ok = rebase_output.status.success();

    if !rebase_ok && output.contains("CONFLICT") {
        // Abort rebase on conflict — let user resolve manually
        let _ = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rebase", "--abort"])
            .output();

        anyhow::bail!(
            "git pull --rebase produced conflicts.\n\
             Resolve manually:\n  cd {}\n  git status\n  git add .\n  git rebase --continue\n  kb sync",
            root.display()
        );
    }

    // 3. Re-link projects
    let mut linked_projects = vec![];
    let cfg = config::load()?;
    let templates_dir = root.join("templates").join("project");

    let projects_to_link: Vec<String> = if only_projects.is_empty() {
        cfg.active_projects.clone()
    } else {
        only_projects.to_vec()
    };

    for project_name in &projects_to_link {
        let repo_dir = default_project_dir(project_name)?;
        if repo_dir.exists() {
            let _ = project::link(&root, project_name, &repo_dir, &templates_dir);
            linked_projects.push(project_name.clone());
        }
    }

    Ok(SyncResult {
        git_pull: GitPullResult {
            already_up_to_date,
            output: output.to_string(),
        },
        rebase_ok,
        linked_projects,
    })
}

/// Git push changes for specific projects only.
///
/// Stages only files under `projects/<name>/` for each specified project,
/// plus any shared files (INDEX.md, AGENTS.md) if modified.
pub fn push(
    kb_root: Option<&Path>,
    projects: &[String],
    message: Option<&str>,
) -> Result<PushResult> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // 1. Check what's changed
    let status_output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain"])
        .output()
        .context("failed to git status")?;

    let status_str = String::from_utf8_lossy(&status_output.stdout);

    if status_str.trim().is_empty() {
        return Ok(PushResult {
            committed: false,
            commit_message: None,
            pushed: false,
            files_changed: vec![],
        });
    }

    // 2. Stage only files for specified projects
    let mut files_to_stage = vec![];

    for line in status_str.lines() {
        let file = line[3..].trim();

        // Always stage shared files
        if file == "INDEX.md"
            || file == "AGENTS.md"
            || file.starts_with("agent-rules/")
            || file.starts_with("templates/")
        {
            files_to_stage.push(file.to_string());
            continue;
        }

        // Stage files for specified projects
        for project_name in projects {
            let prefix = format!("projects/{}/", project_name);
            if file.starts_with(&prefix) {
                files_to_stage.push(file.to_string());
                break;
            }
        }
    }

    if files_to_stage.is_empty() {
        return Ok(PushResult {
            committed: false,
            commit_message: None,
            pushed: false,
            files_changed: vec![],
        });
    }

    // 3. Stage files
    for file in &files_to_stage {
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["add", file])
            .output()
            .context(format!("failed to git add {}", file))?;
    }

    // 4. Commit
    let commit_msg = message.map(|m| m.to_string()).unwrap_or_else(|| {
        if projects.len() == 1 {
            format!("update {}", projects[0])
        } else {
            format!("update {}", projects.join(", "))
        }
    });

    let commit_output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("failed to git commit")?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        anyhow::bail!("git commit failed: {}", stderr);
    }

    // 5. Push
    let push_output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["push"])
        .output()
        .context("failed to git push")?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        anyhow::bail!("git push failed: {}", stderr);
    }

    Ok(PushResult {
        committed: true,
        commit_message: Some(commit_msg),
        pushed: true,
        files_changed: files_to_stage,
    })
}

/// Git pull (fast-forward only).
fn git_pull_ff(root: &Path) -> Result<GitPullResult> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["pull", "--ff-only"])
        .output()
        .context("failed to run git pull")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        anyhow::bail!(
            "git pull failed:\n{}",
            if stderr.is_empty() { &stdout } else { &stderr }
        );
    }

    let already_up_to_date = stdout.contains("Already up to date");

    Ok(GitPullResult {
        already_up_to_date,
        output: stdout,
    })
}

/// Default project directory for a named project.
fn default_project_dir(name: &str) -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join("Projects").join(name))
}

/// Export project memory to a portable tarball.
pub fn export_project(
    kb_root: Option<&Path>,
    project_name: &str,
    output_path: Option<&Path>,
) -> Result<std::path::PathBuf> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;
    let project_dir = root.join("projects").join(project_name);

    if !project_dir.exists() {
        anyhow::bail!(
            "project memory not found at {}.\nRun `kb link {}` first.",
            project_dir.display(),
            project_name
        );
    }

    // Determine output path
    let dest = if let Some(p) = output_path {
        if p.is_dir() {
            p.join(format!("{}.tar.gz", project_name))
        } else {
            p.to_path_buf()
        }
    } else {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
        home.join(format!("{}.tar.gz", project_name))
    };

    // Create parent if needed
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create tarball
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&dest)
        .arg("-C")
        .arg(root.join("projects"))
        .arg(project_name)
        .status()
        .context("failed to run tar")?;

    if !status.success() {
        anyhow::bail!("tar failed to create archive at {}", dest.display());
    }

    Ok(dest)
}

/// Import project memory from a tarball.
pub fn import_project(
    kb_root: Option<&Path>,
    tarball: &Path,
    project_name: Option<&str>,
) -> Result<String> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    if !tarball.exists() {
        anyhow::bail!("tarball not found: {}", tarball.display());
    }

    // Determine project name from tarball or argument
    let name = if let Some(n) = project_name {
        n.to_string()
    } else {
        let stem = tarball
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        // Strip .tar if present
        stem.strip_suffix(".tar").unwrap_or(stem).to_string()
    };

    let target_dir = root.join("projects").join(&name);
    if target_dir.exists() {
        anyhow::bail!(
            "project memory already exists at {}.\nRemove it first or specify a different name.",
            target_dir.display()
        );
    }

    // Extract tarball
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(tarball)
        .arg("-C")
        .arg(root.join("projects"))
        .status()
        .context("failed to run tar")?;

    if !status.success() {
        anyhow::bail!("tar failed to extract archive");
    }

    Ok(name)
}
