use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn help_prints_usage() {
    Command::cargo_bin("rescope")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Inspect and record resource usage",
        ));
}

#[test]
fn snapshot_process_runs() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["snapshot", "--limit", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("System:"));
}

#[test]
fn snapshot_user_runs() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["snapshot", "--group", "user", "--limit", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("USER"));
}

#[test]
fn record_runs() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "record",
            "--duration",
            "1s",
            "--interval",
            "1s",
            "--limit",
            "5",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("rescope report"));
}

#[test]
fn record_no_matches_is_successful_empty_report() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "record",
            "--duration",
            "1s",
            "--interval",
            "1s",
            "--name",
            "definitely-no-such-process",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("no matching processes"));
}

#[test]
fn record_all_includes_idle_rows() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "record",
            "--duration",
            "1s",
            "--interval",
            "1s",
            "--all",
            "--json",
            "-",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"display_name\""));
}

#[test]
fn snapshot_exports_json_and_csv() {
    let dir = tempfile::tempdir().unwrap();
    let json = dir.path().join("snapshot.json");
    let csv = dir.path().join("snapshot.csv");

    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "snapshot",
            "--limit",
            "2",
            "--json",
            json.to_str().unwrap(),
            "--csv",
            csv.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(json.exists());
    assert!(csv.exists());

    let json_value: Value = serde_json::from_slice(&std::fs::read(&json).unwrap()).unwrap();
    assert_eq!(json_value["tool"], "rescope");
    assert_eq!(json_value["mode"], "snapshot");
    assert!(json_value["logical_cpu_count"].as_u64().unwrap() >= 1);
    assert!(json_value["rows"].is_array());

    let csv_text = std::fs::read_to_string(csv).unwrap();
    assert!(csv_text.contains("cpu_normalized_percent"));
}

#[test]
fn snapshot_supports_new_groups_all_and_normalized_cpu() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "snapshot",
            "--group",
            "parent",
            "--all",
            "--normalize-cpu",
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("PARENT"));
}

#[test]
fn snapshot_supports_cgroup_systemd_and_container_groups() {
    for (group, header) in [
        ("cgroup", "CGROUP"),
        ("systemd", "SYSTEMD"),
        ("container", "CONTAINER"),
    ] {
        Command::cargo_bin("rescope")
            .unwrap()
            .args(["snapshot", "--group", group, "--limit", "5"])
            .assert()
            .success()
            .stdout(predicate::str::contains(header));
    }
}

#[test]
fn snapshot_profile_tree_uses_parent_grouping() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["snapshot", "--profile", "tree", "--limit", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PARENT"));
}

#[test]
fn snapshot_accepts_executable_and_parent_filters() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "snapshot",
            "--exe",
            "rescope",
            "--parent",
            "1",
            "--parent-name",
            "system",
            "--limit",
            "1",
        ])
        .assert()
        .success();
}

#[test]
fn snapshot_accepts_flexible_process_filter_and_path_display() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "snapshot",
            "--process",
            "rescope",
            "--show-path",
            "--limit",
            "5",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("PATH"));
}

#[test]
fn snapshot_accepts_path_alias_for_executable_filter() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["snapshot", "--path", "/usr", "--limit", "1"])
        .assert()
        .success();
}

#[test]
fn snapshot_json_exports_executable_paths_when_requested() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "snapshot",
            "--process",
            "rescope",
            "--show-path",
            "--limit",
            "5",
            "--json",
            "-",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"show_path\": true"))
        .stdout(predicate::str::contains("\"process_substrings\""));
}

#[test]
fn config_file_applies_profile_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("rescope.json");
    std::fs::write(&config, r#"{"profile":"users","limit":5,"hide_self":true}"#).unwrap();

    Command::cargo_bin("rescope")
        .unwrap()
        .args(["--config", config.to_str().unwrap(), "snapshot"])
        .assert()
        .success()
        .stdout(predicate::str::contains("USER"));
}

#[test]
fn config_file_applies_named_profile() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("rescope.json");
    std::fs::write(
        &config,
        r#"{"profiles":{"containers":{"group":"container","limit":5}}}"#,
    )
    .unwrap();

    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "--config",
            config.to_str().unwrap(),
            "--config-profile",
            "containers",
            "snapshot",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("CONTAINER"));
}

#[test]
fn snapshot_accepts_regex_threshold_and_invert_filters() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "snapshot",
            "--name-regex",
            ".*",
            "--min-cpu",
            "0",
            "--min-ram",
            "0",
            "--min-io",
            "0",
            "--invert",
            "--limit",
            "1",
        ])
        .assert()
        .success();
}

#[test]
fn record_csv_contains_percentile_and_lifecycle_columns() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "record",
            "--duration",
            "1s",
            "--interval",
            "1s",
            "--csv",
            "-",
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("cpu_p95_percent"))
        .stdout(predicate::str::contains("started_count"));
}

#[test]
fn live_once_can_export_json_to_stdout() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["live", "--once", "--json", "-", "--limit", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mode\": \"snapshot\""));
}

#[test]
fn live_once_can_stream_jsonl_to_stdout() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["live", "--once", "--quiet", "--jsonl", "-", "--limit", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mode\":\"live\""));
}

#[test]
fn live_once_can_stream_csv_to_stdout() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "live",
            "--once",
            "--quiet",
            "--csv-stream",
            "-",
            "--limit",
            "2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "timestamp,group_type,display_name",
        ));
}

#[test]
fn live_once_can_export_prometheus_to_stdout() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "live",
            "--once",
            "--quiet",
            "--prometheus",
            "-",
            "--limit",
            "2",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("rescope_system_cpu_percent"));
}

#[test]
fn live_export_without_once_is_rejected() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["live", "--json", "-", "--limit", "2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("only with --once"));
}

#[test]
fn invalid_interval_is_rejected() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["snapshot", "--interval", "1ms"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("interval must be at least"));
}

#[test]
fn invalid_regex_is_rejected() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["snapshot", "--name-regex", "["])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid regex"));
}

#[test]
fn stdout_export_streams_cannot_be_combined() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["snapshot", "--json", "-", "--csv", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("only one of --json - or --csv -"));
}

#[test]
fn tree_command_runs_and_exports_json() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["tree", "--limit", "5", "--json", "-"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mode\": \"tree\""))
        .stdout(predicate::str::contains("\"nodes\""));
}

#[test]
fn record_raw_samples_can_be_replayed() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("raw.json");

    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "record",
            "--duration",
            "1s",
            "--interval",
            "1s",
            "--raw-samples",
            raw.to_str().unwrap(),
            "--quiet",
        ])
        .assert()
        .success();

    Command::cargo_bin("rescope")
        .unwrap()
        .args(["replay", raw.to_str().unwrap(), "--json", "-"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mode\": \"record\""));
}

#[test]
fn completions_and_man_can_write_stdout() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rescope"));

    Command::cargo_bin("rescope")
        .unwrap()
        .arg("man")
        .assert()
        .success()
        .stdout(predicate::str::contains("rescope"));
}

#[test]
fn watch_no_match_exits_successfully() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "watch",
            "--duration",
            "1s",
            "--interval",
            "1s",
            "--name",
            "definitely-no-such-process",
            "--quiet",
        ])
        .assert()
        .success();
}

#[test]
fn diff_command_compares_json_reports() {
    let dir = tempfile::tempdir().unwrap();
    let before = dir.path().join("before.json");
    let after = dir.path().join("after.json");
    std::fs::write(
        &before,
        r#"{"mode":"snapshot","rows":[{"group_type":"process","display_name":"alpha","pid":1,"cpu_percent":1,"ram_bytes":100,"disk_io_delta_bytes":0}]}"#,
    )
    .unwrap();
    std::fs::write(
        &after,
        r#"{"mode":"snapshot","rows":[{"group_type":"process","display_name":"alpha","pid":1,"cpu_percent":2,"ram_bytes":150,"disk_io_delta_bytes":20},{"group_type":"process","display_name":"beta","pid":2,"cpu_percent":1,"ram_bytes":50,"disk_io_delta_bytes":0}]}"#,
    )
    .unwrap();

    Command::cargo_bin("rescope")
        .unwrap()
        .args([
            "--json",
            "-",
            "diff",
            before.to_str().unwrap(),
            after.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mode\": \"diff\""))
        .stdout(predicate::str::contains("\"status\": \"changed\""))
        .stdout(predicate::str::contains("\"status\": \"added\""));
}
