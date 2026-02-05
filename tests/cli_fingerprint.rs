//! Integration tests for the `nab fingerprint` command.
//!
//! The fingerprint command is fully self-contained (no network, no browser
//! state) so every test here is deterministic.

#![allow(deprecated)] // cargo_bin deprecation — replacement not yet stable

use assert_cmd::Command;
use predicates::prelude::*;

/// Helper: get a Command for the `nab` binary.
fn nab() -> Command {
    Command::cargo_bin("nab").expect("binary 'nab' should be built")
}

// ─── Default invocation ──────────────────────────────────────────────────────

#[test]
fn fingerprint_default_generates_three_profiles() {
    nab()
        .arg("fingerprint")
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile 1:"))
        .stdout(predicate::str::contains("Profile 2:"))
        .stdout(predicate::str::contains("Profile 3:"))
        .stdout(predicate::str::contains("UA:"))
        .stdout(predicate::str::contains("Accept-Language:"));
}

#[test]
fn fingerprint_single_profile() {
    let output = nab()
        .args(["fingerprint", "--count", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("Profile 1:"), "should show profile 1");
    assert!(
        !stdout.contains("Profile 2:"),
        "should NOT show profile 2 when count=1"
    );
}

#[test]
fn fingerprint_many_profiles() {
    nab()
        .args(["fingerprint", "--count", "10"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile 10:"));
}

#[test]
fn fingerprint_zero_count_produces_no_profiles() {
    let output = nab()
        .args(["fingerprint", "--count", "0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    assert!(
        !stdout.contains("Profile 1:"),
        "count=0 should produce no profiles"
    );
}

// ─── Output content validation ───────────────────────────────────────────────

#[test]
fn fingerprint_user_agent_looks_realistic() {
    let output = nab()
        .args(["fingerprint", "--count", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();

    // Every generated UA should mention Mozilla (standard browser UA prefix)
    assert!(
        stdout.contains("Mozilla/"),
        "generated User-Agent should start with Mozilla/"
    );
}

#[test]
fn fingerprint_profiles_are_randomized() {
    // Generate two batches and check they are not identical.
    // With 5 profiles each, the chance of identical output is negligible.
    let out1 = nab()
        .args(["fingerprint", "--count", "5"])
        .output()
        .expect("first run should succeed");

    let out2 = nab()
        .args(["fingerprint", "--count", "5"])
        .output()
        .expect("second run should succeed");

    assert_ne!(
        out1.stdout, out2.stdout,
        "two fingerprint runs should produce different profiles"
    );
}
