use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Expand `~` at the start of a path to the actual home directory.
pub fn expand_home(path: &Path) -> Result<PathBuf> {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        Ok(home.join(rest))
    } else if s == "~" {
        dirs::home_dir().context("cannot determine home directory")
    } else {
        Ok(path.to_path_buf())
    }
}

/// Validate a project name — must be safe for use in path joins.
///
/// Rejects names containing path separators, `..`, or null bytes.
pub fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("project name cannot be empty");
    }
    if name.contains('/') || name.contains('\\') {
        anyhow::bail!(
            "project name cannot contain path separators: {}",
            name
        );
    }
    if name == ".." || name == "." {
        anyhow::bail!("project name cannot be '.' or '..'");
    }
    if name.contains("..") {
        anyhow::bail!("project name cannot contain '..': {}", name);
    }
    if name.contains('\0') {
        anyhow::bail!("project name cannot contain null bytes");
    }
    Ok(())
}

/// Resolve the default project repo path: `~/Projects/<name>`.
pub fn default_project_dir(name: &str) -> Result<PathBuf> {
    validate_project_name(name)?;
    let home = dirs::home_dir().context("cannot determine home directory")?;
    Ok(home.join("Projects").join(name))
}

/// Normalize a path for display (forward slashes).
pub fn normalize_display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Check if a path exists and is a directory.
pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

/// Check if a path exists and is a symlink.
pub fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Read the target of a symlink. Returns None if not a symlink.
pub fn read_link_target(path: &Path) -> Option<PathBuf> {
    std::fs::read_link(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_home_leaves_non_home_paths_unchanged() {
        let p = PathBuf::from("/tmp/test");
        assert_eq!(expand_home(&p).unwrap(), p);
    }

    #[test]
    fn expand_home_handles_tilde_slash() {
        let p = PathBuf::from("~/foo");
        let expanded = expand_home(&p).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(expanded, home.join("foo"));
    }

    #[test]
    fn expand_home_handles_bare_tilde() {
        let p = PathBuf::from("~");
        let expanded = expand_home(&p).unwrap();
        assert_eq!(expanded, dirs::home_dir().unwrap());
    }

    #[test]
    fn normalize_display_fixes_backslashes() {
        let p = PathBuf::from("/home/user/Projects");
        assert_eq!(normalize_display(&p), "/home/user/Projects");
    }

    #[test]
    fn validate_project_name_rejects_empty() {
        assert!(validate_project_name("").is_err());
    }

    #[test]
    fn validate_project_name_rejects_dotdot() {
        assert!(validate_project_name("..").is_err());
        assert!(validate_project_name("../etc").is_err());
        assert!(validate_project_name("foo/../bar").is_err());
    }

    #[test]
    fn validate_project_name_rejects_slashes() {
        assert!(validate_project_name("foo/bar").is_err());
        assert!(validate_project_name("foo\\bar").is_err());
    }

    #[test]
    fn validate_project_name_rejects_null() {
        assert!(validate_project_name("foo\0bar").is_err());
    }

    #[test]
    fn validate_project_name_accepts_valid() {
        assert!(validate_project_name("dioxus-auth").is_ok());
        assert!(validate_project_name("my_project").is_ok());
        assert!(validate_project_name("Project123").is_ok());
    }
}
