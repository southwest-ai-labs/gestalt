use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use gestalt_acp::transport::http::{acp_router, JsonRpcRequest, JsonRpcResponse};
use serde_json::json;
use tower::ServiceExt; // for `oneshot` and `ready`

#[tokio::test]
async fn test_rpc_handler() {
    let app = acp_router::<()>();

    let rpc_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        method: "testMethod".to_string(),
        params: None,
    };

    let req = Request::builder()
        .method("POST")
        .uri("/acp/rpc")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&rpc_req).unwrap()))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let rpc_res: JsonRpcResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(rpc_res.jsonrpc, "2.0");
    assert_eq!(rpc_res.id, json!(1));
    assert_eq!(rpc_res.result, Some(json!("ok")));
}

#[tokio::test]
async fn test_sse_handler() {
    let app = acp_router::<()>();

    let req = Request::builder()
        .method("GET")
        .uri("/acp/events")
        .header("accept", "text/event-stream")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}
