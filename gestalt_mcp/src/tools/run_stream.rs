use async_trait::async_trait;
use mcp_protocol_sdk::core::tool::ToolHandler;
use mcp_protocol_sdk::protocol::types::{CallToolResult, ContentBlock};
use mcp_protocol_sdk::McpResult;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::tools::err_result;

pub struct GestaltRunStreamHandler;

#[async_trait]
impl ToolHandler for GestaltRunStreamHandler {
    async fn call(&self, arguments: HashMap<String, Value>) -> McpResult<CallToolResult> {
        let cmd = arguments
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if cmd.is_empty() {
            return Ok(err_result("command is required".to_string()));
        }

        let mut child = match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return Ok(err_result(format!("Failed to spawn process: {}", e))),
        };

        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stderr = child.stderr.take().expect("Failed to open stderr");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut output = String::new();

        let mut stdout_done = false;
        let mut stderr_done = false;

        loop {
            tokio::select! {
                line = stdout_reader.next_line(), if !stdout_done => {
                    match line {
                        Ok(Some(line)) => {
                            output.push_str(&line);
                            output.push('\n');
                        }
                        Ok(None) => stdout_done = true,
                        Err(e) => {
                            output.push_str(&format!("Error reading stdout: {}\n", e));
                            stdout_done = true;
                        }
                    }
                }
                line = stderr_reader.next_line(), if !stderr_done => {
                    match line {
                        Ok(Some(line)) => {
                            output.push_str(&line);
                            output.push('\n');
                        }
                        Ok(None) => stderr_done = true,
                        Err(e) => {
                            output.push_str(&format!("Error reading stderr: {}\n", e));
                            stderr_done = true;
                        }
                    }
                }
            }
            if stdout_done && stderr_done {
                break;
            }
        }

        let status = match child.wait().await {
            Ok(s) => s,
            Err(e) => return Ok(err_result(format!("Failed to wait for process: {}", e))),
        };

        Ok(CallToolResult {
            content: vec![ContentBlock::text(output)],
            is_error: Some(!status.success()),
            structured_content: None,
            meta: None,
        })
    }
}
