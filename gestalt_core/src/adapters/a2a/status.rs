pub use crate::application::a2a::dispatcher::A2aTaskError;
use crate::application::a2a::dispatcher::{Dispatcher, TaskState};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_task_status(
    dispatcher: Arc<RwLock<Dispatcher>>,
    task_id: String,
) -> Result<TaskState, A2aTaskError> {
    let dispatcher = dispatcher.read().await;
    match dispatcher.get_task(&task_id) {
        Some(task) => Ok(task.state),
        None => Err(A2aTaskError::NotFound(task_id)),
    }
}
