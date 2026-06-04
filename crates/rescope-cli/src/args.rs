use std::collections::HashMap;
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
        long = "config-profile",
        global = true,
        help = "Apply a named profile from the JSON config file"
    )]
    pub config_profile: Option<String>,

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
            if self.config_profile.is_some() {
                anyhow::bail!("--config-profile requires --config");
            }
            return Ok(self);
        };
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let config: CliConfig = serde_json::from_str(&text)
            .with_context(|| format!("parsing config {}", path.display()))?;
        let snapshot_overlay = config.snapshot.clone();
        let live_overlay = config.live.clone();
        let record_overlay = config.record.clone();
        let tree_overlay = config.tree.clone();
        let watch_overlay = config.watch.clone();
        let config = if let Some(profile_name) = &self.config_profile {
            let overlay = config
                .profiles
                .get(profile_name)
                .with_context(|| format!("config profile \"{profile_name}\" not found"))?
                .clone();
            config.with_overlay(Some(&overlay))
        } else {
            config
        };

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
            Some(Command::Snapshot(args)) => {
                apply_snapshot_config(args, &config.with_overlay(snapshot_overlay.as_ref()))?
            }
            Some(Command::Live(args)) => {
                apply_live_config(args, &config.with_overlay(live_overlay.as_ref()))?
            }
            Some(Command::Record(args)) => {
                apply_record_config(args, &config.with_overlay(record_overlay.as_ref()))?
            }
            Some(Command::Replay(_)) => {}
            Some(Command::Tree(args)) => {
                apply_tree_config(args, &config.with_overlay(tree_overlay.as_ref()))?
            }
            Some(Command::Watch(args)) => {
                apply_watch_config(args, &config.with_overlay(watch_overlay.as_ref()))?
            }
            Some(Command::Diff(_)) => {}
            Some(Command::Completions(_) | Command::Man(_)) => {}
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
    pub pids: Option<Vec<u32>>,
    pub users: Option<Vec<String>>,
    pub process: Option<Vec<String>>,
    pub names: Option<Vec<String>>,
    pub name_regexes: Option<Vec<String>>,
    pub command: Option<Vec<String>>,
    pub command_regexes: Option<Vec<String>>,
    pub executable: Option<Vec<String>>,
    pub executable_regexes: Option<Vec<String>>,
    pub parent_pids: Option<Vec<u32>>,
    pub parent_names: Option<Vec<String>>,
    pub parent_regexes: Option<Vec<String>>,
    pub min_cpu: Option<f32>,
    pub min_ram: Option<String>,
    pub min_io: Option<String>,
    pub invert: Option<bool>,
    pub duration: Option<String>,
    pub timeline: Option<usize>,
    pub all: Option<bool>,
    pub show_system: Option<bool>,
    pub once: Option<bool>,
    pub tui: Option<bool>,
    pub plain: Option<bool>,
    pub jsonl: Option<PathBuf>,
    pub csv_stream: Option<PathBuf>,
    pub prometheus: Option<String>,
    pub stream: Option<bool>,
    pub exit_code: Option<u8>,
    pub snapshot: Option<ConfigOverlay>,
    pub live: Option<ConfigOverlay>,
    pub record: Option<ConfigOverlay>,
    pub tree: Option<ConfigOverlay>,
    pub watch: Option<ConfigOverlay>,
    pub profiles: HashMap<String, ConfigOverlay>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct ConfigOverlay {
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
    pub pids: Option<Vec<u32>>,
    pub users: Option<Vec<String>>,
    pub process: Option<Vec<String>>,
    pub names: Option<Vec<String>>,
    pub name_regexes: Option<Vec<String>>,
    pub command: Option<Vec<String>>,
    pub command_regexes: Option<Vec<String>>,
    pub executable: Option<Vec<String>>,
    pub executable_regexes: Option<Vec<String>>,
    pub parent_pids: Option<Vec<u32>>,
    pub parent_names: Option<Vec<String>>,
    pub parent_regexes: Option<Vec<String>>,
    pub min_cpu: Option<f32>,
    pub min_ram: Option<String>,
    pub min_io: Option<String>,
    pub invert: Option<bool>,
    pub duration: Option<String>,
    pub timeline: Option<usize>,
    pub all: Option<bool>,
    pub show_system: Option<bool>,
    pub once: Option<bool>,
    pub tui: Option<bool>,
    pub plain: Option<bool>,
    pub jsonl: Option<PathBuf>,
    pub csv_stream: Option<PathBuf>,
    pub prometheus: Option<String>,
    pub stream: Option<bool>,
    pub exit_code: Option<u8>,
}

impl CliConfig {
    fn with_overlay(&self, overlay: Option<&ConfigOverlay>) -> Self {
        let mut merged = self.clone();
        if let Some(overlay) = overlay {
            merged.profile = overlay.profile.or(merged.profile);
            merged.group = overlay.group.or(merged.group);
            merged.sort = overlay.sort.or(merged.sort);
            merged.limit = overlay.limit.or(merged.limit);
            merged.interval = overlay.interval.clone().or(merged.interval);
            merged.normalize_cpu = overlay.normalize_cpu.or(merged.normalize_cpu);
            merged.show_command = overlay.show_command.or(merged.show_command);
            merged.show_path = overlay.show_path.or(merged.show_path);
            merged.hide_self = overlay.hide_self.or(merged.hide_self);
            merged.include_idle = overlay.include_idle.or(merged.include_idle);
            merged.pids = overlay.pids.clone().or(merged.pids);
            merged.users = overlay.users.clone().or(merged.users);
            merged.process = overlay.process.clone().or(merged.process);
            merged.names = overlay.names.clone().or(merged.names);
            merged.name_regexes = overlay.name_regexes.clone().or(merged.name_regexes);
            merged.command = overlay.command.clone().or(merged.command);
            merged.command_regexes = overlay.command_regexes.clone().or(merged.command_regexes);
            merged.executable = overlay.executable.clone().or(merged.executable);
            merged.executable_regexes = overlay
                .executable_regexes
                .clone()
                .or(merged.executable_regexes);
            merged.parent_pids = overlay.parent_pids.clone().or(merged.parent_pids);
            merged.parent_names = overlay.parent_names.clone().or(merged.parent_names);
            merged.parent_regexes = overlay.parent_regexes.clone().or(merged.parent_regexes);
            merged.min_cpu = overlay.min_cpu.or(merged.min_cpu);
            merged.min_ram = overlay.min_ram.clone().or(merged.min_ram);
            merged.min_io = overlay.min_io.clone().or(merged.min_io);
            merged.invert = overlay.invert.or(merged.invert);
            merged.duration = overlay.duration.clone().or(merged.duration);
            merged.timeline = overlay.timeline.or(merged.timeline);
            merged.all = overlay.all.or(merged.all);
            merged.show_system = overlay.show_system.or(merged.show_system);
            merged.once = overlay.once.or(merged.once);
            merged.tui = overlay.tui.or(merged.tui);
            merged.plain = overlay.plain.or(merged.plain);
            merged.jsonl = overlay.jsonl.clone().or(merged.jsonl);
            merged.csv_stream = overlay.csv_stream.clone().or(merged.csv_stream);
            merged.prometheus = overlay.prometheus.clone().or(merged.prometheus);
            merged.stream = overlay.stream.or(merged.stream);
            merged.exit_code = overlay.exit_code.or(merged.exit_code);
        }
        merged.snapshot = None;
        merged.live = None;
        merged.record = None;
        merged.tree = None;
        merged.watch = None;
        merged
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Snapshot(SnapshotArgs),
    Live(LiveArgs),
    Record(RecordArgs),
    Replay(ReplayArgs),
    Tree(TreeArgs),
    Watch(WatchArgs),
    Diff(DiffArgs),
    Completions(CompletionsArgs),
    Man(ManArgs),
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

    #[arg(long, help = "Hide the system summary before the process table")]
    pub no_system: bool,
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

    #[arg(
        long,
        help = "Stream newline-delimited JSON snapshots to a file or '-' for stdout"
    )]
    pub jsonl: Option<PathBuf>,

    #[arg(
        long = "csv-stream",
        help = "Stream CSV snapshot rows to a file or '-' for stdout"
    )]
    pub csv_stream: Option<PathBuf>,

    #[arg(
        long,
        value_name = "TARGET",
        help = "Publish Prometheus metrics to '-', a file, or an HTTP bind address such as 127.0.0.1:9898"
    )]
    pub prometheus: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct RecordArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(long, default_value = "30s", value_parser = parse_duration_arg, help = "Recording duration, for example 30s or 5m")]
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

    #[arg(
        long = "raw-samples",
        help = "Write replayable raw samples to this JSON file"
    )]
    pub raw_samples: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct ReplayArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(help = "Raw sample JSON written by record --raw-samples")]
    pub input: PathBuf,

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

#[derive(Debug, Clone, Args)]
pub struct TreeArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(long, default_value = "1s", value_parser = parse_duration_arg, help = "Sampling interval used for CPU and I/O deltas")]
    pub interval: Duration,

    #[arg(long, value_enum, default_value = "cpu", help = "Sort sibling nodes")]
    pub sort: CliSortBy,

    #[arg(long, default_value_t = 100, value_parser = parse_limit, help = "Maximum process nodes to print")]
    pub limit: usize,

    #[arg(long, help = "Show every matching node")]
    pub all: bool,

    #[arg(long, help = "Normalize CPU percentages to one logical CPU")]
    pub normalize_cpu: bool,
}

#[derive(Debug, Clone, Args)]
pub struct WatchArgs {
    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(long, default_value = "30s", value_parser = parse_duration_arg, help = "Maximum watch duration")]
    pub duration: Duration,

    #[arg(
        long = "for",
        default_value = "0s",
        value_parser = parse_duration_arg,
        help = "Require the alert to match continuously for this duration"
    )]
    pub for_duration: Duration,

    #[arg(long, default_value = "1s", value_parser = parse_duration_arg, help = "Sampling interval")]
    pub interval: Duration,

    #[arg(long, value_enum, default_value = "cpu", help = "Sort matching rows")]
    pub sort: CliSortBy,

    #[arg(long, default_value_t = 20, value_parser = parse_limit, help = "Maximum rows to print when a match is found")]
    pub limit: usize,

    #[arg(long, help = "Show all matching rows instead of applying --limit")]
    pub all: bool,

    #[arg(long, default_value_t = 10, value_parser = parse_exit_code, help = "Process exit code when the alert matches")]
    pub exit_code: u8,

    #[arg(
        long,
        help = "Print every sample instead of only the first matching sample"
    )]
    pub stream: bool,

    #[arg(long, help = "Normalize CPU percentages to one logical CPU")]
    pub normalize_cpu: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DiffArgs {
    #[arg(help = "Older rescope JSON report")]
    pub before: PathBuf,

    #[arg(help = "Newer rescope JSON report")]
    pub after: PathBuf,

    #[arg(long, default_value_t = 20, value_parser = parse_limit, help = "Maximum diff rows to print")]
    pub limit: usize,

    #[arg(long, help = "Show all changed rows")]
    pub all: bool,
}

#[derive(Debug, Clone, Args)]
pub struct CompletionsArgs {
    #[arg(value_enum, help = "Shell to generate completions for")]
    pub shell: clap_complete::Shell,

    #[arg(short, long, help = "Write completions to this file instead of stdout")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct ManArgs {
    #[arg(
        short,
        long,
        help = "Write the man page to this file instead of stdout"
    )]
    pub output: Option<PathBuf>,
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
    Cgroup,
    Systemd,
    Container,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CliSortBy {
    Cpu,
    CpuMax,
    CpuP95,
    Ram,
    RamAvg,
    RamEnd,
    Read,
    Write,
    Io,
    IoAvg,
    Pid,
    Name,
    User,
    Started,
    Exited,
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
            CliGroupBy::Cgroup => GroupBy::Cgroup,
            CliGroupBy::Systemd => GroupBy::Systemd,
            CliGroupBy::Container => GroupBy::Container,
        }
    }
}

impl From<CliSortBy> for SortBy {
    fn from(value: CliSortBy) -> Self {
        match value {
            CliSortBy::Cpu => SortBy::Cpu,
            CliSortBy::CpuMax => SortBy::CpuMax,
            CliSortBy::CpuP95 => SortBy::CpuP95,
            CliSortBy::Ram => SortBy::Ram,
            CliSortBy::RamAvg => SortBy::RamAvg,
            CliSortBy::RamEnd => SortBy::RamEnd,
            CliSortBy::Read => SortBy::Read,
            CliSortBy::Write => SortBy::Write,
            CliSortBy::Io => SortBy::Io,
            CliSortBy::IoAvg => SortBy::IoAvg,
            CliSortBy::Pid => SortBy::Pid,
            CliSortBy::Name => SortBy::Name,
            CliSortBy::User => SortBy::User,
            CliSortBy::Started => SortBy::Started,
            CliSortBy::Exited => SortBy::Exited,
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
    )?;
    if !args.all
        && let Some(all) = config.all
    {
        args.all = all;
    }
    if !args.show_system
        && let Some(show_system) = config.show_system
    {
        args.show_system = show_system;
        args.no_system = !show_system;
    }
    Ok(())
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
    )?;
    if !args.all
        && let Some(all) = config.all
    {
        args.all = all;
    }
    if !args.once
        && let Some(once) = config.once
    {
        args.once = once;
    }
    if !args.tui
        && let Some(tui) = config.tui
    {
        args.tui = tui;
    }
    if !args.plain
        && let Some(plain) = config.plain
    {
        args.plain = plain;
    }
    if args.jsonl.is_none() {
        args.jsonl = config.jsonl.clone();
    }
    if args.csv_stream.is_none() {
        args.csv_stream = config.csv_stream.clone();
    }
    if args.prometheus.is_none() {
        args.prometheus = config.prometheus.clone();
    }
    Ok(())
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
    if !args.all
        && let Some(all) = config.all
    {
        args.all = all;
    }
    if args.duration == Duration::from_secs(30)
        && let Some(duration) = &config.duration
    {
        args.duration = parse_duration_arg(duration)?;
    }
    if args.timeline == 5
        && let Some(timeline) = config.timeline
    {
        args.timeline = timeline;
    }
    Ok(())
}

fn apply_tree_config(args: &mut TreeArgs, config: &CliConfig) -> AnyhowResult<()> {
    if args.sort == CliSortBy::Cpu
        && let Some(config_sort) = config.sort
    {
        args.sort = config_sort;
    }
    if args.limit == 100
        && let Some(config_limit) = config.limit
    {
        args.limit = parse_limit(&config_limit.to_string())?;
    }
    if args.interval == Duration::from_secs(1)
        && let Some(config_interval) = &config.interval
    {
        args.interval = parse_duration_arg(config_interval)?;
    }
    if !args.normalize_cpu
        && let Some(config_normalize_cpu) = config.normalize_cpu
    {
        args.normalize_cpu = config_normalize_cpu;
    }
    if !args.all
        && let Some(all) = config.all
    {
        args.all = all;
    }
    apply_filter_config(&mut args.filters, config)?;
    Ok(())
}

fn apply_watch_config(args: &mut WatchArgs, config: &CliConfig) -> AnyhowResult<()> {
    if args.sort == CliSortBy::Cpu
        && let Some(config_sort) = config.sort
    {
        args.sort = config_sort;
    }
    if args.limit == 20
        && let Some(config_limit) = config.limit
    {
        args.limit = parse_limit(&config_limit.to_string())?;
    }
    if args.interval == Duration::from_secs(1)
        && let Some(config_interval) = &config.interval
    {
        args.interval = parse_duration_arg(config_interval)?;
    }
    if !args.normalize_cpu
        && let Some(config_normalize_cpu) = config.normalize_cpu
    {
        args.normalize_cpu = config_normalize_cpu;
    }
    if args.duration == Duration::from_secs(30)
        && let Some(duration) = &config.duration
    {
        args.duration = parse_duration_arg(duration)?;
    }
    if !args.all
        && let Some(all) = config.all
    {
        args.all = all;
    }
    if !args.stream
        && let Some(stream) = config.stream
    {
        args.stream = stream;
    }
    if args.exit_code == 10
        && let Some(exit_code) = config.exit_code
    {
        args.exit_code = parse_exit_code(&exit_code.to_string()).map_err(anyhow::Error::msg)?;
    }
    apply_filter_config(&mut args.filters, config)?;
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
    apply_filter_config(filters, config)?;
    Ok(())
}

fn apply_filter_config(filters: &mut FilterArgs, config: &CliConfig) -> AnyhowResult<()> {
    if filters.pids.is_empty()
        && let Some(pids) = &config.pids
    {
        filters.pids = pids.clone();
    }
    if filters.users.is_empty()
        && let Some(users) = &config.users
    {
        filters.users = users.clone();
    }
    if filters.process_substrings.is_empty()
        && let Some(process) = &config.process
    {
        filters.process_substrings = process.clone();
    }
    if filters.names.is_empty()
        && let Some(names) = &config.names
    {
        filters.names = names.clone();
    }
    if filters.name_regexes.is_empty()
        && let Some(regexes) = &config.name_regexes
    {
        for regex in regexes {
            parse_regex_arg(regex).map_err(anyhow::Error::msg)?;
        }
        filters.name_regexes = regexes.clone();
    }
    if filters.command_substrings.is_empty()
        && let Some(command) = &config.command
    {
        filters.command_substrings = command.clone();
    }
    if filters.command_regexes.is_empty()
        && let Some(regexes) = &config.command_regexes
    {
        for regex in regexes {
            parse_regex_arg(regex).map_err(anyhow::Error::msg)?;
        }
        filters.command_regexes = regexes.clone();
    }
    if filters.executable_substrings.is_empty()
        && let Some(executable) = &config.executable
    {
        filters.executable_substrings = executable.clone();
    }
    if filters.executable_regexes.is_empty()
        && let Some(regexes) = &config.executable_regexes
    {
        for regex in regexes {
            parse_regex_arg(regex).map_err(anyhow::Error::msg)?;
        }
        filters.executable_regexes = regexes.clone();
    }
    if filters.parent_pids.is_empty()
        && let Some(parent_pids) = &config.parent_pids
    {
        filters.parent_pids = parent_pids.clone();
    }
    if filters.parent_names.is_empty()
        && let Some(parent_names) = &config.parent_names
    {
        filters.parent_names = parent_names.clone();
    }
    if filters.parent_regexes.is_empty()
        && let Some(regexes) = &config.parent_regexes
    {
        for regex in regexes {
            parse_regex_arg(regex).map_err(anyhow::Error::msg)?;
        }
        filters.parent_regexes = regexes.clone();
    }
    if filters.min_cpu.is_none() {
        filters.min_cpu = config.min_cpu;
    }
    if filters.min_ram.is_none()
        && let Some(min_ram) = &config.min_ram
    {
        filters.min_ram = Some(parse_size_arg(min_ram).map_err(anyhow::Error::msg)?);
    }
    if filters.min_io.is_none()
        && let Some(min_io) = &config.min_io
    {
        filters.min_io = Some(parse_size_arg(min_io).map_err(anyhow::Error::msg)?);
    }
    if !filters.invert
        && let Some(invert) = config.invert
    {
        filters.invert = invert;
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

fn parse_exit_code(input: &str) -> Result<u8, String> {
    let value = input
        .parse::<u8>()
        .map_err(|_| format!("invalid exit code \"{input}\""))?;
    if value == 0 {
        Err("exit code must be between 1 and 255".to_string())
    } else {
        Ok(value)
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
            || matches!(
                self.effective_group(),
                GroupBy::Cgroup | GroupBy::Systemd | GroupBy::Container
            )
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
            || matches!(
                self.effective_group(),
                GroupBy::Cgroup | GroupBy::Systemd | GroupBy::Container
            )
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
            || matches!(
                self.effective_group(),
                GroupBy::Cgroup | GroupBy::Systemd | GroupBy::Container
            )
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

impl ReplayArgs {
    pub fn effective_limit(&self) -> usize {
        if self.all { usize::MAX } else { self.limit }
    }

    pub fn effective_include_idle(&self) -> bool {
        self.all || self.include_idle
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

impl TreeArgs {
    pub fn effective_limit(&self) -> usize {
        if self.all { usize::MAX } else { self.limit }
    }

    pub fn needs_command(&self) -> bool {
        self.filters.needs_command() || self.filters.show_command
    }

    pub fn needs_executable(&self) -> bool {
        self.filters.needs_executable() || self.filters.show_path
    }

    pub fn effective_sort(&self) -> SortBy {
        self.sort.into()
    }

    pub fn effective_show_command(&self) -> bool {
        self.filters.show_command
    }

    pub fn effective_show_path(&self) -> bool {
        self.filters.show_path
    }
}

impl WatchArgs {
    pub fn effective_limit(&self) -> usize {
        if self.all { usize::MAX } else { self.limit }
    }

    pub fn needs_command(&self) -> bool {
        self.filters.needs_command() || self.filters.show_command
    }

    pub fn needs_executable(&self) -> bool {
        self.filters.needs_executable() || self.filters.show_path
    }

    pub fn effective_sort(&self) -> SortBy {
        self.sort.into()
    }

    pub fn effective_show_command(&self) -> bool {
        self.filters.show_command
    }

    pub fn effective_show_path(&self) -> bool {
        self.filters.show_path
    }
}
