use gestalt_mcp::tools::run_stream::GestaltRunStreamHandler;
use mcp_protocol_sdk::core::tool::ToolHandler;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_run_stream() {
    let handler = GestaltRunStreamHandler;
    
    // Test missing command
    let args = HashMap::new();
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    
    // Test valid command
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("echo Hello World"));
    let result = handler.call(args).await.unwrap();
    assert!(!result.is_error.unwrap_or(true));
}
