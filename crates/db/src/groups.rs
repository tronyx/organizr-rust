//! Storage of groups.

use organizr_domain::{Change, Group, GroupChanges, GroupId, GroupLevel, NewGroup};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};

use crate::DbError;
use crate::entity::{group, user};

/// Reads and writes groups.
#[derive(Debug, Clone, Copy)]
pub struct Groups<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Groups<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Stores a new group. A new group that is the default takes that mark from every
    /// other group. Both happen, or nothing does.
    ///
    /// Fails with [`DbError::Duplicate`] when the level is taken, or when the name is
    /// taken, whatever its letter case.
    pub async fn insert(&self, new: NewGroup) -> Result<Group, DbError> {
        let transaction = self.connection.begin().await?;
        if new.is_default {
            clear_default(&transaction).await?;
        }
        let group = insert_in(&transaction, new).await?;
        transaction.commit().await?;
        Ok(group)
    }

    /// Changes a group and returns it as it is afterwards, or `None` when there is no
    /// group with the id. A change that makes the group the default takes that mark
    /// from every other group. Both happen, or nothing does.
    ///
    /// Fails with [`DbError::Duplicate`] when the new name is taken, whatever its letter
    /// case.
    pub async fn update(
        &self,
        id: GroupId,
        changes: GroupChanges,
    ) -> Result<Option<Group>, DbError> {
        let transaction = self.connection.begin().await?;
        let Some(current) = group::Entity::find_by_id(id.get())
            .one(&transaction)
            .await?
        else {
            return Ok(None);
        };
        if changes.is_default == Change::Set(true) {
            clear_default(&transaction).await?;
        }
        let mut row: group::ActiveModel = current.clone().into();
        if let Change::Set(name) = changes.name {
            row.name_key = ActiveValue::Set(name.to_lowercase());
            row.name = ActiveValue::Set(name);
        }
        if let Change::Set(image) = changes.image {
            row.image = ActiveValue::Set(Some(image));
        }
        if let Change::Set(is_default) = changes.is_default {
            row.is_default = ActiveValue::Set(is_default);
        }
        let group = if row.is_changed() {
            row.update(&transaction).await?
        } else {
            current
        };
        transaction.commit().await?;
        group.try_into().map(Some)
    }

    /// Whether a group has the name, whatever its letter case. The group `except` does
    /// not count: a group can keep its own name.
    pub async fn is_name_taken(
        &self,
        name: &str,
        except: Option<GroupId>,
    ) -> Result<bool, DbError> {
        let mut query =
            group::Entity::find().filter(group::Column::NameKey.eq(name.to_lowercase()));
        if let Some(id) = except {
            query = query.filter(group::Column::Id.ne(id.get()));
        }
        Ok(query.one(self.connection).await?.is_some())
    }

    /// The highest level in use that is not the level of Guest, or `None` when no such
    /// group exists.
    pub async fn highest_level_below_guest(&self) -> Result<Option<GroupLevel>, DbError> {
        let highest: Option<Option<i32>> = group::Entity::find()
            .select_only()
            .column_as(group::Column::Level.max(), "highest")
            .filter(group::Column::Level.ne(i32::from(GroupLevel::GUEST.get())))
            .into_tuple()
            .one(self.connection)
            .await?;
        highest
            .flatten()
            .map(|level| {
                GroupLevel::try_from(i64::from(level)).map_err(|error| DbError::InvalidRecord {
                    record: "group",
                    reason: error.to_string(),
                })
            })
            .transpose()
    }

    /// How many users are in the group.
    pub async fn user_count(&self, id: GroupId) -> Result<u64, DbError> {
        Ok(user::Entity::find()
            .filter(user::Column::GroupId.eq(id.get()))
            .count(self.connection)
            .await?)
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

    /// The group that new users join. There is at most one, because storing a default
    /// group takes the mark from the others.
    pub async fn find_default(&self) -> Result<Option<Group>, DbError> {
        group::Entity::find()
            .filter(group::Column::IsDefault.eq(true))
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

/// Takes the "default" mark from every group.
async fn clear_default<C: ConnectionTrait>(connection: &C) -> Result<(), DbError> {
    group::Entity::update_many()
        .col_expr(group::Column::IsDefault, Expr::value(false))
        .filter(group::Column::IsDefault.eq(true))
        .exec(connection)
        .await?;
    Ok(())
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
