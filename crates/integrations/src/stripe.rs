//! Stripe: the payments of the donate item of the homepage. Every request carries the
//! secret key of the Stripe account of the Admin.

use std::time::Duration;

use serde_json::Value;

use crate::http::{Outbound, without_address};

/// The address of the API of Stripe.
pub const STRIPE_API: &str = "https://api.stripe.com";

/// How long Stripe has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The most characters of a text of Stripe that an error holds.
const SHOWN: usize = 300;

/// Why Stripe gave no usable answer. The text names no key.
#[derive(Debug, thiserror::Error)]
pub enum StripeError {
    /// Stripe cannot be reached, or it did not answer in time.
    #[error("Stripe cannot be reached: {0}")]
    Unreachable(String),
    /// Stripe refused the request. The text is the one that Stripe sent.
    #[error("{0}")]
    Refused(String),
    /// The answer is not what Stripe sends for the request.
    #[error("The answer of Stripe cannot be read")]
    Unreadable,
}

/// The Stripe account of the Admin.
#[derive(Clone, Copy)]
pub struct Account<'a> {
    /// The address of the API: [`STRIPE_API`], or a stand-in for a test.
    pub api: &'a str,
    /// The secret key of the account.
    pub secret: &'a str,
}

/// A payment that a person is about to make.
#[derive(Debug, Clone, Copy)]
pub struct Checkout<'a> {
    /// The id of the product of the account.
    pub product: &'a str,
    /// The amount in cents of a US dollar.
    pub cents: u64,
    /// Where Stripe sends the browser after the payment.
    pub success: &'a str,
    /// Where Stripe sends the browser when the person gives up.
    pub cancel: &'a str,
    /// The e-mail address of the person, when Organizr knows one.
    pub email: Option<&'a str>,
}

/// The text of the error of an answer of Stripe.
fn refusal(status: u16, answer: &Value) -> StripeError {
    let message = answer
        .pointer("/error/message")
        .and_then(Value::as_str)
        .map_or_else(
            || format!("Stripe answered with the status {status}"),
            |text| text.chars().take(SHOWN).collect(),
        );
    StripeError::Refused(message)
}

impl Account<'_> {
    async fn answer(&self, request: reqwest::RequestBuilder) -> Result<Value, StripeError> {
        let response = request
            .bearer_auth(self.secret.trim())
            .timeout(TIME_LIMIT)
            .send()
            .await
            .map_err(|error| StripeError::Unreachable(without_address(error)))?;
        let status = response.status();
        let answer: Value = response.json().await.unwrap_or(Value::Null);
        if status.is_success() {
            Ok(answer)
        } else {
            Err(refusal(status.as_u16(), &answer))
        }
    }

    /// The last 100 charges of the account.
    pub async fn charges(&self, http: &Outbound) -> Result<Vec<Value>, StripeError> {
        let url = format!("{}/v1/charges", self.api.trim_end_matches('/'));
        let request = http
            .client(&url, false)
            .get(&url)
            .query(&[("limit", "100")]);
        match self.answer(request).await?.get_mut("data").map(Value::take) {
            Some(Value::Array(charges)) => Ok(charges),
            _ => Err(StripeError::Unreadable),
        }
    }

    /// Starts a payment, and gives the address of the page of Stripe where the person
    /// pays.
    pub async fn checkout(
        &self,
        http: &Outbound,
        checkout: Checkout<'_>,
    ) -> Result<String, StripeError> {
        let url = format!("{}/v1/checkout/sessions", self.api.trim_end_matches('/'));
        let cents = checkout.cents.to_string();
        let mut form = vec![
            ("payment_method_types[0]", "card"),
            ("line_items[0][price_data][product]", checkout.product),
            ("line_items[0][price_data][unit_amount]", cents.as_str()),
            ("line_items[0][price_data][currency]", "usd"),
            ("line_items[0][quantity]", "1"),
            ("mode", "payment"),
            ("success_url", checkout.success),
            ("cancel_url", checkout.cancel),
        ];
        if let Some(email) = checkout.email {
            form.push(("customer_email", email));
        }
        let request = http.client(&url, false).post(&url).form(&form);
        let answer = self.answer(request).await?;
        answer
            .get("url")
            .and_then(Value::as_str)
            .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
            .map(str::to_owned)
            .ok_or(StripeError::Unreadable)
    }
}

/// One charge in the form of the history of the donate item: the day and the time,
/// the e-mail address of the payer, and the amount in dollars. Nothing for a charge
/// that took no money.
pub fn donation(charge: &Value) -> Option<Value> {
    let cents = charge.get("amount_captured").and_then(Value::as_i64)?;
    if cents <= 0 {
        return None;
    }
    let created = charge.get("created").and_then(Value::as_i64).unwrap_or(0);
    let date = jiff::Timestamp::from_second(created)
        .unwrap_or(jiff::Timestamp::UNIX_EPOCH)
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    // PHP writes a whole number of dollars without a fraction.
    let amount = if cents.rem_euclid(100) == 0 {
        serde_json::json!(cents / 100)
    } else {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an amount of money in cents is far below the numbers that a float cannot hold"
        )]
        let dollars = cents as f64 / 100.0;
        serde_json::json!(dollars)
    };
    Some(serde_json::json!({
        "date": date,
        "email": charge.pointer("/billing_details/email").cloned().unwrap_or(Value::Null),
        "amount": amount,
    }))
}

/// The e-mail address of the payer of a charge, in small letters.
pub fn payer(charge: &Value) -> String {
    charge
        .pointer("/billing_details/email")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{StripeError, donation, payer, refusal};

    #[test]
    fn charge_reads_as_a_donation() {
        let charge = json!({
            "amount_captured": 1250, "created": 1_795_168_800,
            "billing_details": {"email": "Robin@Example.com"},
        });
        assert_eq!(
            donation(&charge),
            Some(
                json!({"date": "2026-11-20T10:00:00Z", "email": "Robin@Example.com", "amount": 12.5})
            )
        );
        assert_eq!(payer(&charge), "robin@example.com");
        assert_eq!(
            donation(&json!({"amount_captured": 500, "created": 0})),
            Some(json!({"date": "1970-01-01T00:00:00Z", "email": null, "amount": 5}))
        );
        assert_eq!(donation(&json!({"amount_captured": 0})), None);
        assert_eq!(donation(&json!({})), None);
        assert_eq!(payer(&json!({})), "");
    }

    #[test]
    fn refusal_has_the_text_of_stripe_or_the_status() {
        let StripeError::Refused(text) = refusal(
            400,
            &json!({"error": {"message": "No such product: 'prod_x'"}}),
        ) else {
            panic!("a refusal");
        };
        assert_eq!(text, "No such product: 'prod_x'");
        let StripeError::Refused(text) = refusal(502, &json!(null)) else {
            panic!("a refusal");
        };
        assert_eq!(text, "Stripe answered with the status 502");
    }
}
