//! The rule for the name of a tab or of a category.

use thiserror::Error;

use crate::email::php_trim;

/// The most bytes the name of a tab or of a category can have.
pub const MENU_NAME_LIMIT: usize = 50;

/// Why a text cannot be the name of a tab or of a category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum InvalidMenuName {
    /// Nothing is left when the space around the text is cut off.
    #[error("the name cannot be empty")]
    Empty,
    /// The text is longer than [`MENU_NAME_LIMIT`] bytes.
    #[error("the name cannot be longer than {MENU_NAME_LIMIT} bytes")]
    TooLong,
}

/// Makes the name of a tab or of a category from what a caller sent: the space around
/// it is cut off, as PHP's `trim` does it. The name must not be empty or longer than
/// [`MENU_NAME_LIMIT`] bytes.
pub fn menu_name(text: &str) -> Result<String, InvalidMenuName> {
    let name = php_trim(text);
    if name.is_empty() {
        Err(InvalidMenuName::Empty)
    } else if name.len() > MENU_NAME_LIMIT {
        Err(InvalidMenuName::TooLong)
    } else {
        Ok(name.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{InvalidMenuName, menu_name};

    #[test]
    fn name_loses_the_space_around_it() {
        assert_eq!(menu_name("  Plex \t\n").as_deref(), Ok("Plex"));
        assert_eq!(menu_name("My Plex").as_deref(), Ok("My Plex"));
    }

    #[test]
    fn name_cannot_be_empty_or_only_space() {
        assert_eq!(menu_name(""), Err(InvalidMenuName::Empty));
        assert_eq!(menu_name(" \t "), Err(InvalidMenuName::Empty));
    }

    #[test]
    fn name_is_limited_to_50_bytes() {
        assert_eq!(menu_name(&"x".repeat(50)), Ok("x".repeat(50)));
        assert_eq!(menu_name(&"x".repeat(51)), Err(InvalidMenuName::TooLong));
        // 26 letters of two bytes each. The space around the name does not count.
        assert_eq!(menu_name(&"é".repeat(26)), Err(InvalidMenuName::TooLong));
        assert_eq!(
            menu_name(&format!("  {}  ", "é".repeat(25))),
            Ok("é".repeat(25))
        );
    }
}
