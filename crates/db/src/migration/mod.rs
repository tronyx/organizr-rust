//! Schema migrations. They are part of the binary and run at start-up.
//!
//! Each migration is written once and runs on both engines. A released migration is
//! never changed; a schema change is a new migration.

use sea_orm_migration::prelude::*;

mod m20261118_000001_groups_and_users;
mod m20261118_000002_categories_and_tabs;
mod m20261118_000003_settings;
mod m20261118_000004_sessions;
mod m20261118_000005_user_email_key;
mod m20261119_000006_user_logins_and_legacy_settings;
mod m20261119_000007_user_recovery;
mod m20261120_000008_bookmarks;
mod m20261120_000009_invites;
mod m20261120_000010_extensions;

/// The list of all migrations, oldest first.
pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261118_000001_groups_and_users::Migration),
            Box::new(m20261118_000002_categories_and_tabs::Migration),
            Box::new(m20261118_000003_settings::Migration),
            Box::new(m20261118_000004_sessions::Migration),
            Box::new(m20261118_000005_user_email_key::Migration),
            Box::new(m20261119_000006_user_logins_and_legacy_settings::Migration),
            Box::new(m20261119_000007_user_recovery::Migration),
            Box::new(m20261120_000008_bookmarks::Migration),
            Box::new(m20261120_000009_invites::Migration),
            Box::new(m20261120_000010_extensions::Migration),
        ]
    }
}
