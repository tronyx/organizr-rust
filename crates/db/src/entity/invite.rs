//! The `invites` table.

use sea_orm::entity::prelude::*;

/// A row of `invites`.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "invites")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub code: String,
    /// The code in lower case, unique. A code is found whatever its letter case.
    #[sea_orm(unique)]
    pub code_key: String,
    /// Unix seconds.
    pub created_at: i64,
    pub email: String,
    pub username: String,
    /// Unix seconds.
    pub used_at: Option<i64>,
    pub used_by: Option<String>,
    pub used_from: Option<String>,
    pub is_valid: bool,
    pub kind: String,
    pub invited_by: String,
}

impl ActiveModelBehavior for ActiveModel {}
