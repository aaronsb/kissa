// Terse text formatter for MCP responses (ADR-300)
//
// State tags: [listing], [status], [scan_complete], [blocked], [error], [batch]
// Next hints: → next: tool1 | tool2
// Elicitation: ? ask user: question

use kissa::core::index::{FreshnessSummary, IndexSummary};
use kissa::core::repo::Repo;

/// Format a repo list for MCP output.
pub fn format_repo_list(repos: &[Repo]) -> String {
    let mut lines = Vec::new();
    lines.push(format!("[listing] {} repos", repos.len()));

    for repo in repos {
        let mut flags = Vec::new();
        if repo.dirty {
            flags.push("dirty");
        }
        if repo.ahead > 0 {
            flags.push("unpushed");
        }
        if repo.remotes.is_empty() {
            flags.push("orphan");
        }
        let flag_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(","))
        };

        lines.push(format!(
            "  {} ({}) {}{}",
            repo.name,
            repo.freshness.label(),
            repo.path.display(),
            flag_str,
        ));
    }

    lines.push("→ next: repo_status <name> | list_repos --dirty".into());
    lines.join("\n")
}

/// Format a single repo status for MCP output.
pub fn format_repo_status(repo: &Repo) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "[status] {} ({})",
        repo.name,
        repo.freshness.label()
    ));
    lines.push(format!("  path: {}", repo.path.display()));

    if let Some(ref branch) = repo.current_branch {
        lines.push(format!(
            "  branch: {} / {}",
            branch,
            repo.default_branch.as_deref().unwrap_or("?")
        ));
    }

    let mut tree = Vec::new();
    if repo.dirty {
        tree.push("dirty");
    }
    if repo.staged {
        tree.push("staged");
    }
    if repo.untracked {
        tree.push("untracked");
    }
    if tree.is_empty() {
        tree.push("clean");
    }
    lines.push(format!("  tree: {}", tree.join(", ")));

    if repo.ahead > 0 || repo.behind > 0 {
        lines.push(format!("  tracking: ↑{} ↓{}", repo.ahead, repo.behind));
    }

    if !repo.remotes.is_empty() {
        for remote in &repo.remotes {
            lines.push(format!("  remote: {} → {}", remote.name, remote.url));
        }
    }

    lines.push("→ next: list_repos | freshness".into());
    lines.join("\n")
}

/// Format the freshness summary for MCP output.
pub fn format_freshness(summary: &FreshnessSummary) -> String {
    let total = summary.active + summary.recent + summary.stale + summary.dormant + summary.ancient;
    let mut lines = Vec::new();
    lines.push(format!("[freshness] {} repos", total));
    lines.push(format!("  active:  {}", summary.active));
    lines.push(format!("  recent:  {}", summary.recent));
    lines.push(format!("  stale:   {}", summary.stale));
    lines.push(format!("  dormant: {}", summary.dormant));
    lines.push(format!("  ancient: {}", summary.ancient));
    lines.push("→ next: list_repos --freshness stale | list_repos --dirty".into());
    lines.join("\n")
}

/// Format a scan result for MCP output.
pub fn format_scan_complete(discovered: usize, indexed: usize, duration_secs: f64) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "[scan_complete] {} discovered, {} indexed in {:.1}s",
        discovered, indexed, duration_secs
    ));
    lines.push("→ next: list_repos | freshness".into());
    lines.join("\n")
}

/// Format an index summary for MCP output.
pub fn format_summary(summary: &IndexSummary) -> String {
    let mut lines = Vec::new();
    lines.push(format!("[summary] {} repos", summary.total_repos));
    lines.push(format!("  dirty: {}", summary.dirty_count));
    lines.push(format!("  unpushed: {}", summary.unpushed_count));
    lines.push(format!("  orphan: {}", summary.orphan_count));
    lines.push(format!("  lost: {}", summary.lost_count));
    if let Some(ref ts) = summary.last_scan {
        lines.push(format!("  last scan: {}", ts.format("%Y-%m-%d %H:%M")));
    }
    lines.push("→ next: list_repos --dirty | freshness | scan".into());
    lines.join("\n")
}

/// Format a permission denied error for MCP output.
pub fn format_blocked(operation: &str, required: &str, current: &str) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "[blocked] {} requires '{}', current is '{}'",
        operation, required, current
    ));
    lines.push("? ask user: increase difficulty level or use per-path override".into());
    lines.join("\n")
}
