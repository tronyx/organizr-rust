//! Puts a database into a state the recorded cases start from.

use organizr_db::{Database, NewInstall};
use organizr_domain::{NewSession, PasswordHash, SessionTokenHash, Timestamp};

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
