//! Puts a database into a state the recorded cases start from.

use organizr_auth::install::gravatar;
use organizr_db::{Database, NewInstall};
use organizr_domain::{NewSession, NewUser, PasswordHash, SessionTokenHash, Timestamp};

/// The set-up code of the applications that the test helpers build. A real server makes
/// a random one at each start.
pub const SETUP_CODE: &str = "PARK-TYPA-RKTY";

/// The id of the seeded install. The set-up wizard of the PHP application makes a random
/// one.
pub const INSTALL_ID: &str = "5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21";

/// The API key of the seeded install, as in `parity/recorder/record.py`. It protects
/// nothing.
pub const API_KEY: &str = "parityapikey01234567";

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
/// The password hash of the admin is one that PHP made, as an imported install has it.
/// No settings are stored; [`installed_app`](crate::installed_app) has the settings of
/// the install. The session that the wizard leaves behind is not included; see
/// [`wizard_session`].
pub async fn installed(database: &Database) {
    database
        .install()
        .run(NewInstall {
            admin_username: ADMIN_NAME.to_owned(),
            admin_email: ADMIN_EMAIL.to_owned(),
            admin_password_hash: PasswordHash::new(ADMIN_PASSWORD_HASH.to_owned()),
            admin_image: Some(ADMIN_IMAGE.to_owned()),
            created_at: Timestamp::from_unix_seconds(1_700_000_000),
            settings: Vec::new(),
        })
        .await
        .expect("the install can be set up");
}

/// The users that the recorded group `register` leaves behind, with the name and the
/// e-mail address of each: the ids 2 to 5, all in the group "User".
pub const REGISTERED_USERS: [(&str, &str); 4] = [
    (
        "yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy",
        "yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy@example.com",
    ),
    ("zoe", "zoe@example.com"),
    ("Yan", "Yan@Example.com"),
    ("form", "form@example.com"),
];

/// Stores [`REGISTERED_USERS`]. Call it after [`installed`], for the replay of a group
/// that the recorder runs after the group `register`.
pub async fn registered_users(database: &Database) {
    let group = database
        .groups()
        .find_default()
        .await
        .expect("the lookup works")
        .expect("the install has a default group");
    for (username, email) in REGISTERED_USERS {
        database
            .users()
            .insert(NewUser {
                username: username.to_owned(),
                email: Some(email.to_owned()),
                // Nobody logs in as one of these users.
                password_hash: PasswordHash::new(ADMIN_PASSWORD_HASH.to_owned()),
                group: group.id,
                image: Some(gravatar(email)),
                created_at: Timestamp::from_unix_seconds(1_700_000_100),
            })
            .await
            .expect("the user can be stored");
    }
}

/// The password of the users that [`logged_in`] stores.
pub const USER_PASSWORD: &str = "the password of the test users";

/// Stores a user in the group of the given level, logs the user in through the
/// application, and returns the session cookie as a browser sends it back.
pub async fn logged_in(database: &Database, app: &axum::Router, name: &str, level: u16) -> String {
    let level = organizr_domain::GroupLevel::new(level).expect("the level is valid");
    let group = database
        .groups()
        .find_by_level(level)
        .await
        .expect("the lookup works")
        .expect("the install has a group of the level");
    database
        .users()
        .insert(NewUser {
            username: name.to_owned(),
            email: None,
            password_hash: organizr_auth::password::hash(USER_PASSWORD)
                .expect("the password can be hashed"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored");
    let body = serde_json::json!({"username": name, "password": USER_PASSWORD});
    let answer = crate::http::call(app, crate::http::As::Guest, "POST", "/login", Some(body)).await;
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_"))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap_or_else(|| panic!("the login sets the session cookie: {}", answer.body))
        .to_owned()
}
