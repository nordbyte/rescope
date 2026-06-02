use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, SystemTime};

use crate::metrics::{
    AggregateRow, GroupBy, GroupKey, RawProcessSample, SnapshotRow, SortBy, SystemSample,
};
use crate::sort::{sort_recording_rows_limit, sort_snapshot_rows_limit};

const MAX_TIMELINE_POINTS: usize = 2048;

#[derive(Debug, Clone, Copy)]
pub struct RecordingAggregateOptions {
    pub group_by: GroupBy,
    pub sort_by: SortBy,
    pub interval: Duration,
    pub measured_duration: Duration,
    pub show_command: bool,
    pub limit: usize,
    pub include_idle: bool,
}

pub fn aggregate_snapshot(
    sample: &SystemSample,
    group_by: GroupBy,
    sort_by: SortBy,
    interval: Duration,
    show_command: bool,
    limit: usize,
) -> Vec<SnapshotRow> {
    let interval_seconds = duration_seconds(interval);
    let mut grouped: HashMap<GroupKey, SnapshotAcc> = HashMap::new();

    for process in &sample.processes {
        let key = group_key(process, group_by);
        grouped
            .entry(key.clone())
            .or_insert_with(|| {
                SnapshotAcc::new(key, group_by, process, show_command, sample.timestamp)
            })
            .add(process, sort_by, show_command);
    }

    let mut rows: Vec<SnapshotRow> = grouped
        .into_values()
        .map(|acc| acc.into_row(interval_seconds))
        .collect();
    sort_snapshot_rows_limit(&mut rows, sort_by, limit);
    rows
}

pub fn aggregate_recording(
    samples: &[SystemSample],
    options: RecordingAggregateOptions,
) -> Vec<AggregateRow> {
    let fallback_interval_seconds = duration_seconds(options.interval);
    let measured_seconds =
        duration_seconds(options.measured_duration).max(fallback_interval_seconds);
    let started_at = samples.first().map(|sample| sample.timestamp);
    let ended_at = samples.last().map(|sample| sample.timestamp);
    let mut rows_by_key: HashMap<GroupKey, RecordingAcc> = HashMap::new();

    for sample in samples {
        let sample_interval_seconds = if sample.sample_interval.is_zero() {
            fallback_interval_seconds
        } else {
            duration_seconds(sample.sample_interval)
        };
        let grouped = group_sample_for_recording(
            sample,
            options.group_by,
            options.sort_by,
            options.show_command,
        );
        for (key, group_sample) in grouped {
            rows_by_key
                .entry(key.clone())
                .or_insert_with(|| RecordingAcc::new(key, options.group_by, &group_sample))
                .add(&group_sample, sample_interval_seconds);
        }
    }

    let mut rows: Vec<AggregateRow> = rows_by_key
        .into_values()
        .filter(|acc| options.include_idle || acc.has_activity())
        .map(|acc| acc.into_row(measured_seconds, started_at, ended_at))
        .collect();
    sort_recording_rows_limit(&mut rows, options.sort_by, options.limit);
    rows
}

fn group_sample_for_recording(
    sample: &SystemSample,
    group_by: GroupBy,
    sort_by: SortBy,
    show_command: bool,
) -> HashMap<GroupKey, GroupSample> {
    let mut grouped: HashMap<GroupKey, GroupSample> = HashMap::new();

    for process in &sample.processes {
        let key = group_key(process, group_by);
        grouped
            .entry(key.clone())
            .or_insert_with(|| GroupSample::new(group_by, process, show_command, sample.timestamp))
            .add(process, sort_by, show_command);
    }

    grouped
}

fn group_key(process: &RawProcessSample, group_by: GroupBy) -> GroupKey {
    match group_by {
        GroupBy::Process => GroupKey::Process(process.identity.clone()),
        GroupBy::Name => GroupKey::Name(process.identity.name.clone()),
        GroupBy::User => GroupKey::User(process.user_display()),
        GroupBy::Command => GroupKey::Command(
            process
                .command
                .clone()
                .filter(|command| !command.trim().is_empty())
                .unwrap_or_else(|| process.identity.name.clone()),
        ),
        GroupBy::Executable => GroupKey::Executable(
            process
                .executable
                .clone()
                .filter(|path| !path.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        GroupBy::Parent => GroupKey::Parent(
            process
                .parent_pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        ),
    }
}

fn duration_seconds(duration: Duration) -> f64 {
    duration.as_secs_f64().max(0.001)
}

fn summarize_users(users: &BTreeSet<String>) -> Option<String> {
    match users.len() {
        0 => None,
        1 => users.iter().next().cloned(),
        2 => Some(users.iter().cloned().collect::<Vec<_>>().join(",")),
        len => users
            .iter()
            .next()
            .map(|first| format!("{first},+{}", len - 1)),
    }
}

fn metric_contribution(process: &RawProcessSample, sort_by: SortBy) -> f64 {
    match sort_by {
        SortBy::Cpu => process.cpu_percent as f64,
        SortBy::Ram => process.memory_bytes as f64,
        SortBy::Read => process.disk_read_delta_bytes as f64,
        SortBy::Write => process.disk_write_delta_bytes as f64,
        SortBy::Io => (process.disk_read_delta_bytes + process.disk_write_delta_bytes) as f64,
        SortBy::Pid => process.identity.pid as f64,
        SortBy::Name | SortBy::User => process.cpu_percent as f64,
    }
}

#[derive(Debug)]
struct SnapshotAcc {
    key: GroupKey,
    group_type: GroupBy,
    display_name: String,
    pid: Option<u32>,
    user_name: Option<String>,
    users: BTreeSet<String>,
    process_count: usize,
    cpu_percent: f32,
    ram_bytes: u64,
    virtual_ram_bytes: u64,
    disk_read_delta_bytes: u64,
    disk_write_delta_bytes: u64,
    top_process: Option<(String, f64)>,
    timestamp: SystemTime,
}

impl SnapshotAcc {
    fn new(
        key: GroupKey,
        group_type: GroupBy,
        process: &RawProcessSample,
        show_command: bool,
        timestamp: SystemTime,
    ) -> Self {
        let display_name = match group_type {
            GroupBy::Process => process.display_process(show_command),
            GroupBy::Name => process.identity.name.clone(),
            GroupBy::User => process.user_display(),
            GroupBy::Command => process
                .command
                .clone()
                .filter(|command| !command.trim().is_empty())
                .unwrap_or_else(|| process.identity.name.clone()),
            GroupBy::Executable => process
                .executable
                .clone()
                .filter(|path| !path.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
            GroupBy::Parent => process
                .parent_pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        };
        Self {
            key,
            group_type,
            display_name,
            pid: (group_type == GroupBy::Process).then_some(process.identity.pid),
            user_name: matches!(group_type, GroupBy::Process).then(|| process.user_display()),
            users: BTreeSet::new(),
            process_count: 0,
            cpu_percent: 0.0,
            ram_bytes: 0,
            virtual_ram_bytes: 0,
            disk_read_delta_bytes: 0,
            disk_write_delta_bytes: 0,
            top_process: None,
            timestamp,
        }
    }

    fn add(&mut self, process: &RawProcessSample, sort_by: SortBy, show_command: bool) {
        self.process_count += 1;
        self.cpu_percent += process.cpu_percent;
        self.ram_bytes += process.memory_bytes;
        self.virtual_ram_bytes += process.virtual_memory_bytes;
        self.disk_read_delta_bytes += process.disk_read_delta_bytes;
        self.disk_write_delta_bytes += process.disk_write_delta_bytes;
        self.users.insert(process.user_display());

        let process_name = process.display_process(show_command);
        let contribution = metric_contribution(process, sort_by);
        if self
            .top_process
            .as_ref()
            .is_none_or(|(_, current)| contribution > *current)
        {
            self.top_process = Some((process_name, contribution));
        }
    }

    fn into_row(self, interval_seconds: f64) -> SnapshotRow {
        let disk_io_delta_bytes = self.disk_read_delta_bytes + self.disk_write_delta_bytes;
        SnapshotRow {
            key: self.key,
            group_type: self.group_type,
            display_name: self.display_name,
            pid: self.pid,
            user_name: self.user_name,
            users: summarize_users(&self.users),
            process_count: self.process_count,
            cpu_percent: self.cpu_percent,
            ram_bytes: self.ram_bytes,
            virtual_ram_bytes: self.virtual_ram_bytes,
            disk_read_delta_bytes: self.disk_read_delta_bytes,
            disk_write_delta_bytes: self.disk_write_delta_bytes,
            disk_io_delta_bytes,
            read_bps: self.disk_read_delta_bytes as f64 / interval_seconds,
            write_bps: self.disk_write_delta_bytes as f64 / interval_seconds,
            io_bps: disk_io_delta_bytes as f64 / interval_seconds,
            top_process: self.top_process.map(|(name, _)| name),
            timestamp: self.timestamp,
        }
    }
}

#[derive(Debug)]
struct GroupSample {
    display_name: String,
    pid: Option<u32>,
    user_name: Option<String>,
    users: BTreeSet<String>,
    process_count: usize,
    cpu_percent: f32,
    memory_bytes: u64,
    read_delta_bytes: u64,
    write_delta_bytes: u64,
    top_process: Option<(String, f64)>,
    timestamp: SystemTime,
}

impl GroupSample {
    fn new(
        group_type: GroupBy,
        process: &RawProcessSample,
        show_command: bool,
        timestamp: SystemTime,
    ) -> Self {
        let display_name = match group_type {
            GroupBy::Process => process.display_process(show_command),
            GroupBy::Name => process.identity.name.clone(),
            GroupBy::User => process.user_display(),
            GroupBy::Command => process
                .command
                .clone()
                .filter(|command| !command.trim().is_empty())
                .unwrap_or_else(|| process.identity.name.clone()),
            GroupBy::Executable => process
                .executable
                .clone()
                .filter(|path| !path.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
            GroupBy::Parent => process
                .parent_pid
                .map(|pid| pid.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        };
        Self {
            display_name,
            pid: (group_type == GroupBy::Process).then_some(process.identity.pid),
            user_name: matches!(group_type, GroupBy::Process).then(|| process.user_display()),
            users: BTreeSet::new(),
            process_count: 0,
            cpu_percent: 0.0,
            memory_bytes: 0,
            read_delta_bytes: 0,
            write_delta_bytes: 0,
            top_process: None,
            timestamp,
        }
    }

    fn add(&mut self, process: &RawProcessSample, sort_by: SortBy, show_command: bool) {
        self.process_count += 1;
        self.cpu_percent += process.cpu_percent;
        self.memory_bytes += process.memory_bytes;
        self.read_delta_bytes += process.disk_read_delta_bytes;
        self.write_delta_bytes += process.disk_write_delta_bytes;
        self.users.insert(process.user_display());

        let process_name = process.display_process(show_command);
        let contribution = metric_contribution(process, sort_by);
        if self
            .top_process
            .as_ref()
            .is_none_or(|(_, current)| contribution > *current)
        {
            self.top_process = Some((process_name, contribution));
        }
    }
}

#[derive(Debug)]
struct RecordingAcc {
    key: GroupKey,
    group_type: GroupBy,
    display_name: String,
    pid: Option<u32>,
    user_name: Option<String>,
    users: BTreeSet<String>,
    max_process_count: usize,
    top_process: Option<(String, f64)>,
    cpu_max_percent: f32,
    cpu_core_seconds: f64,
    ram_start_bytes: u64,
    ram_end_bytes: u64,
    ram_min_bytes: u64,
    ram_max_bytes: u64,
    ram_sum_bytes: u128,
    ram_sample_count: usize,
    disk_read_total_bytes: u64,
    disk_write_total_bytes: u64,
    first_seen: SystemTime,
    last_seen: SystemTime,
    ram_timeline: Vec<(SystemTime, u64)>,
    cpu_timeline: Vec<(SystemTime, f32)>,
    read_timeline: Vec<(SystemTime, u64)>,
    write_timeline: Vec<(SystemTime, u64)>,
}

impl RecordingAcc {
    fn new(key: GroupKey, group_type: GroupBy, sample: &GroupSample) -> Self {
        Self {
            key,
            group_type,
            display_name: sample.display_name.clone(),
            pid: sample.pid,
            user_name: sample.user_name.clone(),
            users: sample.users.clone(),
            max_process_count: 0,
            top_process: None,
            cpu_max_percent: 0.0,
            cpu_core_seconds: 0.0,
            ram_start_bytes: sample.memory_bytes,
            ram_end_bytes: sample.memory_bytes,
            ram_min_bytes: sample.memory_bytes,
            ram_max_bytes: sample.memory_bytes,
            ram_sum_bytes: 0,
            ram_sample_count: 0,
            disk_read_total_bytes: 0,
            disk_write_total_bytes: 0,
            first_seen: sample.timestamp,
            last_seen: sample.timestamp,
            ram_timeline: Vec::new(),
            cpu_timeline: Vec::new(),
            read_timeline: Vec::new(),
            write_timeline: Vec::new(),
        }
    }

    fn has_activity(&self) -> bool {
        self.cpu_core_seconds > 0.0
            || self.disk_read_total_bytes > 0
            || self.disk_write_total_bytes > 0
            || self.ram_start_bytes != self.ram_end_bytes
    }

    fn add(&mut self, sample: &GroupSample, interval_seconds: f64) {
        self.users.extend(sample.users.iter().cloned());
        self.max_process_count = self.max_process_count.max(sample.process_count);
        self.cpu_max_percent = self.cpu_max_percent.max(sample.cpu_percent);
        self.cpu_core_seconds += sample.cpu_percent as f64 / 100.0 * interval_seconds;
        self.ram_end_bytes = sample.memory_bytes;
        self.ram_min_bytes = self.ram_min_bytes.min(sample.memory_bytes);
        self.ram_max_bytes = self.ram_max_bytes.max(sample.memory_bytes);
        self.ram_sum_bytes += sample.memory_bytes as u128;
        self.ram_sample_count += 1;
        self.disk_read_total_bytes += sample.read_delta_bytes;
        self.disk_write_total_bytes += sample.write_delta_bytes;
        self.last_seen = sample.timestamp;
        push_bounded_timeline(
            &mut self.ram_timeline,
            (sample.timestamp, sample.memory_bytes),
        );
        push_bounded_timeline(
            &mut self.cpu_timeline,
            (sample.timestamp, sample.cpu_percent),
        );
        push_bounded_timeline(
            &mut self.read_timeline,
            (sample.timestamp, sample.read_delta_bytes),
        );
        push_bounded_timeline(
            &mut self.write_timeline,
            (sample.timestamp, sample.write_delta_bytes),
        );

        if let Some((name, contribution)) = &sample.top_process
            && self
                .top_process
                .as_ref()
                .is_none_or(|(_, current)| contribution > current)
        {
            self.top_process = Some((name.clone(), *contribution));
        }
    }

    fn into_row(
        self,
        measured_seconds: f64,
        recording_started_at: Option<SystemTime>,
        recording_ended_at: Option<SystemTime>,
    ) -> AggregateRow {
        let ram_avg_bytes = if self.ram_sample_count == 0 {
            0
        } else {
            (self.ram_sum_bytes / self.ram_sample_count as u128) as u64
        };
        let disk_io_total_bytes = self.disk_read_total_bytes + self.disk_write_total_bytes;

        AggregateRow {
            key: self.key,
            group_type: self.group_type,
            display_name: self.display_name,
            pid: self.pid,
            user_name: self.user_name,
            users: summarize_users(&self.users),
            process_count: self.max_process_count,
            top_process: self.top_process.map(|(name, _)| name),
            cpu_avg_percent: ((self.cpu_core_seconds / measured_seconds) * 100.0) as f32,
            cpu_max_percent: self.cpu_max_percent,
            cpu_core_seconds: self.cpu_core_seconds,
            ram_start_bytes: self.ram_start_bytes,
            ram_end_bytes: self.ram_end_bytes,
            ram_min_bytes: self.ram_min_bytes,
            ram_max_bytes: self.ram_max_bytes,
            ram_avg_bytes,
            ram_delta_bytes: self.ram_end_bytes as i64 - self.ram_start_bytes as i64,
            disk_read_total_bytes: self.disk_read_total_bytes,
            disk_write_total_bytes: self.disk_write_total_bytes,
            disk_io_total_bytes,
            read_bytes_per_second_avg: self.disk_read_total_bytes as f64 / measured_seconds,
            write_bytes_per_second_avg: self.disk_write_total_bytes as f64 / measured_seconds,
            io_bytes_per_second_avg: disk_io_total_bytes as f64 / measured_seconds,
            first_seen: self.first_seen,
            last_seen: self.last_seen,
            lifecycle_status: lifecycle_status(
                self.first_seen,
                self.last_seen,
                recording_started_at,
                recording_ended_at,
            ),
            ram_timeline: self.ram_timeline,
            cpu_timeline: self.cpu_timeline,
            read_timeline: self.read_timeline,
            write_timeline: self.write_timeline,
        }
    }
}

fn push_bounded_timeline<T: Copy>(timeline: &mut Vec<(SystemTime, T)>, entry: (SystemTime, T)) {
    if timeline.len() >= MAX_TIMELINE_POINTS {
        let retained = timeline
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, value)| (index % 2 == 0).then_some(value))
            .collect();
        *timeline = retained;
    }
    timeline.push(entry);
}

fn lifecycle_status(
    first_seen: SystemTime,
    last_seen: SystemTime,
    recording_started_at: Option<SystemTime>,
    recording_ended_at: Option<SystemTime>,
) -> String {
    let visible_at_start = recording_started_at == Some(first_seen);
    let visible_at_end = recording_ended_at == Some(last_seen);

    match (visible_at_start, visible_at_end) {
        (true, true) => "observed_full_duration",
        (false, true) => "started_during_recording",
        (true, false) => "exited_during_recording",
        (false, false) => "started_and_exited_during_recording",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use crate::metrics::{ProcessIdentity, RawProcessSample, SystemSample};

    use super::*;

    fn process(
        pid: u32,
        start_time: u64,
        name: &str,
        user: &str,
        cpu: f32,
        ram: u64,
        io: (u64, u64),
    ) -> RawProcessSample {
        let (read, write) = io;
        RawProcessSample {
            timestamp: SystemTime::UNIX_EPOCH,
            identity: ProcessIdentity {
                pid,
                start_time_epoch_s: start_time,
                name: name.to_string(),
            },
            user_id: Some(user.to_string()),
            user_name: Some(user.to_string()),
            parent_pid: Some(1),
            executable: Some(format!("/usr/bin/{name}")),
            command: Some(format!("/usr/bin/{name} --flag")),
            memory_bytes: ram,
            virtual_memory_bytes: ram * 2,
            cpu_percent: cpu,
            disk_total_read_bytes: read,
            disk_total_write_bytes: write,
            disk_read_delta_bytes: read,
            disk_write_delta_bytes: write,
        }
    }

    fn system(processes: Vec<RawProcessSample>) -> SystemSample {
        SystemSample {
            timestamp: SystemTime::UNIX_EPOCH,
            total_memory_bytes: 1000,
            available_memory_bytes: 500,
            global_cpu_percent: 10.0,
            processes,
            sample_interval: Duration::from_secs(1),
            logical_cpu_count: 4,
        }
    }

    fn system_at(timestamp: SystemTime, processes: Vec<RawProcessSample>) -> SystemSample {
        SystemSample {
            timestamp,
            total_memory_bytes: 1000,
            available_memory_bytes: 500,
            global_cpu_percent: 10.0,
            processes,
            sample_interval: Duration::from_secs(1),
            logical_cpu_count: 4,
        }
    }

    #[test]
    fn groups_by_user_and_sums_metrics() {
        let sample = system(vec![
            process(1, 1, "node", "alice", 10.0, 100, (10, 5)),
            process(2, 1, "postgres", "alice", 20.0, 200, (0, 5)),
            process(3, 1, "bash", "root", 5.0, 50, (2, 0)),
        ]);

        let rows = aggregate_snapshot(
            &sample,
            GroupBy::User,
            SortBy::Ram,
            Duration::from_secs(1),
            false,
            10,
        );

        assert_eq!(rows[0].display_name, "alice");
        assert_eq!(rows[0].process_count, 2);
        assert_eq!(rows[0].ram_bytes, 300);
        assert_eq!(rows[0].disk_io_delta_bytes, 20);
    }

    #[test]
    fn recording_tracks_pid_reuse_by_identity() {
        let samples = vec![
            system(vec![process(7, 100, "worker", "alice", 50.0, 100, (5, 0))]),
            system(vec![process(7, 200, "worker", "alice", 25.0, 300, (7, 0))]),
        ];

        let rows = aggregate_recording(
            &samples,
            RecordingAggregateOptions {
                group_by: GroupBy::Process,
                sort_by: SortBy::Pid,
                interval: Duration::from_secs(1),
                measured_duration: Duration::from_secs(2),
                show_command: false,
                limit: 10,
                include_idle: true,
            },
        );

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pid, Some(7));
        assert_eq!(rows[1].pid, Some(7));
    }

    #[test]
    fn recording_computes_cpu_core_seconds_and_ram_delta() {
        let samples = vec![
            system(vec![process(1, 1, "node", "alice", 100.0, 100, (0, 10))]),
            system(vec![process(1, 1, "node", "alice", 50.0, 200, (0, 20))]),
        ];

        let rows = aggregate_recording(
            &samples,
            RecordingAggregateOptions {
                group_by: GroupBy::Name,
                sort_by: SortBy::Io,
                interval: Duration::from_secs(1),
                measured_duration: Duration::from_secs(2),
                show_command: false,
                limit: 10,
                include_idle: true,
            },
        );

        assert_eq!(rows.len(), 1);
        assert!((rows[0].cpu_core_seconds - 1.5).abs() < f64::EPSILON);
        assert_eq!(rows[0].ram_delta_bytes, 100);
        assert_eq!(rows[0].disk_write_total_bytes, 30);
    }

    #[test]
    fn groups_by_command_executable_and_parent() {
        let sample = system(vec![
            process(1, 1, "node", "alice", 10.0, 100, (10, 5)),
            process(2, 1, "node", "alice", 20.0, 200, (0, 5)),
        ]);

        let command_rows = aggregate_snapshot(
            &sample,
            GroupBy::Command,
            SortBy::Name,
            Duration::from_secs(1),
            false,
            10,
        );
        assert_eq!(command_rows.len(), 1);
        assert!(command_rows[0].display_name.contains("/usr/bin/node"));

        let executable_rows = aggregate_snapshot(
            &sample,
            GroupBy::Executable,
            SortBy::Name,
            Duration::from_secs(1),
            false,
            10,
        );
        assert_eq!(executable_rows[0].display_name, "/usr/bin/node");

        let parent_rows = aggregate_snapshot(
            &sample,
            GroupBy::Parent,
            SortBy::Name,
            Duration::from_secs(1),
            false,
            10,
        );
        assert_eq!(parent_rows[0].display_name, "1");
    }

    #[test]
    fn recording_reports_lifecycle_status() {
        let first = SystemTime::UNIX_EPOCH;
        let second = first + Duration::from_secs(1);
        let samples = vec![
            system_at(
                first,
                vec![process(1, 1, "node", "alice", 0.0, 100, (0, 0))],
            ),
            system_at(
                second,
                vec![process(2, 1, "bun", "alice", 1.0, 200, (0, 0))],
            ),
        ];

        let rows = aggregate_recording(
            &samples,
            RecordingAggregateOptions {
                group_by: GroupBy::Process,
                sort_by: SortBy::Pid,
                interval: Duration::from_secs(1),
                measured_duration: Duration::from_secs(2),
                show_command: false,
                limit: 10,
                include_idle: true,
            },
        );

        assert_eq!(rows[0].lifecycle_status, "exited_during_recording");
        assert_eq!(rows[1].lifecycle_status, "started_during_recording");
    }

    #[test]
    fn recording_timelines_are_bounded_for_long_runs() {
        let samples = (0..3000)
            .map(|index| {
                system_at(
                    SystemTime::UNIX_EPOCH + Duration::from_secs(index),
                    vec![process(1, 1, "node", "alice", 1.0, 100 + index, (0, 0))],
                )
            })
            .collect::<Vec<_>>();

        let rows = aggregate_recording(
            &samples,
            RecordingAggregateOptions {
                group_by: GroupBy::Process,
                sort_by: SortBy::Pid,
                interval: Duration::from_secs(1),
                measured_duration: Duration::from_secs(3000),
                show_command: false,
                limit: 10,
                include_idle: true,
            },
        );

        assert_eq!(rows.len(), 1);
        assert!(rows[0].ram_timeline.len() <= MAX_TIMELINE_POINTS);
        assert!(rows[0].cpu_timeline.len() <= MAX_TIMELINE_POINTS);
    }
}
