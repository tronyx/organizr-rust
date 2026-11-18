//! The settings of the application: every key, its type and its default.
//!
//! This is the one place a setting is defined. A key that is not listed here does not
//! exist: it cannot be read, and an attempt to store it is refused.
//!
//! The keys and the defaults are those of `api/config/default.php` of Organizr 2.1.5000.
//! Keys are added here when the feature that uses them is built.

use serde_json::Value;
use thiserror::Error;

use crate::choices::{AuthType, UnsortedTabs};
use crate::codec::{Choice, Codec, Flag, Text, TextNumber};

/// Why a setting cannot be read or changed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SettingsError {
    /// The key is not a setting.
    #[error("`{0}` is not a setting")]
    UnknownKey(String),

    /// The value does not fit the setting.
    #[error("the value of `{key}` is not valid: {reason}")]
    InvalidValue {
        /// The key of the setting.
        key: &'static str,
        /// What is wrong with the value.
        reason: String,
    },

    /// A stored value is not JSON.
    #[error("the stored value of `{key}` is not JSON: {reason}")]
    InvalidJson {
        /// The key the value is stored under.
        key: String,
        /// What the JSON parser reported.
        reason: String,
    },
}

/// Defines the settings: one line per setting gives the field, the codec, the key of the
/// PHP application and the default.
macro_rules! settings {
    ($( $(#[$doc:meta])* $field:ident: $codec:ty = $key:literal, $default:expr; )*) => {
        /// All settings with their current values, typed.
        ///
        /// A field is read directly, for example `settings.remember_me_days`. [`Settings::get`]
        /// and [`Settings::set`] work with the key and the stored form instead, for the
        /// configuration routes of the API.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Settings {
            $( $(#[$doc])* pub $field: <$codec as Codec>::Value, )*
        }

        impl Settings {
            /// The key of every setting, in the order of definition.
            pub const KEYS: &'static [&'static str] = &[ $( $key, )* ];

            /// Every setting at its default.
            pub fn defaults() -> Self {
                Self { $( $field: $default, )* }
            }

            /// The value of a setting in its stored form, or `None` for an unknown key.
            pub fn get(&self, key: &str) -> Option<Value> {
                match key {
                    $( $key => Some(<$codec as Codec>::encode(&self.$field)), )*
                    _ => None,
                }
            }

            /// Changes a setting from a value in its stored form.
            ///
            /// An unknown key or a value that does not fit is refused, and nothing changes.
            pub fn set(&mut self, key: &str, stored: &Value) -> Result<(), SettingsError> {
                match key {
                    $(
                        $key => {
                            self.$field = <$codec as Codec>::decode(stored)
                                .map_err(|reason| SettingsError::InvalidValue { key: $key, reason })?;
                            Ok(())
                        }
                    )*
                    _ => Err(SettingsError::UnknownKey(key.to_owned())),
                }
            }
        }
    };
}

settings! {
    /// The name of the install, shown as the page title.
    title: Text = "title", "Organizr V2".to_owned();
    /// The description of the install, for search engines and link previews.
    description: Text = "description", "Organizr - Accept no others".to_owned();
    /// The theme of the user interface.
    theme: Text = "theme", "Organizr".to_owned();
    /// The colour style of the user interface.
    style: Text = "style", "dark".to_owned();
    /// Where tabs of the default category stand in the menu.
    unsorted_tabs: Choice<UnsortedTabs> = "unsortedTabs", UnsortedTabs::Top;

    /// Which password check a login uses.
    auth_type: Choice<AuthType> = "authType", AuthType::Internal;
    /// The external service that checks passwords, or nothing.
    auth_backend: Text = "authBackend", String::new();
    /// How many days a login with "remember me" lasts.
    remember_me_days: TextNumber<u32> = "rememberMeDays", 7;
    /// How many failed logins lock a caller out.
    login_attempts: TextNumber<u32> = "loginAttempts", 5;
    /// How long a lock-out lasts, in milliseconds.
    login_lockout: TextNumber<u64> = "loginLockout", 60_000;
    /// Whether a session is only valid from the address it was created at.
    match_user_ip: Flag = "matchUserIP", false;
    /// Whether a session is only valid from the browser it was created in.
    match_user_agents: Flag = "matchUserAgents", false;
    /// Whether users can lock their session behind the lock screen.
    lock_screen: Flag = "lockScreen", false;
    /// Whether the login page hides the link to register.
    hide_registration: Flag = "hideRegistration", false;
    /// Addresses and ranges, separated by commas, that count as the local network.
    local_ip_list: Text = "localIPList", String::new();
}

impl Settings {
    /// Builds the settings from what the database holds: the defaults, with each stored
    /// value on top.
    ///
    /// `stored` gives each key with its value as JSON text. A stored entry that cannot be
    /// used (an unknown key, a value that does not fit, text that is not JSON) is left out
    /// and reported, and its setting keeps the default. The application can then still
    /// start and show the problem.
    pub fn from_stored<'a, I>(stored: I) -> (Self, Vec<SettingsError>)
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut settings = Self::defaults();
        let mut problems = Vec::new();
        for (key, text) in stored {
            let applied = serde_json::from_str::<Value>(text)
                .map_err(|error| SettingsError::InvalidJson {
                    key: key.to_owned(),
                    reason: error.to_string(),
                })
                .and_then(|value| settings.set(key, &value));
            if let Err(problem) = applied {
                problems.push(problem);
            }
        }
        (settings, problems)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{Settings, SettingsError};
    use crate::choices::{AuthType, UnsortedTabs};

    /// The defaults of `api/config/default.php` of Organizr 2.1.5000, as JSON.
    fn php_defaults() -> Vec<(&'static str, Value)> {
        vec![
            ("title", json!("Organizr V2")),
            ("description", json!("Organizr - Accept no others")),
            ("theme", json!("Organizr")),
            ("style", json!("dark")),
            ("unsortedTabs", json!("top")),
            ("authType", json!("internal")),
            ("authBackend", json!("")),
            ("rememberMeDays", json!("7")),
            ("loginAttempts", json!("5")),
            ("loginLockout", json!("60000")),
            ("matchUserIP", json!(false)),
            ("matchUserAgents", json!(false)),
            ("lockScreen", json!(false)),
            ("hideRegistration", json!(false)),
            ("localIPList", json!("")),
        ]
    }

    #[test]
    fn every_default_is_the_default_of_the_php_application() {
        let defaults = Settings::defaults();
        let expected = php_defaults();

        let listed: Vec<&str> = expected.iter().map(|(key, _)| *key).collect();
        assert_eq!(listed, Settings::KEYS, "the test lists every setting once");
        for (key, value) in expected {
            assert_eq!(defaults.get(key), Some(value), "the default of `{key}`");
        }
    }

    #[test]
    fn fields_are_typed() {
        let defaults = Settings::defaults();

        assert_eq!(defaults.remember_me_days, 7_u32);
        assert_eq!(defaults.login_lockout, 60_000_u64);
        assert_eq!(defaults.auth_type, AuthType::Internal);
        assert_eq!(defaults.unsorted_tabs, UnsortedTabs::Top);
        assert!(!defaults.match_user_ip);
        assert_eq!(defaults.title, "Organizr V2");
    }

    #[test]
    fn valid_value_changes_the_setting() {
        let mut settings = Settings::defaults();

        settings
            .set("rememberMeDays", &json!("30"))
            .expect("thirty days is valid");
        settings
            .set("authType", &json!("both"))
            .expect("both is a choice");
        settings
            .set("matchUserIP", &json!(true))
            .expect("true is valid");
        settings
            .set("title", &json!("Home"))
            .expect("text is valid");

        assert_eq!(settings.remember_me_days, 30);
        assert_eq!(settings.auth_type, AuthType::Both);
        assert!(settings.match_user_ip);
        assert_eq!(settings.title, "Home");
        assert_eq!(settings.get("rememberMeDays"), Some(json!("30")));
        assert_eq!(settings.get("authType"), Some(json!("both")));
    }

    #[test]
    fn number_that_is_stored_as_a_number_is_read_and_written_back_as_text() {
        let mut settings = Settings::defaults();

        settings
            .set("loginAttempts", &json!(3))
            .expect("a JSON number is read");

        assert_eq!(settings.login_attempts, 3);
        assert_eq!(settings.get("loginAttempts"), Some(json!("3")));
    }

    #[test]
    fn value_that_does_not_fit_is_refused_and_nothing_changes() {
        let mut settings = Settings::defaults();

        let cases = [
            ("rememberMeDays", json!("soon")),
            ("rememberMeDays", json!("-1")),
            ("rememberMeDays", json!(true)),
            ("authType", json!("sometimes")),
            ("authType", json!(1)),
            ("matchUserIP", json!("true")),
            ("title", json!(42)),
        ];
        for (key, value) in cases {
            let result = settings.set(key, &value);
            assert!(
                matches!(result, Err(SettingsError::InvalidValue { key: refused, .. }) if refused == key),
                "`{key}` = {value} gave {result:?}"
            );
        }

        assert_eq!(settings, Settings::defaults());
    }

    #[test]
    fn refusal_of_a_choice_names_the_choices() {
        let mut settings = Settings::defaults();

        let error = settings
            .set("authType", &json!("sometimes"))
            .expect_err("sometimes is not a choice");

        assert_eq!(
            error.to_string(),
            "the value of `authType` is not valid: expected one of internal, external, both, found \"sometimes\""
        );
    }

    #[test]
    fn unknown_key_is_refused() {
        let mut settings = Settings::defaults();

        assert_eq!(
            settings.set("organizrHash", &json!("x")),
            Err(SettingsError::UnknownKey("organizrHash".to_owned()))
        );
        assert_eq!(settings.get("organizrHash"), None);
        assert_eq!(settings, Settings::defaults());
    }

    #[test]
    fn stored_values_go_on_top_of_the_defaults() {
        let stored = [("rememberMeDays", "\"14\""), ("lockScreen", "true")];

        let (settings, problems) = Settings::from_stored(stored);

        assert_eq!(problems, []);
        assert_eq!(settings.remember_me_days, 14);
        assert!(settings.lock_screen);
        assert_eq!(
            settings.login_attempts, 5,
            "a setting that is not stored keeps its default"
        );
    }

    #[test]
    fn stored_entry_that_cannot_be_used_is_reported_and_left_out() {
        let stored = [
            ("rememberMeDays", "\"14\""),
            ("noSuchSetting", "true"),
            ("loginAttempts", "\"many\""),
            ("title", "not json"),
        ];

        let (settings, problems) = Settings::from_stored(stored);

        assert_eq!(settings.remember_me_days, 14, "the usable entry is applied");
        assert_eq!(settings.login_attempts, 5);
        assert_eq!(settings.title, "Organizr V2");
        assert_eq!(problems.len(), 3);
        assert!(matches!(&problems[0], SettingsError::UnknownKey(key) if key == "noSuchSetting"));
        assert!(matches!(
            &problems[1],
            SettingsError::InvalidValue {
                key: "loginAttempts",
                ..
            }
        ));
        assert!(matches!(&problems[2], SettingsError::InvalidJson { key, .. } if key == "title"));
    }
}
