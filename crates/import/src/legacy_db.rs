//! Reads the SQLite database of the PHP application.
//!
//! The file is opened for reading only. A database that grew over years does not look
//! like the one a new PHP install creates: upgrades appended columns, so their order
//! differs, and SQLite lets any column hold any kind of value. So the reader asks the
//! file which columns a table has, reads every value as text, and gives each row as a
//! map from the name of a column to its value.

use std::collections::BTreeMap;
use std::path::Path;

use sea_orm::{ConnectOptions, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use thiserror::Error;

/// Why the database of the PHP application cannot be read.
#[derive(Debug, Error)]
pub enum LegacyDbError {
    /// There is no file at the path.
    #[error("there is no database file at {0}")]
    Missing(String),
    /// SQLite reported an error.
    #[error("the database of the PHP application cannot be read: {0}")]
    Engine(#[from] DbErr),
}

/// One row of a table: the value of each column as text. `NULL` is no value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyRow {
    values: BTreeMap<String, Option<String>>,
}

impl LegacyRow {
    /// A row with the given values. A test uses it.
    pub fn of<'a>(values: impl IntoIterator<Item = (&'a str, Option<&'a str>)>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(column, value)| (column.to_owned(), value.map(str::to_owned)))
                .collect(),
        }
    }

    /// The value of a column. `None` when the value is `NULL` or the table has no such
    /// column.
    pub fn text(&self, column: &str) -> Option<&str> {
        self.values.get(column)?.as_deref()
    }

    /// The value of a column, when it is more than white space.
    pub fn filled(&self, column: &str) -> Option<&str> {
        self.text(column).filter(|text| !text.trim().is_empty())
    }

    /// The value of a column as a whole number.
    pub fn number(&self, column: &str) -> Option<i64> {
        self.text(column)?.trim().parse().ok()
    }

    /// Whether a switch is on. PHP stores 1 for on, and 0 or nothing for off.
    pub fn is_on(&self, column: &str) -> bool {
        self.number(column) == Some(1)
    }
}

/// The database of a PHP install, open for reading.
#[derive(Debug)]
pub struct LegacyDatabase {
    connection: DatabaseConnection,
}

impl LegacyDatabase {
    /// Opens the database file for reading only.
    pub async fn open(file: &Path) -> Result<Self, LegacyDbError> {
        if !file.is_file() {
            return Err(LegacyDbError::Missing(file.display().to_string()));
        }
        let url = format!("sqlite:{}", file.display().to_string().replace('\\', "/"));
        let mut options = ConnectOptions::new(url);
        options.sqlx_logging(false).max_connections(1);
        options.map_sqlx_sqlite_opts(|sqlite| sqlite.read_only(true).create_if_missing(false));
        let connection = sea_orm::Database::connect(options).await?;
        Ok(Self { connection })
    }

    /// The names of the tables, sorted. The tables of SQLite itself are left out.
    pub async fn tables(&self) -> Result<Vec<String>, LegacyDbError> {
        let rows = self
            .query(
                "SELECT name FROM sqlite_master WHERE type = 'table' \
                 AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .await?;
        let mut names = Vec::new();
        for row in rows {
            names.push(row.try_get_by::<String, _>("name")?);
        }
        Ok(names)
    }

    /// The number of rows of a table.
    pub async fn count(&self, table: &str) -> Result<u64, LegacyDbError> {
        let rows = self
            .query(&format!("SELECT COUNT(*) AS rows FROM {}", quoted(table)))
            .await?;
        let count = match rows.first() {
            Some(row) => row.try_get_by::<i64, _>("rows")?,
            None => 0,
        };
        Ok(u64::try_from(count).unwrap_or_default())
    }

    /// The rows of a table, in the order in which they were stored. A table that the
    /// file does not have gives no rows.
    pub async fn rows(&self, table: &str) -> Result<Vec<LegacyRow>, LegacyDbError> {
        let mut columns = Vec::new();
        for row in self
            .query(&format!("PRAGMA table_info({})", quoted(table)))
            .await?
        {
            columns.push(row.try_get_by::<String, _>("name")?);
        }
        if columns.is_empty() {
            return Ok(Vec::new());
        }

        // Every value as text: a column of numbers can hold text, and the other way round.
        let selected: Vec<String> = columns
            .iter()
            .zip(0_usize..)
            .map(|(column, index)| format!("CAST({} AS TEXT) AS c{index}", quoted(column)))
            .collect();
        let sql = format!(
            "SELECT {} FROM {} ORDER BY rowid",
            selected.join(", "),
            quoted(table)
        );
        let mut rows = Vec::new();
        for row in self.query(&sql).await? {
            let mut values = BTreeMap::new();
            for (column, index) in columns.iter().zip(0_usize..) {
                let value = row.try_get_by::<Option<String>, _>(format!("c{index}").as_str())?;
                values.insert(column.clone(), value);
            }
            rows.push(LegacyRow { values });
        }
        Ok(rows)
    }

    /// Closes the file.
    pub async fn close(self) -> Result<(), LegacyDbError> {
        Ok(self.connection.close().await?)
    }

    async fn query(&self, sql: &str) -> Result<Vec<sea_orm::QueryResult>, DbErr> {
        self.connection
            .query_all_raw(Statement::from_string(DbBackend::Sqlite, sql.to_owned()))
            .await
    }
}

/// A name as SQLite takes it inside a statement.
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
