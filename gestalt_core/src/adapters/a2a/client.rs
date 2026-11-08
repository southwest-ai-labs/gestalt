use serde::{Deserialize, Serialize};
use thiserror::Error;
use reqwest::Client;
use super::{AgentCard, TaskPayload, AGENT_JSON_PATH};

#[derive(Error, Debug)]
pub enum A2aError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("API error: {0}")]
    ApiError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TaskResponse {
    pub status: String,
    pub message: Option<String>,
    pub result: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct A2aClient {
    client: Client,
}

impl Default for A2aClient {
    fn default() -> Self {
        Self::new()
    }
}

impl A2aClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn discover(&self, agent_base_url: &str) -> Result<AgentCard, A2aError> {
        let base = agent_base_url.trim_end_matches('/');
        let url = format!("{}{}", base, AGENT_JSON_PATH);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(A2aError::ApiError(format!("Failed to fetch agent card: {}", response.status())));
        }
        
        let card = response.json::<AgentCard>().await?;
        Ok(card)
    }

    pub async fn submit_task(&self, agent_url: &str, task_spec: TaskPayload) -> Result<TaskResponse, A2aError> {
        let response = self.client.post(agent_url)
            .json(&task_spec)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(A2aError::ApiError(format!("Task submission failed: {}", response.status())));
        }
        
        let task_response = response.json::<TaskResponse>().await?;
        Ok(task_response)
    }
}
