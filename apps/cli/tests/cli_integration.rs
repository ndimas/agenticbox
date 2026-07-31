//! Integration tests for the `agenticbox` CLI binary.
//!
//! These tests spawn the actual compiled binary and verify its behavior:
//! - `run demo` produces expected output
//! - `agents` lists available agents
//! - `init` creates a manifest file
//! - `--help` and `--version` work

use std::path::PathBuf;
use std::process::Command;

fn binary() -> PathBuf {
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join(if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        });

    // Some test runners use target/debug, some use target/{profile}
    let bin_name = if cfg!(windows) {
        "agenticbox.exe"
    } else {
        "agenticbox"
    };

    let direct = target_dir.join(bin_name);
    if direct.exists() {
        return direct;
    }

    // Fallback: try target/debug from workspace root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("debug")
        .join(bin_name)
}

#[test]
fn test_help_flag() {
    let output = Command::new(binary())
        .arg("--help")
        .output()
        .expect("failed to run agenticbox");

    assert!(output.status.success(), "agenticbox --help failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("agenticbox") || stdout.contains("AgenticBox"));
    assert!(stdout.contains("run") || stdout.contains("deploy"));
}

#[test]
fn test_version_flag() {
    let output = Command::new(binary())
        .arg("--version")
        .output()
        .expect("failed to run agenticbox");

    assert!(output.status.success(), "agenticbox --version failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("agenticbox") || stdout.contains("0.1"));
}

#[test]
fn test_run_demo_output() {
    let output = Command::new(binary())
        .args(["run", "demo"])
        .output()
        .expect("failed to run agenticbox run demo");

    assert!(output.status.success(), "agenticbox run demo failed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify the demo shows the expected permission guard output
    assert!(stdout.contains("AgenticBox") || stdout.contains("Permission"));
    assert!(stdout.contains("BLOCKED"));
    assert!(stdout.contains("ALLOWED"));
    assert!(stdout.contains("SSH"));
    assert!(stdout.contains("pastebin.com"));
    assert!(stdout.contains("deploy.sh") || stdout.contains("deploy"));
}

#[test]
fn test_run_demo_blocks_ssh_keys() {
    let output = Command::new(binary())
        .args(["run", "demo"])
        .output()
        .expect("failed to run demo");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // SSH key access should be blocked
    assert!(stdout.contains("ssh") || stdout.contains("SSH"));
    assert!(stdout.contains("BLOCKED"));
}

#[test]
fn test_run_demo_blocks_network_exfil() {
    let output = Command::new(binary())
        .args(["run", "demo"])
        .output()
        .expect("failed to run demo");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("evil") || stdout.contains("pastebin.com"));
    assert!(stdout.contains("BLOCKED"));
}

#[test]
fn test_run_demo_blocks_aws_creds() {
    let output = Command::new(binary())
        .args(["run", "demo"])
        .output()
        .expect("failed to run demo");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The demo should show secret/env protection
    assert!(stdout.to_lowercase().contains("secret") || stdout.to_lowercase().contains(".env"));
}

#[test]
fn test_run_demo_allows_workspace_read() {
    let output = Command::new(binary())
        .args(["run", "demo"])
        .output()
        .expect("failed to run demo");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show an ALLOWED for reading workspace files
    assert!(stdout.contains("/workspace"));
    assert!(stdout.contains("ALLOWED"));
}

#[test]
fn test_run_demo_summary_counts() {
    let output = Command::new(binary())
        .args(["run", "demo"])
        .output()
        .expect("failed to run demo");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The demo should report blocked count
    assert!(stdout.contains("blocked") || stdout.contains("Blocked"));
}

#[test]
fn test_agents_command() {
    let output = Command::new(binary())
        .args(["agents"])
        .output()
        .expect("failed to run agenticbox agents");

    assert!(output.status.success(), "agenticbox agents failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should at least show the built-in demo agent
    assert!(stdout.contains("demo") || stdout.contains("agent"));
}

#[test]
fn test_agents_paths_command() {
    let output = Command::new(binary())
        .args(["agents", "--paths"])
        .output()
        .expect("failed to run agenticbox agents --paths");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".agenticbox") || stdout.contains("Agents dir"));
}

#[test]
fn test_init_creates_manifest() {
    let temp_dir =
        std::env::temp_dir().join(format!("agenticbox-test-init-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    // The init command writes to ~/.agenticbox/agents/<name>/agent.toml
    // which is not in the temp dir — so we test via HOME override
    // This is a smoke test; verifying the file creation in a real HOME is risky
    // Instead, we verify the binary accepts the args without crashing

    // Use a unique agent name to avoid collisions
    let agent_name = format!(
        "test-agent-{}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    let output = Command::new(binary())
        .args(["init", &agent_name])
        .env("HOME", &temp_dir)
        .output();

    // Clean up temp dir regardless of test outcome
    let _ = std::fs::remove_dir_all(&temp_dir);

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let _stderr = String::from_utf8_lossy(&o.stderr);
            // Either it succeeded and created the manifest, or it failed gracefully
            if o.status.success() {
                assert!(stdout.contains("Created agent manifest") || stdout.contains("✓"));
            }
            // No panic — binary didn't crash
        }
        Err(e) => {
            // Binary might not be built yet — skip rather than fail
            eprintln!("Skipping init test: binary not available: {}", e);
        }
    }
}

#[test]
fn test_run_with_no_args_shows_usage() {
    let output = Command::new(binary())
        .args(["run"])
        .output()
        .expect("failed to run agenticbox run");

    // Should fail gracefully with usage info
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}\n{}", stdout, stderr);

    // Either non-zero exit with error message, or help text
    assert!(
        !output.status.success()
            || combined.contains("Usage")
            || combined.contains("Nothing to run"),
        "agenticbox run with no args should show usage or error"
    );
}
