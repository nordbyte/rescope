use rescope_core::{SnapshotReport, format_bytes};

pub fn render_header(report: &SnapshotReport, raw_bytes: bool, tick_count: u64) {
    let used = report
        .total_memory_bytes
        .saturating_sub(report.available_memory_bytes);
    println!(
        "rescope live | CPU {:.1}% | RAM {} / {} | processes {} | interval {} | refresh #{}",
        report.global_cpu_percent,
        format_bytes(used, raw_bytes),
        format_bytes(report.total_memory_bytes, raw_bytes),
        report.process_total,
        humantime::format_duration(report.interval),
        tick_count
    );
    println!();
}

pub fn render_footer() {
    println!();
    println!("q / Esc / Ctrl-C quit | --plain uses non-interactive refresh");
}
