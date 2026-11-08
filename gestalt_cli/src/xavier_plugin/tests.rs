use crate::xavier_plugin::manifest::XavierPluginManifest;
use axum::{body::Body, http::Request};
use tower::ServiceExt;

#[tokio::test]
async fn test_manifest_endpoint() {
    let app = crate::xavier_plugin::router::<()>();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/plugin/manifest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let manifest: XavierPluginManifest = serde_json::from_slice(&body).unwrap();
    assert_eq!(manifest.name, "gestalt");
    assert!(manifest.capabilities.contains(&"task_routing".to_string()));
}
