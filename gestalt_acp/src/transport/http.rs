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
    pub session_manager: Option<Arc<crate::session::manager::AcpSessionManager>>,
}

pub struct AcpHttpTransport {
    session_manager: Arc<crate::session::manager::AcpSessionManager>,
}

impl AcpHttpTransport {
    pub fn new(session_manager: Arc<crate::session::manager::AcpSessionManager>) -> Self {
        Self { session_manager }
    }

    pub fn into_router(self) -> Router {
        let (tx, _) = broadcast::channel(100);
        let state = Arc::new(AcpState {
            tx,
            session_manager: Some(self.session_manager),
        });

        Router::new()
            .route("/acp/rpc", post(handle_rpc))
            .route("/acp/events", get(handle_sse))
            .with_state(state)
            .layer(tower_http::cors::CorsLayer::permissive())
    }
}

pub fn acp_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(AcpState { tx, session_manager: None });

    Router::new()
        .route("/acp/rpc", post(handle_rpc))
        .route("/acp/events", get(handle_sse))
        .with_state(state)
        .layer(tower_http::cors::CorsLayer::permissive())
}

async fn handle_rpc(
    State(state): State<Arc<AcpState>>,
    Json(payload): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let result = match payload.method.as_str() {
        "initialize" => serde_json::json!({
            "capabilities": {
                "tools": true,
                "prompts": true
            }
        }),
        "session/new" => {
            let session_id = if let Some(mgr) = &state.session_manager {
                let params = crate::session::manager::SessionParams {
                    workspace_root: ".".to_string(),
                };
                mgr.create_session(params)
            } else {
                uuid::Uuid::new_v4().to_string()
            };
            serde_json::json!({
                "sessionId": session_id
            })
        },
        "session/prompt" => serde_json::json!({
            "status": "success"
        }),
        "session/cancel" => {
            if let Some(mgr) = &state.session_manager {
                if let Some(params) = &payload.params {
                    if let Some(sid) = params.get("sessionId").and_then(|s| s.as_str()) {
                        let _ = mgr.cancel_session(sid);
                    }
                }
            }
            serde_json::json!({
                "status": "cancelled"
            })
        },
        _ => Value::String("ok".to_string()),
    };

    Json(JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: payload.id,
        result: Some(result),
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
