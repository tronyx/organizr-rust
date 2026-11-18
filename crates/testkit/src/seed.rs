//! Puts a database into a state the recorded cases start from.

use organizr_db::Database;
use organizr_domain::{
    CategoryId, CategoryNumber, GroupLevel, LevelRange, NewCategory, NewGroup, NewSession, NewTab,
    NewUser, PasswordHash, SessionTokenHash, TabKind, Timestamp,
};

/// The groups the set-up wizard of the PHP application creates: level, name, image, default.
const WIZARD_GROUPS: [(u16, &str, &str, bool); 6] = [
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

/// The tabs the set-up wizard creates: position, name, address, enabled, least access
/// level, image. Both are pages of Organizr itself.
const WIZARD_TABS: [(i64, &str, &str, bool, u16, &str); 2] = [
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

/// The id of the seeded install. The set-up wizard of the PHP application makes a random
/// one.
pub const INSTALL_ID: &str = "5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21";

/// The name of the admin account, as in `parity/recorder/record.py`.
pub const ADMIN_NAME: &str = "admin";

/// The e-mail address of the admin account, as in `parity/recorder/record.py`.
pub const ADMIN_EMAIL: &str = "admin@example.com";

/// The password of the admin account, as in `parity/recorder/record.py`. It protects
/// nothing.
pub const ADMIN_PASSWORD: &str = "parity-admin-password";

/// A hash of [`ADMIN_PASSWORD`] as the set-up wizard stores it. PHP 8.5 made it with
/// `password_hash($password, PASSWORD_BCRYPT)`.
pub const ADMIN_PASSWORD_HASH: &str =
    "$2y$12$WG7fEmudO2Nw2eBtDlehJOQDekAJhq89HWiaa2NQLdwG4hMT7aUNW";

/// The picture the set-up wizard gives the admin account: the Gravatar address for
/// [`ADMIN_EMAIL`].
pub const ADMIN_IMAGE: &str =
    "https://www.gravatar.com/avatar/e64c7d89f26bd1972efa854d13d7dd61?s=100&d=mm";

/// Stores the session that the set-up wizard of the PHP application leaves behind: it
/// logs the new admin in, for one day, from a request without a `User-Agent` header.
///
/// No one knows the token of this session. It is there so that the sessions of the
/// recorded cases get the ids they had in the recording. Call it after [`installed`].
pub async fn wizard_session(database: &Database) {
    let admin = database
        .users()
        .find_by_username(ADMIN_NAME)
        .await
        .expect("the lookup works")
        .expect("the admin account is stored");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is past 1970");
    let now = i64::try_from(now.as_secs()).expect("the time fits");
    database
        .sessions()
        .insert(NewSession {
            token_hash: SessionTokenHash::new("00".repeat(32)),
            user: admin.id,
            user_agent: None,
            ip: std::net::IpAddr::from([127, 0, 0, 1]),
            created_at: Timestamp::from_unix_seconds(now),
            expires_at: Timestamp::from_unix_seconds(now + 86_400),
        })
        .await
        .expect("the session of the wizard can be stored");
}

/// Makes the database look as it does after the recorder has run the set-up wizard: the
/// six standard groups, the admin account of `parity/recorder/record.py`, the category
/// "Unsorted" and the tabs "Settings" and "Homepage".
///
/// The session that the wizard leaves behind is not included; see [`wizard_session`].
pub async fn installed(database: &Database) {
    let mut admin_group = None;
    for (level, name, image, is_default) in WIZARD_GROUPS {
        let group = database
            .groups()
            .insert(NewGroup {
                level: GroupLevel::new(level).expect("the wizard levels are valid"),
                name: name.to_owned(),
                image: Some(image.to_owned()),
                is_default,
            })
            .await
            .expect("a wizard group can be stored");
        if group.level == GroupLevel::ADMIN {
            admin_group = Some(group.id);
        }
    }

    database
        .users()
        .insert(NewUser {
            username: ADMIN_NAME.to_owned(),
            email: Some(ADMIN_EMAIL.to_owned()),
            password_hash: PasswordHash::new(ADMIN_PASSWORD_HASH.to_owned()),
            group: admin_group.expect("the wizard creates the Admin group"),
            image: Some(ADMIN_IMAGE.to_owned()),
            created_at: Timestamp::from_unix_seconds(1_700_000_000),
        })
        .await
        .expect("the admin account can be stored");

    let unsorted = database
        .categories()
        .insert(NewCategory {
            number: CategoryNumber::new(0),
            position: 1,
            name: "Unsorted".to_owned(),
            image: Some("fontawesome::question".to_owned()),
            is_default: true,
        })
        .await
        .expect("the wizard category can be stored");

    for (position, name, url, enabled, least_access, image) in WIZARD_TABS {
        database
            .tabs()
            .insert(wizard_tab(
                unsorted.id,
                position,
                name,
                url,
                enabled,
                least_access,
                image,
            ))
            .await
            .expect("a wizard tab can be stored");
    }
}

/// A tab as the set-up wizard creates it.
fn wizard_tab(
    category: CategoryId,
    position: i64,
    name: &str,
    url: &str,
    enabled: bool,
    least_access: u16,
    image: &str,
) -> NewTab {
    let least_access = GroupLevel::new(least_access).expect("the wizard levels are valid");
    NewTab {
        position,
        category,
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
    }
}
