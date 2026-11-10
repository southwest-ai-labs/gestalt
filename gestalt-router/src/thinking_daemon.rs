use crate::thinking::ThinkingLoop;
use chrono::{DateTime, Utc};
use gestalt_state::StateDb;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Configuration for the ThinkingDaemon.
#[derive(Debug, Clone)]
pub struct ThinkingDaemonConfig {
    pub interval: Duration,
    pub min_count: usize,
    pub cancel_token: CancellationToken,
}

impl Default for ThinkingDaemonConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300), // 5 minutes
            min_count: 3,
            cancel_token: CancellationToken::new(),
        }
    }
}

/// A background daemon that evaluates the execution gate for the Thinking Loop.
pub struct ThinkingDaemon {
    config: ThinkingDaemonConfig,
}

impl ThinkingDaemon {
    pub fn new(config: ThinkingDaemonConfig) -> Self {
        Self { config }
    }

    /// Run the daemon loop until cancelled.
    pub async fn run(&self, db: Arc<StateDb>, thinking_loop: Arc<ThinkingLoop>) {
        info!("Starting ThinkingDaemon with interval {:?}", self.config.interval);
        let mut ticker = tokio::time::interval(self.config.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let last_check = thinking_loop
                        .last_insight_time()
                        .await
                        .unwrap_or(DateTime::<Utc>::MIN_UTC);

                    if should_trigger(&db, last_check, self.config.min_count) {
                        info!("Thinking daemon triggered synthesis (>= {} new executions)", self.config.min_count);
                        if let Err(e) = thinking_loop.run(&db, false).await {
                            warn!("Thinking loop execution failed: {}", e);
                        }
                    } else {
                        info!("Thinking daemon skipping synthesis (< {} new executions)", self.config.min_count);
                    }
                }
                _ = self.config.cancel_token.cancelled() => {
                    info!("ThinkingDaemon cancelled, shutting down");
                    break;
                }
            }
        }
    }
}

/// Check if the execution gate conditions are met.
pub fn should_trigger(db: &StateDb, last_check: DateTime<Utc>, min_count: usize) -> bool {
    let events = match db.recent_timeline(1000) {
        Ok(evs) => evs,
        Err(e) => {
            warn!("Failed to query recent timeline: {}", e);
            return false;
        }
    };

    let count = events
        .iter()
        .filter(|e| {
            let is_execution = e.event_type == "run_finished" || e.event_type == "run_started";
            let dt = e.created_at;
            let is_newer = dt > last_check;
            is_execution && is_newer
        })
        .count();

    count >= min_count
}
