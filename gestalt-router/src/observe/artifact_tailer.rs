use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use serde_json::Value;
use gestalt_ws::BusEvent;
use tracing::{debug, warn};

/// Specifies the format of the log file being tailed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Claude Code local session log format.
    ClaudeJsonl,
    /// Generic/Hermes JSON format.
    GenericJson,
}

/// A stateful tailer that preserves read offsets across invocations to prevent re-ingesting logs.
pub struct ArtifactTailer {
    offsets: HashMap<PathBuf, u64>,
}

impl Default for ArtifactTailer {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactTailer {
    pub fn new() -> Self {
        Self {
            offsets: HashMap::new(),
        }
    }

    /// Incrementally read a log file, parsing new lines into `BusEvent`s and updating the read offset.
    pub fn tail_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        format: LogFormat,
        agent_name: &str,
    ) -> Result<Vec<BusEvent>, std::io::Error> {
        let path_ref = path.as_ref();

        let mut file = match File::open(path_ref) {
            Ok(f) => f,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Ok(Vec::new());
                }
                return Err(e);
            }
        };

        let metadata = file.metadata()?;
        let current_size = metadata.len();

        let saved_offset = self.offsets.get(path_ref).copied().unwrap_or(0);

        // Handle file rotation/truncation
        let offset = if current_size < saved_offset {
            debug!("File truncated or rotated, resetting offset for {:?}", path_ref);
            0
        } else {
            saved_offset
        };

        if offset == current_size {
            return Ok(Vec::new()); // No new data
        }

        file.seek(SeekFrom::Start(offset))?;

        let mut reader = BufReader::new(file);
        let mut line = String::new();
        let mut events = Vec::new();
        let mut current_offset = offset;

        loop {
            line.clear();
            let bytes_read = match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => {
                    warn!("Error reading file {:?}: {}", path_ref, e);
                    break;
                }
            };

            // Avoid parsing if the line isn't cleanly ending with newline (could be mid-write)
            // But if it's EOF and we read something without newline, we might process it if it's valid JSON
            // For now, let's try parsing it. If it fails, it might be truncated.

            let trimmed = line.trim();
            if trimmed.is_empty() {
                current_offset += bytes_read as u64;
                continue;
            }

            match serde_json::from_str::<Value>(trimmed) {
                Ok(val) => {
                    let mut event = BusEvent::new(agent_name, "artifact_log", "Parsed log entry");
                    event.metadata = val;
                    // Try extracting more specific info based on format
                    if format == LogFormat::ClaudeJsonl {
                        if let Some(msg_type) = event.metadata.get("type").and_then(|t| t.as_str()) {
                            event.event_type = msg_type.to_string();
                            if let Some(msg) = event.metadata.get("message").and_then(|m| m.as_str()) {
                                event.summary = msg.to_string();
                            } else {
                                event.summary = format!("Claude event: {}", msg_type);
                            }
                        }
                    } else if format == LogFormat::GenericJson {
                        if let Some(msg) = event.metadata.get("message").and_then(|m| m.as_str()) {
                            event.summary = msg.to_string();
                        }
                    }

                    events.push(event);
                    current_offset += bytes_read as u64;
                }
                Err(e) => {
                    warn!("Failed to parse line from {:?}: {}", path_ref, e);
                    // It might be a truncated line, but we'll skip it and advance anyway to prevent infinite loops on bad lines
                    // A better approach for purely truncated lines (no newline) is to stop reading and not advance current_offset,
                    // but since `read_line` consumed it, we advance. If it didn't have a newline, we might want to rewind.
                    if !line.ends_with('\n') {
                        // Probably truncated write, we shouldn't advance offset
                        debug!("Skipping parsing of non-newline terminated line, likely truncated write");
                        break;
                    }
                    current_offset += bytes_read as u64;
                }
            }
        }

        self.offsets.insert(path_ref.to_path_buf(), current_offset);

        Ok(events)
    }
}
