use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_prints_usage() {
    let mut cmd = Command::cargo_bin("webtools").expect("binary");

    cmd.arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "webtools fetch [--json|--md] <url>",
        ));
}

#[test]
fn version_prints_package_version() {
    let mut cmd = Command::cargo_bin("webtools").expect("binary");

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn fetch_rejects_conflicting_output_flags() {
    let mut cmd = Command::cargo_bin("webtools").expect("binary");

    cmd.args(["fetch", "--json", "--md", "https://example.com"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "choose only one output mode: --md or --json",
        ));
}

#[test]
fn fetch_blocks_localhost_as_json() {
    let mut cmd = Command::cargo_bin("webtools").expect("binary");

    let output = cmd
        .args(["fetch", "http://localhost:3000"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("json stdout");

    assert_eq!(json["ok"], false);
    assert_eq!(json["kind"], "error");
    assert_eq!(json["error"], "blocked_host");
    assert_eq!(json["content"], "");
}

#[test]
fn fetch_invalid_url_is_json_success() {
    let mut cmd = Command::cargo_bin("webtools").expect("binary");

    let output = cmd
        .args(["fetch", "not-a-url"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("json stdout");

    assert_eq!(json["ok"], false);
    assert_eq!(json["error"], "invalid_url");
}
