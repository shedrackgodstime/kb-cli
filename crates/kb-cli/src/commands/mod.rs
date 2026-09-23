pub mod archive;
pub mod clone_refs;
pub mod config;
pub mod diff;
pub mod doctor;
pub mod done;
pub mod export;
pub mod global_sync;
pub mod hooks;
pub mod import;
pub mod init;
pub mod link;
pub mod log;
pub mod open;
pub mod projects;
pub mod pull;
pub mod push;
pub mod rules;
pub mod search;
pub mod snapshot;
pub mod status;
pub mod subscriptions;
pub mod sync;
pub mod unlink;
pub mod work;

use anyhow::{Context, Result};
use kb_core::paths;
use std::path::{Path, PathBuf};

/// Resolve a project argument to `(name, repo_dir)`.
///
/// Accepts a quick-start name (resolved against `~/Projects/<name>`) or a full
/// path to a project repo. When a path like `.` or `..` is given, the current
/// directory name is used.
pub fn resolve_project(input: &str) -> Result<(String, PathBuf)> {
    let expanded = paths::expand_home(Path::new(input))?;

    if expanded.exists() && expanded.is_dir() {
        let repo_dir = expanded
            .canonicalize()
            .context("cannot canonicalize project path")?;
        let name = repo_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| input.to_string());

        let name = if name == "." || name == ".." {
            std::env::current_dir()
                .context("cannot determine current directory")?
                .file_name()
                .context("cannot determine current directory name")?
                .to_string_lossy()
                .to_string()
        } else {
            name
        };

        return Ok((name, repo_dir));
    }

    if input.contains('/') || input.contains('\\') {
        if expanded.exists() {
            let repo_dir = expanded
                .canonicalize()
                .context("cannot canonicalize project path")?;
            let name = repo_dir
                .file_name()
                .context("cannot determine project name from path")?
                .to_string_lossy()
                .to_string();
            return Ok((name, repo_dir));
        }
        anyhow::bail!("project path does not exist: {}", expanded.display());
    }

    let name = input.to_string();
    let repo_dir = paths::default_project_dir(&name)?;
    if !repo_dir.exists() {
        anyhow::bail!(
            "project repo not found at {}.\n\
             Pass the full path instead: kb link /path/to/{}",
            repo_dir.display(),
            name
        );
    }
    Ok((name, repo_dir))
}
