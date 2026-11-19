//! Gestalt State — SQLite StateDB + DashMap MemState for agent orchestration.
//!
//! Provides two state backends:
//! - [`StateDb`]: Persistent SQLite-based state store with WAL mode, run/agent/lock/timeline tables.
//! - [`MemState`]: In-memory DashMap-backed state store with broadcast channels for events.

pub mod checkpoint;
pub mod memstate;
pub mod retention;
pub mod schema;
pub mod statedb;
pub mod transaction;
pub mod vfs_snapshot;
pub mod virtual_fs;

pub use memstate::MemState;
pub use schema::{AgentRecord, AgentState, FileLock, RunRecord, TimelineEvent};
#[allow(deprecated)]
#[deprecated(note = "Use TransactionalStateDb instead")]
pub use statedb::TransactionalStateDB;
pub use statedb::{DbMetrics, StateDb, TransactionalStateDb};
pub use transaction::StateDbError;
pub use vfs_snapshot::{VfsSnapshot, VfsSnapshotManager};
pub use virtual_fs::StateDbVfs;
