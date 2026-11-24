//! The donate item: donations to the Admin through the Stripe account of the Admin.
//!
//! A person names an amount, and the server starts a payment at Stripe with the
//! secret key of the settings. The browser goes to the page of Stripe, where the
//! person pays. Organizr sees no card.

use axum::Extension;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_domain::GroupLevel;
use organizr_integrations::stripe::{self, Account, Checkout, StripeError};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::notice_page;
use crate::router::API_BASE;
use crate::state::AppState;

/// The largest amount of one donation, in cents: what Stripe takes for one payment
/// in US dollars.
const LARGEST: u64 = 99_999_999;

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageDonateEnabled".to_owned()],
        auth: vec!["homepageDonateAuth".to_owned()],
        not_empty: vec![
            "homepageDonateMinimum".to_owned(),
            "homepageDonatePublicToken".to_owned(),
            "homepageDonateSecretToken".to_owned(),
            "homepageDonateProductID".to_owned(),
        ],
    }
}

/// What must hold before a caller gets the list of donations.
fn history_permissions() -> Permissions {
    let mut permissions = permissions();
    permissions
        .enabled
        .push("homepageDonateShowUserHistory".to_owned());
    permissions
}

/// A number of cents as dollars: a whole number when it is one.
fn dollars(cents: u32) -> Value {
    if cents.is_multiple_of(100) {
        json!(cents / 100)
    } else {
        json!(f64::from(cents) / 100.0)
    }
}

/// How the interface shows the item. The keys of Stripe are no part of it.
pub fn options(settings: &Settings) -> Value {
    json!({
        "heading": settings.homepage_donate_customize_heading,
        "description": settings.homepage_donate_customize_description,
        "minimum": dollars(settings.homepage_donate_minimum),
        "history": settings.homepage_donate_show_user_history,
    })
}

fn account<'a>(state: &'a AppState, settings: &'a Settings) -> Account<'a> {
    Account {
        api: state.stripe(),
        secret: &settings.homepage_donate_secret_token,
    }
}

/// The answer for a request that Stripe did not carry out. The text names no key.
fn failed(error: &StripeError) -> Refusal {
    tracing::warn!(%error, "Stripe did not carry out a request of the donate item");
    Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

/// The e-mail address of the caller, when the caller has an account with one.
fn email_of(caller: &Caller) -> Option<&str> {
    caller
        .user()
        .and_then(|user| user.authenticated.user.email.as_deref())
        .filter(|email| !email.is_empty())
}

/// `GET /homepage/donate`: the donations of the caller. An Admin gets the donations
/// of everybody. A caller who does not get the list gets an empty one, as in the PHP
/// application.
pub async fn history(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    let settings = state.settings();
    // An Admin sees every donation. Every other caller sees the donations that name
    // the e-mail address of the account.
    let sees_all = caller.level() <= GroupLevel::ADMIN;
    let email = email_of(&caller).map(str::to_lowercase);
    if !history_permissions().allow(&settings, &caller) || (!sees_all && email.is_none()) {
        return Ok(Envelope::success(Vec::new()));
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    let charges = account(&state, &settings)
        .charges(http)
        .await
        .map_err(|error| failed(&error))?;
    let donations = charges
        .iter()
        .filter(|charge| sees_all || Some(stripe::payer(charge)) == email)
        .filter_map(stripe::donation)
        .collect();
    Ok(Envelope::success(donations))
}

/// The amount of a donation in cents, from what the caller named in dollars: at
/// least the minimum of the settings, and at most what Stripe takes.
fn cents_of(named: Option<f64>, minimum: u32) -> u64 {
    let minimum = u64::from(minimum);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the number is not negative and not above the largest amount"
    )]
    let named = named
        .filter(|dollars| dollars.is_finite() && *dollars > 0.0)
        .map_or(0, |dollars| {
            (dollars * 100.0).round().min(99_999_999.0) as u64
        });
    named.max(minimum).min(LARGEST)
}

/// Starts a payment at Stripe, and sends the browser to the page of Stripe.
async fn checkout(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    named: Option<f64>,
) -> Result<Response, Refusal> {
    let settings = state.settings();
    // The switch and the level of the item, and then the keys with the text of the
    // PHP application.
    let item = permissions();
    Permissions {
        not_empty: Vec::new(),
        ..item
    }
    .check(&settings, caller)?;
    if settings.homepage_donate_public_token.is_empty()
        || settings.homepage_donate_secret_token.is_empty()
        || settings.homepage_donate_product_id.is_empty()
    {
        return Err(Refusal::conflict("Donation Tokens are not setup"));
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    let address = |page: &str| {
        service_url(
            &format!("{API_BASE}/homepage/donate/{page}"),
            headers,
            client,
        )
    };
    let (success, cancel) = (address("success"), address("cancel"));
    let checkout = Checkout {
        product: &settings.homepage_donate_product_id,
        cents: cents_of(named, settings.homepage_donate_minimum),
        success: &success,
        cancel: &cancel,
        // An account that was made for a login of another service has an address
        // that is no address.
        email: email_of(caller).filter(|email| !email.to_lowercase().contains("placeholder")),
    };
    let page = account(state, &settings)
        .checkout(http, checkout)
        .await
        .map_err(|error| failed(&error))?;
    Ok((StatusCode::SEE_OTHER, [(header::LOCATION, page)]).into_response())
}

/// The whole number at the start of a text, as PHP reads one.
fn leading_number(text: &str) -> Option<f64> {
    let digits: String = text
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .take(12)
        .collect();
    digits.parse().ok()
}

/// `GET /homepage/donate/[{amount}]` without an amount: a payment of the minimum.
pub async fn start(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Response, Refusal> {
    checkout(&state, &caller, &client, &headers, None).await
}

/// `GET /homepage/donate/[{amount}]`: a payment of the amount, in whole dollars.
pub async fn start_with_amount(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(amount): Path<String>,
) -> Result<Response, Refusal> {
    checkout(&state, &caller, &client, &headers, leading_number(&amount)).await
}

/// `POST /homepage/donate`: a payment of the amount that the query names, in dollars.
pub async fn start_from_form(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Result<Response, Refusal> {
    let named = query.as_deref().and_then(|query| {
        form_urlencoded::parse(query.as_bytes())
            .find(|(name, _)| name == "amount")
            .and_then(|(_, amount)| amount.trim().parse::<f64>().ok())
    });
    checkout(&state, &caller, &client, &headers, named).await
}

/// `GET /homepage/donate/success`: the page that Stripe sends the browser to after a
/// payment.
pub async fn success() -> Response {
    notice_page::page(
        StatusCode::OK,
        "Donation Success",
        "Thank you for donating!",
        true,
    )
}

/// `GET /homepage/donate/cancel`: the page that Stripe sends the browser to when the
/// person gives up.
pub async fn cancel() -> Response {
    notice_page::page(
        StatusCode::OK,
        "Donation Canceled",
        "Taking you back...",
        true,
    )
}

/// `GET /homepage/donate/error`: the page for a payment that failed.
pub async fn error() -> Response {
    notice_page::page(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Donation Error",
        "An error has occurred!",
        true,
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{cents_of, dollars, leading_number};

    #[test]
    fn amount_is_at_least_the_minimum_and_at_most_what_stripe_takes() {
        assert_eq!(cents_of(Some(5.0), 100), 500);
        assert_eq!(cents_of(Some(5.5), 100), 550);
        assert_eq!(cents_of(Some(0.5), 100), 100);
        assert_eq!(cents_of(None, 250), 250);
        assert_eq!(cents_of(Some(-3.0), 250), 250);
        assert_eq!(cents_of(Some(f64::NAN), 250), 250);
        assert_eq!(cents_of(Some(f64::INFINITY), 250), 250);
        assert_eq!(cents_of(Some(1.0e12), 100), 99_999_999);
        assert_eq!(cents_of(Some(19.999), 100), 2000);
    }

    #[test]
    fn amount_of_the_path_is_its_whole_number() {
        assert_eq!(leading_number("25"), Some(25.0));
        assert_eq!(leading_number("25.75"), Some(25.0));
        assert_eq!(leading_number(" 7 dollars"), Some(7.0));
        assert_eq!(leading_number("many"), None);
        assert_eq!(leading_number("-5"), None);
        assert_eq!(leading_number(""), None);
    }

    #[test]
    fn cents_read_as_dollars() {
        assert_eq!(dollars(100), json!(1));
        assert_eq!(dollars(2500), json!(25));
        assert_eq!(dollars(250), json!(2.5));
    }
}
