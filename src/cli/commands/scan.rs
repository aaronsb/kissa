use crate::cli::OutputFormat;

#[derive(clap::Args)]
pub struct ScanArgs {
    /// Full filesystem scan (walks all roots)
    #[arg(long)]
    pub full: bool,

    /// Override scan roots
    #[arg(long)]
    pub roots: Option<Vec<String>>,
}

pub fn run(_args: ScanArgs, _format: OutputFormat) -> anyhow::Result<()> {
    todo!("Phase 4b: implement scan command")
}
