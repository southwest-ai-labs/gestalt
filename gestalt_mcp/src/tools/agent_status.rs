use async_trait::async_trait;
use mcp_protocol_sdk::core::tool::ToolHandler;
use mcp_protocol_sdk::protocol::types::CallToolResult;
use mcp_protocol_sdk::McpResult;
use serde_json::Value;
use std::collections::HashMap;

use crate::tools::{err_result, ok_result};

pub struct GestaltAgentStatusHandler;

#[async_trait]
impl ToolHandler for GestaltAgentStatusHandler {
    async fn call(&self, arguments: HashMap<String, Value>) -> McpResult<CallToolResult> {
        let cli_name = arguments
            .get("cli_name")
            .and_then(|v| v.as_str())
            .unwrap_or("gestalt");

        // Whitelist to prevent arbitrary executable invocation
        let allowed_clis = ["gestalt", "hermes", "jules", "orca", "claude"];
        if !allowed_clis.contains(&cli_name) {
            return Ok(err_result(format!(
                "CLI '{}' is not an allowed agent CLI. Allowed: {:?}",
                cli_name, allowed_clis
            )));
        }

        let output = std::process::Command::new(cli_name)
            .arg("--version")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let status = serde_json::json!({
                    "cli": cli_name,
                    "status": "Installed",
                    "version": version
                });
                Ok(ok_result(
                    serde_json::to_string_pretty(&status).unwrap_or_default(),
                ))
            },
            Ok(out) => Ok(err_result(format!(
                "CLI {} executed but failed: {}",
                cli_name,
                String::from_utf8_lossy(&out.stderr)
            ))),
            Err(e) => Ok(err_result(format!(
                "Failed to execute CLI {}: {}",
                cli_name, e
            ))),
        }
    }
}
