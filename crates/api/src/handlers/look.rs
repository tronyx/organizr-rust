//! `PUT /user/look`: the layout, theme and color set that the caller chose (D-125).
//!
//! Not a route of the PHP application, which has one theme for the install.

use axum::Extension;
use axum::extract::State;
use organizr_db::StoredLook;
use organizr_domain::GroupLevel;
use organizr_settings::Settings;
use serde_json::Value;

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::look::{
    Look, MODERN_THEMES, OG_STYLES, OG_THEMES, default_modern_theme, effective_look,
    modern_color_sets,
};
use crate::state::AppState;

/// What the caller asked for.
enum Request {
    /// Forget everything that the caller chose.
    Clear,
    /// Store the layout, and its theme and color set. `None` means the default.
    Choose {
        layout: &'static str,
        theme: Option<String>,
        color_set: Option<String>,
    },
    /// Store the layout only: the body names no theme and no color set, so what the
    /// caller last chose for that layout stays.
    Switch { layout: &'static str },
}

/// A field of the body that is text or `null`.
enum Field {
    /// The body lacks the field.
    Absent,
    /// The field is `null`: the default of the layout.
    Default,
    Text(String),
}

impl Field {
    fn text(self) -> Option<String> {
        match self {
            Field::Text(text) => Some(text),
            Field::Absent | Field::Default => None,
        }
    }
}

fn optional_text(data: &ApiData, name: &str, message: &str) -> Result<Field, Refusal> {
    match data.field(name) {
        None => Ok(Field::Absent),
        Some(Value::Null) => Ok(Field::Default),
        Some(Value::String(text)) => Ok(Field::Text(text.clone())),
        Some(_) => Err(Refusal::unprocessable(message)),
    }
}

/// Checks the body and says what it asks for.
fn read_request(data: &ApiData, settings: &Settings) -> Result<Request, Refusal> {
    let layout = match data.field("layout") {
        Some(Value::Null) => return Ok(Request::Clear),
        Some(Value::String(text)) if text == "og" => "og",
        Some(Value::String(text)) if text == "modern" => "modern",
        _ => return Err(Refusal::unprocessable("layout is not valid")),
    };
    let theme = optional_text(data, "theme", "theme is not valid")?;
    let color_set = optional_text(data, "colorSet", "colorSet is not valid")?;
    if matches!((&theme, &color_set), (Field::Absent, Field::Absent)) {
        return Ok(Request::Switch { layout });
    }
    let theme = theme.text();
    let color_set = color_set.text();
    let theme_names: Vec<&str> = if layout == "og" {
        OG_THEMES.to_vec()
    } else {
        MODERN_THEMES.iter().map(|(name, _)| *name).collect()
    };
    if theme
        .as_deref()
        .is_some_and(|theme| !theme_names.contains(&theme))
    {
        return Err(Refusal::unprocessable("theme is not valid"));
    }
    let color_sets: &[&str] = if layout == "og" {
        &OG_STYLES
    } else {
        let theme = theme
            .clone()
            .unwrap_or_else(|| default_modern_theme(settings));
        modern_color_sets(&theme).unwrap_or_default()
    };
    if color_set
        .as_deref()
        .is_some_and(|set| !color_sets.contains(&set))
    {
        return Err(Refusal::unprocessable("colorSet is not valid"));
    }
    Ok(Request::Choose {
        layout,
        theme,
        color_set,
    })
}

/// The stored look after a request: the fields of the layout that was chosen are
/// replaced, and those of the other layout stay.
fn changed(
    mut stored: StoredLook,
    layout: &'static str,
    theme: Option<String>,
    color_set: Option<String>,
) -> StoredLook {
    stored.layout = Some(layout.to_owned());
    if layout == "og" {
        stored.og_theme = theme;
        stored.og_style = color_set;
    } else {
        stored.modern_theme = theme;
        stored.modern_color_set = color_set;
    }
    stored
}

/// `PUT /user/look`: stores the look of the caller, and answers the look that applies.
///
/// The body is `layout` (`og`, `modern` or `null`), `theme` and `colorSet`:
///
/// - A `layout` of `null` clears everything the caller chose.
/// - A body with neither `theme` nor `colorSet` only switches the layout and keeps the
///   theme and color set that the caller stored for that layout.
/// - Otherwise the layout's theme and color set are replaced. An explicit `null` means
///   the default of that layout, and one of the two being absent counts as `null`.
///
/// Only a logged-in user may ask: a script with the API key has no look of its own.
pub async fn set_look(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Envelope<Look>, Refusal> {
    caller.require(GroupLevel::LOGGED_IN)?;
    let Some(me) = caller.user() else {
        return Err(ApiError::NotAuthorized.into());
    };
    let settings = state.settings();
    let request = read_request(&data, &settings)?;
    let looks = state.database().looks();
    let user = me.authenticated.user.id;
    let stored = match request {
        Request::Clear => {
            looks.clear(user).await?;
            None
        }
        Request::Choose {
            layout,
            theme,
            color_set,
        } => {
            let current = looks.get(user).await?.unwrap_or_default();
            let new = changed(current, layout, theme, color_set);
            looks.set(user, &new).await?;
            Some(new)
        }
        Request::Switch { layout } => {
            let mut new = looks.get(user).await?.unwrap_or_default();
            new.layout = Some(layout.to_owned());
            looks.set(user, &new).await?;
            Some(new)
        }
    };
    Ok(Envelope::success(effective_look(
        &settings,
        stored.as_ref(),
    )))
}
