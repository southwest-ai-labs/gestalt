use gestalt_state::statedb::StateDb;
use gestalt_state::transaction::StateDbError;
use std::sync::atomic::{AtomicUsize, Ordering};

fn rand_num_helper() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn setup_db() -> StateDb {
    let db_path = std::env::temp_dir().join(format!("test_transaction_test_{}.db", rand_num_helper()));
    if db_path.exists() {
        let _ = std::fs::remove_file(&db_path);
    }
    StateDb::open(db_path).unwrap()
}

#[test]
fn test_with_transaction_commit() {
    let db = setup_db();
    
    let result: Result<(), StateDbError> = db.with_transaction(|tx| {
        tx.execute(
            "INSERT INTO runs (run_id, spec_json, status, created_at) VALUES ('run-1', '{}', 'running', 'now')",
            [],
        )?;
        Ok(())
    });
    
    assert!(result.is_ok());
    
    let run = db.get_run("run-1").unwrap();
    assert!(run.is_some());
}

#[test]
fn test_with_transaction_rollback_on_error() {
    let db = setup_db();
    
    let result: Result<(), StateDbError> = db.with_transaction(|tx| {
        tx.execute(
            "INSERT INTO runs (run_id, spec_json, status, created_at) VALUES ('run-rollback', '{}', 'running', 'now')",
            [],
        )?;
        
        Err(StateDbError::TransactionAborted("Simulated error".to_string()))
    });
    
    assert!(result.is_err());
    
    let run = db.get_run("run-rollback").unwrap();
    assert!(run.is_none());
}
