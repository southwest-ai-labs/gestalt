use gestalt_router::lock_reaper::LockReaper;
use gestalt_state::StateDb;
use std::time::Duration;

// Note: memory instructions state we must use a custom TempDir in these crates.
struct TempDir {
    path: std::path::PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let name = format!("gestalt-router-test-{}", uuid::Uuid::new_v4());
        let path = std::env::temp_dir().join(name);
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

#[test]
fn test_lock_reaper_evicts_correctly() {
    let tmp = TempDir::new();
    let db = StateDb::open(tmp.path().join("state.db")).unwrap();

    // Set up a run
    let run_id = "test_run_1";
    db.create_run(run_id, "{}").unwrap();

    // 1. Agent 1 is running (Active)
    db.upsert_agent(run_id, "agent_1", "running", None, None, 0, "")
        .unwrap();

    // 2. Agent 2 is crashed (Dead)
    db.upsert_agent(run_id, "agent_2", "crashed", None, None, 0, "")
        .unwrap();

    // Let's inject locks directly into the locks table using execute_transaction so we can manipulate `acquired_at`
    let now = chrono::Utc::now();
    let old_time = now - chrono::Duration::seconds(400); // Exceeds 300s TTL
    let new_time = now - chrono::Duration::seconds(10); // Within 300s TTL

    db.execute_transaction(|tx| {
        // Lock 1: Agent 1 (running) - recent lock. Should be KEPT.
        tx.execute(
            "INSERT INTO locks (path, agent_id, run_id, acquired_at, ttl_secs) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["/tmp/file1", "agent_1", run_id, new_time.to_rfc3339(), 300],
        ).unwrap();

        // Lock 2: Agent 2 (crashed) - recent lock. Should be REAPED.
        tx.execute(
            "INSERT INTO locks (path, agent_id, run_id, acquired_at, ttl_secs) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["/tmp/file2", "agent_2", run_id, new_time.to_rfc3339(), 300],
        ).unwrap();

        // Lock 3: Agent 1 (running) - old lock. Should be REAPED (expired).
        tx.execute(
            "INSERT INTO locks (path, agent_id, run_id, acquired_at, ttl_secs) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["/tmp/file3", "agent_1", run_id, old_time.to_rfc3339(), 300],
        ).unwrap();

        Ok(())
    }).unwrap();

    // Verify setup
    let initial_locks = db.get_locks().unwrap();
    assert_eq!(initial_locks.len(), 3, "Should start with 3 locks");

    // Run reaper
    let reaped = LockReaper::reap_expired_locks(&db, Duration::from_secs(300)).unwrap();
    
    // We expect exactly 2 locks to be reaped (Lock 2 and Lock 3)
    assert_eq!(reaped, 2, "Reaper should have evicted 2 locks");

    // Verify only the correct lock remains
    let remaining_locks = db.get_locks().unwrap();
    assert_eq!(remaining_locks.len(), 1, "Should have 1 lock remaining");
    assert_eq!(remaining_locks[0].path, "/tmp/file1");
    assert_eq!(remaining_locks[0].agent_id, "agent_1");

    // Verify events were emitted
    let events = db.timeline_by_run(run_id).unwrap();
    let evict_events: Vec<_> = events.iter().filter(|e| e.event_type == "lock_evicted").collect();
    assert_eq!(evict_events.len(), 2, "Should have 2 lock_evicted events");

    let mut payloads: Vec<_> = evict_events.iter().map(|e| &e.payload).collect();
    payloads.sort();
    assert!(payloads[0].contains("/tmp/file2"));
    assert!(payloads[0].contains("dead_agent"));
    
    assert!(payloads[1].contains("/tmp/file3"));
    assert!(payloads[1].contains("expired"));
}
