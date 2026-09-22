use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::config::{self, Config};
use crate::paths;
use crate::platform;
use crate::project;
use crate::sparse;

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

/// Outcome of attempting to repair one check with `--fix`.
#[derive(Debug)]
pub struct Fix {
    pub name: &'static str,
    pub action: String,
    pub ok: bool,
}

/// Run all health checks against the knowledge-base.
pub fn run_all(kb_root: &Path) -> Result<DoctorReport> {
    let checks = vec![
        check_config(kb_root),
        check_symlinks(kb_root),
        check_gitignore_global(),
        check_handoffs(kb_root),
        check_orphaned_projects(kb_root),
        check_sparse(kb_root),
    ];

    Ok(DoctorReport { checks })
}

/// Repair every auto-fixable check, in dependency order.
///
/// Sparse reconciliation runs before the orphaned check so that cone-drift
/// residue (a `projects/<name>` dir materialized by a stale cone, then pruned
/// when the cone is rebuilt) is never mistaken for a real orphaned memory and
/// re-registered as active.
///
/// Advisory checks (stale handoffs) are intentionally left alone: writing
/// memory content is never something doctor should do. Returns the list of
/// attempted repairs so the CLI can report what changed.
pub fn fix_all(kb_root: &Path) -> Result<Vec<Fix>> {
    let fixes = vec![
        fix_config(kb_root),
        fix_symlinks(kb_root),
        fix_gitignore_global(),
        fix_sparse(kb_root),
        fix_orphaned_projects(kb_root),
    ];

    Ok(fixes)
}

fn noop(name: &'static str, note: impl Into<String>) -> Fix {
    Fix {
        name,
        action: format!("nothing to fix ({})", note.into()),
        ok: true,
    }
}

fn ok_fix(name: &'static str, action: String) -> Fix {
    Fix {
        name,
        action,
        ok: true,
    }
}

fn err_fix(name: &'static str, err: impl std::fmt::Display) -> Fix {
    Fix {
        name,
        action: format!("not fixed: {err}"),
        ok: false,
    }
}

/// Point the config at the discovered kb_root (missing or broken config).
fn fix_config(kb_root: &Path) -> Fix {
    match config::load() {
        Ok(cfg) if cfg.kb_root.as_deref() == Some(kb_root) && kb_root.exists() => {
            // `kb link .` used to store the literal relative path (cwd-dependent
            // health checks). Rewrite any relative repo_path entries to the
            // conventional absolute default project directory.
            let relative: Vec<String> = cfg
                .projects
                .iter()
                .filter(|(_, p)| {
                    p.repo_path
                        .as_ref()
                        .is_some_and(|r| r.as_path().is_relative())
                })
                .map(|(name, _)| name.clone())
                .collect();

            if relative.is_empty() {
                return noop("config", "already valid");
            }

            let mut rewritten = vec![];
            match config::update(|c| {
                for name in &relative {
                    if let (Ok(path), Some(entry)) =
                        (paths::default_project_dir(name), c.projects.get_mut(name))
                    {
                        entry.repo_path = Some(path);
                        rewritten.push(name.clone());
                    }
                }
            }) {
                Ok(()) if !rewritten.is_empty() => ok_fix(
                    "config",
                    format!("canonicalized repo_path for {}", rewritten.join(", ")),
                ),
                Ok(()) => noop("config", "already valid"),
                Err(e) => err_fix("config", e),
            }
        }
        Ok(cfg) if cfg.kb_root.as_deref() == Some(kb_root) => {
            // Config points here but the directory is missing — no config
            // rewrite can recreate the knowledge-base itself.
            err_fix(
                "config",
                format!("kb_root directory missing: {}", kb_root.display()),
            )
        }
        Ok(_) => match config::update(|c| c.kb_root = Some(kb_root.to_path_buf())) {
            Ok(_) => ok_fix(
                "config",
                format!("set kb_root to {}", paths::normalize_display(kb_root)),
            ),
            Err(e) => err_fix("config", e),
        },
        Err(_) => {
            // Unparseable (or unreadable) config: back it up, then rewrite it
            // pointing at the discovered kb_root. The backup lets the user
            // recover anything `config::save` does not carry over.
            let path = config::config_path().unwrap_or_default();
            let backup = path.with_extension("toml.bak");
            if path.exists() {
                let _ = fs::copy(&path, &backup);
            }
            let fresh = Config {
                kb_root: Some(kb_root.to_path_buf()),
                ..Config::default()
            };
            match config::save(&fresh) {
                Ok(_) => ok_fix(
                    "config",
                    format!("rewrote unreadable config; backup at {}", backup.display()),
                ),
                Err(e2) => err_fix("config", e2),
            }
        }
    }
}

/// Re-link every active project whose symlinks are missing or wrong.
///
/// `project::link` is idempotent and `platform::create_symlink` replaces an
/// existing (even dangling) symlink, so re-linking is safe.
fn fix_symlinks(kb_root: &Path) -> Fix {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => return err_fix("symlinks", e),
    };
    let templates = kb_root.join("templates");
    let rules_target = kb_root.join("agent-rules");

    let mut fixed = vec![];
    let mut skipped = vec![];

    for name in &cfg.active_projects {
        let repo_path = cfg
            .projects
            .get(name)
            .and_then(|p| p.repo_path.clone())
            .unwrap_or_else(|| paths::default_project_dir(name).unwrap_or_default());
        if !repo_path.is_dir() {
            skipped.push(format!("{name} (repo not on this machine)"));
            continue;
        }
        let memory = kb_root.join("projects").join(name);
        let scratch_ok = platform::is_symlink_to(&repo_path.join("scratch"), &memory);
        let rules_ok = platform::is_symlink_to(&repo_path.join(".agent-rules"), &rules_target);
        if scratch_ok && rules_ok {
            continue;
        }
        match project::link(kb_root, name, &repo_path, &templates) {
            Ok(_) => fixed.push(name.clone()),
            Err(e) => skipped.push(format!("{name} ({e})")),
        }
    }

    if fixed.is_empty() && skipped.is_empty() {
        return noop("symlinks", "all active projects healthy");
    }
    let mut parts = vec![];
    if !fixed.is_empty() {
        parts.push(format!("re-linked {}", fixed.join(", ")));
    }
    if !skipped.is_empty() {
        parts.push(format!("skipped {}", skipped.join(", ")));
    }
    Fix {
        name: "symlinks",
        action: parts.join("; "),
        ok: !fixed.is_empty() || skipped.is_empty(),
    }
}

/// Ensure `~/.gitignore` has the kb entries AND git points `core.excludesFile`
/// at it. Respects an already-configured excludesFile (never overrides it).
fn fix_gitignore_global() -> Fix {
    let home = match paths::home_dir() {
        Ok(h) => h,
        Err(e) => return err_fix("gitignore_global", e),
    };
    let gitignore = home.join(".gitignore");

    let file_updated = match project::ensure_global_gitignore() {
        Ok(u) => u,
        Err(e) => return err_fix("gitignore_global", e),
    };

    if global_excludes_configured() {
        return ok_fix(
            "gitignore_global",
            if file_updated {
                "refreshed ~/.gitignore with kb entries".to_string()
            } else {
                "already configured".to_string()
            },
        );
    }

    let output = std::process::Command::new("git")
        .args([
            "config",
            "--global",
            "core.excludesFile",
            gitignore.to_str().unwrap_or_default(),
        ])
        .output();
    match output {
        Ok(o) if o.status.success() => ok_fix(
            "gitignore_global",
            format!("set core.excludesFile -> {}", gitignore.display()),
        ),
        Ok(o) => err_fix(
            "gitignore_global",
            String::from_utf8_lossy(&o.stderr).trim(),
        ),
        Err(e) => err_fix("gitignore_global", e),
    }
}

fn global_excludes_configured() -> bool {
    std::process::Command::new("git")
        .args(["config", "--global", "core.excludesFile"])
        .output()
        .map(|o| {
            let val = String::from_utf8_lossy(&o.stdout).trim().to_string();
            !val.is_empty()
        })
        .unwrap_or(false)
}

/// Register project memories that exist on disk but aren't active. The other
/// half of the hint (remove the directory) is destructive, so doctor never
/// does that automatically.
fn fix_orphaned_projects(kb_root: &Path) -> Fix {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => return err_fix("orphaned", e),
    };
    let projects_dir = kb_root.join("projects");
    if !projects_dir.exists() {
        return noop("orphaned", "no projects directory");
    }

    let mut added = vec![];
    let mut problems = vec![];
    if let Ok(entries) = fs::read_dir(&projects_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip the doc index and import staging dirs (`.import-*`).
            if name == "README.md" || name.starts_with('.') {
                continue;
            }
            if cfg.active_projects.contains(&name) {
                continue;
            }
            match config::update(|c| config::ensure_active_project(c, &name)) {
                Ok(_) => added.push(name),
                Err(e) => problems.push(format!("{name} ({e})")),
            }
        }
    }

    if added.is_empty() && problems.is_empty() {
        return noop("orphaned", "no orphaned project memories");
    }
    let mut parts = vec![];
    if !added.is_empty() {
        parts.push(format!("added to active_projects: {}", added.join(", ")));
    }
    if !problems.is_empty() {
        parts.push(format!("problem {}", problems.join(", ")));
    }
    Fix {
        name: "orphaned",
        action: parts.join("; "),
        ok: problems.is_empty(),
    }
}

/// Reconcile the sparse-checkout cone with `active_projects` (the source of
/// truth): rebuild the cone to exactly the configured subscriptions, or
/// disable sparse-checkout entirely when nothing is subscribed.
fn fix_sparse(kb_root: &Path) -> Fix {
    let enabled = match sparse::enabled(kb_root) {
        Ok(e) => e,
        Err(_) => return noop("sparse", "kb is not a git worktree"),
    };
    if !enabled {
        return noop("sparse", "full checkout");
    }

    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => return err_fix("sparse", e),
    };
    let configured = cfg.active_projects;
    let materialized = sparse::subscribed_projects(kb_root).unwrap_or_default();

    let missing: Vec<String> = configured
        .iter()
        .filter(|p| !materialized.contains(p))
        .cloned()
        .collect();
    let extra: Vec<String> = materialized
        .iter()
        .filter(|p| !configured.contains(p))
        .cloned()
        .collect();

    if missing.is_empty() && extra.is_empty() {
        return noop(
            "sparse",
            format!(
                "cone matches subscriptions ({} project(s))",
                configured.len()
            ),
        );
    }

    let mut detail = vec![];
    if !missing.is_empty() {
        detail.push(format!("missing: {}", missing.join(", ")));
    }
    if !extra.is_empty() {
        detail.push(format!("extra: {}", extra.join(", ")));
    }

    let result = if configured.is_empty() {
        sparse::disable(kb_root)
            .map(|_| "disabled sparse-checkout (restored full checkout)".to_string())
    } else {
        sparse::cone(kb_root, &configured)
            .and_then(|cone| sparse::set_cone(kb_root, &cone))
            .map(|_| format!("rebuilt cone for {} subscription(s)", configured.len()))
    };

    match result {
        Ok(action) => ok_fix("sparse", format!("{} ({})", action, detail.join("; "))),
        Err(e) => err_fix("sparse", e),
    }
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

/// Check that a sparse-checkout, when active, matches the configured
/// subscriptions (`active_projects` is the source of truth).
fn check_sparse(kb_root: &Path) -> Check {
    let enabled = match sparse::enabled(kb_root) {
        Ok(e) => e,
        Err(_) => {
            return Check {
                name: "sparse",
                severity: Severity::Pass,
                message: "skipped (kb not a git worktree)".to_string(),
                fix: None,
            };
        }
    };

    if !enabled {
        return Check {
            name: "sparse",
            severity: Severity::Pass,
            message: "full checkout (all project memory present on this device)".to_string(),
            fix: None,
        };
    }

    let configured = match config::load() {
        Ok(cfg) => cfg.active_projects,
        Err(_) => {
            return Check {
                name: "sparse",
                severity: Severity::Warn,
                message: "sparse-checkout active but config unavailable".to_string(),
                fix: Some("run `kb init` to fix config".to_string()),
            };
        }
    };

    let materialized = sparse::subscribed_projects(kb_root).unwrap_or_default();

    let mut missing: Vec<String> = configured
        .iter()
        .filter(|p| !materialized.contains(p))
        .cloned()
        .collect();
    let mut extra: Vec<String> = materialized
        .iter()
        .filter(|p| !configured.contains(p))
        .cloned()
        .collect();
    missing.sort();
    extra.sort();

    if missing.is_empty() && extra.is_empty() {
        return Check {
            name: "sparse",
            severity: Severity::Pass,
            message: format!(
                "sparse-checkout matches subscriptions ({} project(s))",
                configured.len()
            ),
            fix: None,
        };
    }

    let mut parts = vec![];
    let mut fixes = vec![];
    if !missing.is_empty() {
        parts.push(format!(
            "subscribed but not checked out: {}",
            missing.join(", ")
        ));
        for p in &missing {
            fixes.push(format!("kb subscribe {}", p));
        }
    }
    if !extra.is_empty() {
        parts.push(format!(
            "checked out but not subscribed: {}",
            extra.join(", ")
        ));
        for p in &extra {
            fixes.push(format!("kb unsubscribe {}", p));
        }
    }

    Check {
        name: "sparse",
        severity: Severity::Warn,
        message: parts.join("; "),
        fix: Some(fixes.join("; ")),
    }
}
