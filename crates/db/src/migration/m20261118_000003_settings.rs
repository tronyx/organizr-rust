//! Creates `settings`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{string, text};

const SETTINGS: &str = "settings";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SETTINGS)
                    .col(string("name").primary_key().to_owned())
                    .col(text("value"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SETTINGS).to_owned())
            .await
    }
}
