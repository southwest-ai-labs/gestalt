use gestalt_acp::session::manager::{AcpSessionManager, SessionParams, SessionState, SessionError};

#[tokio::test]
async fn test_session_creation() {
    let manager = AcpSessionManager::new();
    let id = manager.create_session(SessionParams {
        workspace_root: "/path/to/workspace".to_string(),
    });
    
    let session = manager.get_session(&id).unwrap();
    assert_eq!(session.workspace_root, "/path/to/workspace");
    assert_eq!(session.state, SessionState::Idle);
    assert_eq!(session.history.len(), 0);
}

#[tokio::test]
async fn test_session_retrieval_and_history() {
    let manager = AcpSessionManager::new();
    let id = manager.create_session(SessionParams {
        workspace_root: "/path".to_string(),
    });

    {
        let mut session = manager.get_session_mut(&id).unwrap();
        session.history.push(("prompt1".to_string(), "response1".to_string()));
    }

    let session = manager.get_session(&id).unwrap();
    assert_eq!(session.history.len(), 1);
    assert_eq!(session.history[0], ("prompt1".to_string(), "response1".to_string()));
}

#[tokio::test]
async fn test_session_cancellation() {
    let manager = AcpSessionManager::new();
    let id = manager.create_session(SessionParams {
        workspace_root: "/path".to_string(),
    });

    manager.cancel_session(&id).unwrap();

    let session = manager.get_session(&id).unwrap();
    assert_eq!(session.state, SessionState::Cancelled);
    assert!(session.cancel_token.is_cancelled());
}

#[tokio::test]
async fn test_cancel_non_existent_session() {
    let manager = AcpSessionManager::new();
    let result = manager.cancel_session("non-existent-id");
    
    assert!(matches!(result, Err(SessionError::NotFound(_))));
}
