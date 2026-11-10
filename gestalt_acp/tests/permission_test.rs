use gestalt_acp::permission::{PermissionManager, PermissionScope, PermissionError};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::test]
async fn test_permission_grant() {
    let manager = PermissionManager::new();
    let (req, rx) = manager.request_permission(
        PermissionScope::FileWrite(PathBuf::from("/tmp/test.txt")),
        "Testing file write".to_string(),
    );

    // Grant in background
    let manager_clone = manager.clone();
    let req_id = req.id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        manager_clone.grant_permission(&req_id);
    });

    let result = manager
        .wait_for_permission(&req.id, rx, Duration::from_secs(1))
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_permission_deny() {
    let manager = PermissionManager::new();
    let (req, rx) = manager.request_permission(
        PermissionScope::ShellExecution("ls -la".to_string()),
        "Testing shell execution".to_string(),
    );

    // Deny in background
    let manager_clone = manager.clone();
    let req_id = req.id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        manager_clone.deny_permission(&req_id);
    });

    let result = manager
        .wait_for_permission(&req.id, rx, Duration::from_secs(1))
        .await;

    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test]
async fn test_permission_timeout() {
    let manager = PermissionManager::new();
    let (req, rx) = manager.request_permission(
        PermissionScope::NetworkAccess("https://example.com".to_string()),
        "Testing network access".to_string(),
    );

    // Do nothing, let it timeout
    let result = manager
        .wait_for_permission(&req.id, rx, Duration::from_millis(10))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        PermissionError::Timeout => {} // Expected
        _ => panic!("Expected timeout error"),
    }
}
