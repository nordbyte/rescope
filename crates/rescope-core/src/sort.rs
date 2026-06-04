use std::cmp::Ordering;

use crate::metrics::{AggregateRow, SnapshotRow, SortBy};

pub fn sort_snapshot_rows(rows: &mut [SnapshotRow], sort_by: SortBy) {
    rows.sort_by(|left, right| snapshot_cmp(left, right, sort_by));
}

pub fn sort_recording_rows(rows: &mut [AggregateRow], sort_by: SortBy) {
    rows.sort_by(|left, right| recording_cmp(left, right, sort_by));
}

pub fn sort_snapshot_rows_limit(rows: &mut Vec<SnapshotRow>, sort_by: SortBy, limit: usize) {
    if limit == 0 {
        rows.clear();
        return;
    }
    if limit < rows.len() {
        rows.select_nth_unstable_by(limit - 1, |left, right| snapshot_cmp(left, right, sort_by));
        rows.truncate(limit);
    }
    sort_snapshot_rows(rows, sort_by);
}

pub fn sort_recording_rows_limit(rows: &mut Vec<AggregateRow>, sort_by: SortBy, limit: usize) {
    if limit == 0 {
        rows.clear();
        return;
    }
    if limit < rows.len() {
        rows.select_nth_unstable_by(limit - 1, |left, right| recording_cmp(left, right, sort_by));
        rows.truncate(limit);
    }
    sort_recording_rows(rows, sort_by);
}

fn snapshot_cmp(left: &SnapshotRow, right: &SnapshotRow, sort_by: SortBy) -> Ordering {
    match sort_by {
        SortBy::Cpu | SortBy::CpuMax | SortBy::CpuP95 => {
            desc_f32(left.cpu_percent, right.cpu_percent)
        }
        SortBy::Ram | SortBy::RamAvg | SortBy::RamEnd => right.ram_bytes.cmp(&left.ram_bytes),
        SortBy::Read => right.disk_read_delta_bytes.cmp(&left.disk_read_delta_bytes),
        SortBy::Write => right
            .disk_write_delta_bytes
            .cmp(&left.disk_write_delta_bytes),
        SortBy::Io | SortBy::IoAvg => right.disk_io_delta_bytes.cmp(&left.disk_io_delta_bytes),
        SortBy::Pid => left
            .pid
            .unwrap_or(u32::MAX)
            .cmp(&right.pid.unwrap_or(u32::MAX)),
        SortBy::Name => left
            .display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase()),
        SortBy::User => left
            .user_label()
            .to_ascii_lowercase()
            .cmp(&right.user_label().to_ascii_lowercase()),
        SortBy::Started | SortBy::Exited => right.process_count.cmp(&left.process_count),
    }
    .then_with(|| left.display_name.cmp(&right.display_name))
}

fn recording_cmp(left: &AggregateRow, right: &AggregateRow, sort_by: SortBy) -> Ordering {
    match sort_by {
        SortBy::Cpu => desc_f32(left.cpu_avg_percent, right.cpu_avg_percent),
        SortBy::CpuMax => desc_f32(left.cpu_max_percent, right.cpu_max_percent),
        SortBy::CpuP95 => desc_f32(left.cpu_p95_percent, right.cpu_p95_percent),
        SortBy::Ram => right.ram_max_bytes.cmp(&left.ram_max_bytes),
        SortBy::RamAvg => right.ram_avg_bytes.cmp(&left.ram_avg_bytes),
        SortBy::RamEnd => right.ram_end_bytes.cmp(&left.ram_end_bytes),
        SortBy::Read => right.disk_read_total_bytes.cmp(&left.disk_read_total_bytes),
        SortBy::Write => right
            .disk_write_total_bytes
            .cmp(&left.disk_write_total_bytes),
        SortBy::Io => right.disk_io_total_bytes.cmp(&left.disk_io_total_bytes),
        SortBy::IoAvg => desc_f64(left.io_bytes_per_second_avg, right.io_bytes_per_second_avg),
        SortBy::Pid => left
            .pid
            .unwrap_or(u32::MAX)
            .cmp(&right.pid.unwrap_or(u32::MAX)),
        SortBy::Name => left
            .display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase()),
        SortBy::User => left
            .user_label()
            .to_ascii_lowercase()
            .cmp(&right.user_label().to_ascii_lowercase()),
        SortBy::Started => right.started_count.cmp(&left.started_count),
        SortBy::Exited => right.exited_count.cmp(&left.exited_count),
    }
    .then_with(|| left.display_name.cmp(&right.display_name))
}

fn desc_f32(left: f32, right: f32) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

fn desc_f64(left: f64, right: f64) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

trait SortUserLabel {
    fn user_label(&self) -> &str;
}

impl SortUserLabel for SnapshotRow {
    fn user_label(&self) -> &str {
        self.user_name
            .as_deref()
            .or(self.users.as_deref())
            .unwrap_or(&self.display_name)
    }
}

impl SortUserLabel for AggregateRow {
    fn user_label(&self) -> &str {
        self.user_name
            .as_deref()
            .or(self.users.as_deref())
            .unwrap_or(&self.display_name)
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::metrics::{GroupBy, GroupKey, ProcessDetails, SnapshotRow};

    use super::*;

    fn row(name: &str, cpu: f32, ram: u64) -> SnapshotRow {
        SnapshotRow {
            key: GroupKey::Name(name.to_string()),
            group_type: GroupBy::Name,
            display_name: name.to_string(),
            pid: None,
            user_name: None,
            executable_path: None,
            users: None,
            process_count: 1,
            cpu_percent: cpu,
            ram_bytes: ram,
            virtual_ram_bytes: ram,
            disk_read_delta_bytes: 0,
            disk_write_delta_bytes: 0,
            disk_io_delta_bytes: 0,
            read_bps: 0.0,
            write_bps: 0.0,
            io_bps: 0.0,
            top_process: None,
            details: ProcessDetails::default(),
            timestamp: SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn sorts_snapshot_by_cpu_descending() {
        let mut rows = vec![row("low", 1.0, 100), row("high", 10.0, 10)];
        sort_snapshot_rows(&mut rows, SortBy::Cpu);
        assert_eq!(rows[0].display_name, "high");
    }

    #[test]
    fn sorts_snapshot_by_ram_descending() {
        let mut rows = vec![row("low", 10.0, 100), row("high", 1.0, 1_000)];
        sort_snapshot_rows(&mut rows, SortBy::Ram);
        assert_eq!(rows[0].display_name, "high");
    }

    #[test]
    fn limits_snapshot_rows_to_top_k_without_off_by_one() {
        let mut rows = vec![
            row("one", 1.0, 1),
            row("two", 2.0, 2),
            row("three", 3.0, 3),
            row("four", 4.0, 4),
        ];
        sort_snapshot_rows_limit(&mut rows, SortBy::Cpu, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].display_name, "four");
        assert_eq!(rows[1].display_name, "three");
    }

    #[test]
    fn zero_limit_clears_rows_defensively() {
        let mut rows = vec![row("one", 1.0, 1)];
        sort_snapshot_rows_limit(&mut rows, SortBy::Cpu, 0);
        assert!(rows.is_empty());
    }
}
