use std::path::{Path, PathBuf};

/// Inject the Claude hook by delegating to `crate::observe::inject::inject_claude_settings`.
pub fn inject_claude_hook(claude_dir: &Path, command: &str) -> std::io::Result<PathBuf> {
    crate::observe::inject::inject_claude_settings(claude_dir, command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_inject_claude_hook() {
        let dir = tempdir().unwrap();
        let cmd = "echo test";
        let path = inject_claude_hook(dir.path(), cmd).unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();

        let root: serde_json::Value = serde_json::from_str(&content).unwrap();
        let hooks = root.get("hooks").unwrap();
        let post_run = hooks.get("post_run").unwrap().as_array().unwrap();

        assert!(post_run.iter().any(|v| v.as_str() == Some(cmd)));
    }
}
