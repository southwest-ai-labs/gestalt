use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AgentCard {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub capabilities: Vec<String>,
    pub endpoint: String,
}

pub const AGENT_JSON_PATH: &str = "/.well-known/agent.json";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_agent_card_serialization() {
        let card = AgentCard {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            description: Some("A test agent".to_string()),
            capabilities: vec!["test".to_string()],
            endpoint: "http://localhost:8080".to_string(),
        };

        let json = serde_json::to_value(&card).unwrap();
        assert_eq!(
            json,
            json!({
                "id": "test-agent",
                "name": "Test Agent",
                "description": "A test agent",
                "capabilities": ["test"],
                "endpoint": "http://localhost:8080"
            })
        );
    }
}
