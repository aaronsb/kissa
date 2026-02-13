use crate::cli::OutputFormat;

#[derive(clap::Args)]
pub struct InfoArgs {
    /// Repo name or path
    pub repo: String,
}

pub fn run(_args: InfoArgs, _format: OutputFormat) -> anyhow::Result<()> {
    todo!("Phase 4b: implement info command")
}
