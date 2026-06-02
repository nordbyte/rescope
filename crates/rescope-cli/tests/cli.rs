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
fn live_once_can_export_json_to_stdout() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["live", "--once", "--json", "-", "--limit", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"mode\": \"snapshot\""));
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
fn stdout_export_streams_cannot_be_combined() {
    Command::cargo_bin("rescope")
        .unwrap()
        .args(["snapshot", "--json", "-", "--csv", "-"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("only one of --json - or --csv -"));
}
