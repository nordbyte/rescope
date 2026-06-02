mod args;
mod commands;
mod output;
mod tui;

use anyhow::Result;
use clap::Parser;

use crate::args::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse().apply_config()?;
    commands::run(cli)
}
