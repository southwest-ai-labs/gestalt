use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Safe-merge AgY hooks while preserving existing structures and avoiding duplicates.
pub fn merge_agy_hooks(existing: &str, new_hook_cmd: &str) -> String {
    let mut root: Value = serde_json::from_str(existing).unwrap_or_else(|_| json!({}));

    if !root.is_object() {
        root = json!({});
    }

    let root_obj = root.as_object_mut().unwrap();

    let hooks_obj = root_obj
        .entry("lifecycle_hooks".to_string())
        .or_insert_with(|| json!({}));
    if !hooks_obj.is_object() {
        *hooks_obj = json!({});
    }

    let hooks_map = hooks_obj.as_object_mut().unwrap();

    for event_key in &["on_start", "on_stop"] {
        let entry = hooks_map
            .entry(event_key.to_string())
            .or_insert_with(|| json!([]));
        if !entry.is_array() {
            *entry = json!([]);
        }
        let arr = entry.as_array_mut().unwrap();

        let already_exists = arr.iter().any(|v| v.as_str() == Some(new_hook_cmd));
        if !already_exists {
            arr.push(Value::String(new_hook_cmd.to_string()));
        }
    }

    serde_json::to_string_pretty(&root).unwrap_or_else(|_| existing.to_string())
}

pub fn inject_agy_hooks(agy_dir: &Path, new_hook_cmd: &str) -> std::io::Result<PathBuf> {
    fs::create_dir_all(agy_dir)?;
    let hooks_file = agy_dir.join("config.json");
    let existing = if hooks_file.exists() {
        fs::read_to_string(&hooks_file)?
    } else {
        String::new()
    };
    let merged = merge_agy_hooks(&existing, new_hook_cmd);
    fs::write(&hooks_file, merged)?;
    Ok(hooks_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_agy_hooks_empty() {
        let merged = merge_agy_hooks("", "my_hook.sh");
        let parsed: Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(
            parsed["lifecycle_hooks"]["on_start"][0].as_str().unwrap(),
            "my_hook.sh"
        );
        assert_eq!(
            parsed["lifecycle_hooks"]["on_stop"][0].as_str().unwrap(),
            "my_hook.sh"
        );
    }

    #[test]
    fn test_merge_agy_hooks_existing() {
        let existing = json!({
            "lifecycle_hooks": {
                "on_start": ["other_hook.sh"]
            }
        }).to_string();
        let merged = merge_agy_hooks(&existing, "my_hook.sh");
        let parsed: Value = serde_json::from_str(&merged).unwrap();
        let arr_start = parsed["lifecycle_hooks"]["on_start"].as_array().unwrap();
        assert_eq!(arr_start.len(), 2);
        assert_eq!(arr_start[1].as_str().unwrap(), "my_hook.sh");
        let arr_stop = parsed["lifecycle_hooks"]["on_stop"].as_array().unwrap();
        assert_eq!(arr_stop.len(), 1);
        assert_eq!(arr_stop[0].as_str().unwrap(), "my_hook.sh");
    }

    #[test]
    fn test_merge_agy_hooks_duplicate() {
        let existing = json!({
            "lifecycle_hooks": {
                "on_start": ["my_hook.sh"],
                "on_stop": ["my_hook.sh"]
            }
        }).to_string();
        let merged = merge_agy_hooks(&existing, "my_hook.sh");
        let parsed: Value = serde_json::from_str(&merged).unwrap();
        let arr_start = parsed["lifecycle_hooks"]["on_start"].as_array().unwrap();
        assert_eq!(arr_start.len(), 1);
        let arr_stop = parsed["lifecycle_hooks"]["on_stop"].as_array().unwrap();
        assert_eq!(arr_stop.len(), 1);
    }
}
