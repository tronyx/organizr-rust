//! The import of another install: all its records, stored in one step.

use organizr_domain::{Bookmark, BookmarkCategory, Category, Group, Tab, User};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, TransactionTrait,
};

use crate::entity::{
    bookmark_category, bookmark_tab, category, group, legacy_setting, setting, tab, user,
};
use crate::stored_settings::StoredSetting;
use crate::users::UserLogin;
use crate::{DbError, Engine, stored_settings};

/// The tables whose records keep the id of the other install.
const TABLES_WITH_IDS: [&str; 6] = [
    "groups",
    "users",
    "categories",
    "tabs",
    "bookmark_categories",
    "bookmark_tabs",
];

/// A user of the other install, with what that install knows about the login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedUser {
    /// The account.
    pub user: User,
    /// How the user logs in.
    pub login: UserLogin,
}

/// Everything an import stores. Each record keeps its id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportData {
    /// The groups.
    pub groups: Vec<Group>,
    /// The users. The group of each one is in `groups`.
    pub users: Vec<ImportedUser>,
    /// The categories.
    pub categories: Vec<Category>,
    /// The tabs. The category of each one is in `categories`.
    pub tabs: Vec<Tab>,
    /// The categories of the bookmark plugin.
    pub bookmark_categories: Vec<BookmarkCategory>,
    /// The bookmarks of the bookmark plugin.
    pub bookmarks: Vec<Bookmark>,
    /// The settings, in their stored form, with secrets sealed.
    pub settings: Vec<StoredSetting>,
    /// The settings that this application does not know. Each value is sealed.
    pub legacy_settings: Vec<StoredSetting>,
}

/// Stores another install in an empty database.
#[derive(Debug, Clone, Copy)]
pub struct Import<'a> {
    connection: &'a DatabaseConnection,
    engine: Engine,
}

impl<'a> Import<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection, engine: Engine) -> Self {
        Self { connection, engine }
    }

    /// Whether the database is empty: no group, user, category, tab or setting.
    pub async fn is_possible(&self) -> Result<bool, DbError> {
        is_empty(self.connection).await
    }

    /// The settings of an imported install that this application does not know, sorted
    /// by name. Each value is sealed.
    pub async fn legacy_settings(&self) -> Result<Vec<StoredSetting>, DbError> {
        let mut rows: Vec<StoredSetting> = legacy_setting::Entity::find()
            .all(self.connection)
            .await?
            .into_iter()
            .map(|row| StoredSetting {
                key: row.name,
                value: row.value,
            })
            .collect();
        rows.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(rows)
    }

    /// Stores everything, or nothing. Fails with [`DbError::NotEmpty`] when the database
    /// holds records already.
    pub async fn run(&self, data: ImportData) -> Result<(), DbError> {
        let transaction = self.connection.begin().await?;
        if !is_empty(&transaction).await? {
            return Err(DbError::NotEmpty);
        }
        store(&transaction, data).await?;
        if self.engine == Engine::Postgres {
            // The records brought their ids. The next new record must get the id after
            // the highest one, so each counter moves there.
            for table in TABLES_WITH_IDS {
                transaction
                    .execute_unprepared(&format!(
                        "SELECT setval(pg_get_serial_sequence('{table}', 'id'), \
                         (SELECT MAX(id) FROM {table})) WHERE EXISTS (SELECT 1 FROM {table})"
                    ))
                    .await?;
            }
        }
        transaction.commit().await?;
        Ok(())
    }
}

async fn is_empty<C: ConnectionTrait>(connection: &C) -> Result<bool, DbError> {
    Ok(group::Entity::find().count(connection).await? == 0
        && user::Entity::find().count(connection).await? == 0
        && category::Entity::find().count(connection).await? == 0
        && tab::Entity::find().count(connection).await? == 0
        && setting::Entity::find().count(connection).await? == 0)
}

/// Stores the records of the bookmark plugin, with their ids.
async fn store_bookmarks<C: ConnectionTrait>(
    connection: &C,
    categories: Vec<BookmarkCategory>,
    bookmarks: Vec<Bookmark>,
) -> Result<(), DbError> {
    for category in categories {
        bookmark_category::ActiveModel {
            id: ActiveValue::Set(category.id),
            number: ActiveValue::Set(category.number),
            position: ActiveValue::Set(category.position),
            name_key: ActiveValue::Set(category.name.to_lowercase()),
            name: ActiveValue::Set(category.name),
            is_default: ActiveValue::Set(category.is_default),
        }
        .insert(connection)
        .await?;
    }

    for bookmark in bookmarks {
        bookmark_tab::ActiveModel {
            id: ActiveValue::Set(bookmark.id),
            position: ActiveValue::Set(bookmark.position),
            category_number: ActiveValue::Set(bookmark.category),
            name_key: ActiveValue::Set(bookmark.name.to_lowercase()),
            name: ActiveValue::Set(bookmark.name),
            url: ActiveValue::Set(bookmark.url),
            enabled: ActiveValue::Set(bookmark.enabled),
            least_access_level: ActiveValue::Set(bookmark.least_access),
            image: ActiveValue::Set(bookmark.image),
            background_color: ActiveValue::Set(bookmark.background_color),
            text_color: ActiveValue::Set(bookmark.text_color),
        }
        .insert(connection)
        .await?;
    }
    Ok(())
}

async fn store<C: ConnectionTrait>(connection: &C, data: ImportData) -> Result<(), DbError> {
    for group in data.groups {
        group::ActiveModel {
            id: ActiveValue::Set(group.id.get()),
            level: ActiveValue::Set(i32::from(group.level.get())),
            name_key: ActiveValue::Set(group.name.to_lowercase()),
            name: ActiveValue::Set(group.name),
            image: ActiveValue::Set(group.image),
            is_default: ActiveValue::Set(group.is_default),
        }
        .insert(connection)
        .await?;
    }

    for imported in data.users {
        let user = imported.user;
        let login = imported.login;
        let (two_factor_kind, two_factor_secret) = login
            .two_factor
            .map(|factor| (factor.kind, factor.sealed_secret))
            .unzip();
        user::ActiveModel {
            id: ActiveValue::Set(user.id.get()),
            username_key: ActiveValue::Set(user.username.to_lowercase()),
            username: ActiveValue::Set(user.username),
            email_key: ActiveValue::Set(user.email.as_deref().map(str::to_lowercase)),
            email: ActiveValue::Set(user.email),
            password_hash: ActiveValue::Set(user.password_hash.expose().to_owned()),
            group_id: ActiveValue::Set(user.group.get()),
            locked: ActiveValue::Set(user.locked),
            image: ActiveValue::Set(user.image),
            created_at: ActiveValue::Set(user.created_at.unix_seconds()),
            auth_service: ActiveValue::Set(login.auth_service),
            auth_provider: ActiveValue::Set(login.auth_provider),
            two_factor_kind: ActiveValue::Set(two_factor_kind),
            two_factor_secret: ActiveValue::Set(two_factor_secret),
            plex_token: ActiveValue::Set(login.sealed_plex_token),
            recovery_hash: ActiveValue::Set(None),
            recovery_expires_at: ActiveValue::Set(None),
        }
        .insert(connection)
        .await?;
    }

    for category in data.categories {
        category::ActiveModel {
            id: ActiveValue::Set(category.id.get()),
            number: ActiveValue::Set(category.number.get()),
            position: ActiveValue::Set(category.position),
            name_key: ActiveValue::Set(category.name.to_lowercase()),
            name: ActiveValue::Set(category.name),
            image: ActiveValue::Set(category.image),
            is_default: ActiveValue::Set(category.is_default),
        }
        .insert(connection)
        .await?;
    }

    for tab in data.tabs {
        tab::ActiveModel {
            id: ActiveValue::Set(tab.id.get()),
            position: ActiveValue::Set(tab.position),
            category_id: ActiveValue::Set(tab.category.get()),
            name_key: ActiveValue::Set(tab.name.to_lowercase()),
            name: ActiveValue::Set(tab.name),
            url: ActiveValue::Set(tab.url),
            url_local: ActiveValue::Set(tab.url_local),
            is_default: ActiveValue::Set(tab.is_default),
            enabled: ActiveValue::Set(tab.enabled),
            least_access_level: ActiveValue::Set(i32::from(tab.access.least_access().get())),
            most_access_level: ActiveValue::Set(i32::from(tab.access.most_access().get())),
            add_to_admin: ActiveValue::Set(tab.add_to_admin),
            image: ActiveValue::Set(tab.image),
            kind: ActiveValue::Set(i32::from(tab.kind.code())),
            splash: ActiveValue::Set(tab.splash),
            ping: ActiveValue::Set(tab.ping),
            ping_url: ActiveValue::Set(tab.ping_url),
            timeout: ActiveValue::Set(tab.timeout),
            timeout_ms: ActiveValue::Set(tab.timeout_ms),
            preload: ActiveValue::Set(tab.preload),
        }
        .insert(connection)
        .await?;
    }

    for setting in &data.settings {
        stored_settings::put_in(connection, &setting.key, &setting.value).await?;
    }
    store_bookmarks(connection, data.bookmark_categories, data.bookmarks).await?;

    for setting in data.legacy_settings {
        legacy_setting::ActiveModel {
            name: ActiveValue::Set(setting.key),
            value: ActiveValue::Set(setting.value),
        }
        .insert(connection)
        .await?;
    }
    Ok(())
}
