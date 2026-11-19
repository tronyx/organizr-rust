//! Secrets at rest: the passwords, tokens and keys of other services that the install
//! stores as settings.
//!
//! A secret is stored encrypted, so that a copy of the database alone does not give it
//! away. The cipher is XChaCha20-Poly1305, which also notices a stored value that was
//! changed. The key for it is derived from the master key of the install, which lives
//! outside the database: in the environment, or in a key file in the data folder.
//!
//! Each stored value is bound to the name it is stored under. A value that is moved to
//! another name cannot be opened.

use std::fmt;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use organizr_domain::UserId;
use sha2::Sha256;
use thiserror::Error;
use zeroize::Zeroizing;

/// The length of the master key, in bytes.
const KEY_LENGTH: usize = 32;

/// The length of the number that is used once for each sealed value, in bytes.
const NONCE_LENGTH: usize = 24;

/// What a sealed value starts with. The number is the version of this format. A stored
/// value that does not start with it is plain text, for example one that a person wrote
/// into the database by hand.
pub const PREFIX: &str = "organizr-secret:v1:";

/// What the key for settings is derived for. Another use of the master key gets another
/// text, and so another key.
const PURPOSE: &[u8] = b"organizr settings v1";

/// Why a master key cannot be read or made.
#[derive(Debug, Error)]
pub enum KeyError {
    /// The text is not a key.
    #[error("the key must be 64 hex digits, which stand for 32 bytes")]
    Invalid,

    /// The system gave no random bytes.
    #[error("cannot get random bytes for a new key: {0}")]
    Random(#[source] getrandom::Error),
}

/// Why a secret cannot be sealed or opened.
#[derive(Debug, Error)]
pub enum SecretError {
    /// The stored value is damaged, was sealed with another key, or was stored under
    /// another name.
    #[error("the stored secret cannot be opened with the key of this install")]
    CannotOpen,

    /// The system gave no random bytes.
    #[error("cannot get random bytes to seal a secret: {0}")]
    Random(#[source] getrandom::Error),

    /// The cipher refused the value.
    #[error("the secret cannot be sealed")]
    CannotSeal,
}

/// The master key of an install: 32 random bytes. Whoever has it and the database can
/// read every stored secret.
pub struct MasterKey(Zeroizing<[u8; KEY_LENGTH]>);

impl MasterKey {
    /// Makes a new random key.
    pub fn generate() -> Result<Self, KeyError> {
        let mut bytes = Zeroizing::new([0_u8; KEY_LENGTH]);
        getrandom::fill(bytes.as_mut_slice()).map_err(KeyError::Random)?;
        Ok(Self(bytes))
    }

    /// Reads a key from its text form: 64 hex digits. Space around the text is ignored,
    /// so that a key file can end with a line break.
    pub fn from_hex(text: &str) -> Result<Self, KeyError> {
        let mut bytes = Zeroizing::new([0_u8; KEY_LENGTH]);
        hex::decode_to_slice(text.trim(), bytes.as_mut_slice()).map_err(|_| KeyError::Invalid)?;
        Ok(Self(bytes))
    }

    /// The key in its text form, for the key file.
    #[must_use]
    pub fn to_hex(&self) -> Zeroizing<String> {
        Zeroizing::new(hex::encode(self.0.as_slice()))
    }
}

/// The key is a secret, so `Debug` does not show it.
impl fmt::Debug for MasterKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MasterKey(<hidden>)")
    }
}

/// The name under which the token of the Plex account of a user is sealed.
pub fn plex_token_name(user: UserId) -> String {
    format!("users/{}/plex_token", user.get())
}

/// The name under which the secret of the second factor of a user is sealed.
pub fn two_factor_name(user: UserId) -> String {
    format!("users/{}/two_factor_secret", user.get())
}

/// The name under which a setting that this application does not know is sealed.
pub fn legacy_setting_name(key: &str) -> String {
    format!("legacy_settings/{key}")
}

/// Seals secrets for storing and opens stored secrets.
pub struct SecretBox {
    cipher: XChaCha20Poly1305,
}

impl SecretBox {
    /// A box that works with the key that is derived from the master key.
    pub fn new(master: &MasterKey) -> Result<Self, KeyError> {
        let mut key = Zeroizing::new([0_u8; KEY_LENGTH]);
        Hkdf::<Sha256>::new(None, master.0.as_slice())
            .expand(PURPOSE, key.as_mut_slice())
            .map_err(|_| KeyError::Invalid)?;
        let cipher =
            XChaCha20Poly1305::new_from_slice(key.as_slice()).map_err(|_| KeyError::Invalid)?;
        Ok(Self { cipher })
    }

    /// A box with a new random key. What it seals cannot be opened by another box, so it
    /// is for tests and for an application that stores nothing.
    pub fn with_random_key() -> Result<Self, KeyError> {
        Self::new(&MasterKey::generate()?)
    }

    /// Seals a secret for storing under the given name. Sealing the same secret twice
    /// gives two different texts.
    pub fn seal(&self, name: &str, secret: &str) -> Result<String, SecretError> {
        let mut nonce = [0_u8; NONCE_LENGTH];
        getrandom::fill(&mut nonce).map_err(SecretError::Random)?;
        let sealed = self
            .cipher
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: secret.as_bytes(),
                    aad: name.as_bytes(),
                },
            )
            .map_err(|_| SecretError::CannotSeal)?;
        Ok(format!(
            "{PREFIX}{}{}",
            hex::encode(nonce),
            hex::encode(sealed)
        ))
    }

    /// Opens a stored value that was sealed under the given name. A value that is not
    /// sealed comes back as it is.
    pub fn open(&self, name: &str, stored: &str) -> Result<String, SecretError> {
        let Some(sealed) = stored.strip_prefix(PREFIX) else {
            return Ok(stored.to_owned());
        };
        let bytes = hex::decode(sealed).map_err(|_| SecretError::CannotOpen)?;
        let (nonce, sealed) = bytes
            .split_first_chunk::<NONCE_LENGTH>()
            .ok_or(SecretError::CannotOpen)?;
        let secret = self
            .cipher
            .decrypt(
                &XNonce::from(*nonce),
                Payload {
                    msg: sealed,
                    aad: name.as_bytes(),
                },
            )
            .map_err(|_| SecretError::CannotOpen)?;
        String::from_utf8(secret).map_err(|_| SecretError::CannotOpen)
    }
}

/// The box holds a key, so `Debug` does not show it.
impl fmt::Debug for SecretBox {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretBox(<hidden>)")
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyError, MasterKey, PREFIX, SecretBox, SecretError};

    /// A key of a test. It is no key of any install.
    const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    fn secrets() -> SecretBox {
        let key = MasterKey::from_hex(KEY).expect("the key is valid");
        SecretBox::new(&key).expect("a box can be made")
    }

    #[test]
    fn sealed_secret_opens_to_the_same_text() {
        let secrets = secrets();
        for secret in ["a-token", "", "pässwörd with spaces", &"x".repeat(5000)] {
            let sealed = secrets
                .seal("plexToken", secret)
                .expect("the secret can be sealed");

            assert!(sealed.starts_with(PREFIX));
            assert!(!sealed.contains("a-token"));
            assert_eq!(
                secrets.open("plexToken", &sealed).expect("it opens"),
                secret
            );
        }
    }

    #[test]
    fn same_secret_sealed_twice_gives_two_texts() {
        let secrets = secrets();

        let first = secrets
            .seal("plexToken", "a-token")
            .expect("the secret can be sealed");
        let second = secrets
            .seal("plexToken", "a-token")
            .expect("the secret can be sealed");

        assert_ne!(first, second);
    }

    #[test]
    fn value_that_is_not_sealed_comes_back_as_it_is() {
        let secrets = secrets();

        assert_eq!(
            secrets.open("plexToken", "a-token").expect("it is plain"),
            "a-token"
        );
        assert_eq!(secrets.open("plexToken", "").expect("it is plain"), "");
    }

    #[test]
    fn sealed_secret_does_not_open_under_another_name_or_with_another_key() {
        let secrets = secrets();
        let sealed = secrets
            .seal("plexToken", "a-token")
            .expect("the secret can be sealed");
        let other_key = SecretBox::with_random_key().expect("a key can be made");

        for result in [
            secrets.open("embyToken", &sealed),
            other_key.open("plexToken", &sealed),
        ] {
            assert!(matches!(result, Err(SecretError::CannotOpen)), "{result:?}");
        }
    }

    #[test]
    fn changed_or_cut_value_does_not_open() {
        let secrets = secrets();
        let sealed = secrets
            .seal("plexToken", "a-token")
            .expect("the secret can be sealed");
        let last = sealed.chars().last().expect("the text is not empty");
        let flipped = if last == '0' { '1' } else { '0' };
        let changed = format!("{}{flipped}", &sealed[..sealed.len() - 1]);

        for damaged in [
            changed,
            sealed[..sealed.len() - 2].to_owned(),
            format!("{PREFIX}00ff"),
            PREFIX.to_owned(),
            format!("{PREFIX}not hex"),
        ] {
            let result = secrets.open("plexToken", &damaged);
            assert!(
                matches!(result, Err(SecretError::CannotOpen)),
                "{damaged}: {result:?}"
            );
        }
    }

    #[test]
    fn key_is_read_from_64_hex_digits_only() {
        let key = MasterKey::from_hex(&format!("  {KEY}\n")).expect("space around the key is fine");
        assert_eq!(key.to_hex().as_str(), KEY);

        for text in [
            "",
            "00",
            &KEY[..62],
            &format!("{KEY}00"),
            &KEY.replace('0', "g"),
        ] {
            assert!(
                matches!(MasterKey::from_hex(text), Err(KeyError::Invalid)),
                "{text}"
            );
        }
    }

    #[test]
    fn new_keys_differ_and_do_not_show_in_debug_output() {
        let first = MasterKey::generate().expect("a key can be made");
        let second = MasterKey::generate().expect("a key can be made");

        assert_ne!(first.to_hex(), second.to_hex());
        assert_eq!(format!("{first:?}"), "MasterKey(<hidden>)");
        let secrets = SecretBox::new(&first).expect("a box can be made");
        assert_eq!(format!("{secrets:?}"), "SecretBox(<hidden>)");
    }

    #[test]
    fn same_master_key_gives_the_same_box() {
        let sealed = secrets()
            .seal("plexToken", "a-token")
            .expect("the secret can be sealed");

        assert_eq!(
            secrets().open("plexToken", &sealed).expect("it opens"),
            "a-token"
        );
    }
}
