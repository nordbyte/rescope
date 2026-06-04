use std::io::Write;
use std::path::Path;
use std::thread;

use anyhow::{Result, bail};
use rescope_core::{
    SampleSource, SamplerConfig, SnapshotReport, SnapshotReportOptions, SysinfoSampler,
    build_snapshot_report, filter_sample, metrics::system_time_ms, units::MINIMUM_INTERVAL,
};
use serde::Serialize;

use crate::args::{Cli, LiveArgs};
use crate::commands::verbose;
use crate::output::{csv, json, table, terminal};
use crate::tui;

pub fn run(cli: &Cli, args: &LiveArgs) -> Result<()> {
    rescope_core::error::validate_interval(args.interval, MINIMUM_INTERVAL)?;
    if stdout_output_count(cli, args) > 1 {
        bail!("only one of --json - or --csv - can write to stdout");
    }
    if (cli.json.is_some() || cli.csv.is_some()) && !args.once {
        bail!(
            "--json and --csv are supported for live only with --once; use --jsonl or --csv-stream for continuous live exports"
        );
    }
    if streams_stdout(args) && !cli.quiet {
        bail!("stdout live streams require --quiet to keep table output off stdout");
    }
    if args.tui && !args.plain && !args.once && tui::is_available() {
        return tui::run_live(cli, args);
    }
    if args.tui && !tui::is_available() && !cli.quiet {
        eprintln!("interactive TUI is unavailable; using plain terminal refresh");
    }

    let filter = args.filters.to_filter_spec();
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: args.needs_command(),
        include_executable: args.needs_executable(),
    })?;
    verbose(
        cli,
        format!(
            "live group={:?} sort={:?} limit={} interval={} command={} executable={}",
            args.effective_group(),
            args.effective_sort(),
            args.effective_limit(),
            humantime::format_duration(args.interval),
            args.needs_command(),
            args.needs_executable()
        ),
    );
    sampler.warm_up(args.interval)?;
    let mut jsonl = args
        .jsonl
        .as_ref()
        .map(|path| stream_writer(path))
        .transpose()?;
    let mut csv_stream = args
        .csv_stream
        .as_ref()
        .map(|path| csv_stream_writer(path))
        .transpose()?;

    loop {
        let sample = sampler.sample()?;
        let filtered = filter_sample(&sample, &filter);
        if args.once {
            verbose(
                cli,
                format!(
                    "matched {} of {} processes",
                    filtered.processes.len(),
                    sample.processes.len()
                ),
            );
        }
        let report = build_snapshot_report(
            &filtered,
            SnapshotReportOptions {
                interval: args.interval,
                group_by: args.effective_group(),
                sort_by: args.effective_sort(),
                filters: filter.clone(),
                show_command: args.effective_show_command(),
                show_path: args.effective_show_path(),
                limit: args.effective_limit(),
                normalize_cpu: args.normalize_cpu,
            },
        );
        if let Some(writer) = jsonl.as_mut() {
            write_jsonl_snapshot(writer, &report)?;
        }
        if let Some(writer) = csv_stream.as_mut() {
            write_csv_stream_snapshot(writer, &report)?;
        }

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

#[derive(Serialize)]
struct LiveEnvelope<'a> {
    tool: &'static str,
    version: &'static str,
    mode: &'static str,
    #[serde(flatten)]
    report: &'a SnapshotReport,
}

struct CsvStreamWriter {
    writer: ::csv::Writer<Box<dyn Write>>,
}

fn stdout_output_count(cli: &Cli, args: &LiveArgs) -> usize {
    cli.stdout_export_count()
        + path_writes_stdout(&args.jsonl) as usize
        + path_writes_stdout(&args.csv_stream) as usize
}

fn streams_stdout(args: &LiveArgs) -> bool {
    path_writes_stdout(&args.jsonl) || path_writes_stdout(&args.csv_stream)
}

fn path_writes_stdout(path: &Option<std::path::PathBuf>) -> bool {
    path.as_deref() == Some(Path::new("-"))
}

fn stream_writer(path: &Path) -> Result<Box<dyn Write>> {
    if path == Path::new("-") {
        Ok(Box::new(std::io::stdout().lock()))
    } else {
        Ok(Box::new(std::fs::File::create(path)?))
    }
}

fn csv_stream_writer(path: &Path) -> Result<CsvStreamWriter> {
    let writer = stream_writer(path)?;
    let mut writer = ::csv::Writer::from_writer(writer);
    writer.write_record([
        "timestamp",
        "group_type",
        "display_name",
        "pid",
        "user_name",
        "cpu_percent",
        "ram_bytes",
        "disk_read_delta_bytes",
        "disk_write_delta_bytes",
        "disk_io_delta_bytes",
        "status",
        "run_time_seconds",
        "thread_count",
        "open_file_count",
    ])?;
    Ok(CsvStreamWriter { writer })
}

fn write_jsonl_snapshot(writer: &mut Box<dyn Write>, report: &SnapshotReport) -> Result<()> {
    let envelope = LiveEnvelope {
        tool: "rescope",
        version: env!("CARGO_PKG_VERSION"),
        mode: "live",
        report,
    };
    serde_json::to_writer(&mut **writer, &envelope)?;
    writeln!(writer)?;
    writer.flush()?;
    Ok(())
}

fn write_csv_stream_snapshot(writer: &mut CsvStreamWriter, report: &SnapshotReport) -> Result<()> {
    for row in &report.rows {
        writer.writer.write_record([
            system_time_ms(row.timestamp).to_string(),
            format!("{:?}", row.group_type).to_ascii_lowercase(),
            row.display_name.clone(),
            row.pid.map(|pid| pid.to_string()).unwrap_or_default(),
            row.user_name.clone().unwrap_or_default(),
            row.cpu_percent.to_string(),
            row.ram_bytes.to_string(),
            row.disk_read_delta_bytes.to_string(),
            row.disk_write_delta_bytes.to_string(),
            row.disk_io_delta_bytes.to_string(),
            row.details.status.clone().unwrap_or_default(),
            row.details
                .run_time_seconds
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.details
                .thread_count
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.details
                .open_file_count
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ])?;
    }
    writer.writer.flush()?;
    Ok(())
}
