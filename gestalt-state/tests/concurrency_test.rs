use gestalt_state::statedb::StateDb;
use gestalt_state::transaction::StateDbError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn temp_db_path(test_name: &str) -> std::path::PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "gestalt_concurrency_{test_name}_{id}_{}.db",
        std::process::id()
    ));
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    path
}

/// Test 1: 16 concurrent writer threads against a shared StateDb instance
/// using `with_write_transaction` in WAL mode.
/// Verifies zero spurious SQLITE_BUSY deadlocks.
#[test]
fn test_16_concurrent_writers_shared_statedb() {
    let db_path = temp_db_path("shared");
    let db = StateDb::open(&db_path).expect("Failed to open StateDb");

    const NUM_THREADS: usize = 16;
    const WRITES_PER_THREAD: usize = 25;

    let mut handles = Vec::with_capacity(NUM_THREADS);

    for thread_id in 0..NUM_THREADS {
        let db_clone = db.clone();
        let handle = thread::spawn(move || -> Result<(), StateDbError> {
            for i in 0..WRITES_PER_THREAD {
                let run_id = format!("run-shared-t{thread_id}-w{i}");
                db_clone.with_write_transaction(|tx| {
                    tx.execute(
                        "INSERT INTO runs (run_id, spec_json, status, created_at) VALUES (?1, '{}', 'running', 'now')",
                        rusqlite::params![run_id],
                    )?;
                    Ok(())
                })?;
            }
            Ok(())
        });
        handles.push(handle);
    }

    for (i, handle) in handles.into_iter().enumerate() {
        let res = handle.join().expect("Thread panicked");
        assert!(res.is_ok(), "Thread {i} failed with error: {:?}", res.err());
    }

    // Verify all records were persisted
    let total_runs = db
        .with_read_transaction(|tx| {
            let count: i64 = tx.query_row("SELECT count(*) FROM runs", [], |r| r.get(0))?;
            Ok(count)
        })
        .expect("Failed to query runs count");

    assert_eq!(total_runs as usize, NUM_THREADS * WRITES_PER_THREAD);
    let _ = std::fs::remove_file(&db_path);
}

/// Test 2: 16 concurrent writer threads, each opening its own Connection handle
/// to the same SQLite WAL database file, executing `with_write_transaction`.
/// This directly exercises SQLite WAL multi-connection file locks and verifies
/// that TransactionBehavior::Immediate + busy_timeout avoids SQLITE_BUSY.
#[test]
fn test_16_concurrent_writers_multi_connection_wal() {
    let db_path = temp_db_path("multi_conn");
    // Initialize DB schema
    let _init_db = StateDb::open(&db_path).expect("Failed to init StateDb");

    const NUM_THREADS: usize = 16;
    const WRITES_PER_THREAD: usize = 20;

    let mut handles = Vec::with_capacity(NUM_THREADS);

    for thread_id in 0..NUM_THREADS {
        let path = db_path.clone();
        let handle = thread::spawn(move || -> Result<(), StateDbError> {
            let thread_db = StateDb::open(&path).expect("Failed to open connection in thread");
            for i in 0..WRITES_PER_THREAD {
                let run_id = format!("run-multi-t{thread_id}-w{i}");
                thread_db.with_write_transaction(|tx| {
                    tx.execute(
                        "INSERT INTO runs (run_id, spec_json, status, created_at) VALUES (?1, '{}', 'completed', 'now')",
                        rusqlite::params![run_id],
                    )?;
                    Ok(())
                })?;
            }
            Ok(())
        });
        handles.push(handle);
    }

    for (i, handle) in handles.into_iter().enumerate() {
        let res = handle.join().expect("Thread panicked");
        assert!(res.is_ok(), "Multi-conn thread {i} failed: {:?}", res.err());
    }

    let verify_db = StateDb::open(&db_path).expect("Failed to open verification db");
    let total_runs = verify_db
        .with_read_transaction(|tx| {
            let count: i64 = tx.query_row("SELECT count(*) FROM runs", [], |r| r.get(0))?;
            Ok(count)
        })
        .expect("Query failed");

    assert_eq!(total_runs as usize, NUM_THREADS * WRITES_PER_THREAD);
    let _ = std::fs::remove_file(&db_path);
}

/// Test 3: Clean rollback on closure error under concurrent writers.
/// Spawns 16 concurrent threads where even threads commit and odd threads abort.
/// Verifies:
/// - Even thread records are persisted.
/// - Odd thread records are rolled back cleanly.
/// - No SQLITE_BUSY deadlocks occur.
#[test]
fn test_concurrent_writers_with_interleaved_rollbacks() {
    let db_path = temp_db_path("rollbacks");
    let db = StateDb::open(&db_path).expect("Failed to open StateDb");

    const NUM_THREADS: usize = 16;
    const WRITES_PER_THREAD: usize = 15;

    let mut handles = Vec::with_capacity(NUM_THREADS);

    for thread_id in 0..NUM_THREADS {
        let db_clone = db.clone();
        let handle = thread::spawn(move || {
            let is_failing_thread = thread_id % 2 != 0;

            for i in 0..WRITES_PER_THREAD {
                let run_id = format!("run-t{thread_id}-w{i}");
                let res = db_clone.with_write_transaction(|tx| {
                    tx.execute(
                        "INSERT INTO runs (run_id, spec_json, status, created_at) VALUES (?1, '{}', 'test', 'now')",
                        rusqlite::params![run_id],
                    )?;

                    if is_failing_thread {
                        return Err(StateDbError::TransactionAborted(format!(
                            "Simulated abort for thread {thread_id} write {i}"
                        )));
                    }
                    Ok(())
                });

                if is_failing_thread {
                    assert!(res.is_err(), "Expected error for failing thread");
                } else {
                    assert!(res.is_ok(), "Expected success for succeeding thread");
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify even threads committed, odd threads rolled back
    let committed_count: i64 = db
        .with_read_transaction(|tx| {
            let count: i64 = tx.query_row("SELECT count(*) FROM runs", [], |r| r.get(0))?;
            Ok(count)
        })
        .expect("Query failed");

    let expected_even_threads = NUM_THREADS / 2;
    assert_eq!(
        committed_count as usize,
        expected_even_threads * WRITES_PER_THREAD
    );

    // Verify specifically that NO odd runs exist
    for thread_id in (1..NUM_THREADS).step_by(2) {
        for i in 0..WRITES_PER_THREAD {
            let run_id = format!("run-t{thread_id}-w{i}");
            let exists: bool = db
                .with_read_transaction(|tx| {
                    let mut stmt = tx.prepare("SELECT 1 FROM runs WHERE run_id = ?1")?;
                    Ok(stmt.exists(rusqlite::params![run_id])?)
                })
                .expect("Query failed");
            assert!(!exists, "Aborted run {run_id} should not exist");
        }
    }

    let _ = std::fs::remove_file(&db_path);
}

/// Test 4: Async wrapper methods on StateDb (`with_write_transaction_async`,
/// `with_read_transaction_async`, `with_transaction_async`) under concurrent Tokio tasks.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_async_transaction_wrappers_concurrency() {
    let db_path = temp_db_path("async");
    let db = Arc::new(StateDb::open(&db_path).expect("Failed to open StateDb"));

    const NUM_TASKS: usize = 16;
    const WRITES_PER_TASK: usize = 15;

    let mut tasks = Vec::with_capacity(NUM_TASKS);

    for task_id in 0..NUM_TASKS {
        let db_clone = db.clone();
        let task = tokio::spawn(async move {
            for i in 0..WRITES_PER_TASK {
                let run_id = format!("run-async-task{task_id}-w{i}");
                db_clone
                    .with_write_transaction_async(move |tx| {
                        tx.execute(
                            "INSERT INTO runs (run_id, spec_json, status, created_at) VALUES (?1, '{}', 'async_ok', 'now')",
                            rusqlite::params![run_id],
                        )?;
                        Ok(())
                    })
                    .await?;

                // Also test read snapshot async
                let exists: bool = db_clone
                    .with_read_transaction_async({
                        let run_id_check = format!("run-async-task{task_id}-w{i}");
                        move |tx| {
                            let mut stmt = tx.prepare("SELECT 1 FROM runs WHERE run_id = ?1")?;
                            Ok(stmt.exists(rusqlite::params![run_id_check])?)
                        }
                    })
                    .await?;

                assert!(exists, "Run must be visible in read transaction");
            }
            Ok::<(), StateDbError>(())
        });
        tasks.push(task);
    }

    for task in tasks {
        let res = task.await.expect("Tokio task panicked");
        assert!(res.is_ok(), "Task failed: {:?}", res.err());
    }

    // Verify backwards compatible with_transaction_async
    let db_clone = db.clone();
    let res = db_clone
        .with_transaction_async(|tx| {
            tx.execute(
                "INSERT INTO runs (run_id, spec_json, status, created_at) VALUES ('compat-run', '{}', 'ok', 'now')",
                [],
            )?;
            Ok(())
        })
        .await;
    assert!(res.is_ok());

    let final_count: i64 = db
        .with_read_transaction(|tx| {
            let count: i64 = tx.query_row("SELECT count(*) FROM runs", [], |r| r.get(0))?;
            Ok(count)
        })
        .expect("Query failed");

    assert_eq!(final_count as usize, NUM_TASKS * WRITES_PER_TASK + 1);

    let _ = std::fs::remove_file(&db_path);
}
