use gestalt_router::observe::{parse_orca_event, OrcaBridgeReceiver};
use gestalt_state::StateDb;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::sleep;

// Minimal TempDir struct for test isolation (from memory context)
struct MiniTempDir {
    path: std::path::PathBuf,
}

impl MiniTempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for MiniTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_parse_orca_event_started() {
    let payload = r#"{
        "hook_type": "agent_started",
        "agent": "claude",
        "run_id": "r-123",
        "metadata": {"version": "1.0"},
        "message": "Starting claude"
    }"#;

    let event = parse_orca_event(payload).expect("should parse successfully");
    assert_eq!(event.agent, "claude");
    assert_eq!(event.event_type, "run_started");
    assert_eq!(event.run_id.as_deref(), Some("r-123"));
    assert_eq!(event.summary, "Starting claude");
    assert_eq!(event.metadata, serde_json::json!({"version": "1.0"}));
}

#[test]
fn test_parse_orca_event_defaults() {
    let payload = r#"{
        "hook_type": "tool_call",
        "agent": "copilot",
        "run_id": "r-456"
    }"#;

    let event = parse_orca_event(payload).expect("should parse without optional fields");
    assert_eq!(event.agent, "copilot");
    assert_eq!(event.event_type, "tool_call");
    assert_eq!(event.summary, "Orca event: tool_call"); // Default message
    assert!(event.metadata.is_null()); // Default metadata is usually Null if using standard Option, but serde_json::Value default is Null
}

#[test]
fn test_parse_orca_event_invalid_type() {
    let payload = r#"{
        "hook_type": "unknown_hook",
        "agent": "cursor",
        "run_id": "r-789"
    }"#;

    let result = parse_orca_event(payload);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown hook type: unknown_hook"));
}

#[tokio::test]
async fn test_listen_and_forward_integration() {
    let temp_dir = MiniTempDir::new();
    let db_path = temp_dir.path().join("state.db");
    let db = Arc::new(StateDb::open(&db_path).unwrap());

    // Bind to any open ephemeral port on localhost
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let bind_addr = listener.local_addr().unwrap().to_string();
    drop(listener); // release so we can rebind with tokio

    let receiver = OrcaBridgeReceiver::new(db.clone(), bind_addr.clone());
    let cancel_token = receiver.cancel_token();

    let server_handle = tokio::spawn(async move {
        receiver.listen_and_forward().await.unwrap();
    });

    // Give server time to bind
    sleep(Duration::from_millis(50)).await;

    // Send a payload via HTTP-ish (just raw json with EOF)
    let body = r#"{
        "hook_type": "agent_finished",
        "agent": "antigravity",
        "run_id": "r-999",
        "message": "Done!"
    }"#;
    let payload = format!("POST / HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);

    let mut stream = TcpStream::connect(&bind_addr).await.unwrap();
    stream.write_all(payload.as_bytes()).await.unwrap();
    stream.shutdown().await.unwrap(); // sends EOF

    // Give receiver time to process
    sleep(Duration::from_millis(200)).await;

    // Check DB
    let events = db.get_events_older_than(chrono::Utc::now()).unwrap();
    
    // Cleanup first
    cancel_token.cancel();
    server_handle.await.unwrap();

    assert!(!events.is_empty(), "Event should be persisted");
    let stored = &events[0];
    assert_eq!(stored.agent_id.as_deref(), Some("antigravity"));
    assert_eq!(stored.event_type, "run_finished");
    assert_eq!(stored.run_id, "r-999");
    assert!(stored.payload.contains("Done!"));
}

#[tokio::test]
async fn test_fail_open_timeout() {
    let temp_dir = MiniTempDir::new();
    let db_path = temp_dir.path().join("state_timeout.db");
    let db = Arc::new(StateDb::open(&db_path).unwrap());

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let bind_addr = listener.local_addr().unwrap().to_string();
    drop(listener);

    let receiver = OrcaBridgeReceiver::new(db.clone(), bind_addr.clone());
    let cancel_token = receiver.cancel_token();

    let server_handle = tokio::spawn(async move {
        receiver.listen_and_forward().await.unwrap();
    });

    sleep(Duration::from_millis(50)).await;

    // Connect but never send data
    let mut stream = TcpStream::connect(&bind_addr).await.unwrap();
    
    // We expect the server connection to time out after 3s, fail open, and not crash.
    // Wait > 3s
    sleep(Duration::from_secs(4)).await;

    // Send data after timeout to prove stream is dead from server perspective
    let _res = stream.write_all(b"{}").await;
    // We might get BrokenPipe depending on OS, but the key is server didn't block forever.

    cancel_token.cancel();
    server_handle.await.unwrap();
    
    // DB should have 0 events
    let events = db.get_events_older_than(chrono::Utc::now()).unwrap();
    assert!(events.is_empty());
}
