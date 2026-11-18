//! Session tokens: the secret a browser holds to stay logged in.

use std::fmt;

use organizr_domain::SessionTokenHash;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// How many random bytes a token is made from: 256 bits.
const TOKEN_BYTES: usize = 32;

/// How many characters a token has: two hexadecimal digits for each byte.
const TOKEN_LENGTH: usize = TOKEN_BYTES * 2;

/// The operating system gave no random bytes, so no token could be made.
#[derive(Debug, Error)]
#[error("the operating system gave no random bytes: {0}")]
pub struct TokenError(#[source] getrandom::Error);

/// A session token: 64 hexadecimal characters from 32 random bytes.
///
/// The token goes to the browser in a cookie and is not stored. The database holds its
/// [hash](Self::hash). The `Debug` output hides the token, so it cannot reach a log by
/// accident.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionToken(String);

impl SessionToken {
    /// Makes a new token from the random source of the operating system.
    pub fn generate() -> Result<Self, TokenError> {
        let mut bytes = [0_u8; TOKEN_BYTES];
        getrandom::fill(&mut bytes).map_err(TokenError)?;
        Ok(Self(hex::encode(bytes)))
    }

    /// Reads a token that a browser sent.
    ///
    /// Returns `None` when the text does not have the form of a token. Such a text can
    /// never name a session, so the caller does not need to ask the database.
    pub fn parse(text: &str) -> Option<Self> {
        let has_the_form = text.len() == TOKEN_LENGTH
            && text
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        has_the_form.then(|| Self(text.to_owned()))
    }

    /// The token, for the cookie.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// The form of the token that the database stores.
    pub fn hash(&self) -> SessionTokenHash {
        SessionTokenHash::new(hex::encode(Sha256::digest(self.0.as_bytes())))
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionToken(<hidden>)")
    }
}

#[cfg(test)]
mod tests {
    use super::SessionToken;

    /// A text with the form of a token.
    const TOKEN: &str = "abababababababababababababababababababababababababababababababab";

    /// The SHA-256 hash of [`TOKEN`], from Python's `hashlib`.
    const TOKEN_HASH: &str = "271a413bd339c5709fdceaec41f14f11e9fbfb5042d72d331c65f32b284cd09a";

    #[test]
    fn new_token_is_64_lower_case_hexadecimal_characters() {
        let token = SessionToken::generate().expect("the system gives random bytes");

        assert_eq!(token.expose().len(), 64, "{}", token.expose());
        assert!(
            token
                .expose()
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
            "{}",
            token.expose()
        );
    }

    #[test]
    fn every_new_token_is_another_one() {
        let tokens: std::collections::HashSet<String> = (0..100)
            .map(|_| {
                SessionToken::generate()
                    .expect("the system gives random bytes")
                    .expose()
                    .to_owned()
            })
            .collect();

        assert_eq!(tokens.len(), 100);
    }

    #[test]
    fn new_token_can_be_read_back_from_its_text() {
        let token = SessionToken::generate().expect("the system gives random bytes");

        assert_eq!(SessionToken::parse(token.expose()), Some(token));
    }

    #[test]
    fn stored_form_is_the_sha256_hash_of_the_token() {
        let token = SessionToken::parse(TOKEN).expect("the text has the form of a token");

        assert_eq!(token.hash().as_str(), TOKEN_HASH);
    }

    #[test]
    fn text_without_the_form_of_a_token_is_not_read() {
        let upper_case = TOKEN.to_uppercase();
        let too_long = format!("{TOKEN}ab");
        let not_hexadecimal = TOKEN.replacen('a', "g", 1);
        let with_space = format!(" {}", TOKEN.get(1..).expect("the text is ASCII"));
        // 64 bytes, but not 64 ASCII characters.
        let not_ascii = format!("é{}", TOKEN.get(2..).expect("the text is ASCII"));
        let not_tokens = [
            "",
            "ab",
            TOKEN.get(..63).expect("the text is ASCII"),
            too_long.as_str(),
            upper_case.as_str(),
            not_hexadecimal.as_str(),
            with_space.as_str(),
            not_ascii.as_str(),
            // What a cookie of the PHP application looks like.
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJPcmdhbml6ciJ9.c2lnbmF0dXJl",
        ];

        for text in not_tokens {
            assert_eq!(SessionToken::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn debug_output_hides_the_token() {
        let token = SessionToken::parse(TOKEN).expect("the text has the form of a token");

        assert_eq!(format!("{token:?}"), "SessionToken(<hidden>)");
    }
}
