//! Storage of settings.
//!
//! Only settings that are not at their default have a row. This module stores keys and
//! values as text; which keys exist and which values fit is decided by the settings
//! registry, before a value gets here.

use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait,
};

use crate::DbError;
use crate::entity::setting;

/// A setting as it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSetting {
    /// The key of the setting.
    pub key: String,
    /// The value as JSON text.
    pub value: String,
}

/// Reads and writes stored settings.
#[derive(Debug, Clone, Copy)]
pub struct StoredSettings<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> StoredSettings<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Every stored setting, ordered by key.
    ///
    /// The order is byte order. It is made here and not in the query, because the order
    /// of a query depends on the collation of the database, which an install can choose.
    pub async fn all(&self) -> Result<Vec<StoredSetting>, DbError> {
        let rows = setting::Entity::find().all(self.connection).await?;
        let mut settings: Vec<StoredSetting> = rows
            .into_iter()
            .map(|row| StoredSetting {
                key: row.name,
                value: row.value,
            })
            .collect();
        settings.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(settings)
    }

    /// Stores the value of a setting, in place of any value it had.
    pub async fn put(&self, key: &str, value: &str) -> Result<(), DbError> {
        put_in(self.connection, key, value).await
    }

    /// Removes the stored value of a setting, which puts the setting back to its default.
    /// Returns whether a value was stored.
    /// Stores the given settings and removes the settings with the given keys. All of it
    /// happens, or nothing does.
    pub async fn apply(&self, puts: &[StoredSetting], removes: &[String]) -> Result<(), DbError> {
        let transaction = self.connection.begin().await?;
        for setting in puts {
            put_in(&transaction, &setting.key, &setting.value).await?;
        }
        if !removes.is_empty() {
            setting::Entity::delete_many()
                .filter(setting::Column::Name.is_in(removes.iter().map(String::as_str)))
                .exec(&transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn remove(&self, key: &str) -> Result<bool, DbError> {
        let result = setting::Entity::delete_by_id(key.to_owned())
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

/// Stores the value of a setting through any connection: the pool, or a transaction.
pub(crate) async fn put_in<C: ConnectionTrait>(
    connection: &C,
    key: &str,
    value: &str,
) -> Result<(), DbError> {
    let row = setting::ActiveModel {
        name: ActiveValue::Set(key.to_owned()),
        value: ActiveValue::Set(value.to_owned()),
    };
    setting::Entity::insert(row)
        .on_conflict(
            OnConflict::column(setting::Column::Name)
                .update_column(setting::Column::Value)
                .to_owned(),
        )
        .exec_without_returning(connection)
        .await?;
    Ok(())
}
