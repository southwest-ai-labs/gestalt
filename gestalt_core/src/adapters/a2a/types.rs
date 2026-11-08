use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TaskPayload {
    pub id: String,
    pub task: String,
    pub payload: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_task_payload_serialization() {
        let payload = TaskPayload {
            id: "task-123".to_string(),
            task: "do_something".to_string(),
            payload: Some(json!({"param": "value"})),
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(
            json,
            json!({
                "id": "task-123",
                "task": "do_something",
                "payload": {"param": "value"}
            })
        );
    }
}
