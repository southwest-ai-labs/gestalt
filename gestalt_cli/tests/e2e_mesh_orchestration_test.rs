// Test describe: Gestalt E2E Multi-Agent Mesh Orchestration
// it("should orchestrate task dispatch, stream interception, persistence, and provider health checks")

use std::sync::Arc;
use tokio::net::TcpListener;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::io::{duplex, AsyncWriteExt};

#[path = "../src/bus.rs"]
pub mod bus;
#[path = "../src/task_api.rs"]
pub mod task_api;
#[path = "../src/admin_telemetry.rs"]
pub mod admin_telemetry;
#[path = "../src/api_routes/mod.rs"]
pub mod api_routes;
#[path = "../src/xavier_plugin/mod.rs"]
pub mod xavier_plugin;

pub use gestalt_core::application;

use bus::BusState;
use gestalt_state::StateDb;
use gestalt_router::event_bus::BusEvent;
use gestalt_router::observe::stream_interceptor::StreamInterceptor;

#[tokio::test]
async fn it_should_orchestrate_full_multi_agent_mesh_pipeline() {
    // 1. Initialize temporary SQLite database with StateDb
    let db = Arc::new(StateDb::open(":memory:").unwrap());
    let state = BusState {
        db: db.clone(),
        sink: None,
    };

    // 2. Launch Axum router with event bus and task API
    let bus_app = bus::build_router(state.clone());
    let task_app = task_api::router().with_state(state.clone());
    let app = axum::Router::new().merge(bus_app).merge(task_app);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = Client::new();
    let base_url = format!("http://127.0.0.1:{}", port);
    
    let payload = json!({
        "task": "Test task request from E2E suite",
        "capabilities": [],
        "max_parallel": 2
    });
    
    let res = client
        .post(format!("{}/api/task", base_url))
        .json(&payload)
        .send()
        .await
        .unwrap();
        
    assert!(res.status().is_success(), "Failed to dispatch task: {:?}", res.status());
    let resp_json: Value = res.json().await.unwrap();
    assert!(resp_json.get("run_id").is_some());
    let run_id = resp_json["run_id"].as_str().unwrap().to_string();

    // 4. Stream Interception
    let (mut out_w, out_r) = duplex(1024);
    let (mut err_w, err_r) = duplex(1024);

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let writer_handle = tokio::spawn(async move {
        out_w.write_all(b"mock intercepted stdout line 1\nmock stdout line 2\n").await.unwrap();
        err_w.write_all(b"mock stderr line 1\n").await.unwrap();
        drop(out_w);
        drop(err_w);
    });

    // Execute the interceptor on the background to fill the channel
    let agent_id = "test-agent-orchestrator";
    StreamInterceptor::pipe_to_channel(agent_id, &run_id, Some(out_r), Some(err_r), tx).await;
    writer_handle.await.unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = rx.recv().await {
        chunks.push(chunk);
    }

    assert_eq!(chunks.len(), 3, "Expected 3 stream chunks to be intercepted");

    // 5. Emit intercepted chunks to the bus endpoint as BusEvents
    for chunk in chunks {
        let ev = BusEvent::new(
            &chunk.agent_id,
            "stream_chunk",
            &chunk.chunk
        )
        .with_run_id(chunk.run_id.clone())
        .with_metadata(json!({
            "stream": chunk.stream
        }));

        let emit_res = client
            .post(format!("{}/api/event", base_url))
            .json(&ev)
            .send()
            .await
            .unwrap();

        assert!(emit_res.status().is_success(), "Failed to emit stream chunk event to bus");
    }

    // 6. Verify Database and Provider Health Check
    let health_res = client
        .get(format!("{}/api/health/providers?force=true", base_url))
        .send()
        .await
        .unwrap();

    let status = health_res.status();
    assert!(
        status == reqwest::StatusCode::OK || status == reqwest::StatusCode::SERVICE_UNAVAILABLE,
        "Expected 200 or 503 for providers health, got: {}",
        status
    );
    let health_json: Value = health_res.json().await.unwrap();
    assert!(health_json.get("status").is_some());

    // 7. Assert events are present in timeline endpoint
    let events_res = client
        .get(format!("{}/api/events?limit=100", base_url))
        .send()
        .await
        .unwrap();

    assert!(events_res.status().is_success(), "Failed to fetch events from API");
    let events_json: Value = events_res.json().await.unwrap();
    let count = events_json["count"].as_u64().unwrap();
    
    // We expect at least the 3 stream chunks and the "run_started" event emitted by task dispatch
    assert!(count >= 4, "Expected at least 4 events in the database, found {}", count);

    // Filter JSON events array to find the stream chunks
    let stream_chunks = events_json["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["event_type"].as_str().unwrap() == "stream_chunk")
        .count();

    assert_eq!(stream_chunks, 3, "Expected to find 3 stream_chunk events in the database");
}
