use gestalt_core::adapters::a2a::{A2aClient, TaskPayload};
use std::io::{Read, Write};
use std::net::TcpListener;
use serde_json::json;

fn spawn_mock_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    
    std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0; 1024];
            if let Ok(n) = stream.read(&mut buf) {
                if n == 0 { continue; }
                let req = String::from_utf8_lossy(&buf[..n]);
                if req.starts_with("GET /.well-known/agent.json") {
                    let body = r#"{"id":"test-agent","name":"Test Agent","description":"A test agent","capabilities":["test"],"endpoint":"http://localhost:8080"}"#;
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
                    let _ = stream.write_all(response.as_bytes());
                } else if req.starts_with("POST /tasks") || req.starts_with("POST /") {
                    let body = r#"{"status":"success","message":"task accepted","result":null}"#;
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
                    let _ = stream.write_all(response.as_bytes());
                } else {
                    let response = "HTTP/1.1 404 NOT FOUND\r\nContent-Length: 0\r\n\r\n";
                    let _ = stream.write_all(response.as_bytes());
                }
            }
        }
    });
    
    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn test_discover() {
    let server_url = spawn_mock_server();
    let client = A2aClient::new();
    
    let card = client.discover(&server_url).await.expect("Failed to discover agent card");
    assert_eq!(card.id, "test-agent");
    assert_eq!(card.name, "Test Agent");
    assert_eq!(card.capabilities, vec!["test".to_string()]);
}

#[tokio::test]
async fn test_submit_task() {
    let server_url = spawn_mock_server();
    let client = A2aClient::new();
    
    let payload = TaskPayload {
        id: "task-1".to_string(),
        task: "do_something".to_string(),
        payload: Some(json!({"foo": "bar"})),
    };
    
    let url = format!("{}/tasks", server_url);
    let response = client.submit_task(&url, payload).await.expect("Failed to submit task");
    
    assert_eq!(response.status, "success");
    assert_eq!(response.message.unwrap(), "task accepted");
}
