use std::collections::{BTreeSet, HashMap};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;

use crate::args::{Cli, DiffArgs};
use crate::output::{csv as output_csv, json};

#[derive(Debug, Clone, Serialize)]
struct DiffReport {
    before_mode: String,
    after_mode: String,
    before_rows: usize,
    after_rows: usize,
    changed_rows: usize,
    rows: Vec<DiffRow>,
}

#[derive(Debug, Clone, Serialize)]
struct DiffRow {
    key: String,
    status: DiffStatus,
    before: Option<RowMetrics>,
    after: Option<RowMetrics>,
    delta_cpu_percent: f64,
    delta_ram_bytes: i64,
    delta_io_bytes: i64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum DiffStatus {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct RowMetrics {
    cpu_percent: f64,
    ram_bytes: u64,
    io_bytes: u64,
}

pub fn run(cli: &Cli, args: &DiffArgs) -> Result<()> {
    if cli.stdout_export_count() > 1 {
        bail!("only one of --json - or --csv - can write to stdout");
    }

    let before = read_report(&args.before)?;
    let after = read_report(&args.after)?;
    let mut diff = diff_reports(&before, &after);
    diff.rows
        .sort_by(|left, right| impact(right).total_cmp(&impact(left)));
    if !args.all && diff.rows.len() > args.limit {
        diff.rows.truncate(args.limit);
    }
    diff.changed_rows = diff.rows.len();

    if let Some(path) = &cli.json {
        json::write_custom(path, "diff", &diff)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    if let Some(path) = &cli.csv {
        write_diff_csv(path, &diff).with_context(|| format!("writing {}", path.display()))?;
    }
    if !cli.quiet && !json::writes_stdout(&cli.json) && !output_csv::writes_stdout(&cli.csv) {
        print!("{}", render_diff(&diff, cli.bytes));
    }
    Ok(())
}

fn read_report(path: &std::path::Path) -> Result<Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn diff_reports(before: &Value, after: &Value) -> DiffReport {
    let before_rows = extract_rows(before);
    let after_rows = extract_rows(after);
    let keys = before_rows
        .keys()
        .chain(after_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut rows = Vec::new();

    for key in keys {
        let before = before_rows.get(&key).copied();
        let after = after_rows.get(&key).copied();
        let Some(status) = diff_status(before, after) else {
            continue;
        };
        let delta_cpu_percent = after.map(|value| value.cpu_percent).unwrap_or_default()
            - before.map(|value| value.cpu_percent).unwrap_or_default();
        let delta_ram_bytes = after.map(|value| value.ram_bytes).unwrap_or_default() as i64
            - before.map(|value| value.ram_bytes).unwrap_or_default() as i64;
        let delta_io_bytes = after.map(|value| value.io_bytes).unwrap_or_default() as i64
            - before.map(|value| value.io_bytes).unwrap_or_default() as i64;
        rows.push(DiffRow {
            key,
            status,
            before,
            after,
            delta_cpu_percent,
            delta_ram_bytes,
            delta_io_bytes,
        });
    }

    DiffReport {
        before_mode: string_field(before, "mode"),
        after_mode: string_field(after, "mode"),
        before_rows: before_rows.len(),
        after_rows: after_rows.len(),
        changed_rows: rows.len(),
        rows,
    }
}

fn extract_rows(report: &Value) -> HashMap<String, RowMetrics> {
    report
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| Some((row_key(row)?, row_metrics(row))))
        .collect()
}

fn row_key(row: &Value) -> Option<String> {
    let display_name = row.get("display_name")?.as_str()?.to_string();
    let group_type = row
        .get("group_type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let pid = row.get("pid").and_then(Value::as_u64);
    Some(match pid {
        Some(pid) => format!("{group_type}:{pid}:{display_name}"),
        None => format!("{group_type}:{display_name}"),
    })
}

fn row_metrics(row: &Value) -> RowMetrics {
    RowMetrics {
        cpu_percent: number(row, "cpu_percent")
            .or_else(|| number(row, "cpu_avg_percent"))
            .unwrap_or_default(),
        ram_bytes: number(row, "ram_bytes")
            .or_else(|| number(row, "ram_max_bytes"))
            .unwrap_or_default()
            .max(0.0) as u64,
        io_bytes: number(row, "disk_io_delta_bytes")
            .or_else(|| number(row, "disk_io_total_bytes"))
            .unwrap_or_default()
            .max(0.0) as u64,
    }
}

fn diff_status(before: Option<RowMetrics>, after: Option<RowMetrics>) -> Option<DiffStatus> {
    match (before, after) {
        (None, Some(_)) => Some(DiffStatus::Added),
        (Some(_), None) => Some(DiffStatus::Removed),
        (Some(before), Some(after)) if changed(before, after) => Some(DiffStatus::Changed),
        _ => None,
    }
}

fn changed(before: RowMetrics, after: RowMetrics) -> bool {
    (before.cpu_percent - after.cpu_percent).abs() > f64::EPSILON
        || before.ram_bytes != after.ram_bytes
        || before.io_bytes != after.io_bytes
}

fn impact(row: &DiffRow) -> f64 {
    row.delta_cpu_percent.abs()
        + (row.delta_ram_bytes.unsigned_abs() as f64 / 1024.0 / 1024.0)
        + (row.delta_io_bytes.unsigned_abs() as f64 / 1024.0 / 1024.0)
}

fn render_diff(report: &DiffReport, raw_bytes: bool) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "rescope diff: {} -> {} | rows {} -> {} | changed {}\n",
        report.before_mode,
        report.after_mode,
        report.before_rows,
        report.after_rows,
        report.changed_rows
    ));
    output.push_str("STATUS   CPU_DELTA RAM_DELTA IO_DELTA  KEY\n");
    for row in &report.rows {
        output.push_str(&format!(
            "{:<8} {:>9.1} {:>9} {:>9} {}\n",
            status_label(row.status),
            row.delta_cpu_percent,
            signed_bytes(row.delta_ram_bytes, raw_bytes),
            signed_bytes(row.delta_io_bytes, raw_bytes),
            row.key
        ));
    }
    if report.rows.is_empty() {
        output.push_str("no changed rows\n");
    }
    output
}

fn write_diff_csv(path: &std::path::Path, report: &DiffReport) -> Result<()> {
    let mut writer: Box<dyn std::io::Write> = if path == std::path::Path::new("-") {
        Box::new(std::io::stdout().lock())
    } else {
        Box::new(std::fs::File::create(path)?)
    };
    let mut csv = ::csv::Writer::from_writer(&mut writer);
    csv.write_record([
        "status",
        "key",
        "before_cpu_percent",
        "after_cpu_percent",
        "delta_cpu_percent",
        "before_ram_bytes",
        "after_ram_bytes",
        "delta_ram_bytes",
        "before_io_bytes",
        "after_io_bytes",
        "delta_io_bytes",
    ])?;
    for row in &report.rows {
        csv.write_record([
            status_label(row.status).to_string(),
            row.key.clone(),
            row.before
                .map(|value| value.cpu_percent.to_string())
                .unwrap_or_default(),
            row.after
                .map(|value| value.cpu_percent.to_string())
                .unwrap_or_default(),
            row.delta_cpu_percent.to_string(),
            row.before
                .map(|value| value.ram_bytes.to_string())
                .unwrap_or_default(),
            row.after
                .map(|value| value.ram_bytes.to_string())
                .unwrap_or_default(),
            row.delta_ram_bytes.to_string(),
            row.before
                .map(|value| value.io_bytes.to_string())
                .unwrap_or_default(),
            row.after
                .map(|value| value.io_bytes.to_string())
                .unwrap_or_default(),
            row.delta_io_bytes.to_string(),
        ])?;
    }
    csv.flush()?;
    Ok(())
}

fn number(value: &Value, field: &str) -> Option<f64> {
    value.get(field).and_then(Value::as_f64)
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

fn status_label(status: DiffStatus) -> &'static str {
    match status {
        DiffStatus::Added => "added",
        DiffStatus::Removed => "removed",
        DiffStatus::Changed => "changed",
    }
}

fn signed_bytes(value: i64, raw_bytes: bool) -> String {
    rescope_core::format_signed_bytes(value, raw_bytes)
}
