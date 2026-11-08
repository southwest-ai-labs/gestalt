use gestalt_state::StateDb;
use gestalt_state::checkpoint::CheckpointMode;
use std::fs;

struct TempDir {
    path: std::path::PathBuf,
}
impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
    fn path(&self) -> &std::path::Path {
        &self.path
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_wal_checkpoint() {
    let temp_dir = TempDir::new();
    let db_path = temp_dir.path().join("test_state.db");
    let wal_path = temp_dir.path().join("test_state.db-wal");
    
    let db = StateDb::open(&db_path).unwrap();
    
    // Create some initial data to ensure WAL exists and has content
    db.create_run("run1", "{}").unwrap();
    db.create_run("run2", "{}").unwrap();
    
    // Assert WAL file exists
    assert!(wal_path.exists());
    let initial_wal_size = std::fs::metadata(&wal_path).unwrap().len();
    assert!(initial_wal_size > 0);
    
    // Run TRUNCATE checkpoint
    let res = db.checkpoint_wal(CheckpointMode::TRUNCATE).unwrap();
    assert!(!res.is_blocked);
    
    // TRUNCATE mode resets the WAL file back to zero bytes
    let truncated_wal_size = std::fs::metadata(&wal_path).unwrap().len();
    assert_eq!(truncated_wal_size, 0);
    
    db.create_run("run3", "{}").unwrap();
    let new_wal_size = std::fs::metadata(&wal_path).unwrap().len();
    assert!(new_wal_size > 0);
    
    let res2 = db.checkpoint_wal(CheckpointMode::PASSIVE).unwrap();
    assert!(!res2.is_blocked);
}
