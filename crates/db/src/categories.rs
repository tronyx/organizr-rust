//! Storage of categories.

use organizr_domain::{Category, CategoryChanges, CategoryId, CategoryNumber, Change, NewCategory};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};

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

    /// Stores a new category. A new category that is the default takes that mark from
    /// every other category. Both happen, or nothing does.
    ///
    /// Fails with [`DbError::Duplicate`] when the number is taken, or when the name is
    /// taken, whatever its letter case.
    pub async fn insert(&self, new: NewCategory) -> Result<Category, DbError> {
        let transaction = self.connection.begin().await?;
        if new.is_default {
            clear_default(&transaction).await?;
        }
        let category = insert_in(&transaction, new).await?;
        transaction.commit().await?;
        Ok(category)
    }

    /// The category with the given id.
    pub async fn find(&self, id: CategoryId) -> Result<Option<Category>, DbError> {
        let row = category::Entity::find_by_id(id.get())
            .one(self.connection)
            .await?;
        Ok(row.map(Category::from))
    }

    /// Changes a category and returns it as it is afterwards, or `None` when there is
    /// no category with the id. A change that makes the category the default takes that
    /// mark from every other category. Both happen, or nothing does.
    ///
    /// Fails with [`DbError::Duplicate`] when the new number is taken, or when the new
    /// name is taken, whatever its letter case.
    pub async fn update(
        &self,
        id: CategoryId,
        changes: CategoryChanges,
    ) -> Result<Option<Category>, DbError> {
        let transaction = self.connection.begin().await?;
        let Some(current) = category::Entity::find_by_id(id.get())
            .one(&transaction)
            .await?
        else {
            return Ok(None);
        };
        if changes.is_default == Change::Set(true) {
            clear_default(&transaction).await?;
        }
        let mut row: category::ActiveModel = current.clone().into();
        if let Change::Set(number) = changes.number {
            row.number = ActiveValue::Set(number.get());
        }
        if let Change::Set(position) = changes.position {
            row.position = ActiveValue::Set(position);
        }
        if let Change::Set(name) = changes.name {
            row.name_key = ActiveValue::Set(name.to_lowercase());
            row.name = ActiveValue::Set(name);
        }
        if let Change::Set(image) = changes.image {
            row.image = ActiveValue::Set(image);
        }
        if let Change::Set(is_default) = changes.is_default {
            row.is_default = ActiveValue::Set(is_default);
        }
        let category = if row.is_changed() {
            row.update(&transaction).await?
        } else {
            current
        };
        transaction.commit().await?;
        Ok(Some(category.into()))
    }

    /// Whether a category has the name, whatever its letter case. The category `except`
    /// does not count: a category can keep its own name.
    pub async fn is_name_taken(
        &self,
        name: &str,
        except: Option<CategoryId>,
    ) -> Result<bool, DbError> {
        let mut query =
            category::Entity::find().filter(category::Column::NameKey.eq(name.to_lowercase()));
        if let Some(id) = except {
            query = query.filter(category::Column::Id.ne(id.get()));
        }
        Ok(query.one(self.connection).await?.is_some())
    }

    /// Whether a category has the number. The category `except` does not count.
    pub async fn is_number_taken(
        &self,
        number: CategoryNumber,
        except: Option<CategoryId>,
    ) -> Result<bool, DbError> {
        let mut query = category::Entity::find().filter(category::Column::Number.eq(number.get()));
        if let Some(id) = except {
            query = query.filter(category::Column::Id.ne(id.get()));
        }
        Ok(query.one(self.connection).await?.is_some())
    }

    /// The place for a new category at the end of the menu: one after the highest place
    /// in use, or 1 when there is no category.
    pub async fn next_position(&self) -> Result<i64, DbError> {
        self.next_after(category::Column::Position).await
    }

    /// The number for a new category: one after the highest number in use, or 1 when
    /// there is no category.
    pub async fn next_number(&self) -> Result<CategoryNumber, DbError> {
        self.next_after(category::Column::Number)
            .await
            .map(CategoryNumber::new)
    }

    async fn next_after(&self, column: category::Column) -> Result<i64, DbError> {
        let highest: Option<Option<i64>> = category::Entity::find()
            .select_only()
            .column_as(column.max(), "highest")
            .into_tuple()
            .one(self.connection)
            .await?;
        Ok(highest.flatten().map_or(1, |value| value.saturating_add(1)))
    }

    /// Gives categories new places in the menu. Each entry is the id of a category and
    /// its new place. An id without a category changes nothing. All places are set, or
    /// none is.
    pub async fn set_positions(&self, places: &[(CategoryId, i64)]) -> Result<(), DbError> {
        let transaction = self.connection.begin().await?;
        for (id, position) in places {
            category::Entity::update_many()
                .col_expr(category::Column::Position, Expr::value(*position))
                .filter(category::Column::Id.eq(id.get()))
                .exec(&transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// The category with the given number, which the API calls `category_id`.
    pub async fn find_by_number(
        &self,
        number: CategoryNumber,
    ) -> Result<Option<Category>, DbError> {
        let row = category::Entity::find()
            .filter(category::Column::Number.eq(number.get()))
            .one(self.connection)
            .await?;
        Ok(row.map(Category::from))
    }

    /// The category that new tabs join. At most one category is marked as the default,
    /// because storing a default takes the mark from the others.
    pub async fn find_default(&self) -> Result<Option<Category>, DbError> {
        let row = category::Entity::find()
            .filter(category::Column::IsDefault.eq(true))
            .one(self.connection)
            .await?;
        Ok(row.map(Category::from))
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

/// Takes the "default" mark from every category.
async fn clear_default<C: ConnectionTrait>(connection: &C) -> Result<(), DbError> {
    category::Entity::update_many()
        .col_expr(category::Column::IsDefault, Expr::value(false))
        .filter(category::Column::IsDefault.eq(true))
        .exec(connection)
        .await?;
    Ok(())
}

/// Stores a new record through any connection: the pool, or a transaction.
pub(crate) async fn insert_in<C: ConnectionTrait>(
    connection: &C,
    new: NewCategory,
) -> Result<Category, DbError> {
    let row = category::ActiveModel {
        id: ActiveValue::NotSet,
        number: ActiveValue::Set(new.number.get()),
        position: ActiveValue::Set(new.position),
        name_key: ActiveValue::Set(new.name.to_lowercase()),
        name: ActiveValue::Set(new.name),
        image: ActiveValue::Set(new.image),
        is_default: ActiveValue::Set(new.is_default),
    };
    Ok(row.insert(connection).await?.into())
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
