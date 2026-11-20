//! Creates `bookmark_categories` and `bookmark_tabs`, the tables of the bookmark
//! plugin.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    big_integer, big_integer_uniq, big_pk_auto, boolean, integer, string, string_uniq, text,
};

const CATEGORIES: &str = "bookmark_categories";
const TABS: &str = "bookmark_tabs";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CATEGORIES)
                    .col(big_pk_auto("id"))
                    .col(big_integer_uniq("number"))
                    .col(big_integer("position"))
                    .col(string("name"))
                    .col(string_uniq("name_key"))
                    .col(boolean("is_default"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TABS)
                    .col(big_pk_auto("id"))
                    .col(big_integer("position"))
                    .col(big_integer("category_number"))
                    .col(string("name"))
                    .col(string_uniq("name_key"))
                    .col(text("url"))
                    .col(boolean("enabled"))
                    .col(integer("least_access_level"))
                    .col(text("image"))
                    .col(string("background_color"))
                    .col(string("text_color"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TABS).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CATEGORIES).to_owned())
            .await
    }
}
