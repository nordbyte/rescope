use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result as AnyhowResult};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use regex::RegexBuilder;
use rescope_core::{FilterSpec, GroupBy, RescopeError, SortBy, parse_duration};
use serde::Deserialize;

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
        long,
        global = true,
        help = "Load default options from a JSON config file"
    )]
    pub config: Option<PathBuf>,

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

    pub fn apply_config(mut self) -> AnyhowResult<Self> {
        let Some(path) = self.config.clone() else {
            return Ok(self);
        };
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let config: CliConfig = serde_json::from_str(&text)
            .with_context(|| format!("parsing config {}", path.display()))?;

        if !self.bytes
            && let Some(bytes) = config.bytes
        {
            self.bytes = bytes;
        }
        if !self.quiet
            && let Some(quiet) = config.quiet
        {
            self.quiet = quiet;
        }
        if !self.no_color
            && let Some(no_color) = config.no_color
        {
            self.no_color = no_color;
        }
        if matches!(self.color, ColorMode::Auto)
            && let Some(color) = config.color
        {
            self.color = color;
        }

        match &mut self.command {
            Some(Command::Snapshot(args)) => apply_snapshot_config(args, &config)?,
            Some(Command::Live(args)) => apply_live_config(args, &config)?,
            Some(Command::Record(args)) => apply_record_config(args, &config)?,
            None => {}
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct CliConfig {
    pub color: Option<ColorMode>,
    pub no_color: Option<bool>,
    pub bytes: Option<bool>,
    pub quiet: Option<bool>,
    pub profile: Option<CliProfile>,
    pub group: Option<CliGroupBy>,
    pub sort: Option<CliSortBy>,
    pub limit: Option<usize>,
    pub interval: Option<String>,
    pub normalize_cpu: Option<bool>,
    pub show_command: Option<bool>,
    pub show_path: Option<bool>,
    pub hide_self: Option<bool>,
    pub include_idle: Option<bool>,
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
        help = "Apply a convenience preset for common investigations"
    )]
    pub profile: Option<CliProfile>,

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
        help = "Apply a convenience preset for common investigations"
    )]
    pub profile: Option<CliProfile>,

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

    #[arg(
        long,
        value_enum,
        help = "Apply a convenience preset for common investigations"
    )]
    pub profile: Option<CliProfile>,

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

    #[arg(long = "process", action = ArgAction::Append, help = "Flexible process search across PID, name, executable path and command line; repeat for alternatives")]
    pub process_substrings: Vec<String>,

    #[arg(long = "name", action = ArgAction::Append, help = "Only include process names containing this text; repeat for alternatives")]
    pub names: Vec<String>,

    #[arg(long = "name-regex", action = ArgAction::Append, value_parser = parse_regex_arg, help = "Only include process names matching this case-insensitive regex")]
    pub name_regexes: Vec<String>,

    #[arg(long = "cmd", action = ArgAction::Append, help = "Only include command lines containing this text; repeat for alternatives")]
    pub command_substrings: Vec<String>,

    #[arg(long = "cmd-regex", action = ArgAction::Append, value_parser = parse_regex_arg, help = "Only include command lines matching this case-insensitive regex")]
    pub command_regexes: Vec<String>,

    #[arg(long = "exe", alias = "path", action = ArgAction::Append, help = "Only include executable paths containing this text; repeat for alternatives")]
    pub executable_substrings: Vec<String>,

    #[arg(long = "exe-regex", action = ArgAction::Append, value_parser = parse_regex_arg, help = "Only include executable paths matching this case-insensitive regex")]
    pub executable_regexes: Vec<String>,

    #[arg(long = "parent", action = ArgAction::Append, help = "Only include processes whose parent PID matches; repeat for alternatives")]
    pub parent_pids: Vec<u32>,

    #[arg(long = "parent-name", action = ArgAction::Append, help = "Only include parent process names containing this text; repeat for alternatives")]
    pub parent_names: Vec<String>,

    #[arg(long = "parent-regex", action = ArgAction::Append, value_parser = parse_regex_arg, help = "Only include parent process names matching this case-insensitive regex")]
    pub parent_regexes: Vec<String>,

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

    #[arg(long, help = "Display full executable paths where available")]
    pub show_path: bool,
}

impl FilterArgs {
    pub fn to_filter_spec(&self) -> FilterSpec {
        FilterSpec {
            pids: self.pids.clone(),
            users: self.users.clone(),
            process_substrings: self.process_substrings.clone(),
            names: self.names.clone(),
            name_regexes: self.name_regexes.clone(),
            command_substrings: self.command_substrings.clone(),
            command_regexes: self.command_regexes.clone(),
            executable_substrings: self.executable_substrings.clone(),
            executable_regexes: self.executable_regexes.clone(),
            parent_pids: self.parent_pids.clone(),
            parent_names: self.parent_names.clone(),
            parent_regexes: self.parent_regexes.clone(),
            min_cpu_percent: self.min_cpu,
            min_ram_bytes: self.min_ram,
            min_io_delta_bytes: self.min_io,
            hide_self: self.hide_self,
            invert_match: self.invert,
        }
    }

    pub fn needs_command(&self) -> bool {
        self.show_command
            || !self.process_substrings.is_empty()
            || !self.command_substrings.is_empty()
            || !self.command_regexes.is_empty()
    }

    pub fn needs_executable(&self) -> bool {
        self.show_path
            || !self.process_substrings.is_empty()
            || !self.executable_substrings.is_empty()
            || !self.executable_regexes.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CliGroupBy {
    Process,
    Name,
    User,
    Command,
    Executable,
    Parent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CliProfile {
    Cpu,
    Memory,
    Io,
    Commands,
    Users,
    Tree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

fn apply_snapshot_config(args: &mut SnapshotArgs, config: &CliConfig) -> AnyhowResult<()> {
    apply_common_config(
        &mut args.filters,
        &mut args.profile,
        &mut args.group,
        &mut args.sort,
        &mut args.limit,
        &mut args.interval,
        &mut args.normalize_cpu,
        config,
        CliSortBy::Cpu,
    )
}

fn apply_live_config(args: &mut LiveArgs, config: &CliConfig) -> AnyhowResult<()> {
    apply_common_config(
        &mut args.filters,
        &mut args.profile,
        &mut args.group,
        &mut args.sort,
        &mut args.limit,
        &mut args.interval,
        &mut args.normalize_cpu,
        config,
        CliSortBy::Cpu,
    )
}

fn apply_record_config(args: &mut RecordArgs, config: &CliConfig) -> AnyhowResult<()> {
    apply_common_config(
        &mut args.filters,
        &mut args.profile,
        &mut args.group,
        &mut args.sort,
        &mut args.limit,
        &mut args.interval,
        &mut args.normalize_cpu,
        config,
        CliSortBy::Io,
    )?;
    if !args.include_idle
        && let Some(include_idle) = config.include_idle
    {
        args.include_idle = include_idle;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_common_config(
    filters: &mut FilterArgs,
    profile: &mut Option<CliProfile>,
    group: &mut CliGroupBy,
    sort: &mut CliSortBy,
    limit: &mut usize,
    interval: &mut Duration,
    normalize_cpu: &mut bool,
    config: &CliConfig,
    default_sort: CliSortBy,
) -> AnyhowResult<()> {
    if profile.is_none() {
        *profile = config.profile;
    }
    if matches!(*group, CliGroupBy::Process)
        && let Some(config_group) = config.group
    {
        *group = config_group;
    }
    if *sort == default_sort
        && let Some(config_sort) = config.sort
    {
        *sort = config_sort;
    }
    if *limit == 20
        && let Some(config_limit) = config.limit
    {
        *limit = parse_limit(&config_limit.to_string())?;
    }
    if *interval == Duration::from_secs(1)
        && let Some(config_interval) = &config.interval
    {
        *interval = parse_duration_arg(config_interval)?;
    }
    if !*normalize_cpu && let Some(config_normalize_cpu) = config.normalize_cpu {
        *normalize_cpu = config_normalize_cpu;
    }
    if !filters.show_command
        && let Some(show_command) = config.show_command
    {
        filters.show_command = show_command;
    }
    if !filters.show_path
        && let Some(show_path) = config.show_path
    {
        filters.show_path = show_path;
    }
    if !filters.hide_self
        && let Some(hide_self) = config.hide_self
    {
        filters.hide_self = hide_self;
    }
    Ok(())
}

impl CliProfile {
    fn group(self) -> CliGroupBy {
        match self {
            CliProfile::Cpu | CliProfile::Memory | CliProfile::Io => CliGroupBy::Process,
            CliProfile::Commands => CliGroupBy::Command,
            CliProfile::Users => CliGroupBy::User,
            CliProfile::Tree => CliGroupBy::Parent,
        }
    }

    fn sort(self) -> CliSortBy {
        match self {
            CliProfile::Cpu | CliProfile::Commands | CliProfile::Tree => CliSortBy::Cpu,
            CliProfile::Memory | CliProfile::Users => CliSortBy::Ram,
            CliProfile::Io => CliSortBy::Io,
        }
    }

    fn show_command(self) -> bool {
        matches!(self, CliProfile::Commands)
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
        self.filters.needs_command()
            || self.effective_show_command()
            || matches!(self.effective_group(), GroupBy::Command)
    }

    pub fn needs_executable(&self) -> bool {
        self.filters.needs_executable()
            || self.effective_show_path()
            || matches!(self.effective_group(), GroupBy::Executable)
    }

    pub fn effective_group(&self) -> GroupBy {
        self.profile
            .map(CliProfile::group)
            .unwrap_or(self.group)
            .into()
    }

    pub fn effective_sort(&self) -> SortBy {
        self.profile
            .map(CliProfile::sort)
            .unwrap_or(self.sort)
            .into()
    }

    pub fn effective_show_command(&self) -> bool {
        self.filters.show_command || self.profile.is_some_and(CliProfile::show_command)
    }

    pub fn effective_show_path(&self) -> bool {
        self.filters.show_path
    }
}

impl LiveArgs {
    pub fn effective_limit(&self) -> usize {
        if self.all { usize::MAX } else { self.limit }
    }

    pub fn needs_command(&self) -> bool {
        self.filters.needs_command()
            || self.effective_show_command()
            || matches!(self.effective_group(), GroupBy::Command)
    }

    pub fn needs_executable(&self) -> bool {
        self.filters.needs_executable()
            || self.effective_show_path()
            || matches!(self.effective_group(), GroupBy::Executable)
    }

    pub fn effective_group(&self) -> GroupBy {
        self.profile
            .map(CliProfile::group)
            .unwrap_or(self.group)
            .into()
    }

    pub fn effective_sort(&self) -> SortBy {
        self.profile
            .map(CliProfile::sort)
            .unwrap_or(self.sort)
            .into()
    }

    pub fn effective_show_command(&self) -> bool {
        self.filters.show_command || self.profile.is_some_and(CliProfile::show_command)
    }

    pub fn effective_show_path(&self) -> bool {
        self.filters.show_path
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
        self.filters.needs_command()
            || self.effective_show_command()
            || matches!(self.effective_group(), GroupBy::Command)
    }

    pub fn needs_executable(&self) -> bool {
        self.filters.needs_executable()
            || self.effective_show_path()
            || matches!(self.effective_group(), GroupBy::Executable)
    }

    pub fn effective_group(&self) -> GroupBy {
        self.profile
            .map(CliProfile::group)
            .unwrap_or(self.group)
            .into()
    }

    pub fn effective_sort(&self) -> SortBy {
        self.profile
            .map(CliProfile::sort)
            .unwrap_or(self.sort)
            .into()
    }

    pub fn effective_show_command(&self) -> bool {
        self.filters.show_command || self.profile.is_some_and(CliProfile::show_command)
    }

    pub fn effective_show_path(&self) -> bool {
        self.filters.show_path
    }
}
