use anyhow::Result;
use std::path::Path;

use crate::{config, discovery, git, paths, sparse};

/// Result of a `kb subscribe` / `kb unsubscribe` operation.
#[derive(Debug)]
pub struct SubscriptionResult {
    pub project: String,
    /// Full subscription list (config) after the change.
    pub subscribed: Vec<String>,
    /// Whether the working tree uses sparse-checkout after the change.
    pub sparse_enabled: bool,
    /// Project memory currently materialized that this change removes from
    /// the working tree.
    pub dropped: Vec<String>,
    /// Whether this change converts a full checkout to sparse-checkout.
    pub conversion: bool,
    pub dry_run: bool,
}

/// Subscribe this device to a project's memory.
///
/// Ordering guarantee: the sparse-checkout cone is rebuilt *first* and only if
/// git succeeds is config persisted, so a failed cone write can never leave
/// config and disk disagreeing. `--dry-run` performs every check and reports
/// the plan without touching config or the working tree.
pub fn subscribe(kb_root: Option<&Path>, name: &str, dry_run: bool) -> Result<SubscriptionResult> {
    apply(kb_root, name, true, dry_run)
}

/// Unsubscribe this device from a project's memory. A no-op — including on a
/// full checkout — when the project was never subscribed.
pub fn unsubscribe(
    kb_root: Option<&Path>,
    name: &str,
    dry_run: bool,
) -> Result<SubscriptionResult> {
    apply(kb_root, name, false, dry_run)
}

fn apply(
    kb_root: Option<&Path>,
    name: &str,
    add: bool,
    dry_run: bool,
) -> Result<SubscriptionResult> {
    paths::validate_project_name(name)?;
    let (root, _) = discovery::discover_kb_root(kb_root)?;

    if !git::is_inside_work_tree(&root) {
        let what = if add {
            "subscribe to project memory"
        } else {
            "manage subscriptions"
        };
        anyhow::bail!(
            "{} is not a git repository.\n\
             The knowledge-base must be a git repo to {what}.",
            paths::normalize_display(&root)
        );
    }

    let mut config = config::load()?;
    let before = config.active_projects.clone();

    if add
        && !config.active_projects.iter().any(|p| p == name)
        && !sparse::memory_exists(&root, name)?
    {
        anyhow::bail!(
            "no project memory for '{}' in the knowledge-base.\n\
             There is no projects/{}/ in the repo yet.\n\
             Hint: `kb link {}` on another device (or this one) creates it.",
            name,
            name,
            name
        );
    }

    if add {
        config::ensure_active_project(&mut config, name);
    } else {
        config::remove_active_project(&mut config, name);
    }
    let subscribed = config.active_projects.clone();

    let was_sparse = sparse::enabled(&root)?;
    let materialized = sparse::subscribed_projects(&root)?;
    let dropped = dropped_dirs(&materialized, &subscribed);
    let is_noop = !add && before.is_empty();
    let conversion = !was_sparse && !is_noop;

    if !is_noop && !dry_run {
        let cone = sparse::cone(&root, &subscribed)?;
        sparse::set_cone(&root, &cone)?;
        config::save(&config)?;
    }

    let sparse_enabled = if dry_run {
        was_sparse
    } else {
        sparse::enabled(&root)?
    };

    Ok(SubscriptionResult {
        project: name.to_string(),
        subscribed,
        sparse_enabled,
        dropped,
        conversion,
        dry_run,
    })
}

/// Materialized project memory that is leaving the working tree: currently
/// subscribed projects absent from the desired set. Sorted for stable output.
fn dropped_dirs(materialized: &[String], desired: &[String]) -> Vec<String> {
    let mut dropped: Vec<String> = materialized
        .iter()
        .filter(|p| !desired.contains(p))
        .cloned()
        .collect();
    dropped.sort();
    dropped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropped_dirs_lists_projects_leaving_the_worktree() {
        let materialized = vec!["alpha".to_string(), "beta".to_string()];
        assert_eq!(
            dropped_dirs(&materialized, &["alpha".to_string()]),
            vec!["beta".to_string()]
        );
        assert!(dropped_dirs(&materialized, &materialized).is_empty());
        assert_eq!(
            dropped_dirs(&materialized, &[]),
            vec!["alpha".to_string(), "beta".to_string()]
        );
    }
}
