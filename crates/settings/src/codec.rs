//! How a setting is stored, and which Rust type it has in the application.
//!
//! The PHP application keeps its settings as PHP values: a boolean, a string, or a number
//! written as a string (`'7'`). The API returns them in that form, so the stored form
//! stays. A codec turns the stored form into a typed value and back.

use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;

use serde_json::Value;

/// Converts between the stored form of a setting and its typed value.
pub trait Codec {
    /// The type the application works with.
    type Value;

    /// Whether the value is a secret, such as a password or the token of a service.
    /// The API never shows a secret.
    const SECRET: bool = false;

    /// Reads a stored value. The error says what is wrong with it.
    fn decode(stored: &Value) -> Result<Self::Value, String>;

    /// Writes a value in its stored form.
    fn encode(value: &Self::Value) -> Value;
}

/// A switch, stored as `true` or `false`.
#[derive(Debug)]
pub struct Flag;

impl Codec for Flag {
    type Value = bool;

    fn decode(stored: &Value) -> Result<bool, String> {
        stored
            .as_bool()
            .ok_or_else(|| format!("expected true or false, found {stored}"))
    }

    fn encode(value: &bool) -> Value {
        Value::Bool(*value)
    }
}

/// Free text, stored as a string.
#[derive(Debug)]
pub struct Text;

impl Codec for Text {
    type Value = String;

    fn decode(stored: &Value) -> Result<String, String> {
        stored
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("expected text, found {stored}"))
    }

    fn encode(value: &String) -> Value {
        Value::String(value.clone())
    }
}

/// A secret, stored as a string: a password, a token or a key.
#[derive(Debug)]
pub struct Secret;

impl Codec for Secret {
    type Value = String;

    const SECRET: bool = true;

    fn decode(stored: &Value) -> Result<String, String> {
        // The refusal does not repeat the value.
        stored
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "expected text".to_owned())
    }

    fn encode(value: &String) -> Value {
        Value::String(value.clone())
    }
}

/// A whole number that the PHP application stores as a string, such as `"7"`.
///
/// A stored JSON number is read too, because a hand-edited PHP config file can hold one.
/// The value is always written back as a string.
#[derive(Debug)]
pub struct TextNumber<T>(PhantomData<T>);

impl<T> Codec for TextNumber<T>
where
    T: FromStr + Display,
{
    type Value = T;

    fn decode(stored: &Value) -> Result<T, String> {
        let digits = match stored {
            Value::String(text) => text.clone(),
            Value::Number(number) => number.to_string(),
            other => return Err(format!("expected a whole number, found {other}")),
        };
        digits
            .parse()
            .map_err(|_| format!("expected a whole number, found {stored}"))
    }

    fn encode(value: &T) -> Value {
        Value::String(value.to_string())
    }
}

/// A value from a fixed list, stored as the text of the chosen entry.
#[derive(Debug)]
pub struct Choice<T>(PhantomData<T>);

/// A type whose values are a fixed list of texts.
pub trait TextChoice: Sized + Copy + 'static {
    /// Every value, in the order the settings form shows them.
    const ALL: &'static [Self];

    /// The stored text of this value.
    fn as_str(self) -> &'static str;
}

impl<T: TextChoice> Codec for Choice<T> {
    type Value = T;

    fn decode(stored: &Value) -> Result<T, String> {
        let allowed = || {
            T::ALL
                .iter()
                .map(|choice| choice.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let text = stored
            .as_str()
            .ok_or_else(|| format!("expected one of {}, found {stored}", allowed()))?;
        T::ALL
            .iter()
            .copied()
            .find(|choice| choice.as_str() == text)
            .ok_or_else(|| format!("expected one of {}, found {stored}", allowed()))
    }

    fn encode(value: &T) -> Value {
        Value::String(value.as_str().to_owned())
    }
}

/// A whole number, stored as a JSON number.
///
/// Most numbers of the PHP application are stored as text, see [`TextNumber`]. This codec
/// is for the few that are not.
#[derive(Debug)]
pub struct Number<T>(PhantomData<T>);

impl<T> Codec for Number<T>
where
    T: TryFrom<u64> + Into<u64> + Copy,
{
    type Value = T;

    fn decode(stored: &Value) -> Result<T, String> {
        stored
            .as_u64()
            .and_then(|number| T::try_from(number).ok())
            .ok_or_else(|| format!("expected a whole number, found {stored}"))
    }

    fn encode(value: &T) -> Value {
        Value::from((*value).into())
    }
}

/// A value of any form, kept as it is. The default of the PHP application for such a
/// setting is an empty text, and a plugin stores a list or a map there later.
#[derive(Debug)]
pub struct Anything;

impl Codec for Anything {
    type Value = Value;

    fn decode(stored: &Value) -> Result<Value, String> {
        Ok(stored.clone())
    }

    fn encode(value: &Value) -> Value {
        value.clone()
    }
}

/// A list, stored as a JSON list. The entries are kept as they are.
#[derive(Debug)]
pub struct List;

impl Codec for List {
    type Value = Vec<Value>;

    fn decode(stored: &Value) -> Result<Vec<Value>, String> {
        stored
            .as_array()
            .cloned()
            .ok_or_else(|| format!("expected a list, found {stored}"))
    }

    fn encode(value: &Vec<Value>) -> Value {
        Value::Array(value.clone())
    }
}
