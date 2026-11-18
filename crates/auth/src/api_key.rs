//! The API key of the install: a secret that gives a script the access of an Admin.

use subtle::ConstantTimeEq;

/// How many characters an API key has. The PHP application takes no key of another
/// length, so an install with such a key has no API key that works.
pub const API_KEY_LENGTH: usize = 20;

/// Whether `presented` is the API key `configured` for the install.
///
/// Both must have exactly [`API_KEY_LENGTH`] characters. The comparison takes the same
/// time wherever the two differ, so the time of an answer tells nothing about the key.
///
/// The PHP application compares with `==`, which compares two texts that look like
/// numbers as numbers. For a key that looks like a number, PHP therefore takes other
/// spellings of the same number for the key. Here every character must be the same.
pub fn is_api_key(presented: &str, configured: &str) -> bool {
    presented.len() == API_KEY_LENGTH
        && configured.len() == API_KEY_LENGTH
        && bool::from(presented.as_bytes().ct_eq(configured.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::is_api_key;

    const KEY: &str = "parityapikey01234567";

    #[test]
    fn key_of_the_install_is_accepted() {
        assert!(is_api_key(KEY, KEY));
    }

    #[test]
    fn any_other_text_is_refused() {
        for presented in [
            "",
            "parityapikey0123456",
            "parityapikey012345678",
            "parityapikey01234568",
            "Parityapikey01234567",
            "PARITYAPIKEY01234567",
            " parityapikey0123456",
        ] {
            assert!(!is_api_key(presented, KEY), "{presented:?}");
        }
    }

    #[test]
    fn install_without_a_key_of_twenty_characters_has_no_api_key() {
        for configured in ["", "short", "a-key-that-is-longer-than-twenty"] {
            assert!(!is_api_key(configured, configured), "{configured:?}");
            assert!(!is_api_key("", configured), "{configured:?}");
        }
    }

    #[test]
    fn keys_that_look_like_numbers_are_compared_as_text() {
        // The key of the install is the number 10, written with 20 characters. PHP 8.5
        // accepts each of these other spellings of 10 as that key: its `==` compares
        // two texts that look like numbers as numbers.
        let key = "00000000000000000010";
        for presented in [
            "000000000000000001e1",
            "0000000000000000010.",
            "000000000000000010.0",
            "                  10",
            "10                  ",
        ] {
            assert_eq!(presented.len(), 20);
            assert!(!is_api_key(presented, key), "{presented:?}");
        }
        assert!(is_api_key(key, key));
    }

    #[test]
    fn length_is_counted_in_bytes_as_php_counts_it() {
        // Ten two-byte characters: 20 bytes.
        let key = "éééééééééé";
        assert_eq!(key.len(), 20);
        assert!(is_api_key(key, key));
    }
}
