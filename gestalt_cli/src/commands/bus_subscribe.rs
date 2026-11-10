use colored::*;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::signal;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BusEvent {
    pub agent: String,
    pub event_type: String,
    pub summary: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub ts: Option<String>,
}

pub fn matches_filter(
    event: &BusEvent,
    filter_agent: Option<&str>,
    filter_type: Option<&str>,
) -> bool {
    if let Some(agent) = filter_agent {
        if event.agent != agent {
            return false;
        }
    }
    if let Some(ev_type) = filter_type {
        if event.event_type != ev_type {
            return false;
        }
    }
    true
}

pub fn format_event(event: &BusEvent) -> String {
    let time = event.ts.as_deref().unwrap_or("now");
    let agent_str = format!("[{}]", event.agent).blue();
    let type_str = format!("[{}]", event.event_type).green();
    let time_str = format!("[{}]", time).truecolor(100, 100, 100);
    format!("{} {} {} {}", time_str, agent_str, type_str, event.summary)
}

#[allow(dead_code)]
pub async fn run_subscribe(
    ws_url: &str,
    filter_agent: Option<String>,
    filter_type: Option<String>,
) -> anyhow::Result<()> {
    println!("🔌 Connecting to WebSocket at {}", ws_url);

    let (ws_stream, _) = connect_async(ws_url).await?;
    println!("✅ Connected! Waiting for events...");

    let (_, mut read) = ws_stream.split();

    let filter_agent_ref = filter_agent.as_deref();
    let filter_type_ref = filter_type.as_deref();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(event) = serde_json::from_str::<BusEvent>(&text) {
                            if matches_filter(&event, filter_agent_ref, filter_type_ref) {
                                println!("{}", format_event(&event));
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        println!("🔌 Connection closed by server.");
                        break;
                    }
                    Some(Err(e)) => {
                        eprintln!("❌ WebSocket error: {}", e);
                        break;
                    }
                    _ => {} // Ignore other message types
                }
            }
            _ = signal::ctrl_c() => {
                println!("\n👋 Exiting...");
                break;
            }
        }
    }

    Ok(())
}
