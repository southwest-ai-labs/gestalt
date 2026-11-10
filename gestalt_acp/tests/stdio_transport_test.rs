use gestalt_acp::transport::stdio::StdioTransport;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::time::Duration;

#[tokio::test]
async fn test_valid_request() {
    let (client, server) = tokio::io::duplex(1024);
    let (server_reader, server_writer) = tokio::io::split(server);
    let mut transport = StdioTransport::new(server_reader, server_writer);

    let handler = |req: serde_json::Value| async move {
        // Echo back a response modifying the method
        req.get("method").and_then(|m| m.as_str()).map(|method| {
            serde_json::json!({
                "jsonrpc": "2.0",
                "result": format!("handled: {}", method),
                "id": req.get("id").unwrap_or(&serde_json::Value::Null)
            })
        })
    };

    let server_task = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), transport.run_loop(handler))
            .await
            .expect("Server task timed out")
            .unwrap();
    });

    let (mut client_reader, mut client_writer) = tokio::io::split(client);
    let mut client_buf_reader = BufReader::new(&mut client_reader);

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "test_method",
        "params": [],
        "id": 1
    });
    let mut req_str = serde_json::to_string(&req).unwrap();
    req_str.push('\n');
    
    client_writer.write_all(req_str.as_bytes()).await.unwrap();
    client_writer.flush().await.unwrap();

    let mut response_line = String::new();
    client_buf_reader.read_line(&mut response_line).await.unwrap();
    
    let response: serde_json::Value = serde_json::from_str(&response_line).unwrap();
    assert_eq!(response.get("result").unwrap().as_str().unwrap(), "handled: test_method");
    assert_eq!(response.get("id").unwrap().as_i64().unwrap(), 1);

    drop(client_buf_reader);
    drop(client_reader);
    drop(client_writer);
    server_task.await.unwrap();
}

#[tokio::test]
async fn test_malformed_json() {
    let (client, server) = tokio::io::duplex(1024);
    let (server_reader, server_writer) = tokio::io::split(server);
    let mut transport = StdioTransport::new(server_reader, server_writer);

    let handler = |_req: serde_json::Value| async move {
        None
    };

    let server_task = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), transport.run_loop(handler))
            .await
            .expect("Server task timed out")
            .unwrap();
    });

    let (mut client_reader, mut client_writer) = tokio::io::split(client);
    let mut client_buf_reader = BufReader::new(&mut client_reader);

    let req_str = "invalid json\n";
    client_writer.write_all(req_str.as_bytes()).await.unwrap();
    client_writer.flush().await.unwrap();

    let mut response_line = String::new();
    client_buf_reader.read_line(&mut response_line).await.unwrap();
    
    let response: serde_json::Value = serde_json::from_str(&response_line).unwrap();
    assert_eq!(response.get("jsonrpc").unwrap().as_str().unwrap(), "2.0");
    assert_eq!(response.get("id").unwrap(), &serde_json::Value::Null);
    
    let error = response.get("error").unwrap();
    assert_eq!(error.get("code").unwrap().as_i64().unwrap(), -32700);
    assert_eq!(error.get("message").unwrap().as_str().unwrap(), "Parse error");

    // Also verify loop is still running by sending a valid request
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "test_method",
        "id": 2
    });
    let mut req_str = serde_json::to_string(&req).unwrap();
    req_str.push('\n');
    
    client_writer.write_all(req_str.as_bytes()).await.unwrap();
    client_writer.flush().await.unwrap();

    // Since our handler returns None, we shouldn't get a response. We just drop the client.
    drop(client_buf_reader);
    drop(client_reader);
    drop(client_writer);
    
    // Server task should exit cleanly now
    server_task.await.unwrap();
}
