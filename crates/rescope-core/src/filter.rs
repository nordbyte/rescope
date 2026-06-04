use std::process;

use crate::metrics::{FilterSpec, RawProcessSample, SystemSample};
use regex::{Regex, RegexBuilder};

pub fn filter_sample(sample: &SystemSample, filters: &FilterSpec) -> SystemSample {
    let matcher = CompiledFilter::new(filters);
    let mut filtered = sample.clone();
    filtered.processes = sample
        .processes
        .iter()
        .filter(|process| matcher.matches(process))
        .cloned()
        .collect();
    filtered
}

pub fn matches_filters(process_sample: &RawProcessSample, filters: &FilterSpec) -> bool {
    CompiledFilter::new(filters).matches(process_sample)
}

#[derive(Debug, Clone)]
pub struct CompiledFilter {
    pids: Vec<u32>,
    users: Vec<String>,
    process_substrings: Vec<String>,
    names: Vec<String>,
    name_regexes: Vec<Regex>,
    command_substrings: Vec<String>,
    command_regexes: Vec<Regex>,
    executable_substrings: Vec<String>,
    executable_regexes: Vec<Regex>,
    parent_pids: Vec<u32>,
    parent_names: Vec<String>,
    parent_regexes: Vec<Regex>,
    min_cpu_percent: Option<f32>,
    min_ram_bytes: Option<u64>,
    min_io_delta_bytes: Option<u64>,
    hide_self: bool,
    invert_match: bool,
    has_positive_filters: bool,
}

impl CompiledFilter {
    pub fn new(filters: &FilterSpec) -> Self {
        Self {
            pids: filters.pids.clone(),
            users: lower_all(&filters.users),
            process_substrings: lower_all(&filters.process_substrings),
            names: lower_all(&filters.names),
            name_regexes: compile_regexes(&filters.name_regexes),
            command_substrings: lower_all(&filters.command_substrings),
            command_regexes: compile_regexes(&filters.command_regexes),
            executable_substrings: lower_all(&filters.executable_substrings),
            executable_regexes: compile_regexes(&filters.executable_regexes),
            parent_pids: filters.parent_pids.clone(),
            parent_names: lower_all(&filters.parent_names),
            parent_regexes: compile_regexes(&filters.parent_regexes),
            min_cpu_percent: filters.min_cpu_percent,
            min_ram_bytes: filters.min_ram_bytes,
            min_io_delta_bytes: filters.min_io_delta_bytes,
            hide_self: filters.hide_self,
            invert_match: filters.invert_match,
            has_positive_filters: has_positive_filters(filters),
        }
    }

    pub fn matches(&self, process_sample: &RawProcessSample) -> bool {
        if self.hide_self && process_sample.identity.pid == process::id() {
            return false;
        }

        if !self.has_positive_filters {
            return true;
        }

        let matched = self.matches_positive_filters(process_sample);
        if self.invert_match { !matched } else { matched }
    }

    fn matches_positive_filters(&self, process_sample: &RawProcessSample) -> bool {
        if !self.pids.is_empty() && !self.pids.contains(&process_sample.identity.pid) {
            return false;
        }

        if !self.users.is_empty() && !matches_user(process_sample, &self.users) {
            return false;
        }

        if !self.process_substrings.is_empty()
            && !matches_process_search(process_sample, &self.process_substrings)
        {
            return false;
        }

        if !self.names.is_empty() && !matches_any_lower(&process_sample.identity.name, &self.names)
        {
            return false;
        }

        if !self.name_regexes.is_empty()
            && !matches_any_regex(&process_sample.identity.name, &self.name_regexes)
        {
            return false;
        }

        if !self.command_substrings.is_empty() {
            let command = process_sample.command.as_deref().unwrap_or_default();
            if !matches_any_lower(command, &self.command_substrings) {
                return false;
            }
        }

        if !self.command_regexes.is_empty() {
            let command = process_sample.command.as_deref().unwrap_or_default();
            if !matches_any_regex(command, &self.command_regexes) {
                return false;
            }
        }

        if !self.executable_substrings.is_empty() {
            let executable = process_sample.executable.as_deref().unwrap_or_default();
            if !matches_any_lower(executable, &self.executable_substrings) {
                return false;
            }
        }

        if !self.executable_regexes.is_empty() {
            let executable = process_sample.executable.as_deref().unwrap_or_default();
            if !matches_any_regex(executable, &self.executable_regexes) {
                return false;
            }
        }

        if !self.parent_pids.is_empty()
            && !process_sample
                .parent_pid
                .is_some_and(|pid| self.parent_pids.contains(&pid))
        {
            return false;
        }

        if !self.parent_names.is_empty() {
            let parent = process_sample.parent_name.as_deref().unwrap_or("unknown");
            if !matches_any_lower(parent, &self.parent_names) {
                return false;
            }
        }

        if !self.parent_regexes.is_empty() {
            let parent = process_sample.parent_name.as_deref().unwrap_or("unknown");
            if !matches_any_regex(parent, &self.parent_regexes) {
                return false;
            }
        }

        if let Some(min_cpu_percent) = self.min_cpu_percent
            && process_sample.cpu_percent < min_cpu_percent
        {
            return false;
        }

        if let Some(min_ram_bytes) = self.min_ram_bytes
            && process_sample.memory_bytes < min_ram_bytes
        {
            return false;
        }

        if let Some(min_io_delta_bytes) = self.min_io_delta_bytes {
            let io_delta =
                process_sample.disk_read_delta_bytes + process_sample.disk_write_delta_bytes;
            if io_delta < min_io_delta_bytes {
                return false;
            }
        }

        true
    }
}

fn has_positive_filters(filters: &FilterSpec) -> bool {
    !filters.pids.is_empty()
        || !filters.users.is_empty()
        || !filters.process_substrings.is_empty()
        || !filters.names.is_empty()
        || !filters.name_regexes.is_empty()
        || !filters.command_substrings.is_empty()
        || !filters.command_regexes.is_empty()
        || !filters.executable_substrings.is_empty()
        || !filters.executable_regexes.is_empty()
        || !filters.parent_pids.is_empty()
        || !filters.parent_names.is_empty()
        || !filters.parent_regexes.is_empty()
        || filters.min_cpu_percent.is_some()
        || filters.min_ram_bytes.is_some()
        || filters.min_io_delta_bytes.is_some()
}

fn lower_all(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn compile_regexes(patterns: &[String]) -> Vec<Regex> {
    patterns
        .iter()
        .filter_map(|pattern| {
            RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .ok()
        })
        .collect()
}

fn matches_user(process_sample: &RawProcessSample, filters: &[String]) -> bool {
    filters.iter().any(|wanted| {
        let name_match = process_sample
            .user_name
            .as_ref()
            .is_some_and(|name| name.eq_ignore_ascii_case(wanted));
        let id_match = process_sample
            .user_id
            .as_ref()
            .is_some_and(|id| id.eq_ignore_ascii_case(wanted));
        let unknown_match = wanted == "unknown"
            && process_sample.user_name.is_none()
            && process_sample.user_id.is_none();

        name_match || id_match || unknown_match
    })
}

fn matches_any_lower(value: &str, needles: &[String]) -> bool {
    let value = value.to_ascii_lowercase();
    needles.iter().any(|needle| value.contains(needle))
}

fn matches_any_regex(value: &str, regexes: &[Regex]) -> bool {
    regexes.iter().any(|regex| regex.is_match(value))
}

fn matches_process_search(process_sample: &RawProcessSample, needles: &[String]) -> bool {
    needles.iter().any(|needle| {
        process_sample.identity.pid.to_string().contains(needle)
            || process_sample
                .identity
                .name
                .to_ascii_lowercase()
                .contains(needle)
            || process_sample
                .executable
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains(needle)
            || process_sample
                .command
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains(needle)
    })
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::metrics::{ProcessDetails, ProcessIdentity, RawProcessSample};

    use super::*;

    fn sample() -> RawProcessSample {
        RawProcessSample {
            timestamp: SystemTime::UNIX_EPOCH,
            identity: ProcessIdentity {
                pid: 42,
                start_time_epoch_s: 1,
                name: "Node".to_string(),
            },
            user_id: Some("1000".to_string()),
            user_name: Some("alice".to_string()),
            parent_pid: Some(1),
            parent_name: Some("systemd".to_string()),
            executable: Some("/usr/bin/node".to_string()),
            command: Some("/usr/bin/node server.js".to_string()),
            memory_bytes: 1,
            virtual_memory_bytes: 2,
            cpu_percent: 3.0,
            disk_total_read_bytes: 4,
            disk_total_write_bytes: 5,
            disk_read_delta_bytes: 6,
            disk_write_delta_bytes: 7,
            details: ProcessDetails::default(),
        }
    }

    #[test]
    fn filter_groups_are_and_within_group_or() {
        let mut filters = FilterSpec {
            users: vec!["alice".to_string()],
            names: vec!["node".to_string(), "bun".to_string()],
            ..FilterSpec::default()
        };
        assert!(matches_filters(&sample(), &filters));

        filters.users = vec!["root".to_string()];
        assert!(!matches_filters(&sample(), &filters));
    }

    #[test]
    fn command_filter_does_not_require_displaying_command() {
        let filters = FilterSpec {
            command_substrings: vec!["SERVER".to_string()],
            ..FilterSpec::default()
        };
        assert!(matches_filters(&sample(), &filters));
    }

    #[test]
    fn process_filter_matches_name_pid_executable_and_command() {
        for needle in ["node", "42", "/usr/bin/node", "server.js"] {
            let filters = FilterSpec {
                process_substrings: vec![needle.to_string()],
                ..FilterSpec::default()
            };
            assert!(matches_filters(&sample(), &filters), "{needle}");
        }
    }

    #[test]
    fn regex_and_threshold_filters_match_processes() {
        let filters = FilterSpec {
            name_regexes: vec!["^no.e$".to_string()],
            command_regexes: vec!["server\\.js$".to_string()],
            executable_substrings: vec!["bin/node".to_string()],
            parent_names: vec!["system".to_string()],
            min_cpu_percent: Some(2.0),
            min_ram_bytes: Some(1),
            min_io_delta_bytes: Some(13),
            ..FilterSpec::default()
        };
        assert!(matches_filters(&sample(), &filters));
    }

    #[test]
    fn executable_and_parent_regex_filters_match_processes() {
        let filters = FilterSpec {
            executable_regexes: vec!["node$".to_string()],
            parent_pids: vec![1],
            parent_regexes: vec!["^sys.*d$".to_string()],
            ..FilterSpec::default()
        };
        assert!(matches_filters(&sample(), &filters));
    }

    #[test]
    fn invert_match_negates_positive_filters_but_keeps_hide_self() {
        let filters = FilterSpec {
            names: vec!["postgres".to_string()],
            invert_match: true,
            ..FilterSpec::default()
        };
        assert!(matches_filters(&sample(), &filters));
    }

    #[test]
    fn invert_without_positive_filters_keeps_all_rows() {
        let filters = FilterSpec {
            invert_match: true,
            ..FilterSpec::default()
        };
        assert!(matches_filters(&sample(), &filters));
    }
}
