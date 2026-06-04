use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use rescope_core::{
    CompiledFilter, RecordingReportOptions, SystemSample, build_recording_report,
    filter_sample_with,
};
use serde::Deserialize;

use crate::args::{Cli, ReplayArgs};
use crate::output::{csv, json, table};

#[derive(Debug, Deserialize)]
struct RawSamplesEnvelope {
    #[allow(dead_code)]
    mode: Option<String>,
    #[serde(default)]
    interval_ms: Option<u64>,
    samples: Vec<SystemSample>,
}

pub fn run(cli: &Cli, args: &ReplayArgs) -> Result<()> {
    if cli.stdout_export_count() > 1 {
        bail!("only one of --json - or --csv - can write to stdout");
    }

    let text = std::fs::read_to_string(&args.input)
        .with_context(|| format!("reading {}", args.input.display()))?;
    let raw: RawSamplesEnvelope =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", args.input.display()))?;
    if raw.samples.is_empty() {
        bail!("raw sample file does not contain samples");
    }

    let filter = args.filters.to_filter_spec();
    let matcher = CompiledFilter::new(&filter);
    let samples = raw
        .samples
        .iter()
        .map(|sample| filter_sample_with(sample, &matcher))
        .collect::<Vec<_>>();
    let duration = measured_duration(&raw.samples)
        .or_else(|| {
            raw.interval_ms
                .map(|ms| Duration::from_millis(ms) * raw.samples.len() as u32)
        })
        .unwrap_or_else(|| Duration::from_secs(1));

    let report = build_recording_report(
        &samples,
        RecordingReportOptions {
            requested_duration: duration,
            interval: raw
                .interval_ms
                .map(Duration::from_millis)
                .unwrap_or_else(|| {
                    samples
                        .first()
                        .map(|sample| sample.sample_interval)
                        .unwrap_or(duration)
                }),
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

fn measured_duration(samples: &[SystemSample]) -> Option<Duration> {
    let start = samples.first().map(|sample| sample.timestamp)?;
    let end = samples.last().map(|sample| sample.timestamp)?;
    duration_between(start, end).filter(|duration| !duration.is_zero())
}

fn duration_between(start: SystemTime, end: SystemTime) -> Option<Duration> {
    end.duration_since(start).ok()
}
