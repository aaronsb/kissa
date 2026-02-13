use kissa::core::repo::{Freshness, Repo};
use kissa::core::index::FreshnessSummary;

/// Render a single repo as a one-line summary for list output.
pub fn render_repo_line(_repo: &Repo) -> String {
    todo!("Phase 4a: implement repo line rendering")
}

/// Render detailed status for a single repo.
pub fn render_status(_repo: &Repo) -> String {
    todo!("Phase 4a: implement status rendering")
}

/// Render the freshness bar chart.
pub fn render_freshness(_summary: &FreshnessSummary, _total: usize) -> String {
    todo!("Phase 4a: implement freshness chart rendering")
}

/// Get the terminal color for a freshness tier.
pub fn freshness_style(_f: Freshness) -> owo_colors::Style {
    todo!("Phase 4a: implement freshness colors")
}
