//! Creates `invites`, the table of the invites plugin.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    big_integer, big_integer_null, big_pk_auto, boolean, string, string_null, string_uniq,
};

const INVITES: &str = "invites";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(INVITES)
                    .col(big_pk_auto("id"))
                    .col(string("code"))
                    .col(string_uniq("code_key"))
                    .col(big_integer("created_at"))
                    .col(string("email"))
                    .col(string("username"))
                    .col(big_integer_null("used_at"))
                    .col(string_null("used_by"))
                    .col(string_null("used_from"))
                    .col(boolean("is_valid"))
                    .col(string("kind"))
                    .col(string("invited_by"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(INVITES).to_owned())
            .await
    }
}
