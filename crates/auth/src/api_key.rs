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
/// The PHP application compares with `==`, which reads two texts of digits as numbers
/// and takes some wrong keys for the right one. Here every character must be the same.
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
    fn keys_of_digits_are_compared_as_text() {
        // PHP takes these two for the same key: as numbers, both are 1.2345678901234567e19.
        assert!(!is_api_key("12345678901234567890", "12345678901234567891"));
        assert!(!is_api_key("00000000000000000010", "0000000000000000001e1"));
    }

    #[test]
    fn length_is_counted_in_bytes_as_php_counts_it() {
        // Ten two-byte characters: 20 bytes.
        let key = "éééééééééé";
        assert_eq!(key.len(), 20);
        assert!(is_api_key(key, key));
    }
}
