//! `GET /launch` with the look, and `PUT /user/look`: each person chooses the own
//! layout, theme and colour set.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app, installed_app_with, seed};
use serde_json::{Value, json};

fn default_look() -> Value {
    json!({"layout": "og", "theme": "Organizr", "colourSet": "dark", "source": "default"})
}

#[tokio::test]
async fn launch_has_the_default_look_for_a_guest() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let answer = call(&app, As::Guest, "GET", "/launch", None).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data()["look"], default_look());
    })
    .await;
}

#[tokio::test]
async fn the_admin_default_is_the_look_of_a_person_who_chose_none() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("layout", json!("modern")),
                ("modernColourSet", json!("light")),
            ],
        );

        let answer = call(&app, As::Guest, "GET", "/launch", None).await;

        assert_eq!(
            answer.data()["look"],
            json!({"layout": "modern", "theme": "organizr", "colourSet": "light", "source": "default"})
        );
    })
    .await;
}

#[tokio::test]
async fn user_chooses_the_light_colours_of_the_modern_theme() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        let body = json!({"layout": "modern", "theme": "organizr", "colourSet": "light"});
        let set = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        let own =
            json!({"layout": "modern", "theme": "organizr", "colourSet": "light", "source": "user"});
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        assert_eq!(set.data(), own);
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(launch.data()["look"], own);
    })
    .await;
}

#[tokio::test]
async fn user_chooses_a_brand_theme() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        for theme in ["plex", "jellyfin", "emby"] {
            let body = json!({"layout": "modern", "theme": theme, "colourSet": "light"});
            let set = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
            let own =
                json!({"layout": "modern", "theme": theme, "colourSet": "light", "source": "user"});
            assert_eq!(set.status, StatusCode::OK, "{}", set.body);
            assert_eq!(set.data(), own);
            let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
            assert_eq!(launch.data()["look"], own);
        }
    })
    .await;
}

#[tokio::test]
async fn an_admin_default_brand_theme_reaches_a_guest() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("layout", json!("modern")),
                ("modernTheme", json!("emby")),
                ("modernColourSet", json!("light")),
            ],
        );

        let answer = call(&app, As::Guest, "GET", "/launch", None).await;

        assert_eq!(
            answer.data()["look"],
            json!({"layout": "modern", "theme": "emby", "colourSet": "light", "source": "default"})
        );
    })
    .await;
}

#[tokio::test]
async fn user_chooses_silo_and_its_accent_sets() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        for set in ["ember", "ice", "rose"] {
            let body = json!({"layout": "modern", "theme": "silo", "colourSet": set});
            let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
            let own =
                json!({"layout": "modern", "theme": "silo", "colourSet": set, "source": "user"});
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.data(), own);
        }
        // Without a colour set Silo gets its first one, Ember.
        let body = json!({"layout": "modern", "theme": "silo"});
        let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["look"],
            json!({"layout": "modern", "theme": "silo", "colourSet": "ember", "source": "user"})
        );
    })
    .await;
}

#[tokio::test]
async fn user_chooses_nocturne_and_its_colour_sets() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        for set in ["amethyst", "crimson", "moonlit"] {
            let body = json!({"layout": "modern", "theme": "nocturne", "colourSet": set});
            let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
            let own = json!({
                "layout": "modern", "theme": "nocturne", "colourSet": set, "source": "user"
            });
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.data(), own);
        }
        // Without a colour set Nocturne gets its first one, Amethyst.
        let body = json!({"layout": "modern", "theme": "nocturne"});
        let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["look"],
            json!({
                "layout": "modern", "theme": "nocturne", "colourSet": "amethyst", "source": "user"
            })
        );
    })
    .await;
}

#[tokio::test]
async fn user_chooses_riot_and_its_colour_sets() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        for set in ["acid", "hotwire", "blacklight"] {
            let body = json!({"layout": "modern", "theme": "riot", "colourSet": set});
            let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
            let own = json!({
                "layout": "modern", "theme": "riot", "colourSet": set, "source": "user"
            });
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.data(), own);
        }
        // Without a colour set Riot gets its first one, Acid.
        let body = json!({"layout": "modern", "theme": "riot"});
        let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["look"],
            json!({
                "layout": "modern", "theme": "riot", "colourSet": "acid", "source": "user"
            })
        );
    })
    .await;
}

#[tokio::test]
async fn user_chooses_tempest_and_its_colour_sets() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        for set in ["squall", "abyss", "gale"] {
            let body = json!({"layout": "modern", "theme": "tempest", "colourSet": set});
            let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
            let own = json!({
                "layout": "modern", "theme": "tempest", "colourSet": set, "source": "user"
            });
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.data(), own);
        }
        // Without a colour set Tempest gets its first one, Squall.
        let body = json!({"layout": "modern", "theme": "tempest"});
        let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["look"],
            json!({
                "layout": "modern", "theme": "tempest", "colourSet": "squall", "source": "user"
            })
        );
    })
    .await;
}

#[tokio::test]
async fn user_chooses_circuit_and_its_colour_sets() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        for set in ["matrix", "synth", "amber"] {
            let body = json!({"layout": "modern", "theme": "circuit", "colourSet": set});
            let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
            let own = json!({
                "layout": "modern", "theme": "circuit", "colourSet": set, "source": "user"
            });
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.data(), own);
        }
        // Without a colour set Circuit gets its first one, Matrix.
        let body = json!({"layout": "modern", "theme": "circuit"});
        let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["look"],
            json!({
                "layout": "modern", "theme": "circuit", "colourSet": "matrix", "source": "user"
            })
        );
    })
    .await;
}

#[tokio::test]
async fn user_chooses_prism_and_its_colour_sets() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        for set in ["daydream", "twilight", "candy"] {
            let body = json!({"layout": "modern", "theme": "prism", "colourSet": set});
            let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
            let own = json!({
                "layout": "modern", "theme": "prism", "colourSet": set, "source": "user"
            });
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.data(), own);
        }
        // Without a colour set Prism gets its first one, Daydream.
        let body = json!({"layout": "modern", "theme": "prism"});
        let answer = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(
            launch.data()["look"],
            json!({
                "layout": "modern", "theme": "prism", "colourSet": "daydream", "source": "user"
            })
        );
    })
    .await;
}

#[tokio::test]
async fn an_admin_default_of_silo_reaches_a_guest() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("layout", json!("modern")),
                ("modernTheme", json!("silo")),
                ("modernColourSet", json!("rose")),
            ],
        );

        let answer = call(&app, As::Guest, "GET", "/launch", None).await;

        assert_eq!(
            answer.data()["look"],
            json!({"layout": "modern", "theme": "silo", "colourSet": "rose", "source": "default"})
        );
    })
    .await;
}

#[tokio::test]
async fn user_sets_and_clears_their_look() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let zoe = seed::logged_in(database, &app, "zoe", 4).await;

        let body = json!({"layout": "og", "theme": "Nord", "colourSet": "light"});
        let set = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        let own = json!({"layout": "og", "theme": "Nord", "colourSet": "light", "source": "user"});
        assert_eq!(set.status, StatusCode::OK, "{}", set.body);
        assert_eq!(set.data(), own);
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(launch.data()["look"], own);
        // Nobody else is affected.
        let other = call(&app, As::User(&zoe), "GET", "/launch", None).await;
        assert_eq!(other.data()["look"], default_look());

        let body = json!({"layout": null, "theme": null, "colourSet": null});
        let cleared = call(&app, As::User(&robin), "PUT", "/user/look", Some(body)).await;
        assert_eq!(cleared.status, StatusCode::OK, "{}", cleared.body);
        assert_eq!(cleared.data(), default_look());
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(launch.data()["look"], default_look());
    })
    .await;
}

#[tokio::test]
async fn switching_layout_keeps_the_other_layouts_choice() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let put = |body: Value| call(&app, As::User(&robin), "PUT", "/user/look", Some(body));

        put(json!({"layout": "og", "theme": "Dracula", "colourSet": "light"})).await;
        let modern =
            put(json!({"layout": "modern", "theme": "organizr", "colourSet": "dark"})).await;
        assert_eq!(
            modern.data(),
            json!({"layout": "modern", "theme": "organizr", "colourSet": "dark", "source": "user"})
        );

        // Back to the original layout with its defaults for theme and colour set: the
        // choice for the modern layout stays in the database.
        let back = put(json!({"layout": "og", "theme": null, "colourSet": null})).await;
        assert_eq!(
            back.data(),
            json!({"layout": "og", "theme": "Organizr", "colourSet": "dark", "source": "user"})
        );
        let stored = database
            .looks()
            .get(
                database
                    .users()
                    .find_by_username("robin")
                    .await
                    .expect("the lookup works")
                    .expect("the user is there")
                    .id,
            )
            .await
            .expect("the look can be read")
            .expect("the look is stored");
        assert_eq!(stored.modern_theme.as_deref(), Some("organizr"));
        assert_eq!(stored.modern_colour_set.as_deref(), Some("dark"));
        assert_eq!(stored.og_theme, None);

        // Choosing the original layout again with a theme keeps the modern choice.
        put(json!({"layout": "og", "theme": "Dracula", "colourSet": "dark"})).await;
        let again = put(json!({"layout": "modern", "theme": null, "colourSet": null})).await;
        assert_eq!(
            again.data(),
            json!({"layout": "modern", "theme": "organizr", "colourSet": "dark", "source": "user"})
        );
    })
    .await;
}

#[tokio::test]
async fn switch_to_modern_then_back_to_og_keeps_the_og_theme() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;
        let put = |body: Value| call(&app, As::User(&robin), "PUT", "/user/look", Some(body));

        put(json!({"layout": "og", "theme": "Dracula", "colourSet": "light"})).await;
        let modern = put(json!({"layout": "modern"})).await;
        assert_eq!(modern.status, StatusCode::OK, "{}", modern.body);
        assert_eq!(
            modern.data(),
            json!({"layout": "modern", "theme": "organizr", "colourSet": "dark", "source": "user"})
        );
        let back = put(json!({"layout": "og"})).await;
        assert_eq!(
            back.data(),
            json!({"layout": "og", "theme": "Dracula", "colourSet": "light", "source": "user"})
        );
        // An explicit null still means the default of that layout.
        let reset = put(json!({"layout": "og", "theme": null, "colourSet": null})).await;
        assert_eq!(
            reset.data(),
            json!({"layout": "og", "theme": "Organizr", "colourSet": "dark", "source": "user"})
        );
    })
    .await;
}

#[tokio::test]
async fn invalid_values_are_refused_with_422() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        let cases = [
            (json!({"layout": "tiled"}), "layout is not valid"),
            (json!({"layout": 3}), "layout is not valid"),
            (
                json!({"layout": "og", "theme": "Nope"}),
                "theme is not valid",
            ),
            (
                json!({"layout": "og", "theme": "Nord", "colourSet": "purple"}),
                "colourSet is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "Nord"}),
                "theme is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "organizr", "colourSet": "purple"}),
                "colourSet is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "plex", "colourSet": "purple"}),
                "colourSet is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "silo", "colourSet": "dark"}),
                "colourSet is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "nocturne", "colourSet": "dark"}),
                "colourSet is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "riot", "colourSet": "amethyst"}),
                "colourSet is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "tempest", "colourSet": "acid"}),
                "colourSet is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "circuit", "colourSet": "acid"}),
                "colourSet is not valid",
            ),
            (
                json!({"layout": "modern", "theme": "prism", "colourSet": "light"}),
                "colourSet is not valid",
            ),
        ];
        for (body, message) in cases {
            let answer = call(
                &app,
                As::User(&robin),
                "PUT",
                "/user/look",
                Some(body.clone()),
            )
            .await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
            assert_eq!(answer.message(), json!(message), "{body}");
        }
        // Nothing was stored.
        let launch = call(&app, As::User(&robin), "GET", "/launch", None).await;
        assert_eq!(launch.data()["look"], default_look());
    })
    .await;
}

#[tokio::test]
async fn look_cannot_be_set_without_a_logged_in_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let body = json!({"layout": "og", "theme": "Nord", "colourSet": "dark"});

        for who in [As::Guest, As::Admin] {
            let answer = call(&app, who, "PUT", "/user/look", Some(body.clone())).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{}", answer.body);
        }
    })
    .await;
}
