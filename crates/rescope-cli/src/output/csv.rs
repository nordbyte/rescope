use std::fs::File;
use std::path::Path;

use anyhow::Result;
use rescope_core::{RecordingReport, SnapshotReport, metrics::system_time_ms};

pub fn write_snapshot(path: &Path, report: &SnapshotReport) -> Result<()> {
    let mut writer = csv::Writer::from_writer(File::create(path)?);
    writer.write_record([
        "group_type",
        "display_name",
        "pid",
        "user_name",
        "process_count",
        "cpu_percent",
        "ram_bytes",
        "virtual_ram_bytes",
        "disk_read_delta_bytes",
        "disk_write_delta_bytes",
        "disk_io_delta_bytes",
        "read_bps",
        "write_bps",
        "io_bps",
        "timestamp",
    ])?;

    for row in &report.rows {
        writer.write_record([
            format!("{:?}", row.group_type).to_ascii_lowercase(),
            row.display_name.clone(),
            option_u32(row.pid),
            row.user_name.clone().unwrap_or_default(),
            row.process_count.to_string(),
            row.cpu_percent.to_string(),
            row.ram_bytes.to_string(),
            row.virtual_ram_bytes.to_string(),
            row.disk_read_delta_bytes.to_string(),
            row.disk_write_delta_bytes.to_string(),
            row.disk_io_delta_bytes.to_string(),
            row.read_bps.to_string(),
            row.write_bps.to_string(),
            row.io_bps.to_string(),
            system_time_ms(row.timestamp).to_string(),
        ])?;
    }

    writer.flush()?;
    Ok(())
}

pub fn write_recording(path: &Path, report: &RecordingReport) -> Result<()> {
    let mut writer = csv::Writer::from_writer(File::create(path)?);
    writer.write_record([
        "group_type",
        "display_name",
        "pid",
        "user_name",
        "process_count",
        "cpu_avg_percent",
        "cpu_max_percent",
        "cpu_core_seconds",
        "ram_start_bytes",
        "ram_end_bytes",
        "ram_min_bytes",
        "ram_max_bytes",
        "ram_avg_bytes",
        "ram_delta_bytes",
        "disk_read_total_bytes",
        "disk_write_total_bytes",
        "disk_io_total_bytes",
        "read_bps_avg",
        "write_bps_avg",
        "io_bps_avg",
        "first_seen",
        "last_seen",
    ])?;

    for row in &report.rows {
        writer.write_record([
            format!("{:?}", row.group_type).to_ascii_lowercase(),
            row.display_name.clone(),
            option_u32(row.pid),
            row.user_name.clone().unwrap_or_default(),
            row.process_count.to_string(),
            row.cpu_avg_percent.to_string(),
            row.cpu_max_percent.to_string(),
            row.cpu_core_seconds.to_string(),
            row.ram_start_bytes.to_string(),
            row.ram_end_bytes.to_string(),
            row.ram_min_bytes.to_string(),
            row.ram_max_bytes.to_string(),
            row.ram_avg_bytes.to_string(),
            row.ram_delta_bytes.to_string(),
            row.disk_read_total_bytes.to_string(),
            row.disk_write_total_bytes.to_string(),
            row.disk_io_total_bytes.to_string(),
            row.read_bytes_per_second_avg.to_string(),
            row.write_bytes_per_second_avg.to_string(),
            row.io_bytes_per_second_avg.to_string(),
            system_time_ms(row.first_seen).to_string(),
            system_time_ms(row.last_seen).to_string(),
        ])?;
    }

    writer.flush()?;
    Ok(())
}

fn option_u32(value: Option<u32>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}
