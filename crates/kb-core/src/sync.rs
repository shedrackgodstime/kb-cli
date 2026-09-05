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

/// Pull the knowledge-base repo and re-link active projects.
pub fn pull(kb_root: Option<&Path>, link: bool) -> Result<PullResult> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    // 1. Git pull
    let git_pull = git_pull(&root)?;

    // 2. Re-link active projects if requested
    let mut linked_projects = vec![];

    if link {
        let cfg = config::load()?;
        let templates_dir = root.join("templates").join("project");

        for project_name in &cfg.active_projects {
            linked_projects.push(project_name.clone());

            let repo_dir = default_project_dir(project_name)?;
            if repo_dir.exists() {
                let _ = project::link(&root, project_name, &repo_dir, &templates_dir);
            }
        }
    }

    Ok(PullResult {
        git_pull,
        linked_projects,
    })
}

/// Git pull the knowledge-base repo.
fn git_pull(root: &Path) -> Result<GitPullResult> {
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
pub fn export_project(kb_root: Option<&Path>, project_name: &str, output_path: Option<&Path>) -> Result<std::path::PathBuf> {
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
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
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
pub fn import_project(kb_root: Option<&Path>, tarball: &Path, project_name: Option<&str>) -> Result<String> {
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
