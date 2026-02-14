use crate::cli::OutputFormat;
use kissa::config;
use kissa::core::filter::RepoFilter;
use kissa::core::index::Index;
use kissa::core::repo::{Freshness, RepoState};

#[derive(clap::Args)]
pub struct ListArgs {
    /// Show only dirty repos
    #[arg(long)]
    pub dirty: bool,

    /// Show only stale repos
    #[arg(long)]
    pub stale: bool,

    /// Show only repos with unpushed commits
    #[arg(long)]
    pub unpushed: bool,

    /// Show only orphan repos (no remote)
    #[arg(long)]
    pub orphan: bool,

    /// Show only lost repos (path missing)
    #[arg(long)]
    pub lost: bool,

    /// Filter by remote org/owner
    #[arg(long)]
    pub org: Option<String>,

    /// Filter by freshness tier
    #[arg(long)]
    pub freshness: Option<String>,

    /// Filter by path prefix
    #[arg(long, value_name = "PATH")]
    pub path_prefix: Option<String>,

    /// Filter by ownership (personal, work, work:label, community, third-party, local)
    #[arg(long)]
    pub ownership: Option<String>,

    /// Filter by intention
    #[arg(long)]
    pub intention: Option<String>,

    /// Filter by category (origin, clone, fork, mirror)
    #[arg(long)]
    pub category: Option<String>,

    /// Filter by tags (comma-separated, all must match)
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,

    /// Filter by name (substring match)
    #[arg(long)]
    pub name: Option<String>,
}

pub fn run(args: ListArgs, format: OutputFormat) -> anyhow::Result<()> {
    let index = Index::open(&config::index_path())?;

    let freshness = args.freshness.as_deref().and_then(|s| {
        serde_plain::from_str::<Freshness>(s).ok()
    });

    let state = if args.lost {
        Some(RepoState::Lost)
    } else {
        None
    };

    let filter = RepoFilter {
        dirty: if args.dirty { Some(true) } else { None },
        unpushed: if args.unpushed { Some(true) } else { None },
        orphan: if args.orphan { Some(true) } else { None },
        org: args.org,
        freshness,
        ownership: args.ownership,
        intention: args.intention,
        category: args.category,
        tags: args.tags,
        path_prefix: args.path_prefix,
        has_remote: None,
        name_contains: args.name,
        state,
    };

    let repos = index.list_repos(&filter)?;

    crate::cli::output::output_repos(&repos, format, &mut std::io::stdout())?;

    Ok(())
}
