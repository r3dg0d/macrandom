//! CLI smoke tests — no root required.

use assert_cmd::Command;
use predicates::prelude::*;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use tempfile::TempDir;

struct TestCommand {
    command: Command,
    _paths: TempDir,
}

impl Deref for TestCommand {
    type Target = Command;
    fn deref(&self) -> &Command {
        &self.command
    }
}

impl DerefMut for TestCommand {
    fn deref_mut(&mut self) -> &mut Command {
        &mut self.command
    }
}

fn bin() -> TestCommand {
    let paths = tempfile::tempdir().unwrap();
    let mut command = Command::cargo_bin("macrandom").unwrap();
    for (variable, directory) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
    ] {
        command.env(variable, paths.path().join(directory));
    }
    TestCommand {
        command,
        _paths: paths,
    }
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
    let mut command = bin();
    let assertion = command.arg("list").assert();
    if Path::new("/sys/class/net").is_dir() {
        assertion.success();
    } else {
        // Nix sandboxes omit sysfs; verify the documented I/O failure instead.
        assertion
            .code(7)
            .stderr(predicate::str::contains("/sys/class/net"));
    }
}

#[test]
fn list_all_json() {
    let mut command = bin();
    let assertion = command.args(["--json", "list", "--all"]).assert();
    if Path::new("/sys/class/net").is_dir() {
        assertion
            .success()
            .stdout(predicate::str::contains("interfaces"));
    } else {
        assertion
            .code(7)
            .stderr(predicate::str::contains("/sys/class/net"));
    }
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
    let mut command = bin();
    let assertion = command.args(["--dry-run", "randomize", "lo"]).assert();
    if Path::new("/sys/class/net/lo").exists() {
        assertion
            .code(3)
            .stderr(predicate::str::contains("default skip list"));
    } else {
        assertion
            .code(2)
            .stderr(predicate::str::contains("interface not found: lo"));
    }
}

#[test]
fn dry_run_randomize_lo_force() {
    let mut command = bin();
    let assertion = command
        .args(["--dry-run", "randomize", "lo", "--force"])
        .assert();
    if Path::new("/sys/class/net/lo").exists() {
        assertion
            .success()
            .stdout(predicate::str::contains("[dry-run] lo:"));
    } else {
        assertion
            .code(2)
            .stderr(predicate::str::contains("interface not found: lo"));
    }
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
