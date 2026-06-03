use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupBy {
    Process,
    Name,
    User,
    Command,
    Executable,
    Parent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    Cpu,
    Ram,
    Read,
    Write,
    Io,
    Pid,
    Name,
    User,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FilterSpec {
    pub pids: Vec<u32>,
    pub users: Vec<String>,
    pub process_substrings: Vec<String>,
    pub names: Vec<String>,
    pub name_regexes: Vec<String>,
    pub command_substrings: Vec<String>,
    pub command_regexes: Vec<String>,
    pub executable_substrings: Vec<String>,
    pub executable_regexes: Vec<String>,
    pub parent_pids: Vec<u32>,
    pub parent_names: Vec<String>,
    pub parent_regexes: Vec<String>,
    pub min_cpu_percent: Option<f32>,
    pub min_ram_bytes: Option<u64>,
    pub min_io_delta_bytes: Option<u64>,
    pub hide_self: bool,
    pub invert_match: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub start_time_epoch_s: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawProcessSample {
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub timestamp: SystemTime,
    pub identity: ProcessIdentity,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub parent_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub cpu_percent: f32,
    pub disk_total_read_bytes: u64,
    pub disk_total_write_bytes: u64,
    pub disk_read_delta_bytes: u64,
    pub disk_write_delta_bytes: u64,
}

impl RawProcessSample {
    pub fn user_display(&self) -> String {
        self.user_name
            .clone()
            .or_else(|| self.user_id.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn display_process(&self, show_command: bool, show_path: bool) -> String {
        if show_command {
            self.command
                .as_ref()
                .filter(|cmd| !cmd.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| self.identity.name.clone())
        } else if show_path {
            self.executable
                .as_ref()
                .filter(|path| !path.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| self.identity.name.clone())
        } else {
            self.identity.name.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSample {
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub timestamp: SystemTime,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub global_cpu_percent: f32,
    pub processes: Vec<RawProcessSample>,
    #[serde(
        rename = "sample_interval_ms",
        serialize_with = "serialize_duration_ms"
    )]
    pub sample_interval: Duration,
    pub logical_cpu_count: usize,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupKey {
    Process(ProcessIdentity),
    Name(String),
    User(String),
    Command(String),
    Executable(String),
    Parent(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotRow {
    #[serde(skip)]
    pub key: GroupKey,
    pub group_type: GroupBy,
    pub display_name: String,
    pub pid: Option<u32>,
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    pub users: Option<String>,
    pub process_count: usize,
    pub cpu_percent: f32,
    pub ram_bytes: u64,
    pub virtual_ram_bytes: u64,
    pub disk_read_delta_bytes: u64,
    pub disk_write_delta_bytes: u64,
    pub disk_io_delta_bytes: u64,
    pub read_bps: f64,
    pub write_bps: f64,
    pub io_bps: f64,
    pub top_process: Option<String>,
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct AggregateRow {
    #[serde(skip)]
    pub key: GroupKey,
    pub group_type: GroupBy,
    pub display_name: String,
    pub pid: Option<u32>,
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    pub users: Option<String>,
    pub process_count: usize,
    pub top_process: Option<String>,
    pub cpu_avg_percent: f32,
    pub cpu_max_percent: f32,
    pub cpu_p95_percent: f32,
    pub cpu_p99_percent: f32,
    pub cpu_core_seconds: f64,
    pub ram_start_bytes: u64,
    pub ram_end_bytes: u64,
    pub ram_min_bytes: u64,
    pub ram_max_bytes: u64,
    pub ram_p95_bytes: u64,
    pub ram_avg_bytes: u64,
    pub ram_delta_bytes: i64,
    pub disk_read_total_bytes: u64,
    pub disk_write_total_bytes: u64,
    pub disk_io_total_bytes: u64,
    pub io_p95_bytes: u64,
    pub read_bytes_per_second_avg: f64,
    pub write_bytes_per_second_avg: f64,
    pub io_bytes_per_second_avg: f64,
    pub started_count: usize,
    pub exited_count: usize,
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub first_seen: SystemTime,
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub last_seen: SystemTime,
    pub lifecycle_status: String,
    #[serde(serialize_with = "serialize_timeline_ms")]
    pub ram_timeline: Vec<(SystemTime, u64)>,
    #[serde(serialize_with = "serialize_f32_timeline_ms")]
    pub cpu_timeline: Vec<(SystemTime, f32)>,
    #[serde(serialize_with = "serialize_timeline_ms")]
    pub read_timeline: Vec<(SystemTime, u64)>,
    #[serde(serialize_with = "serialize_timeline_ms")]
    pub write_timeline: Vec<(SystemTime, u64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotReport {
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub started_at: SystemTime,
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub ended_at: SystemTime,
    #[serde(rename = "duration_ms", serialize_with = "serialize_duration_ms")]
    pub duration: Duration,
    #[serde(rename = "interval_ms", serialize_with = "serialize_duration_ms")]
    pub interval: Duration,
    pub sample_count: usize,
    pub group_by: GroupBy,
    pub sort_by: SortBy,
    pub filters: FilterSpec,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub global_cpu_percent: f32,
    pub process_total: usize,
    pub logical_cpu_count: usize,
    pub cpu_normalized: bool,
    pub show_path: bool,
    pub rows: Vec<SnapshotRow>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingReport {
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub started_at: SystemTime,
    #[serde(serialize_with = "serialize_system_time_ms")]
    pub ended_at: SystemTime,
    #[serde(rename = "duration_ms", serialize_with = "serialize_duration_ms")]
    pub duration: Duration,
    #[serde(rename = "interval_ms", serialize_with = "serialize_duration_ms")]
    pub interval: Duration,
    pub sample_count: usize,
    pub group_by: GroupBy,
    pub sort_by: SortBy,
    pub filters: FilterSpec,
    pub logical_cpu_count: usize,
    pub cpu_normalized: bool,
    pub show_path: bool,
    pub rows: Vec<AggregateRow>,
    pub notes: Vec<String>,
}

pub fn system_time_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

pub fn serialize_system_time_ms<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(system_time_ms(*time))
}

pub fn serialize_duration_ms<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(duration.as_millis().min(u64::MAX as u128) as u64)
}

pub fn serialize_timeline_ms<S>(
    timeline: &[(SystemTime, u64)],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let converted: Vec<(u64, u64)> = timeline
        .iter()
        .map(|(timestamp, memory)| (system_time_ms(*timestamp), *memory))
        .collect();
    converted.serialize(serializer)
}

pub fn serialize_f32_timeline_ms<S>(
    timeline: &[(SystemTime, f32)],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let converted: Vec<(u64, f32)> = timeline
        .iter()
        .map(|(timestamp, value)| (system_time_ms(*timestamp), *value))
        .collect();
    converted.serialize(serializer)
}
