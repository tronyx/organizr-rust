//! Adds `auth_subject` to `users`: the subject (`sub`) of the account at the provider of
//! an `oidc` login.
//!
//! A login through a provider finds its account by the provider and the subject first.
//! No two accounts have the same pair: the unique index allows any number of rows
//! without a subject, on both engines, because two empty values are never equal there.
//! The PHP application stores no subject, so an imported account has none until its
//! first login through the provider.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::string_null;

const USERS: &str = "users";
const INDEX: &str = "idx_users_auth_provider_subject";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(USERS)
                    .add_column(string_null("auth_subject"))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name(INDEX)
                    .table(USERS)
                    .col("auth_provider")
                    .col("auth_subject")
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(INDEX).table(USERS).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(USERS)
                    .drop_column("auth_subject")
                    .to_owned(),
            )
            .await
    }
}
