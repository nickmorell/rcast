use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct AddEpisodeTotalListenTime;

impl Migration for AddEpisodeTotalListenTime {
    fn name(&self) -> &'static str {
        "add_episode_listen_time"
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "ALTER TABLE episodes ADD COLUMN total_listen_seconds INTEGER NOT NULL DEFAULT 0;",
        )?;
        Ok(())
    }

    fn down(&self, _transaction: &Transaction) -> Result<(), DatabaseError> {
        // SQLite does not support DROP COLUMN; this migration is intentionally irreversible.
        Ok(())
    }
}
