use std::sync::Arc;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;
use tracing::{info, error};

use gestalt_state::StateDb;
use gestalt_router::xavier_sink::XavierEventSink;

/// The cursor key used in the StateDb metadata table to store the last synced sequence number.
const REPLAY_CURSOR_KEY: &str = "xavier_replay_cursor";

/// Daemon that continuously monitors Xavier's availability and synchronizes missed events.
pub struct ReplaySyncDaemon {
    #[allow(dead_code)]
    db: Arc<StateDb>,
    #[allow(dead_code)]
    sink: Arc<XavierEventSink>,
}

impl ReplaySyncDaemon {
    pub fn new(db: Arc<StateDb>, sink: Arc<XavierEventSink>) -> Self {
        Self { db, sink }
    }

    /// Run the daemon loop until the cancellation token is triggered.
    #[allow(dead_code)]
    pub async fn run(self, cancel: CancellationToken) {
        let mut ticker = interval(Duration::from_secs(10));
        let mut was_online = false;

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("ReplaySyncDaemon shutting down");
                    break;
                }
                _ = ticker.tick() => {
                    let is_online = self.sink.is_available().await;

                    if is_online && !was_online {
                        info!("Xavier transitioned to online; starting replay sync");
                        if let Err(e) = self.sync_all().await {
                            error!("Failed during replay sync: {}", e);
                        } else {
                            info!("Replay sync completed successfully");
                        }
                    }

                    was_online = is_online;
                }
            }
        }
    }

    /// Syncs all unsynced events by fetching batches.
    #[allow(dead_code)]
    async fn sync_all(&self) -> Result<(), String> {
        loop {
            let synced = self.sync_batch(&self.db, &self.sink, 100).await?;
            if synced == 0 {
                break;
            }
        }
        Ok(())
    }

    /// Syncs a single batch of events and updates the cursor.
    pub async fn sync_batch(&self, db: &StateDb, sink: &XavierEventSink, limit: usize) -> Result<usize, String> {
        // Read cursor
        let cursor_str = db.get_metadata(REPLAY_CURSOR_KEY)
            .map_err(|e| format!("Failed to read cursor: {}", e))?;
        
        let cursor: Option<i64> = cursor_str.and_then(|s| s.parse().ok());

        // Fetch un-synced events
        let events = db.query_timeline(None, None, None, cursor, limit as i64)
            .map_err(|e| format!("Failed to query timeline: {}", e))?;

        if events.is_empty() {
            return Ok(0);
        }

        let mut max_seq = cursor.unwrap_or(0);
        let mut count = 0;
        let event_count = events.len();

        // Send to sink
        for ev in events {
            let bus_ev = gestalt_router::event_bus::BusEvent {
                run_id: Some(ev.run_id),
                agent: ev.agent_id.unwrap_or_else(|| "unknown".to_string()),
                event_type: ev.event_type,
                summary: "".to_string(), // Missing from TimelineEvent, maybe reconstruct or pass blank?
                project: ev.project,
                state: Some("historical".to_string()),
                ts: ev.created_at.to_rfc3339(),
                metadata: serde_json::from_str(&ev.payload).unwrap_or(serde_json::json!({})),
            };

            // In test environment, the real sink might fail. We want to stop processing on failure.
            if let Err(e) = sink.sink(&bus_ev).await {
                // If we failed, break the loop so we only save up to the last successful event sequence.
                error!("Failed to sink event: {:?}", e);
                break;
            }

            if let Some(seq) = ev.seq {
                if seq > max_seq {
                    max_seq = seq;
                }
            }
            count += 1;
        }

        // Update cursor
        if count > 0 {
            db.set_metadata(REPLAY_CURSOR_KEY, &max_seq.to_string())
                .map_err(|e| format!("Failed to set cursor: {}", e))?;
        }

        // If count is less than the requested limit and events wasn't empty, it means we hit an error midway.
        // It's technically okay, next loop iteration will retry the rest, but we could return an error to trigger backoff.
        // Returning Ok(count) lets sync_all retry the remaining ones, which might loop if the network remains down.
        // So return Err if count == 0 and we failed on the first event.
        if count == 0 && event_count > 0 {
            return Err("Failed to sync any events in this batch".to_string());
        }

        Ok(count)
    }
}
