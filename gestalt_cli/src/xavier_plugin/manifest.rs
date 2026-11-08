use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XavierPluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
}

impl Default for XavierPluginManifest {
    fn default() -> Self {
        Self {
            name: "gestalt".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "Gestalt integration plugin for Xavier".to_string(),
            capabilities: vec![
                "task_routing".to_string(),
                "event_bus".to_string(),
                "state_sync".to_string(),
            ],
        }
    }
}
