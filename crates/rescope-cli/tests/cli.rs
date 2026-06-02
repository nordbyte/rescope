use assert_cmd::Command;
use predicates::prelude::*;

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
}
