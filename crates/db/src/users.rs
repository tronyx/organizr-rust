//! Storage of users.

use organizr_domain::{GroupId, NewUser, PasswordHash, Timestamp, User, UserId};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::DbError;
use crate::entity::user;

/// Reads and writes users.
#[derive(Debug, Clone, Copy)]
pub struct Users<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Users<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Stores a new user. The account starts unlocked.
    ///
    /// Fails with [`DbError::Duplicate`] when the name is taken, whatever its letter case,
    /// and with [`DbError::ReferenceViolation`] when the group does not exist.
    pub async fn insert(&self, new: NewUser) -> Result<User, DbError> {
        let row = user::ActiveModel {
            id: ActiveValue::NotSet,
            username_key: ActiveValue::Set(new.username.to_lowercase()),
            username: ActiveValue::Set(new.username),
            email: ActiveValue::Set(new.email),
            password_hash: ActiveValue::Set(new.password_hash.expose().to_owned()),
            group_id: ActiveValue::Set(new.group.get()),
            locked: ActiveValue::Set(false),
            image: ActiveValue::Set(new.image),
            created_at: ActiveValue::Set(new.created_at.unix_seconds()),
        };
        Ok(row.insert(self.connection).await?.into())
    }

    /// The user with the given name, whatever its letter case.
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, DbError> {
        let row = user::Entity::find()
            .filter(user::Column::UsernameKey.eq(username.to_lowercase()))
            .one(self.connection)
            .await?;
        Ok(row.map(User::from))
    }
}

impl From<user::Model> for User {
    fn from(row: user::Model) -> Self {
        Self {
            id: UserId::new(row.id),
            username: row.username,
            email: row.email,
            password_hash: PasswordHash::new(row.password_hash),
            group: GroupId::new(row.group_id),
            locked: row.locked,
            image: row.image,
            created_at: Timestamp::from_unix_seconds(row.created_at),
        }
    }
}
