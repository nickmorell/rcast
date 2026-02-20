use crate::errors::DatabaseError;
use crate::migrations::versions::Migration;
use rusqlite::Transaction;

pub struct InitialMigration;

impl Migration for InitialMigration {
    fn name(&self) -> &'static str {
        return "Initial Migration";
    }

    fn up(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        // Create Settings Table
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
            [],
        )?;

        // Create Podcast Table
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS podcasts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            image_url TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
            [],
        )?;

        // Create Episode Table
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                podcast_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                url TEXT NOT NULL,
                audio_type TEXT NOT NULL,
                publish_date INTEGER NOT NULL,
                is_played INTEGER NOT NULL DEFAULT 0,
                duration INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (podcast_id) REFERENCES podcasts(id) ON DELETE CASCADE
        )",
            [],
        )?;

        // Create Queue Table
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            episode_id INTEGER NOT NULL,
            position INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (episode_id) REFERENCES episodes(id) ON DELETE CASCADE
        )",
            [],
        )?;
        Ok(())
    }

    fn down(&self, transaction: &Transaction) -> Result<(), DatabaseError> {
        transaction.execute("DROP TABLE IF EXISTS queue", [])?;
        transaction.execute("DROP TABLE IF EXISTS settings", [])?;
        transaction.execute("DROP TABLE IF EXISTS episodes", [])?;
        transaction.execute("DROP TABLE IF EXISTS podcasts", [])?;
        transaction.execute("DROP TABLE IF EXISTS migrations", [])?;
        Ok(())
    }
}
