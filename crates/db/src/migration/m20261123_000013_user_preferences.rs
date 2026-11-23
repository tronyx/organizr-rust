//! Creates `user_preferences`: the layout and the theme that a person chose for
//! themselves. A column that is null means the person uses the default.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_integer, text_null};

const USER_PREFERENCES: &str = "user_preferences";
const USERS: &str = "users";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(USER_PREFERENCES)
                    .col(big_integer("user_id").primary_key())
                    .col(text_null("layout"))
                    .col(text_null("og_theme"))
                    .col(text_null("og_style"))
                    .col(text_null("modern_theme"))
                    .col(text_null("modern_colour_set"))
                    // The look of a person has no use without the person.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_preferences_user_id")
                            .from(USER_PREFERENCES, "user_id")
                            .to(USERS, "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(USER_PREFERENCES).to_owned())
            .await
    }
}
