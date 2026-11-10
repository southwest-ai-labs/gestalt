use dashmap::DashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Running,
    Cancelled,
}

#[derive(Debug)]
pub struct SessionParams {
    pub workspace_root: String,
}

#[derive(Debug)]
pub struct AcpSession {
    pub id: String,
    pub workspace_root: String,
    pub state: SessionState,
    pub created_at: std::time::SystemTime,
    pub history: Vec<(String, String)>,
    pub cancel_token: CancellationToken,
}

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
}

#[derive(Clone)]
pub struct AcpSessionManager {
    sessions: Arc<DashMap<String, AcpSession>>,
}

impl AcpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    pub fn create_session(&self, params: SessionParams) -> String {
        let id = Uuid::new_v4().to_string();
        let session = AcpSession {
            id: id.clone(),
            workspace_root: params.workspace_root,
            state: SessionState::Idle,
            created_at: std::time::SystemTime::now(),
            history: Vec::new(),
            cancel_token: CancellationToken::new(),
        };
        self.sessions.insert(id.clone(), session);
        id
    }

    pub fn get_session(&self, id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, AcpSession>> {
        self.sessions.get(id)
    }

    pub fn get_session_mut(&self, id: &str) -> Option<dashmap::mapref::one::RefMut<'_, String, AcpSession>> {
        self.sessions.get_mut(id)
    }

    pub fn cancel_session(&self, id: &str) -> Result<(), SessionError> {
        if let Some(mut session) = self.sessions.get_mut(id) {
            session.cancel_token.cancel();
            session.state = SessionState::Cancelled;
            Ok(())
        } else {
            Err(SessionError::NotFound(id.to_string()))
        }
    }
}

impl Default for AcpSessionManager {
    fn default() -> Self {
        Self::new()
    }
}
