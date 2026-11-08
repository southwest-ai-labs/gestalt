use gestalt_router::lease::LeaseManager;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[test]
fn test_acquire_read() {
    let manager = LeaseManager::new();
    let path = PathBuf::from("/tmp/test.txt");

    // Multiple agents can acquire read lock
    assert!(manager.acquire_read(&path, "agent1"));
    assert!(manager.acquire_read(&path, "agent2"));

    // Release should work
    manager.release(&path, "agent1");
    manager.release(&path, "agent2");

    // Should be able to acquire again
    assert!(manager.acquire_read(&path, "agent3"));
}

#[test]
fn test_acquire_write() {
    let manager = LeaseManager::new();
    let path = PathBuf::from("/tmp/test.txt");
    let ttl = Duration::from_secs(5);

    // One agent can acquire write lock
    assert!(manager.acquire_write(&path, "agent1", ttl));

    // Same agent can re-acquire/extend write lock
    assert!(manager.acquire_write(&path, "agent1", ttl));

    manager.release(&path, "agent1");
}

#[test]
fn test_write_conflict() {
    let manager = LeaseManager::new();
    let path = PathBuf::from("/tmp/test.txt");
    let ttl = Duration::from_secs(5);

    // Agent1 acquires write lock
    assert!(manager.acquire_write(&path, "agent1", ttl));

    // Agent2 should not be able to acquire write lock
    assert!(!manager.acquire_write(&path, "agent2", ttl));
}

#[test]
fn test_read_write_conflict() {
    let manager = LeaseManager::new();
    let path = PathBuf::from("/tmp/test.txt");
    let ttl = Duration::from_secs(5);

    // Case 1: Read lock prevents write lock
    assert!(manager.acquire_read(&path, "agent1"));
    assert!(!manager.acquire_write(&path, "agent2", ttl));
    manager.release(&path, "agent1");

    // Case 2: Write lock prevents read lock
    assert!(manager.acquire_write(&path, "agent1", ttl));
    assert!(!manager.acquire_read(&path, "agent2"));
}

#[test]
fn test_write_ttl_expiration() {
    let manager = LeaseManager::new();
    let path = PathBuf::from("/tmp/test.txt");
    
    // Agent1 acquires short-lived write lock
    let ttl = Duration::from_millis(10);
    assert!(manager.acquire_write(&path, "agent1", ttl));
    
    // Agent2 fails to acquire immediately
    assert!(!manager.acquire_write(&path, "agent2", Duration::from_secs(1)));
    
    // Wait for TTL to expire
    thread::sleep(Duration::from_millis(20));
    
    // Agent2 should now be able to acquire the write lock
    assert!(manager.acquire_write(&path, "agent2", Duration::from_secs(1)));
    
    // Let's also check read lock after expiration
    manager.release(&path, "agent2"); // Need to release first for next test phase
    
    assert!(manager.acquire_write(&path, "agent3", Duration::from_millis(10)));
    thread::sleep(Duration::from_millis(20));
    
    // Agent4 should be able to read after Agent3's write lock expires
    assert!(manager.acquire_read(&path, "agent4"));
}
