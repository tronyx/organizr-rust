//! The fields of a request body, read as the PHP application reads them.
//!
//! PHP's `apiData()` decides by the `Content-Type` header:
//!
//! - a type that contains `application/json`, in any letter case: the body is JSON. Only
//!   a JSON object has fields. Any other JSON value, and text that is not JSON, has none.
//! - any other type, on a `POST` request only: the body is a form
//!   (`application/x-www-form-urlencoded`). When a name comes twice, the last one counts.
//!
//! A body without fields is not an error. Each route decides what a missing field means.

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{FromRequest, Request};
use axum::http::{Method, header};
use serde_json::{Map, Value};

/// The fields of a request body. Use it as an extractor.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApiData {
    fields: Map<String, Value>,
}

impl ApiData {
    /// Reads a body.
    pub fn parse(method: &Method, content_type: Option<&str>, body: &[u8]) -> Self {
        let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
        let fields = if content_type.contains("application/json") {
            match serde_json::from_slice(body) {
                Ok(Value::Object(fields)) => fields,
                _ => Map::new(),
            }
        } else if method == Method::POST && content_type.contains("x-www-form-urlencoded") {
            form_urlencoded::parse(body)
                .map(|(name, value)| (name.into_owned(), Value::String(value.into_owned())))
                .collect()
        } else {
            Map::new()
        };
        Self { fields }
    }

    /// Whether a field has a value. A field that is `null` has none, as with PHP's
    /// `isset()`. A field that is `false` or empty has one.
    pub fn is_set(&self, name: &str) -> bool {
        self.fields.get(name).is_some_and(|value| !value.is_null())
    }

    /// A field as text, as PHP reads a value where it expects text: a number becomes its
    /// digits, `true` becomes `1` and `false` becomes the empty text.
    ///
    /// Returns `None` when the field is missing, `null`, a list or an object.
    pub fn text(&self, name: &str) -> Option<String> {
        match self.fields.get(name)? {
            Value::String(text) => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(true) => Some("1".to_owned()),
            Value::Bool(false) => Some(String::new()),
            Value::Null | Value::Array(_) | Value::Object(_) => None,
        }
    }
}

impl<S: Send + Sync> FromRequest<S> for ApiData {
    type Rejection = BytesRejection;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let method = request.method().clone();
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = Bytes::from_request(request, state).await?;
        Ok(Self::parse(&method, content_type.as_deref(), &body))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::Method;

    use super::ApiData;

    const JSON: Option<&str> = Some("application/json");
    const FORM: Option<&str> = Some("application/x-www-form-urlencoded");

    fn json(body: &str) -> ApiData {
        ApiData::parse(&Method::POST, JSON, body.as_bytes())
    }

    #[test]
    fn json_object_gives_its_fields() {
        let data = json(r#"{"username": "admin", "remember": false, "nothing": null}"#);

        assert_eq!(data.text("username").as_deref(), Some("admin"));
        assert_eq!(data.text("missing"), None);
        assert_eq!(data.text("nothing"), None);
        assert!(data.is_set("username"));
        assert!(data.is_set("remember"), "false is a value");
        assert!(!data.is_set("nothing"), "null is no value");
        assert!(!data.is_set("missing"));
    }

    #[test]
    fn json_type_is_found_in_any_letter_case_and_with_a_charset() {
        for content_type in [
            "application/json",
            "Application/JSON; charset=utf-8",
            "application/json;charset=UTF-8",
        ] {
            let data = ApiData::parse(&Method::POST, Some(content_type), br#"{"a": "b"}"#);

            assert_eq!(data.text("a").as_deref(), Some("b"), "{content_type}");
        }
    }

    #[test]
    fn json_is_read_for_every_method() {
        for method in [Method::POST, Method::PUT, Method::PATCH, Method::DELETE] {
            let data = ApiData::parse(&method, JSON, br#"{"a": "b"}"#);

            assert_eq!(data.text("a").as_deref(), Some("b"), "{method}");
        }
    }

    #[test]
    fn json_that_is_no_object_has_no_fields() {
        for body in [
            "",
            "null",
            "\"admin\"",
            "5",
            r#"["username", "admin"]"#,
            r#"{"username": "admin", "#,
            "username=admin",
        ] {
            assert_eq!(json(body), ApiData::default(), "{body:?}");
        }
    }

    #[test]
    fn text_is_made_from_a_value_as_php_makes_it() {
        let data = json(
            r#"{"text": " a B ", "empty": "", "whole": 123, "negative": -5, "yes": true,
                "no": false, "list": ["admin"], "object": {"a": 1}}"#,
        );

        assert_eq!(data.text("text").as_deref(), Some(" a B "));
        assert_eq!(data.text("empty").as_deref(), Some(""));
        assert_eq!(data.text("whole").as_deref(), Some("123"));
        assert_eq!(data.text("negative").as_deref(), Some("-5"));
        assert_eq!(data.text("yes").as_deref(), Some("1"));
        assert_eq!(data.text("no").as_deref(), Some(""));
        assert_eq!(data.text("list"), None);
        assert_eq!(data.text("object"), None);
        assert!(data.is_set("list"));
    }

    #[test]
    fn form_gives_its_fields_as_text() {
        let data = ApiData::parse(
            &Method::POST,
            Some("application/x-www-form-urlencoded; charset=UTF-8"),
            b"username=%61dmin&password=a+b%26c%3Dd&remember=on&empty=&twice=1&twice=2",
        );

        assert_eq!(data.text("username").as_deref(), Some("admin"));
        assert_eq!(data.text("password").as_deref(), Some("a b&c=d"));
        assert_eq!(data.text("empty").as_deref(), Some(""));
        assert_eq!(data.text("twice").as_deref(), Some("2"));
        assert!(data.is_set("remember"));
        assert!(!data.is_set("missing"));
    }

    #[test]
    fn form_is_read_for_post_only() {
        for method in [Method::PUT, Method::PATCH, Method::DELETE, Method::GET] {
            let data = ApiData::parse(&method, FORM, b"a=b");

            assert_eq!(data, ApiData::default(), "{method}");
        }
    }

    #[test]
    fn body_is_not_guessed_from_its_content() {
        // JSON without the JSON type, and JSON sent as a form.
        assert_eq!(
            ApiData::parse(&Method::POST, None, br#"{"username": "admin"}"#),
            ApiData::default()
        );
        let as_form = ApiData::parse(&Method::POST, FORM, br#"{"username": "admin"}"#);
        assert_eq!(as_form.text("username"), None);
        // A form with the JSON type.
        assert_eq!(
            ApiData::parse(&Method::POST, JSON, b"username=admin"),
            ApiData::default()
        );
    }
}
