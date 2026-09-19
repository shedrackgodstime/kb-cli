pub mod clone_refs;
pub mod config;
pub mod doctor;
pub mod done;
pub mod export;
pub mod global_sync;
pub mod import;
pub mod init;
pub mod link;
pub mod projects;
pub mod pull;
pub mod push;
pub mod rules;
pub mod search;
pub mod status;
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
pub(crate) fn resolve_project(input: &str) -> Result<(String, PathBuf)> {
    let expanded = paths::expand_home(Path::new(input))?;

    if expanded.exists() && expanded.is_dir() {
        let name = expanded
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
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

        return Ok((name, expanded));
    }

    if input.contains('/') || input.contains('\\') {
        if expanded.exists() {
            let name = expanded
                .file_name()
                .context("cannot determine project name from path")?
                .to_string_lossy()
                .to_string();
            return Ok((name, expanded));
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
