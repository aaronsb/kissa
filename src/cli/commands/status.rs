use std::path::Path;

use crate::cli::OutputFormat;
use kissa::config;
use kissa::core::index::Index;

#[derive(clap::Args)]
pub struct StatusArgs {
    /// Repo name or path
    pub repo: String,
}

pub fn run(args: StatusArgs, format: OutputFormat) -> anyhow::Result<()> {
    let index = Index::open(&config::index_path())?;

    let repo = if Path::new(&args.repo).is_absolute() {
        index.get_repo_by_path(Path::new(&args.repo))?
    } else {
        index.get_repo_by_name(&args.repo)?
    };

    let Some(repo) = repo else {
        anyhow::bail!("repo not found: {}", args.repo);
    };

    match format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(std::io::stdout(), &repo)?;
            println!();
        }
        OutputFormat::Paths => {
            println!("{}", repo.path.display());
        }
        OutputFormat::PathsNull => {
            print!("{}\0", repo.path.display());
        }
        OutputFormat::Human => {
            println!("{}", crate::cli::display::render_status(&repo));
        }
    }

    Ok(())
}
