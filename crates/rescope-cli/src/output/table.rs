use comfy_table::{Cell, Color, ContentArrangement, Table, presets::NOTHING};
use std::cmp::Reverse;
use std::fmt::Write as _;

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

pub fn print_snapshot(report: &SnapshotReport, raw_bytes: bool, show_system: bool, color: bool) {
    print!("{}", render_snapshot(report, raw_bytes, show_system, color));
}

pub fn render_snapshot(
    report: &SnapshotReport,
    raw_bytes: bool,
    show_system: bool,
    color: bool,
) -> String {
    let mut output = String::new();

    if show_system {
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
    match report.group_by {
        GroupBy::Process => {
            table.set_header(vec![
                "PID", "USER", "PROCESS", "CPU%", "RAM", "READ/s", "WRITE/s", "READ", "WRITE",
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    cell(row.pid.map(|pid| pid.to_string()).unwrap_or_default()),
                    truncated_cell(
                        row.user_name.as_deref().unwrap_or("unknown"),
                        USER_DISPLAY_MAX_CHARS,
                    ),
                    truncated_cell(&row.display_name, PROCESS_DISPLAY_MAX_CHARS),
                    cpu_percent_cell(
                        row.cpu_percent,
                        report.logical_cpu_count,
                        report.cpu_normalized,
                        color,
                    ),
                    cell(format_bytes(row.ram_bytes, raw_bytes)),
                    cell(format_bps(row.read_bps, raw_bytes)),
                    cell(format_bps(row.write_bps, raw_bytes)),
                    cell(format_bytes(row.disk_read_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_delta_bytes, raw_bytes)),
                ]);
            }
        }
        GroupBy::Name | GroupBy::Command | GroupBy::Executable | GroupBy::Parent => {
            table.set_header(vec![
                group_label(report.group_by),
                "PROCS",
                "USERS",
                "CPU%",
                "RAM",
                "READ/s",
                "WRITE/s",
                "READ",
                "WRITE",
                "TOP_PROCESS",
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    group_cell(&row.display_name, report.group_by),
                    cell(row.process_count.to_string()),
                    truncated_cell(
                        row.users.as_deref().unwrap_or("unknown"),
                        USER_DISPLAY_MAX_CHARS,
                    ),
                    cpu_percent_cell(
                        row.cpu_percent,
                        report.logical_cpu_count,
                        report.cpu_normalized,
                        color,
                    ),
                    cell(format_bytes(row.ram_bytes, raw_bytes)),
                    cell(format_bps(row.read_bps, raw_bytes)),
                    cell(format_bps(row.write_bps, raw_bytes)),
                    cell(format_bytes(row.disk_read_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_delta_bytes, raw_bytes)),
                    truncated_cell(
                        row.top_process.as_deref().unwrap_or("n/a"),
                        TOP_PROCESS_MAX_CHARS,
                    ),
                ]);
            }
        }
        GroupBy::User => {
            table.set_header(vec![
                "USER",
                "PROCS",
                "CPU%",
                "RAM",
                "READ/s",
                "WRITE/s",
                "READ",
                "WRITE",
                "TOP_PROCESS",
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    truncated_cell(&row.display_name, USER_DISPLAY_MAX_CHARS),
                    cell(row.process_count.to_string()),
                    cpu_percent_cell(
                        row.cpu_percent,
                        report.logical_cpu_count,
                        report.cpu_normalized,
                        color,
                    ),
                    cell(format_bytes(row.ram_bytes, raw_bytes)),
                    cell(format_bps(row.read_bps, raw_bytes)),
                    cell(format_bps(row.write_bps, raw_bytes)),
                    cell(format_bytes(row.disk_read_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_delta_bytes, raw_bytes)),
                    truncated_cell(
                        row.top_process.as_deref().unwrap_or("n/a"),
                        TOP_PROCESS_MAX_CHARS,
                    ),
                ]);
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
                "TOP_PROCESS",
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
    if !filters.command_substrings.is_empty() {
        parts.push(format!("cmd={:?}", filters.command_substrings));
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
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

fn cell(value: impl Into<String>) -> Cell {
    Cell::new(value.into())
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
    let mut chars = value.chars();
    let mut truncated = String::new();

    for _ in 0..max_chars {
        match chars.next() {
            Some(ch) => truncated.push(ch),
            None => return value.to_string(),
        }
    }

    if chars.next().is_none() {
        return value.to_string();
    }

    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    truncated.truncate(
        truncated
            .char_indices()
            .nth(max_chars - 3)
            .map(|(idx, _)| idx)
            .unwrap_or(truncated.len()),
    );
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
    use super::truncate_for_table;

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
}
