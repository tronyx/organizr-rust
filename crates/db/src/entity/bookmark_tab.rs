//! The `bookmark_tabs` table.

use sea_orm::entity::prelude::*;

/// A row of `bookmark_tabs`.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "bookmark_tabs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// The place in its category. The API calls it `order`.
    pub position: i64,
    /// The number of its category. No rule of the database ties it to a category: a
    /// bookmark of a category that is gone stays, and the page does not show it.
    pub category_number: i64,
    pub name: String,
    /// The name in lower case, unique.
    #[sea_orm(unique)]
    pub name_key: String,
    pub url: String,
    pub enabled: bool,
    /// The level with the least access that sees the bookmark. The API calls it
    /// `group_id`.
    pub least_access_level: i32,
    pub image: String,
    pub background_color: String,
    pub text_color: String,
}

impl ActiveModelBehavior for ActiveModel {}
