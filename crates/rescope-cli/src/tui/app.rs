use std::io::{self, Write};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use rescope_core::{
    SampleSource, SamplerConfig, SnapshotReportOptions, SortBy, SysinfoSampler,
    build_snapshot_report, filter_sample,
};

use crate::args::{Cli, LiveArgs};
use crate::output::table;
use crate::tui::view;

#[derive(Debug)]
pub struct TuiApp {
    tick_count: u64,
    sort_by: SortBy,
    sort_picker: Option<SortPicker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SortPicker {
    selected_index: usize,
}

impl TuiApp {
    fn new(sort_by: SortBy) -> Self {
        Self {
            tick_count: 0,
            sort_by,
            sort_picker: None,
        }
    }

    fn open_sort_picker(&mut self) {
        self.sort_picker = Some(SortPicker {
            selected_index: sort_index(self.sort_by),
        });
    }

    fn close_sort_picker(&mut self) {
        self.sort_picker = None;
    }

    fn move_sort_picker(&mut self, direction: PickerDirection) {
        if let Some(picker) = &mut self.sort_picker {
            let len = view::SORT_OPTIONS.len();
            picker.selected_index = match direction {
                PickerDirection::Previous => {
                    if picker.selected_index == 0 {
                        len - 1
                    } else {
                        picker.selected_index - 1
                    }
                }
                PickerDirection::Next => (picker.selected_index + 1) % len,
            };
        }
    }

    fn apply_sort_picker(&mut self) {
        if let Some(picker) = self.sort_picker {
            self.sort_by = view::SORT_OPTIONS[picker.selected_index];
            self.sort_picker = None;
        }
    }

    fn sort_picker_selected_index(&self) -> Option<usize> {
        self.sort_picker.map(|picker| picker.selected_index)
    }
}

pub fn run_live(cli: &Cli, args: &LiveArgs) -> Result<()> {
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: args.needs_command(),
        include_executable: args.needs_executable(),
    })?;
    sampler.warm_up(args.interval)?;

    let filter = args.filters.to_filter_spec();
    let mut app = TuiApp::new(args.sort.into());
    let _guard = enter_terminal()?;

    loop {
        let sample = sampler.sample()?;
        let filtered = filter_sample(&sample, &filter);
        let report = build_snapshot_report(
            &filtered,
            SnapshotReportOptions {
                interval: args.interval,
                group_by: args.group.into(),
                sort_by: app.sort_by,
                filters: filter.clone(),
                show_command: args.filters.show_command,
                limit: args.effective_limit(),
                normalize_cpu: args.normalize_cpu,
            },
        );
        app.tick_count += 1;

        execute!(
            io::stdout(),
            Clear(ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        let mut output = String::new();
        output.push_str(&view::format_header(&report, cli.bytes, app.tick_count));
        output.push_str(&table::render_snapshot(
            &report,
            cli.bytes,
            false,
            cli.color_enabled(),
        ));
        if let Some(selected_index) = app.sort_picker_selected_index() {
            output.push_str(&view::format_sort_picker(selected_index, app.sort_by));
        }
        output.push_str(&view::format_footer(app.sort_picker.is_some()));
        write_tui_text(&output)?;
        io::stdout().flush()?;

        let next_tick = Instant::now() + args.interval;
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
    let sort_picker_open = app.sort_picker.is_some();
    if ctrl_c || is_quit_key(code) || (!sort_picker_open && code == KeyCode::Esc) {
        return TuiInput::Quit;
    }

    if sort_picker_open {
        return handle_sort_picker_key(app, code);
    }

    let sort_modifier = modifiers.is_empty() || modifiers == KeyModifiers::SHIFT;
    if sort_modifier && matches_sort_key(code) {
        app.open_sort_picker();
        return TuiInput::RefreshNow;
    }

    TuiInput::Tick
}

fn handle_sort_picker_key(app: &mut TuiApp, code: KeyCode) -> TuiInput {
    match code {
        KeyCode::Up => {
            app.move_sort_picker(PickerDirection::Previous);
            TuiInput::RefreshNow
        }
        KeyCode::Down => {
            app.move_sort_picker(PickerDirection::Next);
            TuiInput::RefreshNow
        }
        KeyCode::Enter => {
            app.apply_sort_picker();
            TuiInput::RefreshNow
        }
        KeyCode::Esc => {
            app.close_sort_picker();
            TuiInput::RefreshNow
        }
        _ => TuiInput::Tick,
    }
}

fn is_quit_key(code: KeyCode) -> bool {
    matches!(code, KeyCode::Char('q') | KeyCode::Char('Q'))
}

fn matches_sort_key(code: KeyCode) -> bool {
    matches!(code, KeyCode::Char('s') | KeyCode::Char('S'))
}

fn sort_index(sort_by: SortBy) -> usize {
    view::SORT_OPTIONS
        .iter()
        .position(|option| *option == sort_by)
        .unwrap_or(0)
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

    #[test]
    fn opens_sort_picker_with_s_only() {
        let mut app = TuiApp::new(SortBy::Cpu);
        assert_eq!(
            handle_key(&mut app, KeyCode::Char('c'), KeyModifiers::empty()),
            TuiInput::Tick
        );
        assert!(app.sort_picker.is_none());

        assert_eq!(
            handle_key(&mut app, KeyCode::Char('s'), KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert_eq!(app.sort_picker_selected_index(), Some(0));
    }

    #[test]
    fn sort_picker_moves_and_applies_selection() {
        let mut app = TuiApp::new(SortBy::Cpu);
        app.open_sort_picker();

        assert_eq!(
            handle_key(&mut app, KeyCode::Down, KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert_eq!(app.sort_picker_selected_index(), Some(1));

        assert_eq!(
            handle_key(&mut app, KeyCode::Enter, KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert_eq!(app.sort_by, SortBy::Ram);
        assert!(app.sort_picker.is_none());
    }

    #[test]
    fn sort_picker_escape_closes_without_quitting() {
        let mut app = TuiApp::new(SortBy::Cpu);
        app.open_sort_picker();

        assert_eq!(
            handle_key(&mut app, KeyCode::Esc, KeyModifiers::empty()),
            TuiInput::RefreshNow
        );
        assert!(app.sort_picker.is_none());
    }

    #[test]
    fn escape_quits_when_sort_picker_is_closed() {
        let mut app = TuiApp::new(SortBy::Cpu);
        assert_eq!(
            handle_key(&mut app, KeyCode::Esc, KeyModifiers::empty()),
            TuiInput::Quit
        );
    }
}
