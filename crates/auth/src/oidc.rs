//! The Organizr account of a person after a login with an OpenID Connect provider.
//!
//! The provider has checked who the person is. This module finds the account of the
//! person, links it to the provider, or makes it, as the settings say. The caller starts
//! the session.

use organizr_db::{Database, DbError};
use organizr_domain::{Change, GroupId, GroupLevel, Timestamp, User, UserChanges};
use thiserror::Error;

use crate::external::PLACEHOLDER_DOMAIN;
use crate::recovery::{self, RecoveryError};
use crate::registration::{self, NewAccount, RegistrationError};

/// The `auth_service` of an account that logs in through a provider.
pub const SERVICE: &str = "oidc";

/// A person as the provider describes the person, and what the settings allow.
#[derive(Debug, Clone, Copy)]
pub struct OidcAccount<'a> {
    /// The key of the provider, for example `authentik`.
    pub provider: &'a str,
    /// The user name. It is not empty.
    pub username: &'a str,
    /// The e-mail address. Empty when the provider gives none.
    pub email: &'a str,
    /// Whether the provider has checked the address, when it says so.
    pub email_verified: Option<bool>,
    /// The address of the picture of the person, when the provider gives one.
    pub picture: Option<&'a str>,
    /// The level of the Organizr group of the person, from the group mappings.
    pub level: GroupLevel,
    /// The setting `oidcLinkExistingUsers`.
    pub link_existing: bool,
    /// The setting `oidcAutoCreateUsers`.
    pub auto_create: bool,
    /// The setting `oidcUpdateGroupsOnLogin`.
    pub update_group: bool,
}

/// Why no account could be used.
#[derive(Debug, Error)]
pub enum OidcAccountError {
    /// An account has the name, and the settings do not let the provider link it.
    #[error("the account `{0}` exists, and linking existing accounts is off")]
    NotLinked(String),
    /// The account is an Admin or a Co-Admin account that this provider has not linked.
    #[error(
        "the account `{0}` is an Admin or a Co-Admin account, which is never linked by its name or its address"
    )]
    Protected(String),
    /// No account fits, and the settings make none.
    #[error("no account is named `{0}`, and the automatic creation of accounts is off")]
    NoAccount(String),
    /// No group has the level, and no group is the default group.
    #[error("no group has the level {0}, and no group is the default group")]
    NoGroup(u16),
    /// The account could not be made.
    #[error("the account could not be made: {0}")]
    Account(#[source] RegistrationError),
    /// No random password could be made.
    #[error(transparent)]
    Random(#[from] RecoveryError),
    /// The database failed.
    #[error(transparent)]
    Database(#[from] DbError),
}

/// The account of the person, with whether this login made it.
#[derive(Debug)]
pub struct Found {
    pub user: User,
    pub created: bool,
}

/// Finds, links or makes the account of a person, as the PHP application does:
///
/// 1. The account with the user name, whatever the letter case.
/// 2. With `link_existing`, the account with the e-mail address, unless the provider
///    says that it has not checked the address.
/// 3. With `auto_create`, a new account with a random password.
///
/// An account found by its name is linked only with `link_existing`, or when this
/// provider has linked or made it before. An Admin or a Co-Admin account is linked only
/// when this provider has linked or made it before, whatever the settings. The account gets the service `oidc` and the
/// provider. With `update_group`, a found account moves to the mapped group.
pub async fn find_or_create(
    database: &Database,
    account: OidcAccount<'_>,
    now: Timestamp,
) -> Result<Found, OidcAccountError> {
    let users = database.users();
    let group = group_for(database, account.level).await?;

    let by_name = users.find_by_username(account.username).await?;
    let found = match by_name {
        Some(user) => {
            if !account.link_existing && !is_linked(database, &user, account.provider).await? {
                return Err(OidcAccountError::NotLinked(user.username));
            }
            Some(user)
        }
        None if account.link_existing
            && !account.email.is_empty()
            && account.email_verified != Some(false) =>
        {
            users.find_by_email(account.email).await?
        }
        None => None,
    };

    // An Admin or a Co-Admin account is never linked by its name or its address: only
    // an account that this provider linked or made before logs in.
    if let Some(user) = &found
        && !is_linked(database, user, account.provider).await?
        && is_privileged(database, user).await?
    {
        return Err(OidcAccountError::Protected(user.username.clone()));
    }

    if let Some(user) = found {
        users
            .set_auth_service(user.id, SERVICE, Some(account.provider))
            .await?;
        let user = if account.update_group && user.group != group {
            let changes = UserChanges {
                group: Change::Set(group),
                ..UserChanges::none()
            };
            users.update(user.id, changes).await?.unwrap_or(user)
        } else {
            user
        };
        tracing::info!(user = %user.username, provider = account.provider, "an account was linked to the provider");
        return Ok(Found {
            user,
            created: false,
        });
    }

    if !account.auto_create {
        return Err(OidcAccountError::NoAccount(account.username.to_owned()));
    }
    let email = if account.email.is_empty() {
        format!("{}@{PLACEHOLDER_DOMAIN}", recovery::new_password()?)
    } else {
        account.email.to_owned()
    };
    let new = NewAccount {
        username: account.username.to_owned(),
        password: recovery::new_password()?,
        email,
    };
    let user = registration::create_account(database, new, now)
        .await
        .map_err(OidcAccountError::Account)?;
    users
        .set_auth_service(user.id, SERVICE, Some(account.provider))
        .await?;
    let picture = account.picture.filter(|address| is_web_address(address));
    let changes = UserChanges {
        group: if user.group == group {
            Change::Keep
        } else {
            Change::Set(group)
        },
        image: picture.map_or(Change::Keep, |address| {
            Change::Set(Some(address.to_owned()))
        }),
        ..UserChanges::none()
    };
    let user = users.update(user.id, changes).await?.unwrap_or(user);
    tracing::info!(user = %user.username, provider = account.provider, "an account was made for a user of the provider");
    Ok(Found {
        user,
        created: true,
    })
}

/// Whether the account is in a group of the level Admin or Co-Admin, as it is stored
/// now. An account whose group is gone counts as one.
async fn is_privileged(database: &Database, user: &User) -> Result<bool, DbError> {
    Ok(database
        .groups()
        .find(user.group)
        .await?
        .is_none_or(|group| group.level <= GroupLevel::CO_ADMIN))
}

/// Whether this provider has linked or made the account before.
async fn is_linked(database: &Database, user: &User, provider: &str) -> Result<bool, DbError> {
    Ok(database
        .users()
        .login_of(user.id)
        .await?
        .is_some_and(|login| {
            login.auth_service == SERVICE && login.auth_provider.as_deref() == Some(provider)
        }))
}

/// The group of a level. When no group has the level, the default group, so that the
/// account has a group that exists.
async fn group_for(database: &Database, level: GroupLevel) -> Result<GroupId, OidcAccountError> {
    let groups = database.groups();
    if let Some(group) = groups.find_by_level(level).await? {
        return Ok(group.id);
    }
    tracing::warn!(
        level = level.get(),
        "no group has the level of the group mapping; the default group counts"
    );
    groups
        .find_default()
        .await?
        .map(|group| group.id)
        .ok_or(OidcAccountError::NoGroup(level.get()))
}

/// Whether a text is an address on the web, which a page can show as a picture.
fn is_web_address(address: &str) -> bool {
    let lower = address.trim().to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && !address
            .chars()
            .any(|character| character.is_whitespace() || character == '"')
}

#[cfg(test)]
mod tests {
    use super::is_web_address;

    #[test]
    fn only_web_addresses_become_pictures() {
        assert!(is_web_address("https://id.example.com/avatar.png"));
        assert!(is_web_address("HTTP://id.example.com/a.png"));
        assert!(!is_web_address("javascript:alert(1)"));
        assert!(!is_web_address("data:image/png;base64,AAAA"));
        assert!(!is_web_address("https://a.example.com/a b.png"));
    }
}
