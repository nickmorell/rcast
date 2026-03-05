use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct AddEpisodeUniqueIndex;

impl Migration for AddEpisodeUniqueIndex {
    fn name(&self) -> &'static str {
        "add_episode_unique_index"
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        // Remove any duplicates that already exist before adding the constraint.
        // Keep the row with the lowest id (the first one inserted).
        transaction.execute(
            "DELETE FROM episodes
             WHERE id NOT IN (
                 SELECT MIN(id)
                 FROM episodes
                 GROUP BY podcast_id, url
             )",
            [],
        )?;

        transaction.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_episodes_podcast_url
             ON episodes (podcast_id, url)",
            [],
        )?;

        Ok(())
    }

    fn down(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute("DROP INDEX IF EXISTS idx_episodes_podcast_url", [])?;
        Ok(())
    }
}
