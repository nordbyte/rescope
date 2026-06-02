use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use rescope_core::{FilterSpec, GroupBy, RescopeError, SortBy, parse_duration};

#[derive(Debug, Parser)]
#[command(name = "rescope")]
#[command(version)]
#[command(about = "Inspect and record resource usage by process and user")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(long, default_value = "auto", global = true)]
    pub color: ColorMode,

    #[arg(long, global = true)]
    pub no_color: bool,

    #[arg(long, global = true)]
    pub json: Option<PathBuf>,

    #[arg(long, global = true)]
    pub csv: Option<PathBuf>,

    #[arg(long, global = true)]
    pub bytes: bool,

    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    #[arg(short, long, global = true)]
    pub quiet: bool,
}

impl Cli {
    pub fn color_enabled(&self) -> bool {
        if self.no_color {
            return false;
        }

        match self.color {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => std::io::stdout().is_terminal(),
        }
    }

    pub fn stdout_export_count(&self) -> usize {
        [&self.json, &self.csv]
            .into_iter()
            .filter(|path| path.as_deref() == Some(std::path::Path::new("-")))
            .count()
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Snapshot(SnapshotArgs),
    Live(LiveArgs),
    Record(RecordArgs),
}

#[derive(Debug, Clone, Args)]
pub struct SnapshotArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(long, value_enum, default_value = "process")]
    pub group: CliGroupBy,

    #[arg(long, value_enum, default_value = "cpu")]
    pub sort: CliSortBy,

    #[arg(long, default_value_t = 20, value_parser = parse_limit)]
    pub limit: usize,

    #[arg(long)]
    pub all: bool,

    #[arg(long, default_value = "1s", value_parser = parse_duration_arg)]
    pub interval: Duration,

    #[arg(long)]
    pub normalize_cpu: bool,

    #[arg(long)]
    pub show_system: bool,
}

#[derive(Debug, Clone, Args)]
pub struct LiveArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(long, value_enum, default_value = "process")]
    pub group: CliGroupBy,

    #[arg(long, value_enum, default_value = "cpu")]
    pub sort: CliSortBy,

    #[arg(long, default_value_t = 20, value_parser = parse_limit)]
    pub limit: usize,

    #[arg(long)]
    pub all: bool,

    #[arg(long, default_value = "1s", value_parser = parse_duration_arg)]
    pub interval: Duration,

    #[arg(long)]
    pub normalize_cpu: bool,

    #[arg(long)]
    pub once: bool,

    #[arg(long)]
    pub tui: bool,

    #[arg(long)]
    pub plain: bool,
}

#[derive(Debug, Clone, Args)]
pub struct RecordArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(long, value_parser = parse_duration_arg)]
    pub duration: Duration,

    #[arg(long, default_value = "1s", value_parser = parse_duration_arg)]
    pub interval: Duration,

    #[arg(long, value_enum, default_value = "process")]
    pub group: CliGroupBy,

    #[arg(long, value_enum, default_value = "io")]
    pub sort: CliSortBy,

    #[arg(long, default_value_t = 20, value_parser = parse_limit)]
    pub limit: usize,

    #[arg(long)]
    pub all: bool,

    #[arg(long, default_value_t = 5)]
    pub timeline: usize,

    #[arg(long)]
    pub normalize_cpu: bool,

    #[arg(long)]
    pub include_idle: bool,
}

#[derive(Debug, Clone, Args, Default)]
pub struct FilterArgs {
    #[arg(long = "pid", action = ArgAction::Append)]
    pub pids: Vec<u32>,

    #[arg(long = "user", action = ArgAction::Append)]
    pub users: Vec<String>,

    #[arg(long = "name", action = ArgAction::Append)]
    pub names: Vec<String>,

    #[arg(long = "cmd", action = ArgAction::Append)]
    pub command_substrings: Vec<String>,

    #[arg(long)]
    pub hide_self: bool,

    #[arg(long)]
    pub show_command: bool,
}

impl FilterArgs {
    pub fn to_filter_spec(&self) -> FilterSpec {
        FilterSpec {
            pids: self.pids.clone(),
            users: self.users.clone(),
            names: self.names.clone(),
            command_substrings: self.command_substrings.clone(),
            hide_self: self.hide_self,
        }
    }

    pub fn needs_command(&self) -> bool {
        self.show_command || !self.command_substrings.is_empty()
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliGroupBy {
    Process,
    Name,
    User,
    Command,
    Executable,
    Parent,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliSortBy {
    Cpu,
    Ram,
    Read,
    Write,
    Io,
    Pid,
    Name,
    User,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

impl From<CliGroupBy> for GroupBy {
    fn from(value: CliGroupBy) -> Self {
        match value {
            CliGroupBy::Process => GroupBy::Process,
            CliGroupBy::Name => GroupBy::Name,
            CliGroupBy::User => GroupBy::User,
            CliGroupBy::Command => GroupBy::Command,
            CliGroupBy::Executable => GroupBy::Executable,
            CliGroupBy::Parent => GroupBy::Parent,
        }
    }
}

impl From<CliSortBy> for SortBy {
    fn from(value: CliSortBy) -> Self {
        match value {
            CliSortBy::Cpu => SortBy::Cpu,
            CliSortBy::Ram => SortBy::Ram,
            CliSortBy::Read => SortBy::Read,
            CliSortBy::Write => SortBy::Write,
            CliSortBy::Io => SortBy::Io,
            CliSortBy::Pid => SortBy::Pid,
            CliSortBy::Name => SortBy::Name,
            CliSortBy::User => SortBy::User,
        }
    }
}

fn parse_duration_arg(input: &str) -> Result<Duration, RescopeError> {
    parse_duration(input)
}

fn parse_limit(input: &str) -> Result<usize, RescopeError> {
    let limit = input
        .parse::<usize>()
        .map_err(|_| RescopeError::InvalidLimit)?;
    if limit == 0 {
        Err(RescopeError::InvalidLimit)
    } else {
        Ok(limit)
    }
}

impl SnapshotArgs {
    pub fn effective_limit(&self) -> usize {
        if self.all { usize::MAX } else { self.limit }
    }

    pub fn needs_command(&self) -> bool {
        self.filters.needs_command() || matches!(self.group, CliGroupBy::Command)
    }

    pub fn needs_executable(&self) -> bool {
        matches!(self.group, CliGroupBy::Executable)
    }
}

impl LiveArgs {
    pub fn effective_limit(&self) -> usize {
        if self.all { usize::MAX } else { self.limit }
    }

    pub fn needs_command(&self) -> bool {
        self.filters.needs_command() || matches!(self.group, CliGroupBy::Command)
    }

    pub fn needs_executable(&self) -> bool {
        matches!(self.group, CliGroupBy::Executable)
    }
}

impl RecordArgs {
    pub fn effective_limit(&self) -> usize {
        if self.all { usize::MAX } else { self.limit }
    }

    pub fn effective_include_idle(&self) -> bool {
        self.all || self.include_idle
    }

    pub fn needs_command(&self) -> bool {
        self.filters.needs_command() || matches!(self.group, CliGroupBy::Command)
    }

    pub fn needs_executable(&self) -> bool {
        matches!(self.group, CliGroupBy::Executable)
    }
}
