//! Puts a database into a state the recorded cases start from.

use organizr_db::Database;
use organizr_domain::{GroupLevel, NewGroup, NewUser, PasswordHash, Timestamp};

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

/// Makes the database look as it does after the recorder has run the set-up wizard: the
/// six standard groups and the admin account of `parity/recorder/record.py`.
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
}
