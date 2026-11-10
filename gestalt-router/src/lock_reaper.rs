use crate::event_bus::{persist_event, BusEvent};
use gestalt_state::StateDb;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Background task to clean up expired or orphaned file locks.
pub struct LockReaper {
    db: Arc<StateDb>,
    ttl: Duration,
    cancel_token: CancellationToken,
}

impl LockReaper {
    /// Create a new LockReaper.
    pub fn new(db: Arc<StateDb>, ttl: Duration) -> Self {
        Self {
            db,
            ttl,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Retrieve the cancellation token to allow graceful shutdown.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Run the reaper in the background until the cancellation token is triggered.
    pub async fn run(&self) {
        // Run reaper every TTL / 2
        let tick_duration = Duration::from_secs(std::cmp::max(1, self.ttl.as_secs() / 2));
        let mut timer = interval(tick_duration);

        loop {
            tokio::select! {
                _ = timer.tick() => {
                    match Self::reap_expired_locks(&self.db, self.ttl) {
                        Ok(count) if count > 0 => {
                            info!("Reaped {} expired/orphaned file locks", count);
                        }
                        Err(e) => {
                            warn!("Failed to reap locks: {}", e);
                        }
                        _ => {} // count == 0
                    }
                }
                _ = self.cancel_token.cancelled() => {
                    break;
                }
            }
        }
    }

    /// Synchronously reap expired locks or locks held by dead agents.
    pub fn reap_expired_locks(db: &StateDb, ttl: Duration) -> Result<usize, String> {
        let locks = db.get_locks().map_err(|e| format!("Failed to get locks: {}", e))?;
        let now = chrono::Utc::now();
        let mut reaped_count = 0;

        for lock in locks {
            let is_expired = now.signed_duration_since(lock.acquired_at).num_seconds() > ttl.as_secs() as i64;
            let mut is_dead = false;

            if !is_expired {
                // Check if the agent is still running
                match db.get_agent(&lock.run_id, &lock.agent_id) {
                    Ok(Some(agent)) => {
                        let state = agent.state.to_lowercase();
                        if state != "running" && state != "pending" {
                            is_dead = true;
                        }
                    }
                    Ok(None) => {
                        // Agent record is missing, consider dead
                        is_dead = true;
                    }
                    Err(e) => {
                        warn!("Failed to get agent {} for lock on {}: {}", lock.agent_id, lock.path, e);
                    }
                }
            }

            if is_expired || is_dead {
                if let Err(e) = db.release_lock(&lock.path, &lock.agent_id) {
                    warn!("Failed to release lock on {}: {}", lock.path, e);
                    continue;
                }
                reaped_count += 1;

                let reason = if is_expired { "expired" } else { "dead_agent" };
                
                let ev = BusEvent::new(
                    "router",
                    "lock_evicted",
                    format!("Evicted lock on {} (reason: {})", lock.path, reason),
                )
                .with_run_id(&lock.run_id)
                .with_metadata(serde_json::json!({
                    "path": lock.path,
                    "agent_id": lock.agent_id,
                    "acquired_at": lock.acquired_at.to_rfc3339(),
                    "reason": reason,
                }));

                if let Err(e) = persist_event(db, &ev) {
                    warn!("Failed to persist lock_evicted event: {}", e);
                }
            }
        }

        Ok(reaped_count)
    }
}
