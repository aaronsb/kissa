use crate::cli::OutputFormat;
use kissa::config;

pub fn run(format: OutputFormat) -> anyhow::Result<()> {
    let cfg = config::load_config()?;

    match format {
        OutputFormat::Json => {
            serde_json::to_writer_pretty(std::io::stdout(), &cfg)?;
            println!();
        }
        _ => {
            // Human-readable: just use TOML format
            let toml_str = toml::to_string_pretty(&cfg)?;
            println!("{}", toml_str);
        }
    }

    Ok(())
}
