use clap::Parser;

mod cli;
mod mcp;

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();

    if args.mcp {
        mcp::serve_stdio()?;
    } else {
        cli::run(args)?;
    }

    Ok(())
}
