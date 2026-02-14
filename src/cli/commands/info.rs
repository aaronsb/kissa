use std::path::Path;

use crate::cli::OutputFormat;
use kissa::config;
use kissa::core::git_ops;
use kissa::core::index::Index;
use kissa::core::repo::Freshness;

#[derive(clap::Args)]
pub struct InfoArgs {
    /// Repo name or path
    pub repo: String,

    /// Refresh git vitals before displaying
    #[arg(long)]
    pub refresh: bool,
}

pub fn run(args: InfoArgs, format: OutputFormat) -> anyhow::Result<()> {
    let index = Index::open(&config::index_path())?;

    let repo = if Path::new(&args.repo).is_absolute() {
        index.get_repo_by_path(Path::new(&args.repo))?
    } else {
        index.get_repo_by_name(&args.repo)?
    };

    let Some(mut repo) = repo else {
        anyhow::bail!("repo not found: {}", args.repo);
    };

    // Optionally refresh vitals from disk
    if args.refresh {
        if let Ok(vitals) = git_ops::extract_vitals(&repo.path) {
            repo.dirty = vitals.dirty;
            repo.staged = vitals.staged;
            repo.untracked = vitals.untracked;
            repo.ahead = vitals.ahead;
            repo.behind = vitals.behind;
            repo.last_commit = vitals.last_commit;
            repo.current_branch = vitals.current_branch;
            repo.branch_count = vitals.branch_count;
            repo.stale_branch_count = vitals.stale_branch_count;
            repo.freshness = Freshness::from_commit_time(vitals.last_commit);
            repo.last_verified = Some(chrono::Utc::now());
            index.upsert_repo(&repo)?;
        }
    }

    match format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(std::io::stdout(), &repo)?;
            println!();
        }
        _ => {
            println!("{}", crate::cli::display::render_status(&repo));
        }
    }

    Ok(())
}
