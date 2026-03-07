use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct AddPodcastLastSyncedAt;

impl Migration for AddPodcastLastSyncedAt {
    fn name(&self) -> &'static str {
        "add_podcast_last_synced_at"
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute(
            "ALTER TABLE podcasts ADD COLUMN last_synced_at INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
        Ok(())
    }

    fn down(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "CREATE TABLE podcasts_backup AS SELECT
                id, url, title, description, image_url, created_at, updated_at
             FROM podcasts;
             DROP TABLE podcasts;
             ALTER TABLE podcasts_backup RENAME TO podcasts;",
        )?;
        Ok(())
    }
}
