use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{discovery, paths, sparse};

/// Open a project's KB directory (or the KB root) in the configured editor,
/// falling back to the platform's file manager when no editor is set.
///
/// Returns the directory that was opened.
pub fn open(kb_root: Option<&Path>, project: Option<&str>) -> Result<PathBuf> {
    let dir = resolve_dir(kb_root, project)?;
    open_dir(&dir)?;
    Ok(dir)
}

/// Resolve the directory `kb open` should reveal. A project name maps to
/// `projects/<name>/`; omitting it maps to the KB root itself.
pub fn resolve_dir(kb_root: Option<&Path>, project: Option<&str>) -> Result<PathBuf> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    let Some(name) = project else {
        return Ok(root);
    };

    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("project name cannot be empty");
    }
    paths::validate_project_name(name)?;

    if !sparse::memory_exists(&root, name)? {
        anyhow::bail!(
            "no project memory for '{}' in the knowledge-base.\n\
             There is no projects/{}/ in the repo yet.\n\
             Hint: `kb link {}` on another device (or this one) creates it.",
            name,
            name,
            name
        );
    }

    let dir = root.join("projects").join(name);
    if !dir.is_dir() {
        anyhow::bail!(
            "project '{}' is not checked out on this device.\n\
             Run `kb subscribe {}` to keep its memory here, then try again.",
            name,
            name
        );
    }

    Ok(dir)
}

/// The editor from the environment: `$VISUAL` wins over `$EDITOR`.
pub fn env_editor() -> Option<String> {
    std::env::var("VISUAL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
}

/// Platform file-manager launcher used when no editor is configured.
pub fn default_launcher() -> &'static str {
    if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    }
}

/// Spawn the viewer for `dir`.
///
/// With an editor set, the editor runs with inherited stdio and `kb` waits
/// for it to finish (so terminal editors keep the console). Without one, the
/// platform file manager is launched detached and `kb` returns immediately.
fn open_dir(dir: &Path) -> Result<()> {
    if let Some(editor) = env_editor() {
        let (command, args) = split_editor(&editor);
        Command::new(&command)
            .args(&args)
            .arg(dir)
            .spawn()
            .context(format!(
                "failed to launch editor `{editor}` to open {}.\n\
                 Set $EDITOR to the path of an editor binary.",
                dir.display()
            ))?
            .wait()
            .context("failed to wait for editor to exit")?;
        return Ok(());
    }

    let launcher = default_launcher();
    Command::new(launcher).arg(dir).spawn().context(format!(
        "failed to launch `{launcher}` to open {}.\n\
             Set $EDITOR (e.g. `code`) to choose an application.",
        dir.display()
    ))?;
    Ok(())
}

/// Split an `$EDITOR` value into a command and optional leading arguments
/// (e.g. `powershell -NoProfile -File hook.ps1`).
fn split_editor(editor: &str) -> (String, Vec<String>) {
    let tokens: Vec<String> = editor.split_whitespace().map(str::to_string).collect();
    match tokens.as_slice() {
        [] => (String::new(), Vec::new()),
        [command, rest @ ..] => (command.clone(), rest.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn fake_kb_root() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("AGENTS.md"), "").unwrap();
        fs::write(dir.path().join("INDEX.md"), "").unwrap();
        fs::create_dir_all(dir.path().join("projects/alpha")).unwrap();
        fs::write(dir.path().join("projects/alpha/HANDOFF.md"), "").unwrap();
        dir
    }

    #[test]
    fn open_without_project_resolves_kb_root() {
        let kb = fake_kb_root();
        assert_eq!(
            resolve_dir(Some(kb.path()), None).unwrap(),
            kb.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn open_with_project_resolves_memory_dir() {
        let kb = fake_kb_root();
        let dir = resolve_dir(Some(kb.path()), Some("alpha")).unwrap();
        assert!(dir.ends_with("projects/alpha"), "dir: {}", dir.display());
    }

    #[test]
    fn open_unknown_project_errors() {
        let kb = fake_kb_root();
        let err = resolve_dir(Some(kb.path()), Some("nope")).unwrap_err();
        assert!(err.to_string().contains("no project memory"), "err: {err}");
    }

    #[test]
    fn open_rejects_empty_or_invalid_names() {
        let kb = fake_kb_root();
        assert!(resolve_dir(Some(kb.path()), Some(" ")).is_err());
        assert!(resolve_dir(Some(kb.path()), Some("a/b")).is_err());
        assert!(resolve_dir(Some(kb.path()), Some("..")).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_default_launcher_is_xdg_open() {
        assert_eq!(default_launcher(), "xdg-open");
    }

    #[cfg(windows)]
    #[test]
    fn windows_default_launcher_is_explorer() {
        assert_eq!(default_launcher(), "explorer");
    }

    #[test]
    fn split_editor_handles_single_command_and_args() {
        let (cmd, args) = split_editor("code");
        assert_eq!(cmd, "code");
        assert!(args.is_empty());

        let (cmd, args) = split_editor("powershell -NoProfile -File hook.ps1");
        assert_eq!(cmd, "powershell");
        assert_eq!(args, vec!["-NoProfile", "-File", "hook.ps1"]);
    }
}
