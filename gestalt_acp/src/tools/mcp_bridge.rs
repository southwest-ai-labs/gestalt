use anyhow::Result;
use mcp_protocol_sdk::server::McpServer;
use serde_json::{json, Value};

pub struct AcpMcpBridge {
    server: McpServer,
}

impl AcpMcpBridge {
    pub async fn new() -> Result<Self> {
        let server = McpServer::new("gestalt-acp".to_string(), "1.0.0".to_string());
        gestalt_mcp::tools::register_standard_tools(&server).await?;
        Ok(Self { server })
    }

    pub async fn list_tools(&self) -> Result<Vec<Value>> {
        let tools = self.server.list_tools().await?;
        let mut acp_tools = Vec::new();
        for tool in tools {
            acp_tools.push(json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema,
            }));
        }
        Ok(acp_tools)
    }

    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        let arguments = if args.is_null() {
            None
        } else if let Value::Object(map) = args {
            let mut arg_map = std::collections::HashMap::new();
            for (k, v) in map {
                arg_map.insert(k, v);
            }
            Some(arg_map)
        } else {
            return Err("Arguments must be an object".to_string());
        };

        match self.server.call_tool(name, arguments).await {
            Ok(result) => {
                let is_error = result.is_error.unwrap_or(false);
                let mut text_content = String::new();
                for block in result.content {
                    if let mcp_protocol_sdk::protocol::types::ContentBlock::Text { text, .. } = block {
                        text_content.push_str(&text);
                    }
                }
                
                if is_error {
                    Err(text_content)
                } else {
                    Ok(json!({ "content": text_content }))
                }
            }
            Err(e) => Err(format!("Tool execution failed: {}", e)),
        }
    }
}
