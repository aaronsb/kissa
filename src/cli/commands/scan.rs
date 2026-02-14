use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;

use crate::cli::OutputFormat;
use kissa::config;
use kissa::core::classify;
use kissa::core::git_ops;
use kissa::core::index::Index;
use kissa::core::repo::Repo;
use kissa::core::scanner::{self, ScanEvent};

#[derive(clap::Args)]
pub struct ScanArgs {
    /// Full filesystem scan (walks all roots)
    #[arg(long)]
    pub full: bool,

    /// Override scan roots
    #[arg(long)]
    pub roots: Option<Vec<String>>,
}

pub fn run(args: ScanArgs, format: OutputFormat) -> anyhow::Result<()> {
    let cfg = config::load_config()?;
    let index = Index::open(&config::index_path())?;

    let roots: Vec<PathBuf> = if let Some(ref r) = args.roots {
        r.iter().map(PathBuf::from).collect()
    } else {
        cfg.scan.roots.clone()
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    let pb_clone = pb.clone();
    let progress: Option<Box<dyn Fn(ScanEvent) + Send>> = Some(Box::new(move |event| {
        match event {
            ScanEvent::RepoFound(p) => {
                pb_clone.set_message(format!("found {}", p.display()));
            }
            ScanEvent::DirectoryEntered(p) => {
                pb_clone.set_message(format!("scanning {}", p.display()));
            }
            ScanEvent::Skipped { .. } => {}
            ScanEvent::Error { path, error } => {
                pb_clone.set_message(format!("error: {} — {}", path.display(), error));
            }
        }
        pb_clone.tick();
    }));

    let result = scanner::full_scan(&roots, &cfg.scan, progress)?;
    pb.finish_and_clear();

    // Extract vitals and upsert each discovered repo
    let mut upserted = 0;
    for discovered in &result.discovered {
        match git_ops::extract_vitals(&discovered.path) {
            Ok(vitals) => {
                let mut repo = Repo::from_vitals(vitals, discovered.path.clone());
                classify::classify_repo(&mut repo, &cfg);
                if index.upsert_repo(&repo).is_ok() {
                    upserted += 1;
                }
            }
            Err(e) => {
                eprintln!(
                    "  {} could not read {}: {}",
                    "warn:".yellow(),
                    discovered.path.display(),
                    e
                );
            }
        }
    }

    index.record_scan(&roots, upserted)?;

    match format {
        OutputFormat::Json => {
            let summary = serde_json::json!({
                "discovered": result.discovered.len(),
                "upserted": upserted,
                "skipped_excluded": result.skipped_excluded,
                "skipped_mounts": result.skipped_mounts,
                "errors": result.errors.len(),
                "duration_ms": result.duration.as_millis(),
            });
            serde_json::to_writer_pretty(std::io::stdout(), &summary)?;
            println!();
        }
        _ => {
            println!(
                "  {} {} repos in {:.1}s",
                "scanned:".green().bold(),
                result.discovered.len(),
                result.duration.as_secs_f64(),
            );
            println!(
                "  {} {} repos indexed",
                "indexed:".bold(),
                upserted,
            );
            if result.skipped_excluded > 0 {
                println!(
                    "  {} {} paths excluded",
                    "skipped:".dimmed(),
                    result.skipped_excluded,
                );
            }
            if !result.errors.is_empty() {
                println!(
                    "  {} {} errors",
                    "errors:".red(),
                    result.errors.len(),
                );
            }
        }
    }

    Ok(())
}
