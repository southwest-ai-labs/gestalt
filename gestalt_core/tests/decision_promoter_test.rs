use gestalt_core::application::agent::xavier::{
    DecisionRecord, DecisionStatus,
};

#[test]
fn test_schema_validation() {
    let record = DecisionRecord {
        title: "Use Rust".to_string(),
        rationale: "Memory safety".to_string(),
        status: DecisionStatus::Proposed,
        author: "Jules".to_string(),
        insight_id: "insight-123".to_string(),
        timestamp: "2023-01-01T00:00:00Z".to_string(),
    };

    assert_eq!(record.title, "Use Rust");
    assert_eq!(record.rationale, "Memory safety");
    assert_eq!(record.status, DecisionStatus::Proposed);
    assert_eq!(record.author, "Jules");
    assert_eq!(record.insight_id, "insight-123");
    assert_eq!(record.timestamp, "2023-01-01T00:00:00Z");
}

#[test]
fn test_status_transitions() {
    let mut record = DecisionRecord {
        title: "Use Rust".to_string(),
        rationale: "Memory safety".to_string(),
        status: DecisionStatus::Proposed,
        author: "Jules".to_string(),
        insight_id: "insight-123".to_string(),
        timestamp: "2023-01-01T00:00:00Z".to_string(),
    };

    assert_eq!(record.status, DecisionStatus::Proposed);
    
    record.status = DecisionStatus::Accepted;
    assert_eq!(record.status, DecisionStatus::Accepted);
    
    record.status = DecisionStatus::Superseded;
    assert_eq!(record.status, DecisionStatus::Superseded);
}

#[test]
fn test_json_serialization() {
    let record = DecisionRecord {
        title: "Use Rust".to_string(),
        rationale: "Memory safety".to_string(),
        status: DecisionStatus::Accepted,
        author: "Jules".to_string(),
        insight_id: "insight-123".to_string(),
        timestamp: "2023-01-01T00:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&record).unwrap();
    assert!(json.contains(r#""title":"Use Rust""#));
    assert!(json.contains(r#""status":"Accepted""#));
    assert!(json.contains(r#""insight_id":"insight-123""#));

    let deserialized: DecisionRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.title, record.title);
    assert_eq!(deserialized.status, DecisionStatus::Accepted);
}
