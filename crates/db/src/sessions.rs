//! Storage of sessions.

use std::net::IpAddr;

use organizr_domain::{NewSession, Session, SessionId, SessionTokenHash, Timestamp, UserId};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};

use crate::DbError;
use crate::entity::session;

/// Reads and writes sessions.
///
/// A session is valid at a moment when it expires after that moment. The caller says
/// what moment "now" is.
#[derive(Debug, Clone, Copy)]
pub struct Sessions<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Sessions<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Stores a new session.
    ///
    /// Fails with [`DbError::Duplicate`] when a session with the same token hash exists,
    /// and with [`DbError::ReferenceViolation`] when the user does not exist.
    pub async fn insert(&self, new: NewSession) -> Result<Session, DbError> {
        let row = session::ActiveModel {
            id: ActiveValue::NotSet,
            token_hash: ActiveValue::Set(new.token_hash.as_str().to_owned()),
            user_id: ActiveValue::Set(new.user.get()),
            user_agent: ActiveValue::Set(new.user_agent),
            ip: ActiveValue::Set(new.ip.to_string()),
            created_at: ActiveValue::Set(new.created_at.unix_seconds()),
            expires_at: ActiveValue::Set(new.expires_at.unix_seconds()),
        };
        row.insert(self.connection).await?.try_into()
    }

    /// The session with the given token hash, when it is still valid at `now`.
    pub async fn find_valid(
        &self,
        token_hash: &SessionTokenHash,
        now: Timestamp,
    ) -> Result<Option<Session>, DbError> {
        session::Entity::find()
            .filter(session::Column::TokenHash.eq(token_hash.as_str()))
            .filter(session::Column::ExpiresAt.gt(now.unix_seconds()))
            .one(self.connection)
            .await?
            .map(Session::try_from)
            .transpose()
    }

    /// The sessions of a user that are still valid at `now`, oldest first.
    pub async fn list_valid_of_user(
        &self,
        user: UserId,
        now: Timestamp,
    ) -> Result<Vec<Session>, DbError> {
        session::Entity::find()
            .filter(session::Column::UserId.eq(user.get()))
            .filter(session::Column::ExpiresAt.gt(now.unix_seconds()))
            .order_by_asc(session::Column::Id)
            .all(self.connection)
            .await?
            .into_iter()
            .map(Session::try_from)
            .collect()
    }

    /// Deletes the session with the given token hash. Returns whether it existed.
    pub async fn delete_by_token(&self, token_hash: &SessionTokenHash) -> Result<bool, DbError> {
        let result = session::Entity::delete_many()
            .filter(session::Column::TokenHash.eq(token_hash.as_str()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Deletes one session, but only when it belongs to the given user. Returns whether
    /// such a session existed.
    pub async fn delete_one_of_user(&self, user: UserId, id: SessionId) -> Result<bool, DbError> {
        let result = session::Entity::delete_many()
            .filter(session::Column::Id.eq(id.get()))
            .filter(session::Column::UserId.eq(user.get()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Deletes all sessions of a user. Returns how many there were.
    pub async fn delete_all_of_user(&self, user: UserId) -> Result<u64, DbError> {
        let result = session::Entity::delete_many()
            .filter(session::Column::UserId.eq(user.get()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected)
    }

    /// Deletes all sessions of a user but one, such as the session of a request that
    /// changed the password. Returns how many there were.
    pub async fn delete_all_of_user_except(
        &self,
        user: UserId,
        kept: SessionId,
    ) -> Result<u64, DbError> {
        let result = session::Entity::delete_many()
            .filter(session::Column::UserId.eq(user.get()))
            .filter(session::Column::Id.ne(kept.get()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected)
    }

    /// Deletes the sessions that are no longer valid at `now`. Returns how many there were.
    pub async fn delete_expired(&self, now: Timestamp) -> Result<u64, DbError> {
        let result = session::Entity::delete_many()
            .filter(session::Column::ExpiresAt.lte(now.unix_seconds()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected)
    }
}

impl TryFrom<session::Model> for Session {
    type Error = DbError;

    fn try_from(row: session::Model) -> Result<Self, Self::Error> {
        let ip: IpAddr = row.ip.parse().map_err(|_| DbError::InvalidRecord {
            record: "session",
            reason: format!("`{}` is not an IP address", row.ip),
        })?;

        Ok(Self {
            id: SessionId::new(row.id),
            token_hash: SessionTokenHash::new(row.token_hash),
            user: UserId::new(row.user_id),
            user_agent: row.user_agent,
            ip,
            created_at: Timestamp::from_unix_seconds(row.created_at),
            expires_at: Timestamp::from_unix_seconds(row.expires_at),
        })
    }
}

#[cfg(test)]
mod tests {
    use organizr_domain::Session;

    use crate::DbError;
    use crate::entity::session;

    fn row(ip: &str) -> session::Model {
        session::Model {
            id: 1,
            token_hash: "ab".repeat(32),
            user_id: 2,
            user_agent: None,
            ip: ip.to_owned(),
            created_at: 1_800_000_000,
            expires_at: 1_800_003_600,
        }
    }

    #[test]
    fn row_with_an_address_becomes_a_session() {
        for ip in ["192.0.2.7", "2001:db8::1", "::ffff:192.0.2.7"] {
            let session = Session::try_from(row(ip)).expect("the row is valid");

            assert_eq!(session.ip.to_string(), ip);
        }
    }

    #[test]
    fn row_with_a_text_that_is_no_address_is_refused() {
        for ip in ["", "unknown", "192.0.2.7:8080", "192.0.2.300"] {
            let result = Session::try_from(row(ip));

            assert!(
                matches!(
                    result,
                    Err(DbError::InvalidRecord {
                        record: "session",
                        ..
                    })
                ),
                "{ip:?}: {result:?}"
            );
        }
    }
}
