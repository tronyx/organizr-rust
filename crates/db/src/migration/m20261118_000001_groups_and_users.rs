//! Creates `groups` and `users`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    big_integer, big_pk_auto, boolean, integer_uniq, string, string_uniq, text, text_null,
};

const GROUPS: &str = "groups";
const USERS: &str = "users";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GROUPS)
                    .col(big_pk_auto("id"))
                    .col(integer_uniq("level"))
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
                    .table(USERS)
                    .col(big_pk_auto("id"))
                    .col(string("username"))
                    .col(string_uniq("username_key"))
                    .col(text_null("email"))
                    .col(text("password_hash"))
                    .col(big_integer("group_id"))
                    .col(boolean("locked"))
                    .col(text_null("image"))
                    .col(big_integer("created_at"))
                    // No `ON DELETE` action: the default refuses to delete a group that
                    // has users, and both engines report it as a foreign-key error.
                    // With `RESTRICT`, SQLite reports another error code (1811), which
                    // is not recognized as a foreign-key error.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_users_group_id")
                            .from(USERS, "group_id")
                            .to(GROUPS, "id"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_users_group_id")
                    .table(USERS)
                    .col("group_id")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(USERS).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(GROUPS).to_owned())
            .await
    }
}
