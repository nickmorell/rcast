use crate::errors::DatabaseError;
use rusqlite::Transaction;
pub mod initial_migration_02082026;
pub trait Migration {
    fn name(&self) -> &'static str;

    fn up(&self, tx: &Transaction) -> Result<(), DatabaseError>;

    fn down(&self, tx: &Transaction) -> Result<(), DatabaseError>;
}
