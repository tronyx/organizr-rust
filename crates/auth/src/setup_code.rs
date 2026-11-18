//! The set-up code: proof that the person who sets an install up can read the server log.
//!
//! A new install has no account, so nothing can say who may create the first one. The
//! PHP application let the first visitor do it. Here the server makes a random code when
//! it starts on an install that is not set up, and writes it to its log. The set-up
//! wizard must send the code.

use std::fmt;

use subtle::ConstantTimeEq;

/// The characters of a code: capital letters and digits, without the ones that are easy
/// to mix up (I and 1, L, O and 0).
const ALPHABET: &[u8; 31] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// How many characters a code has, without the dashes. Twelve characters of this
/// alphabet are 59 bits, far more than anyone can guess through the network.
const LENGTH: usize = 12;

/// How many characters stand between two dashes when a code is shown.
const GROUP: usize = 4;

/// A set-up code. It is shown as three groups of four characters, for example
/// `K7QM-2XWD-9RTB`.
#[derive(Clone, PartialEq, Eq)]
pub struct SetupCode([u8; LENGTH]);

impl SetupCode {
    /// Makes a new code from the random source of the operating system.
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut code = [0_u8; LENGTH];
        let mut filled = 0;
        while filled < LENGTH {
            let mut random = [0_u8; 2 * LENGTH];
            getrandom::fill(&mut random)?;
            // Five bits of a byte choose a character. The one value of the 32 that has
            // no character is skipped, so every character is as likely as any other.
            for byte in random {
                let character = ALPHABET.get(usize::from(byte & 0b1_1111));
                if let (Some(character), Some(place)) = (character, code.get_mut(filled)) {
                    *place = *character;
                    filled += 1;
                }
            }
        }
        Ok(Self(code))
    }

    /// Reads a code from text, as a test or a person writes it. Dashes and spaces are
    /// left out, and small letters count as capital letters.
    ///
    /// Returns `None` when the text is not twelve characters of a code.
    pub fn parse(text: &str) -> Option<Self> {
        let characters: Vec<u8> = text
            .bytes()
            .filter(|byte| !matches!(byte, b'-' | b' '))
            .map(|byte| byte.to_ascii_uppercase())
            .collect();
        let code: [u8; LENGTH] = characters.try_into().ok()?;
        code.iter()
            .all(|character| ALPHABET.contains(character))
            .then_some(Self(code))
    }

    /// Whether `text` is this code. The time this takes does not tell how much of a
    /// wrong code was right.
    pub fn matches(&self, text: &str) -> bool {
        Self::parse(text).is_some_and(|candidate| bool::from(candidate.0.ct_eq(&self.0)))
    }
}

impl fmt::Display for SetupCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, character) in self.0.iter().enumerate() {
            if index > 0 && index % GROUP == 0 {
                formatter.write_str("-")?;
            }
            write!(formatter, "{}", char::from(*character))?;
        }
        Ok(())
    }
}

/// The code is a secret until the set-up is done, so `Debug` hides it. It is written to
/// the log on purpose, with `Display`, in one place.
impl fmt::Debug for SetupCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SetupCode(<hidden>)")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::SetupCode;

    const CODE: &str = "K7QM-2XWD-9RTB";

    #[test]
    fn new_code_is_three_groups_of_four_clear_characters() {
        let code = SetupCode::generate()
            .expect("the system gives random bytes")
            .to_string();

        let groups: Vec<&str> = code.split('-').collect();
        assert_eq!(groups.len(), 3, "{code}");
        for group in groups {
            assert_eq!(group.len(), 4, "{code}");
            assert!(
                group
                    .bytes()
                    .all(|byte| b"ABCDEFGHJKMNPQRSTUVWXYZ23456789".contains(&byte)),
                "{code}"
            );
        }
    }

    #[test]
    fn every_new_code_is_another_one() {
        let codes: HashSet<String> = (0..100)
            .map(|_| {
                SetupCode::generate()
                    .expect("the system gives random bytes")
                    .to_string()
            })
            .collect();

        assert_eq!(codes.len(), 100);
    }

    #[test]
    fn new_code_matches_what_it_shows() {
        let code = SetupCode::generate().expect("the system gives random bytes");

        assert!(code.matches(&code.to_string()));
    }

    #[test]
    fn code_is_read_as_a_person_types_it() {
        let code = SetupCode::parse(CODE).expect("the text is a code");

        assert_eq!(code.to_string(), CODE);
        for typed in [
            "K7QM-2XWD-9RTB",
            "k7qm-2xwd-9rtb",
            "K7QM2XWD9RTB",
            " K7QM 2XWD 9RTB ",
            "K7-QM2X-WD9RTB",
        ] {
            assert!(code.matches(typed), "{typed:?}");
        }
    }

    #[test]
    fn any_other_text_does_not_match() {
        let code = SetupCode::parse(CODE).expect("the text is a code");

        for typed in [
            "",
            "K7QM-2XWD-9RT",
            "K7QM-2XWD-9RTBB",
            "K7QM-2XWD-9RTC",
            "A7QM-2XWD-9RTB",
            // Characters that no code has.
            "K7QM-2XWD-9RT0",
            "K7QM-2XWD-9RTI",
            "K7QM_2XWD_9RTB",
            "K7QM-2XWD-9RTÉ",
        ] {
            assert!(!code.matches(typed), "{typed:?}");
        }
    }

    #[test]
    fn debug_output_hides_the_code() {
        let code = SetupCode::parse(CODE).expect("the text is a code");

        assert_eq!(format!("{code:?}"), "SetupCode(<hidden>)");
    }
}
