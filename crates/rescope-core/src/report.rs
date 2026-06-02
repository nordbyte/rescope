use std::time::{Duration, SystemTime};

use crate::aggregate::{RecordingAggregateOptions, aggregate_recording, aggregate_snapshot};
use crate::metrics::{FilterSpec, GroupBy, RecordingReport, SnapshotReport, SortBy, SystemSample};

#[derive(Debug, Clone)]
pub struct RecordingReportOptions {
    pub requested_duration: Duration,
    pub interval: Duration,
    pub group_by: GroupBy,
    pub sort_by: SortBy,
    pub filters: FilterSpec,
    pub show_command: bool,
    pub limit: usize,
    pub include_idle: bool,
}

pub fn build_snapshot_report(
    sample: &SystemSample,
    interval: Duration,
    group_by: GroupBy,
    sort_by: SortBy,
    filters: FilterSpec,
    show_command: bool,
    limit: usize,
) -> SnapshotReport {
    let rows = aggregate_snapshot(sample, group_by, sort_by, interval, show_command, limit);
    SnapshotReport {
        started_at: sample.timestamp,
        ended_at: sample.timestamp,
        duration: interval,
        interval,
        sample_count: 1,
        group_by,
        sort_by,
        filters,
        total_memory_bytes: sample.total_memory_bytes,
        available_memory_bytes: sample.available_memory_bytes,
        global_cpu_percent: sample.global_cpu_percent,
        process_total: sample.processes.len(),
        rows,
        notes: platform_notes(),
    }
}

pub fn build_recording_report(
    samples: &[SystemSample],
    options: RecordingReportOptions,
) -> RecordingReport {
    let started_at = samples
        .first()
        .map(|sample| sample.timestamp)
        .unwrap_or_else(SystemTime::now);
    let ended_at = samples
        .last()
        .map(|sample| sample.timestamp)
        .unwrap_or(started_at);
    let rows = aggregate_recording(
        samples,
        RecordingAggregateOptions {
            group_by: options.group_by,
            sort_by: options.sort_by,
            interval: options.interval,
            measured_duration: options.requested_duration,
            show_command: options.show_command,
            limit: options.limit,
            include_idle: options.include_idle,
        },
    );

    RecordingReport {
        started_at,
        ended_at,
        duration: options.requested_duration,
        interval: options.interval,
        sample_count: samples.len(),
        group_by: options.group_by,
        sort_by: options.sort_by,
        filters: options.filters,
        rows,
        notes: platform_notes(),
    }
}

pub fn platform_notes() -> Vec<String> {
    let mut notes = vec![
        "CPU% may exceed 100% on multi-core systems.".to_string(),
        "RAM is resident memory when the platform reports it that way.".to_string(),
        "Cached file operations may not increase per-process disk counters on Unix-like systems."
            .to_string(),
    ];

    if cfg!(windows) {
        notes.push(
            "On Windows, per-process I/O may include non-disk I/O depending on OS counters."
                .to_string(),
        );
        notes
            .push("User information may be unavailable for some processes on Windows.".to_string());
    }

    notes
}
