//! The bookmark plugin: a page of links in categories, and the routes that manage
//! the links and the categories.
//!
//! The plugin calls a link a tab, as the PHP application does. Its tabs and categories
//! are apart from the tabs and categories of the menu.

use axum::Extension;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use organizr_db::{Database, DbError};
use organizr_domain::email::php_trim;
use organizr_domain::{
    Bookmark, BookmarkCategory, BookmarkCategoryChanges, BookmarkChanges, Change, GroupLevel,
    InvalidMenuName, LevelRange, MENU_NAME_LIMIT, NewBookmark, NewBookmarkCategory, NewTab,
    TabKind, is_hex_color, menu_name,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::{Envelope, NoContent, NoData};
use crate::handlers::fields::{Fields, Refusal, chosen, php_text, place_of, record_id};
use crate::handlers::groups::GroupRow;
use crate::state::AppState;

/// The level that the routes for managing the bookmarks ask for.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The address of the tab of the menu that shows the page of the bookmarks.
pub const PAGE_ADDRESS: &str = "api/v2/plugins/bookmark/page";

/// The word for a bookmark in the texts of the API.
const TAB: &str = "Tab";

/// The word for a category in the texts of the API.
const CATEGORY: &str = "Category";

/// A bookmark as the API shows it, with the field names of the PHP application.
#[derive(Debug, Serialize)]
pub struct BookmarkRow {
    id: i64,
    order: i64,
    category_id: i64,
    name: String,
    url: String,
    enabled: u8,
    group_id: i32,
    image: String,
    background_color: String,
    text_color: String,
}

impl From<Bookmark> for BookmarkRow {
    fn from(bookmark: Bookmark) -> Self {
        Self {
            id: bookmark.id,
            order: bookmark.position,
            category_id: bookmark.category,
            name: bookmark.name,
            url: bookmark.url,
            enabled: u8::from(bookmark.enabled),
            group_id: bookmark.least_access,
            image: bookmark.image,
            background_color: bookmark.background_color,
            text_color: bookmark.text_color,
        }
    }
}

/// A category of bookmarks as the API shows it.
#[derive(Debug, Serialize)]
pub struct BookmarkCategoryRow {
    id: i64,
    order: i64,
    category: String,
    category_id: i64,
    default: u8,
}

impl From<BookmarkCategory> for BookmarkCategoryRow {
    fn from(category: BookmarkCategory) -> Self {
        Self {
            id: category.id,
            order: category.position,
            category: category.name,
            category_id: category.number,
            default: u8::from(category.is_default),
        }
    }
}

/// The plugin answers only when it is switched on.
fn check_enabled(state: &AppState) -> Result<(), Refusal> {
    if state.settings().bookmark_enabled {
        Ok(())
    } else {
        Err(Refusal::conflict("BOOKMARK-enabled module is not enabled"))
    }
}

/// The checks of the routes that manage the bookmarks.
fn check_manager(state: &AppState, caller: &Caller) -> Result<(), Refusal> {
    check_enabled(state)?;
    caller.require(MANAGER)?;
    Ok(())
}

/// A colour of the tile of a bookmark: lighter or darker by a part of what is left to
/// white or to black. `part` runs from -1 to 1.
fn adjusted(color: &str, part: f64) -> String {
    let digits = color.trim_start_matches('#');
    let wide: String = if digits.len() == 3 {
        digits.chars().flat_map(|digit| [digit, digit]).collect()
    } else {
        digits.to_owned()
    };
    let shade = |place: usize| -> u8 {
        let value = wide
            .get(place..place + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .unwrap_or(0);
        let room = if part < 0.0 {
            f64::from(value)
        } else {
            f64::from(255 - value)
        };
        let moved = (f64::from(value) + (room * part).ceil()).clamp(0.0, 255.0);
        // The value is a whole number from 0 to 255 after the clamp.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the value is clamped to the range of a byte"
        )]
        let byte = moved as u8;
        byte
    };
    format!("#{:02x}{:02x}{:02x}", shade(0), shade(2), shade(4))
}

/// `GET /plugins/bookmark/page`: the bookmarks that the caller sees, in their
/// categories. A category without such a bookmark is left out.
///
/// The page is for callers who see the tab of the menu that shows it. Without such a
/// tab, everybody sees it.
pub async fn page(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    check_enabled(&state)?;
    let database = state.database();
    if let Some(tab) = database.tabs().find_by_url(PAGE_ADDRESS).await? {
        caller.require(tab.access.least_access())?;
    }
    let level = i32::from(caller.level().get());
    let bookmarks = database.bookmarks().list().await?;
    let categories: Vec<Value> = database
        .bookmarks()
        .categories()
        .await?
        .into_iter()
        .filter_map(|category| {
            let tabs: Vec<Value> = bookmarks
                .iter()
                .filter(|bookmark| {
                    bookmark.enabled
                        && bookmark.category == category.number
                        && bookmark.least_access >= level
                })
                .map(|bookmark| {
                    json!({
                        "name": bookmark.name,
                        "url": bookmark.url,
                        "image": bookmark.image,
                        "background_color": bookmark.background_color,
                        "text_color": bookmark.text_color,
                        // The tile runs from a darker to a lighter shade of its colour,
                        // with a lighter border.
                        "border_color": adjusted(&bookmark.background_color, 0.3),
                        "dark_color": adjusted(&bookmark.background_color, -0.3),
                        "light_color": adjusted(&bookmark.background_color, 0.1),
                    })
                })
                .collect();
            (!tabs.is_empty()).then(|| json!({ "category": category.name, "tabs": tabs }))
        })
        .collect();
    Ok(Envelope::success(json!({ "categories": categories })))
}

/// `GET /plugins/bookmark/tabs` and `GET /plugins/bookmark/categories`: every
/// bookmark, every category, and the groups, for the editor.
pub async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    check_manager(&state, &caller)?;
    let database = state.database();
    let tabs: Vec<BookmarkRow> = database
        .bookmarks()
        .list()
        .await?
        .into_iter()
        .map(BookmarkRow::from)
        .collect();
    let categories: Vec<BookmarkCategoryRow> = database
        .bookmarks()
        .categories()
        .await?
        .into_iter()
        .map(BookmarkCategoryRow::from)
        .collect();
    let groups: Vec<GroupRow> = database
        .groups()
        .list()
        .await?
        .into_iter()
        .map(GroupRow::from)
        .collect();
    Ok(Envelope::success(json!({
        "tabs": tabs,
        "categories": categories,
        "groups": groups,
    })))
}

/// `GET /plugins/bookmark/tabs/{id}`: one bookmark, for a caller who has its level.
pub async fn one(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<BookmarkRow>, Refusal> {
    check_enabled(&state)?;
    let bookmark = match record_id(&id) {
        Some(id) => state.database().bookmarks().find(id).await?,
        None => None,
    }
    .ok_or_else(|| Refusal::not_found("id not found"))?;
    let level = u16::try_from(bookmark.least_access).unwrap_or(0);
    caller.require(GroupLevel::new(level).unwrap_or(GroupLevel::ADMIN))?;
    Ok(Envelope::success(bookmark.into()))
}

fn name_taken(record: &str, name: &str) -> Refusal {
    Refusal::conflict(format!("{record} name: {name} is already taken"))
}

/// The name of a bookmark or a category from a field of the request.
async fn name_of(
    database: &Database,
    value: &Value,
    record: &'static str,
    field: &str,
    except: Option<i64>,
) -> Result<String, Refusal> {
    let text = php_text(value).ok_or_else(|| Refusal::not_valid(record, field))?;
    let name = menu_name(&text);
    if name == Err(InvalidMenuName::Empty) {
        return Err(Refusal::unprocessable(&format!(
            "{record} name was not supplied"
        )));
    }
    let trimmed = php_trim(&text);
    let taken = if record == TAB {
        database.bookmarks().is_name_taken(trimmed, except).await?
    } else {
        database
            .bookmarks()
            .is_category_name_taken(trimmed, except)
            .await?
    };
    if taken {
        return Err(name_taken(record, trimmed));
    }
    name.map_err(|_| Refusal::over_limit(MENU_NAME_LIMIT))
}

/// A colour from a field of the request, when the request has the field.
fn color_of(data: &ApiData, field: &str, what: &str) -> Result<Option<String>, Refusal> {
    let Some(value) = data.field(field) else {
        return Ok(None);
    };
    let color = php_text(value).map(|text| php_trim(&text).to_owned());
    match color {
        Some(color) if is_hex_color(&color) => Ok(Some(color)),
        _ => Err(Refusal::unprocessable(&format!(
            "Tab {what} color is invalid"
        ))),
    }
}

/// A level from the field `group_id`: a number from 0 to 999.
fn level_of(fields: &Fields<'_>) -> Result<Option<i32>, Refusal> {
    match chosen(fields.number("group_id")?) {
        None => Ok(None),
        Some(level) => i32::try_from(level)
            .ok()
            .filter(|level| (0..=999).contains(level))
            .map(Some)
            .ok_or_else(|| fields.not_valid("group_id")),
    }
}

/// A text that the request has, trimmed. Nothing when the request does not have it.
fn text_of(fields: &Fields<'_>, name: &str) -> Result<Option<String>, Refusal> {
    Ok(match fields.text(name)? {
        Change::Keep => None,
        Change::Set(text) => Some(text.unwrap_or_default()),
    })
}

/// `POST /plugins/bookmark/tabs`: a new bookmark.
pub async fn add(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    if data.is_empty() {
        return Err(Refusal::unprocessable("no data was sent"));
    }
    let database = state.database();
    let fields = Fields::new(&data, TAB);
    let missing = |what: &str| Refusal::unprocessable(&format!("Tab {what} was not supplied"));

    let name = data.field("name").ok_or_else(|| missing("name"))?;
    let name = name_of(database, name, TAB, "name", None).await?;
    let url = text_of(&fields, "url")?.ok_or_else(|| missing("url"))?;
    let image = text_of(&fields, "image")?.ok_or_else(|| missing("image"))?;
    let background_color = color_of(&data, "background_color", "background")?
        .ok_or_else(|| missing("background color"))?;
    let text_color = color_of(&data, "text_color", "text")?.ok_or_else(|| missing("text color"))?;

    let least_access = match level_of(&fields)? {
        Some(level) => level,
        None => database
            .groups()
            .find_default()
            .await?
            .map_or(999, |group| i32::from(group.level.get())),
    };
    let category = match chosen(fields.number("category_id")?) {
        Some(number) => number,
        None => database.bookmarks().default_category().await?.unwrap_or(0),
    };
    let position = match chosen(fields.number("order")?) {
        Some(position) => position,
        None => database.bookmarks().next_position().await?,
    };
    let new = NewBookmark {
        position,
        category,
        name: name.clone(),
        url,
        enabled: fields.switch("enabled")?.or(false),
        least_access,
        image,
        background_color,
        text_color,
    };
    match database.bookmarks().insert(new).await {
        Ok(bookmark) => {
            tracing::info!(bookmark = %bookmark.name, id = bookmark.id, "a bookmark was added");
            Ok(Envelope::success(NoData)
                .with_message("Tab added")
                .into_response())
        }
        Err(DbError::Duplicate) => Err(name_taken(TAB, &name)),
        Err(error) => Err(error.into()),
    }
}

/// `PUT /plugins/bookmark/tabs/{id}`: changes a bookmark.
pub async fn change(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    data: ApiData,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    if id.is_empty() || id == "0" {
        return Err(Refusal::unprocessable("id was not set"));
    }
    if data.is_empty() {
        return Err(Refusal::unprocessable("no data was sent"));
    }
    let database = state.database();
    let fields = Fields::new(&data, TAB);
    let not_found = || Refusal::not_found("No tab info found");
    let bookmark = match record_id(&id) {
        Some(id) => database.bookmarks().find(id).await?,
        None => None,
    }
    .ok_or_else(not_found)?;

    let name = match data.field("name") {
        Some(name) => Some(name_of(database, name, TAB, "name", Some(bookmark.id)).await?),
        None => None,
    };
    let changes = BookmarkChanges {
        position: chosen(fields.number("order")?),
        category: chosen(fields.number("category_id")?),
        name: name.clone(),
        url: text_of(&fields, "url")?,
        enabled: match fields.switch("enabled")? {
            Change::Keep => None,
            Change::Set(enabled) => Some(enabled),
        },
        least_access: level_of(&fields)?,
        image: text_of(&fields, "image")?,
        background_color: color_of(&data, "background_color", "background")?,
        text_color: color_of(&data, "text_color", "text")?,
    };
    match database.bookmarks().update(bookmark.id, changes).await {
        Ok(Some(_)) => {
            tracing::info!(bookmark = %bookmark.name, id = bookmark.id, "a bookmark was changed");
            Ok(Envelope::success(NoData)
                .with_message("Tab info updated")
                .into_response())
        }
        Ok(None) => Err(not_found()),
        Err(DbError::Duplicate) => Err(name_taken(TAB, &name.unwrap_or(bookmark.name))),
        Err(error) => Err(error.into()),
    }
}

/// The places of a request that sets an order: a list of `id` and `order`.
fn places_of(data: &ApiData) -> Result<Vec<(i64, i64)>, Refusal> {
    if data.is_empty() {
        return Err(Refusal::unprocessable("data is empty or not in array"));
    }
    data.entries()
        .map(place_of)
        .collect::<Option<_>>()
        .ok_or_else(|| Refusal::unprocessable("data is malformed"))
}

/// `PUT /plugins/bookmark/tabs`: gives bookmarks new places.
pub async fn reorder(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    let places = places_of(&data)?;
    state.database().bookmarks().set_positions(&places).await?;
    Ok(Envelope::success(NoData)
        .with_message("Tab Order updated")
        .into_response())
}

/// `DELETE /plugins/bookmark/tabs/{id}`: deletes a bookmark.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    let deleted = match record_id(&id) {
        Some(id) => state.database().bookmarks().delete(id).await?,
        None => false,
    };
    if deleted {
        tracing::info!(id, "a bookmark was deleted");
        Ok(NoContent.into_response())
    } else {
        Err(Refusal::not_found("id not found"))
    }
}

/// The number of a category from the field `category_id`, when the request has it and
/// no other category has the number.
async fn number_of(
    database: &Database,
    fields: &Fields<'_>,
    except: Option<i64>,
) -> Result<Option<i64>, Refusal> {
    let Some(number) = chosen(fields.number("category_id")?) else {
        return Ok(None);
    };
    if database
        .bookmarks()
        .is_category_number_taken(number, except)
        .await?
    {
        return Err(Refusal::conflict(format!(
            "Category category_id: {number} is already taken"
        )));
    }
    Ok(Some(number))
}

/// `POST /plugins/bookmark/categories`: a new category.
pub async fn add_category(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    if data.is_empty() {
        return Err(Refusal::unprocessable("no data was sent"));
    }
    let database = state.database();
    let fields = Fields::new(&data, CATEGORY);
    let Some(name) = data.field("category") else {
        return Err(Refusal::unprocessable("Category name was not supplied"));
    };
    let name = name_of(database, name, CATEGORY, "category", None).await?;
    let number = match number_of(database, &fields, None).await? {
        Some(number) => number,
        None => database.bookmarks().next_category_number().await?,
    };
    let position = match chosen(fields.number("order")?) {
        Some(position) => position,
        None => database.bookmarks().next_category_position().await?,
    };
    let new = NewBookmarkCategory {
        number,
        position,
        name: name.clone(),
        is_default: fields.switch("default")?.or(false),
    };
    match database.bookmarks().insert_category(new).await {
        Ok(category) => {
            tracing::info!(category = %category.name, id = category.id, "a bookmark category was added");
            Ok(Envelope::success(NoData)
                .with_message("Category added")
                .into_response())
        }
        Err(DbError::Duplicate) => Err(name_taken(CATEGORY, &name)),
        Err(error) => Err(error.into()),
    }
}

/// `PUT /plugins/bookmark/categories/{id}`: changes a category.
pub async fn change_category(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
    data: ApiData,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    if id.is_empty() || id == "0" {
        return Err(Refusal::unprocessable("id was not set"));
    }
    if data.is_empty() {
        return Err(Refusal::unprocessable("no data was sent"));
    }
    let database = state.database();
    let fields = Fields::new(&data, CATEGORY);
    let not_found = || Refusal::not_found("No category info found");
    let category = match record_id(&id) {
        Some(id) => database.bookmarks().find_category(id).await?,
        None => None,
    }
    .ok_or_else(not_found)?;

    let name = match data.field("category") {
        Some(name) => Some(name_of(database, name, CATEGORY, "category", Some(category.id)).await?),
        None => None,
    };
    let changes = BookmarkCategoryChanges {
        number: number_of(database, &fields, Some(category.id)).await?,
        position: chosen(fields.number("order")?),
        name: name.clone(),
        is_default: match fields.switch("default")? {
            Change::Keep => None,
            Change::Set(is_default) => Some(is_default),
        },
    };
    match database
        .bookmarks()
        .update_category(category.id, changes)
        .await
    {
        Ok(Some(_)) => {
            tracing::info!(category = %category.name, id = category.id, "a bookmark category was changed");
            Ok(Envelope::success(NoData)
                .with_message("Category info updated")
                .into_response())
        }
        Ok(None) => Err(not_found()),
        Err(DbError::Duplicate) => Err(name_taken(CATEGORY, &name.unwrap_or(category.name))),
        Err(error) => Err(error.into()),
    }
}

/// `PUT /plugins/bookmark/categories`: gives categories new places.
pub async fn reorder_categories(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    let places = places_of(&data)?;
    state
        .database()
        .bookmarks()
        .set_category_positions(&places)
        .await?;
    Ok(Envelope::success(NoData)
        .with_message("Category Order updated")
        .into_response())
}

/// `DELETE /plugins/bookmark/categories/{id}`: deletes a category. Its bookmarks
/// stay, and the page does not show them until they have another category.
pub async fn remove_category(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    let deleted = match record_id(&id) {
        Some(id) => state.database().bookmarks().delete_category(id).await?,
        None => false,
    };
    if deleted {
        tracing::info!(id, "a bookmark category was deleted");
        Ok(NoContent.into_response())
    } else {
        Err(Refusal::not_found("id not found"))
    }
}

/// `GET /plugins/bookmark/setup/tab`: makes the tab "Bookmarks" of the menu, which
/// shows the page of the bookmarks, unless a tab for the page exists.
pub async fn set_up_tab(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    let database = state.database();
    if database.tabs().find_by_url(PAGE_ADDRESS).await?.is_some() {
        return Ok(Envelope::success(NoData)
            .with_message("Tab already exists")
            .into_response());
    }
    let failed = || {
        Refusal::Request(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Tab creation error".to_owned(),
        )
    };
    let category = database
        .categories()
        .find_default()
        .await?
        .ok_or_else(failed)?;
    let level = database
        .groups()
        .find_default()
        .await?
        .map_or(GroupLevel::GUEST, |group| group.level);
    let new = NewTab {
        position: database.tabs().next_position().await?,
        category: category.id,
        name: "Bookmarks".to_owned(),
        url: Some(PAGE_ADDRESS.to_owned()),
        url_local: None,
        is_default: false,
        enabled: true,
        access: LevelRange::new(level, GroupLevel::ADMIN).map_err(|_| failed())?,
        add_to_admin: false,
        image: Some("fontawesome::book".to_owned()),
        kind: TabKind::Internal,
        splash: false,
        ping: false,
        ping_url: None,
        timeout: false,
        timeout_ms: None,
        preload: false,
    };
    match database.tabs().insert(new).await {
        Ok(_) => Ok(Envelope::success(NoData)
            .with_message("Tab created")
            .into_response()),
        // A tab of the menu has the name "Bookmarks" already.
        Err(DbError::Duplicate) => Err(Refusal::conflict("Tab name: Bookmarks is already taken")),
        Err(error) => Err(error.into()),
    }
}

/// `GET /plugins/bookmark/setup/category`: makes the category "Unsorted" when there
/// is no category of bookmarks.
pub async fn set_up_category(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Response, Refusal> {
    check_manager(&state, &caller)?;
    let bookmarks = state.database().bookmarks();
    if !bookmarks.categories().await?.is_empty() {
        return Ok(Envelope::success(NoData)
            .with_message("Categories already exists")
            .into_response());
    }
    bookmarks
        .insert_category(NewBookmarkCategory {
            number: 1,
            position: 1,
            name: "Unsorted".to_owned(),
            is_default: true,
        })
        .await?;
    Ok(Envelope::success(NoData)
        .with_message("Category created")
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::adjusted;

    #[test]
    fn colour_gets_lighter_or_darker_by_a_part_of_what_is_left() {
        // 0x80 is 128: 30% of the 127 to white is 39 (rounded up), and 30% of the 128
        // to black is 38 (rounded up towards zero).
        assert_eq!(adjusted("#808080", 0.3), "#a7a7a7");
        assert_eq!(adjusted("#808080", -0.3), "#5a5a5a");
        assert_eq!(adjusted("#fff", 0.3), "#ffffff");
        assert_eq!(adjusted("#000", -0.3), "#000000");
        assert_eq!(adjusted("#f00", 0.1), "#ff1a1a");
        // A text that is no colour is black.
        assert_eq!(adjusted("xyz", 0.0), "#000000");
    }
}
