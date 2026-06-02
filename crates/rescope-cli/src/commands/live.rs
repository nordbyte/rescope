use std::thread;

use anyhow::{Result, bail};
use rescope_core::{
    SampleSource, SamplerConfig, SnapshotReportOptions, SysinfoSampler, build_snapshot_report,
    filter_sample, units::MINIMUM_INTERVAL,
};

use crate::args::{Cli, LiveArgs};
use crate::output::{csv, json, table, terminal};
use crate::tui;

pub fn run(cli: &Cli, args: &LiveArgs) -> Result<()> {
    rescope_core::error::validate_interval(args.interval, MINIMUM_INTERVAL)?;
    if cli.stdout_export_count() > 1 {
        bail!("only one of --json - or --csv - can write to stdout");
    }
    if (cli.json.is_some() || cli.csv.is_some()) && !args.once {
        bail!("--json and --csv are supported for live only with --once");
    }
    if args.tui && !args.plain && !args.once && tui::is_available() {
        return tui::run_live(cli, args);
    }
    if args.tui && !tui::is_available() && !cli.quiet {
        eprintln!("interactive TUI mode is planned; using plain terminal refresh for now");
    }

    let filter = args.filters.to_filter_spec();
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: args.needs_command(),
        include_executable: args.needs_executable(),
    })?;
    sampler.warm_up(args.interval)?;

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

        if args.once {
            if let Some(path) = &cli.json {
                json::write_snapshot(path, &report)?;
            }
            if let Some(path) = &cli.csv {
                csv::write_snapshot(path, &report)?;
            }
        }

        if !cli.quiet {
            if !args.once {
                terminal::clear_screen()?;
            }
            if !json::writes_stdout(&cli.json) && !csv::writes_stdout(&cli.csv) {
                table::print_snapshot(&report, cli.bytes, true, cli.color_enabled());
            }
        }

        if args.once {
            break;
        }

        thread::sleep(args.interval);
    }

    Ok(())
}
