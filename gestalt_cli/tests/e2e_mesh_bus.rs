// Test describe: Gestalt E2E Multi-Agent Mesh Bus Integration Tests
// it("should handle concurrent event emission and deduplication across multiple synthetic agents")

use gestalt_router::event_bus::{handle_event, BusEvent};
use gestalt_state::StateDb;
use std::sync::Arc;
use tokio::task;

#[tokio::test]
async fn it_should_handle_concurrent_mesh_events_with_dedup() {
    // 1. Set up an in-memory StateDb for testing.
    let db = Arc::new(StateDb::open(":memory:").unwrap());

    // 2. Define our 4 synthetic agents.
    let agents = vec!["agent-alpha", "agent-beta", "agent-gamma", "agent-delta"];

    let mut tasks = Vec::new();

    for agent in agents {
        let db_clone = Arc::clone(&db);
        let agent_name = agent.to_string();

        let task = task::spawn(async move {
            let mut emitted = 0;
            let mut deduplicated = 0;
            let mut persisted = 0;

            let run_id = format!("{}-run-1", agent_name);
            let project = "test-mesh-project".to_string();

            let ev_start_1 = BusEvent::new(&agent_name, "run_started", "Agent started")
                .with_run_id(&run_id)
                .with_project(&project)
                .with_state("Running")
                .with_ts(chrono::Utc::now().to_rfc3339());
            
            let ev_prog = BusEvent::new(&agent_name, "checkpoint", "Agent checkpoint 1")
                .with_run_id(&run_id)
                .with_project(&project)
                .with_state("Running")
                .with_ts(chrono::Utc::now().to_rfc3339());
            
            let ev_fin = BusEvent::new(&agent_name, "run_finished", "Agent finished")
                .with_run_id(&run_id)
                .with_project(&project)
                .with_state("Success")
                .with_ts(chrono::Utc::now().to_rfc3339());

            let ev_start_dup_1 = BusEvent::new(&agent_name, "run_started", "Agent started")
                .with_run_id(&run_id)
                .with_project(&project)
                .with_state("Running")
                .with_ts(chrono::Utc::now().to_rfc3339());

            let ev_start_dup_2 = BusEvent::new(&agent_name, "run_started", "Agent started")
                .with_run_id(&run_id)
                .with_project(&project)
                .with_state("Running")
                .with_ts(chrono::Utc::now().to_rfc3339());

            let events_to_emit = vec![
                ev_start_1,
                ev_start_dup_1,
                ev_prog,
                ev_start_dup_2,
                ev_fin,
            ];

            for ev in events_to_emit {
                emitted += 1;
                match handle_event(&db_clone, &ev, None).await {
                    Ok(Some(_seq)) => persisted += 1,
                    Ok(None) => deduplicated += 1,
                    Err(e) => panic!("Failed to handle event: {}", e),
                }
            }

            (emitted, persisted, deduplicated)
        });

        tasks.push(task);
    }

    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.unwrap());
    }

    let mut total_emitted = 0;
    let mut total_persisted = 0;
    let mut total_deduplicated = 0;

    for (emitted, persisted, deduplicated) in results {
        assert_eq!(emitted, 5, "Each agent should emit 5 events");
        assert_eq!(
            persisted, 3,
            "Each agent should have 3 unique events persisted"
        );
        assert_eq!(
            deduplicated, 2,
            "Each agent should have 2 duplicates deduplicated"
        );

        total_emitted += emitted;
        total_persisted += persisted;
        total_deduplicated += deduplicated;
    }

    assert_eq!(
        total_emitted, 20,
        "Total emitted across 4 agents should be 20"
    );
    assert_eq!(
        total_persisted, 12,
        "Total persisted across 4 agents should be 12"
    );
    assert_eq!(
        total_deduplicated, 8,
        "Total deduplicated across 4 agents should be 8"
    );

    // Instead of querying `get_timeline` with integers which is for run_ids,
    // we query using `recent_timeline` or `get_events_older_than` to get all events.
    // Given the memory context "StateDb::get_timeline in gestalt-state returns timeline events ordered by seq ASC LIMIT ?2, whereas recent_timeline reads the timeline in reverse sequence orders",
    // wait, get_timeline takes run_id and limit. Let's use recent_timeline, which takes limit.
    let timeline_events = db.recent_timeline(100).unwrap();
    assert_eq!(
        timeline_events.len(),
        12,
        "Timeline should have exactly 12 events recorded"
    );

    let mut agents_seen = std::collections::HashSet::new();
    for ev in timeline_events {
        if let Some(agent_id) = ev.agent_id {
            agents_seen.insert(agent_id);
        }
    }

    assert_eq!(
        agents_seen.len(),
        4,
        "Events from all 4 agents should be present in the DB"
    );
    assert!(agents_seen.contains("agent-alpha"));
    assert!(agents_seen.contains("agent-beta"));
    assert!(agents_seen.contains("agent-gamma"));
    assert!(agents_seen.contains("agent-delta"));
}
