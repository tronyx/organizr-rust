//! The Tautulli item: what the users of the Plex server watch most, and the libraries.

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use organizr_integrations::http::time_limit;
use organizr_integrations::tautulli::Install;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::picture_cache::Source;
use crate::state::AppState;

/// The statistics about what is watched.
const VIEWS: [&str; 4] = ["top_movies", "popular_movies", "popular_tv", "top_tv"];

/// The statistics about who watches, and on what.
const MISC: [&str; 2] = ["top_platforms", "top_users"];

/// The statistics that the homepage never shows.
const NEVER: [&str; 4] = [
    "top_music",
    "popular_music",
    "last_watched",
    "most_concurrent",
];

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageTautulliEnabled".to_owned()],
        auth: vec!["homepageTautulliAuth".to_owned()],
        not_empty: vec!["tautulliURL".to_owned(), "tautulliApikey".to_owned()],
    }
}

/// A value of Tautulli as text. Tautulli sends an id as a number in some answers and
/// as text in others.
fn written(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

/// Whether a row belongs to a library that the settings leave out.
fn is_excluded(row: &Value, excluded: &[&str]) -> bool {
    row.get("section_id")
        .is_some_and(|id| excluded.contains(&written(id).as_str()))
}

/// The statistics without the rows of the libraries that the settings leave out, and
/// without the statistics whose id is in `hidden`.
fn shown_statistics(response: &mut Value, excluded: &[&str], hidden: &[&str]) {
    let Some(statistics) = response.get_mut("data").and_then(Value::as_array_mut) else {
        return;
    };
    statistics.retain(|statistic| {
        let id = statistic.get("stat_id").and_then(Value::as_str);
        !id.is_some_and(|id| hidden.contains(&id))
    });
    for statistic in statistics {
        if let Some(rows) = statistic.get_mut("rows").and_then(Value::as_array_mut) {
            rows.retain(|row| !is_excluded(row, excluded));
        }
    }
}

/// The sizes of the pictures of the cache: a poster, and a wide picture.
fn picture_sizes(scale: u32) -> ((u32, u32), (u32, u32)) {
    ((200 * scale, 300 * scale), (1200 * scale, 675 * scale))
}

/// Gives the first row of each statistic about what is watched its pictures from the
/// picture cache, and has the cache fetch them.
fn use_cached_pictures(state: &AppState, install: &Install<'_>, response: &mut Value, scale: u32) {
    let (Some(cache), Some(http)) = (state.picture_cache(), state.outbound()) else {
        return;
    };
    let (poster, wide) = picture_sizes(scale);
    let Some(statistics) = response.get_mut("data").and_then(Value::as_array_mut) else {
        return;
    };
    for statistic in statistics {
        let id = statistic.get("stat_id").and_then(Value::as_str);
        if !id.is_some_and(|id| VIEWS.contains(&id)) {
            continue;
        }
        let Some(row) = statistic
            .get_mut("rows")
            .and_then(Value::as_array_mut)
            .and_then(|rows| rows.first_mut())
            .and_then(Value::as_object_mut)
        else {
            continue;
        };
        let rating_key = row.get("rating_key").map_or_default(written);
        if rating_key.is_empty() || !rating_key.chars().all(|digit| digit.is_ascii_digit()) {
            continue;
        }
        for (field, ending, (width, height)) in [("art", "np", wide), ("thumb", "list", poster)] {
            let picture = row.get(field).map_or_default(written);
            if picture.is_empty() {
                continue;
            }
            let name = format!("tautulli-{rating_key}-{ending}");
            cache.fetch_later(
                http,
                &name,
                Source {
                    url: install.picture_address(&picture, &rating_key, width, height),
                    key: None,
                    never_check_certificate: install.never_check_certificate,
                },
            );
            row.insert(field.to_owned(), json!(format!("data/cache/{name}.jpg")));
        }
    }
}

/// `GET /homepage/tautulli/data`: the statistics and the libraries that the caller
/// sees, with the options of the item.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = &state
        .outbound_for(settings.tautulli_use_custom_certificate)
        .ok_or_else(no_requests)?;
    // The setting can hold more than one address, for single sign-on. The homepage
    // shows the first install.
    let address = settings.tautulli_url.split(',').next().unwrap_or_default();
    let url = service_url(address);
    let install = Install {
        url: &url,
        key: &settings.tautulli_apikey,
        timeout: time_limit(&settings.homepage_tautulli_refresh.to_string()),
        never_check_certificate: settings.tautulli_disable_cert_check,
    };
    let may = |level: u32| u32::from(caller.level().get()) <= level;
    let (libraries, views, misc) = (
        may(settings.homepage_tautulli_library_auth),
        may(settings.homepage_tautulli_views_auth),
        may(settings.homepage_tautulli_misc_auth),
    );
    let mut answer = json!({
        "options": {
            "url": url,
            "libraries": settings.tautulli_libraries && libraries,
            "topMovies": settings.tautulli_top_movies && views,
            "topTV": settings.tautulli_top_tv && views,
            "topUsers": settings.tautulli_top_users && misc,
            "topPlatforms": settings.tautulli_top_platforms && misc,
            "popularMovies": settings.tautulli_popular_movies && views,
            "popularTV": settings.tautulli_popular_tv && views,
            "title": settings.tautulli_header_toggle,
            "friendlyName": settings.tautulli_friendly_name,
        },
    });
    let failed = |error: organizr_integrations::tautulli::TautulliError| {
        tracing::warn!(%error, "the statistics of Tautulli could not be loaded");
        Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
    };

    let statistics = install
        .command(http, "get_home_stats", &[("grouping", "1")])
        .await
        .map_err(failed)?;
    let Some(mut statistics) = statistics else {
        return Ok(Envelope::success(answer));
    };
    let mut hidden: Vec<&str> = NEVER.to_vec();
    if !views {
        hidden.extend(VIEWS);
    }
    if !misc {
        hidden.extend(MISC);
    }
    let excluded: Vec<&str> = settings
        .homepage_tautulli_viewing_stats_exclude
        .split(',')
        .collect();
    shown_statistics(&mut statistics, &excluded, &hidden);
    use_cached_pictures(&state, &install, &mut statistics, settings.cache_image_size);
    if let Some(answer) = answer.as_object_mut() {
        answer.insert("homestats".to_owned(), statistics);
    }

    if libraries {
        let table = install
            .command(http, "get_libraries_table", &[])
            .await
            .map_err(failed)?;
        if let Some(mut table) =
            table.and_then(|mut response| response.get_mut("data").map(Value::take))
        {
            let excluded: Vec<&str> = settings
                .homepage_tautulli_library_stats_exclude
                .split(',')
                .collect();
            if let Some(rows) = table.get_mut("data").and_then(Value::as_array_mut) {
                rows.retain(|row| !is_excluded(row, &excluded));
            }
            if let Some(answer) = answer.as_object_mut() {
                answer.insert("libstats".to_owned(), table);
            }
        }
    }
    Ok(Envelope::success(answer))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{picture_sizes, shown_statistics};

    #[test]
    fn statistics_lose_hidden_ids_and_the_rows_of_excluded_libraries() {
        let mut response = json!({"result": "success", "data": [
            {"stat_id": "top_movies", "rows": [
                {"title": "One", "section_id": 1},
                {"title": "Two", "section_id": "2"},
            ]},
            {"stat_id": "top_users", "rows": [{"user": "robin"}]},
            {"stat_id": "last_watched", "rows": []},
        ]});

        shown_statistics(&mut response, &["2"], &["last_watched", "top_users"]);

        assert_eq!(
            response,
            json!({"result": "success", "data": [
                {"stat_id": "top_movies", "rows": [{"title": "One", "section_id": 1}]},
            ]})
        );
    }

    #[test]
    fn pictures_grow_with_the_setting() {
        assert_eq!(picture_sizes(1), ((200, 300), (1200, 675)));
        assert_eq!(picture_sizes(2), ((400, 600), (2400, 1350)));
    }
}
