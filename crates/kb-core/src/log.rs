use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::{discovery, git, paths};

/// A single commit from the knowledge-base history.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub hash: String,
    pub subject: String,
}

/// Options for [`log`].
#[derive(Debug, Clone)]
pub struct LogOptions {
    /// Maximum number of commits to show.
    pub limit: usize,
    /// Include per-file change statistics.
    pub stat: bool,
    /// Only include commits touching `projects/<name>/` (repeatable).
    pub projects: Vec<String>,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            limit: 15,
            stat: false,
            projects: Vec::new(),
        }
    }
}

/// Result of a `kb log` operation.
#[derive(Debug)]
pub struct LogResult {
    pub root: PathBuf,
    pub entries: Vec<LogEntry>,
    /// Raw `git log` output (used to render `--stat`).
    pub raw: String,
    pub stat: bool,
}

/// Recent knowledge-base history from git.
///
/// Wraps `git log` over the KB root, optionally path-filtered to one or more
/// `projects/<name>/` directories. Returns a clean error when the KB root is
/// not a git repository.
pub fn log(kb_root: Option<&Path>, opts: &LogOptions) -> Result<LogResult> {
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    if !git::is_inside_work_tree(&root) {
        anyhow::bail!(
            "{} is not a git repository.\n\
             Initialize it with `kb init` (or `git init`) before using `kb log`.",
            paths::normalize_display(&root)
        );
    }

    let limit = if opts.limit == 0 { 15 } else { opts.limit };

    let mut args: Vec<String> = vec![
        "log".to_string(),
        format!("-n{limit}"),
        "--pretty=format:%h%x09%s".to_string(),
    ];
    if opts.stat {
        args.push("--stat".to_string());
    }
    if !opts.projects.is_empty() {
        args.push("--".to_string());
        for project in &opts.projects {
            args.push(format!("projects/{}/", project.trim_matches('/')));
        }
    }

    let raw = git::checked(&root, &args, "git log")?;
    let entries = parse_entries(&raw);

    Ok(LogResult {
        root,
        entries,
        raw,
        stat: opts.stat,
    })
}

/// Parse `<hash>\t<subject>` commit lines, ignoring `--stat` file lines.
fn parse_entries(raw: &str) -> Vec<LogEntry> {
    raw.lines()
        .filter_map(|line| {
            let (hash, subject) = line.split_once('\t')?;
            if hash.is_empty() || hash.len() > 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            Some(LogEntry {
                hash: hash.to_string(),
                subject: subject.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_entries_reads_commit_lines() {
        let raw = "abc1234\tfirst commit\ndef5678\tsecond commit\n";
        let entries = parse_entries(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].hash, "abc1234");
        assert_eq!(entries[0].subject, "first commit");
        assert_eq!(entries[1].subject, "second commit");
    }

    #[test]
    fn parse_entries_ignores_stat_lines() {
        let raw = "abc1234\tadd index\n\
                   \x20INDEX.md | 1 +\n\
                   \x201 file changed, 1 insertion(+)\n\
                   def5678\tinit\n";
        let entries = parse_entries(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].hash, "def5678");
    }

    #[test]
    fn parse_entries_empty_input() {
        assert!(parse_entries("").is_empty());
    }
}
