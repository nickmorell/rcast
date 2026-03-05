use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct AddEpisodePosition;

impl Migration for AddEpisodePosition {
    fn name(&self) -> &'static str {
        "add_episode_position"
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute(
            "ALTER TABLE episodes ADD COLUMN position_seconds REAL NOT NULL DEFAULT 0.0",
            [],
        )?;
        Ok(())
    }

    fn down(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        // SQLite doesn't support DROP COLUMN before 3.35. Recreate without it.
        transaction.execute_batch(
            "CREATE TABLE episodes_backup AS SELECT
                id, podcast_id, title, description, url, audio_type,
                publish_date, is_played, duration, created_at, updated_at
             FROM episodes;
             DROP TABLE episodes;
             ALTER TABLE episodes_backup RENAME TO episodes;",
        )?;
        Ok(())
    }
}
