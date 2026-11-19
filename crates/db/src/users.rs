//! Storage of users.

use organizr_domain::{GroupId, NewUser, PasswordHash, Timestamp, User, UserId};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
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
        insert_in(self.connection, new).await
    }

    /// All users, in the order in which they were stored.
    pub async fn list(&self) -> Result<Vec<User>, DbError> {
        let rows = user::Entity::find()
            .order_by_asc(user::Column::Id)
            .all(self.connection)
            .await?;
        Ok(rows.into_iter().map(User::from).collect())
    }

    /// How many users there are.
    pub async fn count(&self) -> Result<u64, DbError> {
        Ok(user::Entity::find().count(self.connection).await?)
    }

    /// The user with the given id.
    pub async fn find(&self, id: UserId) -> Result<Option<User>, DbError> {
        let row = user::Entity::find_by_id(id.get())
            .one(self.connection)
            .await?;
        Ok(row.map(User::from))
    }

    /// The user with the given name, whatever its letter case.
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, DbError> {
        let row = user::Entity::find()
            .filter(user::Column::UsernameKey.eq(username.to_lowercase()))
            .one(self.connection)
            .await?;
        Ok(row.map(User::from))
    }

    /// The user who logs in with the given text: the first user, by id, whose name or
    /// whose e-mail address is that text, whatever the letter case.
    pub async fn find_by_login(&self, name_or_email: &str) -> Result<Option<User>, DbError> {
        let key = name_or_email.to_lowercase();
        let row = user::Entity::find()
            .filter(
                Condition::any()
                    .add(user::Column::UsernameKey.eq(key.as_str()))
                    .add(user::Column::EmailKey.eq(key.as_str())),
            )
            .order_by_asc(user::Column::Id)
            .one(self.connection)
            .await?;
        Ok(row.map(User::from))
    }

    /// Whether a user has the given name, or the given e-mail address, whatever the
    /// letter case. A new account can have neither.
    pub async fn is_taken(&self, username: &str, email: &str) -> Result<bool, DbError> {
        let found = user::Entity::find()
            .filter(
                Condition::any()
                    .add(user::Column::UsernameKey.eq(username.to_lowercase()))
                    .add(user::Column::EmailKey.eq(email.to_lowercase())),
            )
            .count(self.connection)
            .await?;
        Ok(found > 0)
    }

    /// Replaces the stored password hash of a user. Returns whether the user existed.
    pub async fn set_password_hash(
        &self,
        id: UserId,
        password_hash: &PasswordHash,
    ) -> Result<bool, DbError> {
        let result = user::Entity::update_many()
            .col_expr(
                user::Column::PasswordHash,
                Expr::value(password_hash.expose()),
            )
            .filter(user::Column::Id.eq(id.get()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Deletes a user and the sessions of that user. Returns whether the user existed.
    pub async fn delete(&self, id: UserId) -> Result<bool, DbError> {
        let result = user::Entity::delete_by_id(id.get())
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

/// Stores a new record through any connection: the pool, or a transaction.
pub(crate) async fn insert_in<C: ConnectionTrait>(
    connection: &C,
    new: NewUser,
) -> Result<User, DbError> {
    let row = user::ActiveModel {
        id: ActiveValue::NotSet,
        username_key: ActiveValue::Set(new.username.to_lowercase()),
        username: ActiveValue::Set(new.username),
        email_key: ActiveValue::Set(new.email.as_deref().map(str::to_lowercase)),
        email: ActiveValue::Set(new.email),
        password_hash: ActiveValue::Set(new.password_hash.expose().to_owned()),
        group_id: ActiveValue::Set(new.group.get()),
        locked: ActiveValue::Set(false),
        image: ActiveValue::Set(new.image),
        created_at: ActiveValue::Set(new.created_at.unix_seconds()),
    };
    Ok(row.insert(connection).await?.into())
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
