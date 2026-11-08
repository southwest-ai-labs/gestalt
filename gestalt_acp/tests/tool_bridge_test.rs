use gestalt_acp::tools::mcp_bridge::AcpMcpBridge;
use serde_json::json;

#[tokio::test]
async fn test_list_tools() {
    let bridge = AcpMcpBridge::new().await.expect("Failed to create bridge");
    let tools = bridge.list_tools().await.expect("Failed to list tools");
    
    assert!(!tools.is_empty(), "Should list some tools");
    
    let has_echo = tools.iter().any(|t| t["name"] == "echo");
    assert!(has_echo, "Should have echo tool");
    
    let echo_tool = tools.iter().find(|t| t["name"] == "echo").unwrap();
    assert!(echo_tool["inputSchema"].is_object(), "Should have input schema");
}

#[tokio::test]
async fn test_execute_echo_tool() {
    let bridge = AcpMcpBridge::new().await.expect("Failed to create bridge");
    let result = bridge.execute_tool("echo", json!({"message": "Hello ACP"})).await;
    
    match result {
        Ok(res) => {
            assert_eq!(res["content"], "Hello ACP");
        }
        Err(e) => panic!("Expected Ok, got Err: {}", e),
    }
}

#[tokio::test]
async fn test_unknown_tool() {
    let bridge = AcpMcpBridge::new().await.expect("Failed to create bridge");
    let result = bridge.execute_tool("unknown_tool_xyz", json!({})).await;
    
    match result {
        Ok(_) => panic!("Expected error for unknown tool"),
        Err(e) => assert!(e.contains("not found") || e.contains("failed"), "Error should mention tool not found: {}", e),
    }
}
