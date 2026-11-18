//! Settings whose value is one of a fixed list.

use crate::codec::TextChoice;

/// Which password check a login uses. The setting is `authType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthType {
    /// Only the passwords Organizr stores.
    Internal,
    /// Only the external service that `authBackend` names.
    External,
    /// The external service first, then the stored password.
    Both,
}

impl TextChoice for AuthType {
    const ALL: &'static [Self] = &[Self::Internal, Self::External, Self::Both];

    fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::External => "external",
            Self::Both => "both",
        }
    }
}

/// Where tabs of the default category stand in the menu. The setting is `unsortedTabs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnsortedTabs {
    /// Above the categories.
    Top,
    /// Below the categories.
    Bottom,
}

impl TextChoice for UnsortedTabs {
    const ALL: &'static [Self] = &[Self::Top, Self::Bottom];

    fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
}
