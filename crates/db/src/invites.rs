//! Storage of the invites.

use organizr_domain::{Invite, NewInvite};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder,
};

use crate::DbError;
use crate::entity::invite;

/// Reads and writes invites.
#[derive(Debug, Clone, Copy)]
pub struct Invites<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Invites<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// All invites, the oldest first. With `invited_by`, only the invites that the
    /// person with that user name made.
    pub async fn list(&self, invited_by: Option<&str>) -> Result<Vec<Invite>, DbError> {
        let mut query = invite::Entity::find().order_by_asc(invite::Column::Id);
        if let Some(name) = invited_by {
            query = query.filter(invite::Column::InvitedBy.eq(name));
        }
        let rows = query.all(self.connection).await?;
        Ok(rows.into_iter().map(Invite::from).collect())
    }

    /// The number of invites that the person with the user name made.
    pub async fn count_of(&self, invited_by: &str) -> Result<u64, DbError> {
        Ok(invite::Entity::find()
            .filter(invite::Column::InvitedBy.eq(invited_by))
            .count(self.connection)
            .await?)
    }

    /// The number of invites that the person with the user name made at or after the
    /// time, in seconds since 1970.
    pub async fn count_since(&self, invited_by: &str, since: i64) -> Result<u64, DbError> {
        Ok(invite::Entity::find()
            .filter(invite::Column::InvitedBy.eq(invited_by))
            .filter(invite::Column::CreatedAt.gte(since))
            .count(self.connection)
            .await?)
    }

    /// The invite with the code, whatever its letter case.
    pub async fn find(&self, code: &str) -> Result<Option<Invite>, DbError> {
        let row = invite::Entity::find()
            .filter(invite::Column::CodeKey.eq(code.to_lowercase()))
            .one(self.connection)
            .await?;
        Ok(row.map(Invite::from))
    }

    /// Stores a new invite, which can be used.
    ///
    /// Fails with [`DbError::Duplicate`] when the code is taken, whatever its letter
    /// case.
    pub async fn insert(&self, new: NewInvite) -> Result<Invite, DbError> {
        insert_in(
            self.connection,
            Invite {
                id: 0,
                code: new.code,
                created_at: new.created_at,
                email: new.email,
                username: new.username,
                used_at: None,
                used_by: None,
                used_from: None,
                is_valid: true,
                kind: new.kind,
                invited_by: new.invited_by,
            },
            false,
        )
        .await
    }

    /// Marks an invite as used, when it can still be used. Returns whether this call
    /// used it: of two calls at the same time, only one does.
    pub async fn mark_used(
        &self,
        code: &str,
        used_by: Option<&str>,
        used_from: &str,
        now: i64,
    ) -> Result<bool, DbError> {
        let result = invite::Entity::update_many()
            .col_expr(invite::Column::IsValid, Expr::value(false))
            .col_expr(invite::Column::UsedAt, Expr::value(now))
            .col_expr(
                invite::Column::UsedBy,
                Expr::value(used_by.map(str::to_owned)),
            )
            .col_expr(invite::Column::UsedFrom, Expr::value(used_from))
            .filter(invite::Column::CodeKey.eq(code.to_lowercase()))
            .filter(invite::Column::IsValid.eq(true))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Makes a used invite usable again.
    pub async fn restore(&self, code: &str) -> Result<(), DbError> {
        invite::Entity::update_many()
            .col_expr(invite::Column::IsValid, Expr::value(true))
            .col_expr(invite::Column::UsedAt, Expr::value(Option::<i64>::None))
            .col_expr(invite::Column::UsedBy, Expr::value(Option::<String>::None))
            .col_expr(
                invite::Column::UsedFrom,
                Expr::value(Option::<String>::None),
            )
            .filter(invite::Column::CodeKey.eq(code.to_lowercase()))
            .exec(self.connection)
            .await?;
        Ok(())
    }

    /// Deletes the invite with the code. Returns whether it existed.
    pub async fn delete(&self, code: &str) -> Result<bool, DbError> {
        let result = invite::Entity::delete_many()
            .filter(invite::Column::CodeKey.eq(code.to_lowercase()))
            .exec(self.connection)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

/// Stores an invite through any connection. With `keep_id`, the record keeps its id,
/// which an import needs.
pub(crate) async fn insert_in<C: ConnectionTrait>(
    connection: &C,
    invite: Invite,
    keep_id: bool,
) -> Result<Invite, DbError> {
    let row = invite::ActiveModel {
        id: if keep_id {
            ActiveValue::Set(invite.id)
        } else {
            ActiveValue::NotSet
        },
        code_key: ActiveValue::Set(invite.code.to_lowercase()),
        code: ActiveValue::Set(invite.code),
        created_at: ActiveValue::Set(invite.created_at),
        email: ActiveValue::Set(invite.email),
        username: ActiveValue::Set(invite.username),
        used_at: ActiveValue::Set(invite.used_at),
        used_by: ActiveValue::Set(invite.used_by),
        used_from: ActiveValue::Set(invite.used_from),
        is_valid: ActiveValue::Set(invite.is_valid),
        kind: ActiveValue::Set(invite.kind),
        invited_by: ActiveValue::Set(invite.invited_by),
    };
    Ok(row.insert(connection).await?.into())
}

impl From<invite::Model> for Invite {
    fn from(row: invite::Model) -> Self {
        Self {
            id: row.id,
            code: row.code,
            created_at: row.created_at,
            email: row.email,
            username: row.username,
            used_at: row.used_at,
            used_by: row.used_by,
            used_from: row.used_from,
            is_valid: row.is_valid,
            kind: row.kind,
            invited_by: row.invited_by,
        }
    }
}
