pub mod app;
pub mod labels;
pub mod view;

use std::io::IsTerminal;

use anyhow::Result;

use crate::args::{Cli, LiveArgs};

pub fn is_available() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

pub fn run_live(cli: &Cli, args: &LiveArgs) -> Result<()> {
    app::run_live(cli, args)
}
