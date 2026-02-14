use crate::cli::OutputFormat;
use kissa::config;
use kissa::core::index::Index;

pub fn run(format: OutputFormat) -> anyhow::Result<()> {
    let index = Index::open(&config::index_path())?;

    let summary = index.freshness_summary()?;
    let total = summary.active + summary.recent + summary.stale + summary.dormant + summary.ancient;

    match format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(std::io::stdout(), &summary)?;
            println!();
        }
        _ => {
            println!("{}", crate::cli::display::render_freshness(&summary, total));
        }
    }

    Ok(())
}
