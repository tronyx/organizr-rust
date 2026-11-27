//! The `/users` routes: the accounts of the install.
//!
//! An Admin or a Co-Admin lists, adds, changes and deletes users. A logged-in user can
//! change some fields of the own account. `docs/deviations.md` lists the differences
//! from the PHP application as D-05, D-15 and D-52.

use std::collections::HashMap;
use std::time::SystemTime;

use axum::Extension;
use axum::extract::{Path, RawQuery, State};
use axum::response::{IntoResponse, Response};
use organizr_auth::registration::{
    LONGEST_NAME, LONGEST_PASSWORD, NewAccount, create_account, hash_password,
};
use organizr_db::{Database, DbError, UserLogin};
use organizr_domain::email::{self, php_trim};
use organizr_domain::{Change, Group, GroupId, GroupLevel, User, UserChanges, UserId};
use serde::Serialize;
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::{Caller, now_stamp};
use crate::client::Client;
use crate::envelope::{Envelope, NoContent, NoData};
use crate::handlers::fields::{Fields, Refusal, has_key, php_text, record_id};
use crate::handlers::groups::GroupRow;
use crate::handlers::proof;
use crate::handlers::token::date_and_time;
use crate::state::AppState;

/// A user as the API shows it. The field names and their order are those of the PHP
/// application.
#[derive(Debug, Serialize)]
pub struct UserRow {
    id: i64,
    username: String,
    /// Always `null`: the hash of a password never leaves the server (D-05).
    password: Option<String>,
    email: Option<String>,
    /// Always `null`: the token of a service never leaves the server (D-05).
    plex_token: Option<String>,
    /// The name of the group of the user.
    group: Option<String>,
    /// The level of the group of the user.
    group_id: Option<u16>,
    locked: u8,
    image: Option<String>,
    /// When the user was stored, as `2026-11-18 19:02:06` in UTC.
    register_date: String,
    /// Which service checks the password of the user, or the kind of the second
    /// factor of the user. The secret of a second factor is never shown (D-57).
    auth_service: String,
}

impl UserRow {
    /// The row for a user. `groups` are the groups of the install, for the name and the
    /// level of the group of the user. `logins` says how each user logs in.
    pub fn of(user: User, groups: &[Group], logins: &HashMap<UserId, UserLogin>) -> Self {
        let group = groups.iter().find(|group| group.id == user.group);
        Self {
            id: user.id.get(),
            username: user.username,
            password: None,
            email: user.email,
            plex_token: None,
            group: group.map(|group| group.name.clone()),
            group_id: group.map(|group| group.level.get()),
            locked: u8::from(user.locked),
            image: user.image,
            register_date: date_and_time(user.created_at),
            auth_service: logins
                .get(&user.id)
                .map_or_else(|| "internal".to_owned(), UserLogin::auth_service_text),
        }
    }
}

/// The level a caller needs to list, add and delete users, and to change another user.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The word for the record in the text of a refusal.
const RECORD: &str = "User";

/// The `data` of `GET /users`: the list of users, or the users and the groups when the
/// request asks for both.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UserList {
    /// The users alone.
    Users(Vec<UserRow>),
    /// The users and every group.
    WithGroups {
        /// The users.
        users: Vec<UserRow>,
        /// The groups.
        groups: Vec<GroupRow>,
    },
}

/// The id of a user from the path.
fn user_id(text: &str) -> Option<UserId> {
    record_id(text).map(UserId::new)
}

/// The user that the path names, when there is one.
pub(crate) async fn user_at(database: &Database, id: &str) -> Result<Option<User>, DbError> {
    match user_id(id) {
        Some(id) => database.users().find(id).await,
        None => Ok(None),
    }
}

/// Whether the user is in the group of the Admin.
pub(crate) fn is_admin(groups: &[Group], user: &User) -> bool {
    groups
        .iter()
        .any(|group| group.id == user.group && group.level == GroupLevel::ADMIN)
}

/// Whether a Co-Admin acts on the account of another Co-Admin. Such a caller cannot take
/// over the account: change its password, address, lock or group, or remove its second
/// factor (D-188).
pub(crate) fn is_another_co_admin(caller: &Caller, groups: &[Group], user: &User) -> bool {
    let is_co_admin = groups
        .iter()
        .any(|group| group.id == user.group && group.level == GroupLevel::CO_ADMIN);
    caller.level() == GroupLevel::CO_ADMIN
        && is_co_admin
        && caller
            .user()
            .is_none_or(|caller| caller.authenticated.user.id != user.id)
}

/// Whether the user is the only one in the group of the Admin. That user cannot be
/// deleted or leave the group, because an install without an Admin cannot be managed.
async fn is_last_admin(
    database: &Database,
    groups: &[Group],
    user: &User,
) -> Result<bool, DbError> {
    Ok(is_admin(groups, user) && database.groups().user_count(user.group).await? <= 1)
}

/// `GET /users`: every user. With `includeGroups` in the query, the answer also holds
/// every group.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Envelope<UserList>, Refusal> {
    caller.require(MANAGER)?;
    let database = state.database();
    let groups = database.groups().list().await?;
    let logins = database.users().logins().await?;
    let users: Vec<UserRow> = database
        .users()
        .list()
        .await?
        .into_iter()
        .map(|user| UserRow::of(user, &groups, &logins))
        .collect();
    let data = if has_key(query.as_deref(), "includeGroups") {
        UserList::WithGroups {
            users,
            groups: groups.into_iter().map(GroupRow::from).collect(),
        }
    } else {
        UserList::Users(users)
    };
    Ok(Envelope::success(data))
}

/// `GET /users/{id}`: one user. As in the PHP application, an id without a user is no
/// error: the answer is `200` and its `data` is `null`.
pub async fn one(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<Option<UserRow>>, Refusal> {
    one_by_text(&state, &caller, &id).await
}

/// [`one`] for the id as the path has it.
pub async fn one_by_text(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> Result<Envelope<Option<UserRow>>, Refusal> {
    caller.require(MANAGER)?;
    let database = state.database();
    let Some(user) = user_at(database, id).await? else {
        return Ok(Envelope::success(None));
    };
    let groups = database.groups().list().await?;
    let logins = database.users().logins().await?;
    Ok(Envelope::success(Some(UserRow::of(user, &groups, &logins))))
}

/// `POST /users`: adds a user to the default group.
///
/// The body must name `username`, `password` and `email`. Other fields are ignored: the
/// group, the picture and the lock of a user are changed with `PUT /users/{id}`.
pub async fn add(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    // What PHP takes for "nothing": no value, the empty text, and the text `0`.
    let field = |name: &str| {
        data.text(name)
            .filter(|value| !value.is_empty() && value != "0")
    };
    let Some(username) = field("username") else {
        return Err(Refusal::conflict("Username was not supplied"));
    };
    let Some(password) = field("password") else {
        return Err(Refusal::conflict("Password was not supplied"));
    };
    let Some(email) = field("email") else {
        return Err(Refusal::conflict("Email was set not supplied"));
    };
    // The PHP application checks the address and the lengths here, and then again with
    // the other checks of a new account.
    if !email::is_valid(&email) {
        return Err(Refusal::invalid_email(&email));
    }
    for (length, limit) in [
        (php_trim(&username).len(), LONGEST_NAME),
        (email::sanitize(&email).len(), LONGEST_NAME),
        (password.len(), LONGEST_PASSWORD),
    ] {
        if length > limit {
            return Err(Refusal::over_limit(limit));
        }
    }

    let account = NewAccount {
        username,
        password,
        email,
    };
    let user = create_account(state.database(), account, now_stamp(SystemTime::now())).await?;
    tracing::info!(user = %user.username, id = %user.id, "a user was added");
    Ok(Envelope::success(NoData)
        .with_message("User created")
        .into_response())
}

/// Who changes a user.
struct Actor {
    /// The level of the caller.
    level: GroupLevel,
    /// Whether the caller changes the own account.
    is_target: bool,
}

/// The name for a user from the field `username`: without the space around it, not
/// empty, not the name or the address of another user, and at most 50 bytes.
async fn username_of(database: &Database, value: &Value, user: &User) -> Result<String, Refusal> {
    let text = php_text(value).ok_or_else(|| Refusal::not_valid(RECORD, "username"))?;
    let name = php_trim(&text);
    if name.is_empty() {
        return Err(Refusal::conflict("Username was set but empty"));
    }
    if database.users().is_taken_by_another(name, user.id).await? {
        return Err(Refusal::conflict(format!(
            "Username: {name} is already taken"
        )));
    }
    if name.len() > LONGEST_NAME {
        return Err(Refusal::over_limit(LONGEST_NAME));
    }
    Ok(name.to_owned())
}

/// The address for a user from the field `email`: an e-mail address that no other user
/// has as address or as name, and at most 50 bytes.
async fn email_of(database: &Database, value: &Value, user: &User) -> Result<String, Refusal> {
    let text = php_text(value).ok_or_else(|| Refusal::not_valid(RECORD, "email"))?;
    if text.is_empty() {
        return Err(Refusal::conflict("Email was set but empty"));
    }
    if !email::is_valid(&text) {
        return Err(Refusal::invalid_email(&text));
    }
    let address = email::sanitize(&text);
    if database
        .users()
        .is_taken_by_another(&address, user.id)
        .await?
    {
        return Err(Refusal::conflict(format!(
            "Email: {address} is already taken"
        )));
    }
    if address.len() > LONGEST_NAME {
        return Err(Refusal::over_limit(LONGEST_NAME));
    }
    Ok(address)
}

/// The group for a user from the field `group_id`, which holds the level of the group.
///
/// Only an Admin or a Co-Admin can change a group, and only an Admin can give the level
/// of the Admin. An Admin stays an Admin in a change of the own account. The last Admin
/// cannot leave the group.
async fn group_of(
    database: &Database,
    fields: &Fields<'_>,
    groups: &[Group],
    user: &User,
    actor: &Actor,
) -> Result<Change<GroupId>, Refusal> {
    let Change::Set(level) = fields.required(fields.number("group_id")?, "group_id")? else {
        return Ok(Change::Keep);
    };
    if actor.level > MANAGER {
        return Err(Refusal::not_authorized("Cannot change your own group_id"));
    }
    if actor.is_target && actor.level == GroupLevel::ADMIN {
        return Ok(Change::Keep);
    }
    if level == i64::from(GroupLevel::ADMIN) && actor.level != GroupLevel::ADMIN {
        return Err(Refusal::not_authorized(
            "Only admins can make others admins",
        ));
    }
    let group = groups
        .iter()
        .find(|group| i64::from(group.level) == level)
        .ok_or_else(|| Refusal::not_found("group_id does not exist"))?;
    if group.id != user.group && is_last_admin(database, groups, user).await? {
        return Err(Refusal::conflict(
            "Cannot change the group of the last admin",
        ));
    }
    Ok(Change::Set(group.id))
}

/// The new password from the field `password`.
fn password_of(value: &Value) -> Result<String, Refusal> {
    let password = php_text(value).ok_or_else(|| Refusal::not_valid(RECORD, "password"))?;
    if password.is_empty() {
        return Err(Refusal::conflict("Password was set but empty"));
    }
    if password.len() > LONGEST_PASSWORD {
        return Err(Refusal::over_limit(LONGEST_PASSWORD));
    }
    Ok(password)
}

/// `PUT /users/{id}`: changes the fields of a user that the body names.
///
/// A logged-in user can change the name, the address, the password and the picture of
/// the own account. A new password or a new address of the own account needs the
/// current password in the field `currentPassword` (D-185). An Admin or a Co-Admin can
/// change every user and also the group and the lock, but only an Admin can change an
/// Admin, and only an Admin can change the password, the address, the lock or the group
/// of a Co-Admin other than the caller (D-188). A field that the application does not know is ignored, and so is `id`.
pub async fn change(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    Path(id): Path<String>,
    data: ApiData,
) -> Result<Response, Refusal> {
    change_by_text(&state, &caller, &client, &id, &data).await
}

/// Ends the sessions of a user whose password was changed, so that a stolen session
/// stops working (D-146). A change of the own password keeps the session of the change;
/// a change by somebody else ends every session of the user.
async fn end_sessions_after_new_password(
    database: &Database,
    caller: &Caller,
    user: &User,
    own: bool,
) -> Result<(), Refusal> {
    let sessions = database.sessions();
    let kept = caller
        .user()
        .filter(|_| own)
        .map(|caller| caller.authenticated.session.id);
    let ended = match kept {
        Some(kept) => sessions.delete_all_of_user_except(user.id, kept).await?,
        None => sessions.delete_all_of_user(user.id).await?,
    };
    tracing::info!(user = %user.username, ended, "the sessions of a user with a new password were ended");
    Ok(())
}

/// [`change`] for the id as the path has it.
pub async fn change_by_text(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    id: &str,
    data: &ApiData,
) -> Result<Response, Refusal> {
    caller.require(GroupLevel::LOGGED_IN)?;
    // What PHP counts as "no id": nothing, or the text `0`.
    if id.is_empty() || id == "0" {
        return Err(Refusal::unprocessable("Id was not supplied"));
    }
    let own_id = caller.user().map(|caller| caller.authenticated.user.id);
    let actor = Actor {
        level: caller.level(),
        is_target: own_id.is_some() && own_id == user_id(id),
    };
    if !actor.is_target {
        caller.require(MANAGER)?;
    }
    let database = state.database();
    let fields = Fields::new(data, RECORD);
    let not_found = || Refusal::not_found("User was not found");
    let user = user_at(database, id).await?.ok_or_else(not_found)?;
    let groups = database.groups().list().await?;
    if is_admin(&groups, &user) && actor.level != GroupLevel::ADMIN {
        return Err(Refusal::not_authorized(
            "Cannot update admin unless you are admin",
        ));
    }

    // The checks of the PHP application, in its order.
    let username = match data.field("username") {
        Some(value) => Change::Set(username_of(database, value, &user).await?),
        None => Change::Keep,
    };
    let email = match data.field("email") {
        Some(value) => Change::Set(email_of(database, value, &user).await?),
        None => Change::Keep,
    };
    let group = group_of(database, &fields, &groups, &user, &actor).await?;
    let locked = fields.switch("locked")?;
    if locked != Change::Keep && actor.level > MANAGER {
        return Err(Refusal::not_authorized("Cannot change your own locked"));
    }
    let password = data.field("password").map(password_of).transpose()?;
    // A Co-Admin cannot take over another Co-Admin (D-188).
    let new_lock = matches!(locked, Change::Set(lock) if lock != user.locked);
    let new_group = matches!(group, Change::Set(group) if group != user.group);
    let new_address =
        matches!(&email, Change::Set(address) if user.email.as_deref() != Some(address.as_str()));
    if is_another_co_admin(caller, &groups, &user)
        && (password.is_some() || new_address || new_lock || new_group)
    {
        return Err(Refusal::not_authorized(
            "Cannot update admin unless you are admin",
        ));
    }
    // A new password or a new address of the own account needs the current password, so
    // that a session alone cannot take the account over (D-185).
    if actor.is_target && (new_address || password.is_some()) {
        proof::password(state, client, &user, data.text("currentPassword")).await?;
    }
    let password_hash = match password {
        Some(password) => Change::Set(hash_password(password).await?),
        None => Change::Keep,
    };
    let image = fields.image()?;
    if data.field("register_date").is_some() {
        return Err(Refusal::conflict("Cannot update register date"));
    }

    let name = username.clone();
    let new_password = matches!(password_hash, Change::Set(_));
    let changes = UserChanges {
        username,
        email,
        password_hash,
        group,
        locked,
        image,
    };
    match database.users().update(user.id, changes).await {
        Ok(Some(_)) => {
            tracing::info!(user = %user.username, id = %user.id, "a user was changed");
            if new_password {
                end_sessions_after_new_password(database, caller, &user, actor.is_target).await?;
            }
            Ok(Envelope::success(NoData)
                .with_message("User info updated")
                .into_response())
        }
        // Another request deleted the user after the lookup above.
        Ok(None) => Err(not_found()),
        // Another request took the name after the check above.
        Err(DbError::Duplicate) => Err(Refusal::conflict(format!(
            "Username: {} is already taken",
            name.or(user.username)
        ))),
        Err(error) => Err(error.into()),
    }
}

/// `DELETE /users/{id}`: deletes a user and the sessions of that user.
///
/// Nobody can delete the own account. Only an Admin can delete an Admin, and the last
/// Admin stays.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    remove_by_text(&state, &caller, &id).await
}

/// [`remove`] for the id as the path has it.
pub async fn remove_by_text(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let own_id = caller.user().map(|caller| caller.authenticated.user.id);
    if own_id.is_some() && own_id == user_id(id) {
        return Err(Refusal::conflict("Cannot delete your own user"));
    }
    let database = state.database();
    let not_found = || Refusal::not_found("id not found");
    let user = user_at(database, id).await?.ok_or_else(not_found)?;
    let groups = database.groups().list().await?;
    if is_admin(&groups, &user) && caller.level() != GroupLevel::ADMIN {
        return Err(Refusal::not_authorized(
            "Cannot delete admin unless you are admin",
        ));
    }
    if is_last_admin(database, &groups, &user).await? {
        return Err(Refusal::conflict("Cannot delete the last admin"));
    }
    if database.users().delete(user.id).await? {
        tracing::info!(user = %user.username, id = %user.id, "a user was deleted");
        Ok(NoContent.into_response())
    } else {
        // Another request deleted the user after the lookup above.
        Err(not_found())
    }
}
