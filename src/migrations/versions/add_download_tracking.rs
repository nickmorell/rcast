use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct AddDownloadTracking;

impl Migration for AddDownloadTracking {
    fn name(&self) -> &'static str {
        "add_download_tracking"
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "ALTER TABLE episodes ADD COLUMN download_status TEXT NOT NULL DEFAULT 'not_downloaded';
             ALTER TABLE episodes ADD COLUMN downloaded_path TEXT;
             ALTER TABLE episodes ADD COLUMN speed_preset REAL;",
        )?;
        Ok(())
    }

    fn down(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "ALTER TABLE episodes DROP COLUMN download_status;
             ALTER TABLE episodes DROP COLUMN downloaded_path;
             ALTER TABLE episodes DROP COLUMN speed_preset;",
        )?;
        Ok(())
    }
}
