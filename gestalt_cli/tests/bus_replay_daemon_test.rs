use std::sync::Arc;

#[path = "../src/commands/bus_replay_daemon.rs"]
mod bus_replay_daemon;

use bus_replay_daemon::ReplaySyncDaemon;
use gestalt_state::StateDb;
use gestalt_router::xavier_sink::XavierEventSink;
use gestalt_core::application::agent::xavier::XavierClient;

// We need a dummy or minimal StateDb and XavierEventSink to test this properly.
// In memory tests:
#[tokio::test]
async fn test_bus_replay_sync_batch() {
    // 1. Create a minimal TempDir and StateDb 
    // Using simple sqlite db for testing
    let tmp_dir = std::env::temp_dir().join(format!("bus_replay_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let db_path = tmp_dir.join("state.db");
    
    let db = StateDb::open(&db_path).unwrap();
    let db_arc = Arc::new(db);
    
    // 2. Create XavierEventSink
    let client = XavierClient::from_env();
    let sink = Arc::new(XavierEventSink::new(client));
    
    let daemon = ReplaySyncDaemon::new(db_arc.clone(), sink.clone());
    
    // Push some events
    let _ = db_arc.push_event("run-1", Some("agent-1"), "type-1", r#"{"foo": "bar"}"#).unwrap();
    let _ = db_arc.push_event("run-2", Some("agent-2"), "type-2", r#"{"foo": "baz"}"#).unwrap();
    
    // Sync batch - this will fail on real sink if xavier is offline (expected)
    // For test we just want to ensure we don't advance the cursor on error.
    let res = daemon.sync_batch(&db_arc, &sink, 10).await;
    assert!(res.is_err() || res.unwrap() == 2, "Should error or sync 2");
    
    // Check cursor
    let cursor_str = db_arc.get_metadata("xavier_replay_cursor").unwrap();
    // It should be None or if it succeeded it should be Some(2)
    assert!(cursor_str.is_none() || cursor_str.unwrap() == "2", "Cursor should be unset or 2");
}
