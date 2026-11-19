//! Adds to `users` what the PHP application keeps about the login of a user, and creates
//! `legacy_settings`.
//!
//! The PHP application stores one text, `auth_service`, per user. It is `internal`, the
//! name of the service that checked the password (`plex`), `oidc::<provider>`, or, for a
//! user with a second factor, `<kind>::<secret>`. Here each part has its own column:
//!
//! - `auth_service`: the service, `internal` for a password of this application.
//! - `auth_provider`: the provider of an `oidc` login.
//! - `two_factor_kind` and `two_factor_secret`: the second factor. The secret is sealed.
//! - `plex_token`: the token of a Plex account, sealed.
//!
//! `legacy_settings` holds the settings of an imported install that this application
//! does not know, for example those of a plugin that is not part of Organizr. Each value
//! is sealed. Nothing reads them yet; they are kept so that an import loses nothing.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{string, string_null, text, text_null};

const USERS: &str = "users";
const LEGACY_SETTINGS: &str = "legacy_settings";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // One column per statement: SQLite adds only one column at a time.
        let columns = [
            string("auth_service").default("internal").to_owned(),
            string_null("auth_provider"),
            string_null("two_factor_kind"),
            text_null("two_factor_secret"),
            text_null("plex_token"),
        ];
        for column in columns {
            manager
                .alter_table(Table::alter().table(USERS).add_column(column).to_owned())
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(LEGACY_SETTINGS)
                    .col(string("name").primary_key().to_owned())
                    .col(text("value"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LEGACY_SETTINGS).to_owned())
            .await?;
        for column in [
            "plex_token",
            "two_factor_secret",
            "two_factor_kind",
            "auth_provider",
            "auth_service",
        ] {
            manager
                .alter_table(Table::alter().table(USERS).drop_column(column).to_owned())
                .await?;
        }
        Ok(())
    }
}
