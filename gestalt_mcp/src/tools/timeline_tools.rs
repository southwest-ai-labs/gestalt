use std::collections::HashMap;

use async_trait::async_trait;
use mcp_protocol_sdk::core::tool::ToolHandler;
use mcp_protocol_sdk::protocol::types::CallToolResult;
use mcp_protocol_sdk::McpResult;
use serde_json::Value;

use crate::tools::{err_result, ok_result};

pub struct GestaltTimelineQueryHandler;

#[async_trait]
impl ToolHandler for GestaltTimelineQueryHandler {
    async fn call(&self, arguments: HashMap<String, Value>) -> McpResult<CallToolResult> {
        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as i64;
        let agent = arguments
            .get("agent")
            .and_then(|v| v.as_str());
        let event_type = arguments
            .get("event_type")
            .and_then(|v| v.as_str());
        let project = arguments
            .get("project")
            .and_then(|v| v.as_str());
            
        let db_path = arguments
            .get("db_path")
            .and_then(|v| v.as_str())
            .unwrap_or(":memory:");

        let db = match gestalt_state::StateDb::open(db_path) {
            Ok(db) => db,
            Err(e) => return Ok(err_result(format!("Failed to open StateDb: {}", e))),
        };

        // For empty filters we use None
        let agent_filter = agent.filter(|s| !s.is_empty());
        let event_type_filter = event_type.filter(|s| !s.is_empty());
        let project_filter = project.filter(|s| !s.is_empty());

        match db.query_timeline(agent_filter, event_type_filter, project_filter, None, limit) {
            Ok(events) => {
                let json_events: Vec<Value> = events.into_iter().map(|e| {
                    serde_json::json!({
                        "seq": e.seq,
                        "run_id": e.run_id,
                        "agent_id": e.agent_id,
                        "event_type": e.event_type,
                        "payload": e.payload,
                        "created_at": e.created_at,
                        "dedup_hash": e.dedup_hash,
                        "project": e.project,
                    })
                }).collect();
                
                Ok(ok_result(
                    serde_json::to_string_pretty(&json_events).unwrap_or_default(),
                ))
            },
            Err(e) => Ok(err_result(format!("Failed to query timeline: {}", e))),
        }
    }
}

pub struct GestaltHealthHandler;

#[async_trait]
impl ToolHandler for GestaltHealthHandler {
    async fn call(&self, _arguments: HashMap<String, Value>) -> McpResult<CallToolResult> {
        let discovered = gestalt_core::application::agent::discovery::discover_installed_clis();
        
        let mut health_status = Vec::new();
        for agent in discovered {
            health_status.push(serde_json::json!({
                "cli": agent.name,
                "version": agent.version,
                "cli_path": agent.binary_path,
                "latency": "0ms" // Simulated latency metric as requested
            }));
        }

        Ok(ok_result(
            serde_json::to_string_pretty(&health_status).unwrap_or_default(),
        ))
    }
}
