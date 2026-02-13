pub mod commands;
pub mod display;
pub mod output;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kissa", about = "Finally herd your repos.", version)]
pub struct Cli {
    /// Start MCP server over stdio
    #[arg(long)]
    pub mcp: bool,

    /// Output format
    #[arg(long, global = true, value_enum, default_value = "human")]
    pub format: OutputFormat,

    /// Cat mode difficulty names
    #[arg(long, global = true)]
    pub cat_mode: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan filesystem for git repositories
    Scan(commands::scan::ScanArgs),
    /// List catalogued repositories
    List(commands::list::ListArgs),
    /// Show detailed status of a repository
    Status(commands::status::StatusArgs),
    /// Show full info dump for a repository
    Info(commands::info::InfoArgs),
    /// Show freshness overview
    Freshness,
    /// Show current configuration
    Config,
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Paths,
    PathsNull,
}

/// Dispatch a CLI command.
pub fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Some(Commands::Scan(args)) => commands::scan::run(args, cli.format),
        Some(Commands::List(args)) => commands::list::run(args, cli.format),
        Some(Commands::Status(args)) => commands::status::run(args, cli.format),
        Some(Commands::Info(args)) => commands::info::run(args, cli.format),
        Some(Commands::Freshness) => commands::freshness::run(cli.format),
        Some(Commands::Config) => commands::config::run(cli.format),
        None => {
            // No subcommand — print help
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}
