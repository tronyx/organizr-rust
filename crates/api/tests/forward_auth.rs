//! Forward auth through the real route table: what the recorded PHP answers cannot
//! show (D-59). Most of it is about whom the application believes.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::SocketAddr;

use axum::Router;
use axum::extract::ConnectInfo;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::{app, installed_settings, seed, state_with};
use serde_json::{Value, json};

/// The address of the reverse proxy in these tests.
const PROXY: [u8; 4] = [172, 18, 0, 2];
/// An address on the internet.
const STRANGER: [u8; 4] = [203, 0, 113, 9];

/// The application of an install that trusts the proxy at [`PROXY`].
fn app_behind_proxy(database: &Database, changed: &[(&str, Value)]) -> Router {
    let state = state_with(database, installed_settings(changed))
        .with_trusted_proxies("172.18.0.2".parse().expect("the address is valid"));
    router(state).expect("the route table is valid")
}

/// Asks a forward-auth route from an address, with the given headers.
async fn ask(app: &Router, from: [u8; 4], path: &str, headers: &[(&str, &str)]) -> Answer {
    ask_with(app, "GET", from, path, headers).await
}

async fn ask_with(
    app: &Router,
    method: &str,
    from: [u8; 4],
    path: &str,
    headers: &[(&str, &str)],
) -> Answer {
    let mut request = request(method, &format!("/api/v2{path}"), headers, "");
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((from, 40_000))));
    send(app, request).await
}

fn location(answer: &Answer) -> Option<String> {
    answer
        .headers
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// What Traefik sends with its question about a request for another application.
const TRAEFIK: [(&str, &str); 4] = [
    ("x-forwarded-server", "traefik"),
    ("x-forwarded-proto", "https"),
    ("x-forwarded-host", "sonarr.example.com"),
    ("x-forwarded-uri", "/series"),
];

#[tokio::test]
async fn headers_of_a_proxy_are_believed_only_from_a_trusted_proxy() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);

        // The trusted proxy: a caller who is refused goes to the login page, with the
        // address to come back to.
        let through_proxy = ask(&app, PROXY, "/auth/4", &TRAEFIK).await;
        assert_eq!(through_proxy.status, StatusCode::FOUND);
        assert_eq!(
            location(&through_proxy).as_deref(),
            Some("https://organizr.example.com/?return=https%3A%2F%2Fsonarr.example.com%2Fseries")
        );

        // Anybody else who sends the same headers gets the plain refusal. Otherwise a
        // stranger could make the application write any address into a redirect.
        let forged = [
            ("x-forwarded-server", "traefik"),
            ("x-forwarded-proto", "https"),
            ("x-forwarded-host", "evil.example.net"),
            ("x-forwarded-uri", "/"),
        ];
        let direct = ask(&app, STRANGER, "/auth/4", &forged).await;
        assert_eq!(direct.status, StatusCode::UNAUTHORIZED, "{}", direct.body);
        assert_eq!(location(&direct), None);
        assert_eq!(
            direct.message(),
            json!("User is not Authorized or User is locked")
        );
    })
    .await;
}

#[tokio::test]
async fn settings_send_a_refused_caller_to_the_login_for_every_proxy() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let on = app_behind_proxy(database, &[("traefikAuthEnable", json!(true))]);

        // No header says "traefik". The setting asks for the redirect.
        let refused = ask(&on, PROXY, "/auth/4", &TRAEFIK[1..]).await;
        assert_eq!(refused.status, StatusCode::FOUND);
        assert_eq!(
            location(&refused).as_deref(),
            Some("https://organizr.example.com/?return=https%3A%2F%2Fsonarr.example.com%2Fseries")
        );
        assert_eq!(refused.body, "200");
        // Without a trusted proxy, the redirect has no address to come back to: the
        // headers of a stranger do not count.
        let direct = ask(&on, STRANGER, "/auth/4", &TRAEFIK).await;
        assert_eq!(direct.status, StatusCode::FOUND);
        assert_eq!(
            location(&direct).as_deref(),
            Some("http://organizr.example.com/")
        );
        // A caller who passes is not redirected.
        let passes = ask(&on, PROXY, "/auth/999", &TRAEFIK).await;
        assert_eq!(passes.status, StatusCode::OK);

        // The address of the login page can be set.
        let settings = [
            ("traefikAuthEnable", json!(true)),
            (
                "traefikDomainOverride",
                json!("https://login.example.org/organizr/"),
            ),
        ];
        let moved = app_behind_proxy(database, &settings);
        let refused = ask(&moved, PROXY, "/auth/4", &TRAEFIK).await;
        assert_eq!(
            location(&refused).as_deref(),
            Some("https://login.example.org/organizr/?return=https%3A%2F%2Fsonarr.example.com%2Fseries")
        );
    })
    .await;
}

#[tokio::test]
async fn address_lists_use_the_address_that_a_trusted_proxy_reports() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);
        let ip_of = |answer: &Answer| answer.data()["user_ip"].clone();

        // The proxy reports the browser at 198.51.100.7.
        let reported = [("x-forwarded-for", "198.51.100.7")];
        let allowed = ask(&app, PROXY, "/auth/0/allow/198.51.100.7", &reported).await;
        assert_eq!(allowed.status, StatusCode::OK, "{}", allowed.body);
        assert_eq!(allowed.message(), json!("User is whitelisted"));
        assert_eq!(ip_of(&allowed), json!("198.51.100.7"));
        let denied = ask(
            &app,
            PROXY,
            "/auth/999/deny/10.0.0.1,198.51.100.7",
            &reported,
        )
        .await;
        assert_eq!(denied.status, StatusCode::UNAUTHORIZED);
        assert_eq!(denied.message(), json!("User is blacklisted"));
        // The proxy itself is not on the list.
        let proxy_only = ask(&app, PROXY, "/auth/0/allow/172.18.0.2", &reported).await;
        assert_eq!(proxy_only.status, StatusCode::UNAUTHORIZED);

        // A stranger who claims an address of the allow list stays a stranger.
        let claimed = ask(&app, STRANGER, "/auth/0/allow/198.51.100.7", &reported).await;
        assert_eq!(claimed.status, StatusCode::UNAUTHORIZED, "{}", claimed.body);
        assert_eq!(ip_of(&claimed), json!("203.0.113.9"));
        // And cannot get off the deny list that way either.
        let other = [("x-forwarded-for", "10.0.0.1")];
        let denied = ask(&app, STRANGER, "/auth/999/deny/203.0.113.9", &other).await;
        assert_eq!(denied.message(), json!("User is blacklisted"));
    })
    .await;
}

#[tokio::test]
async fn address_behind_a_proxy_that_is_not_trusted_is_on_no_allow_list() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);
        // A proxy that is not trusted sends every visitor from its own address. That
        // address on an allow list would let every visitor through.
        let forwarded = [("x-forwarded-for", "198.51.100.7")];
        let behind = ask(&app, STRANGER, "/auth/0/allow/203.0.113.9", &forwarded).await;
        assert_eq!(behind.status, StatusCode::UNAUTHORIZED, "{}", behind.body);
        assert_eq!(
            behind.message(),
            json!("User is not Authorized or User is locked")
        );
        // The same caller without the header is the caller of the connection.
        let direct = ask(&app, STRANGER, "/auth/0/allow/203.0.113.9", &[]).await;
        assert_eq!(
            direct.message(),
            json!("User is whitelisted"),
            "{}",
            direct.body
        );
        // A deny list still counts for that address, since it only makes the check
        // stricter.
        let denied = ask(&app, STRANGER, "/auth/999/deny/203.0.113.9", &forwarded).await;
        assert_eq!(denied.message(), json!("User is blacklisted"));
    })
    .await;
}

#[tokio::test]
async fn address_to_come_back_to_is_encoded() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);
        let headers = [
            ("x-forwarded-server", "traefik"),
            ("x-forwarded-proto", "https"),
            ("x-forwarded-host", "sonarr.example.com"),
            ("x-forwarded-uri", "/series?a=1&b=2 3"),
        ];
        let refused = ask(&app, PROXY, "/auth/4", &headers).await;
        assert_eq!(refused.status, StatusCode::FOUND);
        assert_eq!(
            location(&refused).as_deref(),
            Some(
                "https://organizr.example.com/?return=\
                 https%3A%2F%2Fsonarr.example.com%2Fseries%3Fa%3D1%26b%3D2%203"
            )
        );
    })
    .await;
}

#[tokio::test]
async fn lists_take_addresses_of_both_kinds_and_host_names() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);
        let local = [127, 0, 0, 1];

        // A host name stands for its addresses, in a check that a trusted proxy sends.
        // Spaces around an entry do not count.
        let from_local = [("x-forwarded-for", "127.0.0.1")];
        let path = "/auth/0/allow/10.0.0.1,%20localhost";
        let named = ask(&app, PROXY, path, &from_local).await;
        assert_eq!(
            named.message(),
            json!("User is whitelisted"),
            "{}",
            named.body
        );
        let unknown = ask(
            &app,
            PROXY,
            "/auth/0/allow/no-such-host.invalid",
            &from_local,
        )
        .await;
        assert_eq!(unknown.status, StatusCode::UNAUTHORIZED);
        // Whoever reaches Organizr without the proxy writes the path, so there only the
        // addresses of a list count, and no host name is looked up (D-141).
        let direct = ask(&app, local, path, &[]).await;
        assert_eq!(direct.status, StatusCode::UNAUTHORIZED, "{}", direct.body);
        let denied = ask(&app, local, "/auth/999/deny/localhost", &[]).await;
        assert_eq!(denied.status, StatusCode::OK, "{}", denied.body);
        assert_ne!(denied.message(), json!("User is blacklisted"));

        // An IPv6 address, written in two ways, and an IPv4 address inside an IPv6 one.
        let reported = [("x-forwarded-for", "2001:db8::7")];
        for list in ["2001:db8::7", "2001:0db8:0000:0000:0000:0000:0000:0007"] {
            let path = format!("/auth/0/allow/{list}");
            let allowed = ask(&app, PROXY, &path, &reported).await;
            assert_eq!(allowed.message(), json!("User is whitelisted"), "{list}");
        }
        let mapped = [("x-forwarded-for", "::ffff:198.51.100.7")];
        let allowed = ask(&app, PROXY, "/auth/0/allow/198.51.100.7", &mapped).await;
        assert_eq!(allowed.message(), json!("User is whitelisted"));
        // A part of an address is no match.
        let partly = ask(&app, PROXY, "/auth/0/allow/198.51.100", &mapped).await;
        assert_eq!(partly.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn question_from_another_site_is_answered() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);
        // A proxy hands on the method and the headers of the request that it holds
        // back. That request can be a form of another site.
        let from_another_site = [
            ("origin", "https://other.example.net"),
            ("sec-fetch-site", "cross-site"),
        ];

        for method in ["POST", "PUT", "DELETE", "PATCH"] {
            let answer = ask_with(&app, method, PROXY, "/auth/999", &from_another_site).await;
            assert_eq!(answer.status, StatusCode::OK, "{method}: {}", answer.body);
            assert_eq!(answer.message(), json!("User is authorized"));
        }
        // Every other route still refuses such a request.
        let refused = ask_with(&app, "POST", PROXY, "/tabs", &from_another_site).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            refused.message(),
            json!("Not authorized for current Route: /api/v2/tabs")
        );
    })
    .await;
}

#[tokio::test]
async fn group_in_the_query_asks_at_the_auth_routes_and_the_root_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);

        for path in ["/?group=999", "/auth?group=999"] {
            let answer = ask(&app, PROXY, path, &[]).await;
            assert_eq!(answer.status, StatusCode::OK, "{path}: {}", answer.body);
            assert_eq!(answer.data()["requested_group"], json!(999), "{path}");
        }
        // A proxy such as Caddy sends the query of the visitor with the address of the
        // check. With a group in the path, the query cannot lower the group or add an
        // allow list (D-136).
        for path in [
            "/organizr-auth/0/deny/1.2.3.4?group=999",
            "/auth/1?group=999",
            "/auth-1?group=999",
            "/auth/1?whitelist=172.18.0.2",
            "/auth-1?type=allow&ips=172.18.0.2",
        ] {
            let answer = ask(&app, PROXY, path, &[]).await;
            assert_eq!(
                answer.status,
                StatusCode::UNAUTHORIZED,
                "{path}: {}",
                answer.body
            );
            assert_ne!(answer.data()["requested_group"], json!(999), "{path}");
        }
        // The same allow list in the path lets the caller in, as the operator chose it.
        let allowed = ask(&app, PROXY, "/auth/1/allow/172.18.0.2", &[]).await;
        assert_eq!(allowed.status, StatusCode::OK, "{}", allowed.body);
        let refused = ask(&app, PROXY, "/?group=4", &[]).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        // The root without a group is the redirect that it always was.
        let root = ask(&app, PROXY, "/", &[]).await;
        assert_eq!(root.status, StatusCode::FOUND);
        assert_eq!(location(&root).as_deref(), Some("/api/v2/status"));

        // In PHP, `group=` in the query of any route makes the request a question of
        // forward auth. Here the other routes stay what they are.
        let tabs = call(&app, As::Admin, "GET", "/tabs?group=999", None).await;
        assert_eq!(tabs.status, StatusCode::OK);
        assert!(tabs.data()["tabs"].is_array(), "{}", tabs.body);
        let guest = call(&app, As::Guest, "GET", "/tabs?group=999", None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        assert_eq!(guest.message(), json!("Not Authorized"));
    })
    .await;
}

#[tokio::test]
async fn allow_list_that_the_proxy_wrote_itself_counts() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);
        let path = "/auth-1?whitelist=172.18.0.2";

        // nginx with a fixed address of the check: the visitor asked for /app, so the
        // query is the proxy's, and its allow list counts (D-157).
        let own = ask(&app, PROXY, path, &[("x-forwarded-uri", "/app")]).await;
        assert_eq!(own.status, StatusCode::OK, "{}", own.body);
        assert_eq!(own.message(), json!("User is whitelisted"));

        // Caddy hands on the query of the visitor: the same query, so it does not count.
        let visitors = ask(
            &app,
            PROXY,
            path,
            &[("x-forwarded-uri", "/app?whitelist=172.18.0.2")],
        )
        .await;
        assert_eq!(
            visitors.status,
            StatusCode::UNAUTHORIZED,
            "{}",
            visitors.body
        );
        // A '#' that the visitor adds is cut from the address of the check, but not from
        // the header: the two queries are still the same.
        let fragment = ask(
            &app,
            PROXY,
            "/auth-1?whitelist=172.18.0.2#z",
            &[("x-forwarded-uri", "/app?whitelist=172.18.0.2#z")],
        )
        .await;
        assert_eq!(
            fragment.status,
            StatusCode::UNAUTHORIZED,
            "{}",
            fragment.body
        );
        let long_form = ask(
            &app,
            PROXY,
            "/auth/1?whitelist=172.18.0.2#z",
            &[("x-forwarded-uri", "/app?whitelist=172.18.0.2#z")],
        )
        .await;
        assert_eq!(
            long_form.status,
            StatusCode::UNAUTHORIZED,
            "{}",
            long_form.body
        );
        // Without the header, or from a sender that is no trusted proxy, nothing counts.
        let bare = ask(&app, PROXY, path, &[]).await;
        assert_eq!(bare.status, StatusCode::UNAUTHORIZED, "{}", bare.body);
        let stranger = ask(
            &app,
            STRANGER,
            "/auth-1?whitelist=203.0.113.9",
            &[("x-forwarded-uri", "/app")],
        )
        .await;
        assert_eq!(
            stranger.status,
            StatusCode::UNAUTHORIZED,
            "{}",
            stranger.body
        );
    })
    .await;
}

#[tokio::test]
async fn name_of_a_tab_is_compared_as_text() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);
        let user = seed::logged_in(database, &app, "robin", 4).await;
        let requested = |answer: &Answer| answer.data()["requested_group"].clone();

        // The tab "Homepage" is for the level 4.
        let by_name = call(&app, As::User(&user), "GET", "/auth/Homepage", None).await;
        assert_eq!(by_name.status, StatusCode::OK, "{}", by_name.body);
        assert_eq!(requested(&by_name), json!(4));
        // In PHP `%` stands for any text, so `Home%` names the tab too. Here it names no
        // tab, and a name of no tab asks for the Admin.
        for name in ["Home%25", "_omepage", "%25"] {
            let path = format!("/auth/{name}");
            let answer = call(&app, As::User(&user), "GET", &path, None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{name}");
            assert_eq!(requested(&answer), json!(0), "{name}");
        }
    })
    .await;
}

#[tokio::test]
async fn answer_names_the_device_of_the_caller() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);

        let unknown = ask(&app, PROXY, "/auth/999", &[]).await;
        assert_eq!(unknown.data()["uuid"], json!("n/a"));
        let cookie = [("cookie", "other=1; organizr_user_uuid=0b0e8b0a-device")];
        let known = ask(&app, PROXY, "/auth/999", &cookie).await;
        assert_eq!(known.data()["uuid"], json!("0b0e8b0a-device"));
    })
    .await;
}

#[tokio::test]
async fn install_that_is_not_set_up_lets_nobody_through() {
    on_each_engine(async |database| {
        let app = app(database);

        for path in [
            "/auth/999",
            "/auth-999",
            "/auth/0/allow/0.0.0.0",
            "/?group=999",
        ] {
            let answer = call(&app, As::Guest, "GET", path, None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{path}");
            assert_eq!(
                answer.message(),
                json!("Organizr is not setup or an error occurred"),
                "{path}"
            );
        }
    })
    .await;
}

#[tokio::test]
async fn locked_user_passes_only_by_the_allow_list() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_behind_proxy(database, &[]);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let id = database
            .users()
            .find_by_username("robin")
            .await
            .expect("the lookup works")
            .expect("the user is there")
            .id
            .get();
        let passes = async |path: &str| {
            let cookie = [("cookie", robin.as_str())];
            ask(&app, PROXY, path, &cookie).await
        };
        assert_eq!(passes("/auth/4").await.status, StatusCode::OK);

        let path = format!("/users/{id}");
        let locked = call(&app, As::Admin, "PUT", &path, Some(json!({"locked": 1}))).await;
        assert_eq!(locked.status, StatusCode::OK, "{}", locked.body);

        // The PHP application answers the same: a locked user passes nowhere by the
        // group, also not where a guest passes.
        for path in ["/auth/4", "/auth/999", "/auth-999"] {
            let refused = passes(path).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{path}");
            assert_eq!(
                refused.message(),
                json!("User is not Authorized or User is locked"),
                "{path}"
            );
            assert_eq!(refused.data()["user"], json!("robin"));
            assert!(!refused.headers.contains_key("x-organizr-user"));
        }
        // An address of the allow list passes whoever uses it.
        let listed = passes("/auth/0/allow/172.18.0.2").await;
        assert_eq!(listed.message(), json!("User is whitelisted"));

        let unlocked = call(&app, As::Admin, "PUT", &path, Some(json!({"locked": 0}))).await;
        assert_eq!(unlocked.status, StatusCode::OK);
        assert_eq!(passes("/auth/4").await.status, StatusCode::OK);
    })
    .await;
}
