use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn kb_bin() -> Command {
    Command::cargo_bin("kb").unwrap()
}

/// Helper: create a fake KB root with required files.
fn fake_kb_root(dir: &Path) -> PathBuf {
    let kb = dir.join("knowledge-base");
    fs::create_dir_all(&kb).unwrap();
    fs::write(kb.join("AGENTS.md"), "# Agent Rules").unwrap();
    fs::write(kb.join("INDEX.md"), "# Index").unwrap();
    fs::create_dir_all(kb.join("projects")).unwrap();
    fs::create_dir_all(kb.join("agent-rules")).unwrap();
    fs::create_dir_all(kb.join("templates/project")).unwrap();
    kb
}

/// Helper: create a fake project repo.
fn fake_project_repo(dir: &Path, name: &str) -> PathBuf {
    let repo = dir.join("repos").join(name);
    fs::create_dir_all(&repo).unwrap();
    repo
}

#[test]
fn test_help() {
    kb_bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("knowledge-base"));
}

#[test]
fn test_version() {
    kb_bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("kb 0.1.0"));
}

#[test]
fn test_init_with_flag() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    // Override config path by setting HOME
    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .arg("init")
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("KB root"));
}

#[test]
fn test_link_and_status() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "myapp");

    // Set HOME so config goes to our temp dir
    let home_dir = TempDir::new().unwrap();

    // Link
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Linking"));

    // Verify symlinks created
    assert!(
        repo.join("scratch")
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        repo.join(".agent-rules")
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );

    // Verify global ~/.gitignore updated (not project .gitignore)
    let gitignore = fs::read_to_string(home_dir.path().join(".gitignore")).unwrap();
    assert!(gitignore.contains("/scratch"));
    assert!(gitignore.contains("/.agent-rules"));
    assert!(gitignore.contains("/kb-rules.md"));

    // Verify the personal kb-rules.md map was written into the repo root
    let kb_rules = fs::read_to_string(repo.join("kb-rules.md")).unwrap();
    assert!(kb_rules.contains("kb-rules.md"));
    assert!(kb_rules.contains("scratch/HANDOFF.md"));
    let repo_display = repo.to_string_lossy().replace('\\', "/");
    assert!(kb_rules.contains(&repo_display));

    // Verify project .gitignore was NOT modified
    assert!(!repo.join(".gitignore").exists());

    // Status should show the project
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .arg("status")
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("myapp"));
}

#[test]
fn test_unlink() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "myapp");

    let home_dir = TempDir::new().unwrap();

    // Link first
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    // Unlink
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["unlink", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Unlinking"));

    // Verify symlinks removed
    assert!(!repo.join("scratch").exists());
    assert!(!repo.join(".agent-rules").exists());
    assert!(!repo.join("kb-rules.md").exists());

    // But memory still exists
    assert!(kb.join("projects/myapp").exists());
}

#[test]
fn test_doctor() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .arg("doctor")
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Doctor Results"));
}

#[test]
fn test_projects_list() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .arg("projects")
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Projects"));
}

#[test]
fn test_link_idempotent() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "myapp");

    let home_dir = TempDir::new().unwrap();

    // Link twice — should not fail
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Linking"));
}

#[test]
fn test_link_by_name() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    // Create ~/Projects/myapp structure
    let home_dir = TempDir::new().unwrap();
    let projects = home_dir.path().join("Projects").join("myapp");
    fs::create_dir_all(&projects).unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", "myapp"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Linking myapp"));
}

#[test]
fn test_link_missing_project_fails() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", "nonexistent"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_link_memory_is_portable() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    // Templates are portable: only <project> gets substituted. A scaffold
    // must never bake a machine's home path (e.g. C:\Users\... on Windows
    // or /home/<user> on Linux) into committed, shared memory files. This
    // simulates an old legacy template that still contained the hardcoded
    // Linux home — the renderer must normalize it to `~`, not replace it
    // with the current machine's home.
    fs::write(
        kb.join("templates/project/README.md"),
        "# <project>\n\nProject repo: /home/kristency/Projects/<project>\n",
    )
    .unwrap();
    fs::write(
        kb.join("templates/project/ref-README.md"),
        "## Local Checkouts\n\n```text\nknowledge-base/projects/<project>/ref/\n```\n",
    )
    .unwrap();

    let home_dir = TempDir::new().unwrap();
    let repo = fake_project_repo(dir.path(), "myapp");

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    let readme = fs::read_to_string(kb.join("projects/myapp/README.md")).unwrap();
    let ref_readme = fs::read_to_string(kb.join("projects/myapp/ref/README.md")).unwrap();

    let home = home_dir.path().to_string_lossy().to_lowercase();
    assert!(
        !readme.to_lowercase().contains(&home),
        "README must not contain the machine home path"
    );
    assert!(
        !ref_readme.to_lowercase().contains(&home),
        "ref README must not contain the machine home path"
    );
    assert!(!readme.contains("kristency"));
    assert!(!ref_readme.contains("kristency"));

    // Project-name substitution still works, and the legacy hardcoded home
    // was normalized to a portable `~` instead of a machine-specific path.
    assert!(readme.contains("~/Projects/myapp"));
    assert!(ref_readme.contains("myapp"));
}

#[test]
fn test_link_writes_kb_rules_from_template() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    // The kb-rules.md template is portable (placeholders only, no machine
    // paths) because it ships inside the shared KB repo. Machine paths are
    // baked in at link time — the rendered file is personal + gitignored.
    fs::write(
        kb.join("templates/project/kb-rules.md"),
        "# <project> rules\nRepo: <repo_dir>\nKB: <kb_root>\n",
    )
    .unwrap();

    let home_dir = TempDir::new().unwrap();
    let repo = fake_project_repo(dir.path(), "myapp");

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("kb-rules.md"));

    let content = fs::read_to_string(repo.join("kb-rules.md")).unwrap();
    assert!(content.contains("# myapp rules"));
    let repo_display = repo.to_string_lossy().replace('\\', "/");
    assert!(content.contains(&format!("Repo: {}", repo_display)));
    let canonical_kb = kb.canonicalize().unwrap();
    let kb_display = canonical_kb.to_string_lossy().replace('\\', "/");
    let kb_display = kb_display.strip_prefix("//?/").unwrap_or(&kb_display);
    assert!(content.contains(&format!("KB: {}", kb_display)));

    // Link again: file is preserved, not re-written
    let before = fs::read_to_string(repo.join("kb-rules.md")).unwrap();
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("already present"));
    assert_eq!(
        fs::read_to_string(repo.join("kb-rules.md")).unwrap(),
        before
    );
}
