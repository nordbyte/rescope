use comfy_table::{Cell, Color, ContentArrangement, Table, presets::NOTHING};
use std::cmp::Reverse;
use std::fmt::Write as _;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use rescope_core::{
    AggregateRow, GroupBy, RecordingReport, SnapshotReport, format_bps, format_bytes,
    format_signed_bytes,
};

use crate::output::sparkline;

const PROCESS_DISPLAY_MAX_CHARS: usize = 32;
const USER_DISPLAY_MAX_CHARS: usize = 32;
const TOP_PROCESS_MAX_CHARS: usize = 32;
const COMMAND_DISPLAY_MAX_CHARS: usize = 56;
const EXECUTABLE_DISPLAY_MAX_CHARS: usize = 56;
const PARENT_DISPLAY_MAX_CHARS: usize = 48;
const TIMELINE_DISPLAY_MAX_CHARS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotColumns {
    pub pid: bool,
    pub user: bool,
    pub process_count: bool,
    pub users: bool,
    pub cpu: bool,
    pub ram: bool,
    pub rates: bool,
    pub totals: bool,
    pub top_process: bool,
}

impl Default for SnapshotColumns {
    fn default() -> Self {
        Self {
            pid: true,
            user: true,
            process_count: true,
            users: true,
            cpu: true,
            ram: true,
            rates: true,
            totals: true,
            top_process: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SnapshotRenderOptions {
    pub show_system: bool,
    pub selected_row: Option<usize>,
    pub row_offset: usize,
    pub max_rows: Option<usize>,
    pub columns: SnapshotColumns,
}

pub fn print_snapshot(report: &SnapshotReport, raw_bytes: bool, show_system: bool, color: bool) {
    print!("{}", render_snapshot(report, raw_bytes, show_system, color));
}

pub fn render_snapshot(
    report: &SnapshotReport,
    raw_bytes: bool,
    show_system: bool,
    color: bool,
) -> String {
    render_snapshot_with_options(
        report,
        raw_bytes,
        color,
        SnapshotRenderOptions {
            show_system,
            ..SnapshotRenderOptions::default()
        },
    )
}

pub fn render_snapshot_with_options(
    report: &SnapshotReport,
    raw_bytes: bool,
    color: bool,
    options: SnapshotRenderOptions,
) -> String {
    let mut output = String::new();

    if options.show_system {
        let used = report
            .total_memory_bytes
            .saturating_sub(report.available_memory_bytes);
        writeln!(
            &mut output,
            "System: CPU {:.1}% | RAM {} / {} | processes {} | interval {}",
            report.global_cpu_percent,
            format_bytes(used, raw_bytes),
            format_bytes(report.total_memory_bytes, raw_bytes),
            report.process_total,
            humantime::format_duration(report.interval)
        )
        .expect("writing to a string cannot fail");
        output.push('\n');
    }

    if report.rows.is_empty() {
        output.push_str("no matching processes\n");
        return output;
    }

    let mut table = plain_table();
    let selected_row = options.selected_row;
    let row_range = visible_row_range(report.rows.len(), options.row_offset, options.max_rows);
    match report.group_by {
        GroupBy::Process => {
            table.set_header(snapshot_process_header(&options, color));
            for (index, row) in report.rows[row_range.clone()].iter().enumerate() {
                let absolute_index = row_range.start + index;
                let mut cells = Vec::new();
                push_selection_cell(&mut cells, selected_row, absolute_index, color);
                if options.columns.pid {
                    cells.push(cell(row.pid.map(|pid| pid.to_string()).unwrap_or_default()));
                }
                if options.columns.user {
                    cells.push(truncated_cell(
                        row.user_name.as_deref().unwrap_or("unknown"),
                        USER_DISPLAY_MAX_CHARS,
                    ));
                }
                cells.push(truncated_cell(&row.display_name, PROCESS_DISPLAY_MAX_CHARS));
                push_metric_cells(&mut cells, row, report, raw_bytes, color, options.columns);
                table.add_row(cells);
            }
        }
        GroupBy::Name | GroupBy::Command | GroupBy::Executable | GroupBy::Parent => {
            table.set_header(snapshot_group_header(report.group_by, &options, color));
            for (index, row) in report.rows[row_range.clone()].iter().enumerate() {
                let absolute_index = row_range.start + index;
                let mut cells = Vec::new();
                push_selection_cell(&mut cells, selected_row, absolute_index, color);
                cells.push(group_cell(&row.display_name, report.group_by));
                if options.columns.process_count {
                    cells.push(cell(row.process_count.to_string()));
                }
                if options.columns.users {
                    cells.push(truncated_cell(
                        row.users.as_deref().unwrap_or("unknown"),
                        USER_DISPLAY_MAX_CHARS,
                    ));
                }
                push_metric_cells(&mut cells, row, report, raw_bytes, color, options.columns);
                if options.columns.top_process {
                    cells.push(truncated_cell(
                        row.top_process.as_deref().unwrap_or("n/a"),
                        TOP_PROCESS_MAX_CHARS,
                    ));
                }
                table.add_row(cells);
            }
        }
        GroupBy::User => {
            table.set_header(snapshot_user_header(&options, color));
            for (index, row) in report.rows[row_range.clone()].iter().enumerate() {
                let absolute_index = row_range.start + index;
                let mut cells = Vec::new();
                push_selection_cell(&mut cells, selected_row, absolute_index, color);
                cells.push(truncated_cell(&row.display_name, USER_DISPLAY_MAX_CHARS));
                if options.columns.process_count {
                    cells.push(cell(row.process_count.to_string()));
                }
                push_metric_cells(&mut cells, row, report, raw_bytes, color, options.columns);
                if options.columns.top_process {
                    cells.push(truncated_cell(
                        row.top_process.as_deref().unwrap_or("n/a"),
                        TOP_PROCESS_MAX_CHARS,
                    ));
                }
                table.add_row(cells);
            }
        }
    }
    writeln!(&mut output, "{table}").expect("writing to a string cannot fail");
    output
}

pub fn print_recording(
    report: &RecordingReport,
    raw_bytes: bool,
    timeline_limit: usize,
    color: bool,
) {
    println!("rescope report");
    println!(
        "started: {}",
        humantime::format_rfc3339_seconds(report.started_at)
    );
    println!(
        "ended:   {}",
        humantime::format_rfc3339_seconds(report.ended_at)
    );
    println!(
        "duration: {} | interval: {} | samples: {} | group: {:?} | sort: {:?}",
        humantime::format_duration(report.duration),
        humantime::format_duration(report.interval),
        report.sample_count,
        report.group_by,
        report.sort_by
    );
    println!("filters: {}", describe_filters(report));
    println!();

    if report.rows.is_empty() {
        println!("no matching processes");
    } else {
        print_recording_table(report, raw_bytes, color);
        if timeline_limit > 0 {
            print_ram_timeline(report, raw_bytes, timeline_limit);
        }
    }

    if !report.notes.is_empty() {
        println!();
        println!("notes:");
        for note in &report.notes {
            println!("- {note}");
        }
    }
}

fn print_recording_table(report: &RecordingReport, raw_bytes: bool, color: bool) {
    let mut table = plain_table();
    match report.group_by {
        GroupBy::Process => {
            table.set_header(vec![
                "PID",
                "USER",
                "PROCESS",
                "CPU avg",
                "CPU max",
                "CPU-s",
                "RAM start",
                "RAM end",
                "RAM max",
                "RAM Δ",
                "READ",
                "WRITE",
                "AVG I/O",
                "first",
                "last",
                "STATUS",
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    cell(row.pid.map(|pid| pid.to_string()).unwrap_or_default()),
                    truncated_cell(
                        row.user_name.as_deref().unwrap_or("unknown"),
                        USER_DISPLAY_MAX_CHARS,
                    ),
                    truncated_cell(&row.display_name, PROCESS_DISPLAY_MAX_CHARS),
                    cpu_avg(row, report, color),
                    cpu_max(row, report, color),
                    cell(format!("{:.1}", row.cpu_core_seconds)),
                    cell(format_bytes(row.ram_start_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_end_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_max_bytes, raw_bytes)),
                    signed_bytes_cell(row.ram_delta_bytes, raw_bytes, color),
                    cell(format_bytes(row.disk_read_total_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_total_bytes, raw_bytes)),
                    cell(format_bps(row.io_bytes_per_second_avg, raw_bytes)),
                    cell(humantime::format_rfc3339_seconds(row.first_seen).to_string()),
                    cell(humantime::format_rfc3339_seconds(row.last_seen).to_string()),
                    lifecycle_cell(&row.lifecycle_status),
                ]);
            }
        }
        GroupBy::Name | GroupBy::Command | GroupBy::Executable | GroupBy::Parent => {
            table.set_header(vec![
                group_label(report.group_by),
                "PROCS",
                "USERS",
                "CPU avg",
                "CPU max",
                "CPU-s",
                "RAM start",
                "RAM end",
                "RAM max",
                "RAM Δ",
                "READ",
                "WRITE",
                "AVG I/O",
                "STATUS",
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    group_cell(&row.display_name, report.group_by),
                    cell(row.process_count.to_string()),
                    truncated_cell(
                        row.users.as_deref().unwrap_or("unknown"),
                        USER_DISPLAY_MAX_CHARS,
                    ),
                    cpu_avg(row, report, color),
                    cpu_max(row, report, color),
                    cell(format!("{:.1}", row.cpu_core_seconds)),
                    cell(format_bytes(row.ram_start_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_end_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_max_bytes, raw_bytes)),
                    signed_bytes_cell(row.ram_delta_bytes, raw_bytes, color),
                    cell(format_bytes(row.disk_read_total_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_total_bytes, raw_bytes)),
                    cell(format_bps(row.io_bytes_per_second_avg, raw_bytes)),
                    lifecycle_cell(&row.lifecycle_status),
                ]);
            }
        }
        GroupBy::User => {
            table.set_header(vec![
                "USER",
                "PROCS",
                "CPU avg",
                "CPU max",
                "CPU-s",
                "RAM start",
                "RAM end",
                "RAM max",
                "RAM Δ",
                "READ",
                "WRITE",
                "AVG I/O",
                "TOP",
                "STATUS",
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    truncated_cell(&row.display_name, USER_DISPLAY_MAX_CHARS),
                    cell(row.process_count.to_string()),
                    cpu_avg(row, report, color),
                    cpu_max(row, report, color),
                    cell(format!("{:.1}", row.cpu_core_seconds)),
                    cell(format_bytes(row.ram_start_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_end_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_max_bytes, raw_bytes)),
                    signed_bytes_cell(row.ram_delta_bytes, raw_bytes, color),
                    cell(format_bytes(row.disk_read_total_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_total_bytes, raw_bytes)),
                    cell(format_bps(row.io_bytes_per_second_avg, raw_bytes)),
                    truncated_cell(
                        row.top_process.as_deref().unwrap_or("n/a"),
                        TOP_PROCESS_MAX_CHARS,
                    ),
                    lifecycle_cell(&row.lifecycle_status),
                ]);
            }
        }
    }

    println!("{table}");
}

fn snapshot_process_header(options: &SnapshotRenderOptions, color: bool) -> Vec<Cell> {
    let mut header = Vec::new();
    push_selection_header(&mut header, options.selected_row, color);
    if options.columns.pid {
        header.push(header_cell("PID", color));
    }
    if options.columns.user {
        header.push(header_cell("USER", color));
    }
    header.push(header_cell("PROCESS", color));
    push_metric_header(&mut header, options.columns, color);
    header
}

fn snapshot_group_header(
    group_by: GroupBy,
    options: &SnapshotRenderOptions,
    color: bool,
) -> Vec<Cell> {
    let mut header = Vec::new();
    push_selection_header(&mut header, options.selected_row, color);
    header.push(header_cell(group_label(group_by), color));
    if options.columns.process_count {
        header.push(header_cell("PROCS", color));
    }
    if options.columns.users {
        header.push(header_cell("USERS", color));
    }
    push_metric_header(&mut header, options.columns, color);
    if options.columns.top_process {
        header.push(header_cell("TOP", color));
    }
    header
}

fn snapshot_user_header(options: &SnapshotRenderOptions, color: bool) -> Vec<Cell> {
    let mut header = Vec::new();
    push_selection_header(&mut header, options.selected_row, color);
    header.push(header_cell("USER", color));
    if options.columns.process_count {
        header.push(header_cell("PROCS", color));
    }
    push_metric_header(&mut header, options.columns, color);
    if options.columns.top_process {
        header.push(header_cell("TOP", color));
    }
    header
}

fn push_selection_header(header: &mut Vec<Cell>, selected_row: Option<usize>, color: bool) {
    if selected_row.is_some() {
        header.push(header_cell("", color));
    }
}

fn push_selection_cell(
    cells: &mut Vec<Cell>,
    selected_row: Option<usize>,
    row_index: usize,
    color: bool,
) {
    if let Some(selected_row) = selected_row {
        let mut marker = cell(if selected_row == row_index { ">" } else { "" });
        if color && selected_row == row_index {
            marker = marker.fg(Color::Green);
        }
        cells.push(marker);
    }
}

fn push_metric_header(header: &mut Vec<Cell>, columns: SnapshotColumns, color: bool) {
    if columns.cpu {
        header.push(header_cell("CPU%", color));
    }
    if columns.ram {
        header.push(header_cell("RAM", color));
    }
    if columns.rates {
        header.push(header_cell("READ/s", color));
        header.push(header_cell("WRITE/s", color));
    }
    if columns.totals {
        header.push(header_cell("READ", color));
        header.push(header_cell("WRITE", color));
    }
}

fn push_metric_cells(
    cells: &mut Vec<Cell>,
    row: &rescope_core::SnapshotRow,
    report: &SnapshotReport,
    raw_bytes: bool,
    color: bool,
    columns: SnapshotColumns,
) {
    if columns.cpu {
        cells.push(cpu_percent_cell(
            row.cpu_percent,
            report.logical_cpu_count,
            report.cpu_normalized,
            color,
        ));
    }
    if columns.ram {
        cells.push(cell(format_bytes(row.ram_bytes, raw_bytes)));
    }
    if columns.rates {
        cells.push(cell(format_bps(row.read_bps, raw_bytes)));
        cells.push(cell(format_bps(row.write_bps, raw_bytes)));
    }
    if columns.totals {
        cells.push(cell(format_bytes(row.disk_read_delta_bytes, raw_bytes)));
        cells.push(cell(format_bytes(row.disk_write_delta_bytes, raw_bytes)));
    }
}

fn visible_row_range(
    row_count: usize,
    row_offset: usize,
    max_rows: Option<usize>,
) -> std::ops::Range<usize> {
    let start = row_offset.min(row_count);
    let end = max_rows
        .map(|limit| start.saturating_add(limit).min(row_count))
        .unwrap_or(row_count);
    start..end
}

fn print_ram_timeline(report: &RecordingReport, raw_bytes: bool, timeline_limit: usize) {
    let mut rows = report.rows.clone();
    rows.sort_by_key(|row| Reverse(row.ram_max_bytes));
    rows.truncate(timeline_limit);

    if rows.is_empty() {
        return;
    }

    println!();
    println!("RAM timeline, top {} by RAM max:", rows.len());
    for row in rows {
        let values = row
            .ram_timeline
            .iter()
            .map(|(_, memory)| *memory)
            .collect::<Vec<_>>();
        let start = values.first().copied().unwrap_or(0);
        let end = values.last().copied().unwrap_or(0);
        let display_name = truncate_for_table(&row.display_name, TIMELINE_DISPLAY_MAX_CHARS);
        println!(
            "{:<20} {:>10} {} {:>10}",
            display_name,
            format_bytes(start, raw_bytes),
            sparkline::render(&values, 40),
            format_bytes(end, raw_bytes)
        );
    }
}

fn describe_filters(report: &RecordingReport) -> String {
    let filters = &report.filters;
    let mut parts = Vec::new();
    if !filters.pids.is_empty() {
        parts.push(format!("pid={:?}", filters.pids));
    }
    if !filters.users.is_empty() {
        parts.push(format!("user={:?}", filters.users));
    }
    if !filters.names.is_empty() {
        parts.push(format!("name={:?}", filters.names));
    }
    if !filters.name_regexes.is_empty() {
        parts.push(format!("name-regex={:?}", filters.name_regexes));
    }
    if !filters.command_substrings.is_empty() {
        parts.push(format!("cmd={:?}", filters.command_substrings));
    }
    if !filters.command_regexes.is_empty() {
        parts.push(format!("cmd-regex={:?}", filters.command_regexes));
    }
    if let Some(min_cpu) = filters.min_cpu_percent {
        parts.push(format!("min-cpu={min_cpu:.1}%"));
    }
    if let Some(min_ram) = filters.min_ram_bytes {
        parts.push(format!("min-ram={}", format_bytes(min_ram, false)));
    }
    if let Some(min_io) = filters.min_io_delta_bytes {
        parts.push(format!("min-io={}", format_bytes(min_io, false)));
    }
    if filters.invert_match {
        parts.push("invert".to_string());
    }
    if filters.hide_self {
        parts.push("hide-self".to_string());
    }
    if parts.is_empty() {
        "all processes".to_string()
    } else {
        parts.join(", ")
    }
}

fn plain_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(NOTHING)
        .set_content_arrangement(ContentArrangement::Disabled);
    table
}

fn cell(value: impl Into<String>) -> Cell {
    Cell::new(value.into())
}

fn header_cell(value: impl Into<String>, color: bool) -> Cell {
    let cell = cell(value);
    if color { cell.fg(Color::Cyan) } else { cell }
}

fn truncated_cell(value: &str, max_chars: usize) -> Cell {
    cell(truncate_for_table(value, max_chars))
}

fn group_cell(value: &str, group_by: GroupBy) -> Cell {
    let max_chars = match group_by {
        GroupBy::Command => COMMAND_DISPLAY_MAX_CHARS,
        GroupBy::Executable => EXECUTABLE_DISPLAY_MAX_CHARS,
        GroupBy::Parent => PARENT_DISPLAY_MAX_CHARS,
        GroupBy::Name | GroupBy::Process | GroupBy::User => PROCESS_DISPLAY_MAX_CHARS,
    };
    truncated_cell(value, max_chars)
}

fn lifecycle_cell(value: &str) -> Cell {
    match value {
        "observed_full_duration" => cell("full"),
        "started_during_recording" => cell("started"),
        "exited_during_recording" => cell("exited"),
        "started_and_exited_during_recording" => cell("started+exited"),
        _ => truncated_cell(value, TOP_PROCESS_MAX_CHARS),
    }
}

fn truncate_for_table(value: &str, max_chars: usize) -> String {
    if UnicodeWidthStr::width(value) <= max_chars {
        return value.to_string();
    }

    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let target_width = max_chars - 3;
    let mut width = 0;
    let mut truncated = String::new();
    for ch in value.chars() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + char_width > target_width {
            break;
        }
        width += char_width;
        truncated.push(ch);
    }
    truncated.push_str("...");
    truncated
}

fn cpu_avg(row: &AggregateRow, report: &RecordingReport, color: bool) -> Cell {
    cpu_percent_cell(
        row.cpu_avg_percent,
        report.logical_cpu_count,
        report.cpu_normalized,
        color,
    )
}

fn cpu_max(row: &AggregateRow, report: &RecordingReport, color: bool) -> Cell {
    cpu_percent_cell(
        row.cpu_max_percent,
        report.logical_cpu_count,
        report.cpu_normalized,
        color,
    )
}

fn cpu_percent_cell(value: f32, logical_cpu_count: usize, normalized: bool, color: bool) -> Cell {
    let display = if normalized {
        value / logical_cpu_count.max(1) as f32
    } else {
        value
    };
    let mut cell = cell(format!("{display:.1}%"));
    if color {
        cell = if display >= 90.0 {
            cell.fg(Color::Red)
        } else if display >= 60.0 {
            cell.fg(Color::Yellow)
        } else {
            cell
        };
    }
    cell
}

fn signed_bytes_cell(value: i64, raw_bytes: bool, color: bool) -> Cell {
    let mut cell = cell(format_signed_bytes(value, raw_bytes));
    if color {
        cell = if value > 0 {
            cell.fg(Color::Yellow)
        } else if value < 0 {
            cell.fg(Color::Green)
        } else {
            cell
        };
    }
    cell
}

fn group_label(group_by: GroupBy) -> &'static str {
    match group_by {
        GroupBy::Process => "PROCESS",
        GroupBy::Name => "PROCESS",
        GroupBy::User => "USER",
        GroupBy::Command => "COMMAND",
        GroupBy::Executable => "EXECUTABLE",
        GroupBy::Parent => "PARENT",
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use rescope_core::{FilterSpec, GroupKey, SortBy};

    use super::*;

    #[test]
    fn truncate_for_table_preserves_short_values() {
        assert_eq!(truncate_for_table("process", 10), "process");
        assert_eq!(truncate_for_table("process", 7), "process");
    }

    #[test]
    fn truncate_for_table_shortens_long_values() {
        assert_eq!(
            truncate_for_table("abcdefghijklmnopqrstuvwxyz", 10),
            "abcdefg..."
        );
        assert_eq!(truncate_for_table("abcdef", 3), "...");
    }

    #[test]
    fn truncate_for_table_accounts_for_wide_unicode() {
        assert_eq!(truncate_for_table("界界界界界", 7), "界界...");
    }

    #[test]
    fn render_snapshot_with_selection_marks_visible_row() {
        let report = SnapshotReport {
            started_at: SystemTime::UNIX_EPOCH,
            ended_at: SystemTime::UNIX_EPOCH,
            duration: Duration::from_secs(1),
            interval: Duration::from_secs(1),
            sample_count: 1,
            group_by: GroupBy::Process,
            sort_by: SortBy::Cpu,
            filters: FilterSpec::default(),
            total_memory_bytes: 1024,
            available_memory_bytes: 512,
            global_cpu_percent: 0.0,
            process_total: 2,
            logical_cpu_count: 1,
            cpu_normalized: false,
            rows: vec![snapshot_row(1, "alpha"), snapshot_row(2, "beta")],
            notes: Vec::new(),
        };

        let rendered = render_snapshot_with_options(
            &report,
            false,
            false,
            SnapshotRenderOptions {
                selected_row: Some(1),
                row_offset: 1,
                max_rows: Some(1),
                ..SnapshotRenderOptions::default()
            },
        );

        assert!(rendered.contains(">"));
        assert!(rendered.contains("beta"));
        assert!(!rendered.contains("alpha"));
    }

    fn snapshot_row(pid: u32, name: &str) -> rescope_core::SnapshotRow {
        rescope_core::SnapshotRow {
            key: GroupKey::Name(name.to_string()),
            group_type: GroupBy::Process,
            display_name: name.to_string(),
            pid: Some(pid),
            user_name: Some("alice".to_string()),
            users: None,
            process_count: 1,
            cpu_percent: 1.0,
            ram_bytes: 64,
            virtual_ram_bytes: 64,
            disk_read_delta_bytes: 0,
            disk_write_delta_bytes: 0,
            disk_io_delta_bytes: 0,
            read_bps: 0.0,
            write_bps: 0.0,
            io_bps: 0.0,
            top_process: None,
            timestamp: SystemTime::UNIX_EPOCH,
        }
    }
}
