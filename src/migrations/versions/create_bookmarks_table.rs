use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct CreateBookmarksTable;

impl Migration for CreateBookmarksTable {
    fn name(&self) -> &'static str {
        "create_bookmarks_table"
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS bookmarks (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                podcast_id       INTEGER NOT NULL,
                episode_id       INTEGER,          -- NULL for podcast-level notes
                position_seconds REAL,             -- NULL for untimed notes
                note_text        TEXT NOT NULL DEFAULT '',
                created_at       INTEGER NOT NULL DEFAULT (unixepoch()),
                updated_at       INTEGER NOT NULL DEFAULT (unixepoch())
            );
            CREATE INDEX IF NOT EXISTS idx_bookmarks_episode
                ON bookmarks (episode_id);
            CREATE INDEX IF NOT EXISTS idx_bookmarks_podcast
                ON bookmarks (podcast_id);",
        )?;
        Ok(())
    }

    fn down(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute_batch(
            "DROP INDEX IF EXISTS idx_bookmarks_episode;
             DROP INDEX IF EXISTS idx_bookmarks_podcast;
             DROP TABLE IF EXISTS bookmarks;",
        )?;
        Ok(())
    }
}
