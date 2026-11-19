//! Adds the recovery password to `users`.
//!
//! A person who forgot the password asks for a new one by e-mail. The new password is
//! stored next to the password of the account, as a hash, with the moment until which
//! it works:
//!
//! - `recovery_hash`: the hash of the password from the e-mail.
//! - `recovery_expires_at`: until when that password works, in seconds since 1970.
//!
//! The password of the account stays as it is until somebody logs in with the password
//! from the e-mail. So a stranger who asks for a new password for the address of
//! another person does not lock that person out.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_integer_null, text_null};

const USERS: &str = "users";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // One column per statement: SQLite adds only one column at a time.
        for column in [
            text_null("recovery_hash"),
            big_integer_null("recovery_expires_at"),
        ] {
            manager
                .alter_table(Table::alter().table(USERS).add_column(column).to_owned())
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in ["recovery_expires_at", "recovery_hash"] {
            manager
                .alter_table(Table::alter().table(USERS).drop_column(column).to_owned())
                .await?;
        }
        Ok(())
    }
}
