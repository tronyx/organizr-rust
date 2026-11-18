//! The `users` table.

use sea_orm::entity::prelude::*;

/// A row of `users`.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub username: String,
    /// The user name in lower case, unique. It makes names unique, and lookups work,
    /// whatever the letter case, in the same way on every engine.
    #[sea_orm(unique)]
    pub username_key: String,
    pub email: Option<String>,
    /// The e-mail address in lower case, for the lookup at login. Not unique.
    pub email_key: Option<String>,
    pub password_hash: String,
    /// The row id of the group. A group with users cannot be deleted.
    pub group_id: i64,
    pub locked: bool,
    pub image: Option<String>,
    /// Seconds since the Unix epoch.
    pub created_at: i64,
}

impl ActiveModelBehavior for ActiveModel {}
