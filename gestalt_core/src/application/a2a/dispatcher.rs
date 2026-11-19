use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskState {
    Submitted,
    Working,
    Completed,
    Failed,
    Canceled,
    Queued,
    Running,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Submitted => write!(f, "Submitted"),
            Self::Working => write!(f, "Working"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Canceled => write!(f, "Canceled"),
            Self::Queued => write!(f, "Queued"),
            Self::Running => write!(f, "Running"),
        }
    }
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone)]
pub enum A2aTaskError {
    #[error("Task {0} not found")]
    NotFound(String),

    #[error("Task {task_id} is already in terminal state {state}")]
    AlreadyTerminal { task_id: String, state: TaskState },

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub payload: String,
    pub state: TaskState,
    pub cancellation_token: CancellationToken,
}

impl Task {
    pub fn new(id: String, payload: String) -> Self {
        Self {
            id,
            payload,
            state: TaskState::Submitted,
            cancellation_token: CancellationToken::new(),
        }
    }

    pub fn new_with_token(
        id: String,
        payload: String,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            id,
            payload,
            state: TaskState::Submitted,
            cancellation_token,
        }
    }

    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }
}

pub struct Dispatcher {
    tasks: HashMap<String, Task>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn submit_task(&mut self, id: String, payload: String) {
        self.tasks.insert(id.clone(), Task::new(id, payload));
    }

    pub fn submit_task_with_token(
        &mut self,
        id: String,
        payload: String,
        cancellation_token: CancellationToken,
    ) {
        self.tasks.insert(
            id.clone(),
            Task::new_with_token(id, payload, cancellation_token),
        );
    }

    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }

    pub fn get_cancellation_token(&self, id: &str) -> Option<CancellationToken> {
        self.tasks
            .get(id)
            .map(|task| task.cancellation_token.clone())
    }

    pub fn update_task_state(&mut self, id: &str, new_state: TaskState) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.state = new_state;
        }
    }

    pub fn cancel_task(&mut self, id: &str) -> Result<(), A2aTaskError> {
        let task = self
            .tasks
            .get_mut(id)
            .ok_or_else(|| A2aTaskError::NotFound(id.to_string()))?;
        match task.state {
            TaskState::Canceled => Ok(()),
            TaskState::Completed => Err(A2aTaskError::AlreadyTerminal {
                task_id: id.to_string(),
                state: TaskState::Completed,
            }),
            TaskState::Failed => Err(A2aTaskError::AlreadyTerminal {
                task_id: id.to_string(),
                state: TaskState::Failed,
            }),
            TaskState::Submitted | TaskState::Working | TaskState::Queued | TaskState::Running => {
                task.state = TaskState::Canceled;
                task.cancellation_token.cancel();
                Ok(())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_and_get_task() {
        let mut dispatcher = Dispatcher::new();
        dispatcher.submit_task("task1".to_string(), "do work".to_string());

        let task = dispatcher.get_task("task1").unwrap();
        assert_eq!(task.id, "task1");
        assert_eq!(task.payload, "do work");
        assert_eq!(task.state, TaskState::Submitted);
        assert!(!task.cancellation_token.is_cancelled());
    }

    #[test]
    fn test_update_task_state() {
        let mut dispatcher = Dispatcher::new();
        dispatcher.submit_task("task1".to_string(), "do work".to_string());

        dispatcher.update_task_state("task1", TaskState::Working);
        assert_eq!(
            dispatcher.get_task("task1").unwrap().state,
            TaskState::Working
        );

        dispatcher.update_task_state("task1", TaskState::Completed);
        assert_eq!(
            dispatcher.get_task("task1").unwrap().state,
            TaskState::Completed
        );

        dispatcher.update_task_state("task1", TaskState::Failed);
        assert_eq!(
            dispatcher.get_task("task1").unwrap().state,
            TaskState::Failed
        );

        dispatcher.update_task_state("task1", TaskState::Canceled);
        assert_eq!(
            dispatcher.get_task("task1").unwrap().state,
            TaskState::Canceled
        );
    }

    #[test]
    fn test_cancellation_token_firing() {
        let mut dispatcher = Dispatcher::new();
        dispatcher.submit_task("task1".to_string(), "work".to_string());

        let token = dispatcher.get_cancellation_token("task1").unwrap();
        assert!(!token.is_cancelled());

        let result = dispatcher.cancel_task("task1");
        assert!(result.is_ok());
        assert!(token.is_cancelled());
        assert_eq!(
            dispatcher.get_task("task1").unwrap().state,
            TaskState::Canceled
        );

        // Idempotent cancellation
        let result2 = dispatcher.cancel_task("task1");
        assert!(result2.is_ok());
    }

    #[test]
    fn test_terminal_state_cancellation_rejection() {
        let mut dispatcher = Dispatcher::new();
        dispatcher.submit_task("task_completed".to_string(), "work".to_string());
        dispatcher.update_task_state("task_completed", TaskState::Completed);

        let res = dispatcher.cancel_task("task_completed");
        assert_eq!(
            res,
            Err(A2aTaskError::AlreadyTerminal {
                task_id: "task_completed".to_string(),
                state: TaskState::Completed,
            })
        );

        dispatcher.submit_task("task_failed".to_string(), "work".to_string());
        dispatcher.update_task_state("task_failed", TaskState::Failed);

        let res = dispatcher.cancel_task("task_failed");
        assert_eq!(
            res,
            Err(A2aTaskError::AlreadyTerminal {
                task_id: "task_failed".to_string(),
                state: TaskState::Failed,
            })
        );
    }
}
