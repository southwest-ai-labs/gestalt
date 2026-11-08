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
    pub fn with_transaction<T, F>(&self, f: F) -> Result<T, StateDbError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<T, StateDbError>,
    {
        let mut conn = self.conn().map_err(StateDbError::InternalError)?;
        
        let tx = conn.transaction().map_err(StateDbError::DbError)?;
        
        match f(&tx) {
            Ok(val) => {
                tx.commit().map_err(StateDbError::DbError)?;
                Ok(val)
            }
            Err(e) => {
                let _ = tx.rollback();
                Err(e)
            }
        }
    }
}
