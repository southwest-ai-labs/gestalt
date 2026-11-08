pub use gestalt_router::event_bus;
pub use gestalt_router::health;

#[path = "../src/observe/heartbeat.rs"]
mod heartbeat;

use heartbeat::AgentHeartbeatReporter;
use gestalt_state::StateDb;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_heartbeat_loop_and_cancellation() {
    let db_path = std::env::temp_dir().join(format!("gestalt-heartbeat-test-{}.db", uuid::Uuid::new_v4()));
    let db = Arc::new(StateDb::open(&db_path).unwrap());
    
    // Very fast interval for test
    let reporter = AgentHeartbeatReporter::new(db.clone(), Duration::from_millis(10), Duration::from_secs(1));
    let cancel_token = reporter.cancel_token();
    
    let handle = tokio::spawn(async move {
        reporter.run().await;
    });
    
    // Let it run for a couple of ticks
    tokio::time::sleep(Duration::from_millis(50)).await;
    cancel_token.cancel();
    
    // Wait for the loop to exit
    handle.await.unwrap();
    
    // Ensure we can query the state db for any heartbeats
    // If hermes/claude etc aren't found locally it might be 0, but the loop exited safely.
    let recent = db.recent_timeline(100).unwrap();
    for event in recent {
        if event.event_type == "agent_heartbeat" {
            println!("Found heartbeat event: {:?}", event.event_type);
        }
    }
    
    let _ = std::fs::remove_file(&db_path);
}
