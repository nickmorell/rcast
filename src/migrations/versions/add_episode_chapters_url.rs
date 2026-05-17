use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct AddEpisodeChaptersUrl;

impl Migration for AddEpisodeChaptersUrl {
    fn name(&self) -> &'static str {
        "add_episode_chapters_url"
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "ALTER TABLE episodes ADD COLUMN chapters_url TEXT;",
        )?;
        Ok(())
    }

    fn down(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "ALTER TABLE episodes DROP COLUMN chapters_url;",
        )?;
        Ok(())
    }
}
