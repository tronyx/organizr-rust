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
