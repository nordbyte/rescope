pub mod app;
pub mod view;

use anyhow::Result;

use crate::args::{Cli, LiveArgs};

pub fn is_available() -> bool {
    true
}

pub fn run_live(cli: &Cli, args: &LiveArgs) -> Result<()> {
    app::run_live(cli, args)
}
