use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use crate::bus::BusState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryStats {
    pub process_rss_bytes: u64,
    pub active_state_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThroughputStats {
    pub write_count: u64,
    pub write_latency_ms: u64,
    pub conflict_count: u64,
    pub retry_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdminStatusResponse {
    pub service: String,
    pub status: String,
    pub version: String,
    pub memory: MemoryStats,
    pub active_worktrees: usize,
    pub connected_ws_clients: usize,
    pub throughput: ThroughputStats,
}

pub async fn handle_admin_status(
    State(state): State<BusState>,
) -> impl IntoResponse {
    let metrics = state.db.metrics();
    let throughput = ThroughputStats {
        write_count: metrics.write_count.load(std::sync::atomic::Ordering::Relaxed),
        write_latency_ms: metrics.write_latency_ms.load(std::sync::atomic::Ordering::Relaxed),
        conflict_count: metrics.conflict_count.load(std::sync::atomic::Ordering::Relaxed),
        retry_count: metrics.retry_count.load(std::sync::atomic::Ordering::Relaxed),
    };

    let active_worktrees = crate::task_api::active_runs_count();
    let connected_ws_clients = 0;

    let process_rss_bytes = get_process_rss().unwrap_or(0);
    let memory = MemoryStats {
        process_rss_bytes,
        active_state_entries: 0,
    };

    Json(AdminStatusResponse {
        service: "gestalt".to_string(),
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        memory,
        active_worktrees,
        connected_ws_clients,
        throughput,
    })
}

fn get_process_rss() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = statm.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(pages) = parts[1].parse::<u64>() {
                    return Some(pages * 4096);
                }
            }
        }
    }
    None
}
