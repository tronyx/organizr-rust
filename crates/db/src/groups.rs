//! Storage of groups.

use organizr_domain::{Group, GroupId, GroupLevel, NewGroup};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder,
};

use crate::DbError;
use crate::entity::group;

/// Reads and writes groups.
#[derive(Debug, Clone, Copy)]
pub struct Groups<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Groups<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Stores a new group.
    ///
    /// Fails with [`DbError::Duplicate`] when the level is taken, or when the name is
    /// taken, whatever its letter case.
    pub async fn insert(&self, new: NewGroup) -> Result<Group, DbError> {
        insert_in(self.connection, new).await
    }

    /// The group with the given id.
    pub async fn find(&self, id: GroupId) -> Result<Option<Group>, DbError> {
        group::Entity::find_by_id(id.get())
            .one(self.connection)
            .await?
            .map(Group::try_from)
            .transpose()
    }

    /// The group with the given level.
    pub async fn find_by_level(&self, level: GroupLevel) -> Result<Option<Group>, DbError> {
        group::Entity::find()
            .filter(group::Column::Level.eq(i32::from(level.get())))
            .one(self.connection)
            .await?
            .map(Group::try_from)
            .transpose()
    }

    /// The group that new users join. When several groups are marked as the default,
    /// it is the one of them with the least access.
    pub async fn find_default(&self) -> Result<Option<Group>, DbError> {
        group::Entity::find()
            .filter(group::Column::IsDefault.eq(true))
            .order_by_desc(group::Column::Level)
            .one(self.connection)
            .await?
            .map(Group::try_from)
            .transpose()
    }

    /// All groups, from the most access to the least.
    pub async fn list(&self) -> Result<Vec<Group>, DbError> {
        group::Entity::find()
            .order_by_asc(group::Column::Level)
            .all(self.connection)
            .await?
            .into_iter()
            .map(Group::try_from)
            .collect()
    }

    /// Deletes a group. Returns whether the group existed.
    ///
    /// Fails with [`DbError::ReferenceViolation`] when the group still has users.
    pub async fn delete(&self, id: GroupId) -> Result<bool, DbError> {
        let result = group::Entity::delete_by_id(id.get())
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

/// Stores a new record through any connection: the pool, or a transaction.
pub(crate) async fn insert_in<C: ConnectionTrait>(
    connection: &C,
    new: NewGroup,
) -> Result<Group, DbError> {
    let row = group::ActiveModel {
        id: ActiveValue::NotSet,
        level: ActiveValue::Set(i32::from(new.level.get())),
        name_key: ActiveValue::Set(new.name.to_lowercase()),
        name: ActiveValue::Set(new.name),
        image: ActiveValue::Set(new.image),
        is_default: ActiveValue::Set(new.is_default),
    };
    row.insert(connection).await?.try_into()
}

impl TryFrom<group::Model> for Group {
    type Error = DbError;

    fn try_from(row: group::Model) -> Result<Self, Self::Error> {
        let level =
            GroupLevel::try_from(i64::from(row.level)).map_err(|error| DbError::InvalidRecord {
                record: "group",
                reason: error.to_string(),
            })?;
        Ok(Self {
            id: GroupId::new(row.id),
            level,
            name: row.name,
            image: row.image,
            is_default: row.is_default,
        })
    }
}
