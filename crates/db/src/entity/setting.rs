//! The `settings` table.

use sea_orm::entity::prelude::*;

/// A row of `settings`: one setting that is not at its default.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "settings")]
pub struct Model {
    /// The key of the setting.
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
    /// The value as JSON text.
    pub value: String,
}

impl ActiveModelBehavior for ActiveModel {}
