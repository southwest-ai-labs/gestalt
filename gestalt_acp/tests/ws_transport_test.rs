use axum::serve;
use gestalt_acp::session::manager::AcpSessionManager;
use gestalt_acp::transport::ws::AcpWsTransport;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use futures::{SinkExt, StreamExt};
use serde_json::json;

#[tokio::test]
async fn test_ws_transport() {
    let session_manager = Arc::new(AcpSessionManager::new());
    let transport = AcpWsTransport::new(session_manager);
    let router = transport.into_router();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        serve(listener, router).await.unwrap();
    });

    let ws_url = format!("ws://{}/acp/ws", addr);
    let (mut ws_stream, _) = connect_async(ws_url).await.expect("Failed to connect");

    // Test initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    ws_stream.send(Message::Text(init_req.to_string().into())).await.unwrap();

    let msg = ws_stream.next().await.unwrap().unwrap();
    if let Message::Text(text) = msg {
        let resp: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(resp["id"], 1);
        assert!(resp["result"]["capabilities"]["tools"].as_bool().unwrap());
    } else {
        panic!("Expected text message");
    }

    // Test session/new
    let session_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "session/new",
        "params": {}
    });
    ws_stream.send(Message::Text(session_req.to_string().into())).await.unwrap();

    let msg = ws_stream.next().await.unwrap().unwrap();
    if let Message::Text(text) = msg {
        let resp: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(resp["id"], 2);
        assert!(resp["result"]["sessionId"].as_str().is_some());
    } else {
        panic!("Expected text message");
    }

    server.abort();
}
