//! Gives the stored color set its American name: the column `modern_colour_set` of
//! `user_preferences` becomes `modern_color_set`, and the setting `modernColourSet` becomes
//! `modernColorSet`. Each value is kept.
//!
//! When a database somehow holds both settings, the new one is kept and the old one is
//! dropped.
//!
//! The steps run in one transaction on both engines, also on `SQLite`, where
//! `sea-orm-migration` adds none by default: a failed step leaves the old names and the
//! migration pending, never a column renamed without its setting.

use sea_orm_migration::prelude::*;

const USER_PREFERENCES: &str = "user_preferences";
const OLD_COLUMN: &str = "modern_colour_set";
const NEW_COLUMN: &str = "modern_color_set";
const OLD_SETTING: &str = "modernColourSet";
const NEW_SETTING: &str = "modernColorSet";

/// Renames the setting `from` to `to`. A row named `to` wins over a row named `from`.
async fn rename_setting(manager: &SchemaManager<'_>, from: &str, to: &str) -> Result<(), DbErr> {
    let connection = manager.get_connection();
    connection
        .execute_unprepared(&format!(
            "DELETE FROM settings WHERE name = '{from}' \
             AND EXISTS (SELECT 1 FROM settings WHERE name = '{to}')"
        ))
        .await?;
    connection
        .execute_unprepared(&format!(
            "UPDATE settings SET name = '{to}' WHERE name = '{from}'"
        ))
        .await?;
    Ok(())
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    fn use_transaction(&self) -> Option<bool> {
        Some(true)
    }

    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(USER_PREFERENCES)
                    .rename_column(OLD_COLUMN, NEW_COLUMN)
                    .to_owned(),
            )
            .await?;
        rename_setting(manager, OLD_SETTING, NEW_SETTING).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_setting(manager, NEW_SETTING, OLD_SETTING).await?;
        manager
            .alter_table(
                Table::alter()
                    .table(USER_PREFERENCES)
                    .rename_column(NEW_COLUMN, OLD_COLUMN)
                    .to_owned(),
            )
            .await
    }
}
