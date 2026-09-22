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

/// Helper: remove a (possibly broken) symlink cross-platform.
fn remove_path(path: &Path) {
    if path.symlink_metadata().is_ok() || path.exists() {
        let _ = fs::remove_dir(path);
        let _ = fs::remove_file(path);
    }
}

/// Helper: run git in a directory.
fn git(args: &[&str], cwd: &Path) -> std::process::Output {
    std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

/// Helper: commit everything in a git repo with a fixed identity.
fn git_commit_all(cwd: &Path, msg: &str) {
    assert!(git(&["add", "-A"], cwd).status.success());
    let out = git(
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            msg,
        ],
        cwd,
    );
    assert!(out.status.success(), "git commit failed: {:?}", out);
}

/// Helper: init a KB git repo with an empty bare origin; returns (kb, bare).
fn setup_kb_git_repo(dir: &Path) -> (PathBuf, PathBuf) {
    let kb = fake_kb_root(dir);
    let bare = dir.join("origin.git");
    // The bare origin must default to `main` too: CI git defaults to
    // `master`, which would leave the bare HEAD (and clones) on the wrong
    // branch, so pushes to `main` would never advance `origin/main`.
    let mut bare_init = std::process::Command::new("git");
    bare_init
        .args(["init", "--bare", "-b", "main", bare.to_str().unwrap()])
        .current_dir(dir);
    if !bare_init.output().unwrap().status.success() {
        // old git: init, then point the bare HEAD at main explicitly.
        assert!(
            git(&["init", "--bare", bare.to_str().unwrap()], dir)
                .status
                .success()
        );
        assert!(
            git(&["symbolic-ref", "HEAD", "refs/heads/main"], &bare)
                .status
                .success()
        );
    }

    let mut git_init = std::process::Command::new("git");
    git_init.args(["init", "-b", "main"]).current_dir(&kb);
    if !git_init.output().unwrap().status.success() {
        // old git: fall back to init + branch -M main
        assert!(git(&["init"], &kb).status.success());
        assert!(git(&["branch", "-M", "main"], &kb).status.success());
    }

    // Repo-local identity so `kb global-sync`'s git commit works under a
    // test HOME override even when the user's global config isn't present.
    assert!(git(&["config", "user.name", "Test"], &kb).status.success());
    assert!(
        git(&["config", "user.email", "test@example.com"], &kb)
            .status
            .success()
    );

    git_commit_all(&kb, "init");
    assert!(
        git(&["remote", "add", "origin", bare.to_str().unwrap()], &kb)
            .status
            .success()
    );
    (kb, bare)
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
        .stdout(predicate::str::contains(format!(
            "kb {}",
            env!("CARGO_PKG_VERSION")
        )));
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
fn test_global_sync_works_on_sparse_worktree() {
    let dir = TempDir::new().unwrap();
    let (kb, bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    // Convert to sparse: only alpha subscribed.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    assert!(!kb.join("projects/beta").exists());

    // Edit subscribed + always-on files while unsubscribed memory stays out.
    fs::write(kb.join("projects/alpha/HANDOFF.md"), "# updated\n").unwrap();
    fs::write(kb.join("INDEX.md"), "# index v2\n").unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["global-sync", "--no-link"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Commit"))
        .stdout(predicate::str::contains("Push"));

    // Remote got the sync and the worktree is clean afterwards.
    let log = git(&["log", "--oneline"], &bare);
    assert!(
        String::from_utf8_lossy(&log.stdout).contains("sync knowledge base"),
        "{}",
        String::from_utf8_lossy(&log.stdout)
    );
    let status = git(&["status", "--porcelain"], &kb);
    assert!(String::from_utf8_lossy(&status.stdout).trim().is_empty());
    assert!(!kb.join("projects/beta").exists());
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

#[test]
fn test_global_sync_commits_and_pushes_everything() {
    let dir = TempDir::new().unwrap();
    let (kb, bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    // An entirely new project memory file (untracked) + an edit
    fs::create_dir_all(kb.join("projects/fresh")).unwrap();
    fs::write(kb.join("projects/fresh/HANDOFF.md"), "# fresh\n").unwrap();
    fs::write(kb.join("INDEX.md"), "# updated index\n").unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["global-sync", "--no-link"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Commit"))
        .stdout(predicate::str::contains("Push"));

    // Everything was pushed to the bare origin
    let log = git(&["log", "--oneline"], &bare);
    let log_str = String::from_utf8_lossy(&log.stdout);
    assert!(log_str.contains("sync knowledge base"));

    // Working tree is clean afterwards
    let status = git(&["status", "--porcelain"], &kb);
    assert!(String::from_utf8_lossy(&status.stdout).trim().is_empty());
}

#[test]
fn test_global_sync_detects_divergence() {
    let dir = TempDir::new().unwrap();
    let (kb, bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    // Push the initial commit to origin first.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["global-sync", "--no-link"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    // Second machine clones the KB and pushes a change.
    let clone = dir.path().join("clone");
    assert!(
        git(
            &["clone", bare.to_str().unwrap(), clone.to_str().unwrap()],
            dir.path()
        )
        .status
        .success()
    );
    fs::write(clone.join("REMOTE.md"), "remote change\n").unwrap();
    git_commit_all(&clone, "remote change");
    assert!(git(&["push"], &clone).status.success());

    // Local KB now makes its own (conflicting) commit without pulling.
    fs::write(kb.join("LOCAL.md"), "local change\n").unwrap();
    git_commit_all(&kb, "local change");

    // The commit counters reset fetch state; run global-sync afterwards.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["global-sync", "--no-link"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("diverged"), "stdout: {}", stdout);

    // Nothing was pushed beyond the remote change.
    let log = git(&["log", "--oneline", "origin/main"], &clone);
    let log_str = String::from_utf8_lossy(&log.stdout);
    assert!(!log_str.contains("local change"));
    assert!(log_str.contains("remote change"));
}

#[test]
fn test_global_sync_behind_pulls_then_commits_nothing() {
    let dir = TempDir::new().unwrap();
    let (kb, bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    // Push initial commit.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["global-sync", "--no-link"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    // Remote gains a commit.
    let clone = dir.path().join("clone");
    assert!(
        git(
            &["clone", bare.to_str().unwrap(), clone.to_str().unwrap()],
            dir.path()
        )
        .status
        .success()
    );
    fs::write(clone.join("REMOTE2.md"), "remote 2\n").unwrap();
    git_commit_all(&clone, "remote 2");
    assert!(git(&["push"], &clone).status.success());

    // Local is now behind but clean.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["global-sync", "--no-link"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("fast-forwarded"), "stdout: {}", stdout);
    assert!(stdout.contains("nothing to commit"));

    // Local HEAD points at the remote commit now.
    let local_output = git(&["log", "--oneline", "-1"], &kb);
    let remote_output = git(&["log", "--oneline", "-1"], &clone);
    let local_head = String::from_utf8_lossy(&local_output.stdout);
    let remote_head = String::from_utf8_lossy(&remote_output.stdout);
    assert_eq!(local_head.trim(), remote_head.trim());
}

#[test]
fn test_global_sync_dry_run_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let (kb, bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    fs::write(kb.join("INDEX.md"), "# dry run\n").unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["global-sync", "--no-link", "--dry-run"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run"));

    // Remote untouched, local change still present but uncommitted.
    let log = git(&["log", "--oneline"], &bare);
    assert!(!String::from_utf8_lossy(&log.stdout).contains("dry run"));
    let status = git(&["status", "--porcelain"], &kb);
    assert!(String::from_utf8_lossy(&status.stdout).contains("INDEX.md"));
}

#[test]
fn test_search_finds_matches() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    fs::create_dir_all(kb.join("projects/alpha/spec")).unwrap();
    fs::write(
        kb.join("projects/alpha/HANDOFF.md"),
        "alpha handoff\nblocker: rust cache\n",
    )
    .unwrap();
    fs::write(
        kb.join("projects/alpha/spec/01.md"),
        "alpha spec with a Blocker quote\n",
    )
    .unwrap();

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["search", "blocker"])
        .output()
        .unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("projects/alpha/HANDOFF.md:2"),
        "stdout: {}",
        stdout
    );
    assert!(stdout.contains("projects/alpha/spec/01.md:1"));
}

#[test]
fn test_search_project_filter_and_files_only() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    fs::create_dir_all(kb.join("projects/alpha")).unwrap();
    fs::create_dir_all(kb.join("projects/beta")).unwrap();
    fs::write(kb.join("projects/alpha/HANDOFF.md"), "alpha blocker\n").unwrap();
    fs::write(kb.join("projects/beta/HANDOFF.md"), "beta blocker\n").unwrap();

    // Filtered to one project.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["search", "blocker", "--project", "alpha"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("projects/alpha/HANDOFF.md"));
    assert!(!stdout.contains("projects/beta"));

    // files-only dedupes to one path and shows no line numbers.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["search", "blocker", "--project", "alpha", "--files-only"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.matches("projects/alpha/HANDOFF.md").count(), 1);
    assert!(!stdout.contains(":1:"));
}

#[test]
fn test_search_no_matches_exits_1_and_json_shape() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());

    // No hits -> exit code 1.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["search", "zzz-nonexistent"])
        .assert()
        .code(1);

    // Hits -> JSON output shape.
    fs::create_dir_all(kb.join("projects/alpha")).unwrap();
    fs::write(kb.join("projects/alpha/HANDOFF.md"), "needle here\n").unwrap();

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["search", "here", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"query\": \"here\""));
    assert!(stdout.contains("\"path\": \"projects/alpha/HANDOFF.md\""));
    assert!(stdout.contains("\"line\": 1"));
    assert!(stdout.contains("\"project\": \"alpha\""));
    assert!(stdout.contains("\"count\": 1"));
}

#[test]
fn test_rules_creates_map() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "alpha");

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["rules", repo.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("written"), "stdout: {}", stdout);
    assert!(repo.join("kb-rules.md").exists());
    let content = fs::read_to_string(repo.join("kb-rules.md")).unwrap();
    assert!(content.contains("kb-rules.md - alpha"));
}

#[test]
fn test_rules_up_to_date_and_json() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "alpha");

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["rules", repo.to_str().unwrap()])
        .assert()
        .success();

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["rules", repo.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"action\": \"up-to-date\""));
    assert!(stdout.contains("\"project\": \"alpha\""));
    assert!(stdout.contains("\"file\""));
}

#[test]
fn test_rules_preserves_local_edits() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "alpha");
    fs::write(repo.join("kb-rules.md"), "my personal map\n").unwrap();

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["rules", repo.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("skipped (keep local edits)"),
        "stdout: {}",
        stdout
    );
    assert_eq!(
        fs::read_to_string(repo.join("kb-rules.md")).unwrap(),
        "my personal map\n"
    );
}

#[test]
fn test_rules_dry_run_does_not_write() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "alpha");

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["rules", repo.to_str().unwrap(), "--dry-run"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dry run"), "stdout: {}", stdout);
    assert!(!repo.join("kb-rules.md").exists());
    assert!(stdout.contains("would be written"));
}

#[test]
fn test_rules_all_uses_configured_repos() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "alpha");

    // Point the machine-local config at the fake repo so `--all` finds it.
    let home_dir = TempDir::new().unwrap();
    let config_dir = home_dir.path().join(".kb");
    fs::create_dir_all(&config_dir).unwrap();
    let repo_path = repo.to_str().unwrap().replace('\\', "/");
    fs::write(
        config_dir.join("config.toml"),
        format!("[projects.alpha]\nrepo_path = \"{}\"\n", repo_path),
    )
    .unwrap();

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["rules", "--all"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("alpha"), "stdout: {}", stdout);
    assert!(repo.join("kb-rules.md").exists());
}

#[test]
fn test_config_set_get_roundtrip() {
    let dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();
    let repo = fake_project_repo(dir.path(), "alpha");
    let repo_path = repo.to_str().unwrap().replace('\\', "/");

    let set = kb_bin()
        .args([
            "config",
            "set",
            "active_projects",
            r#"["alpha","beta"]"#,
            "--json",
        ])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(set.status.success(), "set failed: {:?}", set.status.code());

    let set = kb_bin()
        .args(["config", "set", "kb_root", "/kb/root"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(set.status.success());

    let set = kb_bin()
        .args(["config", "set", "projects.alpha.repo_path", &repo_path])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(set.status.success());

    let get = kb_bin()
        .args(["config", "get", "active_projects", "--json"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(get.status.success());
    let stdout = String::from_utf8_lossy(&get.stdout);
    assert!(stdout.contains("\"key\": \"active_projects\""));
    assert!(stdout.contains("\"alpha\""), "stdout: {}", stdout);
    assert!(stdout.contains("\"beta\""));

    let get = kb_bin()
        .args(["config", "get", "projects.alpha.clone_depth"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(get.status.success());
    assert!(String::from_utf8_lossy(&get.stdout).contains("= 0"));
}

#[test]
fn test_config_set_roundtrips_to_disk() {
    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["config", "set", "projects.irosh.clone_depth", "1"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    let config_file = home_dir.path().join(".kb").join("config.toml");
    assert!(config_file.exists());
    let content = fs::read_to_string(&config_file).unwrap();
    assert!(content.contains("clone_depth = 1"), "content: {}", content);
}

#[test]
fn test_config_invalid_key_errors_with_valid_list() {
    let home_dir = TempDir::new().unwrap();
    kb_bin()
        .args(["config", "set", "bogus", "x"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("valid keys"));
}

#[test]
fn test_config_list_json_shape() {
    let home_dir = TempDir::new().unwrap();
    let out = kb_bin()
        .args(["config", "list", "--json"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"ok\": true"));
    assert!(stdout.contains("\"config\""));
    assert!(stdout.contains("\"active_projects\""));
}

#[test]
fn test_log_shows_recent_commits() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .arg("log")
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("init"));
}

#[test]
fn test_log_project_filter() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    fs::create_dir_all(kb.join("projects/alpha")).unwrap();
    fs::write(kb.join("projects/alpha/NOTES.md"), "alpha\n").unwrap();
    git_commit_all(&kb, "alpha work");

    fs::create_dir_all(kb.join("projects/beta")).unwrap();
    fs::write(kb.join("projects/beta/NOTES.md"), "beta\n").unwrap();
    git_commit_all(&kb, "beta work");

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["log", "--project", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("alpha work"), "stdout: {}", stdout);
    assert!(!stdout.contains("beta work"), "stdout: {}", stdout);
}

#[test]
fn test_log_limit_restricts_count() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    for msg in ["change one", "change two", "change three"] {
        fs::write(kb.join("INDEX.md"), format!("# {msg}\n")).unwrap();
        git_commit_all(&kb, msg);
    }

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["log", "-n", "2"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("change three"), "stdout: {}", stdout);
    assert!(stdout.contains("change two"), "stdout: {}", stdout);
    assert!(!stdout.contains("change one"), "stdout: {}", stdout);
}

#[test]
fn test_log_stat_shows_changes() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    fs::write(kb.join("INDEX.md"), "# stat test\n").unwrap();
    git_commit_all(&kb, "stat change");

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["log", "--stat", "--limit", "1"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("stat change"), "stdout: {}", stdout);
    assert!(stdout.contains("INDEX.md"), "stdout: {}", stdout);
}

#[test]
fn test_log_json_shape() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["log", "--json"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"ok\": true"), "stdout: {}", stdout);
    assert!(stdout.contains("\"entries\""), "stdout: {}", stdout);
    assert!(stdout.contains("\"hash\""), "stdout: {}", stdout);
}

#[test]
fn test_log_not_git_repo_errors() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .arg("log")
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a git repository"));
}

#[test]
fn test_config_unset_clears_value() {
    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["config", "set", "active_projects", r#"["alpha"]"#])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    kb_bin()
        .args(["config", "unset", "active_projects"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    let get = kb_bin()
        .args(["config", "get", "active_projects", "--json"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(get.status.success());
    let stdout = String::from_utf8_lossy(&get.stdout);
    assert!(
        stdout.contains("\"value\": null") || stdout.contains("\"value\": []"),
        "stdout: {}",
        stdout
    );
}

/// Seed a KB repo with committed top-level dir and project memories.
fn seed_memory(kb: &Path) {
    fs::create_dir_all(kb.join("agent-rules")).unwrap();
    fs::write(kb.join("agent-rules/README.md"), "# agent rules\n").unwrap();
    for name in ["alpha", "beta"] {
        fs::create_dir_all(kb.join("projects").join(name)).unwrap();
        fs::write(
            kb.join("projects").join(name).join("HANDOFF.md"),
            format!("# {name}\n"),
        )
        .unwrap();
    }
    git_commit_all(kb, "seed memory");
}

#[test]
fn test_subscribe_roundtrip() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    // Subscribe to alpha: enables sparse-checkout, keeps only alpha.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Subscribed"), "stdout: {}", stdout);
    assert!(stdout.contains("alpha"));

    // Config records the subscription.
    let config_file = home_dir.path().join(".kb").join("config.toml");
    let config = fs::read_to_string(&config_file).unwrap();
    assert!(config.contains("alpha"), "config: {}", config);
    assert!(
        !config.contains("beta"),
        "config must not list beta: {}",
        config
    );

    // Sparse cone contains alpha + always-on top-level dir; top-level files
    // stay; unsubscribed project memory is gone from the working tree.
    let list = git(&["sparse-checkout", "list"], &kb);
    let cone = String::from_utf8_lossy(&list.stdout);
    assert!(cone.contains("projects/alpha"), "cone: {}", cone);
    assert!(cone.contains("agent-rules"), "cone: {}", cone);
    assert!(kb.join("projects/alpha/HANDOFF.md").exists());
    assert!(kb.join("INDEX.md").exists(), "top-level file must stay");
    assert!(
        !kb.join("projects/beta").exists(),
        "unsubscribed memory must not be checked out"
    );

    // Subscribe to beta: it gets materialized too.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "beta"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    assert!(kb.join("projects/beta/HANDOFF.md").exists());
    assert!(kb.join("projects/alpha/HANDOFF.md").exists());
    let list = git(&["sparse-checkout", "list"], &kb);
    let cone = String::from_utf8_lossy(&list.stdout);
    assert!(cone.contains("projects/beta"), "cone: {}", cone);

    // Unsubscribe alpha: memory leaves the device, config updated.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["unsubscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Unsubscribed"), "stdout: {}", stdout);

    let config = fs::read_to_string(&config_file).unwrap();
    assert!(!config.contains("alpha"), "config: {}", config);
    assert!(!kb.join("projects/alpha").exists());
    assert!(kb.join("projects/beta/HANDOFF.md").exists());
}

#[test]
fn test_unsubscribe_last_subscription_restores_full_checkout() {
    let dir = TempDir::new().unwrap();
    // KB whose only top-level directory is `projects/`: a bare `kb init`
    // layout. Unsubscribing the last project leaves an *empty* cone there
    // (no always-on dirs), which used to hard-fail in sparse::set_cone.
    let kb = dir.path().join("knowledge-base");
    fs::create_dir_all(&kb).unwrap();
    fs::write(kb.join("AGENTS.md"), "# Agent Rules\n").unwrap();
    fs::write(kb.join("INDEX.md"), "# Index\n").unwrap();
    fs::create_dir_all(kb.join("projects/alpha")).unwrap();
    fs::write(kb.join("projects/alpha/HANDOFF.md"), "# alpha\n").unwrap();
    fs::create_dir_all(kb.join("projects/beta")).unwrap();
    fs::write(kb.join("projects/beta/HANDOFF.md"), "# beta\n").unwrap();
    assert!(git(&["init", "-b", "main"], &kb).status.success());
    assert!(git(&["config", "user.name", "Test"], &kb).status.success());
    assert!(
        git(&["config", "user.email", "test@example.com"], &kb)
            .status
            .success()
    );
    git_commit_all(&kb, "init");

    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    // beta's memory is out of the cone now.
    assert!(!kb.join("projects/beta/HANDOFF.md").exists());

    // Unsubscribing the last subscription must revert to a full checkout
    // (all memory materialized again) rather than erroring out.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["unsubscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "exit: {:?} stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(kb.join("projects/alpha/HANDOFF.md").exists());
    assert!(
        kb.join("projects/beta/HANDOFF.md").exists(),
        "full checkout must materialize all memory again"
    );

    let config_file = home_dir.path().join(".kb").join("config.toml");
    let config = fs::read_to_string(&config_file).unwrap();
    assert!(
        !config.contains("alpha") && !config.contains("beta"),
        "no subscriptions should remain: {}",
        config
    );
}

#[test]
fn test_subscribe_idempotent() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Subscribed"));
}

#[test]
fn test_subscribe_unknown_project_errors() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "nope"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("no project memory"));
}

#[test]
fn test_subscribe_not_git_repo_errors() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a git repository"));
}

#[test]
fn test_subscribe_dry_run_reports_plan_without_changes() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha", "--dry-run", "--json"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"dry_run\": true"), "stdout: {}", stdout);
    assert!(
        stdout.contains("\"sparse_enabled\": false"),
        "stdout: {}",
        stdout
    );

    // Nothing changed: no config written, sparse-checkout untouched.
    let config_file = home_dir.path().join(".kb").join("config.toml");
    assert!(!config_file.exists(), "dry run must not write config");
    let raw = git(&["sparse-checkout", "list"], &kb);
    assert!(String::from_utf8_lossy(&raw.stdout).is_empty());
    assert!(
        kb.join("projects/beta/HANDOFF.md").exists(),
        "dry run must leave the working tree untouched"
    );
}

#[test]
fn test_unsubscribe_unknown_project_is_a_noop() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["unsubscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Unsubscribed"), "stdout: {}", stdout);

    // A full checkout is never converted just by unsubscribing someone who was
    // never subscribed, and no config is written.
    let config_file = home_dir.path().join(".kb").join("config.toml");
    assert!(
        !config_file.exists(),
        "noop must not write config: {:?}",
        config_file
    );
    assert!(
        kb.join("projects/alpha/HANDOFF.md").exists(),
        "full checkout must be left alone"
    );
    let raw = git(&["sparse-checkout", "list"], &kb);
    assert!(String::from_utf8_lossy(&raw.stdout).is_empty());
}

#[test]
fn test_subscribe_to_uncommitted_memory_succeeds() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    // Simulate `kb link kb-cli` on an un-synced device: memory exists on disk
    // but is not committed yet.
    let fresh = kb.join("projects").join("fresh");
    fs::create_dir_all(&fresh).unwrap();
    fs::write(fresh.join("HANDOFF.md"), "# fresh\n").unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "fresh"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    let config_file = home_dir.path().join(".kb").join("config.toml");
    let config = fs::read_to_string(&config_file).unwrap();
    assert!(config.contains("fresh"), "config: {}", config);
    assert!(
        fresh.join("HANDOFF.md").exists(),
        "just-linked memory must not be dropped"
    );
}

#[test]
fn test_doctor_reports_sparse_drift() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    // Healthy: doctor passes the sparse check.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["doctor", "--json"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("sparse-checkout matches subscriptions"),
        "{}",
        stdout
    );

    // Simulate external drift: drop alpha from the cone behind kb's back.
    assert!(
        git(&["sparse-checkout", "set", "agent-rules"], &kb)
            .status
            .success()
    );
    assert!(!kb.join("projects/alpha").exists());

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["doctor", "--json"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"name\": \"sparse\""), "{}", stdout);
    assert!(
        stdout.contains("subscribed but not checked out"),
        "{}",
        stdout
    );
    assert!(stdout.contains("kb subscribe alpha"), "{}", stdout);

    // The suggested fix reconciles the device.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    assert!(kb.join("projects/alpha/HANDOFF.md").exists());
}

/// Helper: write a tiny editor shim that records its directory argument to
/// `log_file`, and return the `$EDITOR` value that runs it.
fn editor_shim(dir: &Path, log_file: &Path) -> String {
    #[cfg(windows)]
    {
        let hook = dir.join("open-hook.ps1");
        fs::write(
            &hook,
            format!(
                "\"$args[0]\" | Out-File -FilePath '{}' -Encoding ascii\n",
                log_file.to_string_lossy()
            ),
        )
        .unwrap();
        format!("powershell -NoProfile -File {}", hook.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        let hook = dir.join("open-hook.sh");
        fs::write(
            &hook,
            format!(
                "#!/bin/sh\necho \"$1\" > \"{}\"\n",
                log_file.to_string_lossy()
            ),
        )
        .unwrap();
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
        hook.to_string_lossy().into_owned()
    }
}

#[test]
fn test_open_project_with_editor() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    let log_file = dir.path().join("open-log.txt");
    let editor = editor_shim(dir.path(), &log_file);

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["open", "alpha"])
        .env("EDITOR", &editor)
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Opened"));

    let logged = fs::read_to_string(&log_file).unwrap();
    assert!(
        logged.contains("alpha"),
        "editor must receive the project dir: {}",
        logged
    );
}

#[test]
fn test_open_kb_root_without_project() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    let log_file = dir.path().join("open-log.txt");
    let editor = editor_shim(dir.path(), &log_file);

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .arg("open")
        .env("EDITOR", &editor)
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Opened"));

    let logged = fs::read_to_string(&log_file).unwrap();
    assert!(
        logged.contains("knowledge-base"),
        "editor must receive the KB root: {}",
        logged
    );
}

#[test]
fn test_open_json_reports_path() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    let log_file = dir.path().join("open-log.txt");
    let editor = editor_shim(dir.path(), &log_file);

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["open", "alpha", "--json"])
        .env("EDITOR", &editor)
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status.code());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"ok\": true"), "{}", stdout);
    assert!(stdout.contains("alpha"), "{}", stdout);
}

#[test]
fn test_open_unknown_project_errors() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["open", "nope"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("no project memory"));
}

#[test]
fn test_open_unsubscribed_project_hints_subscribe() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    // Subscribe to alpha only: beta exists in the repo but is not checked out.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    assert!(!kb.join("projects/beta").exists());

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["open", "beta"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not checked out"));
}

#[test]
fn test_done_commits_and_pushes_freshly_linked_memory() {
    let dir = TempDir::new().unwrap();
    let (kb, bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();

    // NOTE: no upstream is pushed beforehand — `kb done` must push its own
    // branch (`-u origin HEAD`) even when origin/main has no tracking branch.

    // Work on a project placed in the sandboxed home, on the real homepage
    // layout (`~/Projects/<name>`) so `kb work` finds it by name.
    let repo = home_dir.path().join("Projects").join("myapp");
    fs::create_dir_all(&repo).unwrap();
    fs::write(repo.join("src.ts"), "// project code\n").unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["work", "myapp"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    // Freshly-linked memory is entirely untracked: regression for the
    // `?? projects/` collapse that made `kb done` report "No changes".
    fs::write(
        kb.join("projects/myapp/HANDOFF.md"),
        "# myapp\n\nNewly linked memory.\n",
    )
    .unwrap();
    fs::write(kb.join("INDEX.md"), "# Index updated\n").unwrap();

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["done", "-m", "sandbox: add myapp memory"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "exit: {:?} stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Done!"), "{}", stdout);
    assert!(stdout.contains("myapp"), "{}", stdout);
    assert!(stdout.contains("sandbox: add myapp memory"), "{}", stdout);

    // The commit landed on the KB and was pushed to origin.
    let head = git(&["log", "-1", "HEAD"], &kb);
    assert!(
        String::from_utf8_lossy(&head.stdout).contains("add myapp memory"),
        "{}",
        String::from_utf8_lossy(&head.stdout)
    );
    let origin = git(&["log", "-1", "HEAD"], &bare);
    assert!(
        String::from_utf8_lossy(&origin.stdout).contains("add myapp memory"),
        "{}",
        String::from_utf8_lossy(&origin.stdout)
    );
}

#[test]
fn test_export_import_round_trip_with_rename() {
    let dir = TempDir::new().unwrap();
    let (kb, _) = setup_kb_git_repo(dir.path());

    // Build a project with real memory on disk.
    fs::create_dir_all(kb.join("projects/irosh/spec")).unwrap();
    fs::write(kb.join("projects/irosh/HANDOFF.md"), "# irosh\n\nNotes.\n").unwrap();
    fs::write(kb.join("projects/irosh/spec/01.md"), "spec\n").unwrap();

    let tarball = dir.path().join("packed").join("irosh.tar.gz");
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "export", "irosh"])
        .args(["--output", tarball.to_str().unwrap()])
        .assert()
        .success();

    // Importing under a new name must place contents in `projects/reorg`.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "import"])
        .arg(tarball.to_str().unwrap())
        .args(["--name", "reorg"])
        .assert()
        .success();
    assert!(kb.join("projects/reorg/HANDOFF.md").exists());
    assert!(kb.join("projects/reorg/spec/01.md").exists());
    assert_eq!(
        fs::read_to_string(kb.join("projects/reorg/spec/01.md")).unwrap(),
        "spec\n"
    );

    // The import name must not silently resolve back to the original.
    assert!(!kb.join("projects/reorg/projects").exists());
}

#[test]
fn test_archive_roundtrip_unlinks_and_restores() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "myapp");
    let home_dir = TempDir::new().unwrap();

    // Link so the project is wired (symlinks, active list, memory).
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    fs::write(kb.join("projects/myapp/HANDOFF.md"), "# myapp\n\nnotes\n").unwrap();
    let config_file = home_dir.path().join(".kb").join("config.toml");
    assert!(fs::read_to_string(&config_file).unwrap().contains("myapp"));

    // Archive by name: memory leaves projects/, wiring is removed, active list drops.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "archive", "myapp"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("Archived"));

    assert!(
        !kb.join("projects/myapp").exists(),
        "memory must leave projects/"
    );
    assert!(kb.join("archive/myapp/HANDOFF.md").exists());
    assert!(
        !repo.join("scratch").symlink_metadata().is_ok(),
        "scratch symlink must be removed"
    );
    assert!(!repo.join("kb-rules.md").exists());
    let config = fs::read_to_string(&config_file).unwrap();
    assert!(
        !config.contains("\"myapp\""),
        "config must drop the archived project from active_projects: {}",
        config
    );

    // Archived project vanishes from `kb status`.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "status"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("myapp"),
        "archived project must not appear in status"
    );

    // Archiving it again while archived is refused with a restore hint.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "archive", "myapp"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("already archived"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Restore: memory comes back, active list is restored.
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["archive", "--restore", "myapp"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    assert!(kb.join("projects/myapp/HANDOFF.md").exists());
    assert!(!kb.join("archive/myapp").exists());
    let config = fs::read_to_string(&config_file).unwrap();
    assert!(
        config.contains("\"myapp\""),
        "restore must re-add the project to active_projects: {}",
        config
    );

    // Restored (formerly archived) memory is exportable again.
    let tarball = dir.path().join("myapp.tar.gz");
    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "export", "myapp"])
        .args(["--output", tarball.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    assert!(tarball.exists());
}

#[test]
fn test_archive_missing_memory_errors() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let home_dir = TempDir::new().unwrap();

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "archive", "ghost"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no project memory for 'ghost'"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_doctor_fix_relinks_broken_symlinks() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let repo = fake_project_repo(dir.path(), "myapp");
    let home_dir = TempDir::new().unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["link", repo.to_str().unwrap()])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();
    assert!(
        fs::symlink_metadata(repo.join("scratch"))
            .unwrap()
            .file_type()
            .is_symlink()
    );

    // Simulate drifted wiring: both symlinks vanish.
    remove_path(&repo.join("scratch"));
    remove_path(&repo.join(".agent-rules"));
    assert!(repo.join("scratch").symlink_metadata().is_err());

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "doctor", "--fix"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("re-linked myapp"));

    assert!(
        fs::symlink_metadata(repo.join("scratch"))
            .unwrap()
            .file_type()
            .is_symlink(),
        "scratch symlink must be re-created"
    );
    assert!(
        fs::symlink_metadata(repo.join(".agent-rules"))
            .unwrap()
            .file_type()
            .is_symlink(),
        ".agent-rules symlink must be re-created"
    );

    // Health check is clean again.
    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "doctor"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("broken symlink"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn test_doctor_fix_creates_missing_config() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let home_dir = TempDir::new().unwrap();

    assert!(!home_dir.path().join(".kb").join("config.toml").exists());

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "doctor", "--fix"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    let config_file = home_dir.path().join(".kb").join("config.toml");
    assert!(config_file.exists(), "doctor --fix must create the config");
    let config = fs::read_to_string(&config_file).unwrap();
    assert!(
        config.contains("knowledge-base"),
        "config must reference the kb_root: {}",
        config
    );
}

#[test]
fn test_doctor_fix_registers_orphaned_memory() {
    let dir = TempDir::new().unwrap();
    let kb = fake_kb_root(dir.path());
    let home_dir = TempDir::new().unwrap();

    // Memory on disk with no config entry (e.g. a manual import).
    fs::create_dir_all(kb.join("projects/extra")).unwrap();
    fs::write(kb.join("projects/extra/HANDOFF.md"), "# extra\n").unwrap();

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "doctor", "--fix"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("added to active_projects: extra"));

    let config_file = home_dir.path().join(".kb").join("config.toml");
    let config = fs::read_to_string(&config_file).unwrap();
    assert!(
        config.contains("\"extra\""),
        "orphaned memory must be registered: {}",
        config
    );
}

#[test]
fn test_doctor_fix_rebuilds_sparse_cone() {
    let dir = TempDir::new().unwrap();
    let (kb, _bare) = setup_kb_git_repo(dir.path());
    let home_dir = TempDir::new().unwrap();
    seed_memory(&kb);

    kb_bin()
        .args(["--kb-root", kb.to_str().unwrap()])
        .args(["subscribe", "alpha"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .assert()
        .success();

    // External drift: beta materialized even though nothing subscribes to it.
    assert!(
        git(
            &[
                "sparse-checkout",
                "set",
                "agent-rules",
                "projects/alpha",
                "projects/beta"
            ],
            &kb,
        )
        .status
        .success()
    );
    assert!(kb.join("projects/beta").exists());

    let out = kb_bin()
        .args(["--kb-root", kb.to_str().unwrap(), "doctor", "--fix"])
        .env("HOME", home_dir.path())
        .env("USERPROFILE", home_dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("rebuilt cone"), "{}", stdout);

    // Cone re-converged to the subscription: beta is pruned, alpha kept.
    assert!(
        !kb.join("projects/beta").exists(),
        "unsubscribed memory must be removed from the cone"
    );
    assert!(kb.join("projects/alpha").exists());
}
