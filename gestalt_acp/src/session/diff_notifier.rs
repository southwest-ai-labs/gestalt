use serde::{Deserialize, Serialize};
use similar::TextDiff;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileUpdateParams {
    pub path: String,
    pub old_content: String,
    pub new_content: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileUpdateNotification {
    pub method: String,
    pub params: FileUpdateParams,
}

pub struct DiffNotifier {
    tx: broadcast::Sender<FileUpdateNotification>,
}

impl DiffNotifier {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<FileUpdateNotification> {
        self.tx.subscribe()
    }

    pub fn generate_diff_notification(path: &str, before: &str, after: &str) -> FileUpdateNotification {
        let diff = TextDiff::from_lines(before, after)
            .unified_diff()
            .header("a", "b")
            .to_string();

        FileUpdateNotification {
            method: "session/fileUpdate".to_string(),
            params: FileUpdateParams {
                path: path.to_string(),
                old_content: before.to_string(),
                new_content: after.to_string(),
                diff,
            },
        }
    }

    pub fn broadcast_file_update(&self, path: &str, before: &str, after: &str) -> Result<usize, broadcast::error::SendError<FileUpdateNotification>> {
        let notification = Self::generate_diff_notification(path, before, after);
        self.tx.send(notification)
    }
}
