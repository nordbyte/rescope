use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use rescope_core::{
    FilterSpec, GroupBy, RawProcessSample, SampleSource, SamplerConfig, SnapshotReport,
    SnapshotReportOptions, SortBy, SysinfoSampler, SystemSample, build_snapshot_report,
    filter_sample, format_bps, format_bytes, system_time_ms,
};

use crate::args::{Cli, LiveArgs};
use crate::output::{csv, json, table};
use crate::tui::view;

const GROUP_OPTIONS: [GroupBy; 6] = [
    GroupBy::Process,
    GroupBy::Name,
    GroupBy::User,
    GroupBy::Command,
    GroupBy::Executable,
    GroupBy::Parent,
];

const OPTIONS_ITEMS: [&str; 8] = [
    "Sort", "Group", "Filters", "View", "Sampling", "Export", "Details", "Help",
];
const FILTER_ITEMS: [&str; 3] = ["Edit search", "Clear search", "Toggle hide self"];
const VIEW_ITEMS: [&str; 3] = [
    "Toggle normalized CPU",
    "Toggle raw bytes",
    "Toggle command display",
];
const SAMPLING_ITEMS: [&str; 5] = [
    "Increase row limit",
    "Decrease row limit",
    "Faster refresh",
    "Slower refresh",
    "Pause or resume",
];
const EXPORT_ITEMS: [&str; 2] = ["Export JSON", "Export CSV"];
const INTERVAL_STEPS: [Duration; 6] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(5),
    Duration::from_secs(10),
];

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
    paused: bool,
    overlay: Overlay,
    status_message: Option<String>,
    pending_export: Option<ExportFormat>,
    last_report: Option<SnapshotReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Overlay {
    None,
    Help,
    Options { selected: usize },
    Sort { selected: usize },
    Group { selected: usize },
    Filter { selected: usize },
    View { selected: usize },
    Sampling { selected: usize },
    Export { selected: usize },
    Detail,
    Search { input: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportFormat {
    Json,
    Csv,
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
            paused: false,
            overlay: Overlay::None,
            status_message: None,
            pending_export: None,
            last_report: None,
        }
    }

    fn overlay_open(&self) -> bool {
        !matches!(self.overlay, Overlay::None)
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
        self.status_message = Some(if self.paused {
            "paused".to_string()
        } else {
            "resumed".to_string()
        });
    }

    fn toggle_normalized_cpu(&mut self) {
        self.normalize_cpu = !self.normalize_cpu;
    }

    fn toggle_raw_bytes(&mut self) {
        self.raw_bytes = !self.raw_bytes;
    }

    fn toggle_show_command(&mut self) {
        self.show_command = !self.show_command;
    }

    fn perform_pending_export(&mut self) {
        let Some(format) = self.pending_export.take() else {
            return;
        };
        let Some(report) = &self.last_report else {
            self.status_message = Some("no snapshot to export yet".to_string());
            return;
        };

        let timestamp = system_time_ms(report.ended_at);
        let path = match format {
            ExportFormat::Json => PathBuf::from(format!("rescope-snapshot-{timestamp}.json")),
            ExportFormat::Csv => PathBuf::from(format!("rescope-snapshot-{timestamp}.csv")),
        };
        let result = match format {
            ExportFormat::Json => json::write_snapshot(path.as_path(), report),
            ExportFormat::Csv => csv::write_snapshot(path.as_path(), report),
        };
        self.status_message = Some(match result {
            Ok(()) => format!("exported {}", path.display()),
            Err(error) => format!("export failed: {error}"),
        });
    }
}

pub fn run_live(cli: &Cli, args: &LiveArgs) -> Result<()> {
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: true,
        include_executable: true,
    })?;
    sampler.warm_up(args.interval)?;

    let mut app = TuiApp::new(cli, args);
    let _guard = enter_terminal()?;
    let mut cached_sample: Option<SystemSample> = None;

    loop {
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
        app.tick_count += 1;
        app.set_report(report.clone());
        app.perform_pending_export();

        execute!(
            io::stdout(),
            Clear(ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        write_tui_text(&render_app(&app, &report, cli.color_enabled()))?;
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
        Overlay::Help | Overlay::Detail => handle_simple_overlay_key(app, code),
        Overlay::Options { .. }
        | Overlay::Sort { .. }
        | Overlay::Group { .. }
        | Overlay::Filter { .. }
        | Overlay::View { .. }
        | Overlay::Sampling { .. }
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
        KeyCode::Char('s') | KeyCode::Char('S') => app.open_sort_picker(),
        KeyCode::Char('g') | KeyCode::Char('G') => app.open_group_picker(),
        KeyCode::Char('f') | KeyCode::Char('F') => app.overlay = Overlay::Filter { selected: 0 },
        KeyCode::Char('v') | KeyCode::Char('V') => app.overlay = Overlay::View { selected: 0 },
        KeyCode::Char('e') | KeyCode::Char('E') => app.overlay = Overlay::Export { selected: 0 },
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
        KeyCode::Enter => {
            if app.selected_row_count() > 0 {
                app.overlay = Overlay::Detail;
            }
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
        Overlay::Export { selected } => move_index(selected, EXPORT_ITEMS.len(), direction),
        Overlay::None | Overlay::Help | Overlay::Detail | Overlay::Search { .. } => {}
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
            5 => app.overlay = Overlay::Export { selected: 0 },
            6 => app.overlay = Overlay::Detail,
            7 => app.overlay = Overlay::Help,
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
            2 => app.filter.hide_self = !app.filter.hide_self,
            _ => {}
        },
        Overlay::View { selected } => match selected {
            0 => app.toggle_normalized_cpu(),
            1 => app.toggle_raw_bytes(),
            2 => app.toggle_show_command(),
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
        Overlay::Export { selected } => {
            app.pending_export = Some(if selected == 0 {
                ExportFormat::Json
            } else {
                ExportFormat::Csv
            });
            app.close_overlay();
        }
        Overlay::None | Overlay::Help | Overlay::Detail | Overlay::Search { .. } => {}
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

fn render_app(app: &TuiApp, report: &SnapshotReport, color: bool) -> String {
    let mut output = String::new();
    output.push_str(&view::format_header(report, app.raw_bytes, app.tick_count));
    output.push_str(&format_state_line(app, report));
    output.push_str(&table::render_snapshot(report, app.raw_bytes, false, color));
    output.push_str(&format_overlay(app));
    output.push_str(&format_footer(app));
    output
}

fn format_state_line(app: &TuiApp, report: &SnapshotReport) -> String {
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
    writeln!(
        &mut output,
        "mode {status} | group {} | limit {} | selected {selected} | search {search} | normalized {} | bytes {} | command {}",
        group_label(app.group_by),
        limit_label(app.limit),
        on_off(app.normalize_cpu),
        on_off(app.raw_bytes),
        on_off(app.show_command)
    )
    .expect("writing to a string cannot fail");
    if let Some(message) = &app.status_message {
        writeln!(&mut output, "status: {message}").expect("writing to a string cannot fail");
    }
    output.push('\n');
    output
}

fn format_overlay(app: &TuiApp) -> String {
    match &app.overlay {
        Overlay::None => String::new(),
        Overlay::Help => format_help(),
        Overlay::Options { selected } => format_menu("Options", OPTIONS_ITEMS, *selected, None),
        Overlay::Sort { selected } => format_sort_menu(*selected, app.sort_by),
        Overlay::Group { selected } => format_group_menu(*selected, app.group_by),
        Overlay::Filter { selected } => format_filter_menu(*selected, app),
        Overlay::View { selected } => format_view_menu(*selected, app),
        Overlay::Sampling { selected } => format_sampling_menu(*selected, app),
        Overlay::Export { selected } => format_menu("Export", EXPORT_ITEMS, *selected, None),
        Overlay::Detail => format_detail(app),
        Overlay::Search { input } => format!("Search\n> {input}\n\nEnter apply | Esc cancel\n\n"),
    }
}

fn format_menu(
    title: &str,
    items: impl IntoIterator<Item = &'static str>,
    selected: usize,
    suffixes: Option<Vec<String>>,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "{title}").expect("writing to a string cannot fail");
    for (index, item) in items.into_iter().enumerate() {
        let marker = if index == selected { ">" } else { " " };
        let suffix = suffixes
            .as_ref()
            .and_then(|values| values.get(index))
            .map(|value| format!(" {value}"))
            .unwrap_or_default();
        writeln!(&mut output, "{marker} {item}{suffix}").expect("writing to a string cannot fail");
    }
    output.push('\n');
    output
}

fn format_sort_menu(selected: usize, current_sort: SortBy) -> String {
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
    )
}

fn format_group_menu(selected: usize, current_group: GroupBy) -> String {
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
    )
}

fn format_filter_menu(selected: usize, app: &TuiApp) -> String {
    let suffixes = vec![
        if app.search_query.is_empty() {
            "none".to_string()
        } else {
            app.search_query.clone()
        },
        String::new(),
        on_off(app.filter.hide_self).to_string(),
    ];
    format_menu("Filters", FILTER_ITEMS, selected, Some(suffixes))
}

fn format_view_menu(selected: usize, app: &TuiApp) -> String {
    let suffixes = vec![
        on_off(app.normalize_cpu).to_string(),
        on_off(app.raw_bytes).to_string(),
        on_off(app.show_command).to_string(),
    ];
    format_menu("View", VIEW_ITEMS, selected, Some(suffixes))
}

fn format_sampling_menu(selected: usize, app: &TuiApp) -> String {
    let suffixes = vec![
        limit_label(app.limit),
        limit_label(app.limit),
        humantime::format_duration(app.interval).to_string(),
        humantime::format_duration(app.interval).to_string(),
        if app.paused { "paused" } else { "live" }.to_string(),
    ];
    format_menu("Sampling", SAMPLING_ITEMS, selected, Some(suffixes))
}

fn format_help() -> String {
    [
        "Help",
        "o options menu",
        "s sort menu | g group menu | f filters | v view | e export",
        "/ search | up/down select row | Enter details",
        "space pause/resume | +/- row limit | [/] refresh interval",
        "n normalized CPU | b raw bytes | c command display",
        "Esc close overlay or quit main view | q quit",
        "",
    ]
    .join("\n")
}

fn format_detail(app: &TuiApp) -> String {
    let Some(report) = &app.last_report else {
        return "Details\nno snapshot yet\n\n".to_string();
    };
    let Some(row) = report.rows.get(app.selected_row) else {
        return "Details\nno selected row\n\n".to_string();
    };

    let mut output = String::new();
    writeln!(&mut output, "Details").expect("writing to a string cannot fail");
    writeln!(&mut output, "name: {}", row.display_name).expect("writing to a string cannot fail");
    if let Some(pid) = row.pid {
        writeln!(&mut output, "pid: {pid}").expect("writing to a string cannot fail");
    }
    writeln!(&mut output, "group: {}", group_label(row.group_type))
        .expect("writing to a string cannot fail");
    writeln!(
        &mut output,
        "user: {}",
        row.user_name
            .as_deref()
            .or(row.users.as_deref())
            .unwrap_or("unknown")
    )
    .expect("writing to a string cannot fail");
    writeln!(
        &mut output,
        "cpu: {:.1}% | ram: {} | read: {} | write: {} | io: {}",
        row.cpu_percent,
        format_bytes(row.ram_bytes, app.raw_bytes),
        format_bps(row.read_bps, app.raw_bytes),
        format_bps(row.write_bps, app.raw_bytes),
        format_bps(row.io_bps, app.raw_bytes)
    )
    .expect("writing to a string cannot fail");
    if let Some(process) = &row.top_process {
        writeln!(&mut output, "top process: {process}").expect("writing to a string cannot fail");
    }
    output.push_str("\nEsc close | q quit\n\n");
    output
}

fn format_footer(app: &TuiApp) -> String {
    if app.overlay_open() {
        "\nup/down choose | Enter apply | Esc back | q quit\n".to_string()
    } else {
        "\no options | ? help | / search | Enter details | s sort | q quit\n".to_string()
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
            paused: false,
            overlay: Overlay::None,
            status_message: None,
            pending_export: None,
            last_report: None,
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
}
