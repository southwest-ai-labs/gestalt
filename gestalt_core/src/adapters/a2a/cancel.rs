use std::sync::Arc;
use tokio::sync::Mutex;
use crate::application::a2a::dispatcher::{Dispatcher, TaskState};

pub async fn cancel_task_handler(
    dispatcher: Arc<Mutex<Dispatcher>>,
    task_id: String,
) -> Result<(), String> {
    let mut dispatcher = dispatcher.lock().await;
    match dispatcher.get_task(&task_id) {
        Some(task) => {
            match task.state {
                TaskState::Canceled => Ok(()),
                TaskState::Completed => Err(format!("Cannot cancel task {}: already completed", task_id)),
                TaskState::Failed => Err(format!("Cannot cancel task {}: already failed", task_id)),
                _ => {
                    dispatcher.update_task_state(&task_id, TaskState::Canceled);
                    Ok(())
                }
            }
        }
        None => Err(format!("Task {} not found", task_id)),
    }
}
