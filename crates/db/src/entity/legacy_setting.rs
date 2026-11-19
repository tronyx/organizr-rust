//! The `legacy_settings` table: settings of an imported install that this application
//! does not know. Each value is sealed.

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "legacy_settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
    pub value: String,
}

impl ActiveModelBehavior for ActiveModel {}
