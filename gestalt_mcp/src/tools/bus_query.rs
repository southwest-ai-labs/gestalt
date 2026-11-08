use std::collections::HashMap;

use async_trait::async_trait;
use mcp_protocol_sdk::McpResult;
use mcp_protocol_sdk::core::tool::ToolHandler;
use mcp_protocol_sdk::protocol::types::{ContentBlock, CallToolResult};
use serde_json::{json, Value};

fn ok_result(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::text(text)],
        is_error: Some(false),
        structured_content: None,
        meta: None,
    }
}

pub struct BusQueryHandler;

#[async_trait]
impl ToolHandler for BusQueryHandler {
    async fn call(&self, arguments: HashMap<String, Value>) -> McpResult<CallToolResult> {
        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);
        let agent = arguments
            .get("agent")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let event_type = arguments
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let project = arguments
            .get("project")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // In a full implementation, this would query the local StateDb or
        // the event bus HTTP endpoint (`/api/events`). For now, we return
        // a mocked stub response that demonstrates the integration surface.

        let result = json!({
            "limit": limit,
            "agent": agent,
            "event_type": event_type,
            "project": project,
            "events": [
                {
                    "seq": 1,
                    "agent": if agent.is_empty() { "system" } else { agent },
                    "event_type": if event_type.is_empty() { "run_started" } else { event_type },
                    "project": project,
                    "timestamp": 1690000000
                }
            ]
        });

        Ok(ok_result(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bus_query_handler() {
        let handler = BusQueryHandler;
        let mut args = HashMap::new();
        args.insert("limit".to_string(), json!(5));
        args.insert("agent".to_string(), json!("jules"));

        let result = handler.call(args).await.unwrap();
        assert!(!result.is_error.unwrap_or(false));
        assert_eq!(result.content.len(), 1);

        let text = match &result.content[0] {
            ContentBlock::Text { text, .. } => text,
            _ => panic!("Expected Text ContentBlock"),
        };

        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["limit"], 5);
        assert_eq!(parsed["agent"], "jules");
        assert_eq!(parsed["events"][0]["agent"], "jules");
    }
}
