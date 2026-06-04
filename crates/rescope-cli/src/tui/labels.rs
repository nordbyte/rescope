use rescope_core::GroupBy;

pub fn group_label(group_by: GroupBy) -> &'static str {
    match group_by {
        GroupBy::Process => "process",
        GroupBy::Name => "name",
        GroupBy::User => "user",
        GroupBy::Command => "command",
        GroupBy::Executable => "executable",
        GroupBy::Parent => "parent",
        GroupBy::Cgroup => "cgroup",
        GroupBy::Systemd => "systemd",
        GroupBy::Container => "container",
    }
}
