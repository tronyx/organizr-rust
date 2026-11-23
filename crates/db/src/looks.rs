//! Storage of the layout and the theme that each person chose.

use organizr_domain::UserId;
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait, sea_query::OnConflict};

use crate::DbError;
use crate::entity::user_preference;

/// The layout and the theme of one person. A field that is `None` means the default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredLook {
    /// The layout.
    pub layout: Option<String>,
    /// The theme of the original layout.
    pub og_theme: Option<String>,
    /// The style of the original layout.
    pub og_style: Option<String>,
    /// The theme of the modern layout.
    pub modern_theme: Option<String>,
    /// The colour set of the modern layout.
    pub modern_colour_set: Option<String>,
}

impl From<user_preference::Model> for StoredLook {
    fn from(row: user_preference::Model) -> Self {
        Self {
            layout: row.layout,
            og_theme: row.og_theme,
            og_style: row.og_style,
            modern_theme: row.modern_theme,
            modern_colour_set: row.modern_colour_set,
        }
    }
}

/// Reads and writes the look of each person.
#[derive(Debug, Clone, Copy)]
pub struct Looks<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Looks<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// The look of a user, or `None` when the user has not chosen one.
    pub async fn get(&self, user: UserId) -> Result<Option<StoredLook>, DbError> {
        Ok(user_preference::Entity::find_by_id(user.get())
            .one(self.connection)
            .await?
            .map(StoredLook::from))
    }

    /// Stores the look of a user and replaces the one they had.
    ///
    /// Fails with [`DbError::ReferenceViolation`] when the user does not exist.
    pub async fn set(&self, user: UserId, look: &StoredLook) -> Result<(), DbError> {
        let row = user_preference::ActiveModel {
            user_id: ActiveValue::Set(user.get()),
            layout: ActiveValue::Set(look.layout.clone()),
            og_theme: ActiveValue::Set(look.og_theme.clone()),
            og_style: ActiveValue::Set(look.og_style.clone()),
            modern_theme: ActiveValue::Set(look.modern_theme.clone()),
            modern_colour_set: ActiveValue::Set(look.modern_colour_set.clone()),
        };
        user_preference::Entity::insert(row)
            .on_conflict(
                OnConflict::column(user_preference::Column::UserId)
                    .update_columns([
                        user_preference::Column::Layout,
                        user_preference::Column::OgTheme,
                        user_preference::Column::OgStyle,
                        user_preference::Column::ModernTheme,
                        user_preference::Column::ModernColourSet,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(self.connection)
            .await?;
        Ok(())
    }

    /// Deletes the look of a user, so they use the defaults again. Does nothing when
    /// there is none.
    pub async fn clear(&self, user: UserId) -> Result<(), DbError> {
        user_preference::Entity::delete_by_id(user.get())
            .exec(self.connection)
            .await?;
        Ok(())
    }
}
