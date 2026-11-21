//! Creates `chatroom`, the table of the chat plugin, with the name and the columns of the
//! PHP application. The time is stored in seconds since 1970.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_integer, big_pk_auto, string, text};

const CHATROOM: &str = "chatroom";
const INDEX: &str = "idx_chatroom_sent_at";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CHATROOM)
                    .col(big_pk_auto("id"))
                    .col(string("username"))
                    .col(text("gravatar"))
                    .col(string("uid"))
                    .col(big_integer("sent_at"))
                    .col(string("ip"))
                    .col(text("message"))
                    .to_owned(),
            )
            .await?;
        // The chat reads the newest messages.
        manager
            .create_index(
                Index::create()
                    .name(INDEX)
                    .table(CHATROOM)
                    .col("sent_at")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CHATROOM).to_owned())
            .await
    }
}
