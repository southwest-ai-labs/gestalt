use gestalt_router::health::{check_all, HealthReport, PROVIDERS, DEFAULT_TIMEOUT, DEFAULT_TTL, HealthStatus};
use gestalt_state::schema::FileLock;
use gestalt_state::StateDb;
use gestalt_core::application::agent::xavier::XavierClient;
use serde::Serialize;
use anyhow::Result;

#[derive(Debug, Clone, Serialize)]
pub struct BusInfo {
    pub endpoint: String,
    pub event_count: i64,
    pub xavier_connected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub bus: BusInfo,
    pub providers: HealthReport,
    pub locks: Vec<FileLock>,
}

pub fn render_status_report(bus_info: BusInfo, providers: HealthReport, locks: Vec<FileLock>) -> String {
    let mut out = String::new();
    
    // Panel 1: Universal Bus Status
    out.push_str("📡 Universal Bus Status\n");
    out.push_str("=======================\n");
    out.push_str(&format!("Endpoint: {}\n", bus_info.endpoint));
    out.push_str(&format!("Events:   {}\n", bus_info.event_count));
    let x_status = if bus_info.xavier_connected { "Online" } else { "Offline" };
    out.push_str(&format!("Xavier:   {}\n\n", x_status));

    // Panel 2: Providers Matrix
    out.push_str("🤖 Providers Matrix\n");
    out.push_str("===================\n");
    for check in providers.providers {
        let status_emoji = match check.status {
            HealthStatus::Healthy => "✅",
            HealthStatus::Degraded => "⚠️",
            HealthStatus::Unhealthy => "❌",
        };
        out.push_str(&format!(
            "{} {:<10} | latency: {:>4}ms | {}\n",
            status_emoji,
            check.provider,
            check.latency_ms,
            check.message.unwrap_or_default()
        ));
    }
    out.push('\n');

    // Panel 3: Active Worktrees & File Locks
    out.push_str("🔒 Active File Locks\n");
    out.push_str("====================\n");
    if locks.is_empty() {
        out.push_str("No active locks.\n");
    } else {
        for lock in locks {
            out.push_str(&format!("- {} (held by agent {})\n", lock.path, lock.agent_id));
        }
    }
    
    out
}

pub async fn run_status(json_mode: bool) -> Result<()> {
    // 1. Gather Bus Info
    let db_path = home::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".gestalt")
        .join("state.db");
    
    let (event_count, locks) = match StateDb::open(&db_path) {
        Ok(db) => {
            let count = db.event_count().unwrap_or(0);
            let active_locks = db.get_locks().unwrap_or_default();
            (count, active_locks)
        },
        Err(_) => (0, vec![]),
    };

    let xavier_client = XavierClient::from_env();
    let xavier_connected = xavier_client.health().await.is_ok();

    let bus_info = BusInfo {
        endpoint: "http://127.0.0.1:8081".to_string(), // Default local bus port
        event_count,
        xavier_connected,
    };

    // 2. Gather Providers Health
    let checks = check_all(PROVIDERS, DEFAULT_TIMEOUT).await;
    let providers = HealthReport::from_checks(checks, false, DEFAULT_TTL);

    if json_mode {
        let report = StatusReport {
            bus: bus_info,
            providers,
            locks,
        };
        let json_str = serde_json::to_string_pretty(&report)?;
        println!("{}", json_str);
    } else {
        let text = render_status_report(bus_info, providers, locks);
        println!("{}", text);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_status_report() {
        let bus = BusInfo {
            endpoint: "http://127.0.0.1:8081".to_string(),
            event_count: 42,
            xavier_connected: true,
        };
        let providers = HealthReport::from_checks(vec![], false, DEFAULT_TTL);
        let locks = vec![];
        let rendered = render_status_report(bus, providers, locks);

        assert!(rendered.contains("Universal Bus Status"));
        assert!(rendered.contains("42"));
        assert!(rendered.contains("Online"));
        assert!(rendered.contains("Providers Matrix"));
        assert!(rendered.contains("No active locks"));
    }
}
