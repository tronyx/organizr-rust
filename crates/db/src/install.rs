//! The set-up of a new install: everything the set-up wizard stores, in one step.

use organizr_domain::{
    CategoryNumber, GroupLevel, LevelRange, NewCategory, NewGroup, NewTab, NewUser, PasswordHash,
    TabKind, Timestamp, User,
};
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, TransactionTrait};

use crate::entity::user;
use crate::stored_settings::StoredSetting;
use crate::{DbError, categories, groups, stored_settings, tabs, users};

/// The groups of a new install: level, name, picture, and whether new users join it.
const GROUPS: [(u16, &str, &str, bool); 6] = [
    (0, "Admin", "plugins/images/groups/admin.png", false),
    (1, "Co-Admin", "plugins/images/groups/coadmin.png", false),
    (
        2,
        "Super User",
        "plugins/images/groups/superuser.png",
        false,
    ),
    (
        3,
        "Power User",
        "plugins/images/groups/poweruser.png",
        false,
    ),
    (4, "User", "plugins/images/groups/user.png", true),
    (999, "Guest", "plugins/images/groups/guest.png", false),
];

/// The tabs of a new install: position, name, address, enabled, the level with the least
/// access that sees the tab, picture. Both are pages of Organizr itself.
const TABS: [(i64, &str, &str, bool, u16, &str); 2] = [
    (
        1,
        "Settings",
        "api/v2/page/settings",
        true,
        1,
        "fontawesome::cog",
    ),
    (
        2,
        "Homepage",
        "api/v2/page/homepage",
        false,
        4,
        "fontawesome::home",
    ),
];

/// What the person who sets an install up decides. The rest of a new install is the
/// same everywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewInstall {
    /// The name of the first account, which is an Admin.
    pub admin_username: String,
    /// The e-mail address of that account.
    pub admin_email: String,
    /// The hashed password of that account.
    pub admin_password_hash: PasswordHash,
    /// The picture of that account: a path or a URL.
    pub admin_image: Option<String>,
    /// The moment of the set-up.
    pub created_at: Timestamp,
    /// The settings that the set-up writes, in their stored form.
    pub settings: Vec<StoredSetting>,
}

/// Sets up a new install.
#[derive(Debug, Clone, Copy)]
pub struct Install<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> Install<'a> {
    pub(crate) const fn new(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Whether the install has been set up: it has a user.
    pub async fn is_done(&self) -> Result<bool, DbError> {
        Ok(user::Entity::find().count(self.connection).await? > 0)
    }

    /// Stores what a new install starts with: the six groups, the first Admin account,
    /// the category "Unsorted", the tabs "Settings" and "Homepage", and the settings of
    /// `new`. Returns the Admin account.
    ///
    /// Everything is stored, or nothing is. Fails with [`DbError::AlreadySetUp`] when the
    /// install has a user, or when another set-up got there first.
    pub async fn run(&self, new: NewInstall) -> Result<User, DbError> {
        let transaction = self.connection.begin().await?;
        if user::Entity::find().count(&transaction).await? > 0 {
            return Err(DbError::AlreadySetUp);
        }

        let admin = match store(&transaction, new).await {
            Ok(admin) => admin,
            // A record of a new install is there already: two set-ups ran at once.
            Err(DbError::Duplicate) => return Err(DbError::AlreadySetUp),
            Err(error) => return Err(error),
        };
        transaction.commit().await?;
        Ok(admin)
    }
}

async fn store<C: sea_orm::ConnectionTrait>(
    connection: &C,
    new: NewInstall,
) -> Result<User, DbError> {
    let invalid = |record: &'static str, reason: String| DbError::InvalidRecord { record, reason };

    let mut admin_group = None;
    for (level, name, image, is_default) in GROUPS {
        let level = GroupLevel::new(level).map_err(|error| invalid("group", error.to_string()))?;
        let group = groups::insert_in(
            connection,
            NewGroup {
                level,
                name: name.to_owned(),
                image: Some(image.to_owned()),
                is_default,
            },
        )
        .await?;
        if group.level == GroupLevel::ADMIN {
            admin_group = Some(group.id);
        }
    }
    let admin_group = admin_group
        .ok_or_else(|| invalid("group", "a new install has no Admin group".to_owned()))?;

    let admin = users::insert_in(
        connection,
        NewUser {
            username: new.admin_username,
            email: Some(new.admin_email),
            password_hash: new.admin_password_hash,
            group: admin_group,
            image: new.admin_image,
            created_at: new.created_at,
        },
    )
    .await?;

    let unsorted = categories::insert_in(
        connection,
        NewCategory {
            number: CategoryNumber::new(0),
            position: 1,
            name: "Unsorted".to_owned(),
            image: Some("fontawesome::question".to_owned()),
            is_default: true,
        },
    )
    .await?;

    for (position, name, url, enabled, least_access, image) in TABS {
        let least_access =
            GroupLevel::new(least_access).map_err(|error| invalid("tab", error.to_string()))?;
        tabs::insert_in(
            connection,
            NewTab {
                position,
                category: unsorted.id,
                name: name.to_owned(),
                url: Some(url.to_owned()),
                url_local: None,
                is_default: false,
                enabled,
                access: LevelRange::up_to(least_access),
                add_to_admin: false,
                image: Some(image.to_owned()),
                kind: TabKind::Internal,
                splash: false,
                ping: false,
                ping_url: None,
                timeout: false,
                timeout_ms: None,
                preload: false,
            },
        )
        .await?;
    }

    for setting in &new.settings {
        stored_settings::put_in(connection, &setting.key, &setting.value).await?;
    }

    Ok(admin)
}
