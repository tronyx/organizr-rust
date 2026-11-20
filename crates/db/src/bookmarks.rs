//! Storage of the bookmarks and their categories.

use organizr_domain::{
    Bookmark, BookmarkCategory, BookmarkCategoryChanges, BookmarkChanges, NewBookmark,
    NewBookmarkCategory,
};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};

use crate::DbError;
use crate::entity::{bookmark_category as category, bookmark_tab as tab};

/// Reads and writes bookmarks and their categories.
#[derive(Debug, Clone, Copy)]
pub struct Bookmarks<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Bookmarks<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// All categories, in the order of the page.
    pub async fn categories(&self) -> Result<Vec<BookmarkCategory>, DbError> {
        let rows = category::Entity::find()
            .order_by_asc(category::Column::Position)
            .order_by_asc(category::Column::Id)
            .all(self.connection)
            .await?;
        Ok(rows.into_iter().map(BookmarkCategory::from).collect())
    }

    /// All bookmarks, in the order of the page.
    pub async fn list(&self) -> Result<Vec<Bookmark>, DbError> {
        let rows = tab::Entity::find()
            .order_by_asc(tab::Column::Position)
            .order_by_asc(tab::Column::Id)
            .all(self.connection)
            .await?;
        Ok(rows.into_iter().map(Bookmark::from).collect())
    }

    /// The bookmark with the given id.
    pub async fn find(&self, id: i64) -> Result<Option<Bookmark>, DbError> {
        let row = tab::Entity::find_by_id(id).one(self.connection).await?;
        Ok(row.map(Bookmark::from))
    }

    /// The category with the given id.
    pub async fn find_category(&self, id: i64) -> Result<Option<BookmarkCategory>, DbError> {
        let row = category::Entity::find_by_id(id)
            .one(self.connection)
            .await?;
        Ok(row.map(BookmarkCategory::from))
    }

    /// Whether a bookmark has the name, whatever its letter case. The bookmark
    /// `except` does not count.
    pub async fn is_name_taken(&self, name: &str, except: Option<i64>) -> Result<bool, DbError> {
        let mut query = tab::Entity::find().filter(tab::Column::NameKey.eq(name.to_lowercase()));
        if let Some(id) = except {
            query = query.filter(tab::Column::Id.ne(id));
        }
        Ok(query.one(self.connection).await?.is_some())
    }

    /// Whether a category has the name, whatever its letter case. The category
    /// `except` does not count.
    pub async fn is_category_name_taken(
        &self,
        name: &str,
        except: Option<i64>,
    ) -> Result<bool, DbError> {
        let mut query =
            category::Entity::find().filter(category::Column::NameKey.eq(name.to_lowercase()));
        if let Some(id) = except {
            query = query.filter(category::Column::Id.ne(id));
        }
        Ok(query.one(self.connection).await?.is_some())
    }

    /// Whether a category has the number. The category `except` does not count.
    pub async fn is_category_number_taken(
        &self,
        number: i64,
        except: Option<i64>,
    ) -> Result<bool, DbError> {
        let mut query = category::Entity::find().filter(category::Column::Number.eq(number));
        if let Some(id) = except {
            query = query.filter(category::Column::Id.ne(id));
        }
        Ok(query.one(self.connection).await?.is_some())
    }

    /// The place for a new bookmark: one after the highest place in use, or 1.
    pub async fn next_position(&self) -> Result<i64, DbError> {
        let highest: Option<Option<i64>> = tab::Entity::find()
            .select_only()
            .column_as(tab::Column::Position.max(), "highest")
            .into_tuple()
            .one(self.connection)
            .await?;
        Ok(highest.flatten().map_or(1, |value| value.saturating_add(1)))
    }

    async fn next_of_categories(&self, column: category::Column) -> Result<i64, DbError> {
        let highest: Option<Option<i64>> = category::Entity::find()
            .select_only()
            .column_as(column.max(), "highest")
            .into_tuple()
            .one(self.connection)
            .await?;
        Ok(highest.flatten().map_or(1, |value| value.saturating_add(1)))
    }

    /// The place for a new category: one after the highest place in use, or 1.
    pub async fn next_category_position(&self) -> Result<i64, DbError> {
        self.next_of_categories(category::Column::Position).await
    }

    /// The number for a new category: one after the highest number in use, or 1.
    pub async fn next_category_number(&self) -> Result<i64, DbError> {
        self.next_of_categories(category::Column::Number).await
    }

    /// The number of the category that new bookmarks join, when there is a category.
    pub async fn default_category(&self) -> Result<Option<i64>, DbError> {
        let row = category::Entity::find()
            .filter(category::Column::IsDefault.eq(true))
            .order_by_asc(category::Column::Number)
            .one(self.connection)
            .await?;
        Ok(row.map(|row| row.number))
    }

    /// Stores a new bookmark.
    ///
    /// Fails with [`DbError::Duplicate`] when the name is taken, whatever its letter
    /// case.
    pub async fn insert(&self, new: NewBookmark) -> Result<Bookmark, DbError> {
        insert_in(self.connection, new).await
    }

    /// Changes a bookmark and returns it as it is afterwards, or `None` when there is
    /// no bookmark with the id.
    pub async fn update(
        &self,
        id: i64,
        changes: BookmarkChanges,
    ) -> Result<Option<Bookmark>, DbError> {
        let Some(current) = tab::Entity::find_by_id(id).one(self.connection).await? else {
            return Ok(None);
        };
        let mut row: tab::ActiveModel = current.clone().into();
        if let Some(position) = changes.position {
            row.position = ActiveValue::Set(position);
        }
        if let Some(number) = changes.category {
            row.category_number = ActiveValue::Set(number);
        }
        if let Some(name) = changes.name {
            row.name_key = ActiveValue::Set(name.to_lowercase());
            row.name = ActiveValue::Set(name);
        }
        if let Some(url) = changes.url {
            row.url = ActiveValue::Set(url);
        }
        if let Some(enabled) = changes.enabled {
            row.enabled = ActiveValue::Set(enabled);
        }
        if let Some(level) = changes.least_access {
            row.least_access_level = ActiveValue::Set(level);
        }
        if let Some(image) = changes.image {
            row.image = ActiveValue::Set(image);
        }
        if let Some(color) = changes.background_color {
            row.background_color = ActiveValue::Set(color);
        }
        if let Some(color) = changes.text_color {
            row.text_color = ActiveValue::Set(color);
        }
        let bookmark = if row.is_changed() {
            row.update(self.connection).await?
        } else {
            current
        };
        Ok(Some(bookmark.into()))
    }

    /// Deletes a bookmark. Returns whether the bookmark existed.
    pub async fn delete(&self, id: i64) -> Result<bool, DbError> {
        let result = tab::Entity::delete_by_id(id).exec(self.connection).await?;
        Ok(result.rows_affected > 0)
    }

    /// Gives bookmarks new places. Each entry is the id of a bookmark and its new
    /// place. An id without a bookmark changes nothing. All places are set, or none.
    pub async fn set_positions(&self, places: &[(i64, i64)]) -> Result<(), DbError> {
        let transaction = self.connection.begin().await?;
        for (id, position) in places {
            tab::Entity::update_many()
                .col_expr(tab::Column::Position, Expr::value(*position))
                .filter(tab::Column::Id.eq(*id))
                .exec(&transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Stores a new category. A new category that is the default takes that mark from
    /// every other category, and a first category is the default. All happens, or
    /// nothing does.
    ///
    /// Fails with [`DbError::Duplicate`] when the number or the name is taken.
    pub async fn insert_category(
        &self,
        new: NewBookmarkCategory,
    ) -> Result<BookmarkCategory, DbError> {
        let transaction = self.connection.begin().await?;
        if new.is_default {
            clear_default(&transaction).await?;
        }
        let stored = insert_category_in(&transaction, new).await?;
        ensure_default(&transaction).await?;
        let stored = category::Entity::find_by_id(stored.id)
            .one(&transaction)
            .await?
            .map_or(stored, BookmarkCategory::from);
        transaction.commit().await?;
        Ok(stored)
    }

    /// Changes a category and returns it as it is afterwards, or `None` when there is
    /// no category with the id. A change that makes the category the default takes
    /// that mark from every other category. There is always one default category.
    pub async fn update_category(
        &self,
        id: i64,
        changes: BookmarkCategoryChanges,
    ) -> Result<Option<BookmarkCategory>, DbError> {
        let transaction = self.connection.begin().await?;
        let Some(current) = category::Entity::find_by_id(id).one(&transaction).await? else {
            return Ok(None);
        };
        if changes.is_default == Some(true) {
            clear_default(&transaction).await?;
        }
        let mut row: category::ActiveModel = current.clone().into();
        if let Some(number) = changes.number {
            row.number = ActiveValue::Set(number);
        }
        if let Some(position) = changes.position {
            row.position = ActiveValue::Set(position);
        }
        if let Some(name) = changes.name {
            row.name_key = ActiveValue::Set(name.to_lowercase());
            row.name = ActiveValue::Set(name);
        }
        if let Some(is_default) = changes.is_default {
            row.is_default = ActiveValue::Set(is_default);
        }
        if row.is_changed() {
            row.update(&transaction).await?;
        }
        ensure_default(&transaction).await?;
        let stored = category::Entity::find_by_id(id).one(&transaction).await?;
        transaction.commit().await?;
        Ok(stored.map(BookmarkCategory::from))
    }

    /// Deletes a category. Returns whether the category existed. Its bookmarks stay.
    /// When the default category goes, the category with the lowest number is the
    /// default.
    pub async fn delete_category(&self, id: i64) -> Result<bool, DbError> {
        let transaction = self.connection.begin().await?;
        let result = category::Entity::delete_by_id(id)
            .exec(&transaction)
            .await?;
        ensure_default(&transaction).await?;
        transaction.commit().await?;
        Ok(result.rows_affected > 0)
    }

    /// Gives categories new places. An id without a category changes nothing. All
    /// places are set, or none is.
    pub async fn set_category_positions(&self, places: &[(i64, i64)]) -> Result<(), DbError> {
        let transaction = self.connection.begin().await?;
        for (id, position) in places {
            category::Entity::update_many()
                .col_expr(category::Column::Position, Expr::value(*position))
                .filter(category::Column::Id.eq(*id))
                .exec(&transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
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

/// Marks the category with the lowest number as the default when no category has the
/// mark.
async fn ensure_default<C: ConnectionTrait>(connection: &C) -> Result<(), DbError> {
    let has_default = category::Entity::find()
        .filter(category::Column::IsDefault.eq(true))
        .one(connection)
        .await?
        .is_some();
    if has_default {
        return Ok(());
    }
    let first = category::Entity::find()
        .order_by_asc(category::Column::Number)
        .one(connection)
        .await?;
    if let Some(first) = first {
        category::Entity::update_many()
            .col_expr(category::Column::IsDefault, Expr::value(true))
            .filter(category::Column::Id.eq(first.id))
            .exec(connection)
            .await?;
    }
    Ok(())
}

/// Stores a new bookmark through any connection: the pool, or a transaction.
pub(crate) async fn insert_in<C: ConnectionTrait>(
    connection: &C,
    new: NewBookmark,
) -> Result<Bookmark, DbError> {
    let row = tab::ActiveModel {
        id: ActiveValue::NotSet,
        position: ActiveValue::Set(new.position),
        category_number: ActiveValue::Set(new.category),
        name_key: ActiveValue::Set(new.name.to_lowercase()),
        name: ActiveValue::Set(new.name),
        url: ActiveValue::Set(new.url),
        enabled: ActiveValue::Set(new.enabled),
        least_access_level: ActiveValue::Set(new.least_access),
        image: ActiveValue::Set(new.image),
        background_color: ActiveValue::Set(new.background_color),
        text_color: ActiveValue::Set(new.text_color),
    };
    Ok(row.insert(connection).await?.into())
}

/// Stores a new category through any connection: the pool, or a transaction.
pub(crate) async fn insert_category_in<C: ConnectionTrait>(
    connection: &C,
    new: NewBookmarkCategory,
) -> Result<BookmarkCategory, DbError> {
    let row = category::ActiveModel {
        id: ActiveValue::NotSet,
        number: ActiveValue::Set(new.number),
        position: ActiveValue::Set(new.position),
        name_key: ActiveValue::Set(new.name.to_lowercase()),
        name: ActiveValue::Set(new.name),
        is_default: ActiveValue::Set(new.is_default),
    };
    Ok(row.insert(connection).await?.into())
}

impl From<category::Model> for BookmarkCategory {
    fn from(row: category::Model) -> Self {
        Self {
            id: row.id,
            number: row.number,
            position: row.position,
            name: row.name,
            is_default: row.is_default,
        }
    }
}

impl From<tab::Model> for Bookmark {
    fn from(row: tab::Model) -> Self {
        Self {
            id: row.id,
            position: row.position,
            category: row.category_number,
            name: row.name,
            url: row.url,
            enabled: row.enabled,
            least_access: row.least_access_level,
            image: row.image,
            background_color: row.background_color,
            text_color: row.text_color,
        }
    }
}
