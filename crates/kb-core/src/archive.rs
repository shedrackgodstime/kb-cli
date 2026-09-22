use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config;
use crate::paths;
use crate::project::{self, UnlinkResult};
use crate::state;

/// Result of an archive / restore operation.
#[derive(Debug)]
pub struct ArchiveResult {
    pub project_name: String,
    /// `true` for a restore (`archive/<name>/` -> `projects/<name>/`),
    /// `false` for an archive (`projects/<name>/` -> `archive/<name>/`).
    pub restored: bool,
    pub memory_from: PathBuf,
    pub memory_to: PathBuf,
    /// Symlink / kb-rules.md cleanup performed on archive (None on restore).
    pub unlinked: Option<UnlinkResult>,
}

/// Archive a project's memory (or restore it) inside the knowledge-base.
///
/// Archiving moves `projects/<name>/` to `archive/<name>/`, drops the project
/// from `active_projects`, and unlinks its wiring (symlinks + `kb-rules.md`).
/// The per-project config section is kept so `--restore` can reinstate the
/// same repo path / clone depth; only the active list and wiring change.
///
/// Restoring moves the memory back and re-adds the project to
/// `active_projects`; `kb link <name>` wires it up again.
pub fn archive_project(kb_root: &Path, project_name: &str, restore: bool) -> Result<ArchiveResult> {
    paths::validate_project_name(project_name)?;

    let memory_dir = kb_root.join("projects").join(project_name);
    let archive_dir = kb_root.join("archive").join(project_name);

    let repo_dir = config::load()?
        .projects
        .get(project_name)
        .and_then(|c| c.repo_path.clone())
        .or_else(|| paths::default_project_dir(project_name).ok());

    if restore {
        if !archive_dir.exists() {
            bail!(
                "no archived memory for '{}'.\n\
                 Nothing under {}/.\n\
                 Hint: `kb archive {}` on the archiving device first.",
                project_name,
                archive_dir.display(),
                project_name
            );
        }
        if memory_dir.exists() {
            bail!(
                "project memory already exists at {}.\n\
                 Remove it first, or restore under a different name.",
                memory_dir.display()
            );
        }

        fs::rename(&archive_dir, &memory_dir).context(format!(
            "failed to move {} -> {}",
            archive_dir.display(),
            memory_dir.display()
        ))?;

        config::update(|cfg| {
            config::ensure_active_project(cfg, project_name);
        })?;

        return Ok(ArchiveResult {
            project_name: project_name.to_string(),
            restored: true,
            memory_from: archive_dir,
            memory_to: memory_dir,
            unlinked: None,
        });
    }

    if archive_dir.exists() {
        bail!(
            "'{}' is already archived at {}.\n\
             Use `kb archive --restore {}` to bring it back.",
            project_name,
            archive_dir.display(),
            project_name
        );
    }
    if !memory_dir.exists() {
        bail!(
            "no project memory for '{}' in the knowledge-base.\n\
             There is no projects/{}/ yet.\n\
             Hint: `kb link {}` or `kb subscribe {}` creates it.",
            project_name,
            project_name,
            project_name,
            project_name
        );
    }

    // Unlink the wiring FIRST, while `projects/<name>` still exists: the
    // symlink-removal check compares the link's target to the memory dir, so
    // the two must both be resolvable. Moving the directory first would break
    // that comparison (canonicalization falls back and mismatches).
    let unlinked = repo_dir
        .as_ref()
        .map(|repo| project::unlink(kb_root, project_name, repo, true))
        .transpose()
        .context("failed to unlink archived project")?;

    // Stop tracking an in-progress session for the project, if any.
    if state::load()?
        .active_projects
        .iter()
        .any(|p| p == project_name)
    {
        state::remove_project(project_name)?;
    }

    // Then move the memory into the archive.
    fs::create_dir_all(archive_dir.parent().unwrap()).context(format!(
        "failed to create archive directory at {}",
        kb_root.join("archive").display()
    ))?;

    fs::rename(&memory_dir, &archive_dir).context(format!(
        "failed to move {} -> {}",
        memory_dir.display(),
        archive_dir.display()
    ))?;

    Ok(ArchiveResult {
        project_name: project_name.to_string(),
        restored: false,
        memory_from: memory_dir,
        memory_to: archive_dir,
        unlinked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // NOTE: success-path archive/restore writes config + state, which resolves
    // against the real home in unit tests. Those paths are covered by the
    // process-isolated integration tests (HOME override). These unit tests
    // synthesize on-disk state and exercise only the pre-write error branches.

    #[test]
    fn archive_requires_memory() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("projects")).unwrap();

        let err = archive_project(root, "ghost", false).unwrap_err();
        assert!(err.to_string().contains("no project memory for 'ghost'"));
    }

    #[test]
    fn archive_refuses_when_slot_busy() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        // Both slots populated manually: memory exists AND archive slot taken.
        fs::create_dir_all(root.join("projects/alpha")).unwrap();
        fs::create_dir_all(root.join("archive/alpha")).unwrap();

        let err = archive_project(root, "alpha", false).unwrap_err();
        assert!(err.to_string().contains("already archived"));
    }

    #[test]
    fn restore_refuses_when_not_archived() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("archive")).unwrap();

        let err = archive_project(root, "ghost", true).unwrap_err();
        assert!(err.to_string().contains("no archived memory for 'ghost'"));
    }

    #[test]
    fn restore_refuses_when_slot_busy() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("archive/alpha")).unwrap();
        fs::write(root.join("archive/alpha/HANDOFF.md"), "old\n").unwrap();
        fs::create_dir_all(root.join("projects/alpha")).unwrap();

        let err = archive_project(root, "alpha", true).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn validate_project_name_is_enforced() {
        let dir = TempDir::new().unwrap();
        let err = archive_project(dir.path(), "../escape", false).unwrap_err();
        assert!(err.to_string().contains("path separators"));
    }
}
