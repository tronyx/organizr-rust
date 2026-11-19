//! The `/categories` routes: the headings of the menu, for the people who manage them.
//!
//! Every route needs an Admin or a Co-Admin. `GET /categories` is the same as
//! `GET /tabs`, and the router sends it there. As with tabs, a value must be of the kind
//! its field takes. `docs/deviations.md` lists the differences from the PHP application.

use axum::Extension;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use organizr_db::{Database, DbError};
use organizr_domain::email::php_trim;
use organizr_domain::{
    CategoryChanges, CategoryId, CategoryNumber, Change, GroupLevel, InvalidMenuName,
    MENU_NAME_LIMIT, NewCategory, menu_name,
};
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::{Envelope, NoContent, NoData};
use crate::handlers::fields::{Fields, Refusal, chosen, php_text, place_of, record_id};
use crate::state::AppState;

/// The level a caller needs for the category routes.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The word for the record in the text of a refusal.
const RECORD: &str = "Category";

/// The refusal for a name that another category has.
fn name_taken(name: &str) -> Refusal {
    Refusal::conflict(format!("Category name: {name} is already taken"))
}

/// The refusal for a number that another category has. The PHP application has no such
/// answer: it lets two categories share a number.
fn number_taken(number: CategoryNumber) -> Refusal {
    Refusal::conflict(format!("Category category_id: {number} is already taken"))
}

/// The id of a category from the path.
fn category_id(text: &str) -> Option<CategoryId> {
    record_id(text).map(CategoryId::new)
}

/// The name for a category from the field `category`, checked as the PHP application
/// checks it: the space around it is cut off, no other category has it, and it has at
/// most 50 bytes. `except` is the category that gets the name, which can keep the name
/// it has.
///
/// A name that is empty is refused. PHP stores a category without a name.
async fn name_of(
    database: &Database,
    value: &Value,
    except: Option<CategoryId>,
) -> Result<String, Refusal> {
    let text = php_text(value).ok_or_else(|| Refusal::not_valid(RECORD, "category"))?;
    let name = menu_name(&text);
    if name == Err(InvalidMenuName::Empty) {
        return Err(Refusal::unprocessable("Category name was not supplied"));
    }
    let trimmed = php_trim(&text);
    if database.categories().is_name_taken(trimmed, except).await? {
        return Err(name_taken(trimmed));
    }
    name.map_err(|_| Refusal::over_limit(MENU_NAME_LIMIT))
}

/// The number for a category from the field `category_id`. No other category can have
/// it. `except` is the category that gets the number.
async fn number_of(
    database: &Database,
    fields: &Fields<'_>,
    except: Option<CategoryId>,
) -> Result<Change<Option<CategoryNumber>>, Refusal> {
    Ok(match fields.number("category_id")? {
        Change::Keep => Change::Keep,
        Change::Set(None) => Change::Set(None),
        Change::Set(Some(number)) => {
            let number = CategoryNumber::new(number);
            if database
                .categories()
                .is_number_taken(number, except)
                .await?
            {
                return Err(number_taken(number));
            }
            Change::Set(Some(number))
        }
    })
}

/// `POST /categories`: adds a category.
///
/// The body must name `category` (the name) and `image`. What it does not name gets a
/// default: the place after the last category, the number after the highest number, and
/// not the default category.
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
    let Some(name) = data.field("category") else {
        return Err(Refusal::unprocessable("Category name was not supplied"));
    };
    let name = name_of(database, name, None).await?;
    let Change::Set(image) = fields.image()? else {
        return Err(Refusal::unprocessable("Category image was not supplied"));
    };

    let number = match chosen(number_of(database, &fields, None).await?) {
        Some(number) => number,
        None => database.categories().next_number().await?,
    };
    let position = match chosen(fields.number("order")?) {
        Some(position) => position,
        None => database.categories().next_position().await?,
    };
    let new = NewCategory {
        number,
        position,
        name: name.clone(),
        image,
        is_default: fields.switch("default")?.or(false),
    };
    match database.categories().insert(new).await {
        Ok(category) => {
            tracing::info!(category = %category.name, id = %category.id, "a category was added");
            Ok(Envelope::success(NoData)
                .with_message("Category added")
                .into_response())
        }
        // Another request took the name or the number after the checks above.
        Err(DbError::Duplicate) => Err(name_taken(&name)),
        Err(error) => Err(error.into()),
    }
}

/// `PUT /categories/{id}`: changes the fields of a category that the body names.
///
/// A field that the application does not know is ignored, and so is `id`. Making a
/// category the default takes that mark from every other category.
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
    let not_found = || Refusal::not_found("No category info found");
    let category = match category_id(&id) {
        Some(id) => database.categories().find(id).await?,
        None => None,
    }
    .ok_or_else(not_found)?;

    let name = match data.field("category") {
        Some(name) => Change::Set(name_of(database, name, Some(category.id)).await?),
        None => Change::Keep,
    };
    let number = number_of(database, &fields, Some(category.id)).await?;
    let changes = CategoryChanges {
        number: fields.required(number, "category_id")?,
        position: fields.required(fields.number("order")?, "order")?,
        name: name.clone(),
        image: fields.image()?,
        is_default: fields.switch("default")?,
    };

    match database.categories().update(category.id, changes).await {
        Ok(Some(_)) => {
            tracing::info!(category = %category.name, id = %category.id, "a category was changed");
            Ok(Envelope::success(NoData)
                .with_message("Category info updated")
                .into_response())
        }
        // Another request deleted the category after the lookup above.
        Ok(None) => Err(not_found()),
        // Another request took the name or the number after the checks above.
        Err(DbError::Duplicate) => Err(name_taken(&name.or(category.name))),
        Err(error) => Err(error.into()),
    }
}

/// `PUT /categories`: gives categories new places in the menu.
///
/// The body is a list of objects, each with the `id` of a category and its new `order`.
/// An id without a category changes nothing. When one entry is not in this form, no
/// place is changed.
pub async fn reorder(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    if data.is_empty() {
        return Err(Refusal::unprocessable("data is empty or not in array"));
    }
    let places: Vec<(CategoryId, i64)> = data
        .entries()
        .map(|entry| place_of(entry).map(|(id, order)| (CategoryId::new(id), order)))
        .collect::<Option<_>>()
        .ok_or_else(|| Refusal::unprocessable("data is malformed"))?;
    state.database().categories().set_positions(&places).await?;
    tracing::info!(
        categories = places.len(),
        "the order of the categories was changed"
    );
    Ok(Envelope::success(NoData)
        .with_message("Category Order updated")
        .into_response())
}

/// `DELETE /categories/{id}`: deletes a category.
///
/// A category that still has tabs is not deleted: the answer is `409`. The PHP
/// application deletes it and leaves its tabs without a category.
pub async fn remove(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    caller.require(MANAGER)?;
    let database = state.database();
    let not_found = || Refusal::not_found("id not found");
    let category = match category_id(&id) {
        Some(id) => database.categories().find(id).await?,
        None => None,
    }
    .ok_or_else(not_found)?;

    match database.categories().delete(category.id).await {
        Ok(true) => {
            tracing::info!(category = %category.name, id = %category.id, "a category was deleted");
            Ok(NoContent.into_response())
        }
        // Another request deleted the category after the lookup above.
        Ok(false) => Err(not_found()),
        Err(DbError::ReferenceViolation) => Err(Refusal::conflict(format!(
            "Category name: {} still has tabs",
            category.name
        ))),
        Err(error) => Err(error.into()),
    }
}
