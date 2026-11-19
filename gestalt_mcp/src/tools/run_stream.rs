use async_trait::async_trait;
use mcp_protocol_sdk::core::tool::ToolHandler;
use mcp_protocol_sdk::protocol::types::{CallToolResult, ContentBlock};
use mcp_protocol_sdk::McpResult;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::tools::err_result;

pub const FORBIDDEN_METACHARS: &[char] = &[';', '|', '&', '$', '(', ')', '`', '>', '<', '\n', '\r'];
pub const MAX_OUTPUT_BYTES: usize = 1024 * 1024; // 1 MiB
pub const TRUNCATION_WARNING: &str = "\n[WARNING: Output exceeded 1 MiB limit and was truncated]\n";
pub const DEFAULT_ALLOWED_COMMANDS: &[&str] =
    &["echo", "git", "cargo", "ls", "cat", "grep", "diff"];
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Splits a command string into binary and arguments without shell interpolation.
/// Explicitly rejects shell metacharacters: `; | & $ ( ) ` > < \n \r`.
pub fn parse_command_tokens(cmd: &str) -> Result<Vec<String>, String> {
    for c in cmd.chars() {
        if FORBIDDEN_METACHARS.contains(&c) {
            return Err(format!(
                "Command contains forbidden shell metacharacter: {:?}",
                c
            ));
        }
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for c in cmd.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }

        if c == '\\' {
            escaped = true;
            continue;
        }

        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }

        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }

        if c.is_whitespace() && !in_single_quote && !in_double_quote {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
        } else {
            current.push(c);
        }
    }

    if in_single_quote || in_double_quote {
        return Err("Unclosed quote in command".to_string());
    }

    if escaped {
        current.push('\\');
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    if tokens.is_empty() {
        return Err("Command is empty".to_string());
    }

    Ok(tokens)
}

/// Retrieves list of allowed command binaries from `GESTALT_ALLOWED_COMMANDS` or defaults.
pub fn get_allowed_commands() -> Vec<String> {
    if let Ok(val) = std::env::var("GESTALT_ALLOWED_COMMANDS") {
        val.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        DEFAULT_ALLOWED_COMMANDS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

/// Checks whether the binary is in the allowlist.
pub fn is_command_allowed(binary: &str, allowed: &[String]) -> bool {
    let has_path_separator = binary.contains('/') || binary.contains('\\');
    allowed.iter().any(|cmd| {
        if cmd == binary {
            true
        } else if !has_path_separator {
            cmd == binary
        } else {
            false
        }
    })
}

/// Resolves working directory and verifies workspace root confinement.
pub fn resolve_working_directory(cwd_arg: Option<&str>) -> Result<std::path::PathBuf, String> {
    let workspace_root = if let Ok(root_str) = std::env::var("GESTALT_WORKSPACE_ROOT") {
        let p = std::path::PathBuf::from(&root_str);
        if !p.exists() {
            return Err(format!(
                "GESTALT_WORKSPACE_ROOT '{}' does not exist",
                root_str
            ));
        }
        std::fs::canonicalize(&p)
            .map_err(|e| format!("Invalid GESTALT_WORKSPACE_ROOT '{}': {}", root_str, e))?
    } else {
        std::env::current_dir()
            .and_then(std::fs::canonicalize)
            .map_err(|e| format!("Failed to resolve current directory: {}", e))?
    };

    if let Some(cwd_str) = cwd_arg {
        let cwd_str = cwd_str.trim();
        if !cwd_str.is_empty() {
            let target_path = if std::path::Path::new(cwd_str).is_absolute() {
                std::path::PathBuf::from(cwd_str)
            } else {
                workspace_root.join(cwd_str)
            };

            let canonical_cwd = std::fs::canonicalize(&target_path)
                .map_err(|e| format!("Invalid working directory '{}': {}", cwd_str, e))?;

            if !canonical_cwd.starts_with(&workspace_root) {
                return Err(format!(
                    "Path traversal detected: working directory '{}' is outside workspace root '{}'",
                    canonical_cwd.display(),
                    workspace_root.display()
                ));
            }

            return Ok(canonical_cwd);
        }
    }

    Ok(workspace_root)
}

/// Retrieves execution timeout duration.
pub fn get_command_timeout(arguments: &HashMap<String, Value>) -> Duration {
    let secs = arguments
        .get("timeout")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            std::env::var("GESTALT_COMMAND_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
        })
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

pub struct GestaltRunStreamHandler;

#[async_trait]
impl ToolHandler for GestaltRunStreamHandler {
    async fn call(&self, arguments: HashMap<String, Value>) -> McpResult<CallToolResult> {
        let (binary, args) = if let Some(prog) = arguments.get("program").and_then(|v| v.as_str()) {
            if prog.is_empty() {
                return Ok(err_result("program cannot be empty".to_string()));
            }
            if prog.chars().any(|c| FORBIDDEN_METACHARS.contains(&c)) {
                return Ok(err_result(
                    "Program contains forbidden shell metacharacters".to_string(),
                ));
            }
            let prog_trimmed = prog.trim();
            if prog_trimmed.is_empty() {
                return Ok(err_result("program cannot be empty".to_string()));
            }
            let args_vec = arguments
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();
            (prog_trimmed.to_string(), args_vec)
        } else if let Some(cmd) = arguments.get("command").and_then(|v| v.as_str()) {
            if cmd.trim().is_empty() {
                return Ok(err_result("command is required".to_string()));
            }
            match parse_command_tokens(cmd) {
                Ok(tokens) => {
                    let bin = tokens[0].clone();
                    let args = tokens[1..].to_vec();
                    (bin, args)
                },
                Err(e) => return Ok(err_result(e)),
            }
        } else {
            return Ok(err_result("command or program is required".to_string()));
        };

        // 1. Enforce command allowlist
        let allowed = get_allowed_commands();
        if !is_command_allowed(&binary, &allowed) {
            return Ok(err_result(format!(
                "Command '{}' is not in the allowed commands list: {:?}",
                binary, allowed
            )));
        }

        // 2. Enforce working-directory confinement
        let cwd_arg = arguments.get("cwd").and_then(|v| v.as_str());
        let execution_dir = match resolve_working_directory(cwd_arg) {
            Ok(dir) => dir,
            Err(e) => return Ok(err_result(e)),
        };

        // 3. Execution timeout
        let timeout_duration = get_command_timeout(&arguments);

        // Spawn structured process without shell interpolation
        let mut child = match tokio::process::Command::new(&binary)
            .args(&args)
            .current_dir(execution_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                return Ok(err_result(format!(
                    "Failed to spawn process '{}': {}",
                    binary, e
                )))
            },
        };

        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stderr = child.stderr.take().expect("Failed to open stderr");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut output = String::new();
        let mut stdout_done = false;
        let mut stderr_done = false;
        let mut truncated = false;

        let stream_future = async {
            loop {
                tokio::select! {
                    line = stdout_reader.next_line(), if !stdout_done => {
                        match line {
                            Ok(Some(line)) => {
                                let line_len = line.len() + 1;
                                if output.len() + line_len > MAX_OUTPUT_BYTES {
                                    let remaining = MAX_OUTPUT_BYTES.saturating_sub(output.len());
                                    if remaining > 0 {
                                        let mut take_len = 0;
                                        for (idx, _) in line.char_indices() {
                                            if idx <= remaining {
                                                take_len = idx;
                                            } else {
                                                break;
                                            }
                                        }
                                        output.push_str(&line[..take_len]);
                                    }
                                    output.push_str(TRUNCATION_WARNING);
                                    truncated = true;
                                    break;
                                } else {
                                    output.push_str(&line);
                                    output.push('\n');
                                }
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
                                let line_len = line.len() + 1;
                                if output.len() + line_len > MAX_OUTPUT_BYTES {
                                    let remaining = MAX_OUTPUT_BYTES.saturating_sub(output.len());
                                    if remaining > 0 {
                                        let mut take_len = 0;
                                        for (idx, _) in line.char_indices() {
                                            if idx <= remaining {
                                                take_len = idx;
                                            } else {
                                                break;
                                            }
                                        }
                                        output.push_str(&line[..take_len]);
                                    }
                                    output.push_str(TRUNCATION_WARNING);
                                    truncated = true;
                                    break;
                                } else {
                                    output.push_str(&line);
                                    output.push('\n');
                                }
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

            if truncated {
                let _ = child.kill().await;
            }

            let status = match child.wait().await {
                Ok(s) => s.success(),
                Err(e) => {
                    output.push_str(&format!("Failed to wait for process: {}\n", e));
                    false
                },
            };

            (output, status, truncated)
        };

        match tokio::time::timeout(timeout_duration, stream_future).await {
            Ok((out, success, is_trunc)) => {
                let is_error = if is_trunc { false } else { !success };
                Ok(CallToolResult {
                    content: vec![ContentBlock::text(out)],
                    is_error: Some(is_error),
                    structured_content: None,
                    meta: None,
                })
            },
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                Ok(err_result(format!(
                    "Command timed out after {} seconds",
                    timeout_duration.as_secs()
                )))
            },
        }
    }
}
