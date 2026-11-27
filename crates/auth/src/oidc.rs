//! The Organizr account of a person after a login with an OpenID Connect provider.
//!
//! The provider has checked who the person is. This module finds the account of the
//! person, links it to the provider, or makes it, as the settings say. The caller starts
//! the session.

use organizr_db::{Database, DbError};
use organizr_domain::{Change, GroupId, GroupLevel, Timestamp, User, UserChanges, UserId};
use thiserror::Error;

use crate::external::PLACEHOLDER_DOMAIN;
use crate::password::{self, PasswordCheck};
use crate::recovery::{self, RecoveryError};
use crate::registration::{self, NewAccount, RegistrationError};

/// The `auth_service` of an account that logs in through a provider.
pub const SERVICE: &str = "oidc";

/// A person as the provider describes the person, and what the settings allow.
#[derive(Debug, Clone, Copy)]
pub struct OidcAccount<'a> {
    /// The key of the provider, for example `authentik`.
    pub provider: &'a str,
    /// The subject (`sub`) of the checked ID token.
    pub subject: &'a str,
    /// The user name. It is not empty.
    pub username: &'a str,
    /// The preferred user name of the provider, when it gave one. Only this name finds
    /// an existing account by its name, because a display name is often one that the
    /// person can change at the provider.
    pub login_name: Option<&'a str>,
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
    /// The account with the name or the address is linked to another account of this
    /// provider: another subject.
    #[error("the account `{0}` is linked to another account of the provider")]
    OtherSubject(String),
    /// The account with the name or the address is linked to another provider.
    #[error("the account `{0}` is linked to another provider")]
    LinkedElsewhere(String),
    /// Another account is linked to the subject.
    #[error("another account is linked to this account of the provider")]
    SubjectTaken,
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

/// A person who is logged in links the own account to a provider, and the provider has
/// checked who the person is.
#[derive(Debug, Clone, Copy)]
pub struct OidcLink<'a> {
    /// The key of the provider, for example `keycloak`.
    pub provider: &'a str,
    /// The account of the session that started the link.
    pub user: UserId,
    /// The subject (`sub`) of the checked ID token.
    pub subject: &'a str,
}

/// Why an account could not be linked.
#[derive(Debug, Error)]
pub enum OidcLinkError {
    /// The account of the session is gone.
    #[error("the account of the session does not exist")]
    NoAccount,
    /// Another account is linked to the same account at the provider.
    #[error("another account is linked to this account of the provider")]
    SubjectTaken,
    /// The database failed.
    #[error(transparent)]
    Database(DbError),
}

impl From<DbError> for OidcLinkError {
    fn from(error: DbError) -> Self {
        match error {
            DbError::Duplicate => Self::SubjectTaken,
            other => Self::Database(other),
        }
    }
}

/// Links the account of a session to the account of the person at a provider: the
/// subject of the checked ID token. The settings `oidcLinkExistingUsers` and the rule for
/// Admin accounts do not count, and the names may differ: the person is logged in to the
/// account and has logged in at the provider. No account is made.
///
/// The link is refused when another account is linked to the same subject of the
/// provider. A link to another provider, or to another subject, is replaced.
pub async fn link(database: &Database, link: OidcLink<'_>) -> Result<User, OidcLinkError> {
    let users = database.users();
    let user = users
        .find(link.user)
        .await?
        .ok_or(OidcLinkError::NoAccount)?;
    if let Some(other) = users.find_by_subject(link.provider, link.subject).await?
        && other.id != user.id
    {
        return Err(OidcLinkError::SubjectTaken);
    }
    users
        .set_auth_service(user.id, SERVICE, Some(link.provider), Some(link.subject))
        .await?;
    tracing::info!(user = %user.username, provider = link.provider, subject = link.subject, "a user linked the account to the provider");
    Ok(user)
}

/// Why an account could not be unlinked from its provider.
#[derive(Debug, Error)]
pub enum OidcUnlinkError {
    /// The account of the session is gone.
    #[error("the account of the session does not exist")]
    NoAccount,
    /// The account is not linked to a provider.
    #[error("the account is not linked to a provider")]
    NotLinked,
    /// The password is not the password of the account.
    #[error("the password is wrong")]
    WrongPassword,
    /// The check of the password could not run.
    #[error("the password could not be checked")]
    Task,
    /// The database failed.
    #[error(transparent)]
    Database(#[from] DbError),
}

/// Unlinks an account from its provider, so that it logs in with its password again.
/// The password of the account must be given: it proves that the person can still log
/// in once the link is gone. An account that a login of the provider made has a random
/// password, which the person sets first, for example with a reset by e-mail (D-105).
pub async fn unlink(
    database: &Database,
    user: UserId,
    given: &str,
) -> Result<User, OidcUnlinkError> {
    let users = database.users();
    let account = users.find(user).await?.ok_or(OidcUnlinkError::NoAccount)?;
    let login = users
        .login_of(user)
        .await?
        .ok_or(OidcUnlinkError::NoAccount)?;
    if login.auth_service != SERVICE {
        return Err(OidcUnlinkError::NotLinked);
    }
    let (candidate, stored) = (given.to_owned(), account.password_hash.clone());
    let check = password::verify_in_turn(candidate, stored)
        .await
        .map_err(|_| OidcUnlinkError::Task)?;
    if !matches!(check, PasswordCheck::Correct { .. }) {
        return Err(OidcUnlinkError::WrongPassword);
    }
    users
        .set_auth_service(account.id, "internal", None, None)
        .await?;
    tracing::info!(user = %account.username, provider = ?login.auth_provider, "a user unlinked the account from the provider");
    Ok(account)
}

/// The provider that an account is linked to, when it is linked to one.
pub async fn linked_provider(database: &Database, user: UserId) -> Result<Option<String>, DbError> {
    Ok(database
        .users()
        .login_of(user)
        .await?
        .filter(|login| login.auth_service == SERVICE)
        .and_then(|login| login.auth_provider))
}

/// The account of the person, with whether this login made it.
#[derive(Debug)]
pub struct Found {
    pub user: User,
    pub created: bool,
}

/// How an account is linked, as seen from one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LinkState {
    /// The account is not linked to any provider.
    Free,
    /// Linked to this provider before the subject was stored, for example by PHP.
    ThisProviderWithoutSubject,
    /// Linked to this provider, to the account with the stored subject.
    ThisProvider,
    /// Linked to another provider.
    OtherProvider,
}

/// Finds, links or makes the account of a person:
///
/// 1. The account that is linked to the subject of the ID token at this provider.
/// 2. Else the account with the user name, whatever the letter case, when the name is the
///    preferred user name of the provider, or else, only when the provider says that it has
///    checked it (`email_verified` is true), the account with the e-mail address.
///    Such an account is used only when it is linked to no provider yet (and then only
///    with `link_existing`, and never an Admin or a Co-Admin account), or when it is
///    linked to this provider without a stored subject, as an imported account is. The
///    subject is stored then, so this first login decides. An account linked to another
///    subject or another provider is refused.
/// 3. Else, with `auto_create`, a new account with a random password and the subject.
///
/// With `update_group`, a found account moves to the mapped group.
pub async fn find_or_create(
    database: &Database,
    account: OidcAccount<'_>,
    now: Timestamp,
) -> Result<Found, OidcAccountError> {
    let users = database.users();
    let group = group_for(database, account.level).await?;

    if let Some(user) = users
        .find_by_subject(account.provider, account.subject)
        .await?
    {
        let user = regroup(database, user, group, account.update_group).await?;
        return Ok(Found {
            user,
            created: false,
        });
    }

    let by_name = match account.login_name {
        Some(name) => users.find_by_username(name).await?,
        None => None,
    };
    let candidate = match by_name {
        Some(user) => Some(user),
        None if !account.email.is_empty() && account.email_verified == Some(true) => {
            users.find_by_email(account.email).await?
        }
        None => None,
    };

    if let Some(user) = candidate {
        match link_state(database, &user, account.provider).await? {
            LinkState::ThisProvider => {
                return Err(OidcAccountError::OtherSubject(user.username));
            }
            LinkState::OtherProvider => {
                return Err(OidcAccountError::LinkedElsewhere(user.username));
            }
            // An account that the import brought with a link to this provider, but no
            // subject: the first login that matches it takes it. An Admin or a Co-Admin
            // account only by an address that the provider checked and that is the
            // address of the account, never by its name (D-148, D-156).
            LinkState::ThisProviderWithoutSubject => {
                let has_the_address = account.email_verified == Some(true)
                    && user.email.as_deref().is_some_and(|email| {
                        !email.is_empty() && email.eq_ignore_ascii_case(account.email)
                    });
                if !has_the_address && is_privileged(database, &user).await? {
                    return Err(OidcAccountError::Protected(user.username));
                }
            }
            LinkState::Free => {
                if !account.link_existing {
                    return Err(OidcAccountError::NotLinked(user.username));
                }
                // An Admin or a Co-Admin account is never linked by its name or its
                // address.
                if is_privileged(database, &user).await? {
                    return Err(OidcAccountError::Protected(user.username));
                }
            }
        }
        store_subject(database, &user, account).await?;
        let user = regroup(database, user, group, account.update_group).await?;
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
    store_subject(database, &user, account).await?;
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

/// Links an account to the provider and the subject of the person.
async fn store_subject(
    database: &Database,
    user: &User,
    account: OidcAccount<'_>,
) -> Result<(), OidcAccountError> {
    database
        .users()
        .set_auth_service(
            user.id,
            SERVICE,
            Some(account.provider),
            Some(account.subject),
        )
        .await
        .map_err(|error| match error {
            // Another login stored the same subject in between.
            DbError::Duplicate => OidcAccountError::SubjectTaken,
            other => OidcAccountError::Database(other),
        })?;
    // The subject names an account at the provider; it is no secret. The log lets an
    // Admin check which account of the provider a first login linked.
    tracing::info!(
        user = %user.username,
        provider = account.provider,
        subject = account.subject,
        "the account was linked to an account of the provider"
    );
    Ok(())
}

/// The account, moved to `group` when `update` says so.
async fn regroup(
    database: &Database,
    user: User,
    group: GroupId,
    update: bool,
) -> Result<User, OidcAccountError> {
    if !update || user.group == group {
        return Ok(user);
    }
    let changes = UserChanges {
        group: Change::Set(group),
        ..UserChanges::none()
    };
    Ok(database
        .users()
        .update(user.id, changes)
        .await?
        .unwrap_or(user))
}

/// Whether the account is in a group of the level Admin or Co-Admin, as it is stored
/// now. An account whose group is gone counts as one.
pub(crate) async fn is_privileged(database: &Database, user: &User) -> Result<bool, DbError> {
    Ok(database
        .groups()
        .find(user.group)
        .await?
        .is_none_or(|group| group.level <= GroupLevel::CO_ADMIN))
}

/// How the account is linked, as seen from `provider`.
async fn link_state(
    database: &Database,
    user: &User,
    provider: &str,
) -> Result<LinkState, DbError> {
    let Some(login) = database.users().login_of(user.id).await? else {
        return Ok(LinkState::Free);
    };
    if login.auth_service != SERVICE {
        return Ok(LinkState::Free);
    }
    Ok(match (login.auth_provider.as_deref(), login.auth_subject) {
        (Some(linked), None) if linked == provider => LinkState::ThisProviderWithoutSubject,
        (Some(linked), Some(_)) if linked == provider => LinkState::ThisProvider,
        (Some(_), _) => LinkState::OtherProvider,
        (None, _) => LinkState::Free,
    })
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
