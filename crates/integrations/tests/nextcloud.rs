//! The client of Nextcloud for the invites plugin, against a stand-in of its OCS API.

#![allow(
    clippy::expect_used,
    reason = "a test reads the answers it knows, and its helpers may panic"
)]

use std::time::Duration;

use organizr_integrations::http::Outbound;
use organizr_integrations::nextcloud::{Admin, NewUser, create_user, groups};
use organizr_testkit::services::{self, NEXTCLOUD_ADMIN};

const TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn groups_are_read_with_the_admin_account() {
    let stand_in = services::nextcloud().await;
    let http = Outbound::new().expect("a client");
    let url = stand_in.url();
    let admin = Admin {
        url: &url,
        user: NEXTCLOUD_ADMIN.0,
        password: NEXTCLOUD_ADMIN.1,
    };

    let found = groups(&http, admin, TIMEOUT)
        .await
        .expect("Nextcloud answers");

    assert_eq!(found, ["admin", "family", "friends"]);
    let seen = &stand_in.seen()[0];
    assert_eq!(seen.header("ocs-apirequest"), Some("true"));
    let wrong = Admin {
        password: "wrong",
        ..admin
    };
    assert!(groups(&http, wrong, TIMEOUT).await.is_err());
}

#[tokio::test]
async fn a_user_is_made_in_each_group_and_a_refusal_of_ocs_is_an_error() {
    let stand_in = services::nextcloud().await;
    let http = Outbound::new().expect("a client");
    let url = stand_in.url();
    let admin = Admin {
        url: &url,
        user: NEXTCLOUD_ADMIN.0,
        password: NEXTCLOUD_ADMIN.1,
    };
    let groups = ["family".to_owned(), "friends".to_owned()];
    let user = NewUser {
        user_id: "sam@example.com",
        password: "random",
        email: "sam@example.com",
        display_name: "Sam",
        groups: &groups,
        quota: "10GB",
    };

    create_user(&http, admin, user, TIMEOUT)
        .await
        .expect("the user is made");

    let form = stand_in.seen().pop().expect("a request").body;
    assert_eq!(
        form,
        "userid=sam%40example.com&password=random&email=sam%40example.com&displayName=Sam\
         &groups%5B%5D=family&groups%5B%5D=friends&quota=10GB"
    );

    // Nextcloud answers a user that exists with HTTP 200 and the OCS status 102.
    let again = create_user(&http, admin, user, TIMEOUT).await;
    assert!(again.is_err(), "{again:?}");
}
