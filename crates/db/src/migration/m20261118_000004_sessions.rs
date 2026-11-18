//! Creates `sessions`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_integer, big_pk_auto, string, string_uniq, text_null};

const SESSIONS: &str = "sessions";
const USERS: &str = "users";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SESSIONS)
                    .col(big_pk_auto("id"))
                    .col(string_uniq("token_hash"))
                    .col(big_integer("user_id"))
                    .col(text_null("user_agent"))
                    .col(string("ip"))
                    .col(big_integer("created_at"))
                    .col(big_integer("expires_at"))
                    // A session has no use without its user, so it goes with the user.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sessions_user_id")
                            .from(SESSIONS, "user_id")
                            .to(USERS, "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_user_id")
                    .table(SESSIONS)
                    .col("user_id")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SESSIONS).to_owned())
            .await
    }
}
