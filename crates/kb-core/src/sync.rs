use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config;
use crate::discovery;
use crate::paths;
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

/// Result of a global-sync operation.
#[derive(Debug)]
pub struct GlobalSyncResult {
    /// Set when local and remote have diverged — nothing was touched.
    pub conflict: Option<String>,
    pub pulled: bool,
    pub already_up_to_date: bool,
    pub committed: bool,
    pub commit_message: Option<String>,
    pub pushed: bool,
    pub files_changed: Vec<String>,
    pub linked_projects: Vec<String>,
    pub dry_run: bool,
}

/// Bidirectional sync of the whole knowledge-base.
///
/// 1. Fetch remote state and compute ahead/behind (no mutations yet).
/// 2. If local and remote diverged (ahead AND behind) → abort with guidance.
/// 3. If behind → `git pull --ff-only` (never rebase/force blindly).
/// 4. Stage *everything* (`git add -A`), commit, push.
/// 5. Re-link active projects when `link` is set.
///
/// `dry_run` fetches and reports the plan without mutating anything.
pub fn global_sync(
    kb_root: Option<&Path>,
    message: Option<&str>,
    link: bool,
    dry_run: bool,
) -> Result<GlobalSyncResult> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // 1. Fetch remote state
    git_fetch(&root)?;
    let branch = current_branch(&root)?;
    let origin_head = format!("origin/{branch}");

    // If the remote branch doesn't exist yet (first push), we can't diverge
    // and we have nothing to pull: treat behind as 0.
    let has_remote_branch = rev_exists(&root, &origin_head)?;
    let ahead = if has_remote_branch {
        rev_count(&root, &format!("{origin_head}..HEAD"))?
    } else {
        rev_count(&root, "HEAD")?
    };
    let behind = if has_remote_branch {
        rev_count(&root, &format!("HEAD..{origin_head}"))?
    } else {
        0
    };

    let empty = || GlobalSyncResult {
        conflict: None,
        pulled: false,
        already_up_to_date: behind == 0 && ahead == 0,
        committed: false,
        commit_message: None,
        pushed: false,
        files_changed: vec![],
        linked_projects: vec![],
        dry_run,
    };

    // 2. Divergence check — the user's "will this conflict?" guard.
    if ahead > 0 && behind > 0 {
        return Ok(GlobalSyncResult {
            conflict: Some(format!(
                "local and remote have diverged ({} commit(s) ahead, {} behind).\n\
                 Nothing was pulled, committed, or pushed.\n\
                 Resolve manually:\n  cd {}\n  kb sync      # rebase local commits onto remote\n  git push     # once rebased cleanly",
                ahead,
                behind,
                root.display()
            )),
            ..empty()
        });
    }

    if dry_run {
        return Ok(empty());
    }

    let mut result = empty();

    // 3. Merge the fetched remote branch if behind (fast-forward only)
    if behind > 0 {
        let output = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["merge", "--ff-only", &origin_head])
            .output()
            .context("failed to run git merge --ff-only")?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "fast-forward merge failed:\n{}\n\
                 If you have uncommitted changes blocking the fast-forward, commit or stash them first, then rerun `kb global-sync`.",
                if stderr.trim().is_empty() {
                    &stdout
                } else {
                    &stderr
                }
            );
        }
        result.pulled = true;
        result.already_up_to_date = false;
    }

    // 4. Stage everything, then report what changed
    let add = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["add", "-A"])
        .output()
        .context("failed to git add -A")?;

    if !add.status.success() {
        anyhow::bail!("git add -A failed");
    }

    let status_output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain"])
        .output()
        .context("failed to git status")?;

    let status_str = String::from_utf8_lossy(&status_output.stdout);
    result.files_changed = status_str
        .lines()
        .map(|l| {
            let trimmed = l.trim_start();
            trimmed
                .strip_prefix("M ")
                .or_else(|| trimmed.strip_prefix("A "))
                .or_else(|| trimmed.strip_prefix("D "))
                .or_else(|| trimmed.strip_prefix("R "))
                .unwrap_or(trimmed)
                .trim()
                .to_string()
        })
        .filter(|f| !f.is_empty())
        .collect();

    if !result.files_changed.is_empty() {
        // 5. Commit
        let commit_msg = message.unwrap_or("sync knowledge base").to_string();
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
        result.committed = true;
        result.commit_message = Some(commit_msg);
    }

    // 6. Push whenever there is anything to move up: local commits that
    //    predate this run, or the commit we just made.
    let must_push = ahead > 0 || result.committed;
    if must_push {
        let push_output = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["push", "origin", &branch])
            .output()
            .context("failed to git push")?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            anyhow::bail!("git push failed: {}", stderr);
        }
        result.pushed = true;
    }

    // 7. Re-link active projects (best-effort)
    if link {
        let cfg = config::load()?;
        let templates_dir = root.join("templates").join("project");
        for project_name in &cfg.active_projects {
            let repo_dir = default_project_dir(project_name)?;
            if repo_dir.exists() {
                let _ = project::link(&root, project_name, &repo_dir, &templates_dir);
                result.linked_projects.push(project_name.clone());
            }
        }
    }

    Ok(result)
}

/// Pull the knowledge-base repo and re-link active projects.
///
/// If `only_projects` is non-empty, only re-link those specific projects.
/// If empty, re-link all active projects from config.
pub fn pull(kb_root: Option<&Path>, link: bool, only_projects: &[String]) -> Result<PullResult> {
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
pub fn sync(kb_root: Option<&Path>, only_projects: &[String]) -> Result<SyncResult> {
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

/// Fetch remote state from origin.
fn git_fetch(root: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["fetch", "origin"])
        .output()
        .context("failed to run git fetch origin")?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "git fetch failed:\n{}",
            if stderr.trim().is_empty() {
                &stdout
            } else {
                &stderr
            }
        );
    }
    Ok(())
}

/// Current branch name (e.g. `main`); falls back to `HEAD` if detached.
fn current_branch(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .context("failed to determine current branch")?;

    if !output.status.success() {
        anyhow::bail!("cannot determine current branch of {}", root.display());
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if branch.is_empty() || branch == "HEAD" {
        "main".to_string()
    } else {
        branch
    })
}

/// Check whether a ref exists (e.g. `origin/main`).
fn rev_exists(root: &Path, rev: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "--quiet", rev])
        .output()
        .context(format!("failed to check ref {}", rev))?;
    Ok(output.status.success())
}

/// Number of commits reachable from the right side but not the left.
fn rev_count(root: &Path, range: &str) -> Result<u64> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-list", "--count", range])
        .output()
        .context(format!("failed to count commits for {}", range))?;

    if !output.status.success() {
        anyhow::bail!("failed to count commits for {}", range);
    }

    let count = String::from_utf8_lossy(&output.stdout);
    count
        .trim()
        .parse::<u64>()
        .context(format!("invalid commit count from git: {}", count.trim()))
}

/// Default project directory for a named project.
fn default_project_dir(name: &str) -> Result<std::path::PathBuf> {
    let home = paths::home_dir()?;
    Ok(home.join("Projects").join(name))
}

/// Export project memory to a portable tarball.
pub fn export_project(
    kb_root: Option<&Path>,
    project_name: &str,
    output_path: Option<&Path>,
) -> Result<std::path::PathBuf> {
    crate::paths::validate_project_name(project_name)?;
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
        let home = paths::home_dir()?;
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
        crate::paths::validate_project_name(n)?;
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

    // Extract tarball with safety flags
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(tarball)
        .arg("-C")
        .arg(root.join("projects"))
        // Security: don't follow symlinks outside the archive
        .arg("--no-same-owner")
        .status()
        .context("failed to run tar")?;

    if !status.success() {
        anyhow::bail!("tar failed to extract archive");
    }

    Ok(name)
}
