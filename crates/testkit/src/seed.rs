//! Puts a database into a state the recorded cases start from.

use organizr_db::Database;
use organizr_domain::{
    CategoryId, CategoryNumber, GroupLevel, LevelRange, NewCategory, NewGroup, NewTab, NewUser,
    PasswordHash, TabKind, Timestamp,
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

/// Makes the database look as it does after the recorder has run the set-up wizard: the
/// six standard groups, the admin account of `parity/recorder/record.py`, the category
/// "Unsorted" and the tabs "Settings" and "Homepage".
///
/// The password hash is not a real hash. No recorded case logs in with it yet.
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
            username: "admin".to_owned(),
            email: Some("admin@example.com".to_owned()),
            password_hash: PasswordHash::new("not-a-real-hash".to_owned()),
            group: admin_group.expect("the wizard creates the Admin group"),
            image: None,
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
