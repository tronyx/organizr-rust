//! The `extensions` table.

use sea_orm::entity::prelude::*;

/// A row of `extensions`: an extension that an Admin approved.
#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "extensions")]
pub struct Model {
    /// The id of the extension: the name of its folder.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub enabled: bool,
    /// The SHA-256 of the file that was approved, as hex text.
    pub sha256: String,
    /// The permissions that were approved, with commas between them.
    pub permissions: String,
    pub least_access_level: i32,
    /// The values that the extension stored, as a JSON object of texts.
    #[sea_orm(column_type = "Text")]
    pub stored_values: String,
}

impl ActiveModelBehavior for ActiveModel {}
