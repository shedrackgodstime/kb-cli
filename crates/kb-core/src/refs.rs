use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A parsed reference repository entry from ref/README.md.
#[derive(Debug, Clone, PartialEq)]
pub struct RefEntry {
    pub name: String,
    pub url: String,
    pub revision: Option<String>,
    pub purpose: String,
}

/// Parse ref/README.md and extract registered references.
///
/// Expects a markdown table under `## Registered References` with columns:
/// `| Name | URL | Current local revision | Purpose / notes |`
pub fn parse_ref_readme(path: &Path) -> Result<Vec<RefEntry>> {
    let content = fs::read_to_string(path)
        .context(format!("failed to read {}", path.display()))?;

    let mut entries = vec![];
    let mut in_table = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect the table header
        if trimmed.contains("Name") && trimmed.contains("URL") && trimmed.contains('|') {
            in_table = true;
            continue;
        }

        // Skip separator line
        if in_table && trimmed.starts_with("|---") {
            continue;
        }

        // Parse table rows
        if in_table && trimmed.starts_with('|') && !trimmed.starts_with("|---") {
            let cells: Vec<&str> = trimmed
                .split('|')
                .skip(1) // skip empty before first |
                .map(|c| c.trim())
                .collect();

            if cells.len() >= 4 {
                let name = cells[0].to_string();
                let url = cells[1].to_string();

                // Revision is in backticks: `abc1234`
                let revision = cells[2]
                    .trim()
                    .trim_start_matches('`')
                    .trim_end_matches('`')
                    .to_string();
                let revision = if revision.is_empty() {
                    None
                } else {
                    Some(revision)
                };

                let purpose = cells[3].to_string();

                // Skip empty rows or header-like rows
                if !name.is_empty() && !url.is_empty() && url.starts_with("http") {
                    entries.push(RefEntry {
                        name,
                        url,
                        revision,
                        purpose,
                    });
                }
            }
        }

        // Stop parsing table if we hit a non-table line after starting
        if in_table && !trimmed.starts_with('|') && !trimmed.is_empty() {
            in_table = false;
        }
    }

    Ok(entries)
}

/// Check the local status of a single ref.
#[derive(Debug, Clone)]
pub struct RefStatus {
    pub entry: RefEntry,
    pub local_path: PathBuf,
    pub status: RefStatusKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefStatusKind {
    /// Not cloned yet.
    Missing,
    /// Cloned and at the correct revision.
    UpToDate,
    /// Cloned but at a different revision.
    Mismatch { expected: String, actual: String },
    /// Cloned but revision couldn't be determined.
    Unknown,
}

/// Check status of all refs for a project.
pub fn check_refs_status(kb_root: &Path, project_name: &str) -> Result<Vec<RefStatus>> {
    let ref_readme = kb_root
        .join("projects")
        .join(project_name)
        .join("ref")
        .join("README.md");

    if !ref_readme.exists() {
        return Ok(vec![]);
    }

    let entries = parse_ref_readme(&ref_readme)?;
    let ref_dir = kb_root
        .join("projects")
        .join(project_name)
        .join("ref");

    // Build a map of existing directories (case-insensitive) for lookup
    let existing_dirs: HashMap<String, PathBuf> = if ref_dir.exists() {
        fs::read_dir(&ref_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                (name.to_lowercase(), e.path())
            })
            .collect()
    } else {
        HashMap::new()
    };

    let mut statuses = vec![];
    for entry in entries {
        let sanitized = sanitize_dir_name(&entry.name);
        let local_path = existing_dirs
            .get(&sanitized)
            .cloned()
            .unwrap_or_else(|| ref_dir.join(&sanitized));

        if !local_path.exists() {
            statuses.push(RefStatus {
                entry,
                local_path,
                status: RefStatusKind::Missing,
            });
            continue;
        }

        // Check current revision of the cloned repo
        match get_current_revision(&local_path) {
            Ok(actual) => {
                let expected_rev = entry.revision.clone();
                if let Some(ref expected) = expected_rev {
                    if actual.starts_with(expected) || expected.starts_with(&actual) {
                        statuses.push(RefStatus {
                            entry,
                            local_path,
                            status: RefStatusKind::UpToDate,
                        });
                    } else {
                        statuses.push(RefStatus {
                            entry,
                            local_path,
                            status: RefStatusKind::Mismatch {
                                expected: expected.clone(),
                                actual,
                            },
                        });
                    }
                } else {
                    statuses.push(RefStatus {
                        entry,
                        local_path,
                        status: RefStatusKind::Unknown,
                    });
                }
            }
            Err(_) => {
                statuses.push(RefStatus {
                    entry,
                    local_path,
                    status: RefStatusKind::Unknown,
                });
            }
        }
    }

    Ok(statuses)
}

/// Clone a missing ref and checkout the pinned revision.
pub fn clone_ref(
    entry: &RefEntry,
    target_dir: &Path,
    shallow: bool,
) -> Result<()> {
    // Create parent if needed
    if let Some(parent) = target_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    // Remove target if it exists and --force is implied by caller
    if target_dir.exists() {
        fs::remove_dir_all(target_dir)?;
    }

    // Build clone command
    let mut cmd = Command::new("git");
    cmd.arg("clone");

    if shallow {
        cmd.arg("--depth").arg("1");
    }

    cmd.arg(&entry.url).arg(target_dir);

    let output = cmd.output().context("failed to run git clone")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "git clone failed for {}:\n{}",
            entry.url,
            stderr.trim()
        );
    }

    // Checkout pinned revision if specified
    if let Some(revision) = &entry.revision {
        let output = Command::new("git")
            .args(["checkout", revision])
            .current_dir(target_dir)
            .output()
            .context("failed to run git checkout")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail — clone succeeded, just couldn't checkout exact rev
            eprintln!(
                "  warning: could not checkout revision {}: {}",
                revision,
                stderr.trim()
            );
        }
    }

    Ok(())
}

/// Get the current HEAD revision of a git repo.
fn get_current_revision(repo_dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo_dir)
        .output()
        .context("failed to run git rev-parse")?;

    if !output.status.success() {
        anyhow::bail!("not a valid git repository: {}", repo_dir.display());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Sanitize a ref name for use as a directory name.
/// E.g., "Dioxus" -> "dioxus", "axum-login" -> "axum-login"
fn sanitize_dir_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ref_readme_basic() {
        let content = r#"# Test Reference Repos

## Registered References

| Name | URL | Current local revision | Purpose / notes |
|---|---|---:|---|
| Dioxus | https://github.com/DioxusLabs/dioxus | `e2cc82e63` | Framework source. |
| axum-login | https://github.com/maxcountryman/axum-login | `151c72d` | Auth patterns. |

## Local Checkouts

Some other content.
"#;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("README.md");
        fs::write(&path, content).unwrap();

        let entries = parse_ref_readme(&path).unwrap();
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].name, "Dioxus");
        assert_eq!(entries[0].url, "https://github.com/DioxusLabs/dioxus");
        assert_eq!(entries[0].revision, Some("e2cc82e63".to_string()));
        assert_eq!(entries[0].purpose, "Framework source.");

        assert_eq!(entries[1].name, "axum-login");
        assert_eq!(entries[1].revision, Some("151c72d".to_string()));
    }

    #[test]
    fn parse_ref_readme_empty_table() {
        let content = r#"# Test

## Registered References

| Name | URL | Current local revision | Purpose / notes |
|---|---|---:|---|

## Local Checkouts
"#;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("README.md");
        fs::write(&path, content).unwrap();

        let entries = parse_ref_readme(&path).unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn parse_ref_readme_no_revision() {
        let content = r#"## Registered References

| Name | URL | Current local revision | Purpose / notes |
|---|---|---:|---|
| mylib | https://github.com/user/mylib | | Testing. |
"#;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("README.md");
        fs::write(&path, content).unwrap();

        let entries = parse_ref_readme(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].revision, None);
    }

    #[test]
    fn sanitize_dir_name_normalizes() {
        assert_eq!(sanitize_dir_name("Dioxus"), "dioxus");
        assert_eq!(sanitize_dir_name("axum-login"), "axum-login");
        assert_eq!(sanitize_dir_name("AxumSessionAuth"), "axumsessionauth");
        assert_eq!(sanitize_dir_name("my lib!"), "my-lib-");
    }
}
