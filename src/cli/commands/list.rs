use crate::cli::OutputFormat;

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
    #[arg(long)]
    pub path_prefix: Option<String>,

    /// Show duplicate repos
    #[arg(long)]
    pub duplicates: bool,
}

pub fn run(_args: ListArgs, _format: OutputFormat) -> anyhow::Result<()> {
    todo!("Phase 4b: implement list command")
}
