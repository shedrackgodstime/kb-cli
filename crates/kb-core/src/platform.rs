use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Platform information for display and diagnostics.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os: &'static str,
    pub symlink_support: SymlinkSupport,
}

/// What kind of symlink support the current platform has.
#[derive(Debug, Clone, PartialEq)]
pub enum SymlinkSupport {
    /// Symlinks work natively (Linux, macOS).
    Native,
    /// Symlinks work but require Developer Mode or admin (Windows).
    WindowsDevMode,
    /// Symlinks not available or not tested.
    Unsupported,
}

/// Detect current platform and symlink capabilities.
pub fn detect_platform() -> PlatformInfo {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let symlink_support = if cfg!(unix) {
        SymlinkSupport::Native
    } else if cfg!(target_os = "windows") {
        check_windows_dev_mode().unwrap_or(SymlinkSupport::Unsupported)
    } else {
        SymlinkSupport::Unsupported
    };

    PlatformInfo {
        os,
        symlink_support,
    }
}

/// Remove the symlink at `link`, dispatching on target type.
///
/// Windows requires `remove_dir` for directory links and `remove_file` for
/// file links — using the wrong one fails with access denied. On Unix,
/// `remove_file` removes a symlink regardless of target type.
fn remove_symlink_link(link: &Path) -> Result<()> {
    let target_is_dir = fs::read_link(link)
        .ok()
        .and_then(|t| fs::metadata(t).ok())
        .map(|m| m.is_dir());

    let result = match target_is_dir {
        Some(true) => fs::remove_dir(link),
        _ => {
            #[cfg(windows)]
            {
                // This tool only creates directory symlinks; on a broken
                // link, prefer remove_dir and fall back to remove_file.
                fs::remove_dir(link).or_else(|_| fs::remove_file(link))
            }
            #[cfg(not(windows))]
            {
                fs::remove_file(link).or_else(|_| fs::remove_dir(link))
            }
        }
    };

    result.context(format!("failed to remove symlink at {}", link.display()))
}

/// Create a directory symlink from `link` to `target`.
///
/// On Linux/macOS, uses `std::os::unix::fs::symlink`.
/// On Windows, uses `std::os::windows::fs::symlink_dir`.
///
/// If `link` already exists as a symlink, updates it.
/// If `link` exists as a real directory, returns an error.
pub fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    // Validate target exists and is accessible
    if !target.exists() {
        anyhow::bail!("symlink target does not exist: {}", target.display());
    }

    // Validate target is within expected boundaries (not /tmp or /proc etc.)
    if let Ok(canonical) = target.canonicalize() {
        let target_str = canonical.to_string_lossy();
        let bad_prefixes = ["/proc", "/sys", "/dev"];
        for prefix in &bad_prefixes {
            if target_str.starts_with(prefix) {
                anyhow::bail!(
                    "symlink target must be in a real filesystem, not {}",
                    prefix
                );
            }
        }
    }

    // Remove existing symlink if present
    if link
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        remove_symlink_link(link)?;
    }

    // Error if link is a real directory
    if link.is_dir() {
        anyhow::bail!(
            "{} is a real directory, not a symlink. \
             Remove or migrate it manually to prevent data loss.",
            link.display()
        );
    }

    // Create parent directory if needed
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).context(format!(
            "failed to create parent directory for {}",
            link.display()
        ))?;
    }

    // Platform-specific symlink creation
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).context(format!(
            "failed to create symlink: {} -> {}",
            link.display(),
            target.display()
        ))?;
    }

    #[cfg(windows)]
    {
        create_windows_symlink(target, link)?;
    }

    Ok(())
}

/// Remove a symlink at `link` if it points to the expected target.
/// Returns true if removed, false if not a symlink or wrong target.
pub fn remove_symlink(link: &Path, expected_target: &Path) -> Result<bool> {
    let meta = match link.symlink_metadata() {
        Ok(m) => m,
        Err(_) => return Ok(false),
    };

    if !meta.file_type().is_symlink() {
        return Ok(false);
    }

    let current_target =
        fs::read_link(link).context(format!("failed to read symlink at {}", link.display()))?;

    // Normalize for comparison
    let current = current_target.canonicalize().unwrap_or(current_target);
    let expected = expected_target
        .canonicalize()
        .unwrap_or_else(|_| expected_target.to_path_buf());

    if current == expected {
        remove_symlink_link(link)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Check if a path is a symlink pointing to the expected target.
pub fn is_symlink_to(link: &Path, expected_target: &Path) -> bool {
    let meta = match link.symlink_metadata() {
        Ok(m) => m,
        Err(_) => return false,
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    let current = match fs::read_link(link) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let current = current.canonicalize().unwrap_or(current);
    let expected = expected_target
        .canonicalize()
        .unwrap_or_else(|_| expected_target.to_path_buf());
    current == expected
}

// ─── Windows-specific ───────────────────────────────────────────────

#[cfg(windows)]
fn create_windows_symlink(target: &Path, link: &Path) -> Result<()> {
    use std::os::windows::fs;

    match fs::symlink_dir(target, link) {
        Ok(()) => Ok(()),
        Err(e) => {
            let raw_os = e.raw_os_error().unwrap_or(0);
            // Error 1314: ERROR_PRIVILEGE_NOT_HELD
            // Error 1317: ERROR_NOT_A_DIRECTORY (can happen with path issues)
            if raw_os == 1314 {
                anyhow::bail!(
                    "failed to create directory symlink: {} -> {}\n\n\
                     Windows requires Developer Mode or administrator privileges \
                     to create symlinks.\n\n\
                     To enable Developer Mode:\n\
                     1. Open Settings → System → For developers\n\
                     2. Toggle 'Developer Mode' on\n\
                     3. Retry this command\n\n\
                     Or run this terminal as Administrator.",
                    link.display(),
                    target.display()
                );
            } else {
                Err(e).context(format!(
                    "failed to create directory symlink: {} -> {}",
                    link.display(),
                    target.display()
                ))
            }
        }
    }
}

#[cfg(not(windows))]
fn check_windows_dev_mode() -> Option<SymlinkSupport> {
    None
}

#[cfg(windows)]
fn check_windows_dev_mode() -> Option<SymlinkSupport> {
    use std::process::Command;

    // Check if Developer Mode is enabled via registry
    let output = Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock",
            "/v",
            "AllowDevelopmentWithoutDevLicense",
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.contains("0x1") {
                Some(SymlinkSupport::WindowsDevMode)
            } else {
                // Developer mode off, but symlinks might still work with admin
                Some(SymlinkSupport::WindowsDevMode)
            }
        }
        _ => Some(SymlinkSupport::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn create_and_remove_symlink() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("file.txt"), "keep me").unwrap();

        create_symlink(&target, &link).unwrap();
        assert!(is_symlink_to(&link, &target));

        let removed = remove_symlink(&link, &target).unwrap();
        assert!(removed);
        assert!(!link.exists());
        // Target must be untouched — remove must not follow the link.
        assert!(target.join("file.txt").exists());
    }

    #[test]
    fn create_symlink_fails_on_real_dir() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        fs::create_dir(&target).unwrap();
        fs::create_dir(&link).unwrap();

        let result = create_symlink(&target, &link);
        assert!(result.is_err());
    }

    #[test]
    fn detect_platform_returns_valid_os() {
        let info = detect_platform();
        assert!(info.os == "linux" || info.os == "macos" || info.os == "windows");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_support_is_native_on_unix() {
        let info = detect_platform();
        assert_eq!(info.symlink_support, SymlinkSupport::Native);
    }

    #[test]
    fn create_symlink_idempotent() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        fs::create_dir(&target).unwrap();

        create_symlink(&target, &link).unwrap();
        create_symlink(&target, &link).unwrap();
        assert!(is_symlink_to(&link, &target));
    }
}
