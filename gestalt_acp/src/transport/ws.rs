use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::transport::http::{JsonRpcRequest, JsonRpcResponse};

pub struct AcpWsState {
    pub tx: broadcast::Sender<String>,
    pub session_manager: Option<Arc<crate::session::manager::AcpSessionManager>>,
}

pub struct AcpWsTransport {
    session_manager: Arc<crate::session::manager::AcpSessionManager>,
}

impl AcpWsTransport {
    pub fn new(session_manager: Arc<crate::session::manager::AcpSessionManager>) -> Self {
        Self { session_manager }
    }

    pub fn into_router(self) -> Router {
        let (tx, _) = broadcast::channel(100);
        let state = Arc::new(AcpWsState {
            tx,
            session_manager: Some(self.session_manager),
        });

        Router::new()
            .route("/acp/ws", get(ws_handler))
            .with_state(state)
    }
}

pub fn acp_ws_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(AcpWsState {
        tx,
        session_manager: None,
    });

    Router::new()
        .route("/acp/ws", get(ws_handler))
        .with_state(state)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AcpWsState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AcpWsState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();
    
    // We use an mpsc channel to send messages from the receiver task
    // back to the sender task, avoiding broadcasting responses to all clients.
    let (mpsc_tx, mut mpsc_rx) = tokio::sync::mpsc::channel::<Message>(100);

    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(msg) = rx.recv() => {
                    if sender.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Some(msg) = mpsc_rx.recv() => {
                    if sender.send(msg).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let response = match serde_json::from_str::<JsonRpcRequest>(&text) {
                        Ok(req) => handle_rpc_request(&state, req).await,
                        Err(_) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: Value::Null,
                            result: None,
                            error: Some(serde_json::json!({
                                "code": -32700,
                                "message": "Parse error"
                            })),
                        },
                    };

                    let response_text = match serde_json::to_string(&response) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let _ = mpsc_tx.send(Message::Text(response_text.into())).await;
                }
                Message::Ping(data) => {
                    let _ = mpsc_tx.send(Message::Pong(data)).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

async fn handle_rpc_request(state: &AcpWsState, req: JsonRpcRequest) -> JsonRpcResponse {
    let result = match req.method.as_str() {
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
                if let Some(params) = &req.params {
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

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: req.id,
        result: Some(result),
        error: None,
    }
}
