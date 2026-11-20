//! Creates `extensions`, the table of the approved extensions.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{boolean, integer, string, text};

const EXTENSIONS: &str = "extensions";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EXTENSIONS)
                    .col(string("id").primary_key().to_owned())
                    .col(boolean("enabled"))
                    .col(string("sha256"))
                    .col(string("permissions"))
                    .col(integer("least_access_level"))
                    .col(text("stored_values"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EXTENSIONS).to_owned())
            .await
    }
}
