//! Reading the fields of a request that adds or changes a record of the menu: a tab or a
//! category.
//!
//! The PHP application stores whatever a request sends, in any field. Here a value must
//! be of the kind its field takes. A request with another value gets `422` and the text
//! `<Record> <field> is not valid`.

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_db::DbError;
use organizr_domain::Change;
use organizr_domain::email::php_trim;
use serde_json::Value;

use crate::api_data::ApiData;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use organizr_auth::registration::RegistrationError;

/// Why a request to add or change a record is refused.
#[derive(Debug)]
pub enum Refusal {
    /// The request is at fault. The caller gets the status and the text.
    Request(StatusCode, String),
    /// The request is at fault, and the answer has `data` that says more.
    RequestWithData(StatusCode, String, Value),
    /// Any other failure.
    Api(ApiError),
}

impl Refusal {
    /// `422` with the given text.
    pub fn unprocessable(message: &str) -> Self {
        Self::Request(StatusCode::UNPROCESSABLE_ENTITY, message.to_owned())
    }

    /// `409` with the given text.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Request(StatusCode::CONFLICT, message.into())
    }

    /// `404` with the given text.
    pub fn not_found(message: &str) -> Self {
        Self::Request(StatusCode::NOT_FOUND, message.to_owned())
    }

    /// A value that is not of the kind the field takes. `record` is the word for the
    /// record in the text: `Tab` or `Category`.
    pub fn not_valid(record: &str, field: &str) -> Self {
        Self::unprocessable(&format!("{record} {field} is not valid"))
    }

    /// A text that is not an e-mail address. The answer repeats the text, as the PHP
    /// application does.
    pub fn invalid_email(email: &str) -> Self {
        Self::RequestWithData(
            StatusCode::CONFLICT,
            "Email is not a valid email".to_owned(),
            serde_json::json!({ "email": email }),
        )
    }

    /// `401` with the given text: the caller may not do this.
    pub fn not_authorized(message: &str) -> Self {
        Self::Request(StatusCode::UNAUTHORIZED, message.to_owned())
    }

    /// A name that is longer than the limit, in the words of the PHP application.
    pub fn over_limit(limit: usize) -> Self {
        Self::conflict(format!("String is over limit of: {limit}"))
    }
}

impl From<ApiError> for Refusal {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

/// What is wrong with the values for a new account, in the words of the PHP application.
impl From<RegistrationError> for Refusal {
    fn from(error: RegistrationError) -> Self {
        match error {
            RegistrationError::EmptyUsername => Self::conflict("Username was set but empty"),
            RegistrationError::InvalidEmail { email } => Self::invalid_email(&email),
            RegistrationError::Taken { username, email } => Self::conflict(format!(
                "Username: {username} or Email: {email} is already taken"
            )),
            RegistrationError::TooLong { limit } => Self::over_limit(limit),
            other => Self::Api(other.into()),
        }
    }
}

impl From<DbError> for Refusal {
    fn from(error: DbError) -> Self {
        Self::Api(error.into())
    }
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        match self {
            Self::Request(status, message) => {
                Envelope::error(status, message, NoData).into_response()
            }
            Self::RequestWithData(status, message, data) => {
                Envelope::error(status, message, data).into_response()
            }
            Self::Api(error) => error.into_response(),
        }
    }
}

/// Gives an answer the content type `application/json`, without a character set. The
/// PHP application sends this form from the routes that change or delete a tab or a
/// category.
pub async fn as_plain_json(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

/// Whether the query of a request has the given key, with or without a value.
pub fn has_key(query: Option<&str>, key: &str) -> bool {
    query.is_some_and(|query| form_urlencoded::parse(query.as_bytes()).any(|(name, _)| name == key))
}

/// Whether PHP takes a value for "nothing": `null`, `false`, 0, the empty text, the text
/// `0`, and an empty list or object.
pub fn is_nothing_in_php(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(on) => !on,
        Value::Number(number) => number.as_f64() == Some(0.0),
        Value::String(text) => text.is_empty() || text == "0",
        Value::Array(entries) => entries.is_empty(),
        Value::Object(members) => members.is_empty(),
    }
}

/// A whole number as a request can send it: a JSON number, or its digits as text.
pub fn whole_number(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => php_trim(text).parse().ok(),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

/// A value as text, as PHP makes text from it: a number becomes its digits, `true`
/// becomes `1`, and `false` and `null` become the empty text. A list or an object is no
/// text.
pub fn php_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(true) => Some("1".to_owned()),
        Value::Bool(false) | Value::Null => Some(String::new()),
        Value::Array(_) | Value::Object(_) => None,
    }
}

/// The id of a record from the path. Text that is no whole number names no record.
pub fn record_id(text: &str) -> Option<i64> {
    text.parse().ok()
}

/// One entry of a request that sets the order of records: an object with exactly the
/// keys `id` and `order`. PHP takes 0 for "no value", so neither can be 0. Returns the id
/// and the new place.
pub fn place_of(entry: &Value) -> Option<(i64, i64)> {
    let entry = entry.as_object()?;
    if entry.len() != 2 {
        return None;
    }
    let id = whole_number(entry.get("id")?)?;
    let order = whole_number(entry.get("order")?)?;
    (id != 0 && order != 0).then_some((id, order))
}

/// The value a new record gets from a field, or `None` when the request leaves the
/// choice to the application: the field is missing or "nothing".
pub fn chosen<T>(change: Change<Option<T>>) -> Option<T> {
    change.into_option().flatten()
}

/// The fields of a request, read for one kind of record.
#[derive(Debug, Clone, Copy)]
pub struct Fields<'a> {
    data: &'a ApiData,
    /// The word for the record in the text of a refusal: `Tab` or `Category`.
    record: &'static str,
}

impl<'a> Fields<'a> {
    /// Reads the fields of a body for the record with the given word.
    pub const fn new(data: &'a ApiData, record: &'static str) -> Self {
        Self { data, record }
    }

    /// The refusal for a value that is not of the kind the field takes.
    pub fn not_valid(&self, field: &str) -> Refusal {
        Refusal::not_valid(self.record, field)
    }

    /// The whole number in a field. `null` and the empty text stand for "nothing".
    pub fn number(&self, name: &str) -> Result<Change<Option<i64>>, Refusal> {
        match self.data.field(name) {
            None => Ok(Change::Keep),
            Some(Value::Null) => Ok(Change::Set(None)),
            Some(Value::String(text)) if php_trim(text).is_empty() => Ok(Change::Set(None)),
            Some(value) => whole_number(value)
                .map(|number| Change::Set(Some(number)))
                .ok_or_else(|| self.not_valid(name)),
        }
    }

    /// The switch in a field: `true` or `false`, 1 or 0, or one of these as text. `null`
    /// stands for "off".
    pub fn switch(&self, name: &str) -> Result<Change<bool>, Refusal> {
        let on = match self.data.field(name) {
            None => return Ok(Change::Keep),
            Some(Value::Null) => Some(false),
            Some(Value::Bool(on)) => Some(*on),
            Some(Value::Number(number)) => match number.as_i64() {
                Some(0) => Some(false),
                Some(1) => Some(true),
                _ => None,
            },
            Some(Value::String(text)) => match text.as_str() {
                "0" | "false" => Some(false),
                "1" | "true" => Some(true),
                _ => None,
            },
            Some(Value::Array(_) | Value::Object(_)) => None,
        };
        on.map(Change::Set).ok_or_else(|| self.not_valid(name))
    }

    /// The text in a field, as it was sent. `null` stands for "nothing".
    pub fn text(&self, name: &str) -> Result<Change<Option<String>>, Refusal> {
        match self.data.field(name) {
            None => Ok(Change::Keep),
            Some(Value::Null) => Ok(Change::Set(None)),
            Some(value) => php_text(value)
                .map(|text| Change::Set(Some(text)))
                .ok_or_else(|| self.not_valid(name)),
        }
    }

    /// The picture in the field `image`: the text without the space around it. PHP
    /// stores the empty text for `null`.
    pub fn image(&self) -> Result<Change<Option<String>>, Refusal> {
        match self.data.field("image") {
            None => Ok(Change::Keep),
            Some(value) => php_text(value)
                .map(|text| Change::Set(Some(php_trim(&text).to_owned())))
                .ok_or_else(|| self.not_valid("image")),
        }
    }

    /// A change to a field that cannot be empty: "nothing" is no value for it.
    pub fn required<T>(&self, change: Change<Option<T>>, name: &str) -> Result<Change<T>, Refusal> {
        match change {
            Change::Keep => Ok(Change::Keep),
            Change::Set(Some(value)) => Ok(Change::Set(value)),
            Change::Set(None) => Err(self.not_valid(name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    #[test]
    fn what_php_takes_for_nothing() {
        for nothing in [
            serde_json::json!(null),
            serde_json::json!(false),
            serde_json::json!(""),
            serde_json::json!("0"),
            serde_json::json!(0),
            serde_json::json!([]),
            serde_json::json!({}),
        ] {
            assert!(super::is_nothing_in_php(&nothing), "{nothing}");
        }
        for something in [
            serde_json::json!(true),
            serde_json::json!("false"),
            serde_json::json!("00"),
            serde_json::json!(" "),
            serde_json::json!(100),
            serde_json::json!([0]),
        ] {
            assert!(!super::is_nothing_in_php(&something), "{something}");
        }
    }

    #[test]
    fn key_of_a_query_counts_with_or_without_a_value() {
        assert!(super::has_key(Some("includeUsers"), "includeUsers"));
        assert!(super::has_key(
            Some("a=1&includeUsers=&b=2"),
            "includeUsers"
        ));
        assert!(super::has_key(Some("includeUsers=0"), "includeUsers"));
        assert!(!super::has_key(Some("includeusers"), "includeUsers"));
        assert!(!super::has_key(Some("a=includeUsers"), "includeUsers"));
        assert!(!super::has_key(Some(""), "includeUsers"));
        assert!(!super::has_key(None, "includeUsers"));
    }

    use super::{php_text, place_of, record_id, whole_number};

    #[test]
    fn whole_number_is_a_json_number_or_its_digits_as_text() {
        assert_eq!(whole_number(&json!(5)), Some(5));
        assert_eq!(whole_number(&json!(-3)), Some(-3));
        assert_eq!(whole_number(&json!("12")), Some(12));
        assert_eq!(whole_number(&json!(" 12 ")), Some(12));
        for other in [
            json!(1.5),
            json!("1.5"),
            json!("abc"),
            json!(""),
            json!(true),
            Value::Null,
            json!([1]),
            json!({"a": 1}),
        ] {
            assert_eq!(whole_number(&other), None, "{other}");
        }
    }

    #[test]
    fn text_is_made_from_a_value_as_php_makes_it() {
        assert_eq!(php_text(&json!(" a ")).as_deref(), Some(" a "));
        assert_eq!(php_text(&json!(5)).as_deref(), Some("5"));
        assert_eq!(php_text(&json!(true)).as_deref(), Some("1"));
        assert_eq!(php_text(&json!(false)).as_deref(), Some(""));
        assert_eq!(php_text(&Value::Null).as_deref(), Some(""));
        assert_eq!(php_text(&json!(["a"])), None);
        assert_eq!(php_text(&json!({"a": 1})), None);
    }

    #[test]
    fn id_in_a_path_is_a_whole_number() {
        assert_eq!(record_id("3"), Some(3));
        // SQLite, which the PHP application uses, reads this as 1 too.
        assert_eq!(record_id("01"), Some(1));
        for other in ["", "abc", "1abc", "1.5", " 1"] {
            assert_eq!(record_id(other), None, "{other:?}");
        }
    }

    #[test]
    fn place_is_an_object_with_an_id_and_an_order() {
        assert_eq!(place_of(&json!({"id": 1, "order": 5})), Some((1, 5)));
        assert_eq!(place_of(&json!({"order": "6", "id": "2"})), Some((2, 6)));
        assert_eq!(place_of(&json!({"id": 1, "order": -2})), Some((1, -2)));
        for malformed in [
            json!({"id": 1}),
            json!({"id": 1, "order": 5, "name": "x"}),
            json!({"id": 1, "place": 5}),
            // PHP takes 0 for "no value".
            json!({"id": 1, "order": 0}),
            json!({"id": 0, "order": 5}),
            json!({"id": 1, "order": "first"}),
            json!({"id": 1, "order": null}),
            json!([1, 5]),
            json!(1),
            json!("1"),
            Value::Null,
        ] {
            assert_eq!(place_of(&malformed), None, "{malformed}");
        }
    }
}
