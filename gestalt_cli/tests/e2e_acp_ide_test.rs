// Test describe: End-to-end ACP IDE protocol integration test suite
// it("should perform a full ACP protocol lifecycle")

use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;
use serde_json::{json, Value};

use gestalt_acp::session::manager::AcpSessionManager;
use gestalt_acp::transport::http::AcpHttpTransport;

#[tokio::test]
async fn it_should_satisfy_acp_protocol_lifecycle() {
    let session_manager = Arc::new(AcpSessionManager::new());
    let transport = AcpHttpTransport::new(session_manager);
    let app: Router = transport.into_router();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", addr);

    // 1. Initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "ide-client",
                "version": "1.0"
            }
        }
    });

    let init_res: Value = client.post(format!("{}/acp/rpc", base_url))
        .json(&init_req)
        .send().await.unwrap().json().await.unwrap();
    
    assert_eq!(init_res["jsonrpc"], "2.0");
    assert_eq!(init_res["id"], 1);
    assert!(init_res["result"]["capabilities"].is_object(), "initialize should return capabilities");

    // 2. session/new
    let session_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "session/new",
        "params": {}
    });

    let session_res: Value = client.post(format!("{}/acp/rpc", base_url))
        .json(&session_req)
        .send().await.unwrap().json().await.unwrap();

    assert_eq!(session_res["id"], 2);
    let session_id = session_res["result"]["sessionId"].as_str().unwrap();
    assert!(!session_id.is_empty(), "session/new should create a valid session ID");

    // 3. session/prompt
    let prompt_req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "session/prompt",
        "params": {
            "sessionId": session_id,
            "prompt": "Hello world"
        }
    });

    let prompt_res: Value = client.post(format!("{}/acp/rpc", base_url))
        .json(&prompt_req)
        .send().await.unwrap().json().await.unwrap();

    assert_eq!(prompt_res["id"], 3);
    assert_eq!(prompt_res["result"]["status"], "success", "session/prompt should execute successfully");

    // 4. session/cancel
    let cancel_req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "session/cancel",
        "params": {
            "sessionId": session_id
        }
    });

    let cancel_res: Value = client.post(format!("{}/acp/rpc", base_url))
        .json(&cancel_req)
        .send().await.unwrap().json().await.unwrap();

    assert_eq!(cancel_res["id"], 4);
    assert_eq!(cancel_res["result"]["status"], "cancelled", "session/cancel should clean up session state");
}
