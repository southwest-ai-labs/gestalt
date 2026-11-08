use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallHooksResult {
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
    pub dry_run: bool,
}

pub fn install_hooks(dry_run: bool) -> Result<InstallHooksResult, Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    let mut installed = Vec::new();
    let mut skipped = Vec::new();

    let hook_cmd = "gestalt bus post --agent $AGENT --type telemetry";

    // 1. Claude
    let claude_dir = home.join(".claude");
    if claude_dir.exists() {
        if !dry_run {
            match crate::observe::injectors::claude_hook::inject_claude_hook(&claude_dir, hook_cmd) {
                Ok(_) => installed.push("claude".to_string()),
                Err(e) => skipped.push(format!("claude: {}", e)),
            }
        } else {
            installed.push("claude (dry-run)".to_string());
        }
    } else {
        skipped.push("claude: directory not found".to_string());
    }

    // 2. OpenCode
    let opencode_dir = home.join(".config").join("opencode");
    if opencode_dir.exists() {
        if !dry_run {
            match crate::observe::injectors::opencode_hook::inject_opencode_hook(&opencode_dir, hook_cmd) {
                Ok(_) => installed.push("opencode".to_string()),
                Err(e) => skipped.push(format!("opencode: {}", e)),
            }
        } else {
            installed.push("opencode (dry-run)".to_string());
        }
    } else {
        skipped.push("opencode: directory not found".to_string());
    }

    // 3. Hermes
    let hermes_dir = home.join(".hermes");
    if hermes_dir.exists() {
        if !dry_run {
            match crate::observe::injectors::hermes_agy::hermes_hook::inject_hermes_hooks(&hermes_dir, hook_cmd) {
                Ok(_) => installed.push("hermes".to_string()),
                Err(e) => skipped.push(format!("hermes: {}", e)),
            }
        } else {
            installed.push("hermes (dry-run)".to_string());
        }
    } else {
        skipped.push("hermes: directory not found".to_string());
    }

    // 4. AgY
    let agy_dir = home.join(".gemini").join("antigravity");
    if agy_dir.exists() {
        if !dry_run {
            match crate::observe::injectors::hermes_agy::agy_hook::inject_agy_hooks(&agy_dir, hook_cmd) {
                Ok(_) => installed.push("agy".to_string()),
                Err(e) => skipped.push(format!("agy: {}", e)),
            }
        } else {
            installed.push("agy (dry-run)".to_string());
        }
    } else {
        skipped.push("agy: directory not found".to_string());
    }

    Ok(InstallHooksResult {
        installed,
        skipped,
        dry_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_hooks_dry_run() {
        let res = install_hooks(true).expect("dry run should always succeed");
        assert!(res.dry_run);
    }
}
