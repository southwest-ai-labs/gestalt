use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

#[path = "../src/worktree/gc.rs"]
mod gc;
use gc::WorktreeGc;

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let mut path = std::env::temp_dir();
        let unique_id = Uuid::new_v4().to_string();
        path.push(format!("{}_{}", name, unique_id));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[tokio::test]
async fn test_clean_stale() {
    let temp_dir = TempDir::new("gestalt_gc_test");

    let fresh_dir = temp_dir.path.join("fresh_wt");
    fs::create_dir(&fresh_dir).unwrap();
    
    let stale_dir = temp_dir.path.join("stale_wt");
    fs::create_dir(&stale_dir).unwrap();

    let stale_time = SystemTime::now() - Duration::from_secs(3 * 3600);
    
    // Set file time natively using Rust
    let file = fs::File::open(&stale_dir).unwrap();
    file.set_modified(stale_time).unwrap();
    
    let gc = WorktreeGc {
        base_dir: temp_dir.path.clone(),
        max_age: Duration::from_secs(2 * 3600),
    };

    let cleaned = gc.clean_stale().await.unwrap();
    assert_eq!(cleaned, 1);
    
    assert!(fresh_dir.exists(), "Fresh worktree should not be pruned");
    assert!(!stale_dir.exists(), "Stale worktree should be pruned");
}
