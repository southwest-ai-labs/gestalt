use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime};

pub struct WorktreeGc {
    pub base_dir: PathBuf,
    pub max_age: Duration,
}

impl WorktreeGc {
    pub async fn clean_stale(&self) -> Result<usize, io::Error> {
        if !self.base_dir.exists() || !self.base_dir.is_dir() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;
        let now = SystemTime::now();

        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_dir() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > self.max_age {
                            let path = entry.path();
                            
                            tokio::task::spawn_blocking(move || {
                                // Try git worktree remove
                                let _ = Command::new("git")
                                    .args(["worktree", "remove", "--force"])
                                    .arg(&path)
                                    .output();
                                
                                // Ensure cleanup if git command failed or wasn't a worktree
                                if path.exists() {
                                    let _ = std::fs::remove_dir_all(&path);
                                }
                            })
                            .await
                            .map_err(io::Error::other)?;

                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }
}
