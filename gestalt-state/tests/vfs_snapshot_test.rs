use gestalt_core::ports::outbound::vfs::{BlockEdit, VirtualFS};
use gestalt_state::vfs_snapshot::VfsSnapshotManager;
use gestalt_state::{StateDb, StateDbVfs};

#[tokio::test]
async fn test_snapshot_and_restore() {
    // Open DB
    let db = StateDb::open(":memory:").expect("Failed to open StateDb");
    
    // Setup VFS and Snapshot Manager
    let vfs = StateDbVfs::new(db.clone()).expect("Failed to create VFS");
    let snapshot_manager = VfsSnapshotManager::new(db.clone()).expect("Failed to create SnapshotManager");
    
    // Write initial file
    let block1 = BlockEdit {
        agent_id: "agent-1".into(),
        run_id: "run-1".into(),
        old_string: "".into(),
        new_string: "initial".into(),
        context: "".into(),
    };
    vfs.write_block("/test.txt", block1)
        .await
        .expect("Failed to write initial block");
        
    let (content1, _hash1) = vfs.read_file("/test.txt").await.expect("Failed to read");
    assert_eq!(content1, "initial");
    
    // Create snapshot
    let snapshot_id = snapshot_manager
        .create_snapshot("run-1")
        .expect("Failed to create snapshot");
        
    // Modify file
    let block2 = BlockEdit {
        agent_id: "agent-1".into(),
        run_id: "run-1".into(),
        old_string: "initial".into(),
        new_string: "modified".into(),
        context: "".into(),
    };
    vfs.write_block("/test.txt", block2)
        .await
        .expect("Failed to write second block");
        
    let (content2, _hash2) = vfs.read_file("/test.txt").await.expect("Failed to read");
    assert_eq!(content2, "modified");
    
    // Restore snapshot
    snapshot_manager
        .restore_snapshot(&snapshot_id)
        .expect("Failed to restore snapshot");
        
    // Verify file is back to initial
    let (content_restored, _hash_restored) = vfs.read_file("/test.txt").await.expect("Failed to read restored");
    assert_eq!(content_restored, "initial");
}
