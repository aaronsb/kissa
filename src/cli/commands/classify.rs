use std::collections::HashMap;
use std::path::{Path, PathBuf};

use owo_colors::OwoColorize;

use crate::cli::OutputFormat;
use kissa::config;
use kissa::core::classify;
use kissa::core::index::Index;

#[derive(clap::Args)]
pub struct ClassifyArgs {
    /// Re-run classification rules on all indexed repos
    #[arg(long)]
    pub reapply: bool,

    /// Analyze index and suggest classification rules
    #[arg(long)]
    pub suggest: bool,
}

pub fn run(args: ClassifyArgs, format: OutputFormat) -> anyhow::Result<()> {
    let cfg = config::load_config()?;
    let index = Index::open(&config::index_path())?;

    if args.suggest {
        return run_suggest(&index, format);
    }

    if args.reapply {
        return run_reapply(&index, &cfg, format);
    }

    // Default: show classification summary
    run_summary(&index, format)
}

fn run_summary(index: &Index, format: OutputFormat) -> anyhow::Result<()> {
    let repos = index.all_repos()?;

    let mut managed_counts: HashMap<String, usize> = HashMap::new();
    let mut unclassified = 0;

    for repo in &repos {
        if let Some(ref mb) = repo.managed_by {
            *managed_counts.entry(mb.clone()).or_default() += 1;
        } else if repo.ownership.is_none() && repo.intention.is_none() {
            unclassified += 1;
        }
    }

    match format {
        OutputFormat::Json => {
            let summary = serde_json::json!({
                "total": repos.len(),
                "managed": managed_counts,
                "unclassified": unclassified,
            });
            serde_json::to_writer_pretty(std::io::stdout(), &summary)?;
            println!();
        }
        _ => {
            println!(
                "  {} {} repos total",
                "classify:".green().bold(),
                repos.len(),
            );

            if !managed_counts.is_empty() {
                println!("  {}", "managed repos:".bold());
                let mut sorted: Vec<_> = managed_counts.iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(a.1));
                for (tool, count) in sorted {
                    println!("    {:>4} {}", count, tool.dimmed());
                }
            }

            if unclassified > 0 {
                println!(
                    "  {} {} repos unclassified",
                    "note:".yellow(),
                    unclassified,
                );
                println!(
                    "  {} run {} to see suggested rules",
                    "hint:".dimmed(),
                    "kissa classify --suggest".bold(),
                );
            }
        }
    }

    Ok(())
}

fn run_reapply(
    index: &Index,
    cfg: &config::types::KissaConfig,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let repos = index.all_repos()?;
    let mut changed = 0;

    for mut repo in repos {
        let old_managed = repo.managed_by.clone();
        let old_ownership = repo.ownership.clone();
        let old_intention = repo.intention.clone();
        let old_category = repo.category;
        let old_tags_len = repo.tags.len();

        // Reset classification fields before re-applying
        repo.managed_by = None;
        repo.ownership = None;
        repo.intention = None;
        repo.category = None;
        // Keep user tags but allow rule tags to be re-added
        classify::classify_repo(&mut repo, cfg);

        if repo.managed_by != old_managed
            || repo.ownership != old_ownership
            || repo.intention != old_intention
            || repo.category != old_category
            || repo.tags.len() != old_tags_len
        {
            index.upsert_repo(&repo)?;
            changed += 1;
        }
    }

    match format {
        OutputFormat::Json => {
            let result = serde_json::json!({ "updated": changed });
            serde_json::to_writer_pretty(std::io::stdout(), &result)?;
            println!();
        }
        _ => {
            println!(
                "  {} re-classified all repos, {} updated",
                "classify:".green().bold(),
                changed,
            );
        }
    }

    Ok(())
}

fn run_suggest(index: &Index, format: OutputFormat) -> anyhow::Result<()> {
    let repos = index.all_repos()?;

    // Group repos by parent directory
    let mut parent_groups: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for repo in &repos {
        if repo.managed_by.is_some() {
            continue; // Already classified
        }
        if let Some(parent) = repo.path.parent() {
            parent_groups
                .entry(parent.to_path_buf())
                .or_default()
                .push(repo.name.clone());
        }
    }

    // Suggest rules for clusters of 3+ unclassified repos sharing a parent
    let mut suggestions: Vec<(PathBuf, usize)> = parent_groups
        .into_iter()
        .filter(|(_, names)| names.len() >= 3)
        .map(|(path, names)| (path, names.len()))
        .collect();
    suggestions.sort_by(|a, b| b.1.cmp(&a.1));

    match format {
        OutputFormat::Json => {
            let rules: Vec<_> = suggestions
                .iter()
                .map(|(path, count)| {
                    serde_json::json!({
                        "path_pattern": format!("{}/*", path.display()),
                        "repo_count": count,
                    })
                })
                .collect();
            serde_json::to_writer_pretty(std::io::stdout(), &rules)?;
            println!();
        }
        _ => {
            if suggestions.is_empty() {
                println!(
                    "  {} no clusters found to suggest rules for",
                    "suggest:".dimmed(),
                );
            } else {
                println!(
                    "  {} found {} potential classification rules:\n",
                    "suggest:".green().bold(),
                    suggestions.len(),
                );
                for (path, count) in &suggestions {
                    let tilde_path = tilde_path(path);
                    println!("# {} repos under {}", count, tilde_path);
                    println!("[[classify]]");
                    println!(
                        "match = {{ path = \"{}/*\" }}",
                        tilde_path,
                    );
                    println!(
                        "set = {{ intention = \"dependency\", ownership = \"third-party\" }}"
                    );
                    println!("managed_by = \"TODO\"");
                    println!();
                }
                println!(
                    "  {} copy the rules above into your config.toml",
                    "hint:".dimmed(),
                );
            }
        }
    }

    Ok(())
}

/// Replace home dir prefix with ~ for display.
fn tilde_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = path.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}
