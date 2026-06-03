use std::thread;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use rescope_core::{
    RecordingAccumulator, RecordingAccumulatorOptions, RecordingReportOptions, SampleSource,
    SamplerConfig, SysinfoSampler, build_recording_report_from_accumulator, filter_sample,
    units::MINIMUM_INTERVAL,
};

use crate::args::{Cli, RecordArgs};
use crate::commands::verbose;
use crate::output::{csv, json, table};

pub fn run(cli: &Cli, args: &RecordArgs) -> Result<()> {
    if cli.stdout_export_count() > 1 {
        bail!("only one of --json - or --csv - can write to stdout");
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
            "record group={:?} sort={:?} limit={} duration={} interval={} command={} executable={}",
            args.effective_group(),
            args.effective_sort(),
            args.effective_limit(),
            humantime::format_duration(args.duration),
            humantime::format_duration(args.interval),
            args.needs_command(),
            args.needs_executable()
        ),
    );
    sampler.warm_up(args.interval)?;

    if !cli.quiet {
        eprintln!(
            "recording for {} at interval {}...",
            humantime::format_duration(args.duration),
            humantime::format_duration(args.interval)
        );
    }

    let recording_started = Instant::now();
    let deadline = recording_started + args.duration;
    let mut accumulator = RecordingAccumulator::new(RecordingAccumulatorOptions {
        group_by: args.effective_group(),
        sort_by: args.effective_sort(),
        interval: args.interval,
        show_command: args.effective_show_command(),
        show_path: args.effective_show_path(),
        include_idle: args.effective_include_idle(),
    });

    while Instant::now() < deadline {
        let sample = sampler.sample()?;
        let filtered = filter_sample(&sample, &filter);
        verbose(
            cli,
            format!(
                "sample {} matched {} of {} processes",
                accumulator.sample_count() + 1,
                filtered.processes.len(),
                sample.processes.len()
            ),
        );
        accumulator.push_sample(&filtered);

        let now = Instant::now();
        if now >= deadline {
            break;
        }
        thread::sleep(args.interval.min(deadline - now));
    }

    let report = build_recording_report_from_accumulator(
        accumulator,
        RecordingReportOptions {
            requested_duration: recording_started.elapsed(),
            interval: args.interval,
            group_by: args.effective_group(),
            sort_by: args.effective_sort(),
            filters: filter,
            show_command: args.effective_show_command(),
            show_path: args.effective_show_path(),
            limit: args.effective_limit(),
            include_idle: args.effective_include_idle(),
            normalize_cpu: args.normalize_cpu,
        },
    );

    if let Some(path) = &cli.json {
        json::write_recording(path, &report)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    if let Some(path) = &cli.csv {
        csv::write_recording(path, &report)
            .with_context(|| format!("writing {}", path.display()))?;
    }

    if !cli.quiet && !json::writes_stdout(&cli.json) && !csv::writes_stdout(&cli.csv) {
        table::print_recording(&report, cli.bytes, args.timeline, cli.color_enabled());
    }

    Ok(())
}
