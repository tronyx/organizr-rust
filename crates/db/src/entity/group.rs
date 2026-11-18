//! The `groups` table.

use sea_orm::entity::prelude::*;

/// A row of `groups`.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "groups")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// The access level, unique. The API calls it `group_id`.
    #[sea_orm(unique)]
    pub level: i32,
    pub name: String,
    /// The name in lower case, unique. It makes names unique whatever the letter case,
    /// in the same way on every engine.
    #[sea_orm(unique)]
    pub name_key: String,
    pub image: Option<String>,
    pub is_default: bool,
}

impl ActiveModelBehavior for ActiveModel {}
