//! Reads the configuration file of the PHP application without running PHP.
//!
//! The file is PHP code that returns one array:
//!
//! ```php
//! <?php
//! return [
//!     'title' => 'Organizr',
//!     'homepagePlexStreams' => true,
//!     'ignoredNewsIds' => [4, 7],
//! ];
//! ```
//!
//! The reader takes what such a file can hold: arrays in the forms `[...]` and
//! `array(...)`, texts in single and double quotes, numbers, `true`, `false` and `null`,
//! and comments. Everything else, for example a function call or a variable, is an error:
//! the reader never evaluates code.

use serde_json::{Map, Number, Value};
use thiserror::Error;

/// Why a configuration file cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PhpConfigError {
    /// The text is not what the reader takes.
    #[error("line {line}: expected {expected}")]
    Syntax {
        /// The line of the file, counted from 1.
        line: usize,
        /// What the reader takes at that place.
        expected: &'static str,
    },
    /// The file returns something that is not an array of settings.
    #[error("the file does not return an array with a name for each value")]
    NotSettings,
}

/// The settings of a configuration file, in the order of the file. A key that the file
/// holds twice is there once, with its last value, as in PHP.
pub fn parse(text: &str) -> Result<Vec<(String, Value)>, PhpConfigError> {
    let mut reader = Reader {
        text: text.strip_prefix('\u{feff}').unwrap_or(text),
        at: 0,
    };
    reader.skip_space();
    reader.take("<?php", "`<?php` at the start of the file")?;
    reader.skip_space();
    reader.take_word("return", "`return`")?;
    let Element::Array(entries) = reader.value()? else {
        return Err(PhpConfigError::NotSettings);
    };
    reader.skip_space();
    reader.take(";", "`;` after the array")?;
    reader.skip_space();
    if reader.rest().starts_with("?>") {
        reader.at += 2;
        reader.skip_space();
    }
    if !reader.rest().is_empty() {
        return Err(reader.error("the end of the file"));
    }

    let mut settings: Vec<(String, Value)> = Vec::new();
    for (key, element) in entries {
        let Some(Key::Text(key)) = key else {
            return Err(PhpConfigError::NotSettings);
        };
        let value = element.into_json();
        match settings.iter_mut().find(|(name, _)| *name == key) {
            Some(setting) => setting.1 = value,
            None => settings.push((key, value)),
        }
    }
    Ok(settings)
}

/// The key of an array entry.
#[derive(Debug, Clone, PartialEq)]
enum Key {
    Number(i64),
    Text(String),
}

/// A value as the file holds it.
#[derive(Debug, Clone, PartialEq)]
enum Element {
    Scalar(Value),
    Array(Vec<(Option<Key>, Element)>),
}

impl Element {
    /// The value as JSON. An array whose keys count up from zero is a list, as in the
    /// JSON that PHP writes. Every other array is an object.
    fn into_json(self) -> Value {
        let entries = match self {
            Self::Scalar(value) => return value,
            Self::Array(entries) => entries,
        };
        // PHP gives an entry without a key the number after the highest number so far.
        let mut next = 0_i64;
        let mut keyed: Vec<(Key, Value)> = Vec::new();
        for (key, element) in entries {
            let key = key.unwrap_or(Key::Number(next));
            if let Key::Number(number) = key {
                next = next.max(number.saturating_add(1));
            }
            let value = element.into_json();
            match keyed.iter_mut().find(|(known, _)| *known == key) {
                Some(entry) => entry.1 = value,
                None => keyed.push((key, value)),
            }
        }

        let is_list = keyed
            .iter()
            .zip(0_i64..)
            .all(|((key, _), index)| *key == Key::Number(index));
        if is_list {
            return Value::Array(keyed.into_iter().map(|(_, value)| value).collect());
        }
        let mut object = Map::new();
        for (key, value) in keyed {
            let name = match key {
                Key::Number(number) => number.to_string(),
                Key::Text(text) => text,
            };
            object.insert(name, value);
        }
        Value::Object(object)
    }
}

struct Reader<'a> {
    text: &'a str,
    /// The byte position of the next character.
    at: usize,
}

impl<'a> Reader<'a> {
    fn rest(&self) -> &'a str {
        self.text.get(self.at..).unwrap_or("")
    }

    fn error(&self, expected: &'static str) -> PhpConfigError {
        let before = self.text.get(..self.at).unwrap_or(self.text);
        PhpConfigError::Syntax {
            line: before.matches('\n').count() + 1,
            expected,
        }
    }

    /// Skips white space and comments.
    fn skip_space(&mut self) {
        loop {
            let rest = self.rest();
            let trimmed = rest.trim_start();
            self.at += rest.len() - trimmed.len();
            let comment_end = if trimmed.starts_with("//") || trimmed.starts_with('#') {
                Some(trimmed.find('\n').unwrap_or(trimmed.len()))
            } else if trimmed.starts_with("/*") {
                Some(trimmed.find("*/").map_or(trimmed.len(), |end| end + 2))
            } else {
                None
            };
            match comment_end {
                Some(end) => self.at += end,
                None => return,
            }
        }
    }

    fn take(&mut self, token: &str, expected: &'static str) -> Result<(), PhpConfigError> {
        if self.rest().starts_with(token) {
            self.at += token.len();
            Ok(())
        } else {
            Err(self.error(expected))
        }
    }

    /// Takes a word such as `return`, in any letter case, when no other word character
    /// follows it.
    fn take_word(&mut self, word: &str, expected: &'static str) -> Result<(), PhpConfigError> {
        if self.starts_with_word(word) {
            self.at += word.len();
            Ok(())
        } else {
            Err(self.error(expected))
        }
    }

    fn starts_with_word(&self, word: &str) -> bool {
        let rest = self.rest();
        let Some(start) = rest.get(..word.len()) else {
            return false;
        };
        let after = rest.get(word.len()..).and_then(|rest| rest.chars().next());
        start.eq_ignore_ascii_case(word)
            && !after.is_some_and(|character| character.is_alphanumeric() || character == '_')
    }

    fn value(&mut self) -> Result<Element, PhpConfigError> {
        self.skip_space();
        let rest = self.rest();
        if rest.starts_with('[') {
            self.at += 1;
            return self.array(']');
        }
        if self.starts_with_word("array") {
            self.at += "array".len();
            self.skip_space();
            self.take("(", "`(` after `array`")?;
            return self.array(')');
        }
        if rest.starts_with('\'') {
            return Ok(Element::Scalar(Value::String(self.single_quoted()?)));
        }
        if rest.starts_with('"') {
            return Ok(Element::Scalar(Value::String(self.double_quoted()?)));
        }
        for (word, value) in [
            ("true", Value::Bool(true)),
            ("false", Value::Bool(false)),
            ("null", Value::Null),
        ] {
            if self.starts_with_word(word) {
                self.at += word.len();
                return Ok(Element::Scalar(value));
            }
        }
        self.number().map(Element::Scalar)
    }

    /// The entries of an array, after its opening bracket, up to and with `close`.
    fn array(&mut self, close: char) -> Result<Element, PhpConfigError> {
        let mut entries = Vec::new();
        loop {
            self.skip_space();
            if self.rest().starts_with(close) {
                self.at += close.len_utf8();
                return Ok(Element::Array(entries));
            }
            let first = self.value()?;
            self.skip_space();
            if self.rest().starts_with("=>") {
                self.at += 2;
                let key = match first {
                    Element::Scalar(Value::String(text)) => key_of_text(text),
                    Element::Scalar(Value::Number(number)) if number.is_i64() => {
                        Key::Number(number.as_i64().unwrap_or_default())
                    }
                    _ => return Err(self.error("a text or a whole number as the key")),
                };
                entries.push((Some(key), self.value()?));
            } else {
                entries.push((None, first));
            }
            self.skip_space();
            if self.rest().starts_with(',') {
                self.at += 1;
            } else if !self.rest().starts_with(close) {
                return Err(self.error("`,` or the end of the array"));
            }
        }
    }

    /// A text in single quotes. Only `\\` and `\'` are escapes.
    fn single_quoted(&mut self) -> Result<String, PhpConfigError> {
        self.at += 1;
        let mut text = String::new();
        let mut characters = self.rest().char_indices();
        while let Some((index, character)) = characters.next() {
            match character {
                '\'' => {
                    self.at += index + 1;
                    return Ok(text);
                }
                '\\' => match characters.clone().next() {
                    Some((_, escaped @ ('\\' | '\''))) => {
                        text.push(escaped);
                        characters.next();
                    }
                    _ => text.push('\\'),
                },
                other => text.push(other),
            }
        }
        self.at = self.text.len();
        Err(self.error("`'` at the end of the text"))
    }

    /// A text in double quotes. A variable in it is an error, because the reader does
    /// not evaluate code.
    fn double_quoted(&mut self) -> Result<String, PhpConfigError> {
        self.at += 1;
        let mut text = String::new();
        let mut characters = self.rest().char_indices();
        while let Some((index, character)) = characters.next() {
            match character {
                '"' => {
                    self.at += index + 1;
                    return Ok(text);
                }
                '$' => {
                    self.at += index;
                    return Err(self.error("a text without a variable"));
                }
                '\\' => {
                    let escaped = match characters.clone().next() {
                        Some((_, 'n')) => Some('\n'),
                        Some((_, 't')) => Some('\t'),
                        Some((_, 'r')) => Some('\r'),
                        Some((_, 'v')) => Some('\u{b}'),
                        Some((_, 'e')) => Some('\u{1b}'),
                        Some((_, 'f')) => Some('\u{c}'),
                        Some((_, '0')) => Some('\0'),
                        Some((_, same @ ('\\' | '"' | '$'))) => Some(same),
                        _ => None,
                    };
                    match escaped {
                        Some(escaped) => {
                            text.push(escaped);
                            characters.next();
                        }
                        None => text.push('\\'),
                    }
                }
                other => text.push(other),
            }
        }
        self.at = self.text.len();
        Err(self.error("`\"` at the end of the text"))
    }

    fn number(&mut self) -> Result<Value, PhpConfigError> {
        let rest = self.rest();
        let length = rest
            .char_indices()
            .find(|(index, character)| {
                let sign = matches!(character, '-' | '+')
                    && (*index == 0
                        || rest
                            .get(..*index)
                            .is_some_and(|before| before.ends_with(['e', 'E'])));
                !(character.is_ascii_digit() || matches!(character, '.' | 'e' | 'E') || sign)
            })
            .map_or(rest.len(), |(index, _)| index);
        let digits = rest.get(..length).unwrap_or("");
        let value = if let Ok(whole) = digits.parse::<i64>() {
            Some(Value::Number(Number::from(whole)))
        } else {
            digits
                .parse::<f64>()
                .ok()
                .filter(|_| digits.bytes().any(|byte| byte.is_ascii_digit()))
                .and_then(Number::from_f64)
                .map(Value::Number)
        };
        match value {
            Some(value) => {
                self.at += length;
                Ok(value)
            }
            None => Err(self.error("a value: an array, a text, a number, true, false or null")),
        }
    }
}

/// The key that PHP makes of a text: a whole number in its usual form becomes a number.
fn key_of_text(text: String) -> Key {
    match text.parse::<i64>() {
        Ok(number) if number.to_string() == text => Key::Number(number),
        _ => Key::Text(text),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{PhpConfigError, parse};

    fn settings(text: &str) -> Vec<(String, Value)> {
        parse(text).unwrap_or_else(|error| panic!("the file is read: {error}"))
    }

    fn one(value: &str) -> Value {
        let mut settings = settings(&format!("<?php\nreturn ['key' => {value}];"));
        assert_eq!(settings.len(), 1);
        settings.remove(0).1
    }

    #[test]
    fn file_gives_its_settings_in_order() {
        let text = "<?php\nreturn [\n\t'title' => 'Organizr',\n\t'debug' => false,\n\t'count' => 100\n];\n";

        assert_eq!(
            settings(text),
            [
                ("title".to_owned(), json!("Organizr")),
                ("debug".to_owned(), json!(false)),
                ("count".to_owned(), json!(100)),
            ]
        );
    }

    #[test]
    fn old_array_form_comments_and_a_closing_tag_are_read() {
        let text = "\u{feff}<?php\n// A comment.\n# Another one.\n/* And\n a third. */\nRETURN Array(\n  'a' => TRUE, // after a value\n  'b' => NULL,\n);\n?>\n";

        assert_eq!(
            settings(text),
            [("a".to_owned(), json!(true)), ("b".to_owned(), Value::Null)]
        );
    }

    #[test]
    fn texts_keep_what_php_keeps() {
        assert_eq!(one(r"'it\'s a \\ and a \n'"), json!("it's a \\ and a \\n"));
        assert_eq!(
            one(r"'ends with a backslash\\'"),
            json!("ends with a backslash\\")
        );
        assert_eq!(
            one("'two\nlines and ünïcödé'"),
            json!("two\nlines and ünïcödé")
        );
        assert_eq!(
            one(r#""a \"quoted\" tab\t and \$ and \q""#),
            json!("a \"quoted\" tab\t and $ and \\q")
        );
        assert_eq!(one("''"), json!(""));
    }

    #[test]
    fn numbers_are_whole_or_not() {
        assert_eq!(one("0"), json!(0));
        assert_eq!(one("-12"), json!(-12));
        assert_eq!(one("1.5"), json!(1.5));
        assert_eq!(one("-2.5e3"), json!(-2500.0));
        assert_eq!(one("1E-2"), json!(0.01));
    }

    #[test]
    fn array_is_a_list_only_when_its_keys_count_up_from_zero() {
        assert_eq!(one("[]"), json!([]));
        assert_eq!(one("array()"), json!([]));
        assert_eq!(one("[4, '7', [true]]"), json!([4, "7", [true]]));
        assert_eq!(one("[0 => 'a', 1 => 'b']"), json!(["a", "b"]));
        assert_eq!(one("['0' => 'a', 'b']"), json!(["a", "b"]));
        assert_eq!(one("[1 => 'a', 2 => 'b']"), json!({"1": "a", "2": "b"}));
        assert_eq!(one("[5 => 'a', 'b']"), json!({"5": "a", "6": "b"}));
        assert_eq!(
            one("['name' => 'x', 'list' => [1, 2], 'x',]"),
            json!({"name": "x", "list": [1, 2], "0": "x"})
        );
        // A text that is no usual whole number stays a text key.
        assert_eq!(one("['01' => 'a']"), json!({"01": "a"}));
    }

    #[test]
    fn key_that_is_there_twice_keeps_its_place_and_gets_the_last_value() {
        let text = "<?php return ['a' => 1, 'b' => 2, 'a' => 3];";

        assert_eq!(
            settings(text),
            [("a".to_owned(), json!(3)), ("b".to_owned(), json!(2))]
        );
        assert_eq!(one("[0 => 'a', 0 => 'b']"), json!(["b"]));
    }

    #[test]
    fn code_is_never_evaluated() {
        for (text, line) in [
            ("<?php\nreturn ['a' => getenv('HOME')];", 2),
            ("<?php\nreturn [\n'a' => $secret];", 3),
            ("<?php\nreturn ['a' => \"$secret\"];", 2),
            ("<?php\nreturn ['a' => 'x' . 'y'];", 2),
            ("<?php\nreturn ['a' => PHP_VERSION];", 2),
            ("<?php\nreturn ['a' => trueish];", 2),
            ("<?php\n$a = 1;\nreturn [];", 2),
            ("<?php\nreturn ['a' => 1]; echo 'x';", 2),
            ("<?php\nreturn ['a' => 'no end];", 2),
            ("<?php\nreturn ['a' => [1, 2;", 2),
            ("<?php\nreturn [[1] => 2];", 2),
            ("return ['a' => 1];", 1),
            ("<?php\nreturn ['a' => -];", 2),
        ] {
            match parse(text) {
                Err(PhpConfigError::Syntax { line: found, .. }) => {
                    assert_eq!(found, line, "{text}");
                }
                other => panic!("{text} gave {other:?}"),
            }
        }
    }

    #[test]
    fn file_must_return_named_settings() {
        for text in [
            "<?php return 'text';",
            "<?php return ['a', 'b'];",
            "<?php return [1 => 'a'];",
        ] {
            assert_eq!(parse(text), Err(PhpConfigError::NotSettings), "{text}");
        }
        assert_eq!(parse("<?php return [];"), Ok(Vec::new()));
    }
}
