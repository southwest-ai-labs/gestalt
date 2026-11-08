use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Transaction;

pub struct PruningPolicy {
    pub max_events: usize,
    pub max_age_days: usize,
}

impl Default for PruningPolicy {
    fn default() -> Self {
        Self {
            max_events: 50_000,
            max_age_days: 30,
        }
    }
}

pub fn apply_retention_policy(tx: &Transaction<'_>, policy: &PruningPolicy) -> Result<usize> {
    let cutoff = Utc::now() - chrono::Duration::days(policy.max_age_days as i64);
    let cutoff_str = cutoff.to_rfc3339();

    let pruned_by_time = tx
        .execute(
            "DELETE FROM timeline WHERE created_at < ?1",
            rusqlite::params![cutoff_str],
        )
        .context("Failed to prune timeline events by age")?;

    let pruned_by_count = tx
        .execute(
            "DELETE FROM timeline WHERE seq IN (
            SELECT seq FROM timeline ORDER BY seq DESC LIMIT -1 OFFSET ?1
        )",
            rusqlite::params![policy.max_events],
        )
        .context("Failed to prune timeline events by count")?;

    Ok(pruned_by_time + pruned_by_count)
}
