use std::sync::Arc;
use tokio::sync::RwLock;

use gestalt_core::adapters::a2a::{self, A2aTaskError};
use gestalt_core::application::a2a::dispatcher::{Dispatcher, TaskState};

#[tokio::test]
async fn test_get_task_status_success() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    let status = a2a::status::get_task_status(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(status, Ok(TaskState::Submitted));
}

#[tokio::test]
async fn test_get_task_status_not_found() {
    let dispatcher = Dispatcher::new();
    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    let status = a2a::status::get_task_status(arc_dispatcher, "nonexistent".to_string()).await;
    assert_eq!(
        status,
        Err(A2aTaskError::NotFound("nonexistent".to_string()))
    );
}

#[tokio::test]
async fn test_get_task_status_concurrent_readers() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    let mut handles = Vec::new();
    for _ in 0..10 {
        let d = arc_dispatcher.clone();
        handles.push(tokio::spawn(async move {
            a2a::status::get_task_status(d, "task1".to_string()).await
        }));
    }

    for handle in handles {
        let res = handle.await.unwrap();
        assert_eq!(res, Ok(TaskState::Submitted));
    }
}

#[tokio::test]
async fn test_cancel_task_success() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    let result =
        a2a::cancel::cancel_task_handler(arc_dispatcher.clone(), "task1".to_string()).await;
    assert!(result.is_ok());

    let status = a2a::status::get_task_status(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(status, Ok(TaskState::Canceled));
}

#[tokio::test]
async fn test_cancel_task_idempotent() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    // First cancel
    assert!(
        a2a::cancel::cancel_task_handler(arc_dispatcher.clone(), "task1".to_string())
            .await
            .is_ok()
    );
    // Second cancel should be idempotent (returns Ok(()))
    assert!(
        a2a::cancel::cancel_task_handler(arc_dispatcher.clone(), "task1".to_string())
            .await
            .is_ok()
    );

    let status = a2a::status::get_task_status(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(status, Ok(TaskState::Canceled));
}

#[tokio::test]
async fn test_cancel_task_completed_rejects_with_already_terminal() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    dispatcher.update_task_state("task1", TaskState::Completed);
    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    let result = a2a::cancel::cancel_task_handler(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(
        result,
        Err(A2aTaskError::AlreadyTerminal {
            task_id: "task1".to_string(),
            state: TaskState::Completed,
        })
    );
}

#[tokio::test]
async fn test_cancel_task_failed_rejects_with_already_terminal() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    dispatcher.update_task_state("task1", TaskState::Failed);
    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    let result = a2a::cancel::cancel_task_handler(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(
        result,
        Err(A2aTaskError::AlreadyTerminal {
            task_id: "task1".to_string(),
            state: TaskState::Failed,
        })
    );
}

#[tokio::test]
async fn test_cancel_task_not_found() {
    let dispatcher = Dispatcher::new();
    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    let result = a2a::cancel::cancel_task_handler(arc_dispatcher, "nonexistent".to_string()).await;
    assert_eq!(
        result,
        Err(A2aTaskError::NotFound("nonexistent".to_string()))
    );
}

#[tokio::test]
async fn test_in_flight_task_cancellation_fires_cancellation_token() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.submit_task("task1".to_string(), "payload".to_string());
    dispatcher.update_task_state("task1", TaskState::Working);

    let token = dispatcher
        .get_cancellation_token("task1")
        .expect("token must exist for submitted task");
    assert!(!token.is_cancelled());

    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));

    let result =
        a2a::cancel::cancel_task_handler(arc_dispatcher.clone(), "task1".to_string()).await;
    assert!(result.is_ok());

    // CancellationToken must now be fired
    assert!(token.is_cancelled());

    let status = a2a::status::get_task_status(arc_dispatcher, "task1".to_string()).await;
    assert_eq!(status, Ok(TaskState::Canceled));
}

#[tokio::test]
async fn test_in_flight_worker_select_receives_cancellation() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.submit_task("worker_task".to_string(), "job".to_string());
    dispatcher.update_task_state("worker_task", TaskState::Running);

    let token = dispatcher
        .get_cancellation_token("worker_task")
        .expect("token exists");
    let token_clone = token.clone();

    let worker_handle = tokio::spawn(async move {
        tokio::select! {
            _ = token_clone.cancelled() => "received_cancellation",
            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => "timeout",
        }
    });

    let arc_dispatcher = Arc::new(RwLock::new(dispatcher));
    let cancel_res =
        a2a::cancel::cancel_task_handler(arc_dispatcher, "worker_task".to_string()).await;
    assert!(cancel_res.is_ok());

    let outcome = worker_handle.await.expect("worker task completed");
    assert_eq!(outcome, "received_cancellation");
}

#[tokio::test]
async fn test_exhaustive_non_terminal_states_cancelable() {
    let non_terminal_states = [
        TaskState::Submitted,
        TaskState::Working,
        TaskState::Queued,
        TaskState::Running,
    ];

    for state in non_terminal_states {
        let id = format!("task_{:?}", state);
        let mut dispatcher = Dispatcher::new();
        dispatcher.submit_task(id.clone(), "payload".to_string());
        dispatcher.update_task_state(&id, state);

        let token = dispatcher.get_cancellation_token(&id).unwrap();
        assert!(!token.is_cancelled());

        let arc_dispatcher = Arc::new(RwLock::new(dispatcher));
        let res = a2a::cancel::cancel_task_handler(arc_dispatcher.clone(), id.clone()).await;
        assert!(res.is_ok());
        assert!(token.is_cancelled());

        let status = a2a::status::get_task_status(arc_dispatcher, id).await;
        assert_eq!(status, Ok(TaskState::Canceled));
    }
}
