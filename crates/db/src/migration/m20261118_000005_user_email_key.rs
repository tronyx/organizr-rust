//! Adds `email_key` to `users`: the e-mail address in lower case.
//!
//! A user can log in with the e-mail address in any letter case. The lookup compares
//! this column, as the lookup by name compares `username_key`, so it works the same on
//! both engines. The column is not unique: the PHP application did not stop two accounts
//! from having the same address, and an imported install can hold such accounts.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::string_null;

const USERS: &str = "users";
const INDEX: &str = "idx_users_email_key";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(USERS)
                    .add_column(string_null("email_key"))
                    .to_owned(),
            )
            .await?;

        // Rows from before this migration. `LOWER` of SQLite changes only the letters A
        // to Z, which is enough for the addresses a development install can hold.
        manager
            .get_connection()
            .execute_unprepared("UPDATE users SET email_key = LOWER(email)")
            .await?;

        manager
            .create_index(
                Index::create()
                    .name(INDEX)
                    .table(USERS)
                    .col("email_key")
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
                    .drop_column("email_key")
                    .to_owned(),
            )
            .await
    }
}
