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

pub fn format_sort_picker(selected_index: usize, current_sort: SortBy) -> String {
    let mut output = String::new();
    output.push_str("Sort by\n");
    for (index, sort_by) in SORT_OPTIONS.iter().copied().enumerate() {
        let marker = if index == selected_index { ">" } else { " " };
        let active = if sort_by == current_sort {
            " current"
        } else {
            ""
        };
        writeln!(&mut output, "{marker} {}{active}", sort_label(sort_by))
            .expect("writing to a string cannot fail");
    }
    output.push('\n');
    output
}

pub fn format_footer(sort_picker_open: bool) -> String {
    if sort_picker_open {
        "\nup/down choose | Enter apply | Esc close | q quit\n".to_string()
    } else {
        "\ns sort | q / Esc / Ctrl-C quit | --plain uses non-interactive refresh\n".to_string()
    }
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
