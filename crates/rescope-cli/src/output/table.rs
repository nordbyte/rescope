use comfy_table::{Cell, ContentArrangement, Table, presets::NOTHING};
use std::cmp::Reverse;

use rescope_core::{
    AggregateRow, GroupBy, RecordingReport, SnapshotReport, format_bps, format_bytes,
    format_signed_bytes,
};

use crate::output::sparkline;

pub fn print_snapshot(report: &SnapshotReport, raw_bytes: bool, show_system: bool) {
    if show_system {
        let used = report
            .total_memory_bytes
            .saturating_sub(report.available_memory_bytes);
        println!(
            "System: CPU {:.1}% | RAM {} / {} | processes {} | interval {}",
            report.global_cpu_percent,
            format_bytes(used, raw_bytes),
            format_bytes(report.total_memory_bytes, raw_bytes),
            report.process_total,
            humantime::format_duration(report.interval)
        );
        println!();
    }

    if report.rows.is_empty() {
        println!("no matching processes");
        return;
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
                    cell(row.user_name.as_deref().unwrap_or("unknown")),
                    cell(&row.display_name),
                    cell(format!("{:.1}", row.cpu_percent)),
                    cell(format_bytes(row.ram_bytes, raw_bytes)),
                    cell(format_bps(row.read_bps, raw_bytes)),
                    cell(format_bps(row.write_bps, raw_bytes)),
                    cell(format_bytes(row.disk_read_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_delta_bytes, raw_bytes)),
                ]);
            }
        }
        GroupBy::Name => {
            table.set_header(vec![
                "PROCESS", "PROCS", "USERS", "CPU%", "RAM", "READ/s", "WRITE/s", "READ", "WRITE",
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    cell(&row.display_name),
                    cell(row.process_count.to_string()),
                    cell(row.users.as_deref().unwrap_or("unknown")),
                    cell(format!("{:.1}", row.cpu_percent)),
                    cell(format_bytes(row.ram_bytes, raw_bytes)),
                    cell(format_bps(row.read_bps, raw_bytes)),
                    cell(format_bps(row.write_bps, raw_bytes)),
                    cell(format_bytes(row.disk_read_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_delta_bytes, raw_bytes)),
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
                    cell(&row.display_name),
                    cell(row.process_count.to_string()),
                    cell(format!("{:.1}", row.cpu_percent)),
                    cell(format_bytes(row.ram_bytes, raw_bytes)),
                    cell(format_bps(row.read_bps, raw_bytes)),
                    cell(format_bps(row.write_bps, raw_bytes)),
                    cell(format_bytes(row.disk_read_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_delta_bytes, raw_bytes)),
                    cell(row.top_process.as_deref().unwrap_or("n/a")),
                ]);
            }
        }
    }
    println!("{table}");
}

pub fn print_recording(report: &RecordingReport, raw_bytes: bool, timeline_limit: usize) {
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
        print_recording_table(report, raw_bytes);
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

fn print_recording_table(report: &RecordingReport, raw_bytes: bool) {
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
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    cell(row.pid.map(|pid| pid.to_string()).unwrap_or_default()),
                    cell(row.user_name.as_deref().unwrap_or("unknown")),
                    cell(&row.display_name),
                    cpu_avg(row),
                    cpu_max(row),
                    cell(format!("{:.1}", row.cpu_core_seconds)),
                    cell(format_bytes(row.ram_start_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_end_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_max_bytes, raw_bytes)),
                    cell(format_signed_bytes(row.ram_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_read_total_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_total_bytes, raw_bytes)),
                    cell(format_bps(row.io_bytes_per_second_avg, raw_bytes)),
                    cell(humantime::format_rfc3339_seconds(row.first_seen).to_string()),
                    cell(humantime::format_rfc3339_seconds(row.last_seen).to_string()),
                ]);
            }
        }
        GroupBy::Name => {
            table.set_header(vec![
                "PROCESS",
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
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    cell(&row.display_name),
                    cell(row.process_count.to_string()),
                    cell(row.users.as_deref().unwrap_or("unknown")),
                    cpu_avg(row),
                    cpu_max(row),
                    cell(format!("{:.1}", row.cpu_core_seconds)),
                    cell(format_bytes(row.ram_start_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_end_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_max_bytes, raw_bytes)),
                    cell(format_signed_bytes(row.ram_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_read_total_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_total_bytes, raw_bytes)),
                    cell(format_bps(row.io_bytes_per_second_avg, raw_bytes)),
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
            ]);
            for row in &report.rows {
                table.add_row(vec![
                    cell(&row.display_name),
                    cell(row.process_count.to_string()),
                    cpu_avg(row),
                    cpu_max(row),
                    cell(format!("{:.1}", row.cpu_core_seconds)),
                    cell(format_bytes(row.ram_start_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_end_bytes, raw_bytes)),
                    cell(format_bytes(row.ram_max_bytes, raw_bytes)),
                    cell(format_signed_bytes(row.ram_delta_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_read_total_bytes, raw_bytes)),
                    cell(format_bytes(row.disk_write_total_bytes, raw_bytes)),
                    cell(format_bps(row.io_bytes_per_second_avg, raw_bytes)),
                    cell(row.top_process.as_deref().unwrap_or("n/a")),
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
        println!(
            "{:<20} {:>10} {} {:>10}",
            row.display_name,
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

fn cpu_avg(row: &AggregateRow) -> Cell {
    cell(format!("{:.1}%", row.cpu_avg_percent))
}

fn cpu_max(row: &AggregateRow) -> Cell {
    cell(format!("{:.1}%", row.cpu_max_percent))
}
