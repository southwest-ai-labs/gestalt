use chrono::{DateTime, Duration, Utc};
use gestalt_router::thinking_daemon::{should_trigger, ThinkingDaemon, ThinkingDaemonConfig};
use gestalt_router::thinking::{InsightSynthesizer, ThinkingLoop};
use gestalt_state::StateDb;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use gestalt_core::application::agent::xavier::XavierClient;

// Setup a minimal memory DB for tests.
fn setup_db() -> StateDb {
    StateDb::open(":memory:").expect("Failed to open memory db")
}

fn insert_event(db: &StateDb, event_type: &str, timestamp: DateTime<Utc>) {
    let _ = db.execute_transaction(|tx| {
        Ok(tx.execute(
            "INSERT INTO timeline (run_id, agent_id, event_type, payload, created_at, dedup_hash, project)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                "run-1",
                "agent-1",
                event_type,
                "{}",
                timestamp.to_rfc3339(),
                None::<String>,
                None::<String>
            ],
        )?)
    });
}

#[tokio::test]
async fn skips_when_fewer_than_threshold() {
    let db = setup_db();
    let now = Utc::now();
    let last_check = now - Duration::minutes(10);
    
    // Insert only 2 executions (which is less than default 3)
    insert_event(&db, "run_started", now - Duration::minutes(5));
    insert_event(&db, "run_finished", now - Duration::minutes(4));

    // Verify should_trigger returns false
    let trigger = should_trigger(&db, last_check, 3);
    assert!(!trigger, "Should not trigger with fewer than 3 events");
}

#[tokio::test]
async fn triggers_when_threshold_reached() {
    let db = setup_db();
    let now = Utc::now();
    let last_check = now - Duration::minutes(10);
    
    // Insert 3 executions
    insert_event(&db, "run_started", now - Duration::minutes(5));
    insert_event(&db, "run_started", now - Duration::minutes(4));
    insert_event(&db, "run_finished", now - Duration::minutes(3));

    // Verify should_trigger returns true
    let trigger = should_trigger(&db, last_check, 3);
    assert!(trigger, "Should trigger with 3 or more events");
}

struct DummySynthesizer;
#[async_trait::async_trait]
impl InsightSynthesizer for DummySynthesizer {
    async fn synthesize(&self, _executions: &[String]) -> Result<String, String> {
        Ok("dummy insight".to_string())
    }
}

#[tokio::test]
async fn cancels_cleanly() {
    let db = Arc::new(setup_db());
    let token = CancellationToken::new();
    
    let config = ThinkingDaemonConfig {
        interval: std::time::Duration::from_millis(100),
        min_count: 3,
        cancel_token: token.clone(),
    };

    let daemon = ThinkingDaemon::new(config);
    
    let xavier = Arc::new(XavierClient::new("http://localhost:12345".to_string(), "".to_string()).unwrap());
    let synthesizer = Arc::new(DummySynthesizer);
    let loop_obj = Arc::new(ThinkingLoop::new(xavier, synthesizer));

    // Cancel token immediately or soon
    let daemon_task = tokio::spawn({
        let db = db.clone();
        let loop_obj = loop_obj.clone();
        async move {
            daemon.run(db, loop_obj).await;
        }
    });

    token.cancel();
    
    // Test that the daemon actually shuts down and does not hang
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), daemon_task).await;
    assert!(result.is_ok(), "Daemon did not shut down in time upon cancellation");
}
