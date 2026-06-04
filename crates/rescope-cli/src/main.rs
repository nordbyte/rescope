mod args;
mod commands;
mod output;
mod tui;

use anyhow::Result;
use clap::Parser;

use crate::args::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse().apply_config()?;
    let outcome = commands::run(cli)?;
    if outcome.exit_code != 0 {
        std::process::exit(outcome.exit_code.into());
    }
    Ok(())
}
