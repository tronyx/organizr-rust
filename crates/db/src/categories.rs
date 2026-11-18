//! Storage of categories.

use organizr_domain::{Category, CategoryId, CategoryNumber, NewCategory};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, QueryOrder};

use crate::DbError;
use crate::entity::category;

/// Reads and writes categories.
#[derive(Debug, Clone, Copy)]
pub struct Categories<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Categories<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Stores a new category.
    ///
    /// Fails with [`DbError::Duplicate`] when the number is taken, or when the name is
    /// taken, whatever its letter case.
    pub async fn insert(&self, new: NewCategory) -> Result<Category, DbError> {
        let row = category::ActiveModel {
            id: ActiveValue::NotSet,
            number: ActiveValue::Set(new.number.get()),
            position: ActiveValue::Set(new.position),
            name_key: ActiveValue::Set(new.name.to_lowercase()),
            name: ActiveValue::Set(new.name),
            image: ActiveValue::Set(new.image),
            is_default: ActiveValue::Set(new.is_default),
        };
        Ok(row.insert(self.connection).await?.into())
    }

    /// All categories in menu order.
    pub async fn list(&self) -> Result<Vec<Category>, DbError> {
        let rows = category::Entity::find()
            .order_by_asc(category::Column::Position)
            .order_by_asc(category::Column::Id)
            .all(self.connection)
            .await?;
        Ok(rows.into_iter().map(Category::from).collect())
    }

    /// Deletes a category. Returns whether the category existed.
    ///
    /// Fails with [`DbError::ReferenceViolation`] when the category still has tabs.
    pub async fn delete(&self, id: CategoryId) -> Result<bool, DbError> {
        let result = category::Entity::delete_by_id(id.get())
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

impl From<category::Model> for Category {
    fn from(row: category::Model) -> Self {
        Self {
            id: CategoryId::new(row.id),
            number: CategoryNumber::new(row.number),
            position: row.position,
            name: row.name,
            image: row.image,
            is_default: row.is_default,
        }
    }
}
