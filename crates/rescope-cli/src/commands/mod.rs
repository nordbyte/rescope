pub mod live;
pub mod record;
pub mod snapshot;

use anyhow::Result;

use crate::args::{Cli, Command, LiveArgs};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command.clone() {
        Some(Command::Snapshot(args)) => snapshot::run(&cli, &args),
        Some(Command::Live(args)) => live::run(&cli, &args),
        Some(Command::Record(args)) => record::run(&cli, &args),
        None => {
            let args = LiveArgs {
                filters: Default::default(),
                group: crate::args::CliGroupBy::Process,
                sort: crate::args::CliSortBy::Cpu,
                limit: 20,
                all: false,
                interval: std::time::Duration::from_secs(1),
                normalize_cpu: false,
                once: false,
                tui: false,
                plain: true,
            };
            live::run(&cli, &args)
        }
    }
}
