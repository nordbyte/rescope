use crate::metrics::{GroupBy, GroupKey, RawProcessSample};

pub fn group_key(process: &RawProcessSample, group_by: GroupBy) -> GroupKey {
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
        GroupBy::Parent => GroupKey::Parent(parent_display(process)),
        GroupBy::Cgroup => GroupKey::Cgroup(cgroup_display(process)),
        GroupBy::Systemd => GroupKey::Systemd(systemd_display(process)),
        GroupBy::Container => GroupKey::Container(container_display(process)),
    }
}

pub fn display_name_for_group(
    group_type: GroupBy,
    process: &RawProcessSample,
    show_command: bool,
) -> String {
    match group_type {
        GroupBy::Process => process.display_process(show_command, false),
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
        GroupBy::Parent => parent_display(process),
        GroupBy::Cgroup => cgroup_display(process),
        GroupBy::Systemd => systemd_display(process),
        GroupBy::Container => container_display(process),
    }
}

fn parent_display(process: &RawProcessSample) -> String {
    match (process.parent_pid, process.parent_name.as_deref()) {
        (Some(pid), Some(name)) if !name.trim().is_empty() => format!("{pid} ({name})"),
        (Some(pid), _) => pid.to_string(),
        (None, _) => "unknown".to_string(),
    }
}

fn cgroup_display(process: &RawProcessSample) -> String {
    process
        .details
        .cgroup_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn systemd_display(process: &RawProcessSample) -> String {
    let Some(cgroup_path) = process.details.cgroup_path.as_deref() else {
        return "unknown".to_string();
    };
    cgroup_path
        .split('/')
        .rev()
        .find_map(|part| {
            let part = part.trim();
            (part.ends_with(".service") || part.ends_with(".scope") || part.ends_with(".slice"))
                .then_some(part.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn container_display(process: &RawProcessSample) -> String {
    let Some(cgroup_path) = process.details.cgroup_path.as_deref() else {
        return "host".to_string();
    };
    cgroup_path
        .split(&['/', ':', '.'][..])
        .find_map(container_id_from_segment)
        .unwrap_or_else(|| "host".to_string())
}

fn container_id_from_segment(segment: &str) -> Option<String> {
    let trimmed = segment.trim();
    for prefix in [
        "docker-",
        "docker/",
        "cri-containerd-",
        "crio-",
        "libpod-",
        "containerd-",
    ] {
        if let Some(value) = trimmed.strip_prefix(prefix)
            && let Some(id) = leading_hex(value)
        {
            return Some(short_container_id(id));
        }
    }

    leading_hex(trimmed).map(short_container_id)
}

fn leading_hex(value: &str) -> Option<&str> {
    let len = value
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .count();
    (len >= 12).then_some(&value[..len])
}

fn short_container_id(value: &str) -> String {
    value.chars().take(12).collect()
}
