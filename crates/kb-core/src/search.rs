use anyhow::Result;
use globset::{Glob, GlobMatcher};
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};

/// A single search hit.
#[derive(Debug)]
pub struct SearchMatch {
    /// Project name (from `projects/<name>/`), if the file belongs to one.
    pub project: Option<String>,
    /// Path relative to the KB root, forward slashes.
    pub path: String,
    /// 1-based line number.
    pub line: usize,
    /// The matching line's text (without the trailing newline).
    pub text: String,
}

/// Options controlling a KB-wide search.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Only search inside `projects/<name>/` for these projects.
    pub projects: Vec<String>,
    /// Print each matching file once, without line details.
    pub files_only: bool,
    /// Case-sensitive matching (default: case-insensitive).
    pub case_sensitive: bool,
    /// Optional glob to filter files (matched against path and basename).
    pub glob: Option<String>,
    /// Skip files larger than this many bytes.
    pub max_file_size: u64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            files_only: false,
            case_sensitive: false,
            glob: None,
            max_file_size: 5 * 1024 * 1024,
        }
    }
}

/// Search all files under the KB root for `query`.
///
/// The walk is `.gitignore`-aware (like ripgrep), skips hidden entries and
/// the `.git` directory, and never follows symlinks. Binary files are skipped
/// by extension, and files above `max_file_size` are skipped.
pub fn search(kb_root: &Path, query: &str, opts: &SearchOptions) -> Result<Vec<SearchMatch>> {
    let query_lower = if opts.case_sensitive {
        String::new()
    } else {
        query.to_lowercase()
    };

    let glob = opts
        .glob
        .as_ref()
        .map(|g| Glob::new(g))
        .transpose()
        .map(|g| g.map(|g| g.compile_matcher()))?;

    let mut builder = WalkBuilder::new(kb_root);
    builder
        .hidden(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false);

    let mut results: Vec<SearchMatch> = Vec::new();

    for entry in builder.build().filter_map(|e| e.ok()) {
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }

        let path: PathBuf = entry.into_path();

        // Glob filter (against full path and basename).
        if let Some(glob) = &glob
            && !glob_matches(glob, &path)
        {
            continue;
        }

        // Project filter.
        if !opts.projects.is_empty() {
            let matches = project_name_for(kb_root, &path)
                .map(|name| opts.projects.iter().any(|p| p == &name))
                .unwrap_or(false);
            if !matches {
                continue;
            }
        }

        // Binary / size cap.
        if has_binary_extension(&path) || is_symlink(&path) {
            continue;
        }
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > opts.max_file_size {
            continue;
        }

        let content = match fs::read(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if byte_is_binary(&content) {
            continue;
        }

        let text = String::from_utf8_lossy(&content);
        let rel_path = crate::paths::normalize_display(&rel_to(kb_root, &path));
        let project = project_name_for(kb_root, &path);
        let mut file_recorded = false;

        for (idx, line) in text.split('\n').enumerate() {
            let matched = if opts.case_sensitive {
                line.contains(query)
            } else {
                line.to_lowercase().contains(query_lower.as_str())
            };
            if !matched {
                continue;
            }

            if opts.files_only {
                if !file_recorded {
                    results.push(SearchMatch {
                        project: project.clone(),
                        path: rel_path.clone(),
                        line: 0,
                        text: String::new(),
                    });
                    file_recorded = true;
                }
            } else {
                results.push(SearchMatch {
                    project: project.clone(),
                    path: rel_path.clone(),
                    line: idx + 1,
                    text: line.trim_end_matches('\r').to_string(),
                });
            }
        }
    }

    Ok(results)
}

/// Check if a glob matches the path's full relative string or its basename.
fn glob_matches(glob: &GlobMatcher, path: &Path) -> bool {
    let full = crate::paths::normalize_display(path);
    if glob.is_match(&full) {
        return true;
    }
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| glob.is_match(n))
        .unwrap_or(false)
}

/// Compute a path relative to the KB root (lossy but safe for display).
fn rel_to(kb_root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(kb_root).unwrap_or(path).to_path_buf()
}

/// Derive the project name from `projects/<name>/...`, if any.
fn project_name_for(kb_root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(kb_root).ok()?;
    let mut parts = rel.components();
    let first = parts.next()?.as_os_str().to_str()?;
    if first != "projects" {
        return None;
    }
    let name = parts.next()?.as_os_str().to_str()?.to_string();
    // A project is a directory: `projects/README.md` is a file, not a project.
    if !kb_root.join("projects").join(&name).is_dir() {
        return None;
    }
    Some(name)
}

/// Heuristic binary-content detection: significant NUL bytes.
fn byte_is_binary(content: &[u8]) -> bool {
    content.iter().take(2048).filter(|b| **b == 0u8).count() > 0
}

/// Skip symlinks entirely; they could point outside the KB and double-match.
fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Extensions treated as binary (never searched).
fn has_binary_extension(path: &Path) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_ascii_lowercase(),
        None => return false,
    };

    matches!(
        ext.as_str(),
        "exe"
            | "dll"
            | "so"
            | "dylib"
            | "lib"
            | "obj"
            | "o"
            | "a"
            | "class"
            | "jar"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "bmp"
            | "webp"
            | "ico"
            | "tif"
            | "tiff"
            | "pdf"
            | "zip"
            | "gz"
            | "tgz"
            | "xz"
            | "tar"
            | "bz2"
            | "7z"
            | "rar"
            | "ttf"
            | "otf"
            | "woff"
            | "woff2"
            | "mp3"
            | "mp4"
            | "m4a"
            | "wav"
            | "avi"
            | "mov"
            | "mkv"
            | "webm"
            | "mpg"
            | "mpeg"
            | "wasm"
            | "pdb"
            | "dmp"
            | "db"
            | "sqlite"
            | "sqlite3"
            | "pyc"
            | "pyo"
            | "pyd"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn kb_structure(dir: &std::path::Path) {
        fs::create_dir_all(dir.join("projects").join("alpha").join("spec")).unwrap();
        fs::create_dir_all(dir.join("projects").join("beta")).unwrap();
        fs::create_dir_all(dir.join("templates")).unwrap();
        fs::write(dir.join("AGENTS.md"), "# rules\n").unwrap();
        fs::write(dir.join("INDEX.md"), "# index\n").unwrap();
        fs::write(
            dir.join("projects").join("alpha").join("HANDOFF.md"),
            "alpha handoff\nblocker noted\n",
        )
        .unwrap();
        fs::write(
            dir.join("projects")
                .join("alpha")
                .join("spec")
                .join("01.md"),
            "alpha spec with Blocker words\n",
        )
        .unwrap();
        fs::write(
            dir.join("projects").join("beta").join("HANDOFF.md"),
            "beta handoff\n",
        )
        .unwrap();
        fs::write(dir.join("templates").join("kb-rules.md"), "tpl\n").unwrap();
        fs::write(dir.join("ignored.md"), "secret token here\n").unwrap();
        fs::write(dir.join(".gitignore"), "ignored.md\n").unwrap();
        fs::write(dir.join("secret.bin"), "token\x00\x00\x00binary").unwrap();
    }

    #[test]
    fn search_finds_matches_case_insensitive() {
        let dir = TempDir::new().unwrap();
        kb_structure(dir.path());

        let hits = search(dir.path(), "BLOCKER", &SearchOptions::default()).unwrap();
        assert!(
            hits.iter()
                .any(|h| h.path == "projects/alpha/HANDOFF.md" && h.line == 2)
        );
        assert!(
            hits.iter()
                .any(|h| h.path == "projects/alpha/spec/01.md" && h.line == 1)
        );
        // Both HANDOFF hits carry the project name.
        assert!(hits.iter().all(|h| h.project.is_some()));
        // Gitignored + binary files must not match.
        assert!(!hits.iter().any(|h| h.path == "ignored.md"));
    }

    #[test]
    fn search_respects_project_filter() {
        let dir = TempDir::new().unwrap();
        kb_structure(dir.path());

        let opts = SearchOptions {
            projects: vec!["beta".to_string()],
            ..Default::default()
        };
        let hits = search(dir.path(), "handoff", &opts).unwrap();
        assert!(hits.len() == 1);
        assert_eq!(hits[0].project.as_deref(), Some("beta"));
        assert_eq!(hits[0].path, "projects/beta/HANDOFF.md");
    }

    #[test]
    fn search_files_only_deduplicates_per_file() {
        let dir = TempDir::new().unwrap();
        kb_structure(dir.path());

        let opts = SearchOptions {
            files_only: true,
            ..Default::default()
        };
        let hits = search(dir.path(), "alpha", &opts).unwrap();
        let paths: Vec<_> = hits.iter().map(|h| h.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.ends_with("alpha/HANDOFF.md")));
        assert!(paths.windows(2).all(|w| w[0] != w[1]), "no duplicate files");
    }

    #[test]
    fn search_glob_filters_files() {
        let dir = TempDir::new().unwrap();
        kb_structure(dir.path());

        let opts = SearchOptions {
            glob: Some("**/*.md".to_string()),
            ..Default::default()
        };
        let hits = search(dir.path(), "handoff", &opts).unwrap();
        assert!(!hits.is_empty());
    }

    #[test]
    fn search_case_sensitive_distinguishes() {
        let dir = TempDir::new().unwrap();
        kb_structure(dir.path());
        fs::write(
            dir.path().join("projects").join("alpha").join("notes.md"),
            "HANDOFF keyword appears uppercase\n",
        )
        .unwrap();

        let opts = SearchOptions {
            case_sensitive: true,
            ..Default::default()
        };
        let hits = search(dir.path(), "HANDOFF", &opts).unwrap();
        assert!(!hits.is_empty());
        // Case-sensitive must NOT match lowercase "handoff" content lines.
        assert!(hits.iter().all(|h| h.text.contains("HANDOFF")));
        assert!(!hits.iter().any(|h| !h.text.contains("HANDOFF")));

        // The same query is case-insensitive by default and matches more.
        let insensitive = search(dir.path(), "HANDOFF", &SearchOptions::default()).unwrap();
        assert!(
            insensitive
                .iter()
                .any(|h| h.path == "projects/alpha/HANDOFF.md")
        );
        assert!(insensitive.len() >= hits.len());
    }

    #[test]
    fn search_no_matches_returns_empty() {
        let dir = TempDir::new().unwrap();
        kb_structure(dir.path());

        let hits = search(dir.path(), "zzz-not-present", &SearchOptions::default()).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn search_top_level_projects_file_is_not_a_project() {
        let dir = TempDir::new().unwrap();
        kb_structure(dir.path());
        fs::write(
            dir.path().join("projects").join("README.md"),
            "needle listed here\n",
        )
        .unwrap();

        let hits = search(dir.path(), "needle", &SearchOptions::default()).unwrap();
        let top_level = hits
            .iter()
            .find(|h| h.path == "projects/README.md")
            .expect("projects/README.md must be searched");
        assert_eq!(
            top_level.project, None,
            "a file in projects/ is not a project"
        );
    }
}
