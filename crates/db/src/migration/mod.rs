//! Schema migrations. They are part of the binary and run at start-up.
//!
//! Each migration is written once and runs on both engines. A released migration is
//! never changed; a schema change is a new migration.

use sea_orm_migration::prelude::*;

mod m20261118_000001_groups_and_users;
mod m20261118_000002_categories_and_tabs;
mod m20261118_000003_settings;

/// The list of all migrations, oldest first.
pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261118_000001_groups_and_users::Migration),
            Box::new(m20261118_000002_categories_and_tabs::Migration),
            Box::new(m20261118_000003_settings::Migration),
        ]
    }
}
