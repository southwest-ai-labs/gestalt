use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Safe-merge OpenCode hooks.
/// Ensures a root `hooks` array exists and appends the `new_hook_cmd` if it is not already present.
pub fn merge_opencode_hooks(existing: &str, new_hook_cmd: &str) -> String {
    let mut root: Value = serde_json::from_str(existing).unwrap_or_else(|_| json!({}));

    if !root.is_object() {
        root = json!({});
    }

    let root_obj = root.as_object_mut().unwrap();

    let hooks_arr = root_obj
        .entry("hooks".to_string())
        .or_insert_with(|| json!([]));

    if !hooks_arr.is_array() {
        *hooks_arr = json!([]);
    }

    let arr = hooks_arr.as_array_mut().unwrap();

    let already_exists = arr.iter().any(|v| v.as_str() == Some(new_hook_cmd));
    if !already_exists {
        arr.push(Value::String(new_hook_cmd.to_string()));
    }

    serde_json::to_string_pretty(&root).unwrap_or_else(|_| existing.to_string())
}

/// Inject the OpenCode hook into `hooks.json` under the given directory.
pub fn inject_opencode_hook(opencode_dir: &Path, new_hook_cmd: &str) -> std::io::Result<PathBuf> {
    fs::create_dir_all(opencode_dir)?;
    let hooks_file = opencode_dir.join("hooks.json");
    let existing = if hooks_file.exists() {
        fs::read_to_string(&hooks_file)?
    } else {
        String::new()
    };
    let merged = merge_opencode_hooks(&existing, new_hook_cmd);
    fs::write(&hooks_file, merged)?;
    Ok(hooks_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_merge_opencode_hooks() {
        let existing = r#"{"hooks": ["existing_hook"]}"#;
        let new_cmd = "new_hook";
        let merged = merge_opencode_hooks(existing, new_cmd);

        let root: Value = serde_json::from_str(&merged).unwrap();
        let hooks = root.get("hooks").unwrap().as_array().unwrap();
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0].as_str().unwrap(), "existing_hook");
        assert_eq!(hooks[1].as_str().unwrap(), "new_hook");

        // Test idempotency
        let merged_again = merge_opencode_hooks(&merged, new_cmd);
        let root_again: Value = serde_json::from_str(&merged_again).unwrap();
        let hooks_again = root_again.get("hooks").unwrap().as_array().unwrap();
        assert_eq!(hooks_again.len(), 2);
    }

    #[test]
    fn test_inject_opencode_hook() {
        let dir = tempdir().unwrap();
        let cmd = "echo test";
        let path = inject_opencode_hook(dir.path(), cmd).unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();

        let root: Value = serde_json::from_str(&content).unwrap();
        let hooks = root.get("hooks").unwrap().as_array().unwrap();
        assert!(hooks.iter().any(|v| v.as_str() == Some(cmd)));
    }
}
