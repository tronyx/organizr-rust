//! The `tabs` table.

use sea_orm::entity::prelude::*;

/// A row of `tabs`.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tabs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// The place in the category. The API calls it `order`.
    pub position: i64,
    /// The row id of the category. A category with tabs cannot be deleted.
    pub category_id: i64,
    pub name: String,
    /// The name in lower case, unique.
    #[sea_orm(unique)]
    pub name_key: String,
    pub url: Option<String>,
    pub url_local: Option<String>,
    pub is_default: bool,
    pub enabled: bool,
    /// The level with the least access that sees the tab. The API calls it `group_id`.
    pub least_access_level: i32,
    /// The level with the most access that sees the tab. The API calls it `group_id_max`.
    pub most_access_level: i32,
    pub add_to_admin: bool,
    pub image: Option<String>,
    /// How the tab opens: 0, 1 or 2. The API calls it `type`.
    pub kind: i32,
    pub splash: bool,
    pub ping: bool,
    pub ping_url: Option<String>,
    pub timeout: bool,
    pub timeout_ms: Option<i64>,
    pub preload: bool,
}

impl ActiveModelBehavior for ActiveModel {}
