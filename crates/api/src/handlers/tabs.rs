//! The `/tabs` routes: the tabs of the menu, for the people who manage them.
//!
//! Every route needs an Admin or a Co-Admin. The PHP application stores whatever a
//! request sends, in any field. Here a value must be of the kind its field takes, and a
//! request with another value gets `422`.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_db::{Database, DbError};
use organizr_domain::email::php_trim;
use organizr_domain::{
    Category, CategoryNumber, Change, Group, GroupLevel, InvalidTabName, LevelRange, NewTab, Tab,
    TabChanges, TabId, TabKind, tab_name,
};
use serde::Serialize;
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::{Caller, Identity};
use crate::cookies::cookie_value;
use crate::envelope::{Envelope, NoContent, NoData};
use crate::error::ApiError;
use crate::state::AppState;

/// The level a caller needs for the tab routes.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// A tab as the API shows it: the columns of the PHP application, in their order.
/// Switches are the numbers 0 and 1.
#[derive(Debug, Serialize)]
pub struct TabRow {
    id: i64,
    order: i64,
    /// The number of the category, not its `id`.
    category_id: Option<i64>,
    name: String,
    url: Option<String>,
    url_local: Option<String>,
    default: u8,
    enabled: u8,
    group_id: u16,
    group_id_max: u16,
    add_to_admin: u8,
    image: Option<String>,
    r#type: u8,
    splash: u8,
    ping: u8,
    ping_url: Option<String>,
    timeout: u8,
    timeout_ms: Option<i64>,
    preload: u8,
}

impl TabRow {
    /// Shows a tab. `categories` gives the number of its category.
    pub fn of(tab: Tab, categories: &[Category]) -> Self {
        let category = categories
            .iter()
            .find(|category| category.id == tab.category)
            .map(|category| category.number.get());
        Self {
            id: tab.id.get(),
            order: tab.position,
            category_id: category,
            name: tab.name,
            url: tab.url,
            url_local: tab.url_local,
            default: u8::from(tab.is_default),
            enabled: u8::from(tab.enabled),
            group_id: tab.access.least_access().get(),
            group_id_max: tab.access.most_access().get(),
            add_to_admin: u8::from(tab.add_to_admin),
            image: tab.image,
            r#type: tab.kind.code(),
            splash: u8::from(tab.splash),
            ping: u8::from(tab.ping),
            ping_url: tab.ping_url,
            timeout: u8::from(tab.timeout),
            timeout_ms: tab.timeout_ms,
            preload: u8::from(tab.preload),
        }
    }

    /// The address of the tab.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// The tab with the placeholders of its two addresses filled in.
    #[must_use]
    pub fn filled_in(mut self, variables: &TabVariables) -> Self {
        self.url = variables.fill_in(self.url.as_deref());
        self.url_local = variables.fill_in(self.url_local.as_deref());
        self
    }
}

/// What stands for the placeholders in the address of a tab. An address such as
/// `https://{domain}/user/{username}` is filled in for the caller of the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabVariables {
    /// Each placeholder with its value.
    values: [(&'static str, String); 7],
}

impl TabVariables {
    /// The values for the caller of a request.
    ///
    /// `{username_lower}` gets the name as it is, not in lower case, as in the PHP
    /// application.
    pub fn of(caller: &Caller, headers: &HeaderMap) -> Self {
        let (username, email, group, level) = match &caller.identity {
            // A script with the API key is the user "Organizr API" of the group Admin.
            _ if caller.has_api_key => (
                "Organizr API".to_owned(),
                String::new(),
                "Admin".to_owned(),
                GroupLevel::ADMIN,
            ),
            Identity::User(user) => (
                user.authenticated.user.username.clone(),
                user.authenticated.user.email.clone().unwrap_or_default(),
                user.authenticated.group.name.clone(),
                user.authenticated.group.level,
            ),
            Identity::Guest | Identity::Rejected => (
                "Guest".to_owned(),
                String::new(),
                "Guest".to_owned(),
                GroupLevel::GUEST,
            ),
        };
        let header = |name| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_owned()
        };
        Self {
            values: [
                ("{domain}", header(header::HOST)),
                ("{username}", username.clone()),
                ("{username_lower}", username),
                ("{email}", email),
                ("{group}", group),
                ("{group_id}", level.get().to_string()),
                (
                    "{komga}",
                    cookie_value(headers, "komga_token")
                        .unwrap_or_default()
                        .to_owned(),
                ),
            ],
        }
    }

    /// Fills in the placeholders of an address. Each one is replaced once: a value that
    /// holds a placeholder is not filled in again. An address that is empty becomes
    /// `None`, as in the PHP application.
    pub fn fill_in(&self, address: Option<&str>) -> Option<String> {
        let mut rest = address.filter(|address| !address.is_empty())?;
        let mut filled = String::with_capacity(rest.len());
        while let Some(start) = rest.find('{') {
            let (before, from_brace) = rest.split_at(start);
            filled.push_str(before);
            let known = self
                .values
                .iter()
                .find(|(placeholder, _)| from_brace.starts_with(placeholder));
            if let Some((placeholder, value)) = known {
                filled.push_str(value);
                rest = from_brace.get(placeholder.len()..).unwrap_or_default();
            } else {
                filled.push('{');
                rest = from_brace.get(1..).unwrap_or_default();
            }
        }
        filled.push_str(rest);
        Some(filled)
    }
}

/// A category as the API shows it.
#[derive(Debug, Serialize)]
pub struct CategoryRow {
    id: i64,
    order: i64,
    category: String,
    category_id: i64,
    image: Option<String>,
    default: u8,
}

impl From<&Category> for CategoryRow {
    fn from(category: &Category) -> Self {
        Self {
            id: category.id.get(),
            order: category.position,
            category: category.name.clone(),
            category_id: category.number.get(),
            image: category.image.clone(),
            default: u8::from(category.is_default),
        }
    }
}

/// A group as the API shows it.
#[derive(Debug, Serialize)]
struct GroupRow {
    id: i64,
    group: String,
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

/// The `data` of `GET /tabs`: what the tab editor shows.
#[derive(Debug, Serialize)]
pub struct TabEditorData {
    tabs: Vec<TabRow>,
    categories: Vec<CategoryRow>,
    groups: Vec<GroupRow>,
}

/// Why a tab request is refused.
#[derive(Debug)]
pub enum Refusal {
    /// The request is at fault. The caller gets the status and the text.
    Request(StatusCode, String),
    /// Any other failure.
    Api(ApiError),
}

impl Refusal {
    fn unprocessable(message: &str) -> Self {
        Self::Request(StatusCode::UNPROCESSABLE_ENTITY, message.to_owned())
    }

    /// A value that is not of the kind the field takes.
    fn not_valid(field: &str) -> Self {
        Self::unprocessable(&format!("Tab {field} is not valid"))
    }

    fn name_taken(name: &str) -> Self {
        Self::Request(
            StatusCode::CONFLICT,
            format!("Tab name: {name} is already taken"),
        )
    }
}

impl From<ApiError> for Refusal {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

impl From<DbError> for Refusal {
    fn from(error: DbError) -> Self {
        Self::Api(error.into())
    }
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        match self {
            Self::Request(status, message) => {
                Envelope::error(status, message, NoData).into_response()
            }
            Self::Api(error) => error.into_response(),
        }
    }
}

/// Gives an answer the content type `application/json`, without a character set. The
/// PHP application sends this form from the routes that change or delete a tab.
pub async fn as_plain_json(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

/// A whole number as a request can send it: a JSON number, or its digits as text.
fn whole_number(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => php_trim(text).parse().ok(),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

/// The whole number in a field. `null` and the empty text stand for "nothing".
fn number_of(data: &ApiData, name: &str) -> Result<Change<Option<i64>>, Refusal> {
    match data.field(name) {
        None => Ok(Change::Keep),
        Some(Value::Null) => Ok(Change::Set(None)),
        Some(Value::String(text)) if php_trim(text).is_empty() => Ok(Change::Set(None)),
        Some(value) => whole_number(value)
            .map(|number| Change::Set(Some(number)))
            .ok_or_else(|| Refusal::not_valid(name)),
    }
}

/// The switch in a field: `true` or `false`, 1 or 0, or one of these as text. `null`
/// stands for "off".
fn switch_of(data: &ApiData, name: &str) -> Result<Change<bool>, Refusal> {
    let on = match data.field(name) {
        None => return Ok(Change::Keep),
        Some(Value::Null) => Some(false),
        Some(Value::Bool(on)) => Some(*on),
        Some(Value::Number(number)) => match number.as_i64() {
            Some(0) => Some(false),
            Some(1) => Some(true),
            _ => None,
        },
        Some(Value::String(text)) => match text.as_str() {
            "0" | "false" => Some(false),
            "1" | "true" => Some(true),
            _ => None,
        },
        Some(Value::Array(_) | Value::Object(_)) => None,
    };
    on.map(Change::Set).ok_or_else(|| Refusal::not_valid(name))
}

/// A value as text, as PHP makes text from it: a number becomes its digits, `true`
/// becomes `1`, and `false` and `null` become the empty text. A list or an object is no
/// text.
fn php_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(true) => Some("1".to_owned()),
        Value::Bool(false) | Value::Null => Some(String::new()),
        Value::Array(_) | Value::Object(_) => None,
    }
}

/// The text in a field, as it was sent. `null` stands for "nothing".
fn text_of(data: &ApiData, name: &str) -> Result<Change<Option<String>>, Refusal> {
    match data.field(name) {
        None => Ok(Change::Keep),
        Some(Value::Null) => Ok(Change::Set(None)),
        Some(value) => php_text(value)
            .map(|text| Change::Set(Some(text)))
            .ok_or_else(|| Refusal::not_valid(name)),
    }
}

/// The picture in the field `image`: the text without the space around it. PHP stores
/// the empty text for `null`.
fn image_of(data: &ApiData) -> Result<Change<Option<String>>, Refusal> {
    match data.field("image") {
        None => Ok(Change::Keep),
        Some(value) => php_text(value)
            .map(|text| Change::Set(Some(php_trim(&text).to_owned())))
            .ok_or_else(|| Refusal::not_valid("image")),
    }
}

/// The group level in a field.
fn level_of(data: &ApiData, name: &str) -> Result<Change<Option<GroupLevel>>, Refusal> {
    Ok(match number_of(data, name)? {
        Change::Keep => Change::Keep,
        Change::Set(None) => Change::Set(None),
        Change::Set(Some(number)) => Change::Set(Some(
            GroupLevel::try_from(number).map_err(|_| Refusal::not_valid(name))?,
        )),
    })
}

/// How the tab opens, from the field `type`.
fn kind_of(data: &ApiData) -> Result<Change<Option<TabKind>>, Refusal> {
    Ok(match number_of(data, "type")? {
        Change::Keep => Change::Keep,
        Change::Set(None) => Change::Set(None),
        Change::Set(Some(number)) => Change::Set(Some(
            TabKind::try_from(number).map_err(|_| Refusal::not_valid("type"))?,
        )),
    })
}

/// A change to a field that cannot be empty: "nothing" is no value for it.
fn required<T>(change: Change<Option<T>>, name: &str) -> Result<Change<T>, Refusal> {
    match change {
        Change::Keep => Ok(Change::Keep),
        Change::Set(Some(value)) => Ok(Change::Set(value)),
        Change::Set(None) => Err(Refusal::not_valid(name)),
    }
}

/// The value a new record gets from a field, or `None` when the request leaves the
/// choice to the application: the field is missing or "nothing".
fn chosen<T>(change: Change<Option<T>>) -> Option<T> {
    change.into_option().flatten()
}

/// The name for a tab from the field `name`, checked as the PHP application checks it:
/// the space around it is cut off, no other tab has it, and it has at most 50 bytes.
/// `except` is the tab that gets the name, which can keep the name it has.
///
/// A name that is empty is refused. PHP stores a tab without a name.
async fn name_of(
    database: &Database,
    value: &Value,
    except: Option<TabId>,
) -> Result<String, Refusal> {
    let text = php_text(value).ok_or_else(|| Refusal::not_valid("name"))?;
    let name = tab_name(&text);
    if name == Err(InvalidTabName::Empty) {
        return Err(Refusal::unprocessable("Tab name was not supplied"));
    }
    let trimmed = php_trim(&text);
    if database.tabs().is_name_taken(trimmed, except).await? {
        return Err(Refusal::name_taken(trimmed));
    }
    name.map_err(|_| {
        Refusal::Request(
            StatusCode::CONFLICT,
            format!(
                "String is over limit of: {}",
                organizr_domain::TAB_NAME_LIMIT
            ),
        )
    })
}

/// The category that the field `category_id` names by its number.
async fn category_of(
    database: &Database,
    data: &ApiData,
) -> Result<Change<Option<Category>>, Refusal> {
    Ok(match number_of(data, "category_id")? {
        Change::Keep => Change::Keep,
        Change::Set(None) => Change::Set(None),
        Change::Set(Some(number)) => Change::Set(Some(
            database
                .categories()
                .find_by_number(CategoryNumber::new(number))
                .await?
                .ok_or_else(|| Refusal::not_valid("category_id"))?,
        )),
    })
}

/// The id of a tab from the path. Text that is no whole number names no tab.
fn tab_id(text: &str) -> Option<TabId> {
    text.parse().ok().map(TabId::new)
}

/// `GET /tabs`: all tabs, all categories and all groups, for the tab editor.
///
/// The placeholders in the addresses of the tabs are filled in for the caller, and an
/// empty address is `null`. `GET /tabs/{id}` shows the addresses as they are stored.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    headers: HeaderMap,
) -> Result<Envelope<TabEditorData>, ApiError> {
    caller.require(MANAGER)?;
    let database = state.database();
    let categories = database.categories().list().await?;
    let variables = TabVariables::of(&caller, &headers);
    let tabs = database
        .tabs()
        .list()
        .await?
        .into_iter()
        .map(|tab| TabRow::of(tab, &categories).filled_in(&variables))
        .collect();
    let groups = database
        .groups()
        .list()
        .await?
        .into_iter()
        .map(GroupRow::from)
        .collect();
    Ok(Envelope::success(TabEditorData {
        tabs,
        categories: categories.iter().map(CategoryRow::from).collect(),
        groups,
    }))
}

/// `GET /tabs/{id}`: one tab. The caller must also have the level that sees the tab.
pub async fn one(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    caller.require(MANAGER)?;
    let database = state.database();
    let tab = match tab_id(&id) {
        Some(id) => database.tabs().find(id).await?,
        None => None,
    };
    let Some(tab) = tab else {
        return Ok(Envelope::error(StatusCode::NOT_FOUND, "id not found", false).into_response());
    };
    caller.require(tab.access.least_access())?;
    let categories = database.categories().list().await?;
    Ok(Envelope::success(TabRow::of(tab, &categories)).into_response())
}

/// `POST /tabs`: adds a tab.
///
/// The body must name `name`, `image`, and `url` or `url_local`. What it does not name
/// gets a default: the default group, the default category, the place after the last
/// tab, the type 1 (a frame), and every switch off.
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

    // The checks of the PHP application, in its order.
    let Some(name) = data.field("name") else {
        return Err(Refusal::unprocessable("Tab name was not supplied"));
    };
    let name = name_of(database, name, None).await?;
    if data.field("url").is_none() && data.field("url_local").is_none() {
        return Err(Refusal::unprocessable(
            "Tab url or url_local was not supplied",
        ));
    }
    let Change::Set(image) = image_of(&data)? else {
        return Err(Refusal::unprocessable("Tab image was not supplied"));
    };

    let least_access = match chosen(level_of(&data, "group_id")?) {
        Some(level) => level,
        // Without a default group, only Admins see the tab.
        None => database
            .groups()
            .find_default()
            .await?
            .map_or(GroupLevel::ADMIN, |group| group.level),
    };
    let most_access = chosen(level_of(&data, "group_id_max")?).unwrap_or(GroupLevel::ADMIN);
    let access = LevelRange::new(least_access, most_access).map_err(|_| {
        Refusal::Request(
            StatusCode::CONFLICT,
            format!("Tab name: {name} cannot have a lower Group Id Max than Group Id Min"),
        )
    })?;
    let category = match chosen(category_of(database, &data).await?) {
        Some(category) => category,
        None => database
            .categories()
            .find_default()
            .await?
            .ok_or_else(|| Refusal::not_valid("category_id"))?,
    };
    let position = match chosen(number_of(&data, "order")?) {
        Some(position) => position,
        None => database.tabs().next_position().await?,
    };
    let switch = |name| switch_of(&data, name).map(|change| change.or(false));

    let new = NewTab {
        position,
        category: category.id,
        name: name.clone(),
        url: chosen(text_of(&data, "url")?),
        url_local: chosen(text_of(&data, "url_local")?),
        is_default: switch("default")?,
        enabled: switch("enabled")?,
        access,
        add_to_admin: switch("add_to_admin")?,
        image,
        kind: chosen(kind_of(&data)?).unwrap_or(TabKind::Iframe),
        splash: switch("splash")?,
        ping: switch("ping")?,
        ping_url: chosen(text_of(&data, "ping_url")?),
        timeout: switch("timeout")?,
        timeout_ms: chosen(number_of(&data, "timeout_ms")?),
        preload: switch("preload")?,
    };
    match database.tabs().insert(new).await {
        Ok(tab) => {
            tracing::info!(tab = %tab.name, id = %tab.id, "a tab was added");
            Ok(Envelope::success(NoData)
                .with_message("Tab added")
                .into_response())
        }
        // Another request took the name after the check above.
        Err(DbError::Duplicate) => Err(Refusal::name_taken(&name)),
        Err(error) => Err(error.into()),
    }
}

/// The new range of levels of a tab, from the fields `group_id` and `group_id_max`. The
/// checks and their texts are those of the PHP application.
fn access_of(data: &ApiData, tab: &Tab) -> Result<Change<LevelRange>, Refusal> {
    let least_access = required(level_of(data, "group_id")?, "group_id")?;
    let most_access = required(level_of(data, "group_id_max")?, "group_id_max")?;
    let conflict = |what: &str| {
        Refusal::Request(
            StatusCode::CONFLICT,
            format!("Tab name: {} cannot have a {what}", tab.name),
        )
    };
    let lower_maximum = "lower Group Id Max than Group Id Min";

    if let Change::Set(least_access) = least_access
        && least_access < most_access.or(tab.access.most_access())
    {
        return Err(conflict(lower_maximum));
    }
    if let Change::Set(most_access) = most_access
        && most_access > least_access.or(tab.access.least_access())
    {
        return Err(conflict("higher Group Id Min than Group Id Max"));
    }
    if least_access == Change::Keep && most_access == Change::Keep {
        return Ok(Change::Keep);
    }
    LevelRange::new(
        least_access.or(tab.access.least_access()),
        most_access.or(tab.access.most_access()),
    )
    .map(Change::Set)
    .map_err(|_| conflict(lower_maximum))
}

/// `PUT /tabs/{id}`: changes the fields of a tab that the body names.
///
/// A field that the application does not know is ignored, and so is `id`. Making a tab
/// the default takes that mark from every other tab.
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
    let not_found = || Refusal::Request(StatusCode::NOT_FOUND, "No tab info found".to_owned());
    let tab = match tab_id(&id) {
        Some(id) => database.tabs().find(id).await?,
        None => None,
    }
    .ok_or_else(not_found)?;

    let name = match data.field("name") {
        Some(name) => Change::Set(name_of(database, name, Some(tab.id)).await?),
        None => Change::Keep,
    };
    let category = required(category_of(database, &data).await?, "category_id")?;
    let changes = TabChanges {
        position: required(number_of(&data, "order")?, "order")?,
        category: match category {
            Change::Set(category) => Change::Set(category.id),
            Change::Keep => Change::Keep,
        },
        name: name.clone(),
        url: text_of(&data, "url")?,
        url_local: text_of(&data, "url_local")?,
        is_default: switch_of(&data, "default")?,
        enabled: switch_of(&data, "enabled")?,
        access: access_of(&data, &tab)?,
        add_to_admin: switch_of(&data, "add_to_admin")?,
        image: image_of(&data)?,
        kind: required(kind_of(&data)?, "type")?,
        splash: switch_of(&data, "splash")?,
        ping: switch_of(&data, "ping")?,
        ping_url: text_of(&data, "ping_url")?,
        timeout: switch_of(&data, "timeout")?,
        timeout_ms: number_of(&data, "timeout_ms")?,
        preload: switch_of(&data, "preload")?,
    };

    match database.tabs().update(tab.id, changes).await {
        Ok(Some(_)) => {
            tracing::info!(tab = %tab.name, id = %tab.id, "a tab was changed");
            Ok(Envelope::success(NoData)
                .with_message("Tab info updated")
                .into_response())
        }
        // Another request deleted the tab after the lookup above.
        Ok(None) => Err(not_found()),
        // Another request took the name after the check above.
        Err(DbError::Duplicate) => Err(Refusal::name_taken(&name.or(tab.name))),
        Err(error) => Err(error.into()),
    }
}

/// One entry of the body of `PUT /tabs`: an object with exactly the keys `id` and
/// `order`. PHP takes 0 for "no value", so neither can be 0.
fn place_of(entry: &Value) -> Option<(TabId, i64)> {
    let entry = entry.as_object()?;
    if entry.len() != 2 {
        return None;
    }
    let id = whole_number(entry.get("id")?)?;
    let order = whole_number(entry.get("order")?)?;
    (id != 0 && order != 0).then_some((TabId::new(id), order))
}

/// `PUT /tabs`: gives tabs new places in the menu.
///
/// The body is a list of objects, each with the `id` of a tab and its new `order`. An
/// id without a tab changes nothing. When one entry is not in this form, no place is
/// changed.
pub async fn reorder(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    if data.is_empty() {
        return Err(Refusal::unprocessable("data is empty or not in array"));
    }
    let places: Vec<(TabId, i64)> = data
        .entries()
        .map(place_of)
        .collect::<Option<_>>()
        .ok_or_else(|| Refusal::unprocessable("data is malformed"))?;
    state.database().tabs().set_positions(&places).await?;
    tracing::info!(tabs = places.len(), "the order of the tabs was changed");
    Ok(Envelope::success(NoData)
        .with_message("Tab Order updated")
        .into_response())
}

/// `DELETE /tabs/{id}`: deletes a tab.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let existed = match tab_id(&id) {
        Some(id) => state.database().tabs().delete(id).await?,
        None => false,
    };
    if existed {
        tracing::info!(id = %id, "a tab was deleted");
        Ok(NoContent.into_response())
    } else {
        Err(Refusal::Request(
            StatusCode::NOT_FOUND,
            "id not found".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use organizr_domain::TabId;
    use serde_json::{Value, json};

    use super::{TabVariables, php_text, place_of, tab_id, whole_number};

    fn variables() -> TabVariables {
        TabVariables {
            values: [
                ("{domain}", "organizr.example.com".to_owned()),
                ("{username}", "Zoe".to_owned()),
                ("{username_lower}", "Zoe".to_owned()),
                ("{email}", "zoe@example.com".to_owned()),
                ("{group}", "User".to_owned()),
                ("{group_id}", "4".to_owned()),
                ("{komga}", String::new()),
            ],
        }
    }

    #[test]
    fn placeholders_of_an_address_are_filled_in() {
        let fill_in = |address| variables().fill_in(Some(address));

        assert_eq!(
            fill_in("https://{domain}/{username}?mail={email}&group={group}&level={group_id}")
                .as_deref(),
            Some("https://organizr.example.com/Zoe?mail=zoe@example.com&group=User&level=4")
        );
        // The same one twice, one with an empty value, and two without a gap.
        assert_eq!(
            fill_in("{group_id}{group_id}/{komga}/{username_lower}").as_deref(),
            Some("44//Zoe")
        );
        assert_eq!(
            fill_in("https://plex.example.com/").as_deref(),
            Some("https://plex.example.com/")
        );
    }

    #[test]
    fn text_in_braces_that_is_no_placeholder_stays() {
        let fill_in = |address| variables().fill_in(Some(address));

        assert_eq!(fill_in("/{other}/{").as_deref(), Some("/{other}/{"));
        assert_eq!(fill_in("{{group}}").as_deref(), Some("{User}"));
        assert_eq!(fill_in("{Group}/{group").as_deref(), Some("{Group}/{group"));
        assert_eq!(fill_in("{é}{group_id}").as_deref(), Some("{é}4"));
    }

    #[test]
    fn value_that_holds_a_placeholder_is_not_filled_in_again() {
        let mut variables = variables();
        variables.values[1].1 = "{group}".to_owned();

        assert_eq!(
            variables.fill_in(Some("/{username}/{group}")).as_deref(),
            Some("/{group}/User")
        );
    }

    #[test]
    fn empty_address_becomes_none() {
        assert_eq!(variables().fill_in(Some("")), None);
        assert_eq!(variables().fill_in(None), None);
    }

    #[test]
    fn whole_number_is_a_json_number_or_its_digits_as_text() {
        assert_eq!(whole_number(&json!(5)), Some(5));
        assert_eq!(whole_number(&json!(-3)), Some(-3));
        assert_eq!(whole_number(&json!("12")), Some(12));
        assert_eq!(whole_number(&json!(" 12 ")), Some(12));
        for other in [
            json!(1.5),
            json!("1.5"),
            json!("abc"),
            json!(""),
            json!(true),
            Value::Null,
            json!([1]),
            json!({"a": 1}),
        ] {
            assert_eq!(whole_number(&other), None, "{other}");
        }
    }

    #[test]
    fn text_is_made_from_a_value_as_php_makes_it() {
        assert_eq!(php_text(&json!(" a ")).as_deref(), Some(" a "));
        assert_eq!(php_text(&json!(5)).as_deref(), Some("5"));
        assert_eq!(php_text(&json!(true)).as_deref(), Some("1"));
        assert_eq!(php_text(&json!(false)).as_deref(), Some(""));
        assert_eq!(php_text(&Value::Null).as_deref(), Some(""));
        assert_eq!(php_text(&json!(["a"])), None);
        assert_eq!(php_text(&json!({"a": 1})), None);
    }

    #[test]
    fn id_in_a_path_is_a_whole_number() {
        assert_eq!(tab_id("3"), Some(TabId::new(3)));
        // SQLite, which the PHP application uses, reads these as 1 too.
        assert_eq!(tab_id("01"), Some(TabId::new(1)));
        for other in ["", "abc", "1abc", "1.5", " 1"] {
            assert_eq!(tab_id(other), None, "{other:?}");
        }
    }

    #[test]
    fn place_is_an_object_with_an_id_and_an_order() {
        let place = |entry: Value| place_of(&entry).map(|(id, order)| (id.get(), order));

        assert_eq!(place(json!({"id": 1, "order": 5})), Some((1, 5)));
        assert_eq!(place(json!({"order": "6", "id": "2"})), Some((2, 6)));
        assert_eq!(place(json!({"id": 1, "order": -2})), Some((1, -2)));
        for malformed in [
            json!({"id": 1}),
            json!({"id": 1, "order": 5, "name": "x"}),
            json!({"id": 1, "place": 5}),
            // PHP takes 0 for "no value".
            json!({"id": 1, "order": 0}),
            json!({"id": 0, "order": 5}),
            json!({"id": 1, "order": "first"}),
            json!({"id": 1, "order": null}),
            json!([1, 5]),
            json!(1),
            json!("1"),
            Value::Null,
        ] {
            assert_eq!(place(malformed.clone()), None, "{malformed}");
        }
    }
}
