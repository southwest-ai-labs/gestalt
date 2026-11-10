use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::time::timeout;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionScope {
    FileWrite(PathBuf),
    ShellExecution(String),
    NetworkAccess(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionStatus {
    Pending,
    Granted,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub scope: PermissionScope,
    pub description: String,
    pub status: PermissionStatus,
}

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Permission request timed out")]
    Timeout,
    #[error("Permission request channel closed")]
    ChannelClosed,
}

#[derive(Clone)]
pub struct PermissionManager {
    pending_requests: Arc<DashMap<String, oneshot::Sender<bool>>>,
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(DashMap::new()),
        }
    }

    pub fn request_permission(
        &self,
        scope: PermissionScope,
        description: String,
    ) -> (PermissionRequest, oneshot::Receiver<bool>) {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        
        self.pending_requests.insert(id.clone(), tx);
        
        let request = PermissionRequest {
            id,
            scope,
            description,
            status: PermissionStatus::Pending,
        };
        
        (request, rx)
    }

    pub async fn wait_for_permission(
        &self,
        id: &str,
        rx: oneshot::Receiver<bool>,
        timeout_duration: Duration,
    ) -> Result<bool, PermissionError> {
        match timeout(timeout_duration, rx).await {
            Ok(Ok(granted)) => Ok(granted),
            Ok(Err(_)) => {
                self.pending_requests.remove(id);
                Err(PermissionError::ChannelClosed)
            },
            Err(_) => {
                self.pending_requests.remove(id);
                Err(PermissionError::Timeout)
            },
        }
    }

    pub fn grant_permission(&self, id: &str) -> bool {
        if let Some((_, tx)) = self.pending_requests.remove(id) {
            tx.send(true).is_ok()
        } else {
            false
        }
    }

    pub fn deny_permission(&self, id: &str) -> bool {
        if let Some((_, tx)) = self.pending_requests.remove(id) {
            tx.send(false).is_ok()
        } else {
            false
        }
    }
}
