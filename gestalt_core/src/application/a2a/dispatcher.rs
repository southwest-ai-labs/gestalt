#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Submitted,
    Working,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub payload: String,
    pub state: TaskState,
}

pub struct Dispatcher {
    tasks: std::collections::HashMap<String, Task>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            tasks: std::collections::HashMap::new(),
        }
    }

    pub fn submit_task(&mut self, id: String, payload: String) {
        self.tasks.insert(
            id.clone(),
            Task {
                id,
                payload,
                state: TaskState::Submitted,
            },
        );
    }

    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    pub fn update_task_state(&mut self, id: &str, new_state: TaskState) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.state = new_state;
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
    }

    #[test]
    fn test_update_task_state() {
        let mut dispatcher = Dispatcher::new();
        dispatcher.submit_task("task1".to_string(), "do work".to_string());
        
        dispatcher.update_task_state("task1", TaskState::Working);
        assert_eq!(dispatcher.get_task("task1").unwrap().state, TaskState::Working);

        dispatcher.update_task_state("task1", TaskState::Completed);
        assert_eq!(dispatcher.get_task("task1").unwrap().state, TaskState::Completed);

        dispatcher.update_task_state("task1", TaskState::Failed);
        assert_eq!(dispatcher.get_task("task1").unwrap().state, TaskState::Failed);

        dispatcher.update_task_state("task1", TaskState::Canceled);
        assert_eq!(dispatcher.get_task("task1").unwrap().state, TaskState::Canceled);
    }
}
