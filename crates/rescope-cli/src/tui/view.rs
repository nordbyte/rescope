use rescope_core::{SnapshotReport, SortBy, format_bytes};

pub fn render_header(report: &SnapshotReport, raw_bytes: bool, tick_count: u64) {
    let used = report
        .total_memory_bytes
        .saturating_sub(report.available_memory_bytes);
    println!(
        "rescope live | CPU {:.1}% | RAM {} / {} | processes {} | interval {} | sort {} | refresh #{}",
        report.global_cpu_percent,
        format_bytes(used, raw_bytes),
        format_bytes(report.total_memory_bytes, raw_bytes),
        report.process_total,
        humantime::format_duration(report.interval),
        sort_label(report.sort_by),
        tick_count
    );
    println!();
}

pub fn render_footer() {
    println!();
    println!("sort: c cpu | m ram | i io | r read | w write | p pid | n name | u user");
    println!("q / Esc / Ctrl-C quit | --plain uses non-interactive refresh");
}

fn sort_label(sort_by: SortBy) -> &'static str {
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
