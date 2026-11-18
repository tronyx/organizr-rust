//! Creates `categories` and `tabs`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    big_integer, big_integer_null, big_integer_uniq, big_pk_auto, boolean, integer, string,
    string_uniq, text_null,
};

const CATEGORIES: &str = "categories";
const TABS: &str = "tabs";

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
                    .col(text_null("image"))
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
                    .col(big_integer("category_id"))
                    .col(string("name"))
                    .col(string_uniq("name_key"))
                    .col(text_null("url"))
                    .col(text_null("url_local"))
                    .col(boolean("is_default"))
                    .col(boolean("enabled"))
                    .col(integer("least_access_level"))
                    .col(integer("most_access_level"))
                    .col(boolean("add_to_admin"))
                    .col(text_null("image"))
                    .col(integer("kind"))
                    .col(boolean("splash"))
                    .col(boolean("ping"))
                    .col(text_null("ping_url"))
                    .col(boolean("timeout"))
                    .col(big_integer_null("timeout_ms"))
                    .col(boolean("preload"))
                    // No `ON DELETE` action, as for the users of a group: the default
                    // refuses to delete a category that has tabs.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tabs_category_id")
                            .from(TABS, "category_id")
                            .to(CATEGORIES, "id"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_tabs_category_id")
                    .table(TABS)
                    .col("category_id")
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
