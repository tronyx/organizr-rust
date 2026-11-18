//! Storage of tabs.

use organizr_domain::{CategoryId, GroupLevel, LevelRange, NewTab, Tab, TabId, TabKind};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, QueryOrder};

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

    /// Stores a new tab.
    ///
    /// Fails with [`DbError::Duplicate`] when the name is taken, whatever its letter case,
    /// and with [`DbError::ReferenceViolation`] when the category does not exist.
    pub async fn insert(&self, new: NewTab) -> Result<Tab, DbError> {
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
        row.insert(self.connection).await?.try_into()
    }

    /// The tab with the given id.
    pub async fn find(&self, id: TabId) -> Result<Option<Tab>, DbError> {
        tab::Entity::find_by_id(id.get())
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
