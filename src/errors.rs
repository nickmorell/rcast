use std::fmt;

#[derive(Debug, Clone)]
pub enum DatabaseError {
    GenericError(String),
    MigrationError(String),
    NotFound(String),
    LockPoisoned,
}

impl DatabaseError {
    pub fn generic_error(message: impl Into<String>) -> Self {
        Self::GenericError(message.into())
    }

    pub fn migration_error(message: impl Into<String>) -> Self {
        Self::MigrationError(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GenericError(msg) => write!(f, "Database error: {}", msg),
            Self::NotFound(entity) => write!(f, "Not found: {}", entity),
            Self::MigrationError(issue) => {
                write!(f, "Migration error: {}", issue)
            }
            Self::LockPoisoned => write!(f, "Lock Poisoned {}", ""),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        DatabaseError::generic_error(err.to_string())
    }
}
