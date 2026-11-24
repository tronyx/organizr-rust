//! The `user_preferences` table.

use sea_orm::entity::prelude::*;

/// A row of `user_preferences`. A column that is null means the default.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user_preferences")]
pub struct Model {
    /// The row id of the user. Deleting the user deletes the row.
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: i64,
    /// The layout the person chose.
    pub layout: Option<String>,
    /// The theme of the original layout.
    pub og_theme: Option<String>,
    /// The style of the original layout.
    pub og_style: Option<String>,
    /// The theme of the modern layout.
    pub modern_theme: Option<String>,
    /// The colour set of the modern layout.
    pub modern_color_set: Option<String>,
}

impl ActiveModelBehavior for ActiveModel {}
