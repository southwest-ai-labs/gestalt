// Integration tests verifying Bearer authentication and payload limits on Gestalt bus endpoints.
//
// describe("Bus Bearer Auth & Payload Limit Integration Suite", || {
//     it("returns 401 on /api/event when GESTALT_API_TOKEN is set and auth is missing")
//     it("succeeds on /api/event when valid Bearer token is provided")
//     it("returns 200 on /healthz without auth even when GESTALT_API_TOKEN is set")
//     it("enforces DefaultBodyLimit on oversized payloads")
// })

#[path = "../src/bus.rs"]
pub mod bus;
#[path = "../src/api_routes/mod.rs"]
pub mod api_routes;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use bus::{build_router, BusState};
use gestalt_state::StateDb;
use std::sync::Arc;
use tower::ServiceExt;

static TEST_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test]
async fn test_api_event_unauthorized_when_token_set() {
    let _guard = TEST_ENV_MUTEX.lock().unwrap();
    std::env::set_var("GESTALT_API_TOKEN", "gestalt-test-auth-token-32-chars-long-here");

    let db = Arc::new(StateDb::open(":memory:").unwrap());
    let app = build_router(BusState { db, sink: None });

    let body = serde_json::json!({
        "agent": "test-agent",
        "event_type": "checkpoint",
        "summary": "testing unauth"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/event")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    std::env::remove_var("GESTALT_API_TOKEN");

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_event_authorized_with_valid_bearer_token() {
    let _guard = TEST_ENV_MUTEX.lock().unwrap();
    let token = "gestalt-test-auth-token-32-chars-long-here";
    std::env::set_var("GESTALT_API_TOKEN", token);

    let db = Arc::new(StateDb::open(":memory:").unwrap());
    let app = build_router(BusState { db, sink: None });

    let body = serde_json::json!({
        "agent": "test-agent",
        "event_type": "checkpoint",
        "summary": "testing auth success"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/event")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    std::env::remove_var("GESTALT_API_TOKEN");

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_healthz_endpoint_remains_unauthenticated() {
    let _guard = TEST_ENV_MUTEX.lock().unwrap();
    std::env::set_var("GESTALT_API_TOKEN", "gestalt-test-auth-token-32-chars-long-here");

    let db = Arc::new(StateDb::open(":memory:").unwrap());
    let app = build_router(BusState { db, sink: None });

    let req = Request::builder()
        .method("GET")
        .uri("/healthz")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    std::env::remove_var("GESTALT_API_TOKEN");

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_payload_limit_mitigates_exhaustion() {
    let _guard = TEST_ENV_MUTEX.lock().unwrap();
    let token = "gestalt-test-auth-token-32-chars-long-here";
    std::env::set_var("GESTALT_API_TOKEN", token);

    let db = Arc::new(StateDb::open(":memory:").unwrap());
    let app = build_router(BusState { db, sink: None });

    // Payload strictly exceeding 2 MiB (2 * 1024 * 1024 + 10 bytes)
    let oversized = vec![b'a'; (2 * 1024 * 1024) + 10];
    let req = Request::builder()
        .method("POST")
        .uri("/api/event")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::from(oversized))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    std::env::remove_var("GESTALT_API_TOKEN");

    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn test_validate_bind_host_and_token() {
    let _guard = TEST_ENV_MUTEX.lock().unwrap();

    // Loopback should always succeed without token
    std::env::remove_var("GESTALT_API_TOKEN");
    assert!(bus::validate_bind_host_and_token("127.0.0.1").is_ok());
    assert!(bus::validate_bind_host_and_token("localhost").is_ok());

    // Non-loopback should fail without token
    assert!(bus::validate_bind_host_and_token("0.0.0.0").is_err());

    // Non-loopback should fail with short token (< 32 chars)
    std::env::set_var("GESTALT_API_TOKEN", "too-short");
    assert!(bus::validate_bind_host_and_token("0.0.0.0").is_err());

    // Non-loopback should succeed with >= 32 chars token
    std::env::set_var("GESTALT_API_TOKEN", "12345678901234567890123456789012");
    assert!(bus::validate_bind_host_and_token("0.0.0.0").is_ok());

    std::env::remove_var("GESTALT_API_TOKEN");
}

