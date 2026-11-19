use crate::statedb::StateDb;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateDbError {
    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
    #[error("Transaction aborted: {0}")]
    TransactionAborted(String),
}

impl StateDb {
    /// Execute a write transaction with `TransactionBehavior::Immediate`.
    ///
    /// Using Immediate transaction behavior ensures that concurrent WAL writers
    /// acquire the RESERVED lock upfront, respecting `busy_timeout` instead of
    /// failing immediately with `SQLITE_BUSY` during lock upgrade.
    pub fn with_write_transaction<T, F>(&self, f: F) -> Result<T, StateDbError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<T, StateDbError>,
    {
        let mut conn = self.conn().map_err(StateDbError::InternalError)?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(StateDbError::DbError)?;

        match f(&tx) {
            Ok(val) => {
                tx.commit().map_err(StateDbError::DbError)?;
                Ok(val)
            },
            Err(e) => {
                if let Err(rb_err) = tx.rollback() {
                    tracing::error!("SQLite transaction rollback failed: {rb_err}");
                }
                Err(e)
            },
        }
    }

    /// Execute a read-only snapshot transaction with `TransactionBehavior::Deferred`.
    pub fn with_read_transaction<T, F>(&self, f: F) -> Result<T, StateDbError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<T, StateDbError>,
    {
        let mut conn = self.conn().map_err(StateDbError::InternalError)?;

        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)
            .map_err(StateDbError::DbError)?;

        match f(&tx) {
            Ok(val) => {
                tx.commit().map_err(StateDbError::DbError)?;
                Ok(val)
            },
            Err(e) => {
                if let Err(rb_err) = tx.rollback() {
                    tracing::error!("SQLite transaction rollback failed: {rb_err}");
                }
                Err(e)
            },
        }
    }

    /// Execute a transaction with write behavior (`Immediate`) for backwards compatibility.
    pub fn with_transaction<T, F>(&self, f: F) -> Result<T, StateDbError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<T, StateDbError>,
    {
        self.with_write_transaction(f)
    }

    /// Async wrapper for `with_write_transaction` executed on a blocking thread pool
    /// via `tokio::task::spawn_blocking` so callers in async runtimes do not block Tokio worker threads.
    pub async fn with_write_transaction_async<T, F>(&self, f: F) -> Result<T, StateDbError>
    where
        T: Send + 'static,
        F: FnOnce(&rusqlite::Transaction) -> Result<T, StateDbError> + Send + 'static,
    {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.with_write_transaction(f))
            .await
            .map_err(|join_err| {
                StateDbError::InternalError(anyhow::anyhow!(
                    "spawn_blocking join error: {join_err}"
                ))
            })?
    }

    /// Async wrapper for `with_read_transaction` executed on a blocking thread pool
    /// via `tokio::task::spawn_blocking` so callers in async runtimes do not block Tokio worker threads.
    pub async fn with_read_transaction_async<T, F>(&self, f: F) -> Result<T, StateDbError>
    where
        T: Send + 'static,
        F: FnOnce(&rusqlite::Transaction) -> Result<T, StateDbError> + Send + 'static,
    {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.with_read_transaction(f))
            .await
            .map_err(|join_err| {
                StateDbError::InternalError(anyhow::anyhow!(
                    "spawn_blocking join error: {join_err}"
                ))
            })?
    }

    /// Async wrapper for `with_transaction` (alias to `with_write_transaction_async`).
    pub async fn with_transaction_async<T, F>(&self, f: F) -> Result<T, StateDbError>
    where
        T: Send + 'static,
        F: FnOnce(&rusqlite::Transaction) -> Result<T, StateDbError> + Send + 'static,
    {
        self.with_write_transaction_async(f).await
    }
}
