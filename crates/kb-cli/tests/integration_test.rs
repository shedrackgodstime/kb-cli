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
