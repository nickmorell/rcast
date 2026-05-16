use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct AddPodcastPreferences;

impl Migration for AddPodcastPreferences {
    fn name(&self) -> &'static str {
        "add_podcast_preferences"
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "ALTER TABLE podcasts ADD COLUMN speed_preset REAL;
             ALTER TABLE podcasts ADD COLUMN auto_download INTEGER;
             ALTER TABLE podcasts ADD COLUMN keep_episodes_count INTEGER;
             ALTER TABLE podcasts ADD COLUMN skip_intro_seconds INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE podcasts ADD COLUMN skip_outro_seconds INTEGER NOT NULL DEFAULT 0;",
        )?;
        Ok(())
    }

    fn down(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "ALTER TABLE podcasts DROP COLUMN speed_preset;
             ALTER TABLE podcasts DROP COLUMN auto_download;
             ALTER TABLE podcasts DROP COLUMN keep_episodes_count;
             ALTER TABLE podcasts DROP COLUMN skip_intro_seconds;
             ALTER TABLE podcasts DROP COLUMN skip_outro_seconds;",
        )?;
        Ok(())
    }
}
