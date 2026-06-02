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
}

impl TuiApp {
    fn new(sort_by: SortBy) -> Self {
        Self {
            tick_count: 0,
            sort_by,
        }
    }

    fn set_sort(&mut self, sort_by: SortBy) -> bool {
        if self.sort_by == sort_by {
            false
        } else {
            self.sort_by = sort_by;
            true
        }
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
        view::render_header(&report, cli.bytes, app.tick_count);
        table::print_snapshot(&report, cli.bytes, false, cli.color_enabled());
        view::render_footer();
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
            let quit_key = matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q')
            );
            let ctrl_c =
                key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
            if quit_key || ctrl_c {
                return Ok(TuiInput::Quit);
            }

            let sort_modifier = key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT;
            if sort_modifier
                && let Some(sort_by) = sort_for_key(key.code)
                && app.set_sort(sort_by)
            {
                return Ok(TuiInput::RefreshNow);
            }
        }
    }
}

fn sort_for_key(code: KeyCode) -> Option<SortBy> {
    let KeyCode::Char(ch) = code else {
        return None;
    };

    match ch.to_ascii_lowercase() {
        'c' => Some(SortBy::Cpu),
        'm' => Some(SortBy::Ram),
        'i' => Some(SortBy::Io),
        'r' => Some(SortBy::Read),
        'w' => Some(SortBy::Write),
        'p' => Some(SortBy::Pid),
        'n' => Some(SortBy::Name),
        'u' => Some(SortBy::User),
        _ => None,
    }
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
    fn maps_sort_hotkeys() {
        assert_eq!(sort_for_key(KeyCode::Char('c')), Some(SortBy::Cpu));
        assert_eq!(sort_for_key(KeyCode::Char('M')), Some(SortBy::Ram));
        assert_eq!(sort_for_key(KeyCode::Char('i')), Some(SortBy::Io));
        assert_eq!(sort_for_key(KeyCode::Char('r')), Some(SortBy::Read));
        assert_eq!(sort_for_key(KeyCode::Char('w')), Some(SortBy::Write));
        assert_eq!(sort_for_key(KeyCode::Char('p')), Some(SortBy::Pid));
        assert_eq!(sort_for_key(KeyCode::Char('n')), Some(SortBy::Name));
        assert_eq!(sort_for_key(KeyCode::Char('u')), Some(SortBy::User));
        assert_eq!(sort_for_key(KeyCode::Char('x')), None);
    }

    #[test]
    fn sort_state_only_changes_for_new_sort() {
        let mut app = TuiApp::new(SortBy::Cpu);
        assert!(!app.set_sort(SortBy::Cpu));
        assert!(app.set_sort(SortBy::Ram));
        assert_eq!(app.sort_by, SortBy::Ram);
    }
}
