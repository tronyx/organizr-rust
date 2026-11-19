//! Storage of tabs.

use organizr_domain::{
    CategoryId, Change, GroupLevel, LevelRange, NewTab, Tab, TabChanges, TabId, TabKind,
};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};

use crate::DbError;
use crate::entity::tab;

/// Reads and writes tabs.
#[derive(Debug, Clone, Copy)]
pub struct Tabs<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Tabs<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Stores a new tab. A new tab that is the default takes that mark from every other
    /// tab. Both happen, or nothing does.
    ///
    /// Fails with [`DbError::Duplicate`] when the name is taken, whatever its letter case,
    /// and with [`DbError::ReferenceViolation`] when the category does not exist.
    pub async fn insert(&self, new: NewTab) -> Result<Tab, DbError> {
        let transaction = self.connection.begin().await?;
        if new.is_default {
            clear_default(&transaction).await?;
        }
        let tab = insert_in(&transaction, new).await?;
        transaction.commit().await?;
        Ok(tab)
    }

    /// Changes a tab and returns it as it is afterwards, or `None` when there is no tab
    /// with the id. A change that makes the tab the default takes that mark from every
    /// other tab. Both happen, or nothing does.
    ///
    /// Fails with [`DbError::Duplicate`] when the new name is taken, whatever its letter
    /// case, and with [`DbError::ReferenceViolation`] when the new category does not
    /// exist.
    pub async fn update(&self, id: TabId, changes: TabChanges) -> Result<Option<Tab>, DbError> {
        let transaction = self.connection.begin().await?;
        let Some(current) = tab::Entity::find_by_id(id.get()).one(&transaction).await? else {
            return Ok(None);
        };
        if changes.is_default == Change::Set(true) {
            clear_default(&transaction).await?;
        }
        let row = changed_row(current.clone(), changes);
        let tab = if row.is_changed() {
            row.update(&transaction).await?
        } else {
            current
        };
        transaction.commit().await?;
        tab.try_into().map(Some)
    }

    /// Whether a tab has the name, whatever its letter case. The tab `except` does not
    /// count: a tab can keep its own name.
    pub async fn is_name_taken(&self, name: &str, except: Option<TabId>) -> Result<bool, DbError> {
        let mut query = tab::Entity::find().filter(tab::Column::NameKey.eq(name.to_lowercase()));
        if let Some(id) = except {
            query = query.filter(tab::Column::Id.ne(id.get()));
        }
        Ok(query.one(self.connection).await?.is_some())
    }

    /// The place for a new tab at the end of the menu: one after the highest place in
    /// use, or 1 when there is no tab.
    pub async fn next_position(&self) -> Result<i64, DbError> {
        let highest: Option<Option<i64>> = tab::Entity::find()
            .select_only()
            .column_as(tab::Column::Position.max(), "highest")
            .into_tuple()
            .one(self.connection)
            .await?;
        Ok(highest.flatten().map_or(1, |place| place.saturating_add(1)))
    }

    /// Gives tabs new places in the menu. Each entry is the id of a tab and its new
    /// place. An id without a tab changes nothing. All places are set, or none is.
    pub async fn set_positions(&self, places: &[(TabId, i64)]) -> Result<(), DbError> {
        let transaction = self.connection.begin().await?;
        for (id, position) in places {
            tab::Entity::update_many()
                .col_expr(tab::Column::Position, Expr::value(*position))
                .filter(tab::Column::Id.eq(id.get()))
                .exec(&transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// The tab with the given id.
    pub async fn find(&self, id: TabId) -> Result<Option<Tab>, DbError> {
        tab::Entity::find_by_id(id.get())
            .one(self.connection)
            .await?
            .map(Tab::try_from)
            .transpose()
    }

    /// The tab with the given name, whatever the letter case.
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Tab>, DbError> {
        tab::Entity::find()
            .filter(tab::Column::NameKey.eq(name.to_lowercase()))
            .one(self.connection)
            .await?
            .map(Tab::try_from)
            .transpose()
    }

    /// All tabs in menu order.
    pub async fn list(&self) -> Result<Vec<Tab>, DbError> {
        tab::Entity::find()
            .order_by_asc(tab::Column::Position)
            .order_by_asc(tab::Column::Id)
            .all(self.connection)
            .await?
            .into_iter()
            .map(Tab::try_from)
            .collect()
    }

    /// Deletes a tab. Returns whether the tab existed.
    pub async fn delete(&self, id: TabId) -> Result<bool, DbError> {
        let result = tab::Entity::delete_by_id(id.get())
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

/// Takes the "default" mark from every tab.
async fn clear_default<C: ConnectionTrait>(connection: &C) -> Result<(), DbError> {
    tab::Entity::update_many()
        .col_expr(tab::Column::IsDefault, Expr::value(false))
        .filter(tab::Column::IsDefault.eq(true))
        .exec(connection)
        .await?;
    Ok(())
}

/// A stored row with the changes applied to it, ready to be written.
fn changed_row(current: tab::Model, changes: TabChanges) -> tab::ActiveModel {
    let mut row: tab::ActiveModel = current.into();
    if let Change::Set(position) = changes.position {
        row.position = ActiveValue::Set(position);
    }
    if let Change::Set(category) = changes.category {
        row.category_id = ActiveValue::Set(category.get());
    }
    if let Change::Set(name) = changes.name {
        row.name_key = ActiveValue::Set(name.to_lowercase());
        row.name = ActiveValue::Set(name);
    }
    if let Change::Set(url) = changes.url {
        row.url = ActiveValue::Set(url);
    }
    if let Change::Set(url_local) = changes.url_local {
        row.url_local = ActiveValue::Set(url_local);
    }
    if let Change::Set(is_default) = changes.is_default {
        row.is_default = ActiveValue::Set(is_default);
    }
    if let Change::Set(enabled) = changes.enabled {
        row.enabled = ActiveValue::Set(enabled);
    }
    if let Change::Set(access) = changes.access {
        row.least_access_level = ActiveValue::Set(i32::from(access.least_access().get()));
        row.most_access_level = ActiveValue::Set(i32::from(access.most_access().get()));
    }
    if let Change::Set(add_to_admin) = changes.add_to_admin {
        row.add_to_admin = ActiveValue::Set(add_to_admin);
    }
    if let Change::Set(image) = changes.image {
        row.image = ActiveValue::Set(image);
    }
    if let Change::Set(kind) = changes.kind {
        row.kind = ActiveValue::Set(i32::from(kind.code()));
    }
    if let Change::Set(splash) = changes.splash {
        row.splash = ActiveValue::Set(splash);
    }
    if let Change::Set(ping) = changes.ping {
        row.ping = ActiveValue::Set(ping);
    }
    if let Change::Set(ping_url) = changes.ping_url {
        row.ping_url = ActiveValue::Set(ping_url);
    }
    if let Change::Set(timeout) = changes.timeout {
        row.timeout = ActiveValue::Set(timeout);
    }
    if let Change::Set(timeout_ms) = changes.timeout_ms {
        row.timeout_ms = ActiveValue::Set(timeout_ms);
    }
    if let Change::Set(preload) = changes.preload {
        row.preload = ActiveValue::Set(preload);
    }
    row
}

/// Stores a new record through any connection: the pool, or a transaction.
pub(crate) async fn insert_in<C: ConnectionTrait>(
    connection: &C,
    new: NewTab,
) -> Result<Tab, DbError> {
    let row = tab::ActiveModel {
        id: ActiveValue::NotSet,
        position: ActiveValue::Set(new.position),
        category_id: ActiveValue::Set(new.category.get()),
        name_key: ActiveValue::Set(new.name.to_lowercase()),
        name: ActiveValue::Set(new.name),
        url: ActiveValue::Set(new.url),
        url_local: ActiveValue::Set(new.url_local),
        is_default: ActiveValue::Set(new.is_default),
        enabled: ActiveValue::Set(new.enabled),
        least_access_level: ActiveValue::Set(i32::from(new.access.least_access().get())),
        most_access_level: ActiveValue::Set(i32::from(new.access.most_access().get())),
        add_to_admin: ActiveValue::Set(new.add_to_admin),
        image: ActiveValue::Set(new.image),
        kind: ActiveValue::Set(i32::from(new.kind.code())),
        splash: ActiveValue::Set(new.splash),
        ping: ActiveValue::Set(new.ping),
        ping_url: ActiveValue::Set(new.ping_url),
        timeout: ActiveValue::Set(new.timeout),
        timeout_ms: ActiveValue::Set(new.timeout_ms),
        preload: ActiveValue::Set(new.preload),
    };
    row.insert(connection).await?.try_into()
}

impl TryFrom<tab::Model> for Tab {
    type Error = DbError;

    fn try_from(row: tab::Model) -> Result<Self, Self::Error> {
        let invalid = |reason: String| DbError::InvalidRecord {
            record: "tab",
            reason,
        };
        let least_access = GroupLevel::try_from(i64::from(row.least_access_level))
            .map_err(|error| invalid(error.to_string()))?;
        let most_access = GroupLevel::try_from(i64::from(row.most_access_level))
            .map_err(|error| invalid(error.to_string()))?;
        let access = LevelRange::new(least_access, most_access)
            .map_err(|error| invalid(error.to_string()))?;
        let kind =
            TabKind::try_from(i64::from(row.kind)).map_err(|error| invalid(error.to_string()))?;

        Ok(Self {
            id: TabId::new(row.id),
            position: row.position,
            category: CategoryId::new(row.category_id),
            name: row.name,
            url: row.url,
            url_local: row.url_local,
            is_default: row.is_default,
            enabled: row.enabled,
            access,
            add_to_admin: row.add_to_admin,
            image: row.image,
            kind,
            splash: row.splash,
            ping: row.ping,
            ping_url: row.ping_url,
            timeout: row.timeout,
            timeout_ms: row.timeout_ms,
            preload: row.preload,
        })
    }
}
