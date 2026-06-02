use std::io;
use std::path::Path;

use anyhow::Result;
use rescope_core::{RecordingReport, SnapshotReport, metrics::system_time_ms};

pub fn write_snapshot(path: &Path, report: &SnapshotReport) -> Result<()> {
    if path == Path::new("-") {
        let stdout = io::stdout();
        let mut writer = csv::Writer::from_writer(stdout.lock());
        write_snapshot_rows(&mut writer, report)?;
        writer.flush()?;
        return Ok(());
    }

    let mut temp_file = temp_file_for(path)?;
    {
        let mut writer = csv::Writer::from_writer(temp_file.as_file_mut());
        write_snapshot_rows(&mut writer, report)?;
        writer.flush()?;
    }
    temp_file.as_file_mut().sync_all()?;
    temp_file.persist(path)?;
    Ok(())
}

pub fn writes_stdout(path: &Option<std::path::PathBuf>) -> bool {
    path.as_deref() == Some(Path::new("-"))
}

fn write_snapshot_rows<W: io::Write>(
    writer: &mut csv::Writer<W>,
    report: &SnapshotReport,
) -> Result<()> {
    writer.write_record([
        "group_type",
        "display_name",
        "pid",
        "user_name",
        "process_count",
        "cpu_percent",
        "cpu_normalized_percent",
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
            normalized_cpu(row.cpu_percent, report.logical_cpu_count).to_string(),
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

    Ok(())
}

pub fn write_recording(path: &Path, report: &RecordingReport) -> Result<()> {
    if path == Path::new("-") {
        let stdout = io::stdout();
        let mut writer = csv::Writer::from_writer(stdout.lock());
        write_recording_rows(&mut writer, report)?;
        writer.flush()?;
        return Ok(());
    }

    let mut temp_file = temp_file_for(path)?;
    {
        let mut writer = csv::Writer::from_writer(temp_file.as_file_mut());
        write_recording_rows(&mut writer, report)?;
        writer.flush()?;
    }
    temp_file.as_file_mut().sync_all()?;
    temp_file.persist(path)?;
    Ok(())
}

fn write_recording_rows<W: io::Write>(
    writer: &mut csv::Writer<W>,
    report: &RecordingReport,
) -> Result<()> {
    writer.write_record([
        "group_type",
        "display_name",
        "pid",
        "user_name",
        "process_count",
        "cpu_avg_percent",
        "cpu_avg_normalized_percent",
        "cpu_max_percent",
        "cpu_max_normalized_percent",
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
        "lifecycle_status",
    ])?;

    for row in &report.rows {
        writer.write_record([
            format!("{:?}", row.group_type).to_ascii_lowercase(),
            row.display_name.clone(),
            option_u32(row.pid),
            row.user_name.clone().unwrap_or_default(),
            row.process_count.to_string(),
            row.cpu_avg_percent.to_string(),
            normalized_cpu(row.cpu_avg_percent, report.logical_cpu_count).to_string(),
            row.cpu_max_percent.to_string(),
            normalized_cpu(row.cpu_max_percent, report.logical_cpu_count).to_string(),
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
            row.lifecycle_status.clone(),
        ])?;
    }

    Ok(())
}

fn temp_file_for(path: &Path) -> Result<tempfile::NamedTempFile> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let temp_file = match parent {
        Some(parent) => tempfile::NamedTempFile::new_in(parent)?,
        None => tempfile::NamedTempFile::new_in(".")?,
    };
    Ok(temp_file)
}

fn option_u32(value: Option<u32>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn normalized_cpu(value: f32, logical_cpu_count: usize) -> f32 {
    value / logical_cpu_count.max(1) as f32
}
