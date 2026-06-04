use std::thread;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use rescope_core::{
    SampleSource, SamplerConfig, SnapshotReportOptions, SysinfoSampler, build_snapshot_report,
    filter_sample, units::MINIMUM_INTERVAL,
};

use crate::args::{Cli, WatchArgs};
use crate::commands::verbose;
use crate::output::{csv, json, table};

pub fn run(cli: &Cli, args: &WatchArgs) -> Result<()> {
    if cli.stdout_export_count() > 1 {
        bail!("only one of --json - or --csv - can write to stdout");
    }
    if args.stream && (cli.json.is_some() || cli.csv.is_some()) {
        bail!("watch --stream cannot be combined with --json or --csv");
    }
    rescope_core::error::validate_recording_timing(args.duration, args.interval, MINIMUM_INTERVAL)?;

    let filter = args.filters.to_filter_spec();
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: args.needs_command(),
        include_executable: args.needs_executable(),
    })?;
    verbose(
        cli,
        format!(
            "watch sort={:?} limit={} duration={} interval={} command={} executable={}",
            args.effective_sort(),
            args.effective_limit(),
            humantime::format_duration(args.duration),
            humantime::format_duration(args.interval),
            args.needs_command(),
            args.needs_executable()
        ),
    );
    sampler.warm_up(args.interval)?;

    let started = Instant::now();
    let deadline = started + args.duration;
    let mut matched = false;

    while Instant::now() < deadline {
        let sample = sampler.sample()?;
        let filtered = filter_sample(&sample, &filter);
        let report = build_snapshot_report(
            &filtered,
            SnapshotReportOptions {
                interval: args.interval,
                group_by: rescope_core::GroupBy::Process,
                sort_by: args.effective_sort(),
                filters: filter.clone(),
                show_command: args.effective_show_command(),
                show_path: args.effective_show_path(),
                limit: args.effective_limit(),
                normalize_cpu: args.normalize_cpu,
            },
        );

        if args.stream && !cli.quiet {
            table::print_snapshot(&report, cli.bytes, true, cli.color_enabled());
        }

        if !report.rows.is_empty() {
            matched = true;
            if let Some(path) = &cli.json {
                json::write_snapshot(path, &report)
                    .with_context(|| format!("writing {}", path.display()))?;
            }
            if let Some(path) = &cli.csv {
                csv::write_snapshot(path, &report)
                    .with_context(|| format!("writing {}", path.display()))?;
            }
            if !cli.quiet
                && !args.stream
                && !json::writes_stdout(&cli.json)
                && !csv::writes_stdout(&cli.csv)
            {
                eprintln!(
                    "rescope alert matched {} row(s) after {}",
                    report.rows.len(),
                    humantime::format_duration(started.elapsed())
                );
                table::print_snapshot(&report, cli.bytes, true, cli.color_enabled());
            }
            if !args.stream {
                std::process::exit(args.exit_code.into());
            }
        }

        let now = Instant::now();
        if now >= deadline {
            break;
        }
        thread::sleep(args.interval.min(deadline - now));
    }

    if matched {
        std::process::exit(args.exit_code.into());
    }
    if !cli.quiet {
        eprintln!(
            "rescope alert did not match within {}",
            humantime::format_duration(args.duration)
        );
    }

    Ok(())
}
