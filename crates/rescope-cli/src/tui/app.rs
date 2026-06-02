use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, size as terminal_size,
};
use rescope_core::{
    FilterSpec, GroupBy, RawProcessSample, RecordingReport, RecordingReportOptions, SampleSource,
    SamplerConfig, SnapshotReport, SnapshotReportOptions, SnapshotRow, SortBy, SysinfoSampler,
    SystemSample, build_recording_report, build_snapshot_report, filter_sample, format_bps,
    format_bytes, system_time_ms,
};

use crate::args::{Cli, LiveArgs};
use crate::output::{
    csv, json,
    table::{self, SnapshotColumns, SnapshotRenderOptions},
};
use crate::tui::view;

const GROUP_OPTIONS: [GroupBy; 6] = [
    GroupBy::Process,
    GroupBy::Name,
    GroupBy::User,
    GroupBy::Command,
    GroupBy::Executable,
    GroupBy::Parent,
];

const OPTIONS_ITEMS: [&str; 9] = [
    "Sort",
    "Group",
    "Filters",
    "View",
    "Sampling",
    "Recording",
    "Export",
    "Details",
    "Help",
];
const FILTER_ITEMS: [&str; 8] = [
    "Edit search",
    "Clear search",
    "Toggle invert filters",
    "Toggle hide self",
    "Cycle min CPU",
    "Cycle min RAM",
    "Cycle min I/O",
    "Clear thresholds",
];
const VIEW_ITEMS: [&str; 8] = [
    "Toggle normalized CPU",
    "Toggle raw bytes",
    "Toggle command display",
    "Toggle PID column",
    "Toggle user columns",
    "Toggle rate columns",
    "Toggle total columns",
    "Toggle top column",
];
const SAMPLING_ITEMS: [&str; 5] = [
    "Increase row limit",
    "Decrease row limit",
    "Faster refresh",
    "Slower refresh",
    "Pause or resume",
];
const RECORDING_ITEMS: [&str; 7] = [
    "Start recording",
    "Stop recording",
    "Longer duration",
    "Shorter duration",
    "Toggle include idle",
    "Export recording JSON",
    "Export recording CSV",
];
const EXPORT_ITEMS: [&str; 4] = [
    "Snapshot JSON",
    "Snapshot CSV",
    "Recording JSON",
    "Recording CSV",
];
const INTERVAL_STEPS: [Duration; 6] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(5),
    Duration::from_secs(10),
];
const RECORD_DURATIONS: [Duration; 5] = [
    Duration::from_secs(10),
    Duration::from_secs(30),
    Duration::from_secs(60),
    Duration::from_secs(120),
    Duration::from_secs(300),
];
const CPU_THRESHOLDS: [Option<f32>; 5] = [None, Some(1.0), Some(5.0), Some(10.0), Some(25.0)];
const RAM_THRESHOLDS: [Option<u64>; 5] = [
    None,
    Some(128 * 1024 * 1024),
    Some(512 * 1024 * 1024),
    Some(1024 * 1024 * 1024),
    Some(4 * 1024 * 1024 * 1024),
];
const IO_THRESHOLDS: [Option<u64>; 5] = [
    None,
    Some(4 * 1024),
    Some(64 * 1024),
    Some(1024 * 1024),
    Some(16 * 1024 * 1024),
];
const STATUS_VISIBLE_TICKS: u64 = 4;
const DEFAULT_VIEWPORT: Viewport = Viewport {
    width: 120,
    height: 40,
};

#[derive(Debug)]
pub struct TuiApp {
    tick_count: u64,
    group_by: GroupBy,
    sort_by: SortBy,
    filter: FilterSpec,
    search_query: String,
    selected_row: usize,
    limit: usize,
    interval: Duration,
    normalize_cpu: bool,
    raw_bytes: bool,
    show_command: bool,
    columns: SnapshotColumns,
    paused: bool,
    overlay: Overlay,
    status_message: Option<StatusMessage>,
    pending_export: Option<PendingExport>,
    last_report: Option<SnapshotReport>,
    record_duration: Duration,
    recording_include_idle: bool,
    recording: Option<RecordingSession>,
    last_recording_report: Option<RecordingReport>,
}

#[derive(Debug, Clone)]
enum Overlay {
    None,
    Help,
    Options {
        selected: usize,
    },
    Sort {
        selected: usize,
    },
    Group {
        selected: usize,
    },
    Filter {
        selected: usize,
    },
    View {
        selected: usize,
    },
    Sampling {
        selected: usize,
    },
    Recording {
        selected: usize,
    },
    Export {
        selected: usize,
    },
    ExportPath {
        target: ExportTarget,
        format: ExportFormat,
        input: String,
    },
    Detail {
        row: Box<SnapshotRow>,
    },
    Search {
        input: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportTarget {
    Snapshot,
    Recording,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingExport {
    target: ExportTarget,
    format: ExportFormat,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusMessage {
    text: String,
    expires_at_tick: u64,
}

#[derive(Debug, Clone)]
struct RecordingSession {
    started_at: Instant,
    duration: Duration,
    samples: Vec<SystemSample>,
    group_by: GroupBy,
    sort_by: SortBy,
    filter: FilterSpec,
    show_command: bool,
    limit: usize,
    include_idle: bool,
    normalize_cpu: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Viewport {
    width: u16,
    height: u16,
}

#[derive(Debug, Clone, Copy)]
enum TuiStyle {
    Header,
    Section,
    Label,
    Success,
    Warning,
    Error,
    Accent,
    Muted,
}

impl TuiApp {
    fn new(cli: &Cli, args: &LiveArgs) -> Self {
        Self {
            tick_count: 0,
            group_by: args.group.into(),
            sort_by: args.sort.into(),
            filter: args.filters.to_filter_spec(),
            search_query: String::new(),
            selected_row: 0,
            limit: args.effective_limit(),
            interval: args.interval,
            normalize_cpu: args.normalize_cpu,
            raw_bytes: cli.bytes,
            show_command: args.filters.show_command,
            columns: SnapshotColumns::default(),
            paused: false,
            overlay: Overlay::None,
            status_message: None,
            pending_export: None,
            last_report: None,
            record_duration: Duration::from_secs(30),
            recording_include_idle: false,
            recording: None,
            last_recording_report: None,
        }
    }

    fn overlay_open(&self) -> bool {
        !matches!(self.overlay, Overlay::None)
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(StatusMessage {
            text: message.into(),
            expires_at_tick: self.tick_count + STATUS_VISIBLE_TICKS,
        });
    }

    fn status_text(&self) -> Option<&str> {
        self.status_message
            .as_ref()
            .filter(|message| message.expires_at_tick >= self.tick_count)
            .map(|message| message.text.as_str())
    }

    fn sampler_config(&self) -> SamplerConfig {
        let recording_needs_command = self.recording.as_ref().is_some_and(|recording| {
            recording.show_command || recording.group_by == GroupBy::Command
        });
        let recording_needs_executable = self
            .recording
            .as_ref()
            .is_some_and(|recording| recording.group_by == GroupBy::Executable);
        SamplerConfig {
            include_command: self.show_command
                || self.group_by == GroupBy::Command
                || !self.filter.command_substrings.is_empty()
                || !self.filter.command_regexes.is_empty()
                || !self.search_query.is_empty()
                || recording_needs_command,
            include_executable: self.group_by == GroupBy::Executable
                || !self.search_query.is_empty()
                || recording_needs_executable,
        }
    }

    fn open_sort_picker(&mut self) {
        self.overlay = Overlay::Sort {
            selected: sort_index(self.sort_by),
        };
    }

    fn open_group_picker(&mut self) {
        self.overlay = Overlay::Group {
            selected: group_index(self.group_by),
        };
    }

    fn open_search(&mut self) {
        self.overlay = Overlay::Search {
            input: self.search_query.clone(),
        };
    }

    fn open_export_path(&mut self, target: ExportTarget, format: ExportFormat) {
        if target == ExportTarget::Recording && self.last_recording_report.is_none() {
            self.set_status("no recording report to export yet");
            self.close_overlay();
            return;
        }
        self.overlay = Overlay::ExportPath {
            target,
            format,
            input: default_export_path(self, target, format),
        };
    }

    fn open_detail(&mut self) {
        let Some(row) = self
            .last_report
            .as_ref()
            .and_then(|report| report.rows.get(self.selected_row))
            .cloned()
        else {
            self.set_status("no selected row");
            return;
        };
        self.overlay = Overlay::Detail { row: Box::new(row) };
    }

    fn close_overlay(&mut self) {
        self.overlay = Overlay::None;
    }

    fn set_report(&mut self, report: SnapshotReport) {
        self.selected_row = clamp_selected_row(self.selected_row, report.rows.len());
        self.last_report = Some(report);
    }

    fn selected_row_count(&self) -> usize {
        self.last_report
            .as_ref()
            .map(|report| report.rows.len())
            .unwrap_or(0)
    }

    fn move_selected_row(&mut self, direction: PickerDirection) {
        let row_count = self.selected_row_count();
        if row_count == 0 {
            self.selected_row = 0;
            return;
        }
        self.selected_row = match direction {
            PickerDirection::Previous => {
                if self.selected_row == 0 {
                    row_count - 1
                } else {
                    self.selected_row - 1
                }
            }
            PickerDirection::Next => (self.selected_row + 1) % row_count,
        };
    }

    fn move_selected_page(&mut self, direction: PickerDirection) {
        let row_count = self.selected_row_count();
        if row_count == 0 {
            self.selected_row = 0;
            return;
        }
        let page = 10;
        self.selected_row = match direction {
            PickerDirection::Previous => self.selected_row.saturating_sub(page),
            PickerDirection::Next => (self.selected_row + page).min(row_count - 1),
        };
    }

    fn increase_limit(&mut self) {
        if self.limit == usize::MAX {
            return;
        }
        self.limit = self.limit.saturating_add(5).min(500);
    }

    fn decrease_limit(&mut self) {
        if self.limit == usize::MAX {
            self.limit = 100;
            return;
        }
        self.limit = self.limit.saturating_sub(5).max(1);
        self.selected_row = clamp_selected_row(self.selected_row, self.limit);
    }

    fn faster_interval(&mut self) {
        let index = interval_index(self.interval);
        self.interval = INTERVAL_STEPS[index.saturating_sub(1)];
    }

    fn slower_interval(&mut self) {
        let index = interval_index(self.interval);
        self.interval = INTERVAL_STEPS[(index + 1).min(INTERVAL_STEPS.len() - 1)];
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.set_status(if self.paused { "paused" } else { "resumed" });
    }

    fn toggle_normalized_cpu(&mut self) {
        self.normalize_cpu = !self.normalize_cpu;
        self.set_status(format!("normalized CPU {}", on_off(self.normalize_cpu)));
    }

    fn toggle_raw_bytes(&mut self) {
        self.raw_bytes = !self.raw_bytes;
        self.set_status(format!("raw bytes {}", on_off(self.raw_bytes)));
    }

    fn toggle_show_command(&mut self) {
        self.show_command = !self.show_command;
        self.set_status(format!("command display {}", on_off(self.show_command)));
    }

    fn toggle_recording_include_idle(&mut self) {
        self.recording_include_idle = !self.recording_include_idle;
        self.set_status(format!(
            "recording include idle {}",
            on_off(self.recording_include_idle)
        ));
    }

    fn longer_record_duration(&mut self) {
        let index = record_duration_index(self.record_duration);
        self.record_duration = RECORD_DURATIONS[(index + 1).min(RECORD_DURATIONS.len() - 1)];
    }

    fn shorter_record_duration(&mut self) {
        let index = record_duration_index(self.record_duration);
        self.record_duration = RECORD_DURATIONS[index.saturating_sub(1)];
    }

    fn cycle_min_cpu(&mut self) {
        self.filter.min_cpu_percent =
            next_option_value(self.filter.min_cpu_percent, &CPU_THRESHOLDS);
    }

    fn cycle_min_ram(&mut self) {
        self.filter.min_ram_bytes = next_option_value(self.filter.min_ram_bytes, &RAM_THRESHOLDS);
    }

    fn cycle_min_io(&mut self) {
        self.filter.min_io_delta_bytes =
            next_option_value(self.filter.min_io_delta_bytes, &IO_THRESHOLDS);
    }

    fn clear_thresholds(&mut self) {
        self.filter.min_cpu_percent = None;
        self.filter.min_ram_bytes = None;
        self.filter.min_io_delta_bytes = None;
    }

    fn start_recording(&mut self) {
        if self.recording.is_some() {
            self.set_status("recording already running");
            return;
        }
        self.recording = Some(RecordingSession {
            started_at: Instant::now(),
            duration: self.record_duration,
            samples: Vec::new(),
            group_by: self.group_by,
            sort_by: self.sort_by,
            filter: self.filter.clone(),
            show_command: self.show_command,
            limit: self.limit,
            include_idle: self.recording_include_idle,
            normalize_cpu: self.normalize_cpu,
        });
        self.set_status(format!(
            "recording started for {}",
            humantime::format_duration(self.record_duration)
        ));
    }

    fn stop_recording(&mut self) {
        let Some(recording) = self.recording.take() else {
            self.set_status("no recording running");
            return;
        };
        self.finish_recording(recording);
    }

    fn capture_recording_sample(&mut self, sample: &SystemSample) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let is_duplicate = recording
            .samples
            .last()
            .is_some_and(|last| last.timestamp == sample.timestamp);
        if !is_duplicate {
            recording.samples.push(sample.clone());
        }
        if recording.started_at.elapsed() >= recording.duration
            && let Some(recording) = self.recording.take()
        {
            self.finish_recording(recording);
        }
    }

    fn finish_recording(&mut self, recording: RecordingSession) {
        if recording.samples.is_empty() {
            self.set_status("recording stopped without samples");
            return;
        }

        let elapsed = recording.started_at.elapsed().max(self.interval);
        let report = build_recording_report(
            &recording.samples,
            RecordingReportOptions {
                requested_duration: elapsed,
                interval: self.interval,
                group_by: recording.group_by,
                sort_by: recording.sort_by,
                filters: recording.filter,
                show_command: recording.show_command,
                limit: recording.limit,
                include_idle: recording.include_idle,
                normalize_cpu: recording.normalize_cpu,
            },
        );
        let sample_count = report.sample_count;
        self.last_recording_report = Some(report);
        self.set_status(format!("recording finished with {sample_count} samples"));
    }

    fn perform_pending_export(&mut self) {
        let Some(export) = self.pending_export.take() else {
            return;
        };
        if export.path.as_os_str().is_empty() || export.path == std::path::Path::new("-") {
            self.set_status("export path must be a file path");
            return;
        }
        if export.path.exists() {
            self.set_status(format!("export exists: {}", export.path.display()));
            return;
        }

        let result = match (export.target, export.format) {
            (ExportTarget::Snapshot, ExportFormat::Json) => {
                let Some(report) = &self.last_report else {
                    self.set_status("no snapshot to export yet");
                    return;
                };
                json::write_snapshot(export.path.as_path(), report)
            }
            (ExportTarget::Snapshot, ExportFormat::Csv) => {
                let Some(report) = &self.last_report else {
                    self.set_status("no snapshot to export yet");
                    return;
                };
                csv::write_snapshot(export.path.as_path(), report)
            }
            (ExportTarget::Recording, ExportFormat::Json) => {
                let Some(report) = &self.last_recording_report else {
                    self.set_status("no recording report to export yet");
                    return;
                };
                json::write_recording(export.path.as_path(), report)
            }
            (ExportTarget::Recording, ExportFormat::Csv) => {
                let Some(report) = &self.last_recording_report else {
                    self.set_status("no recording report to export yet");
                    return;
                };
                csv::write_recording(export.path.as_path(), report)
            }
        };
        self.set_status(match result {
            Ok(()) => format!("exported {}", export.path.display()),
            Err(error) => format!("export failed: {error}"),
        });
    }
}

pub fn run_live(cli: &Cli, args: &LiveArgs) -> Result<()> {
    let mut app = TuiApp::new(cli, args);
    let mut sampler_config = app.sampler_config();
    let mut sampler = SysinfoSampler::new(sampler_config)?;
    sampler.warm_up(args.interval)?;

    let _guard = enter_terminal()?;
    let mut cached_sample: Option<SystemSample> = None;

    loop {
        let desired_config = app.sampler_config();
        if desired_config != sampler_config {
            sampler = SysinfoSampler::new(desired_config)?;
            sampler.warm_up(app.interval)?;
            sampler_config = desired_config;
            cached_sample = None;
            app.set_status("sampler details updated");
        }

        if !app.paused || cached_sample.is_none() {
            cached_sample = Some(sampler.sample()?);
        }
        let sample = cached_sample
            .as_ref()
            .expect("cached sample is populated before rendering");
        let filtered = apply_tui_filters(sample, &app);
        let report = build_snapshot_report(
            &filtered,
            SnapshotReportOptions {
                interval: app.interval,
                group_by: app.group_by,
                sort_by: app.sort_by,
                filters: app.filter.clone(),
                show_command: app.show_command,
                limit: app.limit,
                normalize_cpu: app.normalize_cpu,
            },
        );
        app.capture_recording_sample(&filtered);
        app.tick_count += 1;
        app.set_report(report.clone());
        app.perform_pending_export();

        execute!(
            io::stdout(),
            Clear(ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        write_tui_text(&render_app(
            &app,
            &report,
            cli.color_enabled(),
            current_viewport(),
        ))?;
        io::stdout().flush()?;

        let next_tick = Instant::now() + app.interval;
        match wait_for_input_until(next_tick, &mut app)? {
            TuiInput::Quit => break,
            TuiInput::Tick | TuiInput::RefreshNow => {}
        }
    }

    Ok(())
}

fn enter_terminal() -> Result<TerminalGuard> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(TerminalGuard)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiInput {
    Tick,
    RefreshNow,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerDirection {
    Previous,
    Next,
}

fn wait_for_input_until(deadline: Instant, app: &mut TuiApp) -> Result<TuiInput> {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Ok(TuiInput::Tick);
        }
        let timeout = (deadline - now).min(Duration::from_millis(100));
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            return Ok(handle_key(app, key.code, key.modifiers));
        }
    }
}

fn handle_key(app: &mut TuiApp, code: KeyCode, modifiers: KeyModifiers) -> TuiInput {
    let ctrl_c = code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL);
    if ctrl_c || is_quit_key(code) {
        return TuiInput::Quit;
    }

    match app.overlay.clone() {
        Overlay::None => handle_main_key(app, code, modifiers),
        Overlay::Search { input } => handle_search_key(app, code, modifiers, input),
        Overlay::ExportPath {
            target,
            format,
            input,
        } => handle_export_path_key(app, code, modifiers, target, format, input),
        Overlay::Help | Overlay::Detail { .. } => handle_simple_overlay_key(app, code),
        Overlay::Options { .. }
        | Overlay::Sort { .. }
        | Overlay::Group { .. }
        | Overlay::Filter { .. }
        | Overlay::View { .. }
        | Overlay::Sampling { .. }
        | Overlay::Recording { .. }
        | Overlay::Export { .. } => handle_menu_key(app, code),
    }
}

fn handle_main_key(app: &mut TuiApp, code: KeyCode, modifiers: KeyModifiers) -> TuiInput {
    if code == KeyCode::Esc {
        return TuiInput::Quit;
    }
    if !plain_or_shift(modifiers) {
        return TuiInput::Tick;
    }

    match code {
        KeyCode::Char('?') => app.overlay = Overlay::Help,
        KeyCode::Char('o') | KeyCode::Char('O') => app.overlay = Overlay::Options { selected: 0 },
        KeyCode::Char('s') => app.open_sort_picker(),
        KeyCode::Char('g') | KeyCode::Char('G') => app.open_group_picker(),
        KeyCode::Char('f') | KeyCode::Char('F') => app.overlay = Overlay::Filter { selected: 0 },
        KeyCode::Char('v') | KeyCode::Char('V') => app.overlay = Overlay::View { selected: 0 },
        KeyCode::Char('e') | KeyCode::Char('E') => app.overlay = Overlay::Export { selected: 0 },
        KeyCode::Char('r') | KeyCode::Char('R') => app.overlay = Overlay::Recording { selected: 0 },
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char(' ') => app.toggle_pause(),
        KeyCode::Char('+') | KeyCode::Char('=') => app.increase_limit(),
        KeyCode::Char('-') => app.decrease_limit(),
        KeyCode::Char('[') => app.faster_interval(),
        KeyCode::Char(']') => app.slower_interval(),
        KeyCode::Char('n') | KeyCode::Char('N') => app.toggle_normalized_cpu(),
        KeyCode::Char('b') | KeyCode::Char('B') => app.toggle_raw_bytes(),
        KeyCode::Char('c') | KeyCode::Char('C') => app.toggle_show_command(),
        KeyCode::Up => app.move_selected_row(PickerDirection::Previous),
        KeyCode::Down => app.move_selected_row(PickerDirection::Next),
        KeyCode::PageUp => app.move_selected_page(PickerDirection::Previous),
        KeyCode::PageDown => app.move_selected_page(PickerDirection::Next),
        KeyCode::Enter => {
            if app.selected_row_count() > 0 {
                app.open_detail();
            }
        }
        _ => return TuiInput::Tick,
    }
    TuiInput::RefreshNow
}

fn handle_export_path_key(
    app: &mut TuiApp,
    code: KeyCode,
    modifiers: KeyModifiers,
    target: ExportTarget,
    format: ExportFormat,
    mut input: String,
) -> TuiInput {
    match code {
        KeyCode::Esc => app.close_overlay(),
        KeyCode::Enter => {
            app.pending_export = Some(PendingExport {
                target,
                format,
                path: PathBuf::from(input.trim()),
            });
            app.close_overlay();
        }
        KeyCode::Backspace => {
            input.pop();
            app.overlay = Overlay::ExportPath {
                target,
                format,
                input,
            };
        }
        KeyCode::Char(ch) if plain_or_shift(modifiers) => {
            input.push(ch);
            app.overlay = Overlay::ExportPath {
                target,
                format,
                input,
            };
        }
        _ => return TuiInput::Tick,
    }
    TuiInput::RefreshNow
}

fn handle_search_key(
    app: &mut TuiApp,
    code: KeyCode,
    modifiers: KeyModifiers,
    mut input: String,
) -> TuiInput {
    match code {
        KeyCode::Esc => {
            app.close_overlay();
        }
        KeyCode::Enter => {
            app.search_query = input.trim().to_string();
            app.selected_row = 0;
            app.close_overlay();
        }
        KeyCode::Backspace => {
            input.pop();
            app.overlay = Overlay::Search { input };
        }
        KeyCode::Char(ch) if plain_or_shift(modifiers) => {
            input.push(ch);
            app.overlay = Overlay::Search { input };
        }
        _ => return TuiInput::Tick,
    }
    TuiInput::RefreshNow
}

fn handle_simple_overlay_key(app: &mut TuiApp, code: KeyCode) -> TuiInput {
    match code {
        KeyCode::Esc | KeyCode::Enter => {
            app.close_overlay();
            TuiInput::RefreshNow
        }
        _ => TuiInput::Tick,
    }
}

fn handle_menu_key(app: &mut TuiApp, code: KeyCode) -> TuiInput {
    if code == KeyCode::Esc {
        app.close_overlay();
        return TuiInput::RefreshNow;
    }

    match code {
        KeyCode::Up => {
            move_overlay_selection(app, PickerDirection::Previous);
            TuiInput::RefreshNow
        }
        KeyCode::Down => {
            move_overlay_selection(app, PickerDirection::Next);
            TuiInput::RefreshNow
        }
        KeyCode::Enter => {
            apply_overlay_selection(app);
            TuiInput::RefreshNow
        }
        _ => TuiInput::Tick,
    }
}

fn move_overlay_selection(app: &mut TuiApp, direction: PickerDirection) {
    match &mut app.overlay {
        Overlay::Options { selected } => move_index(selected, OPTIONS_ITEMS.len(), direction),
        Overlay::Sort { selected } => move_index(selected, view::SORT_OPTIONS.len(), direction),
        Overlay::Group { selected } => move_index(selected, GROUP_OPTIONS.len(), direction),
        Overlay::Filter { selected } => move_index(selected, FILTER_ITEMS.len(), direction),
        Overlay::View { selected } => move_index(selected, VIEW_ITEMS.len(), direction),
        Overlay::Sampling { selected } => move_index(selected, SAMPLING_ITEMS.len(), direction),
        Overlay::Recording { selected } => move_index(selected, RECORDING_ITEMS.len(), direction),
        Overlay::Export { selected } => move_index(selected, EXPORT_ITEMS.len(), direction),
        Overlay::None
        | Overlay::Help
        | Overlay::Detail { .. }
        | Overlay::Search { .. }
        | Overlay::ExportPath { .. } => {}
    }
}

fn apply_overlay_selection(app: &mut TuiApp) {
    match app.overlay.clone() {
        Overlay::Options { selected } => match selected {
            0 => app.open_sort_picker(),
            1 => app.open_group_picker(),
            2 => app.overlay = Overlay::Filter { selected: 0 },
            3 => app.overlay = Overlay::View { selected: 0 },
            4 => app.overlay = Overlay::Sampling { selected: 0 },
            5 => app.overlay = Overlay::Recording { selected: 0 },
            6 => app.overlay = Overlay::Export { selected: 0 },
            7 => app.open_detail(),
            8 => app.overlay = Overlay::Help,
            _ => app.close_overlay(),
        },
        Overlay::Sort { selected } => {
            app.sort_by = view::SORT_OPTIONS[selected];
            app.close_overlay();
        }
        Overlay::Group { selected } => {
            app.group_by = GROUP_OPTIONS[selected];
            app.selected_row = 0;
            app.close_overlay();
        }
        Overlay::Filter { selected } => match selected {
            0 => app.open_search(),
            1 => {
                app.search_query.clear();
                app.selected_row = 0;
            }
            2 => app.filter.invert_match = !app.filter.invert_match,
            3 => app.filter.hide_self = !app.filter.hide_self,
            4 => app.cycle_min_cpu(),
            5 => app.cycle_min_ram(),
            6 => app.cycle_min_io(),
            7 => app.clear_thresholds(),
            _ => {}
        },
        Overlay::View { selected } => match selected {
            0 => app.toggle_normalized_cpu(),
            1 => app.toggle_raw_bytes(),
            2 => app.toggle_show_command(),
            3 => app.columns.pid = !app.columns.pid,
            4 => {
                app.columns.user = !app.columns.user;
                app.columns.users = !app.columns.users;
            }
            5 => app.columns.rates = !app.columns.rates,
            6 => app.columns.totals = !app.columns.totals,
            7 => app.columns.top_process = !app.columns.top_process,
            _ => {}
        },
        Overlay::Sampling { selected } => match selected {
            0 => app.increase_limit(),
            1 => app.decrease_limit(),
            2 => app.faster_interval(),
            3 => app.slower_interval(),
            4 => app.toggle_pause(),
            _ => {}
        },
        Overlay::Recording { selected } => match selected {
            0 => app.start_recording(),
            1 => app.stop_recording(),
            2 => app.longer_record_duration(),
            3 => app.shorter_record_duration(),
            4 => app.toggle_recording_include_idle(),
            5 => app.open_export_path(ExportTarget::Recording, ExportFormat::Json),
            6 => app.open_export_path(ExportTarget::Recording, ExportFormat::Csv),
            _ => {}
        },
        Overlay::Export { selected } => match selected {
            0 => app.open_export_path(ExportTarget::Snapshot, ExportFormat::Json),
            1 => app.open_export_path(ExportTarget::Snapshot, ExportFormat::Csv),
            2 => app.open_export_path(ExportTarget::Recording, ExportFormat::Json),
            3 => app.open_export_path(ExportTarget::Recording, ExportFormat::Csv),
            _ => app.close_overlay(),
        },
        Overlay::None
        | Overlay::Help
        | Overlay::Detail { .. }
        | Overlay::Search { .. }
        | Overlay::ExportPath { .. } => {}
    }
}

fn move_index(selected: &mut usize, len: usize, direction: PickerDirection) {
    *selected = match direction {
        PickerDirection::Previous => {
            if *selected == 0 {
                len - 1
            } else {
                *selected - 1
            }
        }
        PickerDirection::Next => (*selected + 1) % len,
    };
}

fn render_app(app: &TuiApp, report: &SnapshotReport, color: bool, viewport: Viewport) -> String {
    let mut output = String::new();
    let overlay = format_overlay(app, color);
    let footer = format_footer(app, color);
    let max_rows = table_max_rows(app, &overlay, &footer, viewport);
    let row_offset = row_offset_for_selection(app.selected_row, report.rows.len(), max_rows);
    let columns = columns_for_viewport(app, viewport);

    output.push_str(&paint(
        &view::format_header(report, app.raw_bytes, app.tick_count),
        TuiStyle::Header,
        color,
    ));
    output.push_str(&format_state_line(app, report, color));
    output.push_str(&table::render_snapshot_with_options(
        report,
        app.raw_bytes,
        color,
        SnapshotRenderOptions {
            show_system: false,
            selected_row: Some(app.selected_row),
            row_offset,
            max_rows: Some(max_rows),
            columns,
        },
    ));
    output.push_str(&overlay);
    output.push_str(&footer);
    output
}

fn format_state_line(app: &TuiApp, report: &SnapshotReport, color: bool) -> String {
    let mut output = String::new();
    let search = if app.search_query.is_empty() {
        "none"
    } else {
        app.search_query.as_str()
    };
    let selected = if report.rows.is_empty() {
        "0/0".to_string()
    } else {
        format!("{}/{}", app.selected_row + 1, report.rows.len())
    };
    let status = if app.paused { "paused" } else { "live" };
    let status = if app.paused {
        paint(status, TuiStyle::Warning, color)
    } else {
        paint(status, TuiStyle::Success, color)
    };
    let recording = app
        .recording
        .as_ref()
        .map(|recording| {
            paint(
                &format!(
                    "recording {}/{}",
                    humantime::format_duration(recording.started_at.elapsed()),
                    humantime::format_duration(recording.duration)
                ),
                TuiStyle::Accent,
                color,
            )
        })
        .unwrap_or_else(|| paint("recording off", TuiStyle::Muted, color));
    writeln!(
        &mut output,
        "{} {status} | {} {} | {} {} | {} {} | {} {selected} | {} {search} | {recording}",
        label("mode", color),
        label("group", color),
        group_label(app.group_by),
        label("sort", color),
        view::sort_label(app.sort_by),
        label("limit", color),
        limit_label(app.limit),
        label("row", color),
        label("search", color),
    )
    .expect("writing to a string cannot fail");
    writeln!(
        &mut output,
        "{} {} | {} {} | {} {} | {} {}",
        label("normalized", color),
        on_off(app.normalize_cpu),
        label("bytes", color),
        on_off(app.raw_bytes),
        label("command", color),
        on_off(app.show_command),
        label("filters", color),
        filter_summary(app)
    )
    .expect("writing to a string cannot fail");
    if let Some(message) = app.status_text() {
        writeln!(
            &mut output,
            "{} {}",
            label("status", color),
            paint(message, status_message_style(message), color)
        )
        .expect("writing to a string cannot fail");
    }
    output.push('\n');
    output
}

fn format_overlay(app: &TuiApp, color: bool) -> String {
    match &app.overlay {
        Overlay::None => String::new(),
        Overlay::Help => format_help(color),
        Overlay::Options { selected } => {
            format_menu("Options", OPTIONS_ITEMS, *selected, None, color)
        }
        Overlay::Sort { selected } => format_sort_menu(*selected, app.sort_by, color),
        Overlay::Group { selected } => format_group_menu(*selected, app.group_by, color),
        Overlay::Filter { selected } => format_filter_menu(*selected, app, color),
        Overlay::View { selected } => format_view_menu(*selected, app, color),
        Overlay::Sampling { selected } => format_sampling_menu(*selected, app, color),
        Overlay::Recording { selected } => format_recording_menu(*selected, app, color),
        Overlay::Export { selected } => format_export_menu(*selected, app, color),
        Overlay::ExportPath {
            target,
            format,
            input,
        } => format_export_path(*target, *format, input, color),
        Overlay::Detail { row } => format_detail(app, row, color),
        Overlay::Search { input } => format!(
            "{}\n{} {input}\n\nEnter apply | Esc cancel\n\n",
            section_title("Search", color),
            paint(">", TuiStyle::Success, color)
        ),
    }
}

fn format_menu(
    title: &str,
    items: impl IntoIterator<Item = &'static str>,
    selected: usize,
    suffixes: Option<Vec<String>>,
    color: bool,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "{}", section_title(title, color))
        .expect("writing to a string cannot fail");
    for (index, item) in items.into_iter().enumerate() {
        let marker = if index == selected {
            paint(">", TuiStyle::Success, color)
        } else {
            " ".to_string()
        };
        let suffix = suffixes
            .as_ref()
            .and_then(|values| values.get(index))
            .map(|value| format!(" {}", paint(value, TuiStyle::Muted, color)))
            .unwrap_or_default();
        writeln!(&mut output, "{marker} {item}{suffix}").expect("writing to a string cannot fail");
    }
    output.push('\n');
    output
}

fn format_sort_menu(selected: usize, current_sort: SortBy, color: bool) -> String {
    let suffixes = view::SORT_OPTIONS
        .iter()
        .map(|sort_by| {
            if *sort_by == current_sort {
                "current".to_string()
            } else {
                String::new()
            }
        })
        .collect::<Vec<_>>();
    format_menu(
        "Sort by",
        view::SORT_OPTIONS
            .iter()
            .map(|sort_by| view::sort_label(*sort_by)),
        selected,
        Some(suffixes),
        color,
    )
}

fn format_group_menu(selected: usize, current_group: GroupBy, color: bool) -> String {
    let suffixes = GROUP_OPTIONS
        .iter()
        .map(|group_by| {
            if *group_by == current_group {
                "current".to_string()
            } else {
                String::new()
            }
        })
        .collect::<Vec<_>>();
    format_menu(
        "Group by",
        GROUP_OPTIONS.iter().map(|group_by| group_label(*group_by)),
        selected,
        Some(suffixes),
        color,
    )
}

fn format_filter_menu(selected: usize, app: &TuiApp, color: bool) -> String {
    let suffixes = vec![
        if app.search_query.is_empty() {
            "none".to_string()
        } else {
            app.search_query.clone()
        },
        String::new(),
        on_off(app.filter.invert_match).to_string(),
        on_off(app.filter.hide_self).to_string(),
        threshold_percent_label(app.filter.min_cpu_percent),
        threshold_bytes_label(app.filter.min_ram_bytes, app.raw_bytes),
        threshold_bytes_label(app.filter.min_io_delta_bytes, app.raw_bytes),
        String::new(),
    ];
    format_menu("Filters", FILTER_ITEMS, selected, Some(suffixes), color)
}

fn format_view_menu(selected: usize, app: &TuiApp, color: bool) -> String {
    let suffixes = vec![
        on_off(app.normalize_cpu).to_string(),
        on_off(app.raw_bytes).to_string(),
        on_off(app.show_command).to_string(),
        on_off(app.columns.pid).to_string(),
        if app.columns.user || app.columns.users {
            "on"
        } else {
            "off"
        }
        .to_string(),
        on_off(app.columns.rates).to_string(),
        on_off(app.columns.totals).to_string(),
        on_off(app.columns.top_process).to_string(),
    ];
    format_menu("View", VIEW_ITEMS, selected, Some(suffixes), color)
}

fn format_sampling_menu(selected: usize, app: &TuiApp, color: bool) -> String {
    let suffixes = vec![
        limit_label(app.limit),
        limit_label(app.limit),
        humantime::format_duration(app.interval).to_string(),
        humantime::format_duration(app.interval).to_string(),
        if app.paused { "paused" } else { "live" }.to_string(),
    ];
    format_menu("Sampling", SAMPLING_ITEMS, selected, Some(suffixes), color)
}

fn format_recording_menu(selected: usize, app: &TuiApp, color: bool) -> String {
    let active = app.recording.is_some();
    let suffixes = vec![
        humantime::format_duration(app.record_duration).to_string(),
        if active { "active" } else { "inactive" }.to_string(),
        humantime::format_duration(app.record_duration).to_string(),
        humantime::format_duration(app.record_duration).to_string(),
        on_off(app.recording_include_idle).to_string(),
        if app.last_recording_report.is_some() {
            "ready".to_string()
        } else {
            "none".to_string()
        },
        if app.last_recording_report.is_some() {
            "ready".to_string()
        } else {
            "none".to_string()
        },
    ];
    format_menu(
        "Recording",
        RECORDING_ITEMS,
        selected,
        Some(suffixes),
        color,
    )
}

fn format_export_menu(selected: usize, app: &TuiApp, color: bool) -> String {
    let suffixes = vec![
        "ready".to_string(),
        "ready".to_string(),
        if app.last_recording_report.is_some() {
            "ready".to_string()
        } else {
            "none".to_string()
        },
        if app.last_recording_report.is_some() {
            "ready".to_string()
        } else {
            "none".to_string()
        },
    ];
    format_menu("Export", EXPORT_ITEMS, selected, Some(suffixes), color)
}

fn format_export_path(
    target: ExportTarget,
    format: ExportFormat,
    input: &str,
    color: bool,
) -> String {
    format!(
        "{} {} {}\n{} {input}\n\nEnter export | Backspace edit | Esc cancel\n\n",
        section_title("Export", color),
        export_target_label(target),
        export_format_label(format),
        paint(">", TuiStyle::Success, color)
    )
}

fn format_help(color: bool) -> String {
    let lines = [
        section_title("Help", color),
        "o options menu".to_string(),
        "s sort menu | g group menu | f filters | v view | r recording | e export".to_string(),
        "/ search | up/down select row | PgUp/PgDn page | Enter details".to_string(),
        "space pause/resume | +/- row limit | [/] refresh interval".to_string(),
        "n normalized CPU | b raw bytes | c command display".to_string(),
        "Esc close overlay or quit main view | q quit".to_string(),
        String::new(),
    ];
    lines.join("\n")
}

fn format_detail(app: &TuiApp, row: &SnapshotRow, color: bool) -> String {
    let mut output = String::new();
    writeln!(&mut output, "{}", section_title("Details", color))
        .expect("writing to a string cannot fail");
    writeln!(&mut output, "{} {}", label("name", color), row.display_name)
        .expect("writing to a string cannot fail");
    if let Some(pid) = row.pid {
        writeln!(&mut output, "{} {pid}", label("pid", color))
            .expect("writing to a string cannot fail");
    }
    writeln!(
        &mut output,
        "{} {}",
        label("group", color),
        group_label(row.group_type)
    )
    .expect("writing to a string cannot fail");
    writeln!(
        &mut output,
        "{} {}",
        label("user", color),
        row.user_name
            .as_deref()
            .or(row.users.as_deref())
            .unwrap_or("unknown")
    )
    .expect("writing to a string cannot fail");
    writeln!(
        &mut output,
        "{} {:.1}% | {} {} | {} {} | {} {} | {} {}",
        label("cpu", color),
        row.cpu_percent,
        label("ram", color),
        format_bytes(row.ram_bytes, app.raw_bytes),
        label("read", color),
        format_bps(row.read_bps, app.raw_bytes),
        label("write", color),
        format_bps(row.write_bps, app.raw_bytes),
        label("io", color),
        format_bps(row.io_bps, app.raw_bytes)
    )
    .expect("writing to a string cannot fail");
    if let Some(process) = &row.top_process {
        writeln!(&mut output, "{} {process}", label("top process", color))
            .expect("writing to a string cannot fail");
    }
    writeln!(
        &mut output,
        "\n{}\n",
        paint("Esc close | q quit", TuiStyle::Muted, color)
    )
    .expect("writing to a string cannot fail");
    output
}

fn format_footer(app: &TuiApp, color: bool) -> String {
    if app.overlay_open() {
        format!(
            "\n{}\n",
            paint(
                "up/down choose | Enter apply | Esc back | q quit",
                TuiStyle::Muted,
                color
            )
        )
    } else {
        format!(
            "\n{}\n",
            paint(
                "o options | ? help | / search | Enter details | s sort | r record | q quit",
                TuiStyle::Muted,
                color
            )
        )
    }
}

fn apply_tui_filters(sample: &SystemSample, app: &TuiApp) -> SystemSample {
    let mut filtered = filter_sample(sample, &app.filter);
    let query = app.search_query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return filtered;
    }

    filtered
        .processes
        .retain(|process| process_matches_query(process, &query));
    filtered
}

fn process_matches_query(process: &RawProcessSample, query: &str) -> bool {
    process.identity.pid.to_string().contains(query)
        || process.identity.name.to_ascii_lowercase().contains(query)
        || process.user_display().to_ascii_lowercase().contains(query)
        || process
            .command
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(query)
        || process
            .executable
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(query)
}

fn current_viewport() -> Viewport {
    terminal_size()
        .map(|(width, height)| Viewport { width, height })
        .unwrap_or(DEFAULT_VIEWPORT)
}

fn table_max_rows(app: &TuiApp, overlay: &str, footer: &str, viewport: Viewport) -> usize {
    let header = app
        .last_report
        .as_ref()
        .map(|report| view::format_header(report, app.raw_bytes, app.tick_count))
        .unwrap_or_default();
    let state = app
        .last_report
        .as_ref()
        .map(|report| format_state_line(app, report, false))
        .unwrap_or_default();
    let reserved_lines =
        line_count(&header) + line_count(&state) + line_count(overlay) + line_count(footer) + 2;
    (viewport.height as usize)
        .saturating_sub(reserved_lines)
        .saturating_sub(1)
        .max(3)
}

fn row_offset_for_selection(selected: usize, row_count: usize, max_rows: usize) -> usize {
    if row_count <= max_rows || selected < max_rows {
        0
    } else {
        (selected + 1).saturating_sub(max_rows)
    }
}

fn columns_for_viewport(app: &TuiApp, viewport: Viewport) -> SnapshotColumns {
    let mut columns = app.columns;
    if viewport.width < 110 {
        columns.totals = false;
        columns.top_process = false;
    }
    if viewport.width < 90 {
        columns.rates = false;
    }
    if viewport.width < 76 {
        columns.user = false;
        columns.users = false;
    }
    if viewport.width < 64 {
        columns.pid = false;
        columns.process_count = false;
    }
    columns
}

fn line_count(value: &str) -> usize {
    value.lines().count()
}

fn is_quit_key(code: KeyCode) -> bool {
    matches!(code, KeyCode::Char('q') | KeyCode::Char('Q'))
}

fn plain_or_shift(modifiers: KeyModifiers) -> bool {
    modifiers.is_empty() || modifiers == KeyModifiers::SHIFT
}

fn sort_index(sort_by: SortBy) -> usize {
    view::SORT_OPTIONS
        .iter()
        .position(|option| *option == sort_by)
        .unwrap_or(0)
}

fn group_index(group_by: GroupBy) -> usize {
    GROUP_OPTIONS
        .iter()
        .position(|option| *option == group_by)
        .unwrap_or(0)
}

fn interval_index(interval: Duration) -> usize {
    INTERVAL_STEPS
        .iter()
        .enumerate()
        .min_by_key(|(_, option)| option.abs_diff(interval))
        .map(|(index, _)| index)
        .unwrap_or(2)
}

fn record_duration_index(duration: Duration) -> usize {
    RECORD_DURATIONS
        .iter()
        .enumerate()
        .min_by_key(|(_, option)| option.abs_diff(duration))
        .map(|(index, _)| index)
        .unwrap_or(1)
}

fn next_option_value<T: Copy + PartialEq>(current: Option<T>, options: &[Option<T>]) -> Option<T> {
    let index = options
        .iter()
        .position(|option| *option == current)
        .unwrap_or(0);
    options[(index + 1) % options.len()]
}

fn clamp_selected_row(selected_row: usize, row_count: usize) -> usize {
    if row_count == 0 {
        0
    } else {
        selected_row.min(row_count - 1)
    }
}

fn limit_label(limit: usize) -> String {
    if limit == usize::MAX {
        "all".to_string()
    } else {
        limit.to_string()
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn paint(value: &str, style: TuiStyle, enabled: bool) -> String {
    if !enabled || value.is_empty() {
        return value.to_string();
    }
    format!("{}{}{}", ansi_code(style), value, "\x1b[0m")
}

fn ansi_code(style: TuiStyle) -> &'static str {
    match style {
        TuiStyle::Header => "\x1b[1;36m",
        TuiStyle::Section => "\x1b[1;34m",
        TuiStyle::Label => "\x1b[36m",
        TuiStyle::Success => "\x1b[32m",
        TuiStyle::Warning => "\x1b[33m",
        TuiStyle::Error => "\x1b[31m",
        TuiStyle::Accent => "\x1b[35m",
        TuiStyle::Muted => "\x1b[2m",
    }
}

fn label(value: &str, color: bool) -> String {
    paint(value, TuiStyle::Label, color)
}

fn section_title(value: &str, color: bool) -> String {
    paint(value, TuiStyle::Section, color)
}

fn status_message_style(message: &str) -> TuiStyle {
    let message = message.to_ascii_lowercase();
    if message.contains("failed")
        || message.contains("no ")
        || message.contains("exists")
        || message.contains("must ")
    {
        TuiStyle::Error
    } else if message.contains("paused") || message.contains("updated") {
        TuiStyle::Warning
    } else {
        TuiStyle::Success
    }
}

fn threshold_percent_label(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "none".to_string())
}

fn threshold_bytes_label(value: Option<u64>, raw_bytes: bool) -> String {
    value
        .map(|value| format_bytes(value, raw_bytes))
        .unwrap_or_else(|| "none".to_string())
}

fn filter_summary(app: &TuiApp) -> String {
    let mut active = Vec::new();
    if !app.filter.pids.is_empty() {
        active.push("pid");
    }
    if !app.filter.users.is_empty() {
        active.push("user");
    }
    if !app.filter.names.is_empty() || !app.filter.name_regexes.is_empty() {
        active.push("name");
    }
    if !app.filter.command_substrings.is_empty() || !app.filter.command_regexes.is_empty() {
        active.push("cmd");
    }
    if app.filter.min_cpu_percent.is_some()
        || app.filter.min_ram_bytes.is_some()
        || app.filter.min_io_delta_bytes.is_some()
    {
        active.push("threshold");
    }
    if app.filter.hide_self {
        active.push("hide-self");
    }
    if app.filter.invert_match {
        active.push("invert");
    }
    if active.is_empty() {
        "none".to_string()
    } else {
        active.join(",")
    }
}

fn default_export_path(app: &TuiApp, target: ExportTarget, format: ExportFormat) -> String {
    let timestamp = match target {
        ExportTarget::Snapshot => app
            .last_report
            .as_ref()
            .map(|report| system_time_ms(report.ended_at)),
        ExportTarget::Recording => app
            .last_recording_report
            .as_ref()
            .map(|report| system_time_ms(report.ended_at)),
    }
    .unwrap_or_else(|| system_time_ms(std::time::SystemTime::now()));

    format!(
        "rescope-{}-{timestamp}.{}",
        export_target_label(target),
        export_extension(format)
    )
}

fn export_target_label(target: ExportTarget) -> &'static str {
    match target {
        ExportTarget::Snapshot => "snapshot",
        ExportTarget::Recording => "recording",
    }
}

fn export_format_label(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Json => "JSON",
        ExportFormat::Csv => "CSV",
    }
}

fn export_extension(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Json => "json",
        ExportFormat::Csv => "csv",
    }
}

fn group_label(group_by: GroupBy) -> &'static str {
    match group_by {
        GroupBy::Process => "process",
        GroupBy::Name => "name",
        GroupBy::User => "user",
        GroupBy::Command => "command",
        GroupBy::Executable => "executable",
        GroupBy::Parent => "parent",
    }
}

fn write_tui_text(output: &str) -> io::Result<()> {
    let output = output.replace('\n', "\r\n");
    io::stdout().write_all(output.as_bytes())
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> TuiApp {
        TuiApp {
            tick_count: 0,
            group_by: GroupBy::Process,
            sort_by: SortBy::Cpu,
            filter: FilterSpec::default(),
            search_query: String::new(),
            selected_row: 0,
            limit: 20,
            interval: Duration::from_secs(1),
            normalize_cpu: false,
            raw_bytes: false,
            show_command: false,
            columns: SnapshotColumns::default(),
            paused: false,
            overlay: Overlay::None,
            status_message: None,
            pending_export: None,
            last_report: None,
            record_duration: Duration::from_secs(30),
            recording_include_idle: false,
            recording: None,
            last_recording_report: None,
        }
    }

    #[test]
    fn opens_sort_picker_with_s_only() {
        let mut app = app();
        assert_eq!(
            handle_key(&mut app, KeyCode::Char('c'), KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert!(matches!(app.overlay, Overlay::None));
        assert!(app.show_command);

        assert_eq!(
            handle_key(&mut app, KeyCode::Char('s'), KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert!(matches!(app.overlay, Overlay::Sort { selected: 0 }));

        app.close_overlay();
        assert_eq!(
            handle_key(&mut app, KeyCode::Char('S'), KeyModifiers::SHIFT),
            TuiInput::Tick
        );
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn sort_menu_moves_and_applies_selection() {
        let mut app = app();
        app.open_sort_picker();

        assert_eq!(
            handle_key(&mut app, KeyCode::Down, KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert!(matches!(app.overlay, Overlay::Sort { selected: 1 }));

        assert_eq!(
            handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert_eq!(app.sort_by, SortBy::Ram);
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn group_menu_updates_grouping() {
        let mut app = app();
        app.open_group_picker();
        handle_key(&mut app, KeyCode::Down, KeyModifiers::empty());
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(app.group_by, GroupBy::Name);
    }

    #[test]
    fn search_overlay_applies_query() {
        let mut app = app();
        app.open_search();
        handle_key(&mut app, KeyCode::Char('n'), KeyModifiers::empty());
        handle_key(&mut app, KeyCode::Char('o'), KeyModifiers::empty());
        handle_key(&mut app, KeyCode::Char('d'), KeyModifiers::empty());
        handle_key(&mut app, KeyCode::Char('e'), KeyModifiers::empty());
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(app.search_query, "node");
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn escape_quits_main_but_closes_overlay() {
        let mut app = app();
        assert_eq!(
            handle_key(&mut app, KeyCode::Esc, KeyModifiers::empty()),
            TuiInput::Quit
        );

        app.open_sort_picker();
        assert_eq!(
            handle_key(&mut app, KeyCode::Esc, KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert!(matches!(app.overlay, Overlay::None));
    }

    #[test]
    fn filter_menu_cycles_thresholds_and_invert() {
        let mut app = app();
        app.overlay = Overlay::Filter { selected: 2 };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert!(app.filter.invert_match);

        app.overlay = Overlay::Filter { selected: 4 };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(app.filter.min_cpu_percent, Some(1.0));

        app.overlay = Overlay::Filter { selected: 7 };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(app.filter.min_cpu_percent, None);
    }

    #[test]
    fn view_menu_toggles_columns() {
        let mut app = app();
        assert!(app.columns.pid);
        app.overlay = Overlay::View { selected: 3 };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert!(!app.columns.pid);
    }

    #[test]
    fn export_menu_prompts_for_snapshot_path() {
        let mut app = app();
        app.overlay = Overlay::Export { selected: 0 };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert!(matches!(
            app.overlay,
            Overlay::ExportPath {
                target: ExportTarget::Snapshot,
                format: ExportFormat::Json,
                ..
            }
        ));
    }

    #[test]
    fn recording_menu_starts_and_stops_session() {
        let mut app = app();
        app.overlay = Overlay::Recording { selected: 0 };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert!(app.recording.is_some());

        app.overlay = Overlay::Recording { selected: 1 };
        handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty());
        assert!(app.recording.is_none());
    }

    #[test]
    fn row_offset_keeps_selected_row_visible() {
        assert_eq!(row_offset_for_selection(0, 20, 5), 0);
        assert_eq!(row_offset_for_selection(9, 20, 5), 5);
    }

    #[test]
    fn narrow_viewport_hides_optional_columns() {
        let app = app();
        let columns = columns_for_viewport(
            &app,
            Viewport {
                width: 70,
                height: 20,
            },
        );
        assert!(!columns.user);
        assert!(!columns.rates);
    }

    #[test]
    fn detail_overlay_keeps_entered_row_snapshot() {
        let mut app = app();
        app.last_report = Some(report_with_processes(&["alpha", "beta"]));
        app.selected_row = 1;

        assert_eq!(
            handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        app.last_report = Some(report_with_processes(&["gamma", "delta"]));

        let detail = format_overlay(&app, false);
        assert!(detail.contains("beta"));
        assert!(!detail.contains("delta"));
    }

    #[test]
    fn render_app_uses_ansi_colors_when_enabled() {
        let mut app = app();
        let report = report_with_processes(&["alpha"]);
        app.last_report = Some(report.clone());

        let colored = render_app(&app, &report, true, DEFAULT_VIEWPORT);
        let plain = render_app(&app, &report, false, DEFAULT_VIEWPORT);

        assert!(colored.contains("\x1b["));
        assert!(!plain.contains("\x1b["));
    }

    fn report_with_processes(names: &[&str]) -> SnapshotReport {
        let sample = SystemSample {
            timestamp: std::time::SystemTime::UNIX_EPOCH,
            total_memory_bytes: 1024,
            available_memory_bytes: 512,
            global_cpu_percent: 0.0,
            processes: names
                .iter()
                .enumerate()
                .map(|(index, name)| RawProcessSample {
                    timestamp: std::time::SystemTime::UNIX_EPOCH,
                    identity: rescope_core::ProcessIdentity {
                        pid: index as u32 + 1,
                        start_time_epoch_s: 1,
                        name: (*name).to_string(),
                    },
                    user_id: Some("1000".to_string()),
                    user_name: Some("alice".to_string()),
                    parent_pid: Some(1),
                    executable: Some(format!("/usr/bin/{name}")),
                    command: Some(format!("/usr/bin/{name}")),
                    memory_bytes: 64,
                    virtual_memory_bytes: 64,
                    cpu_percent: index as f32,
                    disk_total_read_bytes: 0,
                    disk_total_write_bytes: 0,
                    disk_read_delta_bytes: 0,
                    disk_write_delta_bytes: 0,
                })
                .collect(),
            sample_interval: Duration::from_secs(1),
            logical_cpu_count: 1,
        };

        build_snapshot_report(
            &sample,
            SnapshotReportOptions {
                interval: Duration::from_secs(1),
                group_by: GroupBy::Process,
                sort_by: SortBy::Name,
                filters: FilterSpec::default(),
                show_command: false,
                limit: 20,
                normalize_cpu: false,
            },
        )
    }
}
