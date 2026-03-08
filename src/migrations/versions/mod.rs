use crate::errors::DatabaseError;
use rusqlite::Transaction;
pub mod add_episode_position;
pub mod add_episode_unique_index;
pub mod add_podcast_last_synced_at;
pub mod create_bookmarks_table;
pub mod initial_migration_02082026;
pub trait Migration {
    fn name(&self) -> &'static str;

    fn up(&self, tx: &Transaction) -> Result<(), DatabaseError>;

    #[allow(dead_code)]
    fn down(&self, tx: &Transaction) -> Result<(), DatabaseError>;
}
