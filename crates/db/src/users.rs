//! Storage of users.

use std::collections::HashMap;

use organizr_domain::{
    Change, GroupId, NewUser, PasswordHash, Timestamp, User, UserChanges, UserId,
};
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

    /// Whether another user than `except` has the text as name or as e-mail address,
    /// whatever the letter case. A user cannot take such a text as name or as address,
    /// because a person logs in with either.
    pub async fn is_taken_by_another(&self, text: &str, except: UserId) -> Result<bool, DbError> {
        let key = text.to_lowercase();
        let found = user::Entity::find()
            .filter(user::Column::Id.ne(except.get()))
            .filter(
                Condition::any()
                    .add(user::Column::UsernameKey.eq(key.as_str()))
                    .add(user::Column::EmailKey.eq(key.as_str())),
            )
            .count(self.connection)
            .await?;
        Ok(found > 0)
    }

    /// Changes a user and returns the user as it is afterwards, or `None` when there is
    /// no user with the id.
    ///
    /// Fails with [`DbError::Duplicate`] when the new name is taken, whatever its letter
    /// case, and with [`DbError::ReferenceViolation`] when the new group does not exist.
    pub async fn update(&self, id: UserId, changes: UserChanges) -> Result<Option<User>, DbError> {
        let Some(current) = user::Entity::find_by_id(id.get())
            .one(self.connection)
            .await?
        else {
            return Ok(None);
        };
        let mut row: user::ActiveModel = current.clone().into();
        if let Change::Set(username) = changes.username {
            row.username_key = ActiveValue::Set(username.to_lowercase());
            row.username = ActiveValue::Set(username);
        }
        if let Change::Set(email) = changes.email {
            row.email_key = ActiveValue::Set(Some(email.to_lowercase()));
            row.email = ActiveValue::Set(Some(email));
        }
        if let Change::Set(password_hash) = changes.password_hash {
            row.password_hash = ActiveValue::Set(password_hash.expose().to_owned());
        }
        if let Change::Set(group) = changes.group {
            row.group_id = ActiveValue::Set(group.get());
        }
        if let Change::Set(locked) = changes.locked {
            row.locked = ActiveValue::Set(locked);
        }
        if let Change::Set(image) = changes.image {
            row.image = ActiveValue::Set(image);
        }
        let user = if row.is_changed() {
            row.update(self.connection).await?
        } else {
            current
        };
        Ok(Some(user.into()))
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

    /// The first user, by id, with the given e-mail address, whatever the letter case.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, DbError> {
        let row = user::Entity::find()
            .filter(user::Column::EmailKey.eq(email.to_lowercase()))
            .order_by_asc(user::Column::Id)
            .one(self.connection)
            .await?;
        Ok(row.map(User::from))
    }

    /// The recovery password of a user: its hash, and until when it works. `None` when
    /// the user has none.
    pub async fn recovery_of(&self, id: UserId) -> Result<Option<Recovery>, DbError> {
        let row = user::Entity::find_by_id(id.get())
            .one(self.connection)
            .await?;
        Ok(row.and_then(|row| {
            Some(Recovery {
                password_hash: PasswordHash::new(row.recovery_hash?),
                expires_at: Timestamp::from_unix_seconds(row.recovery_expires_at?),
            })
        }))
    }

    /// Stores a recovery password for a user, in place of an older one. The password of
    /// the account stays. Returns whether the user exists.
    pub async fn set_recovery(&self, id: UserId, recovery: &Recovery) -> Result<bool, DbError> {
        let result = user::Entity::update_many()
            .col_expr(
                user::Column::RecoveryHash,
                Expr::value(Some(recovery.password_hash.expose().to_owned())),
            )
            .col_expr(
                user::Column::RecoveryExpiresAt,
                Expr::value(Some(recovery.expires_at.unix_seconds())),
            )
            .filter(user::Column::Id.eq(id.get()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Makes the recovery password of a user the password of the account. Returns
    /// whether the user had a recovery password.
    pub async fn use_recovery(&self, id: UserId) -> Result<bool, DbError> {
        let result = user::Entity::update_many()
            .col_expr(
                user::Column::PasswordHash,
                Expr::col(user::Column::RecoveryHash),
            )
            .col_expr(user::Column::RecoveryHash, Expr::value(None::<String>))
            .col_expr(user::Column::RecoveryExpiresAt, Expr::value(None::<i64>))
            .filter(user::Column::Id.eq(id.get()))
            .filter(user::Column::RecoveryHash.is_not_null())
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// How a user logs in. `None` when there is no such user.
    pub async fn login_of(&self, id: UserId) -> Result<Option<UserLogin>, DbError> {
        let row = user::Entity::find_by_id(id.get())
            .one(self.connection)
            .await?;
        Ok(row.map(UserLogin::from))
    }

    /// How each user logs in, by the id of the user.
    pub async fn logins(&self) -> Result<HashMap<UserId, UserLogin>, DbError> {
        Ok(user::Entity::find()
            .all(self.connection)
            .await?
            .into_iter()
            .map(|row| (UserId::new(row.id), UserLogin::from(row)))
            .collect())
    }

    /// Sets or removes the sealed Plex token of a user. Returns whether the user exists.
    pub async fn set_plex_token(
        &self,
        id: UserId,
        sealed_token: Option<String>,
    ) -> Result<bool, DbError> {
        let result = user::Entity::update_many()
            .col_expr(user::Column::PlexToken, Expr::value(sealed_token))
            .filter(user::Column::Id.eq(id.get()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Sets or removes the second factor of a user. Returns whether the user exists.
    pub async fn set_two_factor(
        &self,
        id: UserId,
        two_factor: Option<SealedTwoFactor>,
    ) -> Result<bool, DbError> {
        let (kind, secret) = two_factor
            .map(|factor| (factor.kind, factor.sealed_secret))
            .unzip();
        let result = user::Entity::update_many()
            .col_expr(user::Column::TwoFactorKind, Expr::value(kind))
            .col_expr(user::Column::TwoFactorSecret, Expr::value(secret))
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

/// A password from an e-mail to a person who forgot the password of the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovery {
    /// The hash of the password.
    pub password_hash: PasswordHash,
    /// Until when the password works.
    pub expires_at: Timestamp,
}

/// The second factor of a login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedTwoFactor {
    /// The kind, for example `google`.
    pub kind: String,
    /// The secret, sealed.
    pub sealed_secret: String,
}

/// How a user logs in, as the PHP application keeps it in `auth_service` and
/// `plex_token`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLogin {
    /// The service that checks the password: `internal` for this application.
    pub auth_service: String,
    /// The provider of an `oidc` login.
    pub auth_provider: Option<String>,
    /// The second factor, when the user has one.
    pub two_factor: Option<SealedTwoFactor>,
    /// The token of the Plex account, sealed.
    pub sealed_plex_token: Option<String>,
}

impl From<user::Model> for UserLogin {
    fn from(row: user::Model) -> Self {
        Self {
            auth_service: row.auth_service,
            auth_provider: row.auth_provider,
            two_factor: row.two_factor_kind.zip(row.two_factor_secret).map(
                |(kind, sealed_secret)| SealedTwoFactor {
                    kind,
                    sealed_secret,
                },
            ),
            sealed_plex_token: row.plex_token,
        }
    }
}

impl UserLogin {
    /// The text of the `auth_service` field of the PHP application for this login,
    /// without a secret: the kind of the second factor when there is one, else the
    /// service, with the provider of an `oidc` login behind `::`.
    pub fn auth_service_text(&self) -> String {
        match (&self.two_factor, &self.auth_provider) {
            (Some(factor), _) => factor.kind.clone(),
            (None, Some(provider)) => format!("{}::{provider}", self.auth_service),
            (None, None) => self.auth_service.clone(),
        }
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
        auth_service: ActiveValue::Set("internal".to_owned()),
        auth_provider: ActiveValue::Set(None),
        two_factor_kind: ActiveValue::Set(None),
        two_factor_secret: ActiveValue::Set(None),
        plex_token: ActiveValue::Set(None),
        recovery_hash: ActiveValue::Set(None),
        recovery_expires_at: ActiveValue::Set(None),
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
