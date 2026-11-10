use crate::transaction::StateDbError;
use crate::StateDb;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct VfsSnapshot {
    pub snapshot_id: String,
    pub run_id: String,
    pub created_at: String,
    pub files: HashMap<PathBuf, String>,
}

pub struct VfsSnapshotManager {
    pub state_db: StateDb,
}

impl VfsSnapshotManager {
    pub fn new(state_db: StateDb) -> Result<Self, StateDbError> {
        let manager = Self { state_db };
        manager.migrate()?;
        Ok(manager)
    }

    pub fn migrate(&self) -> Result<(), StateDbError> {
        let conn = self.state_db.conn().map_err(StateDbError::InternalError)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS vfs_snapshots (
                snapshot_id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS vfs_snapshot_files (
                snapshot_id TEXT NOT NULL,
                path TEXT NOT NULL,
                version_hash TEXT NOT NULL,
                PRIMARY KEY (snapshot_id, path)
            );",
        )
        .map_err(StateDbError::DbError)?;
        Ok(())
    }

    pub fn create_snapshot(&self, run_id: &str) -> Result<String, StateDbError> {
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();

        let conn = self.state_db.conn().map_err(StateDbError::InternalError)?;
        let mut stmt = conn
            .prepare(
                "SELECT path, version_hash FROM (
                    SELECT path, version_hash, ROW_NUMBER() OVER (PARTITION BY path ORDER BY created_at DESC) as rn 
                    FROM file_versions
                ) WHERE rn = 1",
            )
            .map_err(StateDbError::DbError)?;

        let mut files_to_insert: Vec<(String, String)> = Vec::new();
        {
            let mut rows = stmt.query([]).map_err(StateDbError::DbError)?;
            while let Some(row) = rows.next().map_err(StateDbError::DbError)? {
                let path: String = row.get(0).map_err(StateDbError::DbError)?;
                let version_hash: String = row.get(1).map_err(StateDbError::DbError)?;
                files_to_insert.push((path, version_hash));
            }
        }

        drop(stmt);
        drop(conn);

        self.state_db.with_transaction(|tx| {
            tx.execute(
                "INSERT INTO vfs_snapshots (snapshot_id, run_id, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![snapshot_id, run_id, created_at],
            )
            .map_err(StateDbError::DbError)?;

            let mut insert_stmt = tx
                .prepare(
                    "INSERT INTO vfs_snapshot_files (snapshot_id, path, version_hash) VALUES (?1, ?2, ?3)",
                )
                .map_err(StateDbError::DbError)?;

            for (path, version_hash) in &files_to_insert {
                insert_stmt
                    .execute(rusqlite::params![snapshot_id, path, version_hash])
                    .map_err(StateDbError::DbError)?;
            }

            Ok(())
        })?;

        Ok(snapshot_id)
    }

    pub fn restore_snapshot(&self, snapshot_id: &str) -> Result<(), StateDbError> {
        self.state_db.with_transaction(|tx| {
            let created_at: String = tx
                .query_row(
                    "SELECT created_at FROM vfs_snapshots WHERE snapshot_id = ?1",
                    rusqlite::params![snapshot_id],
                    |row| row.get(0),
                )
                .map_err(StateDbError::DbError)?;

            let mut stmt = tx
                .prepare("SELECT path FROM vfs_snapshot_files WHERE snapshot_id = ?1")
                .map_err(StateDbError::DbError)?;
            
            let mut paths: Vec<String> = Vec::new();
            {
                let mut rows = stmt.query(rusqlite::params![snapshot_id]).map_err(StateDbError::DbError)?;
                while let Some(row) = rows.next().map_err(StateDbError::DbError)? {
                    let path: String = row.get(0).map_err(StateDbError::DbError)?;
                    paths.push(path);
                }
            }
            drop(stmt);

            let mut delete_stmt = tx
                .prepare("DELETE FROM file_versions WHERE path = ?1 AND created_at > ?2")
                .map_err(StateDbError::DbError)?;

            for path in paths {
                delete_stmt
                    .execute(rusqlite::params![path, created_at])
                    .map_err(StateDbError::DbError)?;
            }

            Ok(())
        })?;

        Ok(())
    }
}
