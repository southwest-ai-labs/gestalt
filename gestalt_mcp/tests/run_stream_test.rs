#![allow(unsafe_code)]

use gestalt_mcp::tools::run_stream::{
    GestaltRunStreamHandler, FORBIDDEN_METACHARS,
};
use mcp_protocol_sdk::core::tool::ToolHandler;
use mcp_protocol_sdk::protocol::types::{CallToolResult, ContentBlock};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

// Global mutex for tests that modify environment variables
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn get_result_text(result: &CallToolResult) -> String {
    if let Some(ContentBlock::Text { text, .. }) = result.content.first() {
        text.clone()
    } else {
        String::new()
    }
}

#[tokio::test]
async fn test_run_stream_missing_input() {
    let handler = GestaltRunStreamHandler;

    // Test missing command / program
    let args = HashMap::new();
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));

    // Test empty command
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!(""));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));

    // Test whitespace command
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("   "));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
}

/// (a) Disallowed binary rejection
#[tokio::test]
async fn test_disallowed_binary_rejection() {
    let handler = GestaltRunStreamHandler;

    // Python is not allowlisted
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("python3 script.py"));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    let text = get_result_text(&result);
    assert!(
        text.contains("not in the allowed commands list"),
        "Unexpected error: {}",
        text
    );

    // Shell invocation directly is not allowlisted
    let mut args = HashMap::new();
    args.insert("program".to_string(), json!("sh"));
    args.insert("args".to_string(), json!(["-c", "echo hello"]));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    let text = get_result_text(&result);
    assert!(
        text.contains("not in the allowed commands list"),
        "Unexpected error: {}",
        text
    );

    // Dangerous binaries like rm, whoami, curl are rejected
    for bin in &["rm", "whoami", "curl", "bash", "powershell"] {
        let mut args = HashMap::new();
        args.insert("command".to_string(), json!(format!("{} foo", bin)));
        let result = handler.call(args).await.unwrap();
        assert!(result.is_error.unwrap_or(false), "Expected {} to be rejected", bin);
    }
}

/// (b) Shell-metacharacter rejection
#[tokio::test]
async fn test_shell_metacharacter_rejection() {
    let handler = GestaltRunStreamHandler;

    // Verify rejection of all specified shell metacharacters: ; | & $ ( ) ` > < \n \r
    let test_payloads = [
        ("echo hello; rm -rf /", ';'),
        ("echo hello | grep h", '|'),
        ("echo hello && echo world", '&'),
        ("echo $USER", '$'),
        ("echo (subshell)", '('),
        ("echo subshell)", ')'),
        ("echo `id`", '`'),
        ("echo hello > out.txt", '>'),
        ("cat < in.txt", '<'),
        ("echo line1\nline2", '\n'),
        ("echo line1\rline2", '\r'),
    ];

    for (payload, _char) in test_payloads {
        let mut args = HashMap::new();
        args.insert("command".to_string(), json!(payload));
        let result = handler.call(args).await.unwrap();
        assert!(
            result.is_error.unwrap_or(false),
            "Expected payload {:?} to be rejected",
            payload
        );
        let text = get_result_text(&result);
        assert!(
            text.contains("forbidden shell metacharacter"),
            "Error message should mention forbidden metacharacters for payload {:?}, got: {}",
            payload,
            text
        );
    }

    // Also verify FORBIDDEN_METACHARS are checked in `program`
    for c in FORBIDDEN_METACHARS {
        let mut args = HashMap::new();
        args.insert("program".to_string(), json!(format!("echo{}", c)));
        let result = handler.call(args).await.unwrap();
        assert!(
            result.is_error.unwrap_or(false),
            "Expected program with {:?} to be rejected",
            c
        );
    }
}

/// (c) Path traversal cwd rejection
#[tokio::test]
async fn test_path_traversal_cwd_rejection() {
    let handler = GestaltRunStreamHandler;

    // Traverse upwards past workspace root
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("echo Hello"));
    args.insert("cwd".to_string(), json!("../../../../../.."));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    let text = get_result_text(&result);
    assert!(
        text.contains("outside workspace root") || text.contains("Path traversal"),
        "Unexpected error: {}",
        text
    );

    // Absolute path traversal outside workspace root
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("echo Hello"));
    args.insert("cwd".to_string(), json!("/etc"));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    let text = get_result_text(&result);
    assert!(
        text.contains("outside workspace root") || text.contains("Path traversal"),
        "Unexpected error: {}",
        text
    );

    // Nonexistent directory traversal
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("echo Hello"));
    args.insert("cwd".to_string(), json!("does_not_exist_traversal/../../.."));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
}

/// (d) Timeout enforcement
#[tokio::test]
async fn test_timeout_enforcement() {
    let _guard = ENV_LOCK.lock().unwrap();
    let handler = GestaltRunStreamHandler;

    // Allow 'sleep' for the purpose of timeout test
    unsafe {
        std::env::set_var("GESTALT_ALLOWED_COMMANDS", "echo,git,cargo,ls,cat,grep,diff,sleep");
        std::env::set_var("GESTALT_COMMAND_TIMEOUT_SECS", "1");
    }

    // Test timeout via GESTALT_COMMAND_TIMEOUT_SECS env var
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("sleep 5"));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    let text = get_result_text(&result);
    assert!(
        text.contains("timed out after 1 seconds"),
        "Expected timeout message, got: {}",
        text
    );

    // Test timeout override via argument
    let mut args = HashMap::new();
    args.insert("program".to_string(), json!("sleep"));
    args.insert("args".to_string(), json!(["5"]));
    args.insert("timeout".to_string(), json!(1));
    let result = handler.call(args).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    let text = get_result_text(&result);
    assert!(
        text.contains("timed out"),
        "Expected timeout message, got: {}",
        text
    );

    // Clean up env vars
    unsafe {
        std::env::remove_var("GESTALT_ALLOWED_COMMANDS");
        std::env::remove_var("GESTALT_COMMAND_TIMEOUT_SECS");
    }
}

/// (e) Safe allowlisted command success
#[tokio::test]
async fn test_safe_allowlisted_command_success() {
    let handler = GestaltRunStreamHandler;

    // 1. Backwards-compatible command string
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("echo Hello World"));
    let result = handler.call(args).await.unwrap();
    assert!(!result.is_error.unwrap_or(true));
    let text = get_result_text(&result);
    assert!(text.contains("Hello World"));

    // 2. Structured arguments execution
    let mut args = HashMap::new();
    args.insert("program".to_string(), json!("echo"));
    args.insert("args".to_string(), json!(["Structured", "Execution", "Works"]));
    let result = handler.call(args).await.unwrap();
    assert!(!result.is_error.unwrap_or(true));
    let text = get_result_text(&result);
    assert!(text.contains("Structured Execution Works"));

    // 3. Allowlisted command inside valid workspace cwd
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("echo Inside Cwd"));
    args.insert("cwd".to_string(), json!("."));
    let result = handler.call(args).await.unwrap();
    assert!(!result.is_error.unwrap_or(true));
    let text = get_result_text(&result);
    assert!(text.contains("Inside Cwd"));
}

/// Output byte cap truncation
#[tokio::test]
async fn test_output_byte_cap_truncation() {
    let handler = GestaltRunStreamHandler;

    // Create a temporary file larger than 1 MiB inside current directory
    use std::io::Write;
    let mut tmp = tempfile::Builder::new()
        .prefix("test_large_")
        .suffix(".txt")
        .tempfile_in(".")
        .unwrap();

    let chunk = "A".repeat(1024) + "\n";
    let total_chunks = 1200; // ~1.2 MiB
    for _ in 0..total_chunks {
        tmp.write_all(chunk.as_bytes()).unwrap();
    }
    tmp.flush().unwrap();

    let path_str = tmp.path().file_name().unwrap().to_str().unwrap().to_string();

    let mut args = HashMap::new();
    args.insert("command".to_string(), json!(format!("cat {}", path_str)));
    let result = handler.call(args).await.unwrap();
    let text = get_result_text(&result);

    // Verify warning is appended
    assert!(
        text.contains("[WARNING: Output exceeded 1 MiB limit and was truncated]"),
        "Expected truncation warning"
    );
}
