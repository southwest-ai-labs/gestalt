#[path = "../src/observe/artifact_tailer.rs"]
mod artifact_tailer;

use artifact_tailer::{ArtifactTailer, LogFormat};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

// Helper since tempfile is not available in dependencies.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("gestalt_test_{}_{}", name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_artifact_tailer_incremental() {
    let dir = TempDir::new("artifact_tailer_incremental");
    let file_path = dir.path().join("claude_session.jsonl");
    let mut tailer = ArtifactTailer::new();

    // 1. Initial read on non-existent file
    let events = tailer.tail_file(&file_path, LogFormat::ClaudeJsonl, "claude-code").unwrap();
    assert!(events.is_empty(), "Should handle missing file gracefully");

    // 2. Create file with one line
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, r#"{{"type":"message","message":"Hello from Claude"}}"#).unwrap();
    }

    let events = tailer.tail_file(&file_path, LogFormat::ClaudeJsonl, "claude-code").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].agent, "claude-code");
    assert_eq!(events[0].event_type, "message");
    assert_eq!(events[0].summary, "Hello from Claude");

    // 3. Read again without modifications - should return no new events
    let events = tailer.tail_file(&file_path, LogFormat::ClaudeJsonl, "claude-code").unwrap();
    assert!(events.is_empty());

    // 4. Append another line
    {
        let mut f = File::options().append(true).open(&file_path).unwrap();
        writeln!(f, r#"{{"type":"tool_call","message":"Calling tool x"}}"#).unwrap();
    }

    let events = tailer.tail_file(&file_path, LogFormat::ClaudeJsonl, "claude-code").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "tool_call");
    assert_eq!(events[0].summary, "Calling tool x");

    // 5. Truncate file (log rotation simulation)
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, r#"{{"type":"error","message":"Something went wrong"}}"#).unwrap();
    }

    let events = tailer.tail_file(&file_path, LogFormat::ClaudeJsonl, "claude-code").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "error");
    assert_eq!(events[0].summary, "Something went wrong");
}

#[test]
fn test_artifact_tailer_malformed_and_truncated() {
    let dir = TempDir::new("artifact_tailer_malformed");
    let file_path = dir.path().join("malformed.jsonl");
    let mut tailer = ArtifactTailer::new();

    // 1. Valid line followed by malformed line (with newline)
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, r#"{{"message":"valid generic"}}"#).unwrap();
        writeln!(f, r#"not valid json"#).unwrap();
    }

    let events = tailer.tail_file(&file_path, LogFormat::GenericJson, "generic-agent").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].summary, "valid generic");

    // 2. Append a valid line
    {
        let mut f = File::options().append(true).open(&file_path).unwrap();
        writeln!(f, r#"{{"message":"recovered"}}"#).unwrap();
    }

    let events = tailer.tail_file(&file_path, LogFormat::GenericJson, "generic-agent").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].summary, "recovered");

    // 3. Append a truncated line (no newline)
    {
        let mut f = File::options().append(true).open(&file_path).unwrap();
        write!(f, r#"{{"message":"truncated"#).unwrap();
    }

    let events = tailer.tail_file(&file_path, LogFormat::GenericJson, "generic-agent").unwrap();
    assert!(events.is_empty(), "Truncated line should not yield events");

    // 4. Complete the truncated line
    {
        let mut f = File::options().append(true).open(&file_path).unwrap();
        writeln!(f, r#""}}"#).unwrap();
    }

    let events = tailer.tail_file(&file_path, LogFormat::GenericJson, "generic-agent").unwrap();
    assert_eq!(events.len(), 1, "Should parse completed line correctly");
    assert_eq!(events[0].summary, "truncated");
}
