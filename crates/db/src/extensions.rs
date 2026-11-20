//! Storage of the extensions that an Admin approved.

use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::DbError;
use crate::entity::extension;

/// An extension that an Admin approved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedExtension {
    /// The id: the name of the folder of the extension.
    pub id: String,
    /// Whether the extension answers requests.
    pub enabled: bool,
    /// The SHA-256 of the file that was approved, as hex text.
    pub sha256: String,
    /// The permissions that were approved, with commas between them.
    pub permissions: String,
    /// The level of the group with the least access that can call the extension.
    pub least_access: i32,
    /// The values that the extension stored, as a JSON object of texts.
    pub values: String,
}

/// Reads and writes the approved extensions.
#[derive(Debug, Clone, Copy)]
pub struct Extensions<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Extensions<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Every approved extension, sorted by id.
    pub async fn list(&self) -> Result<Vec<ApprovedExtension>, DbError> {
        let rows = extension::Entity::find()
            .order_by_asc(extension::Column::Id)
            .all(self.connection)
            .await?;
        Ok(rows.into_iter().map(ApprovedExtension::from).collect())
    }

    /// The approved extension with the id.
    pub async fn find(&self, id: &str) -> Result<Option<ApprovedExtension>, DbError> {
        let row = extension::Entity::find_by_id(id.to_owned())
            .one(self.connection)
            .await?;
        Ok(row.map(ApprovedExtension::from))
    }

    /// Approves a file of an extension with its permissions, and switches the
    /// extension on. The values that the extension stored before stay.
    pub async fn approve(
        &self,
        id: &str,
        sha256: &str,
        permissions: &str,
        least_access: i32,
    ) -> Result<(), DbError> {
        let row = extension::ActiveModel {
            id: ActiveValue::Set(id.to_owned()),
            enabled: ActiveValue::Set(true),
            sha256: ActiveValue::Set(sha256.to_owned()),
            permissions: ActiveValue::Set(permissions.to_owned()),
            least_access_level: ActiveValue::Set(least_access),
            stored_values: ActiveValue::Set("{}".to_owned()),
        };
        extension::Entity::insert(row)
            .on_conflict(
                OnConflict::column(extension::Column::Id)
                    .update_columns([
                        extension::Column::Enabled,
                        extension::Column::Sha256,
                        extension::Column::Permissions,
                        extension::Column::LeastAccessLevel,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(self.connection)
            .await?;
        Ok(())
    }

    /// Switches an approved extension on or off. Returns whether it is approved.
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<bool, DbError> {
        let result = extension::Entity::update_many()
            .col_expr(extension::Column::Enabled, Expr::value(enabled))
            .filter(extension::Column::Id.eq(id))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Stores the values of an extension: a JSON object of texts.
    pub async fn set_values(&self, id: &str, values: &str) -> Result<(), DbError> {
        extension::Entity::update_many()
            .col_expr(extension::Column::StoredValues, Expr::value(values))
            .filter(extension::Column::Id.eq(id))
            .exec(self.connection)
            .await?;
        Ok(())
    }

    /// Forgets an extension with its approval and its values. Returns whether it was
    /// approved.
    pub async fn delete(&self, id: &str) -> Result<bool, DbError> {
        let result = extension::Entity::delete_many()
            .filter(extension::Column::Id.eq(id))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

impl From<extension::Model> for ApprovedExtension {
    fn from(row: extension::Model) -> Self {
        Self {
            id: row.id,
            enabled: row.enabled,
            sha256: row.sha256,
            permissions: row.permissions,
            least_access: row.least_access_level,
            values: row.stored_values,
        }
    }
}
