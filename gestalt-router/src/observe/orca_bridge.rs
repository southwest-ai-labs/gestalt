use crate::event_bus::{persist_event, BusEvent};
use gestalt_state::StateDb;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcaHookPayload {
    pub hook_type: String,
    pub agent: String,
    pub run_id: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub message: Option<String>,
}

pub fn parse_orca_event(raw: &str) -> Result<BusEvent, String> {
    let payload: OrcaHookPayload = serde_json::from_str(raw).map_err(|e| e.to_string())?;

    let event_type = match payload.hook_type.as_str() {
        "agent_started" => "run_started",
        "agent_finished" => "run_finished",
        "tool_call" => "tool_call",
        "error" => "error",
        _ => return Err(format!("Unknown hook type: {}", payload.hook_type)),
    };

    let message = payload
        .message
        .unwrap_or_else(|| format!("Orca event: {}", event_type));

    let mut event = BusEvent::new(&payload.agent, event_type, message).with_run_id(&payload.run_id);
    event.metadata = payload.metadata;

    Ok(event)
}

pub struct OrcaBridgeReceiver {
    db: Arc<StateDb>,
    bind_addr: String,
    cancel_token: CancellationToken,
}

impl OrcaBridgeReceiver {
    pub fn new(db: Arc<StateDb>, bind_addr: String) -> Self {
        Self {
            db,
            bind_addr,
            cancel_token: CancellationToken::new(),
        }
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub async fn listen_and_forward(&self) -> Result<(), String> {
        let listener = TcpListener::bind(&self.bind_addr)
            .await
            .map_err(|e| e.to_string())?;

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    break;
                }
                accept_res = listener.accept() => {
                    if let Ok((mut stream, _addr)) = accept_res {
                        let db_clone = Arc::clone(&self.db);

                        tokio::spawn(async move {
                            // Enforce fail-open 3s timeout
                            let handle_res = timeout(Duration::from_secs(3), async move {
                                let mut buf_reader = BufReader::new(&mut stream);
                                let mut content_length = 0;
                                let mut line = String::new();
                                
                                loop {
                                    line.clear();
                                    if buf_reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                                        break; // EOF
                                    }
                                    
                                    // Empty line indicates end of headers
                                    if line == "\r\n" || line == "\n" {
                                        break;
                                    }
                                    
                                    let lower_line = line.to_lowercase();
                                    if lower_line.starts_with("content-length:") {
                                        if let Some(val_str) = lower_line.strip_prefix("content-length:") {
                                            content_length = val_str.trim().parse().unwrap_or(0);
                                        }
                                    }
                                }

                                if content_length > 0 {
                                    let mut body_buf = vec![0; content_length];
                                    if buf_reader.read_exact(&mut body_buf).await.is_ok() {
                                        if let Ok(raw_str) = String::from_utf8(body_buf) {
                                            let json_body = if let Some(idx) = raw_str.find('{') {
                                                &raw_str[idx..]
                                            } else {
                                                &raw_str
                                            };
                                            
                                            match parse_orca_event(json_body) {
                                                Ok(bus_event) => {
                                                    if let Err(e) = persist_event(&db_clone, &bus_event) {
                                                        warn!("OrcaBridge failed to persist event: {}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    debug!("OrcaBridge could not parse payload: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Send 200 OK so standard HTTP clients don't hang
                                    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;
                                } else {
                                    // Handle non-HTTP or raw JSON socket
                                    let mut raw_buf = Vec::new();
                                    if buf_reader.read_to_end(&mut raw_buf).await.is_ok() {
                                        if let Ok(raw_str) = String::from_utf8(raw_buf) {
                                            let json_body = if let Some(idx) = raw_str.find('{') {
                                                &raw_str[idx..]
                                            } else {
                                                &raw_str
                                            };
                                            
                                            if let Ok(bus_event) = parse_orca_event(json_body) {
                                                let _ = persist_event(&db_clone, &bus_event);
                                            }
                                        }
                                    }
                                }
                            })
                            .await;

                            if handle_res.is_err() {
                                warn!("OrcaBridge connection timed out (fail-open)");
                            }
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
