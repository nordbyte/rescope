use std::io::{self, Write};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use rescope_core::{
    SampleSource, SamplerConfig, SnapshotReportOptions, SysinfoSampler, build_snapshot_report,
    filter_sample,
};

use crate::args::{Cli, LiveArgs};
use crate::output::table;
use crate::tui::view;

#[derive(Debug, Default)]
pub struct TuiApp {
    tick_count: u64,
}

pub fn run_live(cli: &Cli, args: &LiveArgs) -> Result<()> {
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: args.needs_command(),
        include_executable: args.needs_executable(),
    })?;
    sampler.warm_up(args.interval)?;

    let filter = args.filters.to_filter_spec();
    let mut app = TuiApp::default();
    let _guard = enter_terminal()?;
    let mut next_tick = Instant::now();

    loop {
        let sample = sampler.sample()?;
        let filtered = filter_sample(&sample, &filter);
        let report = build_snapshot_report(
            &filtered,
            SnapshotReportOptions {
                interval: args.interval,
                group_by: args.group.into(),
                sort_by: args.sort.into(),
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

        next_tick += args.interval;
        if wait_for_exit_until(next_tick)? {
            break;
        }
        if Instant::now() > next_tick + args.interval {
            next_tick = Instant::now();
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

fn wait_for_exit_until(deadline: Instant) -> Result<bool> {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Ok(false);
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
                return Ok(true);
            }
        }
    }
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
