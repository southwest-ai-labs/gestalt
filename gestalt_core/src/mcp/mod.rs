//! MCP (Model Context Protocol) Module
//!
//! Provides MCP client interfaces and tool execution.
//! Canonical outbound adapter: `crate::adapters::mcp::McpClientAdapter`
//! implementing `crate::ports::outbound::mcp_client::McpClientPort`.

pub mod client_impl;
pub mod registry;

// Re-export canonical outbound adapter & port
pub use crate::adapters::mcp::client_impl::McpClientAdapter;
pub use crate::ports::outbound::mcp_client::{McpClientPort, ResourceDefinition, ToolDefinition};

#[allow(deprecated)]
pub use client_impl::{GestaltMcpClient, McpClient, McpError, ToolCall, ToolInfo, ToolResult};

pub use registry::{DefaultToolContext, McpRegistry, ToolContext};
