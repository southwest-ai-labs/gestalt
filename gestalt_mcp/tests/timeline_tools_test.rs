use gestalt_mcp::tools::timeline_tools::{GestaltHealthHandler, GestaltTimelineQueryHandler};
use mcp_protocol_sdk::core::tool::ToolHandler;
use serde_json::json;
use std::collections::HashMap;
use gestalt_state::StateDb;
use tempfile::tempdir;

#[tokio::test]
async fn test_timeline_query_handler() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    
    // Create db and populate an event
    let db = StateDb::open(&db_path).unwrap();
    db.push_event_with_hash("run-1", Some("agent-x"), "test_event", r#"{"project": "proj-1"}"#, None).unwrap();
    
    let handler = GestaltTimelineQueryHandler;
    
    let mut args = HashMap::new();
    args.insert("db_path".to_string(), json!(db_path.to_str().unwrap()));
    args.insert("agent".to_string(), json!("agent-x"));
    
    let result = handler.call(args).await.unwrap();
    assert!(!result.is_error.unwrap_or(true));
    
    // Check that we got the event back
    let content = &result.content[0];
    if let mcp_protocol_sdk::protocol::types::ContentBlock::Text { text, .. } = content {
        let events: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["agent_id"], "agent-x");
        assert_eq!(events[0]["event_type"], "test_event");
        assert_eq!(events[0]["project"], "proj-1");
    } else {
        panic!("Expected text block");
    }
}

#[tokio::test]
async fn test_health_handler() {
    let handler = GestaltHealthHandler;
    let args = HashMap::new();
    
    let result = handler.call(args).await.unwrap();
    assert!(!result.is_error.unwrap_or(true));
    
    let content = &result.content[0];
    if let mcp_protocol_sdk::protocol::types::ContentBlock::Text { text, .. } = content {
        let statuses: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
        // Since we're executing this in an environment where agents might not be installed,
        // it could be an empty array, but we can verify it's a valid JSON array format.
        for status in statuses {
            assert!(status.get("cli").is_some());
            assert!(status.get("version").is_some());
            assert!(status.get("cli_path").is_some());
            assert!(status.get("latency").is_some());
        }
    } else {
        panic!("Expected text block");
    }
}
