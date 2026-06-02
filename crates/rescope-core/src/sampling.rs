use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use sysinfo::{
    CpuRefreshKind, MINIMUM_CPU_UPDATE_INTERVAL, MemoryRefreshKind, ProcessRefreshKind,
    ProcessesToUpdate, RefreshKind, System, Uid, UpdateKind, Users,
};

use crate::error::RescopeError;
use crate::metrics::{ProcessIdentity, RawProcessSample, SystemSample};

pub trait SampleSource {
    fn sample(&mut self) -> Result<SystemSample, RescopeError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SamplerConfig {
    pub include_command: bool,
}

#[derive(Debug)]
pub struct SysinfoSampler {
    system: System,
    users: Users,
    previous: HashMap<ProcessIdentity, PreviousCounters>,
    config: SamplerConfig,
}

#[derive(Debug, Clone, Copy)]
struct PreviousCounters {
    total_read_bytes: u64,
    total_write_bytes: u64,
    #[allow(dead_code)]
    timestamp: Instant,
}

impl SysinfoSampler {
    pub fn new(config: SamplerConfig) -> Result<Self, RescopeError> {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_memory(MemoryRefreshKind::everything())
                .with_cpu(CpuRefreshKind::everything())
                .with_processes(process_refresh_kind(config.include_command)),
        );
        system.refresh_memory();
        system.refresh_cpu_usage();

        Ok(Self {
            system,
            users: Users::new_with_refreshed_list(),
            previous: HashMap::new(),
            config,
        })
    }

    pub fn warm_up(&mut self, interval: Duration) -> Result<(), RescopeError> {
        self.refresh_once();
        thread::sleep(interval.max(MINIMUM_CPU_UPDATE_INTERVAL));
        self.refresh_once();
        Ok(())
    }

    fn refresh_once(&mut self) -> SystemSample {
        let timestamp = SystemTime::now();
        self.system.refresh_memory();
        self.system.refresh_cpu_usage();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            process_refresh_kind(self.config.include_command),
        );
        self.users.refresh();

        let mut seen = HashSet::new();
        let mut processes = Vec::with_capacity(self.system.processes().len());

        for (pid, process) in self.system.processes() {
            let name =
                os_to_string(process.name()).unwrap_or_else(|| format!("pid-{}", pid.as_u32()));
            let identity = ProcessIdentity {
                pid: pid.as_u32(),
                start_time_epoch_s: process.start_time(),
                name,
            };
            let disk = process.disk_usage();
            let previous = self.previous.get(&identity).copied();
            let disk_read_delta_bytes = previous
                .map(|old| counter_delta(disk.total_read_bytes, old.total_read_bytes))
                .unwrap_or(0);
            let disk_write_delta_bytes = previous
                .map(|old| counter_delta(disk.total_written_bytes, old.total_write_bytes))
                .unwrap_or(0);

            self.previous.insert(
                identity.clone(),
                PreviousCounters {
                    total_read_bytes: disk.total_read_bytes,
                    total_write_bytes: disk.total_written_bytes,
                    timestamp: Instant::now(),
                },
            );
            seen.insert(identity.clone());

            let user_id = process.user_id().map(uid_to_string);
            let user_name = process
                .user_id()
                .and_then(|uid| self.users.get_user_by_id(uid))
                .map(|user| user.name().to_string());

            processes.push(RawProcessSample {
                timestamp,
                identity,
                user_id,
                user_name,
                command: self
                    .config
                    .include_command
                    .then(|| command_to_string(process.cmd()))
                    .flatten(),
                memory_bytes: process.memory(),
                virtual_memory_bytes: process.virtual_memory(),
                cpu_percent: process.cpu_usage(),
                disk_total_read_bytes: disk.total_read_bytes,
                disk_total_write_bytes: disk.total_written_bytes,
                disk_read_delta_bytes,
                disk_write_delta_bytes,
            });
        }

        self.previous.retain(|identity, _| seen.contains(identity));

        SystemSample {
            timestamp,
            total_memory_bytes: self.system.total_memory(),
            available_memory_bytes: self.system.available_memory(),
            global_cpu_percent: self.system.global_cpu_usage(),
            processes,
        }
    }
}

impl SampleSource for SysinfoSampler {
    fn sample(&mut self) -> Result<SystemSample, RescopeError> {
        Ok(self.refresh_once())
    }
}

impl SampleSource for Vec<SystemSample> {
    fn sample(&mut self) -> Result<SystemSample, RescopeError> {
        if self.is_empty() {
            Err(RescopeError::NoSamples)
        } else {
            Ok(self.remove(0))
        }
    }
}

fn process_refresh_kind(include_command: bool) -> ProcessRefreshKind {
    let kind = ProcessRefreshKind::nothing()
        .with_memory()
        .with_cpu()
        .with_disk_usage()
        .with_user(UpdateKind::OnlyIfNotSet)
        .without_tasks();

    if include_command {
        kind.with_cmd(UpdateKind::OnlyIfNotSet)
    } else {
        kind
    }
}

fn os_to_string(value: &std::ffi::OsStr) -> Option<String> {
    let value = value.to_string_lossy().trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn command_to_string(command: &[OsString]) -> Option<String> {
    let command = command
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    (!command.trim().is_empty()).then_some(command)
}

fn uid_to_string(uid: &Uid) -> String {
    let debug = format!("{uid:?}");
    debug
        .strip_prefix("Uid(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(&debug)
        .to_string()
}

fn counter_delta(current: u64, previous: u64) -> u64 {
    current.saturating_sub(previous)
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::metrics::FilterSpec;

    use super::*;

    #[test]
    fn vec_sample_source_is_mockable() {
        let sample = SystemSample {
            timestamp: SystemTime::UNIX_EPOCH,
            total_memory_bytes: 1,
            available_memory_bytes: 1,
            global_cpu_percent: 0.0,
            processes: Vec::new(),
        };
        let mut source = vec![sample.clone()];
        assert_eq!(source.sample().unwrap().total_memory_bytes, 1);
        assert!(source.sample().is_err());
    }

    #[test]
    fn default_filter_is_empty_for_mock_use() {
        assert!(FilterSpec::default().pids.is_empty());
    }

    #[test]
    fn disk_counter_delta_never_underflows() {
        assert_eq!(counter_delta(10, 4), 6);
        assert_eq!(counter_delta(4, 10), 0);
    }
}
