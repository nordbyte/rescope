use std::process;

use crate::metrics::{FilterSpec, RawProcessSample, SystemSample};
use regex::RegexBuilder;

pub fn filter_sample(sample: &SystemSample, filters: &FilterSpec) -> SystemSample {
    let mut filtered = sample.clone();
    filtered.processes = sample
        .processes
        .iter()
        .filter(|process| matches_filters(process, filters))
        .cloned()
        .collect();
    filtered
}

pub fn matches_filters(process_sample: &RawProcessSample, filters: &FilterSpec) -> bool {
    if filters.hide_self && process_sample.identity.pid == process::id() {
        return false;
    }

    let matched = matches_positive_filters(process_sample, filters);
    if filters.invert_match {
        !matched
    } else {
        matched
    }
}

fn matches_positive_filters(process_sample: &RawProcessSample, filters: &FilterSpec) -> bool {
    if !filters.pids.is_empty() && !filters.pids.contains(&process_sample.identity.pid) {
        return false;
    }

    if !filters.users.is_empty() && !matches_user(process_sample, &filters.users) {
        return false;
    }

    if !filters.names.is_empty() && !matches_any_ci(&process_sample.identity.name, &filters.names) {
        return false;
    }

    if !filters.name_regexes.is_empty()
        && !matches_any_regex(&process_sample.identity.name, &filters.name_regexes)
    {
        return false;
    }

    if !filters.command_substrings.is_empty() {
        let command = process_sample.command.as_deref().unwrap_or_default();
        if !matches_any_ci(command, &filters.command_substrings) {
            return false;
        }
    }

    if !filters.command_regexes.is_empty() {
        let command = process_sample.command.as_deref().unwrap_or_default();
        if !matches_any_regex(command, &filters.command_regexes) {
            return false;
        }
    }

    if let Some(min_cpu_percent) = filters.min_cpu_percent
        && process_sample.cpu_percent < min_cpu_percent
    {
        return false;
    }

    if let Some(min_ram_bytes) = filters.min_ram_bytes
        && process_sample.memory_bytes < min_ram_bytes
    {
        return false;
    }

    if let Some(min_io_delta_bytes) = filters.min_io_delta_bytes {
        let io_delta = process_sample.disk_read_delta_bytes + process_sample.disk_write_delta_bytes;
        if io_delta < min_io_delta_bytes {
            return false;
        }
    }

    true
}

fn matches_user(process_sample: &RawProcessSample, filters: &[String]) -> bool {
    filters.iter().any(|wanted| {
        let wanted = wanted.to_ascii_lowercase();
        let name_match = process_sample
            .user_name
            .as_ref()
            .is_some_and(|name| name.eq_ignore_ascii_case(&wanted));
        let id_match = process_sample
            .user_id
            .as_ref()
            .is_some_and(|id| id.eq_ignore_ascii_case(&wanted));
        let unknown_match = wanted == "unknown"
            && process_sample.user_name.is_none()
            && process_sample.user_id.is_none();

        name_match || id_match || unknown_match
    })
}

fn matches_any_ci(value: &str, needles: &[String]) -> bool {
    let value = value.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| value.contains(&needle.to_ascii_lowercase()))
}

fn matches_any_regex(value: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .is_ok_and(|regex| regex.is_match(value))
    })
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use crate::metrics::{ProcessIdentity, RawProcessSample};

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
            executable: Some("/usr/bin/node".to_string()),
            command: Some("/usr/bin/node server.js".to_string()),
            memory_bytes: 1,
            virtual_memory_bytes: 2,
            cpu_percent: 3.0,
            disk_total_read_bytes: 4,
            disk_total_write_bytes: 5,
            disk_read_delta_bytes: 6,
            disk_write_delta_bytes: 7,
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
    fn regex_and_threshold_filters_match_processes() {
        let filters = FilterSpec {
            name_regexes: vec!["^no.e$".to_string()],
            command_regexes: vec!["server\\.js$".to_string()],
            min_cpu_percent: Some(2.0),
            min_ram_bytes: Some(1),
            min_io_delta_bytes: Some(13),
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
}
