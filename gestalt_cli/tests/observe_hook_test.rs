#[path = "../src/observe/inject.rs"]
pub mod inject;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;

const HOOK_SCRIPT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/observe/assets/gestalt-observe-hook.sh"
);

#[test]
fn test_observe_hook_payload_roundtrip_with_special_characters() {
    // Edge-case payload values containing quotes, backslashes, newlines, and "}
    let complex_agent = "agent \"special\" \\backslash\\ 'single' `backtick` $VAR";
    let complex_event = "event\nwith\nnewlines and \\\\ backslash";
    let complex_summary = "summary with \"} attack \r\t and \n multi-line \\\" json injection";

    // Set up a mock HTTP server on an ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral TCP port");
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}/api/event", port);
    let token = "gestalt-secret-token-12345";

    // Handle the request in a background thread
    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut buffer = [0u8; 8192];
        let mut total_read = 0;

        // Read until headers + content
        loop {
            let n = stream.read(&mut buffer[total_read..]).unwrap();
            total_read += n;
            if n == 0 || buffer[..total_read].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        let request_str = String::from_utf8_lossy(&buffer[..total_read]).to_string();

        // Check if there's Content-Length to read remainder of body
        let content_length: usize = request_str
            .lines()
            .find(|line| line.to_lowercase().starts_with("content-length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|val| val.trim().parse().ok())
            .unwrap_or(0);

        let header_end = request_str.find("\r\n\r\n").unwrap() + 4;
        let mut body = buffer[header_end..total_read].to_vec();
        while body.len() < content_length {
            let mut chunk = [0u8; 1024];
            let n = stream.read(&mut chunk).unwrap();
            if n == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..n]);
        }

        // Send 200 OK response
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
        let _ = stream.write_all(response.as_bytes());

        (request_str, String::from_utf8(body).expect("Valid UTF-8 body"))
    });

    // Run the hook script
    let status = Command::new("sh")
        .arg(HOOK_SCRIPT_PATH)
        .arg(complex_agent)
        .arg(complex_event)
        .arg(complex_summary)
        .env("GESTALT_OBSERVE_URL", &url)
        .env("GESTALT_OBSERVE_TOKEN", token)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .expect("Failed to execute hook script");

    assert!(status.success(), "Hook script should exit 0");

    let (headers, body_str) = server_handle.join().expect("Server thread panicked");

    // Verify token was sent in Authorization header
    assert!(
        headers.contains("Authorization: Bearer gestalt-secret-token-12345"),
        "Missing or incorrect Authorization header: {}",
        headers
    );
    assert!(
        headers.to_lowercase().contains("content-type: application/json"),
        "Missing or incorrect Content-Type header: {}",
        headers
    );

    // Verify body is valid JSON and roundtrips EXACT characters
    let parsed: serde_json::Value = serde_json::from_str(&body_str)
        .unwrap_or_else(|e| panic!("Failed to parse body as JSON: {}\nBody: {}", e, body_str));

    assert_eq!(
        parsed["agent"].as_str().unwrap(),
        complex_agent,
        "Agent string did not roundtrip exactly"
    );
    assert_eq!(
        parsed["event_type"].as_str().unwrap(),
        complex_event,
        "Event type string did not roundtrip exactly"
    );
    assert_eq!(
        parsed["summary"].as_str().unwrap(),
        complex_summary,
        "Summary string did not roundtrip exactly"
    );
}

#[test]
fn test_observe_hook_url_validation_non_loopback() {
    // Insecure non-loopback HTTP must be rejected and fail-open (exit 0 without calling the server)
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral TCP port");
    let _port = listener.local_addr().unwrap().port();
    drop(listener); // Close port so if any connection is attempted, it would fail or be refused

    for invalid_url in [
        "http://insecure.remote.com/api/event",
        "http://127.0.0.1.attacker.com/api/event",
        "http://localhost.attacker.com/api/event",
        "ftp://127.0.0.1/api/event",
        "javascript:alert(1)",
    ] {
        let status = Command::new("sh")
            .arg(HOOK_SCRIPT_PATH)
            .arg("test-agent")
            .arg("test-event")
            .arg("test-summary")
            .env("GESTALT_OBSERVE_URL", invalid_url)
            .status()
            .expect("Failed to execute hook script");

        assert!(
            status.success(),
            "Script should fail-open with exit 0 for invalid URL: {}",
            invalid_url
        );
    }
}

#[test]
fn test_observe_hook_without_token() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral TCP port");
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}/api/event", port);

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut buffer = [0u8; 4096];
        let n = stream.read(&mut buffer).unwrap();
        let request_str = String::from_utf8_lossy(&buffer[..n]).to_string();

        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());

        request_str
    });

    let status = Command::new("sh")
        .arg(HOOK_SCRIPT_PATH)
        .arg("test-agent")
        .arg("test-event")
        .arg("test-summary")
        .env("GESTALT_OBSERVE_URL", &url)
        .env_remove("GESTALT_OBSERVE_TOKEN")
        .status()
        .expect("Failed to execute hook script");

    assert!(status.success());
    let headers = server_handle.join().expect("Server thread panicked");
    assert!(
        !headers.contains("Authorization:"),
        "Authorization header should not be sent when token is unset"
    );
}

#[test]
fn test_build_hook_script_hardening_with_special_characters() {
    let malicious_agent = "my-agent\"; rm -rf /; echo \"foo`test`$VAR";
    let malicious_event = "event\"} break {\"evil\": 1}";
    let malicious_summary = "summary\nwith\nnewlines and \"quotes\" and \\backslash\\";

    let generated_script = inject::build_hook_script(malicious_agent, malicious_event, malicious_summary);

    // Mock HTTP server
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral TCP port");
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}/api/event", port);

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut buffer = [0u8; 8192];
        let mut total_read = 0;
        loop {
            let n = stream.read(&mut buffer[total_read..]).unwrap();
            total_read += n;
            if n == 0 || buffer[..total_read].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request_str = String::from_utf8_lossy(&buffer[..total_read]).to_string();
        let header_end = request_str.find("\r\n\r\n").unwrap() + 4;
        let body = &request_str[header_end..];
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
        body.to_string()
    });

    // Execute generated script without arguments so it uses the substituted defaults!
    let status = Command::new("sh")
        .arg("-c")
        .arg(&generated_script)
        .env("GESTALT_OBSERVE_URL", &url)
        .status()
        .expect("Failed to execute generated script");

    assert!(status.success(), "Generated script should execute successfully");

    let body = server_handle.join().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("Failed to parse body JSON: {}\nBody: {}", e, body));

    assert_eq!(parsed["agent"].as_str().unwrap(), malicious_agent);
    assert_eq!(parsed["event_type"].as_str().unwrap(), malicious_event);
    assert_eq!(parsed["summary"].as_str().unwrap(), malicious_summary);
}

#[test]
fn test_observe_hook_python_fallback_roundtrip() {
    // Test the fallback mechanism when jq is NOT in PATH, but python3 IS in PATH
    let temp_bin_dir = tempfile::tempdir().unwrap();
    let bin_path = temp_bin_dir.path();

    // Symlink essential posix utilities + python3/python, but OMIT jq
    for cmd in ["sh", "curl", "python3", "python", "cat", "printf", "test", "["] {
        if let Ok(output) = Command::new("which").arg(cmd).output() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let _ = std::os::unix::fs::symlink(&path_str, bin_path.join(cmd));
            }
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral TCP port");
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}/api/event", port);

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut buffer = [0u8; 8192];
        let mut total_read = 0;
        loop {
            let n = stream.read(&mut buffer[total_read..]).unwrap();
            total_read += n;
            if n == 0 || buffer[..total_read].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request_str = String::from_utf8_lossy(&buffer[..total_read]).to_string();
        let header_end = request_str.find("\r\n\r\n").unwrap() + 4;
        let body = &request_str[header_end..];
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
        body.to_string()
    });

    let complex_summary = "summary with \"} attack \r\t and \n multi-line \\\" json injection";

    let status = Command::new("sh")
        .arg(HOOK_SCRIPT_PATH)
        .arg("agent\"with\"quotes")
        .arg("event\\with\\backslashes")
        .arg(complex_summary)
        .env("PATH", bin_path)
        .env("GESTALT_OBSERVE_URL", &url)
        .status()
        .expect("Failed to execute hook script with python PATH");

    assert!(status.success());
    let body = server_handle.join().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("Failed to parse body JSON in python fallback: {}\nBody: {}", e, body));

    assert_eq!(parsed["agent"].as_str().unwrap(), "agent\"with\"quotes");
    assert_eq!(parsed["event_type"].as_str().unwrap(), "event\\with\\backslashes");
    assert_eq!(parsed["summary"].as_str().unwrap(), complex_summary);
}

#[test]
fn test_observe_hook_posix_fallback_roundtrip() {
    // Test the fallback mechanism when jq and python are not in PATH
    // Create a restricted PATH containing only sh, curl, awk, sed, tr, cat, printf, etc.
    let temp_bin_dir = tempfile::tempdir().unwrap();
    let bin_path = temp_bin_dir.path();

    // Symlink essential posix utilities except jq and python
    for cmd in ["sh", "curl", "awk", "sed", "cat", "printf", "test", "["] {
        if let Ok(output) = Command::new("which").arg(cmd).output() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let _ = std::os::unix::fs::symlink(&path_str, bin_path.join(cmd));
            }
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral TCP port");
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}/api/event", port);

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");
        let mut buffer = [0u8; 8192];
        let mut total_read = 0;
        loop {
            let n = stream.read(&mut buffer[total_read..]).unwrap();
            total_read += n;
            if n == 0 || buffer[..total_read].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request_str = String::from_utf8_lossy(&buffer[..total_read]).to_string();
        let header_end = request_str.find("\r\n\r\n").unwrap() + 4;
        let body = &request_str[header_end..];
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
        body.to_string()
    });

    let complex_summary = "summary with \"} attack \r\t and \n multi-line \\\" json injection";

    let status = Command::new("sh")
        .arg(HOOK_SCRIPT_PATH)
        .arg("agent\"with\"quotes")
        .arg("event\\with\\backslashes")
        .arg(complex_summary)
        .env("PATH", bin_path)
        .env("GESTALT_OBSERVE_URL", &url)
        .status()
        .expect("Failed to execute hook script with restricted PATH");

    assert!(status.success());
    let body = server_handle.join().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("Failed to parse body JSON in fallback: {}\nBody: {}", e, body));

    assert_eq!(parsed["agent"].as_str().unwrap(), "agent\"with\"quotes");
    assert_eq!(parsed["event_type"].as_str().unwrap(), "event\\with\\backslashes");
    assert_eq!(parsed["summary"].as_str().unwrap(), complex_summary);
}
