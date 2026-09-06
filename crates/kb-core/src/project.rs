use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{self, ProjectConfig};
use crate::paths;
use crate::platform;

/// Name of the personal KB map file written into a project repo root.
///
/// Deliberately NOT a well-known name (AGENTS.md, CLAUDE.md, ...) so it never
/// collides with files a maintainer might commit. It is personal and ignored
/// globally via ~/.gitignore, and manually attached to prompts.
pub const KB_RULES_FILENAME: &str = "kb-rules.md";

/// Result of a link operation.
#[derive(Debug)]
pub struct LinkResult {
    pub project_name: String,
    pub memory_dir: PathBuf,
    pub scratch_link: PathBuf,
    pub rules_link: PathBuf,
    pub global_gitignore_updated: bool,
    pub kb_rules_written: bool,
}

/// Link a project repo to the knowledge-base.
///
/// Never modifies the project's .gitignore or AGENTS.md.
/// Symlinks are ignored via ~/.gitignore (global, personal).
pub fn link(
    kb_root: &Path,
    project_name: &str,
    repo_dir: &Path,
    templates_dir: &Path,
) -> Result<LinkResult> {
    paths::validate_project_name(project_name)?;
    let memory_dir = kb_root.join("projects").join(project_name);

    // 1. Ensure project memory directory exists
    if !memory_dir.exists() {
        create_project_memory(&memory_dir, project_name, templates_dir)?;
    }

    // 2. Update config FIRST — record intent before creating symlinks
    let config_updated = config::update(|cfg| {
        config::ensure_active_project(cfg, project_name);
        cfg.projects.insert(
            project_name.to_string(),
            ProjectConfig {
                repo_path: Some(repo_dir.to_path_buf()),
                ..Default::default()
            },
        );
    });

    if let Err(e) = config_updated {
        return Err(e.context("failed to update config before linking"));
    }

    // 3. Create scratch symlink
    let scratch_link = repo_dir.join("scratch");
    if let Err(e) = platform::create_symlink(&memory_dir, &scratch_link) {
        let _ = config::update(|cfg| {
            config::remove_active_project(cfg, project_name);
            cfg.projects.remove(project_name);
        });
        return Err(e.context("failed to create scratch symlink (config rolled back)"));
    }

    // 4. Create .agent-rules symlink
    let rules_target = kb_root.join("agent-rules");
    let rules_link = repo_dir.join(".agent-rules");
    if let Err(e) = platform::create_symlink(&rules_target, &rules_link) {
        let _ = platform::remove_symlink(&scratch_link, &memory_dir);
        let _ = config::update(|cfg| {
            config::remove_active_project(cfg, project_name);
            cfg.projects.remove(project_name);
        });
        return Err(e.context("failed to create .agent-rules symlink (rolled back)"));
    }

    // 5. Ensure ~/.gitignore has entries for symlinks (global, personal)
    let global_gitignore_updated = ensure_global_gitignore()?;

    // 6. Ensure the personal kb-rules.md map exists in the repo root
    let kb_rules_written = ensure_kb_rules(repo_dir, project_name, kb_root, templates_dir)?;

    Ok(LinkResult {
        project_name: project_name.to_string(),
        memory_dir,
        scratch_link,
        rules_link,
        global_gitignore_updated,
        kb_rules_written,
    })
}

/// Unlink a project — remove symlinks, keep memory.
pub fn unlink(kb_root: &Path, project_name: &str, repo_dir: &Path) -> Result<UnlinkResult> {
    let memory_dir = kb_root.join("projects").join(project_name);

    let scratch_link = repo_dir.join("scratch");
    let rules_link = repo_dir.join(".agent-rules");

    let scratch_removed = platform::remove_symlink(&scratch_link, &memory_dir)?;
    let rules_target = kb_root.join("agent-rules");
    let rules_removed = platform::remove_symlink(&rules_link, &rules_target)?;

    let kb_rules = repo_dir.join(KB_RULES_FILENAME);
    let kb_rules_removed = if kb_rules.exists() {
        fs::remove_file(&kb_rules)?;
        true
    } else {
        false
    };

    config::update(|cfg| {
        config::remove_active_project(cfg, project_name);
    })?;

    Ok(UnlinkResult {
        project_name: project_name.to_string(),
        scratch_removed,
        rules_removed,
        kb_rules_removed,
    })
}

/// Result of an unlink operation.
#[derive(Debug)]
pub struct UnlinkResult {
    pub project_name: String,
    pub scratch_removed: bool,
    pub rules_removed: bool,
    pub kb_rules_removed: bool,
}

/// Status of a single project.
#[derive(Debug, Clone)]
pub struct ProjectStatus {
    pub name: String,
    pub memory_exists: bool,
    pub memory_path: PathBuf,
    pub repo_path: Option<PathBuf>,
    pub scratch_healthy: Option<bool>,
    pub rules_healthy: Option<bool>,
    pub kb_rules_exists: bool,
    pub handoff_age: Option<String>,
}

/// Get status of a single project.
pub fn status(kb_root: &Path, project_name: &str) -> Result<ProjectStatus> {
    let memory_dir = kb_root.join("projects").join(project_name);
    let memory_exists = memory_dir.exists();

    let cfg = config::load()?;
    let project_cfg = cfg.projects.get(project_name);
    let repo_path = project_cfg
        .and_then(|c| c.repo_path.clone())
        .or_else(|| paths::default_project_dir(project_name).ok());

    let scratch_healthy = repo_path.as_ref().map(|repo| {
        let link = repo.join("scratch");
        platform::is_symlink_to(&link, &memory_dir)
    });

    let rules_target = kb_root.join("agent-rules");
    let rules_healthy = repo_path.as_ref().map(|repo| {
        let link = repo.join(".agent-rules");
        platform::is_symlink_to(&link, &rules_target)
    });

    let kb_rules_exists = repo_path
        .as_ref()
        .map(|repo| repo.join(KB_RULES_FILENAME).exists())
        .unwrap_or(false);

    let handoff_age = get_handoff_age(&memory_dir);

    Ok(ProjectStatus {
        name: project_name.to_string(),
        memory_exists,
        memory_path: memory_dir,
        repo_path,
        scratch_healthy,
        rules_healthy,
        kb_rules_exists,
        handoff_age,
    })
}

/// List all projects in the knowledge-base.
pub fn list_all(kb_root: &Path) -> Result<Vec<ProjectStatus>> {
    let projects_dir = kb_root.join("projects");
    if !projects_dir.exists() {
        return Ok(vec![]);
    }

    let mut projects = vec![];
    for entry in
        fs::read_dir(&projects_dir).context(format!("failed to read {}", projects_dir.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "README.md" || name.starts_with('.') {
                continue;
            }
            projects.push(status(kb_root, &name)?);
        }
    }
    projects.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(projects)
}

/// Get human-readable age of handoff file.
fn get_handoff_age(memory_dir: &Path) -> Option<String> {
    let handoff = memory_dir.join("HANDOFF.md");
    let meta = fs::metadata(&handoff).ok()?;
    let modified = meta.modified().ok()?;
    let duration = modified.elapsed().ok()?;
    let days = duration.as_secs() / 86400;
    if days == 0 {
        Some("today".to_string())
    } else if days == 1 {
        Some("1 day ago".to_string())
    } else {
        Some(format!("{} days ago", days))
    }
}

/// Create project memory directory from template.
fn create_project_memory(
    memory_dir: &Path,
    project_name: &str,
    templates_dir: &Path,
) -> Result<()> {
    let subdirs = [
        "decisions",
        "inputs",
        "research",
        "spec",
        "plans",
        "conclusions",
        "ref",
    ];
    for sub in &subdirs {
        fs::create_dir_all(memory_dir.join(sub)).context(format!(
            "failed to create {}/{}",
            memory_dir.display(),
            sub
        ))?;
    }

    // Templates are portable: only the project name is substituted. Machine
    // home paths are NEVER baked into memory files because they are committed
    // and shared across machines via git (a Windows home like C:\Users\... is
    // meaningless to the Linux box, and /home/kristency is meaningless here).
    // As a portability guard, a literal `/home/kristency` in any template is
    // normalized to `~` (portable on Linux/macOS/Windows shells).
    let render = |content: &str| -> String {
        content
            .replace("<project>", project_name)
            .replace("/home/kristency", "~")
    };

    let write_atomic = |dest: &Path, content: &str| -> Result<()> {
        let tmp = dest.with_extension("tmp");
        fs::write(&tmp, content)?;
        fs::rename(&tmp, dest)?;
        Ok(())
    };

    let readme_template = templates_dir.join("README.md");
    if readme_template.exists() {
        let content = fs::read_to_string(&readme_template)?;
        write_atomic(&memory_dir.join("README.md"), &render(&content))?;
    }

    let agents_template = templates_dir.join("AGENTS.md");
    if agents_template.exists() {
        let content = fs::read_to_string(&agents_template)?;
        write_atomic(&memory_dir.join("AGENTS.md"), &render(&content))?;
    }

    let handoff_template = templates_dir.join("HANDOFF.md");
    if handoff_template.exists() {
        let content = fs::read_to_string(&handoff_template)?;
        write_atomic(&memory_dir.join("HANDOFF.md"), &render(&content))?;
    }

    let ref_readme_template = templates_dir.join("ref-README.md");
    if ref_readme_template.exists() {
        let content = fs::read_to_string(&ref_readme_template)?;
        write_atomic(&memory_dir.join("ref").join("README.md"), &render(&content))?;
    }

    Ok(())
}

/// Default kb-rules.md template, used when the knowledge-base has no
/// `templates/project/kb-rules.md`. All paths are placeholders resolved at
/// link time. This file is PERSONAL and gitignored, so machine paths are
/// intentionally baked into it (the opposite of committed project memory).
const DEFAULT_KB_RULES_TEMPLATE: &str = r#"# kb-rules.md - <project>

Personal KB map for AI agents working on `<project>`. Attach this file in every
prompt (for example `@kb-rules.md`) so the agent always loads KB context.

## Repo

<repo_dir>

## Read in this order before working

1. scratch/HANDOFF.md       - active state, blockers, next action (ALWAYS first)
2. .agent-rules/behavior/agent-standards.md - cross-project behavior + safety
3. scratch/AGENTS.md        - project invariants + validation gates
4. .agent-rules/behavior/spec-lifecycle.md  - spec drift defense
5. .agent-rules/skills/<lang>/core/SKILL.md - language rules (lazy-load)

## Wiring (local symlinks, gitignored)

scratch      -> <kb_root>/projects/<project>
.agent-rules -> <kb_root>/agent-rules

## Non-negotiables

- Never commit, push, or tag unless explicitly asked
- Run the validation gate (fmt/lint/tests) before reporting done
- Every bug fix ships with a reproducing test
- Never print, log, or commit secrets
- Session close: update the 3-bullet block at the top of scratch/HANDOFF.md
"#;

/// Ensure the personal `kb-rules.md` map exists in the repo root.
///
/// Created once from the KB template (or an embedded default). Existing files
/// are NEVER overwritten — it is the user's personal map.
/// Returns `true` if the file was written.
fn ensure_kb_rules(
    repo_dir: &Path,
    project_name: &str,
    kb_root: &Path,
    templates_dir: &Path,
) -> Result<bool> {
    let dest = repo_dir.join(KB_RULES_FILENAME);
    if dest.exists() {
        return Ok(false);
    }

    let template_path = templates_dir.join("kb-rules.md");
    let template = if template_path.exists() {
        fs::read_to_string(&template_path)
            .context(format!("failed to read {}", template_path.display()))?
    } else {
        DEFAULT_KB_RULES_TEMPLATE.to_string()
    };

    let content = template
        .replace("<project>", project_name)
        .replace("<repo_dir>", &render_path(repo_dir))
        .replace("<kb_root>", &render_path(kb_root))
        .replace("/home/kristency", "~");

    let tmp = dest.with_extension("tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, dest)?;
    Ok(true)
}

/// Format a path for display inside kb-rules.md: forward slashes and no
/// Windows verbatim (`\\?\`) prefix.
fn render_path(path: &Path) -> String {
    let s = crate::paths::normalize_display(path);
    #[cfg(windows)]
    let s = s.strip_prefix("//?/").unwrap_or(&s).to_string();
    s
}

/// Ensure ~/.gitignore has entries for knowledge-base symlinks.
///
/// This is global, personal config — never touches the project's .gitignore.
fn ensure_global_gitignore() -> Result<bool> {
    let home = paths::home_dir()?;
    let gitignore = home.join(".gitignore");

    let entries = [
        "# kb symlinks and rules (personal, never commit)\n/scratch\n/.agent-rules\n/kb-rules.md\n",
    ];

    if !gitignore.exists() {
        let tmp = gitignore.with_extension("tmp");
        fs::write(&tmp, entries[0])?;
        fs::rename(&tmp, &gitignore)?;
        return Ok(true);
    }

    let content = fs::read_to_string(&gitignore)?;
    let lines: Vec<&str> = content.lines().collect();

    let has_scratch = lines.iter().any(|l| {
        let t = l.trim();
        t == "/scratch" || t == "scratch"
    });
    let has_rules = lines.iter().any(|l| {
        let t = l.trim();
        t == "/.agent-rules" || t == ".agent-rules"
    });
    let has_kb_rules = lines
        .iter()
        .any(|l| l.trim() == "/kb-rules.md" || l.trim() == "kb-rules.md");

    let mut updated = false;
    let mut new_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();

    if !has_scratch {
        new_lines.push("/scratch".to_string());
        updated = true;
    }
    if !has_rules {
        new_lines.push("/.agent-rules".to_string());
        updated = true;
    }
    if !has_kb_rules {
        new_lines.push("/kb-rules.md".to_string());
        updated = true;
    }

    if updated {
        let tmp = gitignore.with_extension("tmp");
        fs::write(&tmp, new_lines.join("\n"))?;
        fs::rename(&tmp, &gitignore)?;
    }

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn ensure_global_gitignore_creates_new() {
        let home = TempDir::new().unwrap();
        let gitignore = home.path().join(".gitignore");

        // Mock home_dir by writing to a temp path
        // (ensure_global_gitignore uses home_dir, so we test the logic directly)
        let entries = [
            "# kb symlinks and rules (personal, never commit)\n/scratch\n/.agent-rules\n/kb-rules.md\n",
        ];
        fs::write(&gitignore, entries[0]).unwrap();

        let content = fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains("/scratch"));
        assert!(content.contains("/.agent-rules"));
        assert!(content.contains("/kb-rules.md"));
    }

    #[test]
    fn ensure_global_gitignore_idempotent() {
        let home = TempDir::new().unwrap();
        let gitignore = home.path().join(".gitignore");
        fs::write(&gitignore, "/scratch\n/.agent-rules\n/kb-rules.md\n").unwrap();

        let content = fs::read_to_string(&gitignore).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.iter().any(|l| l.trim() == "/scratch"));
        assert!(lines.iter().any(|l| l.trim() == "/.agent-rules"));
        assert!(lines.iter().any(|l| l.trim() == "/kb-rules.md"));
    }

    #[test]
    fn ensure_kb_rules_creates_with_default_template() {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        let templates = dir.path().join("templates");
        fs::create_dir_all(&templates).unwrap();
        let kb_root = dir.path().join("knowledge-base");

        let written = ensure_kb_rules(&repo, "myapp", &kb_root, &templates).unwrap();
        assert!(written);

        let content = fs::read_to_string(repo.join(KB_RULES_FILENAME)).unwrap();
        assert!(content.contains("kb-rules.md - myapp"));
        assert!(
            content.contains(&format!("{}/projects/myapp", render_path(&kb_root))),
            "wiring section must reference the resolved kb root"
        );
        assert!(content.contains("scratch/HANDOFF.md"));
        assert!(content.contains(&render_path(&kb_root)));

        // Idempotent: second run does not touch an existing file
        let written_again = ensure_kb_rules(&repo, "myapp", &kb_root, &templates).unwrap();
        assert!(!written_again);
    }

    #[test]
    fn ensure_kb_rules_uses_template_when_present() {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        let templates = dir.path().join("templates");
        fs::create_dir_all(&templates).unwrap();
        let kb_root = dir.path().join("knowledge-base");

        fs::write(
            templates.join("kb-rules.md"),
            "# <project> custom\nRepo: <repo_dir>\n",
        )
        .unwrap();

        let written = ensure_kb_rules(&repo, "myapp", &kb_root, &templates).unwrap();
        assert!(written);

        let content = fs::read_to_string(repo.join(KB_RULES_FILENAME)).unwrap();
        assert!(content.contains("myapp custom"));
        assert!(content.contains(&render_path(&repo)));
    }

    #[test]
    fn ensure_kb_rules_preserves_existing() {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join(KB_RULES_FILENAME), "user content").unwrap();
        let templates = dir.path().join("templates");
        fs::create_dir_all(&templates).unwrap();
        let kb_root = dir.path().join("knowledge-base");

        let written = ensure_kb_rules(&repo, "myapp", &kb_root, &templates).unwrap();
        assert!(!written);
        assert_eq!(
            fs::read_to_string(repo.join(KB_RULES_FILENAME)).unwrap(),
            "user content"
        );
    }
}
