//! The `sessions` table.

use sea_orm::entity::prelude::*;

/// A row of `sessions`.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// The SHA-256 hash of the session token as 64 hexadecimal characters, unique. The
    /// token itself is not stored.
    #[sea_orm(unique)]
    pub token_hash: String,
    /// The row id of the user. Deleting the user deletes the session.
    pub user_id: i64,
    /// The API calls it `browser`.
    pub user_agent: Option<String>,
    /// The address the login came from, as text.
    pub ip: String,
    /// Seconds since the Unix epoch.
    pub created_at: i64,
    /// Seconds since the Unix epoch.
    pub expires_at: i64,
}

impl ActiveModelBehavior for ActiveModel {}
