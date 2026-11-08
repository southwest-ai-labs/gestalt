use std::sync::Arc;
use tokio::sync::Mutex;
use crate::application::a2a::dispatcher::{Dispatcher, TaskState};

pub async fn get_task_status(
    dispatcher: Arc<Mutex<Dispatcher>>,
    task_id: String,
) -> Result<TaskState, String> {
    let dispatcher = dispatcher.lock().await;
    match dispatcher.get_task(&task_id) {
        Some(task) => Ok(task.state.clone()),
        None => Err(format!("Task {} not found", task_id)),
    }
}
