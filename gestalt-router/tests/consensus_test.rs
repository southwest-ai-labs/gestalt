use std::sync::Arc;
use gestalt_state::StateDb;
use gestalt_router::consensus::{ConsensusRunner, ConsensusVerdict};

// Custom tempdir to avoid `tempfile` since memory noted it's not a direct dependency
struct TempDir {
    path: std::path::PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("gestalt-consensus-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[tokio::test]
async fn test_approval() {
    let temp = TempDir::new();
    let db = Arc::new(StateDb::open(temp.path().join("db.sqlite")).unwrap());
    
    let verdict = ConsensusVerdict::Approved;
    let result = ConsensusRunner::evaluate(&db, "diff1", &verdict).await.unwrap();
    assert_eq!(result, ConsensusVerdict::Approved);
    
    let events = db.get_events_older_than(chrono::Utc::now() + chrono::Duration::days(1)).unwrap();
    assert!(events.iter().any(|e| e.event_type == "consensus_approved" && e.payload.contains("Consensus Approved")));
}

#[tokio::test]
async fn test_rejection() {
    let temp = TempDir::new();
    let db = Arc::new(StateDb::open(temp.path().join("db.sqlite")).unwrap());
    
    let verdict = ConsensusVerdict::Rejected { feedback: "needs work".to_string() };
    let result = ConsensusRunner::evaluate(&db, "diff2", &verdict).await.unwrap();
    assert_eq!(result, ConsensusVerdict::Rejected { feedback: "needs work".to_string() });
    
    let events = db.get_events_older_than(chrono::Utc::now() + chrono::Duration::days(1)).unwrap();
    assert!(events.iter().any(|e| e.event_type == "consensus_rejected" && e.payload.contains("needs work")));
}

#[tokio::test]
async fn test_conflict() {
    let temp = TempDir::new();
    let db = Arc::new(StateDb::open(temp.path().join("db.sqlite")).unwrap());
    
    let verdict = ConsensusVerdict::Conflict { reason: "divergent views".to_string() };
    let result = ConsensusRunner::evaluate(&db, "diff3", &verdict).await.unwrap();
    assert_eq!(result, ConsensusVerdict::Conflict { reason: "divergent views".to_string() });
    
    let events = db.get_events_older_than(chrono::Utc::now() + chrono::Duration::days(1)).unwrap();
    assert!(events.iter().any(|e| e.event_type == "consensus_conflict" && e.payload.contains("divergent views")));
}
