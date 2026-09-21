use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

/// Run a git command against the repository at `root` and capture its output.
///
/// Never fails on a non-zero exit — callers decide how to interpret the status.
pub fn run<I, S>(root: &Path, args: I) -> Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("failed to run git")
}

/// Run git and return the trimmed stdout, bailing with git's stderr on a
/// non-zero exit. `what` names the operation for the error message.
pub fn checked<I, S>(root: &Path, args: I, what: &str) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run(root, args)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{what} failed: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run git and report whether it exited successfully.
pub fn ok<I, S>(root: &Path, args: I) -> Result<bool>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Ok(run(root, args)?.status.success())
}

/// Whether `root` is inside a git work tree.
pub fn is_inside_work_tree(root: &Path) -> bool {
    ok(root, ["rev-parse", "--is-inside-work-tree"]).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn ok_reports_exit_status() {
        let dir = TempDir::new().unwrap();
        assert!(!ok(dir.path(), ["rev-parse", "--is-inside-work-tree"]).unwrap());
        assert!(!ok(dir.path(), ["rev-parse", "--bogus"]).unwrap());

        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-b", "main"])
                .current_dir(&repo)
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(ok(&repo, ["rev-parse", "--is-inside-work-tree"]).unwrap());
    }

    #[test]
    fn checked_returns_trimmed_stdout_and_bails_on_failure() {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-b", "main"])
                .current_dir(&repo)
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
                    "--allow-empty",
                    "-m",
                    "init"
                ])
                .current_dir(&repo)
                .output()
                .unwrap()
                .status
                .success()
        );

        let head = checked(&repo, ["rev-parse", "HEAD"], "git probe").unwrap();
        assert_eq!(head.len(), 40);
        assert!(head.chars().all(|c| c.is_ascii_hexdigit()));
        let err = checked(&repo, ["rev-parse", "HEAD~9999"], "git probe").unwrap_err();
        assert!(err.to_string().contains("git probe failed"));
    }

    #[test]
    fn is_inside_work_tree_detects_non_repos() {
        let dir = TempDir::new().unwrap();
        assert!(!is_inside_work_tree(dir.path()));

        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-b", "main"])
                .current_dir(&repo)
                .output()
                .unwrap()
                .status
                .success()
        );
        assert!(is_inside_work_tree(&repo));
    }
}
