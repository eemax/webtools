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

    cmd.args(["fetch", "http://localhost:3000"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("\"error\":\"blocked_host\""));
}
