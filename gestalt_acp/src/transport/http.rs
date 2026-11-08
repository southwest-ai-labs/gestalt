use axum::{
    extract::{Json, State},
    response::sse::{Event, Sse},
    routing::{get, post},
    Router,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

pub struct AcpState {
    pub tx: broadcast::Sender<String>,
}

pub fn acp_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(AcpState { tx });

    Router::new()
        .route("/acp/rpc", post(handle_rpc))
        .route("/acp/events", get(handle_sse))
        .with_state(state)
        .layer(tower_http::cors::CorsLayer::permissive())
}

async fn handle_rpc(
    State(_state): State<Arc<AcpState>>,
    Json(payload): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: payload.id,
        result: Some(Value::String("ok".to_string())),
        error: None,
    })
}

async fn handle_sse(
    State(state): State<Arc<AcpState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).map(|msg| match msg {
        Ok(msg) => Ok(Event::default().data(msg)),
        Err(_) => Ok(Event::default().data("")),
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
