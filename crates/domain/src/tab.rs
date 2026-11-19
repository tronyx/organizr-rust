//! Tabs: the entries of the menu, each one an application or a page.

use thiserror::Error;

use crate::{CategoryId, Change, GroupLevel, TabId};

/// How a tab opens. The API calls this `type` and sends the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabKind {
    /// A page of Organizr itself, such as the settings or the homepage. Number 0.
    Internal,
    /// Another application, shown in a frame inside Organizr. Number 1.
    Iframe,
    /// Another application, opened in a new browser window. Number 2.
    NewWindow,
}

impl TabKind {
    /// The number the API and the database use for this kind.
    pub const fn code(self) -> u8 {
        match self {
            Self::Internal => 0,
            Self::Iframe => 1,
            Self::NewWindow => 2,
        }
    }
}

impl TryFrom<i64> for TabKind {
    type Error = InvalidTabKind;

    fn try_from(code: i64) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Internal),
            1 => Ok(Self::Iframe),
            2 => Ok(Self::NewWindow),
            other => Err(InvalidTabKind(other)),
        }
    }
}

/// A number that is not a tab kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{0} is not a tab type; the types are 0, 1 and 2")]
pub struct InvalidTabKind(i64);

/// The group levels that can see a tab.
///
/// A lower level number means more access, so the range runs from `most_access` (the
/// lower number) to `least_access` (the higher number). The API calls the two ends
/// `group_id_max` and `group_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelRange {
    least_access: GroupLevel,
    most_access: GroupLevel,
}

impl LevelRange {
    /// Creates a range. `most_access` must not be a higher number than `least_access`.
    pub fn new(
        least_access: GroupLevel,
        most_access: GroupLevel,
    ) -> Result<Self, InvalidLevelRange> {
        if most_access <= least_access {
            Ok(Self {
                least_access,
                most_access,
            })
        } else {
            Err(InvalidLevelRange {
                least_access,
                most_access,
            })
        }
    }

    /// The range that every level from Admin down to `least_access` is in.
    pub const fn up_to(least_access: GroupLevel) -> Self {
        Self {
            least_access,
            most_access: GroupLevel::ADMIN,
        }
    }

    /// The level with the least access that still sees the tab. The API calls it `group_id`.
    pub const fn least_access(self) -> GroupLevel {
        self.least_access
    }

    /// The level with the most access that still sees the tab. The API calls it
    /// `group_id_max`.
    pub const fn most_access(self) -> GroupLevel {
        self.most_access
    }

    /// Whether a user of the given level sees the tab.
    pub fn contains(self, level: GroupLevel) -> bool {
        self.most_access <= level && level <= self.least_access
    }
}

/// Two levels that do not make a range: the end with the most access is a higher number
/// than the end with the least access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "the level with the most access ({}) is a higher number than the level with the least access ({})",
    most_access.get(),
    least_access.get()
)]
pub struct InvalidLevelRange {
    least_access: GroupLevel,
    most_access: GroupLevel,
}

/// A tab of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a tab has seven independent switches, each one a column of the PHP application"
)]
pub struct Tab {
    /// The identifier of the tab.
    pub id: TabId,
    /// Where the tab stands in its category; lower comes first. The API calls it `order`.
    pub position: i64,
    /// The category the tab belongs to.
    pub category: CategoryId,
    /// The name. No two tabs share a name, whatever the letter case.
    pub name: String,
    /// The address of the application.
    pub url: Option<String>,
    /// The address to use for callers on the local network.
    pub url_local: Option<String>,
    /// Whether this tab opens when Organizr starts.
    pub is_default: bool,
    /// Whether the tab is shown at all.
    pub enabled: bool,
    /// The group levels that see the tab.
    pub access: LevelRange,
    /// Whether admins see the tab even when their level is outside `access`.
    pub add_to_admin: bool,
    /// The picture: an icon name, a path or a URL.
    pub image: Option<String>,
    /// How the tab opens.
    pub kind: TabKind,
    /// Whether the tab appears on the splash screen.
    pub splash: bool,
    /// Whether Organizr checks that the application answers.
    pub ping: bool,
    /// The address to check, when it is not the address of the tab.
    pub ping_url: Option<String>,
    /// Whether the tab closes or reloads after it has been idle.
    pub timeout: bool,
    /// The idle time in milliseconds.
    pub timeout_ms: Option<i64>,
    /// Whether the tab loads in the background when Organizr starts.
    pub preload: bool,
}

/// The data for a tab that does not exist yet. The fields are those of [`Tab`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a tab has seven independent switches, each one a column of the PHP application"
)]
pub struct NewTab {
    /// Where the tab stands in its category; lower comes first.
    pub position: i64,
    /// The category the tab belongs to.
    pub category: CategoryId,
    /// The name.
    pub name: String,
    /// The address of the application.
    pub url: Option<String>,
    /// The address to use for callers on the local network.
    pub url_local: Option<String>,
    /// Whether this tab opens when Organizr starts.
    pub is_default: bool,
    /// Whether the tab is shown at all.
    pub enabled: bool,
    /// The group levels that see the tab.
    pub access: LevelRange,
    /// Whether admins see the tab even when their level is outside `access`.
    pub add_to_admin: bool,
    /// The picture: an icon name, a path or a URL.
    pub image: Option<String>,
    /// How the tab opens.
    pub kind: TabKind,
    /// Whether the tab appears on the splash screen.
    pub splash: bool,
    /// Whether Organizr checks that the application answers.
    pub ping: bool,
    /// The address to check, when it is not the address of the tab.
    pub ping_url: Option<String>,
    /// Whether the tab closes or reloads after it has been idle.
    pub timeout: bool,
    /// The idle time in milliseconds.
    pub timeout_ms: Option<i64>,
    /// Whether the tab loads in the background when Organizr starts.
    pub preload: bool,
}

/// The changes to a tab. A field that is [`Change::Keep`] stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabChanges {
    /// Where the tab stands in its category; lower comes first.
    pub position: Change<i64>,
    /// The category the tab belongs to.
    pub category: Change<CategoryId>,
    /// The name.
    pub name: Change<String>,
    /// The address of the application.
    pub url: Change<Option<String>>,
    /// The address to use for callers on the local network.
    pub url_local: Change<Option<String>>,
    /// Whether this tab opens when Organizr starts. Setting it takes the mark from every
    /// other tab.
    pub is_default: Change<bool>,
    /// Whether the tab is shown at all.
    pub enabled: Change<bool>,
    /// The group levels that see the tab.
    pub access: Change<LevelRange>,
    /// Whether admins see the tab even when their level is outside `access`.
    pub add_to_admin: Change<bool>,
    /// The picture: an icon name, a path or a URL.
    pub image: Change<Option<String>>,
    /// How the tab opens.
    pub kind: Change<TabKind>,
    /// Whether the tab appears on the splash screen.
    pub splash: Change<bool>,
    /// Whether Organizr checks that the application answers.
    pub ping: Change<bool>,
    /// The address to check, when it is not the address of the tab.
    pub ping_url: Change<Option<String>>,
    /// Whether the tab closes or reloads after it has been idle.
    pub timeout: Change<bool>,
    /// The idle time in milliseconds.
    pub timeout_ms: Change<Option<i64>>,
    /// Whether the tab loads in the background when Organizr starts.
    pub preload: Change<bool>,
}

#[cfg(test)]
mod tests {
    use super::{InvalidTabKind, LevelRange, TabKind};
    use crate::GroupLevel;

    fn level(value: u16) -> GroupLevel {
        GroupLevel::new(value).expect("the level is valid")
    }

    #[test]
    fn each_kind_has_the_number_of_the_php_application() {
        assert_eq!(TabKind::Internal.code(), 0);
        assert_eq!(TabKind::Iframe.code(), 1);
        assert_eq!(TabKind::NewWindow.code(), 2);
    }

    #[test]
    fn number_converts_to_its_kind_and_back() {
        for kind in [TabKind::Internal, TabKind::Iframe, TabKind::NewWindow] {
            assert_eq!(TabKind::try_from(i64::from(kind.code())), Ok(kind));
        }
    }

    #[test]
    fn other_number_is_not_a_kind() {
        assert_eq!(TabKind::try_from(3), Err(InvalidTabKind(3)));
        assert_eq!(TabKind::try_from(-1), Err(InvalidTabKind(-1)));
    }

    #[test]
    fn range_holds_the_levels_between_its_ends() {
        let range = LevelRange::new(level(4), level(1)).expect("the range is valid");

        assert!(
            !range.contains(GroupLevel::ADMIN),
            "more access than the range"
        );
        assert!(range.contains(level(1)));
        assert!(range.contains(level(3)));
        assert!(range.contains(level(4)));
        assert!(!range.contains(level(5)), "less access than the range");
        assert!(!range.contains(GroupLevel::GUEST));
    }

    #[test]
    fn range_of_one_level_holds_only_that_level() {
        let range = LevelRange::new(level(2), level(2)).expect("the range is valid");

        assert!(range.contains(level(2)));
        assert!(!range.contains(level(1)));
        assert!(!range.contains(level(3)));
    }

    #[test]
    fn range_up_to_a_level_starts_at_admin() {
        let range = LevelRange::up_to(level(4));

        assert_eq!(range.most_access(), GroupLevel::ADMIN);
        assert_eq!(range.least_access(), level(4));
        assert!(range.contains(GroupLevel::ADMIN));
        assert!(!range.contains(GroupLevel::GUEST));
    }

    #[test]
    fn ends_in_the_wrong_order_are_not_a_range() {
        let error = LevelRange::new(level(1), level(4)).expect_err("the ends are swapped");

        assert_eq!(
            error.to_string(),
            "the level with the most access (4) is a higher number than the level with the least access (1)"
        );
    }
}
