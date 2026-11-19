//! The `/groups` routes: the groups of users, for the people who manage them.
//!
//! Every route needs an Admin or a Co-Admin. A group has a level, which says how much
//! access its users have: 0 is the Admin and 999 is a guest. A new group gets the level
//! after the highest one in use below 999, and the level of a group never changes.
//!
//! As with tabs and categories, a value must be of the kind its field takes.
//! `docs/deviations.md` lists the differences from the PHP application as D-51.

use axum::Extension;
use axum::extract::{Path, RawQuery, State};
use axum::response::{IntoResponse, Response};
use organizr_db::{Database, DbError};
use organizr_domain::email::php_trim;
use organizr_domain::{
    Change, Group, GroupChanges, GroupId, GroupLevel, InvalidMenuName, MENU_NAME_LIMIT, NewGroup,
    menu_name,
};
use serde::Serialize;
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::{Envelope, NoContent, NoData};
use crate::handlers::fields::{Fields, Refusal, has_key, php_text, record_id};
use crate::handlers::users::UserRow;
use crate::state::AppState;

/// The level a caller needs for the group routes.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The word for the record in the text of a refusal.
const RECORD: &str = "Group";

/// A group as the API shows it. The field names are those of the PHP application.
#[derive(Debug, Serialize)]
pub struct GroupRow {
    id: i64,
    group: String,
    /// The level of the group. The PHP application calls it `group_id`.
    group_id: u16,
    image: Option<String>,
    default: u8,
}

impl From<Group> for GroupRow {
    fn from(group: Group) -> Self {
        Self {
            id: group.id.get(),
            group: group.name,
            group_id: group.level.get(),
            image: group.image,
            default: u8::from(group.is_default),
        }
    }
}

/// The `data` of `GET /groups`: the list of groups, or the groups and the users when the
/// request asks for both.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum GroupList {
    /// The groups alone.
    Groups(Vec<GroupRow>),
    /// The groups and every user.
    WithUsers {
        /// The groups.
        groups: Vec<GroupRow>,
        /// The users.
        users: Vec<UserRow>,
    },
}

/// The refusal for a name that another group has.
fn name_taken(name: &str) -> Refusal {
    Refusal::conflict(format!("Group name: {name} is already taken"))
}

/// The id of a group from the path.
fn group_id(text: &str) -> Option<GroupId> {
    record_id(text).map(GroupId::new)
}

/// The group that the path names, when there is one.
async fn group_at(database: &Database, id: &str) -> Result<Option<Group>, DbError> {
    match group_id(id) {
        Some(id) => database.groups().find(id).await,
        None => Ok(None),
    }
}

/// The name for a group from the field `group`, checked as the PHP application checks
/// it: the space around it is cut off, no other group has it, and it has at most 50
/// bytes. `except` is the group that gets the name, which can keep the name it has.
/// `empty` is the refusal for a name that is empty.
async fn name_of(
    database: &Database,
    value: &Value,
    except: Option<GroupId>,
    empty: Refusal,
) -> Result<String, Refusal> {
    let text = php_text(value).ok_or_else(|| Refusal::not_valid(RECORD, "group"))?;
    let name = menu_name(&text);
    if name == Err(InvalidMenuName::Empty) {
        return Err(empty);
    }
    let trimmed = php_trim(&text);
    if database.groups().is_name_taken(trimmed, except).await? {
        return Err(name_taken(trimmed));
    }
    name.map_err(|_| Refusal::over_limit(MENU_NAME_LIMIT))
}

/// `GET /groups`: every group, from the most access to the least. With `includeUsers` in
/// the query, the answer also holds every user.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Envelope<GroupList>, Refusal> {
    caller.require(MANAGER)?;
    let database = state.database();
    let groups = database.groups().list().await?;
    let data = if has_key(query.as_deref(), "includeUsers") {
        let logins = database.users().logins().await?;
        let users = database
            .users()
            .list()
            .await?
            .into_iter()
            .map(|user| UserRow::of(user, &groups, &logins))
            .collect();
        GroupList::WithUsers {
            groups: groups.into_iter().map(GroupRow::from).collect(),
            users,
        }
    } else {
        GroupList::Groups(groups.into_iter().map(GroupRow::from).collect())
    };
    Ok(Envelope::success(data))
}

/// `GET /groups/{id}`: one group. As in the PHP application, an id without a group is no
/// error: the answer is `200` and its `data` is `null`.
pub async fn one(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<Option<GroupRow>>, Refusal> {
    caller.require(MANAGER)?;
    let group = group_at(state.database(), &id).await?;
    Ok(Envelope::success(group.map(GroupRow::from)))
}

/// `POST /groups`: adds a group.
///
/// The body must name `group` (the name) and `image`. The application chooses the level.
/// With `default`, the new group is the group that new users join, and the group that
/// was it before is not any more.
pub async fn add(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    if data.is_empty() {
        return Err(Refusal::unprocessable("no data was sent"));
    }
    let database = state.database();
    let fields = Fields::new(&data, RECORD);

    // The checks of the PHP application, in its order.
    let not_supplied = || Refusal::unprocessable("Group name was not supplied");
    let Some(name) = data.field("group") else {
        return Err(not_supplied());
    };
    let name = name_of(database, name, None, not_supplied()).await?;
    let image = match fields.image()? {
        Change::Keep => return Err(Refusal::unprocessable("Group image was not supplied")),
        Change::Set(image) => image.filter(|image| !image.is_empty()),
    }
    .ok_or_else(|| Refusal::unprocessable("Group image cannot be empty"))?;
    let is_default = fields.switch("default")?.or(false);

    // Every install has the group of the Admin, so there is a highest level.
    let level = database
        .groups()
        .highest_level_below_guest()
        .await?
        .and_then(GroupLevel::next_for_a_new_group)
        .ok_or_else(|| Refusal::conflict("Cannot add group as no level is free"))?;
    let new = NewGroup {
        level,
        name: name.clone(),
        image: Some(image),
        is_default,
    };
    match database.groups().insert(new).await {
        Ok(group) => {
            tracing::info!(group = %group.name, id = %group.id, "a group was added");
            Ok(Envelope::success(NoData)
                .with_message("Group added")
                .into_response())
        }
        // Another request took the name or the level after the checks above.
        Err(DbError::Duplicate) => Err(name_taken(&name)),
        Err(error) => Err(error.into()),
    }
}

/// `PUT /groups/{id}`: changes the fields of a group that the body names.
///
/// A field that the application does not know is ignored, and so is `id`. The level
/// cannot change. Making a group the default takes that mark from the group that had it.
/// The mark cannot be taken from a group in another way, so that there always is a group
/// for new users.
pub async fn change(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    // What PHP counts as "no id": nothing, or the text `0`.
    if id.is_empty() || id == "0" {
        return Err(Refusal::unprocessable("id was not set"));
    }
    if data.is_empty() {
        return Err(Refusal::unprocessable("no data was sent"));
    }
    let database = state.database();
    let fields = Fields::new(&data, RECORD);
    // The text is the one of the PHP application, which says "category" here.
    let not_found = || Refusal::not_found("No category info found");
    let group = group_at(database, &id).await?.ok_or_else(not_found)?;

    if data.field("group_id").is_some() {
        return Err(Refusal::conflict("Cannot change group_id"));
    }
    let name = match data.field("group") {
        Some(name) => {
            let empty = Refusal::conflict("Group was set but empty");
            Change::Set(name_of(database, name, Some(group.id), empty).await?)
        }
        None => Change::Keep,
    };
    let image = match fields.image()? {
        Change::Keep => Change::Keep,
        Change::Set(image) => Change::Set(
            image
                .filter(|image| !image.is_empty())
                .ok_or_else(|| Refusal::conflict("Image was set but empty"))?,
        ),
    };
    let is_default = match fields.switch("default")? {
        Change::Keep => Change::Keep,
        Change::Set(_) if group.level.is_fixed() => {
            return Err(Refusal::conflict(format!(
                "Setting {} as default group is not allowed",
                group.name
            )));
        }
        Change::Set(false) if group.is_default => {
            return Err(Refusal::conflict(format!(
                "Cannot remove default from group: {} as a default group is needed",
                group.name
            )));
        }
        Change::Set(is_default) => Change::Set(is_default),
    };

    let changes = GroupChanges {
        name: name.clone(),
        image,
        is_default,
    };
    match database.groups().update(group.id, changes).await {
        Ok(Some(_)) => {
            tracing::info!(group = %group.name, id = %group.id, "a group was changed");
            Ok(Envelope::success(NoData)
                .with_message("Group info updated")
                .into_response())
        }
        // Another request deleted the group after the lookup above.
        Ok(None) => Err(not_found()),
        // Another request took the name after the check above.
        Err(DbError::Duplicate) => Err(name_taken(&name.or(group.name))),
        Err(error) => Err(error.into()),
    }
}

/// `DELETE /groups/{id}`: deletes a group.
///
/// The groups of the Admin and of guests stay, and so does a group that still has users,
/// and the group that new users join.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let database = state.database();
    let not_found = || Refusal::not_found("id not found");
    let group = group_at(database, &id).await?.ok_or_else(not_found)?;
    let has_users =
        || Refusal::conflict("Cannot delete group as group still has users assigned to it");

    if group.level.is_fixed() {
        return Err(Refusal::conflict(format!(
            "Cannot delete group: {} as it is not allowed",
            group.name
        )));
    }
    if database.groups().user_count(group.id).await? > 0 {
        return Err(has_users());
    }
    if group.is_default {
        return Err(Refusal::conflict(format!(
            "Cannot delete group: {} as it is the default group",
            group.name
        )));
    }
    match database.groups().delete(group.id).await {
        Ok(true) => {
            tracing::info!(group = %group.name, id = %group.id, "a group was deleted");
            Ok(NoContent.into_response())
        }
        // Another request deleted the group after the lookup above.
        Ok(false) => Err(not_found()),
        // Another request put a user in the group after the count above.
        Err(DbError::ReferenceViolation) => Err(has_users()),
        Err(error) => Err(error.into()),
    }
}
