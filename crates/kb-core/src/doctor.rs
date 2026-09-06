use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::config;
use crate::paths;
use crate::platform;

/// Health check severity.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Pass,
    Warn,
    Error,
}

/// A single doctor check result.
#[derive(Debug, Clone)]
pub struct Check {
    pub name: &'static str,
    pub severity: Severity,
    pub message: String,
    pub fix: Option<String>,
}

/// Full doctor report.
#[derive(Debug)]
pub struct DoctorReport {
    pub checks: Vec<Check>,
}

/// Run all health checks against the knowledge-base.
pub fn run_all(kb_root: &Path) -> Result<DoctorReport> {
    let checks = vec![
        check_config(kb_root),
        check_symlinks(kb_root),
        check_gitignore_global(),
        check_handoffs(kb_root),
        check_orphaned_projects(kb_root),
    ];

    Ok(DoctorReport { checks })
}

/// Check that config is valid and kb_root exists.
fn check_config(kb_root: &Path) -> Check {
    match config::load() {
        Ok(cfg) => {
            if cfg.kb_root.is_some() && kb_root.exists() {
                Check {
                    name: "config",
                    severity: Severity::Pass,
                    message: "~/.kb/config.toml valid, kb_root exists".to_string(),
                    fix: None,
                }
            } else {
                Check {
                    name: "config",
                    severity: Severity::Warn,
                    message: "config exists but kb_root is not set".to_string(),
                    fix: Some("run `kb init` to configure".to_string()),
                }
            }
        }
        Err(_) => Check {
            name: "config",
            severity: Severity::Error,
            message: "~/.kb/config.toml missing or invalid".to_string(),
            fix: Some("run `kb init` to create config".to_string()),
        },
    }
}

/// Check that all active project symlinks are healthy.
fn check_symlinks(kb_root: &Path) -> Check {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(_) => {
            return Check {
                name: "symlinks",
                severity: Severity::Warn,
                message: "skipped (config unavailable)".to_string(),
                fix: Some("run `kb init` to fix config".to_string()),
            };
        }
    };

    let mut broken = vec![];
    let rules_target = kb_root.join("agent-rules");

    for project_name in &cfg.active_projects {
        let repo_path = cfg
            .projects
            .get(project_name)
            .and_then(|c| c.repo_path.clone())
            .unwrap_or_else(|| crate::paths::default_project_dir(project_name).unwrap_or_default());

        let memory_dir = kb_root.join("projects").join(project_name);

        // Check scratch symlink
        let scratch_link = repo_path.join("scratch");
        if scratch_link.exists() || scratch_link.symlink_metadata().is_ok() {
            if !platform::is_symlink_to(&scratch_link, &memory_dir) {
                broken.push(format!("{}: scratch broken or wrong target", project_name));
            }
        } else {
            broken.push(format!("{}: scratch symlink missing", project_name));
        }

        // Check .agent-rules symlink
        let rules_link = repo_path.join(".agent-rules");
        if rules_link.exists() || rules_link.symlink_metadata().is_ok() {
            if !platform::is_symlink_to(&rules_link, &rules_target) {
                broken.push(format!(
                    "{}: .agent-rules broken or wrong target",
                    project_name
                ));
            }
        } else {
            broken.push(format!("{}: .agent-rules symlink missing", project_name));
        }
    }

    if broken.is_empty() {
        Check {
            name: "symlinks",
            severity: Severity::Pass,
            message: "all active project symlinks healthy".to_string(),
            fix: None,
        }
    } else {
        Check {
            name: "symlinks",
            severity: Severity::Error,
            message: format!("{} broken symlink(s): {}", broken.len(), broken.join("; ")),
            fix: Some("run `kb link <project>` to re-create symlinks".to_string()),
        }
    }
}

/// Check that global gitignore is set up.
fn check_gitignore_global() -> Check {
    let home = match paths::home_dir() {
        Ok(h) => h,
        Err(_) => {
            return Check {
                name: "gitignore_global",
                severity: Severity::Pass,
                message: "skipped (cannot determine home)".to_string(),
                fix: None,
            };
        }
    };

    let gitignore_global = home.join(".gitignore");

    // Check if global excludes are configured
    let excludes_configured = std::process::Command::new("git")
        .args(["config", "--global", "core.excludesFile"])
        .output()
        .map(|o| {
            let val = String::from_utf8_lossy(&o.stdout).trim().to_string();
            !val.is_empty()
        })
        .unwrap_or(false);

    if !excludes_configured {
        return Check {
            name: "gitignore_global",
            severity: Severity::Warn,
            message: "global gitignore not configured in git".to_string(),
            fix: Some("run: git config --global core.excludesFile ~/.gitignore".to_string()),
        };
    }

    if !gitignore_global.exists() {
        return Check {
            name: "gitignore_global",
            severity: Severity::Warn,
            message: "~/.gitignore does not exist".to_string(),
            fix: Some("create ~/.gitignore with: scratch/ .agent-rules/ kb-rules.md".to_string()),
        };
    }

    let content = fs::read_to_string(&gitignore_global).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    let mut missing = vec![];

    for pattern in &["scratch", ".agent-rules", "kb-rules.md"] {
        let found = lines.iter().any(|l| {
            let trimmed = l.trim();
            trimmed == *pattern
                || trimmed == format!("{}/", pattern)
                || trimmed == format!("/{}", pattern)
        });
        if !found {
            missing.push(format!("{}/", pattern));
        }
    }

    if missing.is_empty() {
        Check {
            name: "gitignore_global",
            severity: Severity::Pass,
            message: "~/.gitignore configured".to_string(),
            fix: None,
        }
    } else {
        Check {
            name: "gitignore_global",
            severity: Severity::Warn,
            message: format!("missing patterns: {}", missing.join(", ")),
            fix: Some(format!("add to ~/.gitignore: {}", missing.join(" "))),
        }
    }
}

/// Check for stale handoffs (older than 14 days).
fn check_handoffs(kb_root: &Path) -> Check {
    let projects_dir = kb_root.join("projects");
    if !projects_dir.exists() {
        return Check {
            name: "handoffs",
            severity: Severity::Pass,
            message: "no projects directory".to_string(),
            fix: None,
        };
    }

    let mut stale = vec![];
    if let Ok(entries) = fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let handoff = entry.path().join("HANDOFF.md");
            let days = fs::metadata(&handoff)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .map(|d| d.as_secs() / 86400)
                .unwrap_or(0);

            if days > 14 {
                stale.push(format!(
                    "{} ({} days)",
                    entry.file_name().to_string_lossy(),
                    days
                ));
            }
        }
    }

    if stale.is_empty() {
        Check {
            name: "handoffs",
            severity: Severity::Pass,
            message: "all handoffs are fresh".to_string(),
            fix: None,
        }
    } else {
        Check {
            name: "handoffs",
            severity: Severity::Warn,
            message: format!("stale handoffs: {}", stale.join(", ")),
            fix: Some("update HANDOFF.md for stale projects".to_string()),
        }
    }
}

/// Check for project memory dirs that aren't in active_projects.
fn check_orphaned_projects(kb_root: &Path) -> Check {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(_) => {
            return Check {
                name: "orphaned",
                severity: Severity::Pass,
                message: "skipped (no config)".to_string(),
                fix: None,
            };
        }
    };

    let projects_dir = kb_root.join("projects");
    if !projects_dir.exists() {
        return Check {
            name: "orphaned",
            severity: Severity::Pass,
            message: "no projects directory".to_string(),
            fix: None,
        };
    }

    let mut orphaned = vec![];
    if let Ok(entries) = fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !cfg.active_projects.contains(&name) {
                orphaned.push(name);
            }
        }
    }

    if orphaned.is_empty() {
        Check {
            name: "orphaned",
            severity: Severity::Pass,
            message: "no orphaned project memories".to_string(),
            fix: None,
        }
    } else {
        Check {
            name: "orphaned",
            severity: Severity::Warn,
            message: format!(
                "project memory exists but not active: {}",
                orphaned.join(", ")
            ),
            fix: Some("add to active_projects in config, or remove the directory".to_string()),
        }
    }
}
