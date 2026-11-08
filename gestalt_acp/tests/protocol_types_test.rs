use gestalt_acp::protocol::types::*;
use serde_json::json;

#[test]
fn test_jsonrpc_request_serialization() {
    let req = JsonRpcRequest::new(
        Id::Number(1),
        "initialize".to_string(),
        Some(json!({ "foo": "bar" })),
    );

    let serialized = serde_json::to_string(&req).unwrap();
    assert!(serialized.contains(r#""jsonrpc":"2.0""#));
    assert!(serialized.contains(r#""id":1"#));
    assert!(serialized.contains(r#""method":"initialize""#));
    assert!(serialized.contains(r#""params":{"foo":"bar"}"#));

    let deserialized: JsonRpcRequest = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, req);
}

#[test]
fn test_jsonrpc_request_no_params() {
    let req = JsonRpcRequest::new(
        Id::String("req-2".to_string()),
        "shutdown".to_string(),
        None,
    );

    let serialized = serde_json::to_string(&req).unwrap();
    assert!(!serialized.contains("params"));

    let deserialized: JsonRpcRequest = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, req);
}

#[test]
fn test_jsonrpc_response_serialization() {
    let res = JsonRpcResponse::result(Id::Number(2), json!({"success": true}));

    let serialized = serde_json::to_string(&res).unwrap();
    assert!(serialized.contains(r#""jsonrpc":"2.0""#));
    assert!(serialized.contains(r#""id":2"#));
    assert!(serialized.contains(r#""result":{"success":true}"#));
    assert!(!serialized.contains("error"));

    let deserialized: JsonRpcResponse = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, res);
}

#[test]
fn test_jsonrpc_error_serialization() {
    let err = JsonRpcError {
        code: -32600,
        message: "Invalid Request".to_string(),
        data: None,
    };
    let res = JsonRpcResponse::error(Id::Number(3), err);

    let serialized = serde_json::to_string(&res).unwrap();
    assert!(serialized.contains(r#""jsonrpc":"2.0""#));
    assert!(serialized.contains(r#""id":3"#));
    assert!(!serialized.contains("result"));
    assert!(serialized.contains(r#""error":{"code":-32600,"message":"Invalid Request"}"#));

    let deserialized: JsonRpcResponse = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, res);
}

#[test]
fn test_jsonrpc_notification_serialization() {
    let notif = JsonRpcNotification::new("session/cancel".to_string(), Some(json!({"sessionId": "abc"})));

    let serialized = serde_json::to_string(&notif).unwrap();
    assert!(serialized.contains(r#""jsonrpc":"2.0""#));
    assert!(serialized.contains(r#""method":"session/cancel""#));
    assert!(serialized.contains(r#""params":{"sessionId":"abc"}"#));
    assert!(!serialized.contains("id"));

    let deserialized: JsonRpcNotification = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, notif);
}

#[test]
fn test_initialize_params() {
    let params = InitializeParams {
        capabilities: ClientCapabilities {
            tools: Some(true),
            resources: None,
            prompt: Some(false),
        },
        client_info: ClientInfo {
            name: "TestClient".to_string(),
            version: "1.0.0".to_string(),
        },
    };

    let serialized = serde_json::to_string(&params).unwrap();
    let deserialized: InitializeParams = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, params);
    assert_eq!(deserialized.capabilities.tools, Some(true));
    assert_eq!(deserialized.capabilities.resources, None);
    assert_eq!(deserialized.client_info.name, "TestClient");
}

#[test]
fn test_new_session_params() {
    let params = NewSessionParams {
        workspace_root: "/path/to/workspace".to_string(),
        model_options: Some(ModelOptions {
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
        }),
    };

    let serialized = serde_json::to_string(&params).unwrap();
    let deserialized: NewSessionParams = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, params);
    assert_eq!(deserialized.workspace_root, "/path/to/workspace");
    assert_eq!(deserialized.model_options.unwrap().model, "gpt-4");
}

#[test]
fn test_prompt_params() {
    let params = PromptParams {
        session_id: "session-123".to_string(),
        message: "Hello world".to_string(),
        context_items: Some(vec![ContextItem {
            r#type: "text".to_string(),
            content: "some text context".to_string(),
        }]),
    };

    let serialized = serde_json::to_string(&params).unwrap();
    let deserialized: PromptParams = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, params);
    assert_eq!(deserialized.context_items.unwrap().len(), 1);
}

#[test]
fn test_cancel_params() {
    let params = CancelParams {
        session_id: "session-456".to_string(),
        reason: Some("User requested".to_string()),
    };

    let serialized = serde_json::to_string(&params).unwrap();
    let deserialized: CancelParams = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized, params);
}
