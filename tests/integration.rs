//! Integration tests for capx CLI.
//!
//! These tests require a live Capacities account.
//! Set CAP_TOKEN env to run them. They are skipped otherwise.

use std::process::Command;

fn capx() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_capx"));
    if let Ok(token) = std::env::var("CAP_TOKEN") {
        cmd.arg("--token").arg(token);
    }
    cmd
}

fn has_token() -> bool {
    std::env::var("CAP_TOKEN").is_ok()
}

#[test]
fn spaces_returns_output() {
    if !has_token() {
        eprintln!("Skipping: CAP_TOKEN not set");
        return;
    }
    let output = capx().arg("spaces").output().expect("failed to run capx");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "spaces should return at least one space"
    );
}

#[test]
fn spaces_json() {
    if !has_token() {
        eprintln!("Skipping: CAP_TOKEN not set");
        return;
    }
    let output = capx()
        .arg("--json")
        .arg("spaces")
        .output()
        .expect("failed to run capx");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("spaces --json should return valid JSON");
    assert!(parsed["spaces"].is_array());
}

#[test]
fn whoami_returns_user() {
    if !has_token() {
        eprintln!("Skipping: CAP_TOKEN not set");
        return;
    }
    let output = capx().arg("whoami").output().expect("failed to run capx");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ID:") || stdout.contains("Email:"),
        "whoami should show user info"
    );
}

#[test]
fn whoami_json() {
    if !has_token() {
        eprintln!("Skipping: CAP_TOKEN not set");
        return;
    }
    let output = capx()
        .arg("--json")
        .arg("whoami")
        .output()
        .expect("failed to run capx");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("whoami --json should return valid JSON");
    assert!(parsed["id"].is_string());
    assert!(parsed["email"].is_string());
}

#[test]
fn search_returns_results() {
    if !has_token() {
        eprintln!("Skipping: CAP_TOKEN not set");
        return;
    }
    let output = capx()
        .arg("search")
        .arg("test")
        .output()
        .expect("failed to run capx");
    // Search may return empty but should not error
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn types_returns_structures() {
    if !has_token() {
        eprintln!("Skipping: CAP_TOKEN not set");
        return;
    }
    let output = capx()
        .arg("--json")
        .arg("types")
        .output()
        .expect("failed to run capx");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("types --json should return valid JSON");
    assert!(parsed.is_array(), "types should return an array");
}

#[test]
fn ls_returns_summary() {
    if !has_token() {
        eprintln!("Skipping: CAP_TOKEN not set");
        return;
    }
    let output = capx()
        .arg("--json")
        .arg("ls")
        .output()
        .expect("failed to run capx");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("ls --json should return valid JSON");
    assert!(parsed["total"].is_number());
}

#[test]
fn crud_lifecycle() {
    if !has_token() {
        eprintln!("Skipping: CAP_TOKEN not set");
        return;
    }

    // Create
    let output = capx()
        .arg("--json")
        .arg("create")
        .arg("Page")
        .arg("capx-test-page")
        .arg("-d")
        .arg("Integration test page")
        .arg("-b")
        .arg("# Test\nHello from capx integration tests")
        .output()
        .expect("failed to create");
    assert!(
        output.status.success(),
        "create failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let created: serde_json::Value =
        serde_json::from_str(&stdout).expect("create should return JSON");
    let id = created["id"].as_str().expect("create should return id");
    assert_eq!(created["status"], "success");

    // Get
    let output = capx()
        .arg("--json")
        .arg("get")
        .arg(id)
        .output()
        .expect("failed to get");
    assert!(
        output.status.success(),
        "get failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let objects: serde_json::Value = serde_json::from_str(&stdout).expect("get should return JSON");
    let arr = objects.as_array().expect("get should return array");
    assert!(!arr.is_empty(), "get should return the created object");
    assert_eq!(arr[0]["title"], "capx-test-page");

    // Update
    let output = capx()
        .arg("--json")
        .arg("update")
        .arg(id)
        .arg("-t")
        .arg("capx-test-page-updated")
        .output()
        .expect("failed to update");
    assert!(
        output.status.success(),
        "update failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let updated: serde_json::Value =
        serde_json::from_str(&stdout).expect("update should return JSON");
    assert_eq!(updated["status"], "success");

    // Get again to verify update
    let output = capx()
        .arg("--json")
        .arg("get")
        .arg(id)
        .output()
        .expect("failed to get after update");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let objects: serde_json::Value = serde_json::from_str(&stdout).expect("get should return JSON");
    let arr = objects.as_array().unwrap();
    assert_eq!(arr[0]["title"], "capx-test-page-updated");

    // Delete
    let output = capx()
        .arg("--json")
        .arg("rm")
        .arg(id)
        .arg("--yes")
        .output()
        .expect("failed to delete");
    assert!(
        output.status.success(),
        "delete failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Undo delete
    let output = capx()
        .arg("--json")
        .arg("undo")
        .arg(id)
        .output()
        .expect("failed to undo");
    assert!(
        output.status.success(),
        "undo failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Final delete (cleanup)
    let output = capx()
        .arg("--json")
        .arg("rm")
        .arg(id)
        .arg("--yes")
        .output()
        .expect("failed to final delete");
    assert!(
        output.status.success(),
        "final delete failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
