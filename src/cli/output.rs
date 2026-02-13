use std::io::Write;

use kissa::core::repo::Repo;
use crate::cli::OutputFormat;

/// Write repos in the requested output format.
pub fn output_repos(
    repos: &[Repo],
    format: OutputFormat,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut *writer, repos)?;
            writeln!(writer)?;
        }
        OutputFormat::Paths => {
            for repo in repos {
                writeln!(writer, "{}", repo.path.display())?;
            }
        }
        OutputFormat::PathsNull => {
            for repo in repos {
                write!(writer, "{}\0", repo.path.display())?;
            }
        }
        OutputFormat::Human => {
            for repo in repos {
                writeln!(writer, "{}", super::display::render_repo_line(repo))?;
            }
        }
    }
    Ok(())
}
