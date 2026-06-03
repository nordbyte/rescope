use std::time::{Duration, SystemTime};

use crate::aggregate::{
    RecordingAccumulator, RecordingAggregateOptions, aggregate_recording, aggregate_snapshot,
};
use crate::metrics::{FilterSpec, GroupBy, RecordingReport, SnapshotReport, SortBy, SystemSample};

#[derive(Debug, Clone)]
pub struct SnapshotReportOptions {
    pub interval: Duration,
    pub group_by: GroupBy,
    pub sort_by: SortBy,
    pub filters: FilterSpec,
    pub show_command: bool,
    pub show_path: bool,
    pub limit: usize,
    pub normalize_cpu: bool,
}

#[derive(Debug, Clone)]
pub struct RecordingReportOptions {
    pub requested_duration: Duration,
    pub interval: Duration,
    pub group_by: GroupBy,
    pub sort_by: SortBy,
    pub filters: FilterSpec,
    pub show_command: bool,
    pub show_path: bool,
    pub limit: usize,
    pub include_idle: bool,
    pub normalize_cpu: bool,
}

pub fn build_snapshot_report(
    sample: &SystemSample,
    options: SnapshotReportOptions,
) -> SnapshotReport {
    let rows = aggregate_snapshot(
        sample,
        options.group_by,
        options.sort_by,
        options.interval,
        options.show_command,
        options.show_path,
        options.limit,
    );
    SnapshotReport {
        started_at: sample.timestamp,
        ended_at: sample.timestamp,
        duration: options.interval,
        interval: options.interval,
        sample_count: 1,
        group_by: options.group_by,
        sort_by: options.sort_by,
        filters: options.filters,
        total_memory_bytes: sample.total_memory_bytes,
        available_memory_bytes: sample.available_memory_bytes,
        global_cpu_percent: sample.global_cpu_percent,
        process_total: sample.processes.len(),
        logical_cpu_count: sample.logical_cpu_count,
        cpu_normalized: options.normalize_cpu,
        show_path: options.show_path,
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
            show_path: options.show_path,
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
        logical_cpu_count: samples
            .iter()
            .map(|sample| sample.logical_cpu_count)
            .max()
            .unwrap_or(1),
        cpu_normalized: options.normalize_cpu,
        show_path: options.show_path,
        rows,
        notes: platform_notes(),
    }
}

pub fn build_recording_report_from_accumulator(
    accumulator: RecordingAccumulator,
    options: RecordingReportOptions,
) -> RecordingReport {
    let started_at = accumulator.started_at().unwrap_or_else(SystemTime::now);
    let ended_at = accumulator.ended_at().unwrap_or(started_at);
    let sample_count = accumulator.sample_count();
    let logical_cpu_count = accumulator.logical_cpu_count();
    let rows = accumulator.into_rows(options.requested_duration, options.limit);

    RecordingReport {
        started_at,
        ended_at,
        duration: options.requested_duration,
        interval: options.interval,
        sample_count,
        group_by: options.group_by,
        sort_by: options.sort_by,
        filters: options.filters,
        logical_cpu_count,
        cpu_normalized: options.normalize_cpu,
        show_path: options.show_path,
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
        "Recording percentiles are approximate for very long runs because timelines are bounded."
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
