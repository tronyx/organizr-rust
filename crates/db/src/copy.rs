//! The copy of an install's `SQLite` database into an empty `PostgreSQL` database.

use sea_orm::{
    ActiveModelTrait, ConnectionTrait, EntityTrait, IntoActiveModel, PaginatorTrait, Statement,
    TransactionTrait,
};

use crate::entity::{
    bookmark_category, bookmark_tab, category, chat_message, extension, group, invite,
    legacy_setting, session, setting, tab, user,
};
use crate::{Database, DbError, Engine};

/// The tables of the schema, in the order that the foreign keys need for a copy: a table
/// comes after the tables it points at.
pub const COPIED_TABLES: [&str; 12] = [
    "groups",
    "users",
    "categories",
    "tabs",
    "settings",
    "legacy_settings",
    "sessions",
    "bookmark_categories",
    "bookmark_tabs",
    "invites",
    "extensions",
    "chatroom",
];

/// The tables with an `id` that `PostgreSQL` counts up by itself.
const TABLES_WITH_SERIAL_ID: [&str; 9] = [
    "groups",
    "users",
    "categories",
    "tabs",
    "sessions",
    "bookmark_categories",
    "bookmark_tabs",
    "invites",
    "chatroom",
];

/// Rows per insert. A statement of `PostgreSQL` takes at most 65535 values.
const ROWS_PER_INSERT: usize = 500;

/// The number of rows of one table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableCount {
    /// The name of the table.
    pub table: &'static str,
    /// The number of rows in it.
    pub rows: u64,
}

async fn copy_table<E, S, C>(source: &S, target: &C) -> Result<(), DbError>
where
    E: EntityTrait,
    E::Model: IntoActiveModel<E::ActiveModel>,
    E::ActiveModel: ActiveModelTrait<Entity = E> + Send,
    S: ConnectionTrait,
    C: ConnectionTrait,
{
    let rows = E::find().all(source).await?;
    for chunk in rows.chunks(ROWS_PER_INSERT) {
        let models = chunk
            .iter()
            .cloned()
            .map(|row| row.into_active_model().reset_all());
        E::insert_many(models).exec(target).await?;
    }
    Ok(())
}

async fn count<E, C>(connection: &C) -> Result<u64, DbError>
where
    E: EntityTrait,
    E::Model: Send + Sync,
    C: ConnectionTrait,
{
    Ok(E::find().count(connection).await?)
}

async fn counts<C: ConnectionTrait>(connection: &C) -> Result<Vec<TableCount>, DbError> {
    let rows = [
        count::<group::Entity, C>(connection).await?,
        count::<user::Entity, C>(connection).await?,
        count::<category::Entity, C>(connection).await?,
        count::<tab::Entity, C>(connection).await?,
        count::<setting::Entity, C>(connection).await?,
        count::<legacy_setting::Entity, C>(connection).await?,
        count::<session::Entity, C>(connection).await?,
        count::<bookmark_category::Entity, C>(connection).await?,
        count::<bookmark_tab::Entity, C>(connection).await?,
        count::<invite::Entity, C>(connection).await?,
        count::<extension::Entity, C>(connection).await?,
        count::<chat_message::Entity, C>(connection).await?,
    ];
    Ok(COPIED_TABLES
        .into_iter()
        .zip(rows)
        .map(|(table, rows)| TableCount { table, rows })
        .collect())
}

impl Database {
    /// The number of rows in each of the [`COPIED_TABLES`], in that order.
    pub async fn row_counts(&self) -> Result<Vec<TableCount>, DbError> {
        counts(&self.connection).await
    }

    /// Whether any of the [`COPIED_TABLES`] exists and holds a row. A table that does not
    /// exist yet counts as empty. Runs no migration and creates nothing.
    pub async fn holds_records(&self) -> Result<bool, DbError> {
        let backend = self.connection.get_database_backend();
        for table in COPIED_TABLES {
            let exists = match self.engine {
                Engine::Sqlite => {
                    format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'")
                }
                Engine::Postgres => format!("SELECT 1 WHERE to_regclass('{table}') IS NOT NULL"),
            };
            if self
                .connection
                .query_one_raw(Statement::from_string(backend, exists))
                .await?
                .is_none()
            {
                continue;
            }
            let has_row = self
                .connection
                .query_one_raw(Statement::from_string(
                    backend,
                    format!("SELECT 1 FROM {table} LIMIT 1"),
                ))
                .await?;
            if has_row.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Copies every row of every table of this `SQLite` database into `target`, an empty
    /// `PostgreSQL` database. Each row keeps its id, and the id counters of `PostgreSQL`
    /// move on, so a new record gets the id after the highest one.
    ///
    /// A `target` that holds a record is refused first, with [`DbError::NotEmpty`], before
    /// anything changes it. Then the schema of `target` is brought up to date. All rows are
    /// read from one snapshot of this database and stored in one transaction, or none. Before
    /// the commit, the number of rows of each table is compared on both sides, and a
    /// difference rolls the transaction back. Returns the counts.
    pub async fn copy_into(&self, target: &Self) -> Result<Vec<TableCount>, DbError> {
        if self.engine != Engine::Sqlite || target.engine != Engine::Postgres {
            return Err(DbError::WrongEngines);
        }
        // A database that holds an install is refused before any migration touches it.
        if target.holds_records().await? {
            return Err(DbError::NotEmpty);
        }
        target.migrate().await?;

        let transaction = target.connection.begin().await?;
        if counts(&transaction)
            .await?
            .iter()
            .any(|entry| entry.rows > 0)
        {
            return Err(DbError::NotEmpty);
        }
        // One read transaction gives every table the same snapshot of the file, even when
        // something writes to it during the copy. On a read-only connection it writes nothing.
        let source = self.connection.begin().await?;
        copy_table::<group::Entity, _, _>(&source, &transaction).await?;
        copy_table::<user::Entity, _, _>(&source, &transaction).await?;
        copy_table::<category::Entity, _, _>(&source, &transaction).await?;
        copy_table::<tab::Entity, _, _>(&source, &transaction).await?;
        copy_table::<setting::Entity, _, _>(&source, &transaction).await?;
        copy_table::<legacy_setting::Entity, _, _>(&source, &transaction).await?;
        copy_table::<session::Entity, _, _>(&source, &transaction).await?;
        copy_table::<bookmark_category::Entity, _, _>(&source, &transaction).await?;
        copy_table::<bookmark_tab::Entity, _, _>(&source, &transaction).await?;
        copy_table::<invite::Entity, _, _>(&source, &transaction).await?;
        copy_table::<extension::Entity, _, _>(&source, &transaction).await?;
        copy_table::<chat_message::Entity, _, _>(&source, &transaction).await?;
        for table in TABLES_WITH_SERIAL_ID {
            transaction
                .execute_unprepared(&format!(
                    "SELECT setval(pg_get_serial_sequence('{table}', 'id'), \
                     (SELECT MAX(id) FROM {table})) WHERE EXISTS (SELECT 1 FROM {table})"
                ))
                .await?;
        }
        // The counts are compared before the commit, so a mismatch rolls the target back.
        let original = counts(&source).await?;
        let copied = counts(&transaction).await?;
        check_counts(&original, &copied)?;
        // The read of the SQLite file ends before the commit. If ending it fails, the
        // target is rolled back too and stays empty, so the copy can run again.
        source.rollback().await?;
        transaction.commit().await?;
        Ok(copied)
    }
}

/// Fails with [`DbError::CopyMismatch`] for the first table that has a different number of
/// rows in `original` and in `copied`.
fn check_counts(original: &[TableCount], copied: &[TableCount]) -> Result<(), DbError> {
    for (from, to) in original.iter().zip(copied) {
        if from.rows != to.rows {
            return Err(DbError::CopyMismatch {
                table: from.table,
                in_sqlite: from.rows,
                in_postgres: to.rows,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{TableCount, check_counts};
    use crate::DbError;

    fn counts(rows: [u64; 3]) -> Vec<TableCount> {
        ["groups", "users", "tabs"]
            .into_iter()
            .zip(rows)
            .map(|(table, rows)| TableCount { table, rows })
            .collect()
    }

    #[test]
    fn equal_counts_pass() {
        assert!(check_counts(&counts([1, 2, 3]), &counts([1, 2, 3])).is_ok());
    }

    #[test]
    fn the_first_differing_table_is_reported() {
        let result = check_counts(&counts([1, 2, 3]), &counts([1, 5, 4]));
        assert!(
            matches!(
                result,
                Err(DbError::CopyMismatch {
                    table: "users",
                    in_sqlite: 2,
                    in_postgres: 5
                })
            ),
            "{result:?}"
        );
    }
}
