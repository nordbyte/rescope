use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use regex::RegexBuilder;
use rescope_core::{FilterSpec, GroupBy, RescopeError, SortBy, parse_duration};

#[derive(Debug, Parser)]
#[command(name = "rescope")]
#[command(version)]
#[command(about = "Inspect and record resource usage by process and user")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(
        long,
        default_value = "auto",
        global = true,
        help = "Control ANSI color output"
    )]
    pub color: ColorMode,

    #[arg(long, global = true, help = "Disable ANSI color output")]
    pub no_color: bool,

    #[arg(
        long,
        global = true,
        help = "Write JSON output to a file or '-' for stdout"
    )]
    pub json: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        help = "Write CSV output to a file or '-' for stdout"
    )]
    pub csv: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        help = "Print byte values without human-readable units"
    )]
    pub bytes: bool,

    #[arg(short, long, action = ArgAction::Count, global = true, help = "Increase diagnostic output")]
    pub verbose: u8,

    #[arg(
        short,
        long,
        global = true,
        help = "Suppress table output when exporting"
    )]
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

    #[arg(
        long,
        value_enum,
        default_value = "process",
        help = "Group rows before sorting"
    )]
    pub group: CliGroupBy,

    #[arg(
        long,
        value_enum,
        default_value = "cpu",
        help = "Sort rows by a metric or label"
    )]
    pub sort: CliSortBy,

    #[arg(long, default_value_t = 20, value_parser = parse_limit, help = "Maximum rows to print")]
    pub limit: usize,

    #[arg(long, help = "Show all matching rows instead of applying --limit")]
    pub all: bool,

    #[arg(long, default_value = "1s", value_parser = parse_duration_arg, help = "Sampling interval used for CPU and I/O deltas")]
    pub interval: Duration,

    #[arg(long, help = "Normalize CPU percentages to one logical CPU")]
    pub normalize_cpu: bool,

    #[arg(long, help = "Print system summary before the process table")]
    pub show_system: bool,
}

#[derive(Debug, Clone, Args)]
pub struct LiveArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(
        long,
        value_enum,
        default_value = "process",
        help = "Group rows before sorting"
    )]
    pub group: CliGroupBy,

    #[arg(
        long,
        value_enum,
        default_value = "cpu",
        help = "Sort rows by a metric or label"
    )]
    pub sort: CliSortBy,

    #[arg(long, default_value_t = 20, value_parser = parse_limit, help = "Maximum rows to print")]
    pub limit: usize,

    #[arg(long, help = "Show all matching rows instead of applying --limit")]
    pub all: bool,

    #[arg(long, default_value = "1s", value_parser = parse_duration_arg, help = "Refresh interval")]
    pub interval: Duration,

    #[arg(long, help = "Normalize CPU percentages to one logical CPU")]
    pub normalize_cpu: bool,

    #[arg(long, help = "Render one live sample and exit")]
    pub once: bool,

    #[arg(long, help = "Use the interactive terminal UI")]
    pub tui: bool,

    #[arg(long, help = "Force plain terminal refresh mode")]
    pub plain: bool,
}

#[derive(Debug, Clone, Args)]
pub struct RecordArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(long, value_parser = parse_duration_arg, help = "Recording duration, for example 30s or 5m")]
    pub duration: Duration,

    #[arg(long, default_value = "1s", value_parser = parse_duration_arg, help = "Sampling interval")]
    pub interval: Duration,

    #[arg(
        long,
        value_enum,
        default_value = "process",
        help = "Group rows before sorting"
    )]
    pub group: CliGroupBy,

    #[arg(long, value_enum, default_value = "io", help = "Sort aggregate rows")]
    pub sort: CliSortBy,

    #[arg(long, default_value_t = 20, value_parser = parse_limit, help = "Maximum rows to print")]
    pub limit: usize,

    #[arg(long, help = "Show all matching rows instead of applying --limit")]
    pub all: bool,

    #[arg(long, default_value_t = 5, help = "Number of RAM timelines to print")]
    pub timeline: usize,

    #[arg(long, help = "Normalize CPU percentages to one logical CPU")]
    pub normalize_cpu: bool,

    #[arg(long, help = "Keep rows without CPU, I/O, or RAM delta activity")]
    pub include_idle: bool,
}

#[derive(Debug, Clone, Args, Default)]
pub struct FilterArgs {
    #[arg(long = "pid", action = ArgAction::Append, help = "Only include a PID; repeat for multiple PIDs")]
    pub pids: Vec<u32>,

    #[arg(long = "user", action = ArgAction::Append, help = "Only include a user name or user id; repeat for multiple users")]
    pub users: Vec<String>,

    #[arg(long = "name", action = ArgAction::Append, help = "Only include process names containing this text; repeat for alternatives")]
    pub names: Vec<String>,

    #[arg(long = "name-regex", action = ArgAction::Append, value_parser = parse_regex_arg, help = "Only include process names matching this case-insensitive regex")]
    pub name_regexes: Vec<String>,

    #[arg(long = "cmd", action = ArgAction::Append, help = "Only include command lines containing this text; repeat for alternatives")]
    pub command_substrings: Vec<String>,

    #[arg(long = "cmd-regex", action = ArgAction::Append, value_parser = parse_regex_arg, help = "Only include command lines matching this case-insensitive regex")]
    pub command_regexes: Vec<String>,

    #[arg(long, value_parser = parse_non_negative_f32, help = "Only include processes at or above this CPU percentage")]
    pub min_cpu: Option<f32>,

    #[arg(long, value_parser = parse_size_arg, help = "Only include processes using at least this much RAM, for example 512MiB")]
    pub min_ram: Option<u64>,

    #[arg(long, value_parser = parse_size_arg, help = "Only include processes with at least this read+write delta per sample")]
    pub min_io: Option<u64>,

    #[arg(long, help = "Invert PID/user/name/command/threshold filters")]
    pub invert: bool,

    #[arg(long, help = "Hide the current rescope process")]
    pub hide_self: bool,

    #[arg(long, help = "Display full command lines where available")]
    pub show_command: bool,
}

impl FilterArgs {
    pub fn to_filter_spec(&self) -> FilterSpec {
        FilterSpec {
            pids: self.pids.clone(),
            users: self.users.clone(),
            names: self.names.clone(),
            name_regexes: self.name_regexes.clone(),
            command_substrings: self.command_substrings.clone(),
            command_regexes: self.command_regexes.clone(),
            min_cpu_percent: self.min_cpu,
            min_ram_bytes: self.min_ram,
            min_io_delta_bytes: self.min_io,
            hide_self: self.hide_self,
            invert_match: self.invert,
        }
    }

    pub fn needs_command(&self) -> bool {
        self.show_command || !self.command_substrings.is_empty() || !self.command_regexes.is_empty()
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

fn parse_regex_arg(input: &str) -> Result<String, String> {
    RegexBuilder::new(input)
        .case_insensitive(true)
        .build()
        .map(|_| input.to_string())
        .map_err(|error| format!("invalid regex \"{input}\": {error}"))
}

fn parse_non_negative_f32(input: &str) -> Result<f32, String> {
    let value = input
        .parse::<f32>()
        .map_err(|_| format!("invalid percentage \"{input}\""))?;
    if value.is_sign_negative() || !value.is_finite() {
        Err(format!(
            "percentage must be finite and non-negative: {input}"
        ))
    } else {
        Ok(value)
    }
}

fn parse_size_arg(input: &str) -> Result<u64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("size must not be empty".to_string());
    }

    let number_len = trimmed
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit() || *ch == '.')
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if number_len == 0 {
        return Err(format!("invalid size \"{input}\""));
    }

    let number = trimmed[..number_len]
        .parse::<f64>()
        .map_err(|_| format!("invalid size \"{input}\""))?;
    if number.is_sign_negative() || !number.is_finite() {
        return Err(format!("size must be finite and non-negative: {input}"));
    }

    let suffix = trimmed[number_len..].trim().to_ascii_lowercase();
    let multiplier = match suffix.as_str() {
        "" | "b" | "byte" | "bytes" => 1.0,
        "k" | "kb" | "kib" => 1024.0,
        "m" | "mb" | "mib" => 1024.0 * 1024.0,
        "g" | "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => {
            return Err(format!(
                "unknown size unit \"{}\"",
                trimmed[number_len..].trim()
            ));
        }
    };

    let bytes = number * multiplier;
    if bytes > u64::MAX as f64 {
        Err(format!("size is too large: {input}"))
    } else {
        Ok(bytes.round() as u64)
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
