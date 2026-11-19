pub use crate::application::a2a::dispatcher::A2aTaskError;
use crate::application::a2a::dispatcher::{Dispatcher, TaskState};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn cancel_task_handler(
    dispatcher: Arc<RwLock<Dispatcher>>,
    task_id: String,
) -> Result<(), A2aTaskError> {
    let mut dispatcher = dispatcher.write().await;
    match dispatcher.get_task_mut(&task_id) {
        Some(task) => match task.state {
            TaskState::Canceled => Ok(()),
            TaskState::Completed => Err(A2aTaskError::AlreadyTerminal {
                task_id,
                state: TaskState::Completed,
            }),
            TaskState::Failed => Err(A2aTaskError::AlreadyTerminal {
                task_id,
                state: TaskState::Failed,
            }),
            TaskState::Submitted | TaskState::Working | TaskState::Queued | TaskState::Running => {
                task.state = TaskState::Canceled;
                task.cancellation_token.cancel();
                Ok(())
            },
        },
        None => Err(A2aTaskError::NotFound(task_id)),
    }
}
