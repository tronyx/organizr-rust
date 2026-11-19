//! Opens the values that the PHP application encrypted in its configuration file.
//!
//! PHP Organizr encrypts each value whose key holds the word "password":
//!
//! ```php
//! openssl_encrypt($value, 'AES-256-CBC', $hash, 0, fillString($hash, 16))
//! ```
//!
//! `$hash` is the `organizrHash` setting. OpenSSL makes the 32 bytes of the key from it:
//! a shorter text is filled up with zero bytes and a longer one is cut. `fillString`
//! makes the 16 bytes that the cipher starts with: a shorter text is filled up from a
//! fixed text and a longer one is cut. The result is in Base64.
//!
//! The importer only opens such values. New secrets are sealed in another way.

use aes::Aes256;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockModeDecrypt, KeyIvInit};

/// What `fillString` of the PHP application fills a short text up with.
const FILLER: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz!@#$%^&*";

const KEY_LENGTH: usize = 32;
const START_LENGTH: usize = 16;

/// The plain text of a value that the PHP application encrypted with `hash`, or `None`
/// when the value is not such a text: it is no Base64, it was encrypted with another
/// hash, or the result is no UTF-8 text.
pub fn decrypt(encrypted: &str, hash: &str) -> Option<String> {
    let bytes = STANDARD.decode(encrypted).ok()?;
    let hash = hash.as_bytes();

    let mut key = [0_u8; KEY_LENGTH];
    for (target, source) in key.iter_mut().zip(hash) {
        *target = *source;
    }
    let mut start = [0_u8; START_LENGTH];
    for (target, source) in start.iter_mut().zip(hash.iter().chain(FILLER)) {
        *target = *source;
    }

    let plain = cbc::Decryptor::<Aes256>::new(&key.into(), &start.into())
        .decrypt_padded_vec::<Pkcs7>(&bytes)
        .ok()?;
    String::from_utf8(plain).ok()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::decrypt;

    /// Made by real PHP: `parity/php-legacy-aes/generate.php`.
    const FIXTURE: &str = include_str!("../tests/fixtures/php_legacy_aes.json");

    #[derive(Deserialize)]
    struct Fixture {
        cases: Vec<Case>,
    }

    #[derive(Deserialize)]
    struct Case {
        key: String,
        plain: String,
        encrypted: String,
    }

    #[test]
    fn values_that_php_encrypted_are_opened() {
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("the fixture is JSON");
        assert_eq!(fixture.cases.len(), 42);

        for case in fixture.cases {
            assert_eq!(
                decrypt(&case.encrypted, &case.key).as_deref(),
                Some(case.plain.as_str()),
                "key {:?}",
                case.key
            );
        }
    }

    #[test]
    fn other_text_is_not_opened() {
        // Encrypted by PHP with the key `short-key`: "made-up password".
        let fixture: Fixture = serde_json::from_str(FIXTURE).expect("the fixture is JSON");
        let case = fixture
            .cases
            .iter()
            .find(|case| case.key == "short-key" && case.plain == "made-up password")
            .expect("the case is in the fixture");

        assert_eq!(decrypt(&case.encrypted, "another-key"), None);
        assert_eq!(decrypt("a plain password", "short-key"), None);
        assert_eq!(decrypt("not base64 !", "short-key"), None);
        assert_eq!(decrypt("", "short-key"), None);
        // Base64, but not a whole number of cipher blocks.
        assert_eq!(decrypt("YWJj", "short-key"), None);
    }
}
