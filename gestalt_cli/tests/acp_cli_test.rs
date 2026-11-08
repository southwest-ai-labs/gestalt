use std::process::Command;

#[test]
fn test_acp_serve_help() {
    let output = Command::new("cargo")
        .args(["run", "-p", "gestalt_cli", "--", "acp", "serve", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Start the ACP server"));
    assert!(stdout.contains("--stdio"));
    assert!(stdout.contains("--port <PORT>"));
}

#[test]
fn test_acp_serve_stdio_help() {
    let output = Command::new("cargo")
        .args(["run", "-p", "gestalt_cli", "--", "acp", "serve", "--stdio", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Start the ACP server"));
}

#[test]
fn test_acp_serve_port_help() {
    let output = Command::new("cargo")
        .args(["run", "-p", "gestalt_cli", "--", "acp", "serve", "--port", "8085", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Start the ACP server"));
}
