//! The error type of the database layer.

use sea_orm::{DbErr, SqlErr};
use thiserror::Error;

/// Why a database operation failed.
///
/// Broken constraints have their own variants, the same on every engine, so callers can
/// answer them without reading engine-specific error codes.
#[derive(Debug, Error)]
pub enum DbError {
    /// The URL names an engine other than `SQLite` or `PostgreSQL`.
    #[error("the database URL must start with `sqlite:` or `postgres:`")]
    UnsupportedUrl,

    /// A value that must be unique is already taken.
    #[error("a record with the same unique value already exists")]
    Duplicate,

    /// The operation would leave a record pointing at a record that does not exist.
    #[error("the operation breaks a reference between records")]
    ReferenceViolation,

    /// The install has been set up already, so it cannot be set up again.
    #[error("the install has been set up already")]
    AlreadySetUp,

    /// A stored record cannot be turned into its domain type.
    #[error("a stored {record} is not valid: {reason}")]
    InvalidRecord {
        /// The kind of record, for example `group`.
        record: &'static str,
        /// What is wrong with it.
        reason: String,
    },

    /// Any other failure of the database or of the connection to it.
    #[error("database error: {0}")]
    Engine(#[source] DbErr),
}

impl From<DbErr> for DbError {
    fn from(error: DbErr) -> Self {
        match error.sql_err() {
            Some(SqlErr::UniqueConstraintViolation(_)) => Self::Duplicate,
            Some(SqlErr::ForeignKeyConstraintViolation(_)) => Self::ReferenceViolation,
            _ => Self::Engine(error),
        }
    }
}
