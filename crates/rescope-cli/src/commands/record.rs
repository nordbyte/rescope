use std::thread;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use rescope_core::{
    RecordingReportOptions, SampleSource, SamplerConfig, SysinfoSampler, build_recording_report,
    filter_sample, units::MINIMUM_INTERVAL,
};

use crate::args::{Cli, RecordArgs};
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
    sampler.warm_up(args.interval)?;

    if !cli.quiet {
        eprintln!(
            "recording for {} at interval {}...",
            humantime::format_duration(args.duration),
            humantime::format_duration(args.interval)
        );
    }

    let deadline = Instant::now() + args.duration;
    let mut samples = Vec::new();

    while Instant::now() < deadline {
        let sample = sampler.sample()?;
        samples.push(filter_sample(&sample, &filter));

        let now = Instant::now();
        if now >= deadline {
            break;
        }
        thread::sleep(args.interval.min(deadline - now));
    }

    let report = build_recording_report(
        &samples,
        RecordingReportOptions {
            requested_duration: args.duration,
            interval: args.interval,
            group_by: args.group.into(),
            sort_by: args.sort.into(),
            filters: filter,
            show_command: args.filters.show_command,
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
