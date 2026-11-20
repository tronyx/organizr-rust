//! The `bookmark_categories` table.

use sea_orm::entity::prelude::*;

/// A row of `bookmark_categories`.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "bookmark_categories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// The number that bookmarks name the category with, unique. The API calls it
    /// `category_id`.
    #[sea_orm(unique)]
    pub number: i64,
    /// The place on the page. The API calls it `order`.
    pub position: i64,
    pub name: String,
    /// The name in lower case, unique.
    #[sea_orm(unique)]
    pub name_key: String,
    pub is_default: bool,
}

impl ActiveModelBehavior for ActiveModel {}
