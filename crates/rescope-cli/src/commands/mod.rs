pub mod diff;
pub mod live;
pub mod record;
pub mod replay;
pub mod snapshot;
pub mod tree;
pub mod watch;

use anyhow::Result;
use clap::CommandFactory;
use std::io::Write;

use crate::args::{Cli, Command, CompletionsArgs, LiveArgs, ManArgs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandOutcome {
    pub exit_code: u8,
}

impl CommandOutcome {
    pub fn success() -> Self {
        Self { exit_code: 0 }
    }

    pub fn with_exit_code(exit_code: u8) -> Self {
        Self { exit_code }
    }
}

pub fn run(cli: Cli) -> Result<CommandOutcome> {
    match cli.command.clone() {
        Some(Command::Snapshot(args)) => {
            snapshot::run(&cli, &args)?;
            Ok(CommandOutcome::success())
        }
        Some(Command::Live(args)) => {
            live::run(&cli, &args)?;
            Ok(CommandOutcome::success())
        }
        Some(Command::Record(args)) => {
            record::run(&cli, &args)?;
            Ok(CommandOutcome::success())
        }
        Some(Command::Replay(args)) => {
            replay::run(&cli, &args)?;
            Ok(CommandOutcome::success())
        }
        Some(Command::Tree(args)) => {
            tree::run(&cli, &args)?;
            Ok(CommandOutcome::success())
        }
        Some(Command::Watch(args)) => watch::run(&cli, &args),
        Some(Command::Diff(args)) => {
            diff::run(&cli, &args)?;
            Ok(CommandOutcome::success())
        }
        Some(Command::Completions(args)) => {
            completions::run(&args)?;
            Ok(CommandOutcome::success())
        }
        Some(Command::Man(args)) => {
            man::run(&args)?;
            Ok(CommandOutcome::success())
        }
        None => {
            let args = LiveArgs {
                filters: Default::default(),
                profile: None,
                group: crate::args::CliGroupBy::Process,
                sort: crate::args::CliSortBy::Cpu,
                limit: 20,
                all: false,
                interval: std::time::Duration::from_secs(1),
                normalize_cpu: false,
                once: false,
                tui: false,
                plain: true,
                jsonl: None,
                csv_stream: None,
                prometheus: None,
            };
            live::run(&cli, &args)?;
            Ok(CommandOutcome::success())
        }
    }
}

mod completions {
    use super::*;

    pub fn run(args: &CompletionsArgs) -> Result<()> {
        let mut command = Cli::command();
        if let Some(path) = &args.output {
            let mut file = std::fs::File::create(path)?;
            clap_complete::generate(args.shell, &mut command, "rescope", &mut file);
            file.flush()?;
        } else {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            clap_complete::generate(args.shell, &mut command, "rescope", &mut handle);
            handle.flush()?;
        }
        Ok(())
    }
}

mod man {
    use super::*;

    pub fn run(args: &ManArgs) -> Result<()> {
        let man = clap_mangen::Man::new(Cli::command());
        if let Some(path) = &args.output {
            let mut file = std::fs::File::create(path)?;
            man.render(&mut file)?;
            file.flush()?;
        } else {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            man.render(&mut handle)?;
            handle.flush()?;
        }
        Ok(())
    }
}

pub fn verbose(cli: &Cli, message: impl AsRef<str>) {
    if cli.verbose > 0 && !cli.quiet {
        eprintln!("rescope: {}", message.as_ref());
    }
}
