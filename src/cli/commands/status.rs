use crate::cli::OutputFormat;

#[derive(clap::Args)]
pub struct StatusArgs {
    /// Repo name or path
    pub repo: String,
}

pub fn run(_args: StatusArgs, _format: OutputFormat) -> anyhow::Result<()> {
    todo!("Phase 4b: implement status command")
}
