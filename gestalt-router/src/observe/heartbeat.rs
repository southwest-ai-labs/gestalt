use crate::event_bus::{persist_event, BusEvent};
use crate::health::{check_all, PROVIDERS, HealthStatus};
use gestalt_state::StateDb;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval};
use tokio_util::sync::CancellationToken;

/// Heartbeat reporter for registered local agents.
pub struct AgentHeartbeatReporter {
    db: Arc<StateDb>,
    interval: Duration,
    budget: Duration,
    cancel_token: CancellationToken,
}

impl AgentHeartbeatReporter {
    /// Create a new reporter.
    pub fn new(db: Arc<StateDb>, interval: Duration, budget: Duration) -> Self {
        Self {
            db,
            interval,
            budget,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Run the heartbeat reporter in the background until the cancellation token is triggered.
    pub async fn run(&self) {
        let mut timer = interval(self.interval);
        
        loop {
            tokio::select! {
                _ = timer.tick() => {
                    self.report_heartbeat().await;
                }
                _ = self.cancel_token.cancelled() => {
                    break;
                }
            }
        }
    }

    /// Report heartbeat for all providers.
    async fn report_heartbeat(&self) {
        let checks = check_all(PROVIDERS, self.budget).await;
        
        for check in checks {
            if check.status == HealthStatus::Healthy {
                let mut metadata = serde_json::Map::new();
                metadata.insert("latency_ms".to_string(), serde_json::Value::Number(check.latency_ms.into()));
                if let Some(version) = check.version {
                    metadata.insert("version".to_string(), serde_json::Value::String(version));
                }
                
                let mut ev = BusEvent::new(
                    &check.provider,
                    "agent_heartbeat",
                    format!("Agent {} heartbeat", check.provider),
                );
                ev.metadata = serde_json::Value::Object(metadata);
                
                let _ = persist_event(&self.db, &ev);
            }
        }
    }
    
    /// Get the cancellation token to stop the reporter.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}
