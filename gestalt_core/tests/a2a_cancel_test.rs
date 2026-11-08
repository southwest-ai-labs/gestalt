use std::sync::Arc;
use tokio::sync::Mutex;
pub use gestalt_core::application;

#[path = "../src/adapters/a2a/mod.rs"]
mod a2a;

#[tokio::test]
async fn test_get_task_status_success() {
    let mut dispatcher = application::a2a::dispatcher::Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    let arc_dispatcher = Arc::new(Mutex::new(dispatcher));
    
    let status = a2a::status::get_task_status(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(status, Ok(application::a2a::dispatcher::TaskState::Submitted));
}

#[tokio::test]
async fn test_get_task_status_not_found() {
    let dispatcher = application::a2a::dispatcher::Dispatcher::new();
    let arc_dispatcher = Arc::new(Mutex::new(dispatcher));
    
    let status = a2a::status::get_task_status(arc_dispatcher, "nonexistent".to_string()).await;
    assert!(status.is_err());
    assert_eq!(status.unwrap_err(), "Task nonexistent not found");
}

#[tokio::test]
async fn test_cancel_task_success() {
    let mut dispatcher = application::a2a::dispatcher::Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    let arc_dispatcher = Arc::new(Mutex::new(dispatcher));
    
    let result = a2a::cancel::cancel_task_handler(arc_dispatcher.clone(), "task1".to_string()).await;
    assert!(result.is_ok());
    
    let status = a2a::status::get_task_status(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(status, Ok(application::a2a::dispatcher::TaskState::Canceled));
}

#[tokio::test]
async fn test_cancel_task_idempotent() {
    let mut dispatcher = application::a2a::dispatcher::Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    let arc_dispatcher = Arc::new(Mutex::new(dispatcher));
    
    // First cancel
    assert!(a2a::cancel::cancel_task_handler(arc_dispatcher.clone(), "task1".to_string()).await.is_ok());
    // Second cancel should be idempotent
    assert!(a2a::cancel::cancel_task_handler(arc_dispatcher.clone(), "task1".to_string()).await.is_ok());
    
    let status = a2a::status::get_task_status(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(status, Ok(application::a2a::dispatcher::TaskState::Canceled));
}

#[tokio::test]
async fn test_cancel_task_completed() {
    let mut dispatcher = application::a2a::dispatcher::Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    dispatcher.update_task_state("task1", application::a2a::dispatcher::TaskState::Completed);
    let arc_dispatcher = Arc::new(Mutex::new(dispatcher));
    
    let result = a2a::cancel::cancel_task_handler(arc_dispatcher, "task1".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Cannot cancel task task1: already completed");
}

#[tokio::test]
async fn test_cancel_task_failed() {
    let mut dispatcher = application::a2a::dispatcher::Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    dispatcher.update_task_state("task1", application::a2a::dispatcher::TaskState::Failed);
    let arc_dispatcher = Arc::new(Mutex::new(dispatcher));
    
    let result = a2a::cancel::cancel_task_handler(arc_dispatcher, "task1".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Cannot cancel task task1: already failed");
}

#[tokio::test]
async fn test_cancel_task_not_found() {
    let dispatcher = application::a2a::dispatcher::Dispatcher::new();
    let arc_dispatcher = Arc::new(Mutex::new(dispatcher));
    
    let result = a2a::cancel::cancel_task_handler(arc_dispatcher, "nonexistent".to_string()).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Task nonexistent not found");
}
