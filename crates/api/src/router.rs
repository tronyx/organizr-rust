//! The route table of the API.

use axum::extract::{DefaultBodyLimit, State};
use axum::handler::Handler;
use axum::http::{HeaderValue, Method, Uri, header};
use axum::middleware::{from_fn, from_fn_with_state, map_response};
use axum::response::Response;
use axum::routing::{MethodRouter, any, delete, get, post, put};
use axum::{Extension, Router};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::api_data::ApiData;
use crate::caller::{Caller, identify_caller};
use crate::client::identify_client;
use crate::device_cookie::ensure_device_cookie;
use crate::error::ApiError;
use crate::handlers::{
    backups, basics, bookmarks, categories, config, connection_tests,
    connection_tests_more as more_tests, fields, forward_auth, groups, health_checks_plugin, help,
    homepage, icon, images, invites, ip, launch, lock, login, logout, mail, news, pages, ping,
    plex, register, speed_test, sponsors, tabs, token, two_factor, users, users_import, wizard,
};
use crate::picture_cache;
use crate::same_origin;
use crate::slim_route::{RoutePatternError, expand};
use crate::state::AppState;

/// The path all API routes start with.
pub const API_BASE: &str = "/api/v2";

/// Builds the router of the application.
///
/// Routes are written as the patterns of the PHP source and expanded for Axum. A request
/// that matches no route, or a route with another method, gets the "not found" answer.
#[expect(
    clippy::too_many_lines,
    reason = "the route table is one list, and it reads best in one piece"
)]
pub fn router(state: AppState) -> Result<Router, RoutePatternError> {
    let routes = Routes::default()
        .route("/", &get(basics::root))?
        .route("/status[/]", &get(basics::status))?
        .route("/launch[/]", &get(launch::launch))?
        .route(
            "/2fa",
            &post(two_factor::verify)
                .put(two_factor::save)
                .delete(two_factor::remove),
        )?
        .route("/2fa/{type}", &post(two_factor::create))?
        .route("/ping", &get(basics::ping).post(ping::many))?
        .route("/ping/{ping}", &get(ping::one))?
        .route("/rox", &get(basics::rox))?
        .route("/roxinsocks", &get(basics::roxinsocks))?
        .route("/wizard", &post(wizard::wizard))?
        .route("/login", &post(login::login))?
        .route("/logout", &get(logout::logout))?
        // The PHP router tries `/users/{id}` for the other methods of this fixed path.
        .route(
            "/users/register",
            &post(register::register)
                .get(|State(state): State<AppState>, Extension(caller): Extension<Caller>| async move {
                    users::one_by_text(&state, &caller, "register").await
                })
                .put(
                    |State(state): State<AppState>,
                     Extension(caller): Extension<Caller>,
                     data: ApiData| async move {
                        users::change_by_text(&state, &caller, "register", &data).await
                    },
                )
                .delete(
                    (|State(state): State<AppState>, Extension(caller): Extension<Caller>| async move {
                        users::remove_by_text(&state, &caller, "register").await
                    })
                    .layer(map_response(fields::as_plain_json)),
                ),
        )?
        // The same for this fixed path.
        .route(
            "/users/recover",
            &post(mail::recover)
                .get(|State(state): State<AppState>, Extension(caller): Extension<Caller>| async move {
                    users::one_by_text(&state, &caller, "recover").await
                })
                .put(
                    |State(state): State<AppState>,
                     Extension(caller): Extension<Caller>,
                     data: ApiData| async move {
                        users::change_by_text(&state, &caller, "recover", &data).await
                    },
                )
                .delete(
                    (|State(state): State<AppState>, Extension(caller): Extension<Caller>| async move {
                        users::remove_by_text(&state, &caller, "recover").await
                    })
                    .layer(map_response(fields::as_plain_json)),
                ),
        )?
        // The PHP router tries `/token/{id}` for a DELETE of the two fixed paths too.
        .route(
            "/token/me",
            &get(token::me).delete(|state, caller| revoke_fixed(state, caller, "me")),
        )?
        .route(
            "/token/validate",
            &post(token::validate).delete(|state, caller| revoke_fixed(state, caller, "validate")),
        )?
        .route("/token/{id}", &delete(token::revoke))?
        // The PHP application sends another content type from the routes that change
        // or delete a tab.
        .route(
            "/tabs",
            &get(tabs::list)
                .post(tabs::add)
                .put(tabs::reorder.layer(map_response(fields::as_plain_json))),
        )?
        .route(
            "/tabs/{id}",
            &get(tabs::one)
                .put(tabs::change.layer(map_response(fields::as_plain_json)))
                .delete(tabs::remove.layer(map_response(fields::as_plain_json))),
        )?
        // The list of categories is the list of the tab editor.
        .route(
            "/categories",
            &get(tabs::list)
                .post(categories::add)
                .put(categories::reorder.layer(map_response(fields::as_plain_json))),
        )?
        .route(
            "/categories/{id}",
            &put(categories::change.layer(map_response(fields::as_plain_json)))
                .delete(categories::remove.layer(map_response(fields::as_plain_json))),
        )?
        .route("/config", &get(config::all).put(config::change))?
        .route("/config/{item}", &get(config::item))?
        .route("/config/{item}/{term}", &get(config::item_with_term))?
        // An upload can be larger than the bodies that the other routes take.
        .route(
            "/image",
            &get(images::list).post(
                images::upload.layer(DefaultBodyLimit::max(images::LARGEST_UPLOAD + UPLOAD_OVERHEAD)),
            ),
        )?
        .route("/homepage/items", &get(homepage::items))?
        .route("/homepage/html/{number}", &get(homepage::html::show))?
        .route("/homepage/healthchecks", &get(homepage::health_checks::all))?
        .route(
            "/homepage/healthchecks/{tags}",
            &get(homepage::health_checks::tagged),
        )?
        .route("/homepage/kuma/data", &get(homepage::uptime_kuma::data))?
        .route("/homepage/weather/data", &get(homepage::weather::data))?
        .route(
            "/homepage/weather/coordinates",
            &post(homepage::helpers::coordinates),
        )?
        .route("/homepage/prompage/data", &get(homepage::prompage::data))?
        .route("/homepage/youtube/{query}", &get(homepage::helpers::trailer))?
        .route("/homepage/scrape", &post(homepage::helpers::scrape))?
        .route("/homepage/trakt/calendar", &get(homepage::trakt::calendar))?
        .route("/oauth/trakt", &get(homepage::trakt::oauth))?
        .route("/homepage/donate/success", &get(homepage::donate::success))?
        .route("/homepage/donate/cancel", &get(homepage::donate::cancel))?
        .route("/homepage/donate/error", &get(homepage::donate::error))?
        .route(
            "/homepage/donate",
            &get(homepage::donate::history).post(homepage::donate::start_from_form),
        )?
        .route("/homepage/donate/", &get(homepage::donate::start))?
        .route(
            "/homepage/donate/{amount}",
            &get(homepage::donate::start_with_amount),
        )?
        .route(
            "/homepage/embyLiveTVTracker/stats",
            &get(homepage::emby_live_tv::stats),
        )?
        .route(
            "/homepage/embyLiveTVTracker/activity",
            &get(homepage::emby_live_tv::activity),
        )?
        .route("/homepage/jellystat", &get(homepage::jellystat::data))?
        .route(
            "/homepage/jellystat/metadata",
            &post(homepage::jellystat::metadata),
        )?
        .route("/homepage/tautulli/data", &get(homepage::tautulli::data))?
        .route("/homepage/tautulli/names", &get(homepage::plex::names))?
        .route("/homepage/plex/streams", &get(homepage::plex::streams))?
        .route("/homepage/plex/recent", &get(homepage::plex::recent))?
        .route("/homepage/plex/playlists", &get(homepage::plex::playlists))?
        .route("/homepage/plex/metadata", &post(homepage::plex::metadata))?
        .route(
            "/homepage/plex/search/{query}",
            &get(homepage::plex::search),
        )?
        .route("/homepage/image", &get(homepage::plex::image))?
        .route("/homepage/emby/streams", &get(homepage::emby::emby_streams))?
        .route("/homepage/emby/recent", &get(homepage::emby::emby_recent))?
        .route("/homepage/emby/metadata", &post(homepage::emby::emby_metadata))?
        .route("/homepage/jellyfin/streams", &get(homepage::emby::jellyfin_streams))?
        .route("/homepage/jellyfin/recent", &get(homepage::emby::jellyfin_recent))?
        .route("/homepage/jellyfin/metadata", &post(homepage::emby::jellyfin_metadata))?
        .route(
            "/homepage/overseerr/metadata/{type}/{id}",
            &get(homepage::overseerr::metadata),
        )?
        .route(
            "/homepage/overseerr/requests",
            &get(homepage::overseerr::requests),
        )?
        .route(
            "/homepage/overseerr/requests/{type}",
            &get(homepage::overseerr::requests_of_type),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{limit}",
            &get(homepage::overseerr::requests_limited),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{limit}/{offset}",
            &get(homepage::overseerr::requests_paged),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{id}/available",
            &post(homepage::overseerr::available),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{id}/unavailable",
            &post(homepage::overseerr::unavailable),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{id}/pending",
            &post(homepage::overseerr::pending),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{id}/approve",
            &post(homepage::overseerr::approve),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{id}/deny",
            &put(homepage::overseerr::deny),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{id}",
            &delete(homepage::overseerr::delete).post(homepage::overseerr::add_request),
        )?
        .route(
            "/homepage/overseerr/requests/{type}/{id}/{seasons}",
            &post(homepage::overseerr::add_request_for_seasons),
        )?
        .route("/homepage/ombi/requests", &get(homepage::ombi::requests))?
        .route(
            "/homepage/ombi/requests/{type}",
            &get(homepage::ombi::requests_of_type),
        )?
        .route(
            "/homepage/ombi/requests/{type}/{limit}",
            &get(homepage::ombi::requests_limited),
        )?
        .route(
            "/homepage/ombi/requests/{type}/{limit}/{offset}",
            &get(homepage::ombi::requests_paged),
        )?
        .route(
            "/homepage/ombi/requests/{type}/{id}/available",
            &post(homepage::ombi::available),
        )?
        .route(
            "/homepage/ombi/requests/{type}/{id}/unavailable",
            &post(homepage::ombi::unavailable),
        )?
        .route(
            "/homepage/ombi/requests/{type}/{id}/approve",
            &post(homepage::ombi::approve),
        )?
        .route(
            "/homepage/ombi/requests/{type}/{id}/deny",
            &put(homepage::ombi::deny),
        )?
        .route(
            "/homepage/ombi/requests/{type}/{id}",
            &delete(homepage::ombi::delete).post(homepage::ombi::add_request),
        )?
        .route("/homepage/calendar", &get(homepage::arr::calendar))?
        .route(
            "/homepage/sickrage/calendar",
            &get(homepage::calendar_more::sickrage_calendar),
        )?
        .route(
            "/homepage/couchpotato/calendar",
            &get(homepage::calendar_more::couchpotato_calendar),
        )?
        .route(
            "/homepage/ical/calendar",
            &get(homepage::calendar_more::ical_calendar),
        )?
        .route(
            "/homepage/sonarr/calendar",
            &get(homepage::arr::sonarr_calendar),
        )?
        .route(
            "/homepage/radarr/calendar",
            &get(homepage::arr::radarr_calendar),
        )?
        .route(
            "/homepage/lidarr/calendar",
            &get(homepage::arr::lidarr_calendar),
        )?
        .route("/homepage/sonarr/queue", &get(homepage::arr::sonarr_queue))?
        .route("/homepage/radarr/queue", &get(homepage::arr::radarr_queue))?
        .route(
            "/homepage/sabnzbd/queue",
            &get(homepage::downloaders::sabnzbd_queue),
        )?
        .route(
            "/homepage/sabnzbd/queue/pause",
            &post(homepage::downloaders::sabnzbd_pause),
        )?
        .route(
            "/homepage/sabnzbd/queue/resume",
            &post(homepage::downloaders::sabnzbd_resume),
        )?
        .route(
            "/homepage/qbittorrent/queue",
            &get(homepage::downloaders::qbittorrent_queue),
        )?
        .route("/homepage/pihole/stats", &get(homepage::pihole::stats))?
        .route("/homepage/adguard/stats", &get(homepage::adguard::stats))?
        .route("/homepage/netdata/data", &get(homepage::netdata::data))?
        .route("/homepage/monitorr/data", &get(homepage::monitorr::data))?
        .route(
            "/homepage/speedtest/data",
            &get(homepage::speedtest_tracker::data),
        )?
        .route("/homepage/octoprint/data", &get(homepage::octoprint::data))?
        .route("/homepage/unifi/data", &get(homepage::unifi::data))?
        .route(
            "/homepage/jackett/download/",
            &post(homepage::indexers::jackett_download),
        )?
        .route(
            "/homepage/jackett/{query}",
            &get(homepage::indexers::jackett_search),
        )?
        .route(
            "/homepage/prowlarr/download/",
            &post(homepage::indexers::prowlarr_download),
        )?
        .route(
            "/homepage/prowlarr/{query}",
            &get(homepage::indexers::prowlarr_search),
        )?
        .route(
            "/homepage/nzbget/queue",
            &get(homepage::downloaders::nzbget_queue),
        )?
        .route(
            "/homepage/transmission/queue",
            &get(homepage::downloaders::transmission_queue),
        )?
        .route(
            "/homepage/deluge/queue",
            &get(homepage::downloaders_more::deluge_queue),
        )?
        .route(
            "/homepage/rtorrent/queue",
            &get(homepage::downloaders_more::rtorrent_queue),
        )?
        .route(
            "/homepage/utorrent/queue",
            &get(homepage::downloaders_more::utorrent_queue),
        )?
        .route(
            "/homepage/jdownloader/queue",
            &get(homepage::downloaders_more::jdownloader_queue),
        )?
        .route("/image/select", &get(images::select))?
        .route("/image/{image}", &delete(images::remove))?
        .route("/plex/servers", &get(plex::servers))?
        // The PHP application sends another content type from the two news routes.
        .route(
            "/news",
            &get(news::ignored.layer(map_response(fields::as_plain_json))),
        )?
        .route(
            "/news/{id}",
            &post(news::ignore.layer(map_response(fields::as_plain_json))),
        )?
        .route(
            "/sponsors/opencollective",
            &get(sponsors::open_collective_list),
        )?
        .route("/sponsors/github", &get(sponsors::github_list))?
        .route("/sponsors/all", &get(sponsors::all))?
        .route("/ip/{ip}", &get(ip::info))?
        .route("/icon", &get(icon::list))?
        .route("/help/smtp", &get(help::smtp))?
        .route("/page", &get(pages::names))?
        .route("/page/{page}", &get(pages::one))?
        .route("/organizr/{page}[/{var1}[/{var2}]]", &get(pages::raw))?
        .route("/backup", &get(backups::list).post(backups::create))?
        .route(
            "/backup/{filename}",
            &get(backups::download).delete(backups::remove),
        )?
        .route(
            "/plugins/healthchecks/run",
            &get(health_checks_plugin::run),
        )?
        .route("/plugins/bookmark/page", &get(bookmarks::page))?
        .route("/plugins/bookmark/setup/tab", &get(bookmarks::set_up_tab))?
        .route(
            "/plugins/bookmark/setup/category",
            &get(bookmarks::set_up_category),
        )?
        .route(
            "/plugins/bookmark/tabs",
            &get(bookmarks::list)
                .post(bookmarks::add)
                .put(bookmarks::reorder),
        )?
        .route(
            "/plugins/bookmark/tabs/{id}",
            &get(bookmarks::one)
                .put(bookmarks::change)
                .delete(bookmarks::remove),
        )?
        .route(
            "/plugins/bookmark/categories",
            &get(bookmarks::list)
                .post(bookmarks::add_category)
                .put(bookmarks::reorder_categories),
        )?
        .route(
            "/plugins/bookmark/categories/{id}",
            &put(bookmarks::change_category).delete(bookmarks::remove_category),
        )?
        .route("/plugins/invites", &get(invites::list).post(invites::create))?
        .route(
            "/plugins/invites/{code}",
            &get(invites::verify)
                .post(invites::use_code)
                .delete(invites::remove),
        )?
        .route("/test/plex", &post(connection_tests::test_plex))?
        .route("/test/sonarr", &post(connection_tests::test_sonarr))?
        .route("/test/radarr", &post(connection_tests::test_radarr))?
        .route("/test/lidarr", &post(connection_tests::test_lidarr))?
        .route("/test/sabnzbd", &post(connection_tests::test_sabnzbd))?
        .route("/test/qbittorrent", &post(connection_tests::test_qbittorrent))?
        .route("/test/tautulli", &post(connection_tests::test_tautulli))?
        .route("/test/overseerr", &post(connection_tests::test_overseerr))?
        .route("/test/ombi", &post(connection_tests::test_ombi))?
        .route("/test/sickrage", &post(connection_tests::test_sickrage))?
        .route("/test/nzbget", &post(connection_tests::test_nzbget))?
        .route("/test/transmission", &post(connection_tests::test_transmission))?
        .route("/test/deluge", &post(connection_tests::test_deluge))?
        .route("/test/rtorrent", &post(connection_tests::test_rtorrent))?
        .route("/test/utorrent", &post(connection_tests::test_utorrent))?
        .route("/test/jdownloader", &post(connection_tests::test_jdownloader))?
        .route("/test/pihole", &post(connection_tests::test_pihole))?
        .route("/test/emby", &post(connection_tests::test_emby))?
        .route("/test/jellyfin", &post(connection_tests::test_jellyfin))?
        .route("/test/jackett", &post(connection_tests::test_jackett))?
        .route("/test/prowlarr", &post(connection_tests::test_prowlarr))?
        .route("/test/adguard", &post(connection_tests::test_adguard))?
        .route("/test/unifi", &post(connection_tests::test_unifi))?
        .route("/test/unifi/site", &post(connection_tests::test_unifi_site))?
        .route(
            "/test/embyLiveTVTracker",
            &post(connection_tests::test_emby_live_tv),
        )?
        .route("/test/jellystat", &post(connection_tests::test_jellystat))?
        .route("/test/iframe", &post(more_tests::test_iframe))?
        .route("/test/path", &post(more_tests::test_path))?
        .route("/test/folder", &post(more_tests::test_folder))?
        .route("/test/database", &post(more_tests::test_database))?
        .route(
            "/test/cron",
            &get(more_tests::test_cron).post(more_tests::test_cron_schedule),
        )?
        .route("/test/slack-logs", &post(more_tests::test_slack_logs))?
        .route("/plugins/speedtest/garbage", &get(speed_test::garbage))?
        .route(
            "/plugins/speedtest/empty",
            &get(speed_test::ping).post(speed_test::upload),
        )?
        .route("/plugins/speedtest/ip", &get(speed_test::ip))?
        .route("/plugins/php-mailer/email/list", &get(mail::list))?
        .route("/plugins/php-mailer/email/send", &post(mail::send))?
        .route("/plugins/php-mailer/email/test", &get(mail::test))?
        .route("/users", &get(users::list).post(users::add))?
        .route("/users/import/{type}", &post(users_import::import))?
        .route("/users/lock", &post(lock::lock_own))?
        .route("/users/unlock", &post(lock::unlock_own))?
        .route("/users/lock/{id}", &post(lock::lock_user))?
        .route("/users/unlock/{id}", &post(lock::unlock_user))?
        .route(
            "/users/{id}",
            &get(users::one)
                .put(users::change)
                .delete(users::remove.layer(map_response(fields::as_plain_json))),
        )?
        .route("/groups", &get(groups::list).post(groups::add))?
        .route(
            "/groups/{id}",
            &get(groups::one)
                .put(groups::change.layer(map_response(fields::as_plain_json)))
                .delete(groups::remove.layer(map_response(fields::as_plain_json))),
        )?;
    #[cfg(feature = "wasm-plugins")]
    let routes = extension_routes(routes)?;

    // The uploaded pictures have the address of the PHP application, which is not under
    // the address of the API.
    let routes = routes
        .router
        .route("/data/userTabs/{name}", get(images::uploaded_picture))
        .route("/data/cache/{name}", get(picture_cache::cached_picture));
    // A reverse proxy asks these routes with the method and the headers of the request
    // that it holds back. Such a request can come from another site, so the routes are
    // outside the check for requests from other sites. They change nothing.
    let open = Routes::default()
        .route("/auth-[{group}[/]]", &any(forward_auth::check))?
        .route(
            "/auth[/[{group}[/{type}[/{ips}]]]]",
            &any(forward_auth::check),
        )?
        .route(
            "/organizr-auth[/[{group}[/{type}[/{ips}]]]]",
            &any(forward_auth::check),
        )?;
    Ok(finish(routes, open.router, state))
}

/// `DELETE /token/{id}` for a path that Axum gives to a route without a parameter.
async fn revoke_fixed(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    id: &'static str,
) -> Result<Response, ApiError> {
    token::revoke_by_text(&state, &caller, id).await
}

/// The routes of the extensions.
#[cfg(feature = "wasm-plugins")]
fn extension_routes(routes: Routes) -> Result<Routes, RoutePatternError> {
    use crate::handlers::extensions;

    let mut routes = routes
        .route("/extensions", &get(extensions::list))?
        .route("/extensions/marketplace", &get(extensions::marketplace))?
        .route("/extensions/marketplace/{id}", &post(extensions::install))?
        .route("/extensions/{id}", &delete(extensions::remove))?
        .route("/extensions/{id}/enable", &post(extensions::enable))?
        .route("/extensions/{id}/disable", &post(extensions::disable))?
        .route("/extension/{id}", &any(extensions::call_root))?;
    // The rest of the path belongs to the extension, whatever it holds.
    routes.router = routes.router.route(
        &format!("{API_BASE}/extension/{{id}}/{{*path}}"),
        any(extensions::call),
    );
    Ok(routes)
}

/// Adds what every route gets, and the answer for a request that no route takes.
fn finish(routes: Router<AppState>, open: Router<AppState>, state: AppState) -> Router {
    routes
        // Route layers run only when a route takes the request, so a path that is not a
        // route, or not one for the method, stays "not found" whoever asks, as in the PHP
        // application.
        .route_layer(same_origin::layer())
        .route_layer(from_fn(same_origin::answer_refusal))
        // Added after the route layers, so the layers do not cover these routes.
        .merge(open)
        .fallback(not_found)
        .method_not_allowed_fallback(not_found)
        .layer(from_fn(ensure_device_cookie))
        // Added before the layer that identifies the client, so it runs after it: the
        // cookie it may drop needs to know whether the browser used HTTPS.
        .layer(from_fn_with_state(state.clone(), identify_caller))
        // Added after the device cookie, so it runs before it: the device cookie reads
        // the client that this layer works out.
        .layer(from_fn_with_state(state.clone(), identify_client))
        // The PHP application starts a PHP session on every request, which adds these
        // three headers to every answer. They also keep answers out of caches.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::PRAGMA,
            HeaderValue::from_static("no-cache"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::EXPIRES,
            HeaderValue::from_static("Thu, 19 Nov 1981 08:52:00 GMT"),
        ))
        .with_state(state)
}

/// Collects routes that are written as patterns of the PHP source.
#[derive(Default)]
struct Routes {
    router: Router<AppState>,
}

impl Routes {
    /// Adds every explicit route a pattern stands for, under [`API_BASE`].
    fn route(
        mut self,
        pattern: &str,
        methods: &MethodRouter<AppState>,
    ) -> Result<Self, RoutePatternError> {
        for route in expand(pattern)? {
            let path = format!("{API_BASE}{}", route.path());
            self.router = self.router.route(&path, methods.clone());
        }
        Ok(self)
    }
}

/// What an upload request holds next to the picture itself: the lines around the parts.
const UPLOAD_OVERHEAD: usize = 64 * 1024;

/// The answer to a request that no route takes.
async fn not_found(method: Method, uri: Uri) -> ApiError {
    ApiError::NotFound {
        endpoint: uri.path().to_owned(),
        method: method.as_str().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode, header};
    use axum::routing::get;
    use http_body_util::BodyExt;
    use organizr_db::Database;
    use organizr_settings::Settings;
    use tower::ServiceExt;

    use super::{Routes, finish};
    use crate::state::AppState;

    /// What the PHP application answers when the form key is missing or wrong.
    const REFUSAL: &str = "{\n    \"response\": {\n        \"result\": \"error\",\n        \"message\": \"Not authorized for current Route: /api/v2/change\",\n        \"data\": null\n    }\n}";

    /// The layers of the real router around one route that reads and one that changes.
    /// The application itself has no route that changes something yet.
    async fn app() -> (Router, tempfile::TempDir) {
        let directory = tempfile::tempdir().expect("a temporary folder can be created");
        let file = directory.path().join("organizr.db");
        let url = format!("sqlite:{}", file.display().to_string().replace('\\', "/"));
        let database = Database::connect(&url).await.expect("the database opens");

        let routes = Routes::default()
            .route(
                "/change",
                &get(|| async { "read" })
                    .post(|| async { "changed" })
                    .put(|| async { "changed" })
                    .delete(|| async { "changed" }),
            )
            .expect("the pattern is valid");
        let state = AppState::new(
            database,
            Settings::defaults(),
            organizr_testkit::seed::secrets(),
        );
        (finish(routes.router, Router::new(), state), directory)
    }

    async fn send(app: &Router, method: Method, headers: &[(&str, &str)]) -> (StatusCode, String) {
        let mut request = Request::builder()
            .method(method)
            .uri("/api/v2/change")
            .header(header::HOST, "organizr.example.com");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("the request is valid"))
            .await
            .expect("the router answers");
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("the body can be read")
            .to_bytes();
        (
            status,
            String::from_utf8(body.to_vec()).expect("the body is UTF-8"),
        )
    }

    #[tokio::test]
    async fn request_from_another_site_that_would_change_something_is_refused() {
        let (app, _directory) = app().await;
        let from_elsewhere: [&[(&str, &str)]; 3] = [
            // A current browser on another site.
            &[("sec-fetch-site", "cross-site")],
            // A current browser on a sibling sub-domain, for example another application
            // behind the same proxy.
            &[("sec-fetch-site", "same-site")],
            // An older browser, which only says where the page came from.
            &[("origin", "https://evil.example.net")],
        ];

        for headers in from_elsewhere {
            for method in [Method::POST, Method::PUT, Method::DELETE] {
                let (status, body) = send(&app, method.clone(), headers).await;

                assert_eq!(
                    status,
                    StatusCode::UNAUTHORIZED,
                    "{method} with {headers:?}"
                );
                assert_eq!(body, REFUSAL, "{method} with {headers:?}");
            }
        }
    }

    #[tokio::test]
    async fn request_from_the_application_itself_changes_something() {
        let (app, _directory) = app().await;
        let from_here: [&[(&str, &str)]; 4] = [
            // A current browser on a page of the application.
            &[("sec-fetch-site", "same-origin")],
            // A request the user started directly, such as from a bookmark.
            &[("sec-fetch-site", "none")],
            // An older browser on a page of the application.
            &[("origin", "https://organizr.example.com")],
            // A script or an API client: no browser headers at all.
            &[],
        ];

        for headers in from_here {
            let (status, body) = send(&app, Method::POST, headers).await;

            assert_eq!(
                (status, body.as_str()),
                (StatusCode::OK, "changed"),
                "{headers:?}"
            );
        }
    }

    #[tokio::test]
    async fn reading_from_another_site_is_not_refused() {
        let (app, _directory) = app().await;

        let (status, body) = send(&app, Method::GET, &[("sec-fetch-site", "cross-site")]).await;

        assert_eq!((status, body.as_str()), (StatusCode::OK, "read"));
    }

    #[tokio::test]
    async fn refusal_carries_the_headers_of_every_answer() {
        let (app, _directory) = app().await;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v2/change")
            .header(header::HOST, "organizr.example.com")
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .expect("the request is valid");

        let response = app.oneshot(request).await.expect("the router answers");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .map(axum::http::HeaderValue::as_bytes),
            Some(b"application/json;charset=UTF-8".as_slice())
        );
        assert_eq!(
            response
                .headers()
                .get(header::CACHE_CONTROL)
                .map(axum::http::HeaderValue::as_bytes),
            Some(b"no-store, no-cache, must-revalidate".as_slice())
        );
    }
}
