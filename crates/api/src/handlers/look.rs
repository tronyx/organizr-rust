//! `PUT /user/look`: the layout, theme and colour set that the caller chose (D-125).
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
    modern_colour_sets,
};
use crate::state::AppState;

/// What the caller asked for.
enum Request {
    /// Forget everything that the caller chose.
    Clear,
    /// Store the layout, and its theme and colour set. `None` means the default.
    Choose {
        layout: &'static str,
        theme: Option<String>,
        colour_set: Option<String>,
    },
}

/// A field that is text or `null`. A field that the body lacks is `null`.
fn optional_text(data: &ApiData, name: &str, message: &str) -> Result<Option<String>, Refusal> {
    match data.field(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
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
    let colour_set = optional_text(data, "colourSet", "colourSet is not valid")?;
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
    let colour_sets: &[&str] = if layout == "og" {
        &OG_STYLES
    } else {
        let theme = theme
            .clone()
            .unwrap_or_else(|| default_modern_theme(settings));
        modern_colour_sets(&theme).unwrap_or_default()
    };
    if colour_set
        .as_deref()
        .is_some_and(|set| !colour_sets.contains(&set))
    {
        return Err(Refusal::unprocessable("colourSet is not valid"));
    }
    Ok(Request::Choose {
        layout,
        theme,
        colour_set,
    })
}

/// The stored look after a request: the fields of the layout that was chosen are
/// replaced, and those of the other layout stay.
fn changed(
    mut stored: StoredLook,
    layout: &'static str,
    theme: Option<String>,
    colour_set: Option<String>,
) -> StoredLook {
    stored.layout = Some(layout.to_owned());
    if layout == "og" {
        stored.og_theme = theme;
        stored.og_style = colour_set;
    } else {
        stored.modern_theme = theme;
        stored.modern_colour_set = colour_set;
    }
    stored
}

/// `PUT /user/look`: stores the look of the caller, and answers the look that applies.
///
/// The body is `layout` (`og`, `modern` or `null`), `theme` and `colourSet`. `null` for
/// the layout forgets the whole choice. Only a logged-in user may ask: a script with the
/// API key has no look of its own.
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
            colour_set,
        } => {
            let current = looks.get(user).await?.unwrap_or_default();
            let new = changed(current, layout, theme, colour_set);
            looks.set(user, &new).await?;
            Some(new)
        }
    };
    Ok(Envelope::success(effective_look(
        &settings,
        stored.as_ref(),
    )))
}
