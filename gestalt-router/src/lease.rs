use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum LeaseState {
    Read(HashSet<String>),
    Write {
        agent_id: String,
        expires_at: Instant,
    },
}

#[derive(Debug, Clone)]
pub struct LeaseManager {
    leases: Arc<Mutex<HashMap<PathBuf, LeaseState>>>,
}

impl LeaseManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            leases: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn acquire_read(&self, path: &Path, agent_id: &str) -> bool {
        let mut leases = self.leases.lock().unwrap();
        let path_buf = path.to_path_buf();

        if let Some(state) = leases.get_mut(&path_buf) {
            match state {
                LeaseState::Read(agents) => {
                    agents.insert(agent_id.to_string());
                    true
                }
                LeaseState::Write { expires_at, .. } => {
                    if Instant::now() > *expires_at {
                        let mut agents = HashSet::new();
                        agents.insert(agent_id.to_string());
                        leases.insert(path_buf, LeaseState::Read(agents));
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            let mut agents = HashSet::new();
            agents.insert(agent_id.to_string());
            leases.insert(path_buf, LeaseState::Read(agents));
            true
        }
    }

    pub fn acquire_write(&self, path: &Path, agent_id: &str, ttl: Duration) -> bool {
        let mut leases = self.leases.lock().unwrap();
        let path_buf = path.to_path_buf();

        if let Some(state) = leases.get(&path_buf) {
            match state {
                LeaseState::Read(agents) => {
                    if agents.is_empty() {
                        leases.insert(
                            path_buf,
                            LeaseState::Write {
                                agent_id: agent_id.to_string(),
                                expires_at: Instant::now() + ttl,
                            },
                        );
                        true
                    } else {
                        false
                    }
                }
                LeaseState::Write {
                    expires_at,
                    agent_id: current_agent,
                } => {
                    if Instant::now() > *expires_at || current_agent == agent_id {
                        leases.insert(
                            path_buf,
                            LeaseState::Write {
                                agent_id: agent_id.to_string(),
                                expires_at: Instant::now() + ttl,
                            },
                        );
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            leases.insert(
                path_buf,
                LeaseState::Write {
                    agent_id: agent_id.to_string(),
                    expires_at: Instant::now() + ttl,
                },
            );
            true
        }
    }

    pub fn release(&self, path: &Path, agent_id: &str) {
        let mut leases = self.leases.lock().unwrap();
        let path_buf = path.to_path_buf();

        if let Some(state) = leases.get_mut(&path_buf) {
            match state {
                LeaseState::Read(agents) => {
                    agents.remove(agent_id);
                    if agents.is_empty() {
                        leases.remove(&path_buf);
                    }
                }
                LeaseState::Write {
                    agent_id: current_agent,
                    ..
                } => {
                    if current_agent == agent_id {
                        leases.remove(&path_buf);
                    }
                }
            }
        }
    }
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new()
    }
}
