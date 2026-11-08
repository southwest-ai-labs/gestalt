use crate::StateDb;
use std::fmt;
use thiserror::Error;

/// Modes for SQLite WAL checkpointing.
/// See https://sqlite.org/pragma.html#pragma_wal_checkpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointMode {
    /// PASSIVE: Checkpoint as many frames as possible without waiting for any database readers or writers to finish.
    PASSIVE,
    /// FULL: This mode blocks until there is no database writer and all readers are reading from the most recent database snapshot.
    FULL,
    /// RESTART: This mode works the same way as FULL with the addition that after checkpointing the log file it blocks until all readers are reading from the database snapshot created by the checkpoint.
    RESTART,
    /// TRUNCATE: This mode works the same way as RESTART with the addition that it also truncates the log file to zero bytes just prior to a successful return.
    TRUNCATE,
}

impl fmt::Display for CheckpointMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckpointMode::PASSIVE => write!(f, "PASSIVE"),
            CheckpointMode::FULL => write!(f, "FULL"),
            CheckpointMode::RESTART => write!(f, "RESTART"),
            CheckpointMode::TRUNCATE => write!(f, "TRUNCATE"),
        }
    }
}

/// The result of a WAL checkpoint operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointResult {
    /// True if the operation was blocked.
    pub is_blocked: bool,
    /// Total number of pages in the WAL file.
    pub wal_size: i32,
    /// Total number of pages checkpointed.
    pub checkpointed_size: i32,
}

#[derive(Debug, Error)]
pub enum StateDbError {
    #[error("Database lock poisoned")]
    LockPoisoned,
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl StateDb {
    /// Checkpoint the WAL file for the database.
    pub fn checkpoint_wal(&self, mode: CheckpointMode) -> Result<CheckpointResult, StateDbError> {
        let conn = self.conn().map_err(|_| StateDbError::LockPoisoned)?;
        
        let pragma = format!("PRAGMA wal_checkpoint({});", mode);
        let mut stmt = conn.prepare(&pragma)?;
        
        let mut rows = stmt.query([])?;
        
        if let Some(row) = rows.next()? {
            let is_blocked_int: i32 = row.get(0)?;
            let wal_size: i32 = row.get(1)?;
            let checkpointed_size: i32 = row.get(2)?;
            
            return Ok(CheckpointResult {
                is_blocked: is_blocked_int != 0,
                wal_size,
                checkpointed_size,
            });
        }
        
        Err(StateDbError::SqliteError(rusqlite::Error::QueryReturnedNoRows))
    }
}
