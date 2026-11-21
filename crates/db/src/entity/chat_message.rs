//! The `chatroom` table.

use sea_orm::entity::prelude::*;

/// A row of `chatroom`.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "chatroom")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub username: String,
    pub gravatar: String,
    pub uid: String,
    /// Unix seconds.
    pub sent_at: i64,
    pub ip: String,
    pub message: String,
}

impl ActiveModelBehavior for ActiveModel {}
