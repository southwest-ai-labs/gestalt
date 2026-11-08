#[path = "../src/admin_telemetry.rs"]
pub mod admin_telemetry;
#[path = "../src/api_routes/mod.rs"]
pub mod api_routes;
#[path = "../src/bus.rs"]
pub mod bus;
#[path = "../src/task_api.rs"]
pub mod task_api;
#[path = "../src/xavier_plugin/mod.rs"]
pub mod xavier_plugin;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use gestalt_state::StateDb;
use std::sync::Arc;
use tower_service::Service;

#[tokio::test]
async fn test_admin_status_endpoint() {
    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    let db = Arc::new(StateDb::open(tmp_db.path()).unwrap());
    let state = bus::BusState { db, sink: None };

    let mut app = task_api::router().with_state(state);

    let req = Request::builder()
        .method("GET")
        .uri("/api/admin/status")
        .body(Body::empty())
        .unwrap();

    let response = app.call(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let res: admin_telemetry::AdminStatusResponse = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(res.service, "gestalt");
    assert_eq!(res.status, "healthy");
    assert_eq!(res.active_worktrees, 0);
    assert_eq!(res.connected_ws_clients, 0);
}
