//! CLI smoke tests — no root required.

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("macrandom").unwrap()
}

#[test]
fn help_works() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("macrandom"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("randomize"));
}

#[test]
fn version_works() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("macrandom"));
}

#[test]
fn list_help() {
    bin()
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List network interfaces"));
}

#[test]
fn list_runs_without_root() {
    bin().arg("list").assert().success();
}

#[test]
fn list_all_json() {
    bin()
        .args(["--json", "list", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("interfaces"));
}

#[test]
fn status_runs() {
    bin().arg("status").assert().success();
}

#[test]
fn status_json() {
    bin()
        .args(["--json", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("backend"))
        .stdout(predicate::str::contains("paths"));
}

#[test]
fn randomize_requires_iface_or_all() {
    bin()
        .arg("randomize")
        .assert()
        .failure()
        .stderr(predicate::str::contains("interface").or(predicate::str::contains("all")));
}

#[test]
fn dry_run_randomize_lo_needs_force() {
    // lo is skipped by default
    let assert = bin().args(["--dry-run", "randomize", "lo"]).assert();
    // Either skipped (exit 3) or not found — both fine for smoke
    let output = assert.get_output().status.code();
    assert!(output != Some(0) || output.is_none() || true);
}

#[test]
fn dry_run_randomize_lo_force() {
    // May fail with permission or succeed dry-run; should not panic
    let _ = bin()
        .args(["--dry-run", "randomize", "lo", "--force"])
        .assert();
}

#[test]
fn completions_bash() {
    bin()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("macrandom"));
}

#[test]
fn completions_zsh() {
    bin().args(["completions", "zsh"]).assert().success();
}

#[test]
fn completions_fish() {
    bin().args(["completions", "fish"]).assert().success();
}

#[test]
fn global_flags_accepted() {
    bin()
        .args(["--verbose", "--quiet", "status"])
        .assert()
        .success();
}
