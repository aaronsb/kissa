use owo_colors::{OwoColorize, Style};

use kissa::core::index::FreshnessSummary;
use kissa::core::repo::{Freshness, Repo, RepoState};

/// Render a single repo as a one-line summary for list output.
pub fn render_repo_line(repo: &Repo) -> String {
    let style = freshness_style(repo.freshness);
    let name = format!("{}", repo.name.style(style));

    let mut indicators = Vec::new();
    if repo.dirty {
        indicators.push("*".red().to_string());
    }
    if repo.staged {
        indicators.push("+".green().to_string());
    }
    if repo.ahead > 0 {
        indicators.push(format!("{}↑", repo.ahead).yellow().to_string());
    }
    if repo.behind > 0 {
        indicators.push(format!("{}↓", repo.behind).yellow().to_string());
    }
    if repo.state == RepoState::Lost {
        indicators.push("LOST".red().bold().to_string());
    }

    let branch = repo
        .current_branch
        .as_deref()
        .unwrap_or("(detached)");

    let indicator_str = if indicators.is_empty() {
        String::new()
    } else {
        format!(" {}", indicators.join(""))
    };

    format!(
        "  {} {} {}{}",
        name,
        format!("[{}]", branch).dimmed(),
        repo.path.display().to_string().dimmed(),
        indicator_str,
    )
}

/// Render detailed status for a single repo.
pub fn render_status(repo: &Repo) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "{} {}",
        repo.name.bold(),
        format!("({})", repo.freshness.label()).style(freshness_style(repo.freshness)),
    ));
    lines.push(format!(
        "  {} {}",
        "path:".dimmed(),
        repo.path.display()
    ));

    if let Some(ref branch) = repo.current_branch {
        lines.push(format!(
            "  {} {} / {}",
            "branch:".dimmed(),
            branch,
            repo.default_branch.as_deref().unwrap_or("?"),
        ));
    }

    lines.push(format!(
        "  {} total: {}, stale: {}",
        "branches:".dimmed(),
        repo.branch_count,
        repo.stale_branch_count,
    ));

    // Working tree
    let mut wt = Vec::new();
    if repo.dirty {
        wt.push("dirty".red().to_string());
    }
    if repo.staged {
        wt.push("staged".green().to_string());
    }
    if repo.untracked {
        wt.push("untracked".yellow().to_string());
    }
    if wt.is_empty() {
        wt.push("clean".green().to_string());
    }
    lines.push(format!("  {} {}", "tree:".dimmed(), wt.join(", ")));

    // Ahead/behind
    if repo.ahead > 0 || repo.behind > 0 {
        lines.push(format!(
            "  {} ↑{} ↓{}",
            "tracking:".dimmed(),
            repo.ahead,
            repo.behind,
        ));
    }

    // Remotes
    if !repo.remotes.is_empty() {
        lines.push(format!("  {}", "remotes:".dimmed()));
        for remote in &repo.remotes {
            lines.push(format!("    {} → {}", remote.name, remote.url));
        }
    } else {
        lines.push(format!(
            "  {} {}",
            "remotes:".dimmed(),
            "none (orphan)".red(),
        ));
    }

    // Classification
    if let Some(ref cat) = repo.category {
        lines.push(format!(
            "  {} {:?}",
            "category:".dimmed(),
            cat,
        ));
    }
    if let Some(ref own) = repo.ownership {
        lines.push(format!(
            "  {} {:?}",
            "ownership:".dimmed(),
            own,
        ));
    }
    if let Some(ref intent) = repo.intention {
        lines.push(format!(
            "  {} {:?}",
            "intention:".dimmed(),
            intent,
        ));
    }

    // Tags
    if !repo.tags.is_empty() {
        lines.push(format!(
            "  {} {}",
            "tags:".dimmed(),
            repo.tags.join(", "),
        ));
    }

    // Last commit
    if let Some(dt) = repo.last_commit {
        lines.push(format!(
            "  {} {}",
            "last commit:".dimmed(),
            dt.format("%Y-%m-%d %H:%M"),
        ));
    }

    lines.join("\n")
}

/// Render the freshness bar chart.
pub fn render_freshness(summary: &FreshnessSummary, total: usize) -> String {
    if total == 0 {
        return "  No repos in index.".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "  {} repos across 5 freshness tiers:\n",
        total.bold()
    ));

    let tiers = [
        ("active", summary.active, Freshness::Active),
        ("recent", summary.recent, Freshness::Recent),
        ("stale", summary.stale, Freshness::Stale),
        ("dormant", summary.dormant, Freshness::Dormant),
        ("ancient", summary.ancient, Freshness::Ancient),
    ];

    let max_bar = 40;

    for (label, count, freshness) in &tiers {
        let pct = if total > 0 {
            (*count as f64 / total as f64 * 100.0) as usize
        } else {
            0
        };
        let bar_len = if total > 0 {
            (*count as f64 / total as f64 * max_bar as f64) as usize
        } else {
            0
        };
        let bar = "█".repeat(bar_len);
        let style = freshness_style(*freshness);

        lines.push(format!(
            "  {:>8} {:>3} ({:>2}%) {}",
            label,
            count,
            pct,
            bar.style(style),
        ));
    }

    lines.join("\n")
}

/// Get the terminal style for a freshness tier.
pub fn freshness_style(f: Freshness) -> Style {
    match f {
        Freshness::Active => Style::new().green(),
        Freshness::Recent => Style::new().cyan(),
        Freshness::Stale => Style::new().yellow(),
        Freshness::Dormant => Style::new().red(),
        Freshness::Ancient => Style::new().dimmed(),
    }
}
