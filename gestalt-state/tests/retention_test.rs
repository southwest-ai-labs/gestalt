use chrono::{Duration, Utc};
use gestalt_state::retention::{apply_retention_policy, PruningPolicy};
use gestalt_state::StateDb;

fn setup_db() -> StateDb {
    StateDb::open(":memory:").expect("Failed to open in-memory DB")
}

#[test]
fn test_retention_policy_by_age() {
    let db = setup_db();
    db.create_run("run-1", "{}").unwrap();

    let old_date = Utc::now() - Duration::days(40);
    let recent_date = Utc::now() - Duration::days(10);

    // Insert manually to override created_at
    db.execute_transaction(|tx| {
        tx.execute(
            "INSERT INTO timeline (run_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["run-1", "old-event", "{}", old_date.to_rfc3339()],
        ).unwrap();
        tx.execute(
            "INSERT INTO timeline (run_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["run-1", "recent-event", "{}", recent_date.to_rfc3339()],
        ).unwrap();
        Ok(())
    }).unwrap();

    let policy = PruningPolicy::default(); // 30 days

    let pruned = db
        .execute_transaction(|tx| apply_retention_policy(tx, &policy))
        .unwrap();
    assert_eq!(pruned, 1, "Should prune exactly 1 old event");

    let events = db.timeline_by_run("run-1").unwrap();
    assert_eq!(events.len(), 1, "Should have 1 recent event left");
    assert_eq!(events[0].event_type, "recent-event");
}

#[test]
fn test_retention_policy_by_count() {
    let db = setup_db();
    db.create_run("run-1", "{}").unwrap();

    let policy = PruningPolicy {
        max_events: 5,
        max_age_days: 30,
    };

    // Insert 10 events
    db.execute_transaction(|tx| {
        for i in 1..=10 {
            tx.execute(
                "INSERT INTO timeline (run_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params!["run-1", format!("event-{}", i), "{}", Utc::now().to_rfc3339()],
            ).unwrap();
        }
        Ok(())
    }).unwrap();

    let pruned = db
        .execute_transaction(|tx| apply_retention_policy(tx, &policy))
        .unwrap();
    assert_eq!(pruned, 5, "Should prune exactly 5 events to keep max 5");

    let events = db.timeline_by_run("run-1").unwrap();
    assert_eq!(events.len(), 5, "Should have 5 events left");

    // The events kept should be the most recent ones (event-6 to event-10)
    for (i, event) in events.iter().enumerate().take(5) {
        assert_eq!(event.event_type, format!("event-{}", i + 6));
    }
}
