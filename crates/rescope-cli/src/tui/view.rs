use std::fmt::Write as _;

use rescope_core::{SnapshotReport, SortBy, format_bytes};

pub const SORT_OPTIONS: [SortBy; 8] = [
    SortBy::Cpu,
    SortBy::Ram,
    SortBy::Io,
    SortBy::Read,
    SortBy::Write,
    SortBy::Pid,
    SortBy::Name,
    SortBy::User,
];

pub fn format_header(report: &SnapshotReport, raw_bytes: bool, tick_count: u64) -> String {
    let used = report
        .total_memory_bytes
        .saturating_sub(report.available_memory_bytes);
    let mut output = String::new();
    writeln!(
        &mut output,
        "rescope live | CPU {:.1}% | RAM {} / {} | processes {} | interval {} | sort {} | refresh #{}",
        report.global_cpu_percent,
        format_bytes(used, raw_bytes),
        format_bytes(report.total_memory_bytes, raw_bytes),
        report.process_total,
        humantime::format_duration(report.interval),
        sort_label(report.sort_by),
        tick_count
    )
    .expect("writing to a string cannot fail");
    output.push('\n');
    output
}

pub fn sort_label(sort_by: SortBy) -> &'static str {
    match sort_by {
        SortBy::Cpu => "cpu",
        SortBy::Ram => "ram",
        SortBy::Read => "read",
        SortBy::Write => "write",
        SortBy::Io => "io",
        SortBy::Pid => "pid",
        SortBy::Name => "name",
        SortBy::User => "user",
    }
}
